mod support;

use support::{open_picker, render_text};
use termux_settings::app::App;
use termux_settings::paths::Paths;
use termux_settings::ui::Glyphs;

#[test]
fn header_and_help_bar_render_on_the_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    let rendered = render_text(&mut app, 80, 20);
    assert!(rendered.contains("termux-settings"));
    assert!(rendered.contains("move"));
    assert!(rendered.contains("open"));
}

#[test]
fn footer_shows_status_when_set() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.set_status("Saved theme.");
    let rendered = render_text(&mut app, 80, 20);
    assert!(rendered.contains("Saved theme."));
}

#[test]
fn menu_still_renders_at_the_minimum_size() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    let rendered = render_text(&mut app, 40, 10);
    assert!(rendered.contains("termux-settings"));
    assert!(rendered.contains("Preset Themes"));
    assert!(rendered.contains("Quit"));
}

#[test]
fn breadcrumb_uses_ascii_separator_in_ascii_mode() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::with_glyphs(Paths::new(dir.path()), Glyphs::new(true));
    open_picker(&mut app);
    let rendered = render_text(&mut app, 80, 20);
    assert!(!rendered.contains('›'));
    assert!(rendered.contains('>'));
}
