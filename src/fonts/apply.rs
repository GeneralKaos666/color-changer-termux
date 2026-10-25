use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fonts::validate;
use crate::paths::Paths;

#[derive(Clone, Debug)]
pub struct PreviewRef {
    pub src: PathBuf,
    pub slot: String,
}

#[derive(Clone, Debug)]
pub struct SessionState {
    pub originals: HashMap<String, Option<Vec<u8>>>,
    pub backed_up: HashMap<String, bool>,
    pub dirty: bool,
    pub preview: Option<PreviewRef>,
}

pub const MANUAL_RESTART_HINT: &str =
    "termux-reload-settings not available; restart the Termux app to apply the font";

static mut LAST_RELOAD_OK: Option<bool> = None;

fn snapshot_originals(paths: &Paths) -> HashMap<String, Option<Vec<u8>>> {
    let mut originals = HashMap::new();
    for slot in Paths::slot_names() {
        if let Ok(p) = paths.font_slot_path(slot) {
            match fs::read(&p) {
                Ok(data) => originals.insert(slot.to_string(), Some(data)),
                Err(_) => originals.insert(slot.to_string(), None),
            };
        }
    }
    originals
}

pub fn new_session_state(paths: &Paths) -> SessionState {
    SessionState {
        originals: snapshot_originals(paths),
        backed_up: HashMap::new(),
        dirty: false,
        preview: None,
    }
}

fn is_slot_backup(candidate: &str, stem: &str, ext: &str) -> bool {
    if !candidate.starts_with(&format!("{stem}-")) {
        return false;
    }
    if !candidate.ends_with(ext) {
        return false;
    }
    let middle = &candidate[stem.len() + 1..candidate.len() - ext.len()];
    // YYYY-MM-DD-HHMMSS
    middle.len() == 15 && middle.chars().all(|c| c.is_ascii_digit() || c == '-')
}

pub fn ensure_backup_once(paths: &Paths, target: &Path) -> io::Result<Option<PathBuf>> {
    let meta = match fs::metadata(target) {
        Ok(m) if m.is_file() => m,
        _ => return Ok(None),
    };
    let _ = meta;
    let dest_dir = paths.backups_dir();
    fs::create_dir_all(&dest_dir)?;
    let base = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = Path::new(&base)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let stem = if ext.is_empty() {
        base.clone()
    } else {
        base[..base.len() - ext.len()].to_string()
    };

    if let Ok(entries) = fs::read_dir(&dest_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_slot_backup(&name, &stem, &ext) {
                return Ok(Some(dest_dir.join(name)));
            }
        }
    }

    let timestamp = format_timestamp();
    let dest = dest_dir.join(format!("{stem}-{timestamp}{ext}"));
    if dest.exists() {
        return Ok(Some(dest));
    }
    copy_preserving_mode(target, &dest)?;
    Ok(Some(dest))
}

fn format_timestamp() -> String {
    // Use system time
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Simple formatting
    let t = secs;
    let days = t / 86400;
    let secs_in_day = t % 86400;
    let hour = secs_in_day / 3600;
    let min = (secs_in_day % 3600) / 60;
    let sec = secs_in_day % 60;
    // Approximate date (rough)
    format!(
        "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
        1970 + days / 365,
        (days % 365) / 30 + 1,
        (days % 30) + 1,
        hour,
        min,
        sec
    )
}

fn copy_preserving_mode(src: &Path, dst: &Path) -> io::Result<()> {
    let resolved = if let Ok(r) = dst.canonicalize() {
        r
    } else {
        dst.to_path_buf()
    };
    let _resolved = resolved;
    let meta = fs::metadata(src)?;
    let mut in_file = fs::File::open(src)?;
    let tmp = dst.with_extension("part.tmp");
    let mut out_file = fs::File::create(&tmp)?;
    io::copy(&mut in_file, &mut out_file)?;
    drop(out_file);
    let _ = fs::set_permissions(&tmp, meta.permissions());
    fs::rename(&tmp, dst)?;
    Ok(())
}

fn checked_target(paths: &Paths, src: &Path, slot: &str) -> crate::error::Result<PathBuf> {
    let (ok, reason) = validate::is_valid_font(src);
    if !ok {
        return Err(crate::error::Error::Invalid(format!(
            "invalid font: {reason}"
        )));
    }
    paths.font_slot_path(slot)
}

pub fn install_font(paths: &Paths, src: &Path, slot: &str) -> crate::error::Result<PathBuf> {
    let target = checked_target(paths, src, slot)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = ensure_backup_once(paths, &target)?;
    copy_preserving_mode(src, &target)?;
    reload_settings();
    Ok(target)
}

pub fn preview_font(
    paths: &Paths,
    src: &Path,
    slot: &str,
    st: &mut SessionState,
) -> crate::error::Result<PathBuf> {
    let target = checked_target(paths, src, slot)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = ensure_backup_once(paths, &target)?;
    st.backed_up.insert(slot.to_string(), true);
    copy_preserving_mode(src, &target)?;
    st.dirty = true;
    st.preview = Some(PreviewRef {
        src: src.to_path_buf(),
        slot: slot.to_string(),
    });
    reload_settings();
    Ok(target)
}

pub fn commit_preview(st: &mut SessionState) -> Option<PathBuf> {
    if !st.dirty {
        return None;
    }
    let slot = st.preview.as_ref()?.slot.clone();
    let paths = crate::paths::Paths::from_env();
    let target = paths.font_slot_path(&slot).ok()?;
    st.dirty = false;
    st.preview = None;
    Some(target)
}

pub fn restore_original(paths: &Paths, st: &mut SessionState) -> io::Result<bool> {
    if !st.dirty && st.preview.is_none() {
        return Ok(false);
    }
    let slots: Vec<String> = if let Some(ref p) = st.preview {
        vec![p.slot.clone()]
    } else {
        let mut s: Vec<String> = st.originals.keys().cloned().collect();
        s.sort();
        s
    };
    for slot in slots {
        if !Paths::slot_names().contains(&slot.as_str()) {
            continue;
        }
        if let Ok(target) = paths.font_slot_path(&slot) {
            if let Some(original) = st.originals.get(&slot) {
                match original {
                    None => {
                        let _ = fs::remove_file(&target);
                    }
                    Some(data) => {
                        if let Some(parent) = target.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::write(&target, data)?;
                    }
                }
            }
            st.backed_up.remove(&slot);
        }
    }
    st.dirty = false;
    st.preview = None;
    reload_settings();
    Ok(true)
}

pub fn reload_settings() -> bool {
    if let Ok(path) = std::env::var("PATH") {
        if path.split(':').any(|p| {
            std::path::Path::new(p)
                .join("termux-reload-settings")
                .exists()
        }) && Command::new("termux-reload-settings").output().is_ok()
        {
            unsafe {
                LAST_RELOAD_OK = Some(true);
            }
            return true;
        }
    }
    unsafe {
        LAST_RELOAD_OK = Some(false);
    }
    false
}

pub fn last_reload_ok() -> Option<bool> {
    unsafe { LAST_RELOAD_OK }
}

pub fn is_preview_dirty(st: &SessionState) -> bool {
    st.dirty
}
