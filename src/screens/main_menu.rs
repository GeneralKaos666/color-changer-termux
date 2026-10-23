//! The landing screen: banner, status and a four-item menu.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::library::{active_colors, active_theme_path};
use crate::paths::Paths;
use crate::screens::{ScreenAction, ScreenKind};
use crate::theme::Palette;
use crate::ui::focus::{Move, list_move};
use crate::ui::text;

/// The menu entries, in display order.
const ITEMS: [(&str, ScreenKind); 4] = [
    ("Preset Themes", ScreenKind::Presets),
    ("Terminal Colors", ScreenKind::Terminal),
    ("App Theme", ScreenKind::AppTheme),
    ("Quit", ScreenKind::Main),
];

/// Rows the bordered menu box needs: four items plus its two borders.
const MENU_HEIGHT: u16 = 6;

/// The landing menu.
pub struct MainMenu {
    /// Which menu entry is highlighted.
    pub list: ListState,
    paths: Paths,
}

impl MainMenu {
    /// Build the menu for the given paths.
    pub fn new(paths: &Paths) -> Self {
        Self {
            list: ListState::default().with_selected(Some(0)),
            paths: paths.clone(),
        }
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent, _paths: &Paths) -> ScreenAction {
        let len = ITEMS.len();
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ScreenAction::Quit,
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
                3 => ScreenAction::Quit,
                index => ScreenAction::Push(ITEMS[index].1),
            },
            _ => ScreenAction::None,
        }
    }

    /// Draw the banner, status line and menu box.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        let colors = active_colors(&self.paths);
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(palette.background)
                    .fg(palette.foreground),
            ),
            area,
        );

        let (stacked, compact) = banner_mode(area.width, area.height);
        let mut header = text::banner(&colors, area.width, stacked, compact);
        header.push(
            Line::from("change terminal & app colours, live")
                .style(Style::default().fg(palette.muted))
                .alignment(Alignment::Center),
        );
        header.push(Line::from(""));
        header.push(text::palette_strip(&colors).alignment(Alignment::Center));
        header.push(Line::from(""));
        header.push(
            Line::from(vec![
                Span::styled("active theme  ", Style::default().fg(palette.muted)),
                Span::styled(
                    active_name(&self.paths),
                    Style::default()
                        .fg(palette.primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
            .alignment(Alignment::Center),
        );

        let header_height = (header.len() as u16).min(area.height.saturating_sub(MENU_HEIGHT));
        let [header_area, menu_area] = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Min(MENU_HEIGHT),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new(header).style(Style::default().bg(palette.background)),
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
        let block = Block::bordered()
            .title(Line::from("menu").alignment(Alignment::Center))
            .border_style(Style::default().fg(palette.primary))
            .style(Style::default().bg(palette.surface));
        let list = List::new(items)
            .block(block)
            .highlight_symbol("› ")
            .highlight_style(
                Style::default()
                    .bg(palette.boost)
                    .fg(palette.foreground)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, menu_rect, &mut self.list);
    }
}

/// Which banner layout fits a terminal of `width` x `height`.
///
/// The thresholds match the Python authority: one row when there is room for
/// both words, two stacked rows on a tall narrow terminal, and the compact
/// wordmark otherwise.
fn banner_mode(width: u16, height: u16) -> (bool, bool) {
    if width >= 88 && height >= 14 {
        (false, false)
    } else if width >= 40 && height >= 24 {
        (true, false)
    } else {
        (false, true)
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
