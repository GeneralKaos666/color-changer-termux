//! Shared rows for the editor forms: section rules and labelled colour fields.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::model::colors::Rgb;
use crate::theme::Palette;
use crate::ui::input::Field;
use crate::ui::text::swatch;

/// Columns reserved for a field's label.
const LABEL_WIDTH: u16 = 26;

/// Width of the inline colour chip, without its trailing gap.
const SWATCH_WIDTH: u16 = 3;

/// Draw a section heading followed by a rule that fills the rest of the row.
pub fn section_rule(frame: &mut Frame, area: Rect, title: &str, palette: &Palette) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let used = (title.chars().count() as u16 + 1).min(area.width);
    let [title_area, rule_area] =
        Layout::horizontal([Constraint::Length(used), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(title.to_uppercase()).style(
            Style::default()
                .fg(palette.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        title_area,
    );
    if rule_area.width > 0 {
        frame.render_widget(
            Paragraph::new("─".repeat(rule_area.width as usize))
                .style(Style::default().fg(palette.muted)),
            rule_area,
        );
    }
}

/// Draw one editor row: the label, a colour chip and the hex field.
///
/// A value that does not parse paints the field in the palette's `error`
/// colour and leaves the chip blank; a valid value fills the chip.
pub fn field_row(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    field: &mut Field,
    focused: bool,
    palette: &Palette,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let label_width = LABEL_WIDTH.min(area.width);
    let [label_area, rest] =
        Layout::horizontal([Constraint::Length(label_width), Constraint::Min(0)]).areas(area);
    let label_style = if focused {
        Style::default()
            .fg(palette.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.foreground)
    };
    let label = truncate(label, label_width as usize);
    frame.render_widget(Paragraph::new(label).style(label_style), label_area);
    if rest.width == 0 {
        return;
    }

    let chip_width = (SWATCH_WIDTH + 1).min(rest.width);
    let [chip_area, input_area] =
        Layout::horizontal([Constraint::Length(chip_width), Constraint::Min(0)]).areas(rest);
    let color = Rgb::parse(field.value());
    let chip = match color {
        Some(color) => Line::from(vec![swatch(color, SWATCH_WIDTH as usize), Span::raw(" ")]),
        None => Line::from(Span::raw(" ".repeat(chip_width as usize))),
    };
    frame.render_widget(Paragraph::new(chip), chip_area);
    if input_area.width == 0 {
        return;
    }

    let invalid = !field.value().trim().is_empty() && color.is_none();
    let style = if invalid {
        Style::default().fg(palette.error).bg(palette.boost)
    } else if focused {
        Style::default().fg(palette.background).bg(palette.primary)
    } else {
        Style::default().fg(palette.foreground).bg(palette.surface)
    };
    field.set_width(input_area.width as usize);
    frame.render_widget(
        Paragraph::new(field.value())
            .style(style)
            .scroll((0, field.display_offset() as u16)),
        input_area,
    );
}

/// Truncate `text` to at most `width` cells, adding an ellipsis when it is cut.
fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut out: String = text.chars().take(width - 1).collect();
    out.push('…');
    out
}
