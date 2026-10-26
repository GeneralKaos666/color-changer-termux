//! Timestamped backups of Termux config files.
//!
//! Before the editor overwrites `colors.properties` or `termux.properties`,
//! [`make_backup`] copies the current file into a backups directory under a
//! timestamped name so a mistaken save can be undone. Symlinks are followed so
//! a backup holds the real colour data rather than a dangling link.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use time::error::IndeterminateOffset;
use time::{OffsetDateTime, UtcOffset};

use crate::error::Result;

/// A completed backup: where it was written and whether UTC was used.
#[derive(Debug)]
pub struct Backup {
    /// The path the backup was written to.
    pub path: PathBuf,
    /// Whether the local offset could not be determined and UTC was used.
    pub utc_fallback: bool,
}

/// Formats `at` as `%Y-%m-%d-%H%M%S`, optionally converting to `offset` first.
///
/// When `offset` is `Some`, `at` is reinterpreted in that offset before
/// formatting; `None` formats `at` as given (already in the desired offset).
pub fn format_timestamp(at: OffsetDateTime, offset: Option<UtcOffset>) -> String {
    let at = match offset {
        Some(offset) => at.to_offset(offset),
        None => at,
    };
    format!(
        "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
        at.year(),
        at.month() as u8,
        at.day(),
        at.hour(),
        at.minute(),
        at.second(),
    )
}

/// Copies `source` into `backups_dir` under a timestamped name.
///
/// Returns `Ok(None)` when `source` does not exist. Otherwise the source bytes
/// are read (following symlinks) and written to
/// `<backups_dir>/<prefix>-<stamp>.properties`, where `<stamp>` is the current
/// local time formatted by [`format_timestamp`]. The directory is created if
/// needed. On a name collision the suffix `-1`, `-2`, … is appended until the
/// destination does not exist. The stamp is captured once per call; when the
/// local offset cannot be determined the time falls back to UTC and
/// [`Backup::utc_fallback`] is set.
pub fn make_backup(source: &Path, backups_dir: &Path, prefix: &str) -> Result<Option<Backup>> {
    make_unique_backup(source, backups_dir, prefix, ".properties")
}

/// Whether `name` is `<stem>-<YYYY-MM-DD-HHMMSS><ext>`.
///
/// The stamp is exactly 17 characters with dashes at offsets 4, 7 and 10 and
/// digits everywhere else, so a file like `font-2026-10-25-123456.ttf` is
/// recognised while a short or misplaced-dash name is not.
pub fn is_stamped_name(name: &str, stem: &str, ext: &str) -> bool {
    let prefix = format!("{stem}-");
    if !name.starts_with(&prefix) || !name.ends_with(ext) {
        return false;
    }
    let middle = &name[prefix.len()..name.len() - ext.len()];
    if middle.len() != 17 {
        return false;
    }
    middle.chars().enumerate().all(|(index, c)| {
        if matches!(index, 4 | 7 | 10) {
            c == '-'
        } else {
            c.is_ascii_digit()
        }
    })
}

/// Copies `source` into `dir` under a timestamped name with extension `ext`.
///
/// See [`make_backup`]; identical except the caller chooses the extension
/// (including the leading dot, e.g. `".ttf"`).
pub fn make_unique_backup(
    source: &Path,
    dir: &Path,
    prefix: &str,
    ext: &str,
) -> Result<Option<Backup>> {
    if !source.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(dir)?;
    let bytes = std::fs::read(source)?;

    let (at, utc_fallback) = match OffsetDateTime::now_local() {
        Ok(local) => (local, false),
        Err(IndeterminateOffset) => (OffsetDateTime::now_utc(), true),
    };
    let stamp = format_timestamp(at, None);

    let mut destination = dir.join(format!("{prefix}-{stamp}{ext}"));
    let mut counter = 1;
    while destination.exists() {
        destination = dir.join(format!("{prefix}-{stamp}-{counter}{ext}"));
        counter += 1;
    }
    std::fs::write(&destination, bytes)?;

    Ok(Some(Backup {
        path: destination,
        utc_fallback,
    }))
}

/// Lists the regular files in `dir`, newest first by modification time.
///
/// Returns an empty vector when `dir` is missing.
pub fn list_backups(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        files.push((entry.metadata()?.modified()?, path));
    }
    files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    Ok(files.into_iter().map(|(_, path)| path).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_in_the_given_offset() {
        let at = OffsetDateTime::from_unix_timestamp(1_767_225_600).unwrap(); // 2026-01-01T00:00:00Z
        let offset = UtcOffset::from_hms(-8, 0, 0).unwrap();
        assert_eq!(format_timestamp(at, Some(offset)), "2025-12-31-160000");
        assert_eq!(format_timestamp(at, None), "2026-01-01-000000");
    }

    #[test]
    fn copies_content_and_follows_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("Theme.properties");
        std::fs::write(&target, "foreground=#ffffff\n").unwrap();
        let link = dir.path().join("colors.properties");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let backups = dir.path().join("backups");
        let backup = make_backup(&link, &backups, "colors").unwrap().unwrap();

        assert!(backup.path.starts_with(&backups));
        assert!(
            backup
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("colors-")
        );
        assert_eq!(
            std::fs::read_to_string(&backup.path).unwrap(),
            "foreground=#ffffff\n"
        );
    }

    #[test]
    fn collisions_get_a_counter_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("colors.properties");
        std::fs::write(&source, "a=1\n").unwrap();
        let backups = dir.path().join("backups");
        let first = make_backup(&source, &backups, "colors")
            .unwrap()
            .unwrap()
            .path;
        let second = make_backup(&source, &backups, "colors")
            .unwrap()
            .unwrap()
            .path;
        assert_ne!(first, second);
        assert!(
            second
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("-1.properties")
        );
    }

    #[test]
    fn absent_source_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            make_backup(&dir.path().join("nope"), &dir.path().join("b"), "x")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn lists_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old"), b"x").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.path().join("new"), b"x").unwrap();
        let listed = list_backups(dir.path()).unwrap();
        assert_eq!(listed[0].file_name().unwrap(), "new");
    }

    #[test]
    fn stamped_name_accepts_17_char_stamp() {
        assert!(is_stamped_name(
            "colors-2026-10-25-123456.properties",
            "colors",
            ".properties"
        ));
        assert!(is_stamped_name("font-2026-10-25-123456.ttf", "font", ".ttf"));
    }

    #[test]
    fn stamped_name_rejects_wrong_length() {
        assert!(!is_stamped_name(
            "colors-2026-10-25-12345.properties",
            "colors",
            ".properties"
        ));
        assert!(!is_stamped_name(
            "colors-2026-10-25.properties",
            "colors",
            ".properties"
        ));
    }

    #[test]
    fn stamped_name_rejects_misplaced_dashes() {
        assert!(!is_stamped_name(
            "colors-2026-1-25-123456.properties",
            "colors",
            ".properties"
        ));
        assert!(!is_stamped_name(
            "colors-2026-10-25-12345x.properties",
            "colors",
            ".properties"
        ));
    }

    #[test]
    fn make_unique_backup_writes_custom_ext() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("font.ttf");
        std::fs::write(&source, b"fontbytes").unwrap();
        let backups = dir.path().join("backups");
        let backup = make_unique_backup(&source, &backups, "font", ".ttf")
            .unwrap()
            .unwrap();
        let name = backup.path.file_name().unwrap().to_str().unwrap().to_string();
        assert!(name.starts_with("font-") && name.ends_with(".ttf"));
        assert_eq!(std::fs::read(&backup.path).unwrap(), b"fontbytes");
    }
}
