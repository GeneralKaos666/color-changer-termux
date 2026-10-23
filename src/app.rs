//! The application: screen stack, palette and status line.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

use crate::library::active_colors;
use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::screens::{Screen, ScreenAction, ScreenKind};
use crate::theme::Palette;

/// Everything the running application owns.
pub struct App {
    paths: Paths,
    screens: Vec<Screen>,
    colors: TerminalColors,
    palette: Palette,
    status: Option<(String, Instant)>,
    quit: bool,
}

impl App {
    /// Build the app on the main menu, themed from the active colours.
    pub fn new(paths: Paths) -> Self {
        let colors = active_colors(&paths);
        let Some(main) = Screen::new(ScreenKind::Main, &paths) else {
            unreachable!("the main screen always exists");
        };
        Self {
            palette: Palette::from_colors(&colors),
            colors,
            screens: vec![main],
            status: None,
            quit: false,
            paths,
        }
    }

    /// Handle a key press from the event loop.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        let Some(screen) = self.screens.last_mut() else {
            return;
        };
        let action = screen.handle_key(key, &self.paths);
        self.apply(action);
    }

    /// Draw the current screen and the status line.
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        if let Some(screen) = self.screens.last_mut() {
            screen.render(frame, area, &self.palette);
        }
        if let Some((message, _)) = &self.status {
            let row = Rect {
                x: area.x,
                y: area.y + area.height.saturating_sub(1),
                width: area.width,
                height: u16::from(area.height > 0),
            };
            frame.render_widget(
                Paragraph::new(message.as_str())
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .bg(self.palette.panel)
                            .fg(self.palette.muted),
                    ),
                row,
            );
        }
    }

    /// Advance time-driven state.
    pub fn tick(&mut self, _now: Instant) {}

    /// The screen on top of the stack.
    pub fn current(&self) -> &Screen {
        self.screens
            .last()
            .expect("the screen stack is never empty")
    }

    /// The derived UI palette.
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Whether the event loop should stop.
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Show `message` in the status line.
    fn set_status(&mut self, message: &str) {
        self.status = Some((message.to_owned(), Instant::now()));
    }

    /// Apply the action a screen returned.
    fn apply(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Quit => self.quit = true,
            ScreenAction::Push(kind) => {
                if let Some(screen) = Screen::new(kind, &self.paths) {
                    self.screens.push(screen);
                }
            }
            ScreenAction::Pop => {
                if self.screens.len() > 1 {
                    self.screens.pop();
                }
            }
            ScreenAction::RefreshPalette(colors) => {
                self.colors = colors;
                self.palette = Palette::from_colors(&self.colors);
            }
            ScreenAction::Status(message) => self.set_status(&message),
        }
    }
}
