//! Browse the theme library and activate a theme.

use std::collections::HashMap;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::library::{ThemeRef, active_theme_path, scan_themes};
use crate::model::colors::TerminalColors;
use crate::paths::Paths;
use crate::presets::materialize;
use crate::screens::ScreenAction;
use crate::theme::Palette;
use crate::ui::focus::{Move, list_move};
use crate::ui::input::Field;

/// Rows a page key jumps in the theme list.
const PAGE: usize = 10;

/// Which region of the picker has the keyboard.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Focus {
    List,
    Filter,
}

/// The theme picker's state.
pub struct PresetPicker {
    /// Every theme found in the library.
    pub all: Vec<ThemeRef>,
    /// Indices into [`PresetPicker::all`] that match the filter, in list order.
    pub visible: Vec<usize>,
    /// The filter text.
    pub filter: Field,
    /// The highlighted row.
    pub list: ListState,
    /// Parsed colours per theme path.
    pub cache: HashMap<PathBuf, TerminalColors>,
    /// The theme file `colors.properties` currently resolves to.
    pub active: Option<PathBuf>,
    focus: Focus,
}

/// Scan the library, seeding the built-in themes when it is empty.
pub fn load_themes(paths: &Paths) -> Vec<ThemeRef> {
    let root = paths.colors_root();
    let mut themes = scan_themes(&root);
    if themes.is_empty() {
        let _ = materialize(&root.join("builtin"));
        themes = scan_themes(&root);
    }
    themes
}

impl PresetPicker {
    /// Build the picker for `paths`, selecting the first theme.
    pub fn new(paths: &Paths) -> Self {
        let all = load_themes(paths);
        let visible: Vec<usize> = (0..all.len()).collect();
        let list = if visible.is_empty() {
            ListState::default()
        } else {
            ListState::default().with_selected(Some(0))
        };
        Self {
            active: active_theme_path(&paths.colors_properties()),
            all,
            visible,
            filter: Field::new(""),
            list,
            cache: HashMap::new(),
            focus: Focus::List,
        }
    }

    /// The theme under the highlighted row.
    pub fn selected(&self) -> Option<&ThemeRef> {
        let row = self.list.selected()?;
        self.all.get(*self.visible.get(row)?)
    }

    /// Keep only the themes whose label contains `needle`, ignoring case.
    ///
    /// The selection follows the theme it was on when that theme survives the
    /// filter; otherwise it moves to the first row.
    pub fn apply_filter(&mut self, needle: &str) {
        let previous = self.selected().map(|theme| theme.path.clone());
        let needle = needle.to_lowercase();
        self.visible = (0..self.all.len())
            .filter(|&index| {
                self.all
                    .get(index)
                    .is_some_and(|theme| theme.label().to_lowercase().contains(&needle))
            })
            .collect();
        let keep = previous.and_then(|path| {
            self.visible
                .iter()
                .position(|&index| self.all.get(index).is_some_and(|theme| theme.path == path))
        });
        self.list
            .select(keep.or_else(|| (!self.visible.is_empty()).then_some(0)));
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent, _paths: &Paths) -> ScreenAction {
        match key.code {
            KeyCode::Esc => return ScreenAction::Pop,
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = if self.focus == Focus::List {
                    Focus::Filter
                } else {
                    Focus::List
                };
                return ScreenAction::None;
            }
            _ => {}
        }
        match self.focus {
            Focus::List => self.handle_list_key(key),
            Focus::Filter => self.handle_filter_key(key),
        }
    }

    /// Handle a key press while the theme list has focus.
    fn handle_list_key(&mut self, key: KeyEvent) -> ScreenAction {
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
            list_move(&mut self.list, self.visible.len(), movement, PAGE);
        }
        ScreenAction::None
    }

    /// Handle a key press while the filter has focus.
    fn handle_filter_key(&mut self, key: KeyEvent) -> ScreenAction {
        if key.code == KeyCode::Enter {
            self.focus = Focus::List;
            return ScreenAction::None;
        }
        if self.filter.handle_key(key) {
            let needle = self.filter.value().to_owned();
            self.apply_filter(&needle);
        }
        ScreenAction::None
    }

    /// Whether `theme` is the one currently linked from `colors.properties`.
    fn is_active(&self, theme: &ThemeRef) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| std::fs::canonicalize(&theme.path).ok().as_ref() == Some(active))
    }

    /// Draw the filter and the theme list.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(palette.background)
                    .fg(palette.foreground),
            ),
            area,
        );
        let [list_area, _preview_area] =
            Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
                .areas(area);
        self.render_list(frame, list_area, palette);
    }

    /// Draw the bordered filter-plus-list pane.
    fn render_list(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        let block = Block::bordered()
            .title(format!("themes ({})", self.visible.len()))
            .border_style(Style::default().fg(palette.primary))
            .style(Style::default().bg(palette.surface));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let [filter_area, list_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);

        self.filter.set_width(filter_area.width as usize);
        let filter_style = if self.focus == Focus::Filter {
            Style::default().bg(palette.boost).fg(palette.foreground)
        } else {
            Style::default().fg(palette.muted)
        };
        let text = if self.filter.value().is_empty() {
            "filter themes…"
        } else {
            self.filter.value()
        };
        frame.render_widget(Paragraph::new(text).style(filter_style), filter_area);

        let items: Vec<ListItem> = if self.visible.is_empty() {
            vec![ListItem::new(
                Line::from("No themes found.").style(Style::default().fg(palette.muted)),
            )]
        } else {
            self.visible
                .iter()
                .filter_map(|&index| self.all.get(index))
                .map(|theme| {
                    let mut spans = vec![Span::raw(theme.label())];
                    if self.is_active(theme) {
                        spans.push(Span::styled(
                            "  ✓ active",
                            Style::default()
                                .fg(palette.success)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    ListItem::new(Line::from(spans))
                })
                .collect()
        };
        let list = List::new(items)
            .highlight_symbol("› ")
            .highlight_style(
                Style::default()
                    .bg(palette.boost)
                    .fg(palette.foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(palette.surface));
        frame.render_stateful_widget(list, list_area, &mut self.list);
    }
}
