//! Shared helpers for the integration tests.
//!
//! Each test binary compiles this module separately, so helpers used by a
//! later task are dead code in an earlier binary.
#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use termux_settings::app::{App, Command};
use termux_settings::paths::Paths;
use termux_settings::screens::Screen;
use termux_settings::screens::app_theme_editor::AppThemeEditor;
use termux_settings::screens::preset_picker::PresetPicker;
use termux_settings::screens::terminal_editor::TerminalEditor;
use termux_settings::ui::dialog::DialogResult;

/// Build a plain key press.
pub fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Build a Ctrl+<char> key press.
pub fn ctrl(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)
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
        termux_settings::screens::Screen::Presets(picker) => {
            picker.selected().expect("a theme is selected").path.clone()
        }
        _ => panic!("the picker is not the current screen"),
    }
}

/// Open the terminal colour editor from the Colors tab.
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
    let editor = editor_mut(app);
    editor.focus = 0;
    editor.fields[0].set_value("");
    editor.revalidate();
}

/// Build an app on the terminal editor whose active theme has `background`.
pub fn editor_with_active(background: &str) -> (App, Paths) {
    editor_with_colors(Some(background))
}

/// Build an app on the terminal editor, optionally with an active theme that
/// sets only `background`.
pub fn editor_with_colors(background: Option<&str>) -> (App, Paths) {
    let root = tempfile::tempdir().unwrap().keep();
    let paths = Paths::new(root);
    if let Some(background) = background {
        std::fs::create_dir_all(paths.colors_root().join("dark")).unwrap();
        let theme = paths.colors_root().join("dark/T.properties");
        std::fs::write(&theme, format!("background={background}\n")).unwrap();
        std::os::unix::fs::symlink(&theme, paths.colors_properties()).unwrap();
    }
    let mut app = App::new(paths.clone());
    open_editor(&mut app);
    (app, paths)
}

/// Set one editor field, adding the `#` that the brief's shorthand omits.
pub fn set_field(app: &mut App, index: usize, value: &str) {
    let value = if value.is_empty() || value.starts_with('#') {
        value.to_owned()
    } else {
        format!("#{value}")
    };
    let editor = editor_mut(app);
    editor.fields[index].set_value(&value);
    editor.revalidate();
}

/// Run Save As and answer its name prompt with `name`.
pub fn save_as(app: &mut App, name: &str) {
    app.command(Command::SaveAs);
    app.resolve_dialog(DialogResult::Text(Some(name.to_owned())));
}

/// Open the app theme editor from the main menu.
pub fn open_app_theme(app: &mut App) {
    // Drill-down: go down to App Theme (index 2) and press Enter
    app.handle_key(key(KeyCode::Down)); // Preset -> Terminal
    app.handle_key(key(KeyCode::Down)); // Terminal -> App Theme
    app.handle_key(key(KeyCode::Enter));
}

/// Build an app on the app theme editor over a temporary directory.
pub fn app_theme_with(contents: &str) -> (App, Paths) {
    let root = tempfile::tempdir().unwrap().keep();
    let paths = Paths::new(root);
    if !contents.is_empty() {
        std::fs::write(paths.termux_properties(), contents).unwrap();
    }
    let mut app = App::new(paths.clone());
    open_app_theme(&mut app);
    (app, paths)
}

/// The open app theme editor.
pub fn app_theme_mut(app: &mut App) -> &mut AppThemeEditor {
    match app.current_mut() {
        Screen::AppTheme(editor) => editor,
        _ => panic!("the app theme editor is not the current screen"),
    }
}

/// The index of the curated row editing `key`.
pub fn row_index(app: &mut App, key: &str) -> usize {
    app_theme_mut(app)
        .rows
        .iter()
        .position(|row| row.spec.key == key)
        .expect("a curated row")
}

/// Move the app theme editor's focus to `index`.
pub fn focus_row(app: &mut App, index: usize) {
    app_theme_mut(app).focus = index;
}

/// Set one curated row, updating its suggestion and validation state.
pub fn set_row(app: &mut App, key: &str, value: &str) {
    let editor = app_theme_mut(app);
    let row = editor
        .rows
        .iter_mut()
        .find(|row| row.spec.key == key)
        .expect("a curated row");
    row.set_value(value);
}

/// The open terminal editor.
pub fn editor_mut(app: &mut App) -> &mut TerminalEditor {
    match app.current_mut() {
        Screen::Terminal(editor) => editor,
        _ => panic!("the editor is not the current screen"),
    }
}

/// Open the Fonts tab from the main menu.
pub fn open_fonts(app: &mut App) {
    app.handle_key(key(KeyCode::Down)); // Preset -> Terminal
    app.handle_key(key(KeyCode::Down)); // Terminal -> App Theme
    app.handle_key(key(KeyCode::Down)); // App Theme -> Fonts
    app.handle_key(key(KeyCode::Enter));
}

/// A minimal but valid TrueType font (156 bytes) that `ttf_parser` accepts.
pub fn minimal_ttf() -> Vec<u8> {
    fn u32(v: u32) -> [u8; 4] {
        v.to_be_bytes()
    }
    fn u16(v: u16) -> [u8; 2] {
        v.to_be_bytes()
    }
    fn i16(v: i16) -> [u8; 2] {
        v.to_be_bytes()
    }

    let mut head = Vec::new();
    head.extend(u32(0x00010000));
    head.extend(u32(0x00010000));
    head.extend(u32(0));
    head.extend(u32(0x5F0F3CF5));
    head.extend(u16(0));
    head.extend(u16(1000));
    head.extend(u64::to_be_bytes(0));
    head.extend(u64::to_be_bytes(0));
    head.extend(i16(0));
    head.extend(i16(0));
    head.extend(i16(1000));
    head.extend(i16(1000));
    head.extend(u16(0));
    head.extend(u16(8));
    head.extend(i16(2));
    head.extend(i16(0));
    head.extend(i16(0));

    let mut hhea = Vec::new();
    hhea.extend(u32(0x00010000));
    hhea.extend(i16(800));
    hhea.extend(i16(-200));
    hhea.extend(i16(0));
    hhea.extend(u16(1000));
    hhea.extend(i16(0));
    hhea.extend(i16(0));
    hhea.extend(i16(1000));
    hhea.extend(i16(1));
    hhea.extend(i16(0));
    hhea.extend(i16(0));
    hhea.extend(i16(0));
    hhea.extend(i16(0));
    hhea.extend(i16(0));
    hhea.extend(i16(0));
    hhea.extend(i16(0));
    hhea.extend(u16(1));

    let mut maxp = Vec::new();
    maxp.extend(u32(0x00005000));
    maxp.extend(u16(1));

    let tables: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"head", head), (b"hhea", hhea), (b"maxp", maxp)];

    let mut d = Vec::new();
    d.extend(u32(0x00010000));
    d.extend(u16(3));
    d.extend(u16(0));
    d.extend(u16(0));
    d.extend(u16(0));
    let mut offset = 12 + 16 * 3;
    for (tag, data) in &tables {
        d.extend_from_slice(*tag);
        d.extend(u32(0));
        d.extend(u32(offset as u32));
        d.extend(u32(data.len() as u32));
        offset += data.len();
    }
    for (_, data) in &tables {
        d.extend_from_slice(data);
    }
    d
}
