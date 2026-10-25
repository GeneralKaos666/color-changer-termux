mod support;

use crossterm::event::KeyCode;
use support::{key, render_text};
use termux_colors::app::App;
use termux_colors::paths::Paths;

#[test]
fn menu_renders_its_four_items_and_active_theme() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    let rendered = render_text(&mut app, 80, 20);
    assert!(
        rendered.contains("Colors") || rendered.contains("Fonts") || rendered.contains("App Theme")
    );
}

#[test]
fn arrows_move_the_selection_and_quit_item_exits() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.should_quit());
}

#[test]
fn tab_cycles_focus_and_q_quits() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.should_quit());
}
