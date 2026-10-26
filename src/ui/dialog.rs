//! Modal dialogs: confirmation, messages and single-line prompts.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};

use crate::chrome;
use crate::theme::Palette;
use crate::ui::input::Field;

/// The outcome of a dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogResult {
    /// A confirmation was accepted (`true`) or cancelled (`false`).
    Confirmed(bool),
    /// A message was dismissed.
    Acknowledged,
    /// A prompt's trimmed answer, or `None` when it was blank or cancelled.
    Text(Option<String>),
}

/// A modal box drawn over the current screen.
#[derive(Clone, Debug)]
pub enum Dialog {
    /// A yes/no question.
    Confirm {
        title: String,
        message: String,
        confirm_label: String,
        cancel_label: String,
        hint: String,
        focus_confirm: bool,
    },
    /// An informational message with a single dismiss button.
    Message { title: String, message: String },
    /// A question with one line of text input.
    Prompt {
        title: String,
        message: String,
        field: Field,
        placeholder: String,
    },
}

/// The most columns a dialog ever occupies.
const MAX_WIDTH: u16 = 64;

impl Dialog {
    /// A yes/no question with `confirm_label` and `cancel_label` buttons.
    pub fn confirm(message: &str, confirm_label: &str, cancel_label: &str) -> Self {
        Self::Confirm {
            title: "Confirm".to_owned(),
            message: message.to_owned(),
            confirm_label: confirm_label.to_owned(),
            cancel_label: cancel_label.to_owned(),
            hint: "esc cancels".to_owned(),
            focus_confirm: true,
        }
    }

    /// An informational message titled `title`.
    pub fn message(message: &str, title: &str) -> Self {
        Self::Message {
            title: title.to_owned(),
            message: message.to_owned(),
        }
    }

    /// A prompt whose field starts at `initial`.
    pub fn prompt(message: &str, initial: &str) -> Self {
        Self::Prompt {
            title: "Input".to_owned(),
            message: message.to_owned(),
            field: Field::new(initial),
            placeholder: String::new(),
        }
    }

    /// Show `placeholder` in the prompt while its field is empty.
    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        if let Self::Prompt {
            placeholder: slot, ..
        } = &mut self
        {
            *slot = placeholder.to_owned();
        }
        self
    }

    /// Handle a key press, returning a result once the dialog is finished.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<DialogResult> {
        match self {
            Self::Confirm { focus_confirm, .. } => match key.code {
                KeyCode::Esc => Some(DialogResult::Confirmed(false)),
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                    *focus_confirm = !*focus_confirm;
                    None
                }
                KeyCode::Enter => Some(DialogResult::Confirmed(*focus_confirm)),
                _ => None,
            },
            Self::Message { .. } => match key.code {
                KeyCode::Esc | KeyCode::Enter => Some(DialogResult::Acknowledged),
                _ => None,
            },
            Self::Prompt { field, .. } => match key.code {
                KeyCode::Esc => Some(DialogResult::Text(None)),
                KeyCode::Enter => {
                    let answer = field.value().trim();
                    Some(DialogResult::Text(
                        (!answer.is_empty()).then(|| answer.to_owned()),
                    ))
                }
                _ => {
                    field.handle_key(key);
                    None
                }
            },
        }
    }

    /// Draw the dialog centred in `area` on top of whatever is already there.
    pub fn render(&self, area: Rect, buf: &mut Buffer, palette: &Palette) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = (area.width * 9 / 10).min(MAX_WIDTH).clamp(1, area.width);
        let height = (self.height() + 2).min(area.height);
        let rect = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" {}", self.title()))
            .title_alignment(Alignment::Left)
            .border_style(Style::default().fg(chrome::accent(palette.dark)))
            .style(Style::default().bg(palette.panel).fg(palette.foreground));
        let inner = block.inner(rect);
        block.render(rect, buf);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let message_style = Style::default().fg(palette.foreground);
        match self {
            Self::Confirm { message, hint, .. } => {
                Paragraph::new(message.as_str())
                    .style(message_style)
                    .wrap(Wrap { trim: true })
                    .render(Rect { height: 1, ..inner }, buf);
                if inner.height > 1 && !hint.is_empty() {
                    let row = Rect {
                        y: inner.y + inner.height - 2,
                        height: 1,
                        ..inner
                    };
                    Paragraph::new(hint.as_str())
                        .style(Style::default().fg(palette.muted))
                        .render(row, buf);
                }
            }
            Self::Message { message, .. } => {
                Paragraph::new(message.as_str())
                    .style(message_style)
                    .wrap(Wrap { trim: true })
                    .render(Rect { height: 1, ..inner }, buf);
            }
            Self::Prompt {
                message,
                field,
                placeholder,
                ..
            } => {
                Paragraph::new(message.as_str())
                    .style(message_style)
                    .wrap(Wrap { trim: true })
                    .render(Rect { height: 1, ..inner }, buf);
                if inner.height > 2 {
                    let row = Rect {
                        x: inner.x + 1,
                        y: inner.y + 2,
                        width: inner.width.saturating_sub(2),
                        height: 1,
                    };
                    render_field(field, placeholder, row, buf, palette);
                }
            }
        }

        self.render_buttons(inner, buf, palette);
    }

    /// Rows the dialog body needs, excluding its borders.
    fn height(&self) -> u16 {
        match self {
            Self::Confirm { .. } | Self::Message { .. } => 3,
            Self::Prompt { .. } => 5,
        }
    }

    /// The border title.
    fn title(&self) -> &str {
        match self {
            Self::Confirm { title, .. }
            | Self::Message { title, .. }
            | Self::Prompt { title, .. } => title,
        }
    }

    /// The button labels and which one is focused, if any.
    fn buttons(&self) -> (Vec<&str>, Option<usize>) {
        match self {
            Self::Confirm {
                confirm_label,
                cancel_label,
                focus_confirm,
                ..
            } => (
                vec![confirm_label.as_str(), cancel_label.as_str()],
                Some(usize::from(!*focus_confirm)),
            ),
            Self::Message { .. } => (vec!["OK"], Some(0)),
            Self::Prompt { .. } => (vec!["OK", "Cancel"], None),
        }
    }

    /// Draw the right-aligned button row on the last inner row.
    fn render_buttons(&self, inner: Rect, buf: &mut Buffer, palette: &Palette) {
        if inner.height == 0 {
            return;
        }
        let (labels, focus) = self.buttons();
        let mut spans = Vec::new();
        for (index, label) in labels.iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw(" "));
            }
            let style = if focus == Some(index) {
                Style::default()
                    .bg(chrome::accent(palette.dark))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(palette.boost).fg(palette.foreground)
            };
            spans.push(Span::styled(format!(" {label} "), style));
        }
        let row = Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        };
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Right)
            .render(row, buf);
    }
}

/// Draw the prompt's text, its placeholder and a cursor block.
fn render_field(field: &Field, placeholder: &str, row: Rect, buf: &mut Buffer, palette: &Palette) {
    let width = row.width as usize;
    if width == 0 {
        return;
    }
    let offset = field.input.visual_scroll(width);
    let value = field.value();
    if value.is_empty() {
        buf.set_string(
            row.x,
            row.y,
            placeholder,
            Style::default().fg(palette.muted),
        );
    }
    let visible: String = value.chars().skip(offset).take(width).collect();
    let style = Style::default().fg(palette.foreground);
    buf.set_string(row.x, row.y, &visible, style);

    let cursor = field.input.visual_cursor().saturating_sub(offset);
    if cursor < width {
        let cell = &mut buf[(row.x + cursor as u16, row.y)];
        cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn message_acknowledges_on_enter() {
        let mut dialog = Dialog::message("done", "Notice");
        assert_eq!(
            dialog.handle_key(press(KeyCode::Enter)),
            Some(DialogResult::Acknowledged)
        );
    }

    #[test]
    fn prompt_keeps_typed_text() {
        let mut dialog = Dialog::prompt("Name", "custom");
        dialog.handle_key(press(KeyCode::Char('x')));
        assert_eq!(
            dialog.handle_key(press(KeyCode::Enter)),
            Some(DialogResult::Text(Some("customx".to_owned())))
        );
    }
}
