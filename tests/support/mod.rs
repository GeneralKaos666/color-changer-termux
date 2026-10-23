//! Shared helpers for the integration tests.
//!
//! Each test binary compiles this module separately, so helpers used by a
//! later task are dead code in an earlier binary.
#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use termux_colors::app::App;
use termux_colors::paths::Paths;
use termux_colors::screens::preset_picker::PresetPicker;

/// Build a plain key press.
pub fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Draw `app` into a `width` x `height` test buffer and return its text.
pub fn render_text(app: &mut App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

/// Open the preset picker from the main menu.
pub fn open_picker(app: &mut App) {
    app.handle_key(key(KeyCode::Enter));
}

/// Build a picker over a temporary library holding `names`.
pub fn picker_with(names: &[&str]) -> PresetPicker {
    let root = tempfile::tempdir().unwrap().keep();
    let library = root.join("colors");
    std::fs::create_dir_all(&library).unwrap();
    for name in names {
        std::fs::write(
            library.join(format!("{name}.properties")),
            "background=#000000\nforeground=#ffffff\ncursor=#ffffff\n",
        )
        .unwrap();
    }
    PresetPicker::new(&Paths::new(root))
}
