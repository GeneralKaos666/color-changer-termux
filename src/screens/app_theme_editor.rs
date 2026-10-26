//! Edit the curated `termux.properties` settings.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Paragraph};

use crate::app::Command;
use crate::chrome;
use crate::library::reload_termux;
use crate::model::app_theme::{AppTheme, CURATED_FIELDS, FieldKind, FieldSpec, validate_value};
use crate::paths::Paths;
use crate::persist::backup::make_backup;
use crate::persist::theme_io::{read_app_theme, write_app_theme};
use crate::screens::ScreenAction;
use crate::theme::Palette;
use crate::ui::dialog::{Dialog, DialogResult};
use crate::ui::form;
use crate::ui::glyphs;
use crate::ui::input::Field;

/// Form rows the editor assumes are visible until the first render measures
/// the pane.
const DEFAULT_VISIBLE_ROWS: usize = 12;

/// Columns reserved for a field's label.
const LABEL_WIDTH: u16 = 26;

/// Columns reserved for the muted key name at the row's end.
const KEY_WIDTH: u16 = 18;

/// One editable row of the form.
pub struct Row {
    /// The curated field this row edits.
    pub spec: &'static FieldSpec,
    /// The chosen value of a selector; [`None`] means unset.
    ///
    /// Text and integer rows keep their value in [`Row::field`] instead, and
    /// mirror it here so a save or revert keeps both in step.
    pub value: Option<String>,
    /// The text buffer of a text or integer row.
    pub field: Field,
    /// Whether the current value fails validation.
    pub invalid: bool,
}

impl Row {
    /// The value the row currently describes.
    fn current(&self) -> String {
        match self.spec.kind {
            FieldKind::Bool | FieldKind::Choice => self.value.clone().unwrap_or_default(),
            FieldKind::Text | FieldKind::Int => self.field.value().trim().to_owned(),
        }
    }

    /// Whether the row renders as a selector rather than a text field.
    fn is_selector(&self) -> bool {
        matches!(self.spec.kind, FieldKind::Bool | FieldKind::Choice)
    }

    /// The values a selector cycles through, in order.
    fn choices(&self) -> Vec<&'static str> {
        match self.spec.kind {
            FieldKind::Bool => vec!["true", "false"],
            FieldKind::Choice => self.spec.choices.to_vec(),
            FieldKind::Text | FieldKind::Int => Vec::new(),
        }
    }
}

/// The step a dialog answer resumes.
#[derive(Clone, Debug)]
pub enum ThemePending {
    /// Back up, then write these values.
    BackupThenWrite { values: AppTheme },
}

/// A slot of the rendered form.
#[derive(Copy, Clone)]
enum Slot {
    Hint,
    Section(&'static str),
    Field(usize),
}

/// The app-theme editor's state.
pub struct AppThemeEditor {
    /// One row per curated field, in schema order.
    pub rows: [Row; 9],
    /// Index into [`AppThemeEditor::rows`] with the keyboard.
    pub focus: usize,
    /// First form row drawn, so the focused field stays visible.
    pub scroll: usize,
    /// The values last read from or written to `termux.properties`.
    theme: AppTheme,
    /// The file the rows edit, shown above them.
    path_hint: String,
    /// The step a dialog answer resumes.
    pending: Option<ThemePending>,
    visible_rows: usize,
}

impl AppThemeEditor {
    /// Load the rows from `termux.properties`.
    pub fn new(paths: &Paths) -> Self {
        let theme = read_app_theme(&paths.termux_properties()).unwrap_or_default();
        let rows = std::array::from_fn(|index| {
            let spec = &CURATED_FIELDS[index];
            let current = theme.get(spec.key).to_owned();
            let value = (!current.is_empty()).then(|| current.clone());
            Row {
                spec,
                field: Field::new(&current),
                invalid: (!current.is_empty()) && validate_value(spec, &current).is_err(),
                value,
            }
        });
        Self {
            rows,
            focus: 0,
            scroll: 0,
            theme,
            path_hint: paths.termux_properties().display().to_string(),
            pending: None,
            visible_rows: DEFAULT_VISIBLE_ROWS,
        }
    }

    /// Run the save or revert flow `command` asks for.
    pub fn command(&mut self, command: Command, paths: &Paths) -> ScreenAction {
        match command {
            Command::Save => self.start_save(paths),
            Command::Revert => self.start_revert(paths),
            Command::SaveAs => ScreenAction::None,
        }
    }

    /// The key bindings the help bar shows for this screen.
    pub fn help(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("↑/↓", "field"),
            ("←/→", "cycle"),
            ("ctrl+s", "save"),
            ("ctrl+r", "revert"),
            ("Esc", "back"),
        ]
    }

    /// Write the validated values, asking for a backup when the file exists.
    pub fn start_save(&mut self, paths: &Paths) -> ScreenAction {
        let values = match self.collect() {
            Ok(values) => values,
            Err((_key, message)) => {
                return ScreenAction::OpenDialog(Dialog::message(&message, "Invalid setting"));
            }
        };
        if paths.termux_properties().exists() {
            self.pending = Some(ThemePending::BackupThenWrite { values });
            return ScreenAction::OpenDialog(Dialog::confirm(
                "Back up termux.properties before saving?",
                "Back up",
                "Skip",
            ));
        }
        self.write(paths, values)
    }

    /// Reload every row from `termux.properties`.
    pub fn start_revert(&mut self, paths: &Paths) -> ScreenAction {
        self.theme = read_app_theme(&paths.termux_properties()).unwrap_or_default();
        self.load_rows();
        ScreenAction::Status("Reverted to the file's current values.".to_owned())
    }

    /// Advance the save flow once a dialog has answered.
    pub fn on_dialog(&mut self, result: DialogResult, paths: &Paths) -> ScreenAction {
        let Some(pending) = self.pending.take() else {
            return ScreenAction::None;
        };
        match (pending, result) {
            (ThemePending::BackupThenWrite { values }, DialogResult::Confirmed(confirmed)) => {
                if confirmed
                    && let Err(error) =
                        make_backup(&paths.termux_properties(), &paths.backups(), "termux")
                {
                    return ScreenAction::OpenDialog(Dialog::message(
                        &format!("Could not back up termux.properties: {error}"),
                        "Backup failed",
                    ));
                }
                self.write(paths, values)
            }
            (ThemePending::BackupThenWrite { .. }, _) => ScreenAction::None,
        }
    }

    /// The validated values, or the first row's key and its error message.
    pub fn collect(&self) -> Result<AppTheme, (String, String)> {
        let mut theme = AppTheme::default();
        for row in &self.rows {
            match validate_value(row.spec, &row.current()) {
                Ok(value) => theme.set(row.spec.key, &value),
                Err(message) => return Err((row.spec.key.to_owned(), message)),
            }
        }
        Ok(theme)
    }

    /// Handle a key press, returning the action the app should apply.
    pub fn handle_key(&mut self, key: KeyEvent) -> ScreenAction {
        match key.code {
            KeyCode::Esc => return ScreenAction::Pop,
            KeyCode::Tab | KeyCode::Down => {
                self.focus = (self.focus + 1).min(self.rows.len() - 1);
                self.scroll_to_focus();
                return ScreenAction::None;
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = self.focus.saturating_sub(1);
                self.scroll_to_focus();
                return ScreenAction::None;
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Enter
                if self.rows[self.focus].is_selector() =>
            {
                let forward = key.code != KeyCode::Left;
                self.cycle(forward);
                self.validate_focus();
                return ScreenAction::None;
            }
            _ => {}
        }
        if self.rows[self.focus].field.handle_key(key) {
            self.rows[self.focus].value = Some(self.rows[self.focus].field.value().to_owned());
            self.validate_focus();
        }
        ScreenAction::None
    }

    /// Move the focused selector to the next choice, or to unset.
    fn cycle(&mut self, forward: bool) {
        let row = &mut self.rows[self.focus];
        let choices = row.choices();
        let step = |index: Option<usize>| match index {
            Some(index) if forward => (index + 1 < choices.len()).then_some(index + 1),
            Some(index) => index.checked_sub(1),
            None if forward => (!choices.is_empty()).then_some(0),
            None => choices.len().checked_sub(1),
        };
        let current = row
            .value
            .as_ref()
            .and_then(|value| choices.iter().position(|choice| choice == value));
        row.value = step(current).map(|index| choices[index].to_owned());
    }

    /// Recompute the focused row's error state.
    fn validate_focus(&mut self) {
        let row = &mut self.rows[self.focus];
        let current = row.current();
        row.invalid = !current.is_empty() && validate_value(row.spec, &current).is_err();
    }

    /// Reset every row from [`AppThemeEditor::theme`].
    fn load_rows(&mut self) {
        for row in &mut self.rows {
            let current = self.theme.get(row.spec.key).to_owned();
            row.value = (!current.is_empty()).then(|| current.clone());
            row.field.set_value(&current);
            row.invalid = (!current.is_empty()) && validate_value(row.spec, &current).is_err();
        }
    }

    /// Write `values` to `termux.properties`, reload and report.
    fn write(&mut self, paths: &Paths, values: AppTheme) -> ScreenAction {
        let path = paths.termux_properties();
        if let Err(error) = write_app_theme(&path, &values, true) {
            return ScreenAction::Status(format!("could not save {}: {error}", path.display()));
        }
        self.theme = values;
        self.load_rows();
        let mut message = "Saved termux.properties.".to_owned();
        if !reload_termux() {
            message.push_str(" Run termux-reload-settings to refresh.");
        }
        ScreenAction::Status(message)
    }

    /// Move [`AppThemeEditor::scroll`] the least amount that keeps the focused
    /// row inside the visible rows.
    fn scroll_to_focus(&mut self) {
        let row = row_of(self.focus);
        if row < self.scroll {
            self.scroll = row;
        } else if row >= self.scroll + self.visible_rows {
            self.scroll = row + 1 - self.visible_rows;
        }
    }

    /// Draw the form.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        frame.render_widget(
            Block::default().style(
                Style::default()
                    .bg(palette.background)
                    .fg(palette.foreground),
            ),
            area,
        );
        self.render_form(frame, area, palette);
    }

    /// Draw the scrolling form of section rules and labelled rows.
    fn render_form(&mut self, frame: &mut Frame, area: Rect, palette: &Palette) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" app theme")
            .title_alignment(Alignment::Left)
            .border_style(Style::default().fg(chrome::accent(palette.dark)))
            .style(Style::default().bg(palette.surface));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        self.visible_rows = inner.height as usize;
        self.scroll_to_focus();
        let slots = slots();
        let end = (self.scroll + self.visible_rows).min(slots.len());
        for (row, slot) in slots.iter().enumerate().take(end).skip(self.scroll) {
            let row_area = Rect {
                x: inner.x,
                y: inner.y + (row - self.scroll) as u16,
                width: inner.width,
                height: 1,
            };
            match *slot {
                Slot::Hint => frame.render_widget(
                    Paragraph::new(format!("Editing {}", self.path_hint))
                        .style(Style::default().fg(palette.muted)),
                    row_area,
                ),
                Slot::Section(section) => {
                    form::section_rule(frame, row_area, section, palette);
                }
                Slot::Field(index) => render_row(
                    frame,
                    row_area,
                    &mut self.rows[index],
                    self.focus == index,
                    palette,
                ),
            }
        }
    }
}

/// The rendered form rows: the path hint, a rule per section and a row per
/// field, in schema order.
fn slots() -> Vec<Slot> {
    let mut slots = vec![Slot::Hint];
    let mut section = "";
    for (index, spec) in CURATED_FIELDS.iter().enumerate() {
        if spec.section != section {
            section = spec.section;
            slots.push(Slot::Section(section));
        }
        slots.push(Slot::Field(index));
    }
    slots
}

/// The form row of field `index`.
fn row_of(index: usize) -> usize {
    slots()
        .iter()
        .position(|slot| matches!(slot, Slot::Field(position) if *position == index))
        .unwrap_or(0)
}

/// Draw one row: label, a selector or text field, and the key name.
fn render_row(frame: &mut Frame, area: Rect, row: &mut Row, focused: bool, palette: &Palette) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let label_width = LABEL_WIDTH.min(area.width);
    let [label_area, rest] =
        Layout::horizontal([Constraint::Length(label_width), Constraint::Min(0)]).areas(area);
    let label = if row.spec.help.is_empty() {
        row.spec.label.to_owned()
    } else {
        format!("{}  ·  {}", row.spec.label, row.spec.help)
    };
    let label_style = if row.invalid {
        Style::default().fg(palette.error)
    } else if focused {
        Style::default()
            .fg(palette.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.foreground)
    };
    frame.render_widget(Paragraph::new(label).style(label_style), label_area);
    if rest.width == 0 {
        return;
    }

    let key_width = if rest.width > 3 * KEY_WIDTH {
        KEY_WIDTH
    } else {
        0
    };
    let [control_area, key_area] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(key_width)]).areas(rest);

    let control_style = if row.invalid {
        Style::default().fg(palette.error).bg(palette.boost)
    } else if focused {
        Style::default()
            .fg(Color::White)
            .bg(chrome::accent(palette.dark))
    } else {
        Style::default().fg(palette.foreground).bg(palette.boost)
    };
    if row.is_selector() {
        let current = row.value.clone().unwrap_or_else(|| "unset".to_owned());
        frame.render_widget(
            Paragraph::new(format!(
                "{} {current} {}",
                glyphs::chip().0,
                glyphs::chip().1
            ))
            .style(control_style),
            control_area,
        );
    } else {
        row.field.set_width(control_area.width as usize);
        frame.render_widget(
            Paragraph::new(row.field.value())
                .style(control_style)
                .scroll((0, row.field.display_offset() as u16)),
            control_area,
        );
    }
    if key_area.width > 0 {
        frame.render_widget(
            Paragraph::new(row.spec.key)
                .alignment(Alignment::Right)
                .style(Style::default().fg(palette.muted)),
            key_area,
        );
    }
}
