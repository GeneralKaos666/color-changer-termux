mod support;

use support::render_text;
use termux_settings::app::App;
use termux_settings::paths::Paths;
use termux_settings::ui::Glyphs;

#[test]
fn ascii_mode_uses_plain_borders_and_ascii_help() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::with_glyphs(Paths::new(dir.path()), Glyphs::new(true));
    let rendered = render_text(&mut app, 80, 20);

    assert!(!rendered.contains('─') && !rendered.contains('│'));
    assert!(rendered.contains("up/dn"));
    assert!(!rendered.contains('↑'));
}
