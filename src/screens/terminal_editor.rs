//! Edit the individual colours of the active theme.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::library::active_theme_path;
use crate::model::colors::{ColorField, PALETTE_SIZE, Rgb, TerminalColors};
use crate::paths::Paths;
use crate::persist::colors_io::read_colors;
use crate::screens::ScreenAction;
use crate::theme::Palette;
use crate::ui::form;
use crate::ui::input::Field;
use crate::ui::text;

/// Number of editable fields: background, foreground, cursor and the palette.
pub const FIELD_COUNT: usize = 3 + PALETTE_SIZE;

/// Form rows the editor assumes are visible until the first render measures
/// the pane. Small on purpose: the focused row must scroll into view even
/// when nothing has been drawn yet.
const DEFAULT_VISIBLE_ROWS: usize = 12;

/// Row holding the `basics` section rule.
const BASICS_ROW: usize = 0;

/// Row holding the `ansi palette` section rule.
const ANSI_ROW: usize = BASICS_ROW + 1 + 3;

/// Total rows the form occupies: two rules plus one row per field.
const FORM_ROWS: usize = ANSI_ROW + 1 + PALETTE_SIZE;

/// How many mock-session lines the preview shows.
const PREVIEW_LINES: usize = 7;

/// The terminal colour editor's state.
pub struct TerminalEditor {
    /// One hex field per [`ColorField`], in editor order.
    pub fields: [Field; FIELD_COUNT],
    /// Whether each field currently holds a parseable colour.
    pub valid: [bool; FIELD_COUNT],
    /// Index into [`TerminalEditor::fields`] with the keyboard.
    pub focus: usize,
    /// First form row drawn, so the focused field stays visible.
    pub scroll: usize,
    /// The theme file saves write to, when a theme is active.
    pub target: Option<PathBuf>,
    /// The colours the fields were loaded from.
    pub base: TerminalColors,
    visible_rows: usize,
}

impl TerminalEditor {
    /// Load the fields from the active theme, or from the defaults when no
    /// theme is linked.
    pub fn new(paths: &Paths) -> Self {
        let target = active_theme_path(&paths.colors_properties());
        let base = match &target {
            Some(path) => read_colors(path).unwrap_or_default(),
            None => TerminalColors::default(),
        };
        let fields =
            std::array::from_fn(|index| Field::new(&base.field(ColorField::ALL[index]).to_hex()));
        Self {
            valid: [true; FIELD_COUNT],
            fields,
            focus: 0,
            scroll: 0,
            target,
            base,
            visible_rows: DEFAULT_VISIBLE_ROWS,
        }
    }

    /// The action the editor emits when it is first opened: preview the
    /// colours it loaded.
    pub fn opened(&mut self) -> ScreenAction {
        ScreenAction::RefreshPalette(self.current_colors())
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> ScreenAction {
        match key.code {
            KeyCode::Esc => return ScreenAction::Pop,
            KeyCode::Tab | KeyCode::Down => {
                self.focus = (self.focus + 1).min(FIELD_COUNT - 1);
                self.revalidate();
                return ScreenAction::None;
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = self.focus.saturating_sub(1);
                self.revalidate();
                return ScreenAction::None;
            }
            _ => {}
        }
        if self.fields[self.focus].handle_key(key) {
            self.revalidate();
            return ScreenAction::RefreshPalette(self.current_colors());
        }
        ScreenAction::None
    }

    /// Recompute [`TerminalEditor::valid`] and scroll the focused row into view.
    pub fn revalidate(&mut self) {
        for (field, valid) in self.fields.iter().zip(self.valid.iter_mut()) {
            *valid = Rgb::parse(field.value()).is_some();
        }
        self.scroll_to_focus();
    }

    /// The colours the fields describe, ignoring invalid values.
    pub fn current_colors(&self) -> TerminalColors {
        let mut colors = self.base.clone();
        for (index, field) in self.fields.iter().enumerate() {
            if let Some(color) = Rgb::parse(field.value()) {
                colors.set_field(ColorField::ALL[index], color);
            }
        }
        colors
    }

    /// The colours to save, or the keys that hold invalid values.
    pub fn collect(&self) -> Result<TerminalColors, Vec<String>> {
        let invalid: Vec<String> = self
            .fields
            .iter()
            .enumerate()
            .filter(|(_, field)| Rgb::parse(field.value()).is_none())
            .map(|(index, _)| ColorField::ALL[index].key())
            .collect();
        if invalid.is_empty() {
            Ok(self.current_colors())
        } else {
            Err(invalid)
        }
    }

    /// The form row holding field `index`.
    fn row_of(index: usize) -> usize {
        if index < 3 {
            BASICS_ROW + 1 + index
        } else {
            ANSI_ROW + 1 + (index - 3)
        }
    }

    /// The field index drawn at form row `row`, when the row is a field.
    fn index_at(row: usize) -> usize {
        if row < ANSI_ROW {
            row - BASICS_ROW - 1
        } else {
            row - ANSI_ROW - 1 + 3
        }
    }

    /// Move [`TerminalEditor::scroll`] the least amount that keeps the focused
    /// field inside the visible rows.
    fn scroll_to_focus(&mut self) {
        let row = Self::row_of(self.focus);
        if row < self.scroll {
            self.scroll = row;
        } else if row >= self.scroll + self.visible_rows {
            self.scroll = row + 1 - self.visible_rows;
        }
    }

    /// Draw the form, the preview and the action row.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(palette.background)
                    .fg(palette.foreground),
            ),
            area,
        );
        let [body, actions] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);
        let [form_area, preview_area] =
            Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)])
                .areas(body);
        self.render_form(frame, form_area, palette);
        self.render_preview(frame, preview_area, palette);
        render_actions(frame, actions, palette);
    }

    /// Draw the scrolling form of labelled fields.
    fn render_form(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        let block = Block::bordered()
            .title("terminal colors")
            .border_style(Style::default().fg(palette.primary))
            .style(Style::default().bg(palette.surface));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        self.visible_rows = inner.height as usize;
        self.scroll_to_focus();
        let end = (self.scroll + self.visible_rows).min(FORM_ROWS);
        for row in self.scroll..end {
            let row_area = Rect {
                x: inner.x,
                y: inner.y + (row - self.scroll) as u16,
                width: inner.width,
                height: 1,
            };
            if row == BASICS_ROW {
                form::section_rule(frame, row_area, "basics", palette);
            } else if row == ANSI_ROW {
                form::section_rule(frame, row_area, "ansi palette", palette);
            } else {
                let index = Self::index_at(row);
                let label = ColorField::ALL[index].label();
                form::field_row(
                    frame,
                    row_area,
                    &label,
                    &mut self.fields[index],
                    self.focus == index,
                    palette,
                );
            }
        }
    }

    /// Draw the live preview of the colours the fields currently describe.
    fn render_preview(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        let block = Block::bordered()
            .title("preview")
            .border_style(Style::default().fg(palette.secondary))
            .style(Style::default().bg(palette.surface));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let colors = self.current_colors();
        let mut lines = vec![
            Line::from(Span::styled(
                "live preview",
                Style::default()
                    .fg(palette.primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];
        lines.extend(text::mock_session(&colors).into_iter().take(PREVIEW_LINES));
        lines.push(Line::from(""));
        lines.extend(text::swatch_grid(&colors));
        lines.push(Line::from(""));
        lines.push(text::palette_strip(&colors));
        frame.render_widget(Paragraph::new(lines), inner);
    }
}

/// Draw the action row: Save, Save As…, Revert and Back.
fn render_actions(frame: &mut Frame, area: Rect, palette: &Palette) {
    let block = Block::bordered()
        .border_style(Style::default().fg(palette.secondary))
        .style(Style::default().bg(palette.surface));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let line = Line::from(vec![
        Span::styled(
            " Save ",
            Style::default()
                .fg(palette.background)
                .bg(palette.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Save As…", Style::default().fg(palette.foreground)),
        Span::raw("  "),
        Span::styled("Revert", Style::default().fg(palette.foreground)),
        Span::raw("  "),
        Span::styled("Back", Style::default().fg(palette.muted)),
    ]);
    frame.render_widget(Paragraph::new(line), inner);
}
