use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::fonts::validate;
use crate::library::reload_termux;
use crate::model::font_slot::FontSlot;
use crate::paths::Paths;
use crate::persist::backup::{is_stamped_name, make_unique_backup};

#[derive(Clone, Debug)]
pub struct PreviewRef {
    pub src: PathBuf,
    pub slot: FontSlot,
}

/// The state one visit to the Fonts tab keeps across previews.
#[derive(Clone, Debug)]
pub struct SessionState {
    /// The bytes of each slot when the session started (`None` = no file).
    originals: HashMap<FontSlot, Option<Vec<u8>>>,
    /// Every slot previewed this session, so restore can undo all of them.
    previewed: BTreeMap<FontSlot, PreviewRef>,
    /// Slots already backed up this session, so a second preview never
    /// writes a second backup.
    backed_up: BTreeSet<FontSlot>,
}

pub const MANUAL_RESTART_HINT: &str =
    "termux-reload-settings not available; restart the Termux app to apply the font";

pub fn new_session_state(paths: &Paths) -> SessionState {
    let mut originals = HashMap::new();
    for slot in FontSlot::ALL {
        let p = paths.font_slot_path(slot);
        match fs::read(&p) {
            Ok(data) => originals.insert(slot, Some(data)),
            Err(_) => originals.insert(slot, None),
        };
    }
    SessionState {
        originals,
        previewed: BTreeMap::new(),
        backed_up: BTreeSet::new(),
    }
}

/// Whether any slot has an uncommitted preview.
pub fn is_dirty(st: &SessionState) -> bool {
    !st.previewed.is_empty()
}

/// Make one timestamped backup of `target`, unless this exact session already
/// made one for the same stem at the same second.
fn ensure_backup_once(paths: &Paths, target: &Path) -> crate::error::Result<Option<PathBuf>> {
    if !target.is_file() {
        return Ok(None);
    }
    let dir = paths.backups_dir();
    fs::create_dir_all(&dir)?;
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
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_stamped_name(&name, &stem, &ext) {
                return Ok(Some(dir.join(name)));
            }
        }
    }
    Ok(make_unique_backup(target, &dir, &stem, &ext)?.map(|backup| backup.path))
}

fn copy_preserving_mode(src: &Path, dst: &Path) -> io::Result<()> {
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

fn checked_target(paths: &Paths, src: &Path, slot: FontSlot) -> crate::error::Result<PathBuf> {
    let (ok, reason) = validate::is_valid_font(src);
    if !ok {
        return Err(crate::error::Error::Invalid(format!(
            "invalid font: {reason}"
        )));
    }
    Ok(paths.font_slot_path(slot))
}

pub fn install_font(paths: &Paths, src: &Path, slot: FontSlot) -> crate::error::Result<PathBuf> {
    let target = checked_target(paths, src, slot)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = ensure_backup_once(paths, &target)?;
    copy_preserving_mode(src, &target)?;
    Ok(target)
}

pub fn preview_font(
    paths: &Paths,
    src: &Path,
    slot: FontSlot,
    st: &mut SessionState,
) -> crate::error::Result<PathBuf> {
    let target = checked_target(paths, src, slot)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    if st.backed_up.insert(slot) {
        let _ = ensure_backup_once(paths, &target)?;
    }
    copy_preserving_mode(src, &target)?;
    st.previewed.insert(
        slot,
        PreviewRef {
            src: src.to_path_buf(),
            slot,
        },
    );
    Ok(target)
}

/// Keep every previewed font in place and clear the preview state.
///
/// Returns whether anything was previewed. `paths` is accepted so the commit
/// path never has to re-read the environment (see the design), and the originals
/// snapshot is left at the session-start bytes so a later revert still undoes
/// the whole session.
pub fn commit_preview(_paths: &Paths, st: &mut SessionState) -> bool {
    if st.previewed.is_empty() {
        return false;
    }
    st.previewed.clear();
    true
}

/// Restore the session-start bytes of every previewed slot.
pub fn restore_original(paths: &Paths, st: &mut SessionState) -> io::Result<bool> {
    if st.previewed.is_empty() {
        return Ok(false);
    }
    for slot in st.previewed.keys().copied() {
        let target = paths.font_slot_path(slot);
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
    st.previewed.clear();
    let _ = reload_termux();
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn u32(v: u32) -> [u8; 4] {
        v.to_be_bytes()
    }
    fn u16(v: u16) -> [u8; 2] {
        v.to_be_bytes()
    }
    fn i16(v: i16) -> [u8; 2] {
        v.to_be_bytes()
    }

    /// A minimal but valid TrueType font (156 bytes) that `ttf_parser` accepts.
    fn minimal_ttf() -> Vec<u8> {
        let mut head = Vec::new();
        head.extend(u32(0x00010000));
        head.extend(u32(0x00010000));
        head.extend(u32(0));
        head.extend(u32(0x5F0F3CF5)); // magic
        head.extend(u16(0));
        head.extend(u16(1000)); // unitsPerEm
        head.extend(u64::to_be_bytes(0));
        head.extend(u64::to_be_bytes(0));
        head.extend(i16(0));
        head.extend(i16(0));
        head.extend(i16(1000));
        head.extend(i16(1000));
        head.extend(u16(0));
        head.extend(u16(8));
        head.extend(i16(2));
        head.extend(i16(0));
        head.extend(i16(0));

        let mut hhea = Vec::new();
        hhea.extend(u32(0x00010000));
        hhea.extend(i16(800));
        hhea.extend(i16(-200));
        hhea.extend(i16(0));
        hhea.extend(u16(1000));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(1000));
        hhea.extend(i16(1));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(u16(1));

        let mut maxp = Vec::new();
        maxp.extend(u32(0x00005000));
        maxp.extend(u16(1));

        let tables: Vec<(&[u8; 4], Vec<u8>)> =
            vec![(b"head", head), (b"hhea", hhea), (b"maxp", maxp)];

        let mut d = Vec::new();
        d.extend(u32(0x00010000));
        d.extend(u16(3));
        d.extend(u16(0));
        d.extend(u16(0));
        d.extend(u16(0));
        let mut offset = 12 + 16 * 3;
        for (tag, data) in &tables {
            d.extend_from_slice(*tag);
            d.extend(u32(0));
            d.extend(u32(offset as u32));
            d.extend(u32(data.len() as u32));
            offset += data.len();
        }
        for (_, data) in &tables {
            d.extend_from_slice(data);
        }
        d
    }

    fn slot_files(paths: &Paths) {
        std::fs::write(paths.font_slot_path(FontSlot::Regular), b"orig-regular").unwrap();
        std::fs::write(paths.font_slot_path(FontSlot::Bold), b"orig-bold").unwrap();
    }

    fn backups(paths: &Paths) -> Vec<PathBuf> {
        std::fs::read_dir(paths.backups_dir())
            .map(|rd| rd.flatten().map(|e| e.path()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn restore_covers_every_previewed_slot() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let font = dir.path().join("F.ttf");
        std::fs::write(&font, minimal_ttf()).unwrap();
        slot_files(&paths);

        let mut st = new_session_state(&paths);
        preview_font(&paths, &font, FontSlot::Regular, &mut st).unwrap();
        preview_font(&paths, &font, FontSlot::Bold, &mut st).unwrap();

        assert_eq!(
            std::fs::read(paths.font_slot_path(FontSlot::Regular)).unwrap(),
            minimal_ttf()
        );
        assert_eq!(
            std::fs::read(paths.font_slot_path(FontSlot::Bold)).unwrap(),
            minimal_ttf()
        );

        restore_original(&paths, &mut st).unwrap();

        assert_eq!(
            std::fs::read(paths.font_slot_path(FontSlot::Regular)).unwrap(),
            b"orig-regular"
        );
        assert_eq!(
            std::fs::read(paths.font_slot_path(FontSlot::Bold)).unwrap(),
            b"orig-bold"
        );
    }

    #[test]
    fn commit_keeps_both_and_leaves_nothing_to_restore() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let font = dir.path().join("F.ttf");
        std::fs::write(&font, minimal_ttf()).unwrap();
        slot_files(&paths);

        let mut st = new_session_state(&paths);
        preview_font(&paths, &font, FontSlot::Regular, &mut st).unwrap();
        preview_font(&paths, &font, FontSlot::Bold, &mut st).unwrap();

        assert!(commit_preview(&paths, &mut st));
        assert!(!is_dirty(&st));

        // Both slots keep the previewed font; nothing left to restore.
        assert_eq!(
            std::fs::read(paths.font_slot_path(FontSlot::Regular)).unwrap(),
            minimal_ttf()
        );
        assert!(!restore_original(&paths, &mut st).unwrap());
    }

    #[test]
    fn ensure_backup_once_writes_once_per_slot() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let font = dir.path().join("F.ttf");
        std::fs::write(&font, minimal_ttf()).unwrap();
        std::fs::write(paths.font_slot_path(FontSlot::Regular), b"orig").unwrap();

        let mut st = new_session_state(&paths);
        preview_font(&paths, &font, FontSlot::Regular, &mut st).unwrap();
        preview_font(&paths, &font, FontSlot::Regular, &mut st).unwrap();

        let files = backups(&paths);
        assert_eq!(files.len(), 1, "expected one backup, got {files:?}");
    }

    #[test]
    fn backup_name_has_a_real_date() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let font = dir.path().join("F.ttf");
        std::fs::write(&font, minimal_ttf()).unwrap();
        std::fs::write(paths.font_slot_path(FontSlot::Regular), b"orig").unwrap();

        let mut st = new_session_state(&paths);
        preview_font(&paths, &font, FontSlot::Regular, &mut st).unwrap();

        let files = backups(&paths);
        assert_eq!(files.len(), 1);
        let name = files[0].file_name().unwrap().to_string_lossy().to_string();
        assert!(name.starts_with("font-") && name.ends_with(".ttf"));

        let today =
            time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
        let expected = format!(
            "{:04}-{:02}-{:02}",
            today.year(),
            today.month() as u8,
            today.day()
        );
        let date_part = &name[5..15];
        assert_eq!(date_part, expected);
    }

    #[test]
    fn install_font_succeeds_without_the_reload_binary() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let font = dir.path().join("F.ttf");
        std::fs::write(&font, minimal_ttf()).unwrap();

        let target = install_font(&paths, &font, FontSlot::Regular).unwrap();
        assert_eq!(target, paths.font_slot_path(FontSlot::Regular));
        assert_eq!(std::fs::read(&target).unwrap(), minimal_ttf());
    }

    #[test]
    fn permissions_are_carried_across_preview() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let font = dir.path().join("F.ttf");
        std::fs::write(&font, minimal_ttf()).unwrap();
        std::fs::set_permissions(&font, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(paths.font_slot_path(FontSlot::Regular), b"orig").unwrap();

        let mut st = new_session_state(&paths);
        preview_font(&paths, &font, FontSlot::Regular, &mut st).unwrap();

        let mode = std::fs::metadata(paths.font_slot_path(FontSlot::Regular))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
