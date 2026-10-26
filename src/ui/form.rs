//! Shared rows for the editor forms: section rules and labelled colour fields.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::chrome;
use crate::model::colors::Rgb;
use crate::ui::Ui;
use crate::ui::glyphs::Glyphs;
use crate::ui::input::Field;
use crate::ui::text::swatch;

/// Columns reserved for a field's label.
const LABEL_WIDTH: u16 = 26;

/// Width of the inline colour chip, without its trailing gap.
const SWATCH_WIDTH: u16 = 3;

/// Draw a section heading followed by a rule that fills the rest of the row.
pub fn section_rule(frame: &mut Frame, area: Rect, title: &str, ui: &Ui) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let used = (title.chars().count() as u16 + 1).min(area.width);
    let [title_area, rule_area] =
        Layout::horizontal([Constraint::Length(used), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(title.to_uppercase()).style(
            Style::default()
                .fg(ui.palette.secondary)
                .add_modifier(Modifier::BOLD),
        ),
        title_area,
    );
    if rule_area.width > 0 {
        frame.render_widget(
            Paragraph::new(
                ui.glyphs
                    .rule_char()
                    .to_string()
                    .repeat(rule_area.width as usize),
            )
            .style(Style::default().fg(ui.palette.muted)),
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
    ui: &Ui,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let label_width = LABEL_WIDTH.min(area.width);
    let [label_area, rest] =
        Layout::horizontal([Constraint::Length(label_width), Constraint::Min(0)]).areas(area);
    let label_style = if focused {
        Style::default()
            .fg(ui.palette.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui.palette.foreground)
    };
    let label = truncate(label, label_width as usize, ui.glyphs);
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
        Style::default().fg(ui.palette.error).bg(ui.palette.boost)
    } else if focused {
        Style::default()
            .fg(Color::White)
            .bg(chrome::accent(ui.palette.dark))
    } else {
        Style::default()
            .fg(ui.palette.foreground)
            .bg(ui.palette.surface)
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
fn truncate(text: &str, width: usize, glyphs: Glyphs) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let ellipsis = glyphs.ellipsis();
    let ellipsis_width = ellipsis.chars().count();
    if width <= ellipsis_width {
        return ellipsis.chars().take(width).collect();
    }
    let mut out: String = text.chars().take(width - ellipsis_width).collect();
    out.push_str(ellipsis);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_uses_unicode_ellipsis_by_default() {
        assert_eq!(truncate("abcdef", 4, Glyphs::new(false)), "abc…");
    }

    #[test]
    fn truncate_uses_ascii_ellipsis_in_ascii_mode() {
        assert_eq!(truncate("abcdef", 5, Glyphs::new(true)), "ab...");
    }
}
