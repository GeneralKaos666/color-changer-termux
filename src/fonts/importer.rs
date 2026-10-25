use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::fonts::validate;
use crate::paths::Paths;

pub const STORAGE_HINT: &str = "permission denied: if the file lives on shared storage (/sdcard/Download/...), run `termux-setup-storage`, grant the storage permission, and retry";

#[derive(Debug)]
pub struct ClashError {
    pub dest: PathBuf,
}

impl std::fmt::Display for ClashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "font already in library: {}",
            self.dest.file_name().unwrap_or_default().to_string_lossy()
        )
    }
}

impl std::error::Error for ClashError {}

pub fn resolve_clash(dest: &Path) -> io::Result<PathBuf> {
    let ext = dest
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let stem = dest
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let dir = dest.parent().unwrap_or(Path::new("."));
    for i in 1.. {
        let candidate = dir.join(format!("{stem}-{i}{ext}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    unreachable!()
}

fn same_file(a: &Path, b: &Path) -> bool {
    if let (Ok(ra), Ok(rb)) = (a.canonicalize(), b.canonicalize()) {
        return ra == rb;
    }
    false
}

fn copy_file(src: &Path, dest: &Path) -> io::Result<()> {
    let mut in_file = fs::File::open(src)?;
    let tmp = dest.with_extension("part.tmp");
    let mut out_file = fs::File::create(&tmp)?;
    io::copy(&mut in_file, &mut out_file)?;
    drop(out_file);
    #[cfg(unix)]
    let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o644));
    #[cfg(not(unix))]
    let _ = tmp;
    fs::rename(&tmp, dest)?;
    Ok(())
}

pub fn import_file(paths: &Paths, src: &Path, clash: &str) -> crate::error::Result<PathBuf> {
    match clash {
        "ask" | "error" | "keep-both" | "replace" => {}
        _ => {
            return Err(crate::error::Error::Invalid(format!(
                "unknown clash mode {clash:?}; choose from ask, error, keep-both, replace"
            )));
        }
    }
    let (ok, reason) = validate::is_valid_font(src);
    if !ok {
        let lowered = reason.to_lowercase();
        if lowered.contains("permission denied") {
            return Err(crate::error::Error::Invalid(format!(
                "{STORAGE_HINT} ({src:?}: {reason})"
            )));
        }
        return Err(crate::error::Error::Invalid(format!(
            "invalid font {src:?}: {reason}"
        )));
    }
    let dest_dir = paths.fonts_dir();
    fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(src.file_name().unwrap_or_default());
    if same_file(src, &dest) {
        return Ok(dest);
    }
    if dest.exists() {
        match clash {
            "ask" => {
                return Err(crate::error::Error::Invalid(format!(
                    "font already in library: {}",
                    dest.file_name().unwrap_or_default().to_string_lossy()
                )));
            }
            "error" => {
                return Err(crate::error::Error::Invalid(format!(
                    "font already in library: {}",
                    dest.file_name().unwrap_or_default().to_string_lossy()
                )));
            }
            "keep-both" => {
                let resolved = resolve_clash(&dest)?;
                copy_file(src, &resolved)?;
                return Ok(resolved);
            }
            "replace" => {}
            _ => {}
        }
    }
    copy_file(src, &dest)?;
    Ok(dest)
}
