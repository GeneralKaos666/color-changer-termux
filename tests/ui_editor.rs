mod support;

use crossterm::event::KeyCode;
use support::{editor_app, editor_with, focus_background_cleared, key, open_editor, render_text};
use termux_colors::app::App;
use termux_colors::paths::Paths;

#[test]
fn fields_initialise_from_the_active_theme() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    std::fs::create_dir_all(paths.colors_root().join("dark")).unwrap();
    std::fs::write(
        paths.colors_root().join("dark/T.properties"),
        "background=#112233\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        paths.colors_root().join("dark/T.properties"),
        paths.colors_properties(),
    )
    .unwrap();
    let mut app = App::new(paths.clone());
    open_editor(&mut app);
    // assert through the rendered buffer
    assert!(render_text(&mut app, 100, 30).contains("#112233"));
}

#[test]
fn invalid_hex_is_flagged_and_ignored_by_the_preview() {
    let mut editor = editor_with("background=#000000\n");
    editor.fields[0].set_value("#zzz");
    editor.revalidate();
    assert!(!editor.valid[0]);
    assert_eq!(editor.current_colors().background.to_hex(), "#000000");
}

#[test]
fn typing_a_valid_hex_emits_a_palette_refresh() {
    use ratatui::style::Color;

    let mut app = editor_app();
    focus_background_cleared(&mut app);
    for ch in "#112233".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    assert_eq!(app.palette().background, Color::Rgb(0x11, 0x22, 0x33));
}

#[test]
fn tab_and_arrows_move_focus_and_scroll_keeps_it_visible() {
    let mut editor = editor_with("");
    assert_eq!(editor.focus, 0);
    editor.handle_key(key(KeyCode::Tab));
    assert_eq!(editor.focus, 1);
    for _ in 0..18 {
        editor.handle_key(key(KeyCode::Tab));
    }
    assert!(editor.scroll > 0);
}
