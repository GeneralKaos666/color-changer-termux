//! Screen stack entry points.

pub mod main_menu;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::theme::Palette;
use crate::ui::dialog::{Dialog, DialogResult};

use self::main_menu::MainMenu;

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
}

impl Screen {
    /// Build the screen of `kind`, if it exists yet.
    pub fn new(kind: ScreenKind, paths: &Paths) -> Option<Self> {
        match kind {
            ScreenKind::Main => Some(Self::Main(MainMenu::new(paths))),
            ScreenKind::Presets | ScreenKind::Terminal | ScreenKind::AppTheme => None,
        }
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(screen) => screen.handle_key(key, paths),
        }
    }

    /// Handle the result of a dialog this screen opened.
    pub fn on_dialog(&mut self, result: DialogResult, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(screen) => screen.on_dialog(result, paths),
        }
    }

    /// Draw the screen into `area`.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        match self {
            Self::Main(screen) => screen.render(frame, area, palette),
        }
    }
}
