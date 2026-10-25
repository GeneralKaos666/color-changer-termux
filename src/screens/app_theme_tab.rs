use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::Command;
use crate::screens::ScreenAction;
use crate::paths::Paths;
use crate::theme::Palette;
use crate::ui::dialog::DialogResult;

pub struct AppThemeTab {
    pub inner: crate::screens::app_theme_editor::AppThemeEditor,
}

impl AppThemeTab {
    pub fn new(paths: &Paths) -> Self {
        Self {
            inner: crate::screens::app_theme_editor::AppThemeEditor::new(paths),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, paths: &Paths) -> ScreenAction {
        self.inner.handle_key(key)
    }

    pub fn opened(&mut self, _paths: &Paths) -> ScreenAction { ScreenAction::None }
    pub fn command(&mut self, command: Command, paths: &Paths) -> ScreenAction { self.inner.command(command, paths) }
    pub fn on_dialog(&mut self, result: DialogResult, paths: &Paths) -> ScreenAction { self.inner.on_dialog(result, paths) }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        self.inner.render(frame, area, palette);
    }
}
