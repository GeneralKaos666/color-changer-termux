//! Screen stack entry points.

pub mod main_menu;
pub mod preset_picker;
pub mod terminal_editor;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::theme::Palette;
use crate::ui::dialog::{Dialog, DialogResult};

use self::main_menu::MainMenu;
use self::preset_picker::PresetPicker;
use self::terminal_editor::TerminalEditor;

/// A screen that can be pushed onto the stack.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScreenKind {
    Main,
    Presets,
    Terminal,
    AppTheme,
}

/// What a screen asks [`crate::app::App`] to do after a key press.
#[derive(Clone, Debug)]
pub enum ScreenAction {
    /// Nothing to do.
    None,
    /// Leave the application.
    Quit,
    /// Open a new screen on top of the current one.
    Push(ScreenKind),
    /// Close the current screen.
    Pop,
    /// Re-derive the UI palette from these colours.
    RefreshPalette(TerminalColors),
    /// Show `message` in the status line.
    Status(String),
    /// Show a modal dialog over the current screen.
    OpenDialog(Dialog),
}

/// The screens this application can show.
pub enum Screen {
    /// The landing menu.
    Main(MainMenu),
    /// The theme library browser.
    Presets(PresetPicker),
    /// The terminal colour editor.
    Terminal(Box<TerminalEditor>),
}

impl Screen {
    /// Build the screen of `kind`, if it exists yet.
    pub fn new(kind: ScreenKind, paths: &Paths) -> Option<Self> {
        match kind {
            ScreenKind::Main => Some(Self::Main(MainMenu::new(paths))),
            ScreenKind::Presets => Some(Self::Presets(PresetPicker::new(paths))),
            ScreenKind::Terminal => Some(Self::Terminal(Box::new(TerminalEditor::new(paths)))),
            ScreenKind::AppTheme => None,
        }
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(screen) => screen.handle_key(key, paths),
            Self::Presets(screen) => screen.handle_key(key, paths),
            Self::Terminal(screen) => screen.handle_key(key),
        }
    }

    /// The action a freshly pushed screen wants the app to apply.
    pub fn opened(&mut self, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(_) => ScreenAction::None,
            Self::Presets(screen) => screen.opened(paths),
            Self::Terminal(screen) => screen.opened(),
        }
    }

    /// Handle the result of a dialog this screen opened.
    pub fn on_dialog(&mut self, result: DialogResult, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(screen) => screen.on_dialog(result, paths),
            Self::Presets(_) | Self::Terminal(_) => ScreenAction::None,
        }
    }

    /// Draw the screen into `area`.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        match self {
            Self::Main(screen) => screen.render(frame, area, palette),
            Self::Presets(screen) => screen.render(frame, area, palette),
            Self::Terminal(screen) => screen.render(frame, area, palette),
        }
    }
}
