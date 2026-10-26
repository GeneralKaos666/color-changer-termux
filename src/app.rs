//! The application: screen stack, palette and status line.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::chrome;
use crate::library::active_colors;
use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::screens::{Screen, ScreenAction, ScreenKind};
use crate::theme::Palette;
use crate::ui::dialog::{Dialog, DialogResult};

/// Smallest terminal width the screens are drawn at.
pub const MIN_WIDTH: u16 = 40;

/// Smallest terminal height the screens are drawn at.
pub const MIN_HEIGHT: u16 = 10;

/// How long a status message stays on screen.
const STATUS_TTL: Duration = Duration::from_secs(4);

/// A save or revert request, usually mapped from a `Ctrl` chord.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Save the current editor to its theme.
    Save,
    /// Save the current editor under a new name.
    SaveAs,
    /// Reload the editor from its theme.
    Revert,
}

/// Everything the running application owns.
pub struct App {
    paths: Paths,
    screens: Vec<Screen>,
    dialog: Option<Dialog>,
    colors: TerminalColors,
    palette: Palette,
    status: Option<(String, Instant)>,
    too_small: bool,
    quit: bool,
}

impl App {
    /// Build the app on the main menu.
    pub fn new(paths: Paths) -> Self {
        let colors = active_colors(&paths);
        let Some(main) = Screen::new(ScreenKind::Main, &paths) else {
            unreachable!("the main menu always exists");
        };
        Self {
            palette: Palette::from_colors(&colors),
            colors,
            screens: vec![main],
            dialog: None,
            status: None,
            too_small: false,
            quit: false,
            paths,
        }
    }

    /// Handle a key press from the event loop.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q'))
        {
            self.quit = true;
            return;
        }
        if self.too_small {
            if key.code == KeyCode::Esc {
                self.quit = true;
            }
            return;
        }
        if let Some(dialog) = self.dialog.as_mut() {
            if let Some(result) = dialog.handle_key(key) {
                self.resolve_dialog(result);
            }
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let command = match key.code {
                KeyCode::Char('s') => Some(Command::Save),
                KeyCode::Char('a') => Some(Command::SaveAs),
                KeyCode::Char('r') => Some(Command::Revert),
                _ => None,
            };
            if let Some(command) = command {
                self.command(command);
                return;
            }
        }
        let Some(screen) = self.screens.last_mut() else {
            return;
        };
        let action = screen.handle_key(key, &self.paths);
        self.apply(action);
    }

    /// Run a save or revert command against the current screen.
    pub fn command(&mut self, command: Command) {
        let action = match self.screens.last_mut() {
            Some(screen) => screen.command(command, &self.paths),
            None => ScreenAction::None,
        };
        self.apply(action);
    }

    /// The dialog currently on screen, if any.
    pub fn dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    /// Show `dialog` over the current screen.
    pub fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
    }

    /// Close the current dialog and deliver `result` to the screen that asked.
    pub fn resolve_dialog(&mut self, result: DialogResult) {
        self.dialog = None;
        let Some(screen) = self.screens.last_mut() else {
            return;
        };
        let action = screen.on_dialog(result, &self.paths);
        self.apply(action);
    }

    /// Draw the current screen and, when set, the status line.
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.too_small = area.width < MIN_WIDTH || area.height < MIN_HEIGHT;
        if self.too_small {
            self.render_too_small(frame, area);
            return;
        }
        let [header_area, content_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);
        self.render_header(frame, header_area);
        if let Some(screen) = self.screens.last_mut() {
            screen.render(frame, content_area, &self.palette);
        }
        self.render_footer(frame, footer_area);
        if let Some(dialog) = &self.dialog {
            let palette = self.palette.clone();
            let buffer = frame.buffer_mut();
            dialog.render(area, buffer, &palette);
        }
    }

    /// Draw the one-row header: the gradient title and, on sub-screens, the
    /// breadcrumb section.
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let mut spans = chrome::gradient_title("termux-settings", self.palette.dark);
        if self.screens.len() > 1 {
            spans.push(Span::styled(" › ", Style::default().fg(self.palette.muted)));
            spans.push(Span::styled(
                self.current().title(),
                Style::default()
                    .fg(chrome::accent(self.palette.dark))
                    .add_modifier(Modifier::BOLD),
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(self.palette.panel)),
            area,
        );
    }

    /// Draw the bottom row: the status message when set, otherwise the
    /// context-sensitive help bar.
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        if let Some((message, _)) = &self.status {
            frame.render_widget(
                Paragraph::new(message.as_str())
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .bg(self.palette.panel)
                            .fg(self.palette.foreground),
                    ),
                area,
            );
        } else {
            let line = chrome::help_bar(
                &self.current().help(),
                chrome::accent(self.palette.dark),
                self.palette.muted,
            );
            frame.render_widget(
                Paragraph::new(line).style(Style::default().bg(self.palette.panel)),
                area,
            );
        }
    }

    /// Advance time-driven state: expire a stale status message.
    pub fn tick(&mut self, now: Instant) {
        if let Some((_, shown)) = &self.status
            && now.duration_since(*shown) >= STATUS_TTL
        {
            self.status = None;
        }
    }

    /// Draw the guard shown when the terminal is below [`MIN_WIDTH`] by
    /// [`MIN_HEIGHT`].
    fn render_too_small(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(self.palette.background)
                    .fg(self.palette.foreground),
            ),
            area,
        );
        let row = Rect {
            x: area.x,
            y: area.y + area.height / 2,
            width: area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(format!(
                "terminal too small ({MIN_WIDTH}×{MIN_HEIGHT} minimum)"
            ))
            .alignment(Alignment::Center)
            .style(Style::default().fg(self.palette.warning)),
            row,
        );
    }

    /// The screen on top of the stack.
    pub fn current(&self) -> &Screen {
        self.screens
            .last()
            .expect("the screen stack is never empty")
    }

    /// The screen on top of the stack, mutably.
    pub fn current_mut(&mut self) -> &mut Screen {
        self.screens
            .last_mut()
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

    /// Show `message` in the status line for [`STATUS_TTL`].
    pub fn set_status(&mut self, message: &str) {
        self.status = Some((message.to_owned(), Instant::now()));
    }

    /// Apply the action a screen returned.
    fn apply(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Quit => self.quit = true,
            ScreenAction::Push(kind) => {
                if let Some(mut screen) = Screen::new(kind, &self.paths) {
                    let action = screen.opened(&self.paths);
                    self.screens.push(screen);
                    self.apply(action);
                }
            }
            ScreenAction::Pop => {
                if self.screens.len() > 1 {
                    self.screens.pop();
                }
                // Refresh the revealed screen (e.g. the menu's active-theme
                // name after a change was applied elsewhere).
                if let Some(screen) = self.screens.last_mut() {
                    screen.refresh(&self.paths);
                }
            }
            ScreenAction::RefreshPalette(colors) => {
                self.colors = colors;
                self.palette = Palette::from_colors(&self.colors);
            }
            ScreenAction::Status(message) => self.set_status(&message),
            ScreenAction::RefreshPaletteAndStatus(colors, message) => {
                self.colors = colors;
                self.palette = Palette::from_colors(&self.colors);
                self.set_status(&message);
            }
            ScreenAction::OpenDialog(dialog) => self.open_dialog(dialog),
        }
    }
}
