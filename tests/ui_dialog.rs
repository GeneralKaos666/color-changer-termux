mod support;

use crossterm::event::KeyCode;
use support::{key, render_text};
use termux_settings::app::App;
use termux_settings::paths::Paths;
use termux_settings::ui::dialog::{Dialog, DialogResult};

#[test]
fn confirm_enter_is_true_and_esc_is_false() {
    let mut d = Dialog::confirm("Back up?", "Back up", "Skip");
    assert_eq!(
        d.handle_key(key(KeyCode::Enter)),
        Some(DialogResult::Confirmed(true))
    );
    let mut d = Dialog::confirm("Back up?", "Back up", "Skip");
    assert_eq!(
        d.handle_key(key(KeyCode::Esc)),
        Some(DialogResult::Confirmed(false))
    );
}

#[test]
fn confirm_tab_moves_to_cancel() {
    let mut d = Dialog::confirm("x", "Yes", "No");
    d.handle_key(key(KeyCode::Tab));
    assert_eq!(
        d.handle_key(key(KeyCode::Enter)),
        Some(DialogResult::Confirmed(false))
    );
}

#[test]
fn prompt_returns_trimmed_text_or_none() {
    let mut d = Dialog::prompt("Name", "custom");
    d.handle_key(key(KeyCode::Backspace));
    for ch in "moon".chars() {
        d.handle_key(key(KeyCode::Char(ch)));
    }
    d.handle_key(key(KeyCode::Enter));
    let mut d = Dialog::prompt("Name", "");
    assert_eq!(
        d.handle_key(key(KeyCode::Enter)),
        Some(DialogResult::Text(None))
    );
}

#[test]
fn app_draws_and_closes_dialogs() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.open_dialog(Dialog::message("Something failed.", "Notice"));
    assert!(render_text(&mut app, 100, 30).contains("Something failed."));
    app.handle_key(key(KeyCode::Esc));
    assert!(app.dialog().is_none());
}
