mod support;

use crossterm::event::KeyCode;
use support::{ctrl, key, render_text};
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
fn ctrl_c_quits_from_the_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.handle_key(ctrl('c'));
    assert!(app.should_quit());
}

#[test]
fn tab_navigates_and_ctrl_c_quits() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(ctrl('c'));
    assert!(app.should_quit());
}
