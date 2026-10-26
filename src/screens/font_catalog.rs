//! Browse and download Nerd Fonts.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::app::Command;
use crate::chrome;
use crate::fonts::downloader;
use crate::paths::Paths;
use crate::runtime::Msg;
use crate::screens::ScreenAction;
use crate::ui::dialog::DialogResult;
use crate::ui::focus::{Move, list_move};
use crate::ui::input::Field;
use crate::ui::{Ui, pane};

/// The Nerd Font download screen.
pub struct FontCatalog {
    entries: Vec<(String, String)>,
    visible: Vec<usize>,
    filter: Field,
    show_filter: bool,
    state: ListState,
    progress: Option<(String, u64, Option<u64>)>,
}

impl FontCatalog {
    pub fn new(_paths: &Paths) -> Self {
        let entries = downloader::sorted_catalog();
        let visible: Vec<usize> = (0..entries.len()).collect();
        let mut state = ListState::default();
        if !visible.is_empty() {
            state.select(Some(0));
        }
        Self {
            entries,
            visible,
            filter: Field::new(""),
            show_filter: false,
            state,
            progress: None,
        }
    }

    fn apply_filter(&mut self, needle: &str) {
        let needle = needle.to_lowercase();
        self.visible = (0..self.entries.len())
            .filter(|&i| self.entries[i].0.to_lowercase().contains(&needle))
            .collect();
        if let Some(selected) = self.state.selected() {
            if selected >= self.visible.len() {
                self.state.select((!self.visible.is_empty()).then_some(0));
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, _paths: &Paths) -> ScreenAction {
        if self.show_filter {
            match key.code {
                KeyCode::Esc => {
                    self.show_filter = false;
                    self.filter.set_value("");
                    self.apply_filter("");
                    return ScreenAction::None;
                }
                KeyCode::Enter => {
                    self.show_filter = false;
                    return ScreenAction::None;
                }
                _ => {
                    if self.filter.handle_key(key) {
                        let needle = self.filter.value().to_owned();
                        self.apply_filter(&needle);
                    }
                    return ScreenAction::None;
                }
            }
        }
        match key.code {
            KeyCode::Esc => ScreenAction::Pop,
            KeyCode::Char('/') => {
                self.show_filter = true;
                ScreenAction::None
            }
            KeyCode::Enter => {
                if let Some(row) = self.state.selected() {
                    if let Some(&index) = self.visible.get(row) {
                        if let Some((name, url)) = self.entries.get(index) {
                            return ScreenAction::SpawnDownload {
                                name: name.clone(),
                                url: url.clone(),
                            };
                        }
                    }
                }
                ScreenAction::None
            }
            _ => {
                let movement = match key.code {
                    KeyCode::Up | KeyCode::Char('k') => Some(Move::Up),
                    KeyCode::Down | KeyCode::Char('j') => Some(Move::Down),
                    KeyCode::Home => Some(Move::Home),
                    KeyCode::End => Some(Move::End),
                    KeyCode::PageUp => Some(Move::PageUp),
                    KeyCode::PageDown => Some(Move::PageDown),
                    _ => None,
                };
                if let Some(movement) = movement {
                    list_move(&mut self.state, self.visible.len(), movement, 10);
                }
                ScreenAction::None
            }
        }
    }

    pub fn on_msg(&mut self, msg: &Msg, _paths: &Paths) -> ScreenAction {
        match msg {
            Msg::DownloadProgress {
                name,
                received,
                total,
            } => {
                self.progress = Some((name.clone(), *received, *total));
                ScreenAction::None
            }
            Msg::Downloaded(Ok(_)) => {
                self.progress = None;
                ScreenAction::status("Font downloaded to the library")
            }
            Msg::Downloaded(Err(error)) => {
                self.progress = None;
                ScreenAction::status(&format!("Download failed: {error}"))
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

    pub fn help(&self) -> Vec<(&'static str, &'static str)> {
        if self.show_filter {
            vec![("type", "filter"), ("Enter", "done"), ("Esc", "clear")]
        } else {
            vec![
                ("↑/↓", "move"),
                ("Enter", "download"),
                ("/", "filter"),
                ("Esc", "back"),
            ]
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, ui: &Ui) {
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(ui.palette.background)
                    .fg(ui.palette.foreground),
            ),
            area,
        );
        let [list_area, footer_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

        let title = format!(" Nerd Fonts ({})", self.visible.len());
        let block = pane(ui, &title);
        let inner = block.inner(list_area);
        frame.render_widget(block, list_area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let [filter_area, list_rows] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);

        let filter_style = if self.show_filter {
            Style::default()
                .bg(ui.palette.boost)
                .fg(ui.palette.foreground)
        } else {
            Style::default().fg(ui.palette.muted)
        };
        let filter_text = if self.filter.value().is_empty() {
            format!("{}filter fonts", ui.glyphs.filter())
        } else {
            self.filter.value().to_owned()
        };
        frame.render_widget(Paragraph::new(filter_text).style(filter_style), filter_area);

        let items: Vec<ListItem> = if self.visible.is_empty() {
            vec![ListItem::new(
                Line::from("No matches.").style(Style::default().fg(ui.palette.muted)),
            )]
        } else {
            self.visible
                .iter()
                .filter_map(|&index| self.entries.get(index))
                .map(|(name, _)| ListItem::new(Line::from(name.as_str())))
                .collect()
        };
        let list = List::new(items)
            .highlight_symbol(ui.glyphs.marker())
            .highlight_style(
                Style::default()
                    .bg(chrome::accent(ui.palette.dark))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(ui.palette.surface));
        let mut list_state = self.state.clone();
        frame.render_stateful_widget(list, list_rows, &mut list_state);

        let footer = match &self.progress {
            Some((name, received, Some(total))) if *total > 0 => {
                let pct = received.saturating_mul(100) / total;
                Line::from(Span::styled(
                    format!("{name} {pct}%"),
                    Style::default().fg(ui.palette.muted),
                ))
            }
            Some((name, received, _)) => Line::from(Span::styled(
                format!("{name} {received} bytes"),
                Style::default().fg(ui.palette.muted),
            )),
            None => Line::from(Span::styled(
                "Enter downloads the selected font into ~/.termux/fonts",
                Style::default().fg(ui.palette.muted),
            )),
        };
        frame.render_widget(Paragraph::new(footer), footer_area);
    }
}
