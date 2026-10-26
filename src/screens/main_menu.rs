//! The landing screen: hero, status and a four-item menu.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::chrome;
use crate::library::{active_colors, active_theme_path};
use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::screens::{ScreenAction, ScreenKind};
use crate::ui::dialog::DialogResult;
use crate::ui::focus::{Move, list_move};
use crate::ui::text;
use crate::ui::{Ui, pane};

/// The menu entries, in display order.
const ITEMS: [(&str, ScreenKind); 5] = [
    ("Preset Themes", ScreenKind::Presets),
    ("Terminal Colors", ScreenKind::Terminal),
    ("App Theme", ScreenKind::AppTheme),
    ("Fonts", ScreenKind::FontsTab),
    ("Quit", ScreenKind::Main),
];

/// Rows the bordered menu box needs: items plus its two borders.
const MENU_HEIGHT: u16 = 7;

/// The landing menu.
pub struct MainMenu {
    /// Which menu entry is highlighted.
    pub list: ListState,
    /// The active theme's colours, captured once and refreshed on return.
    colors: TerminalColors,
    /// The active theme's library-relative name.
    active_label: String,
}

impl MainMenu {
    /// Build the menu for the given paths.
    pub fn new(paths: &Paths) -> Self {
        Self {
            list: ListState::default().with_selected(Some(0)),
            colors: active_colors(paths),
            active_label: active_name(paths),
        }
    }

    /// Re-read the active theme after a change made on another screen.
    pub fn reload(&mut self, paths: &Paths) {
        self.colors = active_colors(paths);
        self.active_label = active_name(paths);
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent, _paths: &Paths) -> ScreenAction {
        let len = ITEMS.len();
        match key.code {
            KeyCode::Esc => ScreenAction::Quit,
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                list_move(&mut self.list, len, Move::Up, len);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                list_move(&mut self.list, len, Move::Down, len);
                ScreenAction::None
            }
            KeyCode::Home => {
                list_move(&mut self.list, len, Move::Home, len);
                ScreenAction::None
            }
            KeyCode::End => {
                list_move(&mut self.list, len, Move::End, len);
                ScreenAction::None
            }
            KeyCode::PageUp => {
                list_move(&mut self.list, len, Move::PageUp, len);
                ScreenAction::None
            }
            KeyCode::PageDown => {
                list_move(&mut self.list, len, Move::PageDown, len);
                ScreenAction::None
            }
            KeyCode::Enter => match self.list.selected().unwrap_or(0) {
                4 => ScreenAction::Quit,
                index => ScreenAction::Push(ITEMS[index].1),
            },
            _ => ScreenAction::None,
        }
    }

    /// Handle the result of a dialog. The menu opens none.
    pub fn on_dialog(&mut self, _result: DialogResult, _paths: &Paths) -> ScreenAction {
        ScreenAction::None
    }

    /// The key bindings the help bar shows for this screen.
    pub fn help(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("↑/↓", "move"),
            ("Enter", "open"),
            ("Esc", "quit"),
            ("Ctrl+C", "quit"),
        ]
    }

    /// Draw the hero, status line and menu box.
    pub fn render(&self, frame: &mut Frame, area: Rect, ui: &Ui) {
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(ui.palette.background)
                    .fg(ui.palette.foreground),
            ),
            area,
        );

        let header = vec![
            Line::from("change terminal & app colours, live")
                .style(Style::default().fg(ui.palette.muted))
                .alignment(Alignment::Center),
            Line::from(""),
            text::palette_strip(&self.colors).alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("active theme  ", Style::default().fg(ui.palette.muted)),
                Span::styled(
                    self.active_label.clone(),
                    Style::default()
                        .fg(ui.palette.primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
            .alignment(Alignment::Center),
        ];

        let header_height = (header.len() as u16).min(area.height.saturating_sub(MENU_HEIGHT));
        let [header_area, menu_area] = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Min(MENU_HEIGHT),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new(header).style(Style::default().bg(ui.palette.background)),
            header_area,
        );

        let menu_width = 62.min(area.width);
        let menu_rect = Rect {
            x: area.x + area.width.saturating_sub(menu_width) / 2,
            y: menu_area.y,
            width: menu_width,
            height: menu_area.height.min(MENU_HEIGHT),
        };
        let items: Vec<ListItem> = ITEMS
            .iter()
            .map(|(label, _)| ListItem::new(Line::from(format!("  {label}"))))
            .collect();
        let list = List::new(items)
            .block(pane(ui, " menu"))
            .highlight_symbol(ui.glyphs.marker())
            .highlight_style(
                Style::default()
                    .bg(chrome::accent(ui.palette.dark))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
        let mut list_state = self.list.clone();
        frame.render_stateful_widget(list, menu_rect, &mut list_state);
    }
}

/// The active theme's library-relative name, or `default`.
fn active_name(paths: &Paths) -> String {
    let Some(path) = active_theme_path(&paths.colors_properties()) else {
        return "default".to_owned();
    };
    match path.strip_prefix(paths.colors_root()) {
        Ok(relative) => relative.with_extension("").to_string_lossy().into_owned(),
        Err(_) => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
    }
}
