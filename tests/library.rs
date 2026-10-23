use std::path::Path;

use termux_colors::library::{active_theme_path, apply_theme, reload_with_path_env, scan_themes};

const BODY: &str = "background=#000000\n";

fn library(root: &Path) {
    for (dir, name) in [
        ("dark", "Dracula"),
        ("dark", "Nord"),
        ("light", "Solarized"),
    ] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
        std::fs::write(root.join(dir).join(format!("{name}.properties")), BODY).unwrap();
    }
    std::fs::write(root.join("cyberpunk.properties"), BODY).unwrap();
    std::fs::write(root.join("colors.properties"), BODY).unwrap();
    std::fs::write(root.join("Dracula.properties.backup.20260101"), BODY).unwrap();
    std::fs::write(root.join("old.bak"), BODY).unwrap();
    std::fs::create_dir_all(root.join(".hidden")).unwrap();
    std::fs::write(root.join(".hidden/x.properties"), BODY).unwrap();
}

#[test]
fn scans_skipping_backups_and_the_active_link() {
    let dir = tempfile::tempdir().unwrap();
    library(dir.path());
    let labels: Vec<String> = scan_themes(dir.path()).iter().map(|t| t.label()).collect();
    assert_eq!(
        labels,
        vec!["dark/Dracula", "dark/Nord", "light/Solarized", "cyberpunk"]
    );
}

#[test]
fn missing_root_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert!(scan_themes(&dir.path().join("nope")).is_empty());
}

#[test]
fn active_path_resolves_links() {
    let dir = tempfile::tempdir().unwrap();
    library(dir.path());
    let link = dir.path().join("colors.properties");
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(dir.path().join("dark/Dracula.properties"), &link).unwrap();
    assert_eq!(
        active_theme_path(&link),
        Some(dir.path().join("dark/Dracula.properties"))
    );
    assert_eq!(active_theme_path(&dir.path().join("absent")), None);
}

#[test]
fn apply_creates_a_link_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    library(dir.path());
    let theme = dir.path().join("dark/Dracula.properties");
    let link = dir.path().join("colors.properties");
    std::fs::remove_file(&link).unwrap();
    apply_theme(&theme, &link).unwrap();
    apply_theme(&theme, &link).unwrap();
    assert_eq!(active_theme_path(&link), Some(theme));
}

#[test]
fn apply_to_the_file_itself_does_not_self_link() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("colors.properties");
    std::fs::write(&file, BODY).unwrap();
    apply_theme(&file, &file).unwrap();
    assert!(!file.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn apply_rejects_a_missing_theme() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        apply_theme(
            &dir.path().join("nope.properties"),
            &dir.path().join("colors.properties")
        )
        .is_err()
    );
}

#[test]
fn reload_returns_false_without_the_binary() {
    assert!(!reload_with_path_env(Some("/nonexistent")));
}
