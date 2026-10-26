//! Read and write the curated subset of `termux.properties`.
//!
//! The app edits a small, curated set of keys (see [`CURATED_FIELDS`]). Every
//! other key in the file — comments, `extra-keys` arrays, unrelated settings —
//! is left untouched by [`write_app_theme`]. Reading returns only the curated
//! keys the file actually sets, so an empty theme means "nothing to edit yet".

use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;
use crate::model::app_theme::{AppTheme, CURATED_FIELDS};
use crate::persist::fs::{read_text, write_atomic};
use crate::persist::properties::{parse_active, set_value, unset_value};

/// Reads the curated keys from `path`, returning an empty theme when missing.
///
/// A missing file is not an error: an installation that has never configured
/// the app theme simply has no curated keys yet. Only the curated keys that are
/// active (non-commented) in the file are stored; every other line is ignored.
/// Reads follow symlinks, so a link to a theme file yields that theme.
pub fn read_app_theme(path: &Path) -> Result<AppTheme> {
    if !path.exists() {
        return Ok(AppTheme::default());
    }
    let pairs: HashMap<String, String> = parse_active(&read_text(path)?).into_iter().collect();
    let mut values = HashMap::new();
    for field in &CURATED_FIELDS {
        if let Some(value) = pairs.get(field.key) {
            values.insert(field.key.to_owned(), value.clone());
        }
    }
    Ok(AppTheme { values })
}

/// Applies `theme` to `text` line-by-line, preserving unmanaged lines.
///
/// Only the curated fields are touched, in schema order. A curated key the
/// theme does not hold is left alone; a key whose value is empty is commented
/// out with [`unset_value`], while a non-empty value is written with
/// [`set_value`]. Non-curated keys are never read from or written to.
pub fn update_theme_text(text: &str, theme: &AppTheme) -> String {
    let mut result = text.to_owned();
    for field in &CURATED_FIELDS {
        let Some(value) = theme.values.get(field.key) else {
            continue;
        };
        result = if value.is_empty() {
            unset_value(&result, field.key)
        } else {
            set_value(&result, field.key, value)
        };
    }
    result
}

/// Writes `theme` to `path`, creating parent directories as needed.
///
/// When `preserve` is true and the file already exists, its current text is
/// updated with [`update_theme_text`] so comments and unrelated keys survive;
/// otherwise the file starts from the termux-settings header comment. The write
/// follows symlinks, so writing a `termux.properties` link edits the file it
/// points at and leaves the link in place.
pub fn write_app_theme(path: &Path, theme: &AppTheme, preserve: bool) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let content = if preserve && path.exists() {
        update_theme_text(&read_text(path)?, theme)
    } else {
        update_theme_text("# Termux properties managed by termux-settings\n", theme)
    };
    write_atomic(path, content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "extra-keys = [['ESC','-']]\n\nallow-external-apps = true\n\n# fullscreen = true\n\nterminal-transcript-rows = 50000\nuse-black-ui = true\nbackground=background.jpg\n";

    #[test]
    fn reads_only_curated_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("termux.properties");
        std::fs::write(&path, SAMPLE).unwrap();
        let theme = read_app_theme(&path).unwrap();
        assert_eq!(theme.get("use-black-ui"), "true");
        assert_eq!(theme.get("terminal-transcript-rows"), "50000");
        assert_eq!(theme.get("fullscreen"), "");
        assert_eq!(theme.get("background"), "");
    }

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            read_app_theme(&dir.path().join("termux.properties"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn write_preserves_unmanaged_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("termux.properties");
        std::fs::write(&path, SAMPLE).unwrap();
        let mut theme = read_app_theme(&path).unwrap();
        theme.set("terminal-transcript-rows", "1000");
        write_app_theme(&path, &theme, true).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains("extra-keys = [['ESC','-']]\n"));
        assert!(out.contains("background=background.jpg\n"));
        assert!(out.contains("terminal-transcript-rows = 1000\n"));
    }

    #[test]
    fn empty_value_comments_the_key_out() {
        let mut theme = AppTheme::default();
        theme.set("use-black-ui", "");
        assert_eq!(
            update_theme_text("use-black-ui = true\n", &theme),
            "# use-black-ui = true\n"
        );
    }

    #[test]
    fn non_curated_keys_are_ignored() {
        let mut theme = AppTheme::default();
        theme.set("background", "#000000");
        assert_eq!(update_theme_text("a=1\n", &theme), "a=1\n");
    }

    #[test]
    fn absent_file_starts_from_the_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("termux.properties");
        let mut theme = AppTheme::default();
        theme.set("use-black-ui", "true");
        write_app_theme(&path, &theme, true).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# Termux properties managed by termux-settings\nuse-black-ui=true\n"
        );
    }
}
