use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::Command;
use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::screens::ScreenAction;
use crate::theme::Palette;
use crate::ui::dialog::DialogResult;
use crate::ui::text::{banner, palette_strip};

#[derive(Clone, Debug)]
pub struct ColorsTab {
    state: ListState,
    items: Vec<String>,
}

impl ColorsTab {
    pub fn new(_paths: &Paths) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            items: vec!["Browse Presets".into(), "Edit Terminal Colors".into()],
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, _paths: &Paths) -> ScreenAction {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.state.selected().unwrap_or(0);
                self.state.select(Some((i + 1).min(self.items.len() - 1)));
                ScreenAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                ScreenAction::None
            }
            KeyCode::Enter => {
                let i = self.state.selected().unwrap_or(0);
                if i == 0 {
                    ScreenAction::Push(crate::screens::ScreenKind::Presets)
                } else if i == 1 {
                    ScreenAction::Push(crate::screens::ScreenKind::Terminal)
                } else {
                    ScreenAction::None
                }
            }
            _ => ScreenAction::None,
        }
    }

    pub fn opened(&mut self, _paths: &Paths) -> ScreenAction {
        ScreenAction::None
    }
    pub fn command(&mut self, _command: Command, _paths: &Paths) -> ScreenAction {
        ScreenAction::None
    }
    pub fn on_dialog(&mut self, _result: DialogResult, _paths: &Paths) -> ScreenAction {
        ScreenAction::None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        let colors = TerminalColors {
            background: crate::model::colors::Rgb::new(0, 0, 0),
            foreground: crate::model::colors::Rgb::new(255, 255, 255),
            cursor: crate::model::colors::Rgb::new(255, 255, 255),
            palette: [
                crate::model::colors::Rgb::new(0, 0, 0),
                crate::model::colors::Rgb::new(204, 0, 0),
                crate::model::colors::Rgb::new(78, 154, 6),
                crate::model::colors::Rgb::new(196, 160, 0),
                crate::model::colors::Rgb::new(52, 101, 164),
                crate::model::colors::Rgb::new(117, 80, 123),
                crate::model::colors::Rgb::new(6, 152, 154),
                crate::model::colors::Rgb::new(211, 215, 207),
                crate::model::colors::Rgb::new(85, 87, 83),
                crate::model::colors::Rgb::new(239, 41, 41),
                crate::model::colors::Rgb::new(138, 226, 52),
                crate::model::colors::Rgb::new(252, 233, 79),
                crate::model::colors::Rgb::new(114, 159, 207),
                crate::model::colors::Rgb::new(173, 127, 168),
                crate::model::colors::Rgb::new(52, 226, 226),
                crate::model::colors::Rgb::new(238, 238, 236),
            ],
            extra: vec![],
        };
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Min(5),
            ])
            .split(area);
        frame.render_widget(
            Paragraph::new(banner(&colors, area.width, false, false)),
            layout[0],
        );
        frame.render_widget(Paragraph::new("Termux Colours"), layout[1]);
        frame.render_widget(Paragraph::new(palette_strip(&colors)), layout[2]);
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Colors"))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(palette.primary),
            );
        frame.render_stateful_widget(list, layout[3], &mut self.state);
    }
}
