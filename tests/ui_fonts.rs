mod support;

use crossterm::event::KeyCode;
use support::{key, minimal_ttf, open_fonts};
use termux_settings::app::App;
use termux_settings::model::font_slot::FontSlot;
use termux_settings::paths::Paths;

fn fonts_app() -> (App, Paths) {
    let root = tempfile::tempdir().unwrap().keep();
    let paths = Paths::new(root);
    std::fs::create_dir_all(paths.fonts_dir()).unwrap();
    std::fs::write(paths.fonts_dir().join("A.ttf"), minimal_ttf()).unwrap();
    std::fs::write(paths.font_slot_path(FontSlot::Regular), b"orig-regular").unwrap();
    std::fs::write(paths.font_slot_path(FontSlot::Bold), b"orig-bold").unwrap();

    let mut app = App::new(paths.clone());
    open_fonts(&mut app);
    (app, paths)
}

#[test]
fn preview_then_quit_restores() {
    let (mut app, paths) = fonts_app();

    app.handle_key(key(KeyCode::Char('p')));
    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Char('p')));

    assert_eq!(
        std::fs::read(paths.font_slot_path(FontSlot::Regular)).unwrap(),
        minimal_ttf()
    );
    assert_eq!(
        std::fs::read(paths.font_slot_path(FontSlot::Bold)).unwrap(),
        minimal_ttf()
    );

    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.should_quit());

    assert_eq!(
        std::fs::read(paths.font_slot_path(FontSlot::Regular)).unwrap(),
        b"orig-regular"
    );
    assert_eq!(
        std::fs::read(paths.font_slot_path(FontSlot::Bold)).unwrap(),
        b"orig-bold"
    );
}
