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
use termux_colors::screens::Screen;
use termux_colors::screens::preset_picker::PresetPicker;
use termux_colors::screens::terminal_editor::TerminalEditor;

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

/// The path of the theme the picker currently highlights.
pub fn picker_theme_path(app: &mut App) -> std::path::PathBuf {
    match app.current() {
        termux_colors::screens::Screen::Presets(picker) => {
            picker.selected().expect("a theme is selected").path.clone()
        }
        _ => panic!("the picker is not the current screen"),
    }
}

/// Open the terminal colour editor from the main menu.
pub fn open_editor(app: &mut App) {
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Enter));
}

/// Build an app on the terminal editor over an empty temporary directory.
pub fn editor_app() -> App {
    let root = tempfile::tempdir().unwrap().keep();
    let mut app = App::new(Paths::new(root));
    open_editor(&mut app);
    app
}

/// Build an editor whose active theme holds `contents`.
pub fn editor_with(contents: &str) -> TerminalEditor {
    let root = tempfile::tempdir().unwrap().keep();
    let paths = Paths::new(root);
    std::fs::create_dir_all(paths.colors_root().join("dark")).unwrap();
    let theme = paths.colors_root().join("dark/T.properties");
    std::fs::write(&theme, contents).unwrap();
    std::os::unix::fs::symlink(&theme, paths.colors_properties()).unwrap();
    TerminalEditor::new(&paths)
}

/// Focus the first field of the open editor and blank it, so that typed
/// characters start from an empty value.
pub fn focus_background_cleared(app: &mut App) {
    match app.current_mut() {
        Screen::Terminal(editor) => {
            editor.focus = 0;
            editor.fields[0].set_value("");
            editor.revalidate();
        }
        _ => panic!("the editor is not the current screen"),
    }
}
