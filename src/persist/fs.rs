//! Atomic file writes and lossy text reads shared by the persistence layer.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::Result;

/// Read `path` as UTF-8, replacing invalid bytes with U+FFFD.
///
/// The file is read as bytes rather than through [`std::fs::read_to_string`] so
/// a Latin-1 comment cannot stop a config file from loading; the replacement
/// matches the Python implementation's `errors="replace"`.
pub fn read_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Write `bytes` to `path` atomically.
///
/// A symlink `path` is resolved first so the write edits the link target and
/// never clobbers the link itself. The bytes are written to a temp file in the
/// same directory, flushed to disk, given the existing file's permissions when
/// there is one, then renamed over the destination so a crash or a full disk
/// can never truncate the destination.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let resolved = resolve(path)?;
    let dir = resolved
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = resolved
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    let tmp = dir.join(format!(".{file_name}.tmp-{}", std::process::id()));

    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    if let Ok(meta) = fs::metadata(&resolved) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }
    if let Err(error) = fs::rename(&tmp, &resolved) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    Ok(())
}

/// The real file `path` writes should edit: `path` itself, or its target when
/// it is a symlink.
fn resolve(path: &Path) -> Result<PathBuf> {
    let is_link = fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    if is_link {
        Ok(fs::canonicalize(path)?)
    } else {
        Ok(path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::{read_text, write_atomic};
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn write_atomic_follows_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real.txt");
        std::fs::write(&target, "old").unwrap();
        let link = dir.path().join("link.txt");
        symlink(&target, &link).unwrap();

        write_atomic(&link, b"new").unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        let meta = std::fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink());
    }

    #[test]
    fn write_atomic_preserves_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg");
        std::fs::write(&path, "a").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        write_atomic(&path, b"bb").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn write_atomic_leaves_old_bytes_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg");
        std::fs::write(&path, "orig").unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();

        assert!(write_atomic(&path, b"clobber").is_err());

        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "orig");
    }

    #[test]
    fn read_text_replaces_invalid_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("latin1");
        std::fs::write(&path, [0xFF, b'a', 0xFE]).unwrap();
        let text = read_text(&path).unwrap();
        assert_eq!(text, "\u{FFFD}a\u{FFFD}");
    }
}
