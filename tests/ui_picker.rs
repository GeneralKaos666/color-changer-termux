mod support;

use support::{key, open_picker, picker_with, render_text};
use termux_colors::app::App;
use termux_colors::paths::Paths;
use termux_colors::screens::preset_picker::load_themes;
use termux_colors::ui::focus::{Move, list_move};

#[test]
fn empty_library_is_seeded_with_builtins() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let themes = load_themes(&paths);
    assert_eq!(themes.len(), 6);
    assert!(
        dir.path()
            .join("colors/builtin/dracula.properties")
            .is_file()
    );
}

#[test]
fn filter_is_case_insensitive_and_keeps_a_visible_selection() {
    let mut picker = picker_with(&["Dracula", "Nord", "Solarized"]);
    picker.apply_filter("dra");
    assert_eq!(picker.visible.len(), 1);
    assert_eq!(picker.selected().unwrap().name, "Dracula");
    picker.apply_filter("");
    assert_eq!(picker.visible.len(), 3);
}

#[test]
fn movement_clamps_and_pages() {
    let mut picker = picker_with(&["a", "b", "c", "d"]);
    list_move(&mut picker.list, 4, Move::Down, 2);
    assert_eq!(picker.list.selected(), Some(1));
    for _ in 0..10 {
        list_move(&mut picker.list, 4, Move::Down, 2);
    }
    assert_eq!(picker.list.selected(), Some(3));
    list_move(&mut picker.list, 4, Move::Home, 2);
    assert_eq!(picker.list.selected(), Some(0));
    list_move(&mut picker.list, 4, Move::PageDown, 2);
    assert_eq!(picker.list.selected(), Some(2));
}

#[test]
fn picker_renders_labels_and_the_count() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    open_picker(&mut app);
    let text = render_text(&mut app, 100, 30);
    assert!(text.contains("themes (6)"));
    assert!(text.contains("dracula"));
}

#[test]
fn escape_returns_to_the_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    open_picker(&mut app);
    app.handle_key(key(crossterm::event::KeyCode::Esc));
    assert!(render_text(&mut app, 100, 30).contains("Preset Themes"));
}
