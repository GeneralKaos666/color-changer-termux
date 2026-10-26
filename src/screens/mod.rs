//! Screen stack entry points.

pub mod app_theme_editor;
pub mod fonts_tab;
pub mod main_menu;
pub mod preset_picker;
pub mod terminal_editor;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::Command;
use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::runtime::Msg;
use crate::ui::Ui;
use crate::ui::dialog::{Dialog, DialogResult};

use self::app_theme_editor::AppThemeEditor;
use self::fonts_tab::FontsTab;
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
    FontsTab,
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
    /// Re-derive the UI palette from these colours and show `message`.
    RefreshPaletteAndStatus(TerminalColors, String),
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
    /// The `termux.properties` editor.
    AppTheme(Box<AppThemeEditor>),
    FontsTab(FontsTab),
}

impl Screen {
    /// Build the screen of `kind`, if it exists yet.
    pub fn new(kind: ScreenKind, paths: &Paths) -> Option<Self> {
        match kind {
            ScreenKind::Main => Some(Self::Main(MainMenu::new(paths))),
            ScreenKind::Presets => Some(Self::Presets(PresetPicker::new(paths))),
            ScreenKind::Terminal => Some(Self::Terminal(Box::new(TerminalEditor::new(paths)))),
            ScreenKind::AppTheme => Some(Self::AppTheme(Box::new(AppThemeEditor::new(paths)))),
            ScreenKind::FontsTab => Some(Self::FontsTab(FontsTab::new(paths))),
        }
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(screen) => screen.handle_key(key, paths),
            Self::Presets(screen) => screen.handle_key(key, paths),
            Self::Terminal(screen) => screen.handle_key(key),
            Self::AppTheme(screen) => screen.handle_key(key),
            Self::FontsTab(screen) => screen.handle_key(key, paths),
        }
    }

    /// Re-read externally mutable data (for example the active theme) after
    /// the screen is revealed again.
    pub fn refresh(&mut self, paths: &Paths) {
        if let Self::Main(screen) = self {
            screen.refresh(paths);
        }
    }

    /// The section name shown in the breadcrumb header, or `""` at the menu.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Main(_) => "",
            Self::Presets(_) => "Preset Themes",
            Self::Terminal(_) => "Terminal Colors",
            Self::AppTheme(_) => "App Theme",
            Self::FontsTab(_) => "Fonts",
        }
    }

    /// The key bindings the help bar shows for the current screen.
    pub fn help(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Self::Main(screen) => screen.help(),
            Self::Presets(screen) => screen.help(),
            Self::Terminal(screen) => screen.help(),
            Self::AppTheme(screen) => screen.help(),
            Self::FontsTab(screen) => screen.help(),
        }
    }

    /// The action a freshly pushed screen wants the app to apply.
    pub fn opened(&mut self, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(_) => ScreenAction::None,
            Self::Presets(screen) => screen.opened(paths),
            Self::Terminal(screen) => screen.opened(),
            Self::AppTheme(_) => ScreenAction::None,
            Self::FontsTab(screen) => screen.opened(paths),
        }
    }

    /// Run a save or revert command from the keyboard.
    pub fn command(&mut self, command: Command, paths: &Paths) -> ScreenAction {
        match self {
            Self::Terminal(screen) => screen.command(command, paths),
            Self::AppTheme(screen) => screen.command(command, paths),
            Self::Main(_) | Self::Presets(_) => ScreenAction::None,
            Self::FontsTab(screen) => screen.command(command, paths),
        }
    }

    /// Handle the result of a dialog this screen opened.
    pub fn on_dialog(&mut self, result: DialogResult, paths: &Paths) -> ScreenAction {
        match self {
            Self::Main(screen) => screen.on_dialog(result, paths),
            Self::Presets(_) => ScreenAction::None,
            Self::Terminal(screen) => screen.on_dialog(result, paths),
            Self::AppTheme(screen) => screen.on_dialog(result, paths),
            Self::FontsTab(screen) => screen.on_dialog(result, paths),
        }
    }

    /// Restore any uncommitted state before the application quits.
    pub fn on_quit(&mut self, paths: &Paths) -> ScreenAction {
        match self {
            Self::FontsTab(screen) => screen.on_quit(paths),
            _ => ScreenAction::None,
        }
    }

    /// Re-derive geometry after a resize.
    pub fn on_resize(&mut self, _area: Rect) {}

    /// Handle a worker message, if this screen cares.
    pub fn on_msg(&mut self, msg: &Msg, paths: &Paths) -> ScreenAction {
        match self {
            Self::FontsTab(screen) => screen.on_msg(msg, paths),
            _ => ScreenAction::None,
        }
    }

    /// Draw the screen into `area`.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, ui: &Ui) {
        match self {
            Self::Main(screen) => screen.render(frame, area, ui),
            Self::Presets(screen) => screen.render(frame, area, ui),
            Self::Terminal(screen) => screen.render(frame, area, ui),
            Self::AppTheme(screen) => screen.render(frame, area, ui),
            Self::FontsTab(screen) => screen.render(frame, area, ui),
        }
    }
}
