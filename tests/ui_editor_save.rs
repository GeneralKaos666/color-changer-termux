mod support;

use support::{editor_with_active, editor_with_colors, render_text, save_as, set_field};
use termux_settings::app::Command;
use termux_settings::screens::terminal_editor::sanitize_name;
use termux_settings::ui::dialog::{Dialog, DialogResult};

#[test]
fn sanitize_replaces_unsafe_runs() {
    assert_eq!(sanitize_name("my theme"), "my-theme");
    // Python's rule replaces each run outside [A-Za-z0-9._-] with `-` and only
    // trims dashes, so leading dots survive (the spec §7.8 step 3).
    assert_eq!(sanitize_name("../../etc/passwd"), "..-..-etc-passwd");
    assert_eq!(sanitize_name("   "), "custom");
    assert_eq!(sanitize_name("ok_Name.1"), "ok_Name.1");
}

#[test]
fn save_backs_up_then_writes_through_the_link() {
    let (mut app, paths) = editor_with_active("#000000");
    set_field(&mut app, 0, "112233");
    app.command(Command::Save);
    assert!(app.dialog().is_some());
    app.resolve_dialog(DialogResult::Confirmed(true));
    assert_eq!(paths.backups().read_dir().unwrap().count(), 1);
    assert!(
        std::fs::read_to_string(paths.colors_root().join("dark/T.properties"))
            .unwrap()
            .contains("background=#112233")
    );
}

#[test]
fn save_without_an_active_theme_asks_for_a_name() {
    let (mut app, paths) = editor_with_colors(None);
    app.command(Command::Save);
    assert!(matches!(app.dialog(), Some(Dialog::Prompt { .. })));
    app.resolve_dialog(DialogResult::Text(Some("neon theme".into())));
    assert!(
        paths
            .colors_root()
            .join("custom/neon-theme.properties")
            .is_file()
    );
}

#[test]
fn declining_overwrite_leaves_the_file_alone() {
    let (mut app, paths) = editor_with_active("#000000");
    std::fs::create_dir_all(paths.colors_root().join("custom")).unwrap();
    std::fs::write(
        paths.colors_root().join("custom/exists.properties"),
        "background=#111111\n",
    )
    .unwrap();
    save_as(&mut app, "exists");
    app.resolve_dialog(DialogResult::Confirmed(false));
    assert_eq!(
        std::fs::read_to_string(paths.colors_root().join("custom/exists.properties")).unwrap(),
        "background=#111111\n"
    );
}

#[test]
fn invalid_value_opens_a_message_naming_the_keys() {
    let (mut app, _paths) = editor_with_active("#000000");
    set_field(&mut app, 0, "#zzz");
    app.command(Command::Save);
    assert!(render_text(&mut app, 100, 30).contains("Invalid hex colour for: background"));
}

#[test]
fn revert_restores_the_saved_values() {
    let (mut app, _paths) = editor_with_active("#000000");
    set_field(&mut app, 0, "#111111");
    app.command(Command::Revert);
    assert!(render_text(&mut app, 100, 30).contains("#000000"));
}
