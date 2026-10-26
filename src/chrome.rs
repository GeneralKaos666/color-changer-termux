//! Fixed "chrome" colours and helpers that do not move with the terminal
//! palette: the violet accent, the gradient title, and the help-bar line.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::model::colors::Rgb;
use crate::theme::{blend, ensure_contrast};

/// Indigo accent for light backgrounds.
pub const ACCENT_LIGHT: Rgb = Rgb::new(0x5B, 0x50, 0xE6);
/// Violet accent for dark backgrounds.
pub const ACCENT_DARK: Rgb = Rgb::new(0xA4, 0x9E, 0xFF);
/// Gradient title start (lavender).
pub const GRADIENT_FROM: Rgb = Rgb::new(0xC9, 0xC2, 0xFF);
/// Gradient title end (indigo).
pub const GRADIENT_TO: Rgb = Rgb::new(0x5B, 0x50, 0xE6);

/// The fixed accent for the given background polarity.
pub fn accent(dark: bool) -> Color {
    let rgb = if dark { ACCENT_DARK } else { ACCENT_LIGHT };
    Color::Rgb(rgb.r, rgb.g, rgb.b)
}

/// Render `text` as a bold lavender→indigo gradient, one span per character.
///
/// On light backgrounds the indigo stop is pushed through
/// [`ensure_contrast`] against white so the title stays legible; the indigo
/// already passes, so in practice it is unchanged.
pub fn gradient_title(text: &str, dark: bool) -> Vec<Span<'static>> {
    let to = if dark {
        GRADIENT_TO
    } else {
        ensure_contrast(GRADIENT_TO, Rgb::new(0xFF, 0xFF, 0xFF), 4.5)
    };
    let chars: Vec<char> = text.chars().collect();
    let steps = chars.len().saturating_sub(1).max(1);
    chars
        .into_iter()
        .enumerate()
        .map(|(index, c)| {
            let rgb = blend(GRADIENT_FROM, to, index as f64 / steps as f64);
            Span::styled(
                c.to_string(),
                Style::default()
                    .fg(Color::Rgb(rgb.r, rgb.g, rgb.b))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect()
}

/// Render a help-bar line: accent key hints + muted descriptions, ` · `-joined.
pub fn help_bar(
    pairs: &[(&'static str, &'static str)],
    accent: Color,
    muted: Color,
) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, desc)) in pairs.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" · ", Style::default().fg(muted)));
        }
        spans.push(Span::styled(
            (*key).to_owned(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));
        if !desc.is_empty() {
            spans.push(Span::styled(format!(" {desc}"), Style::default().fg(muted)));
        }
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};
    use ratatui::text::Span;

    fn span_text(spans: &[Span]) -> String {
        spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn accent_flips_with_background() {
        assert_eq!(accent(false), Color::Rgb(0x5B, 0x50, 0xE6));
        assert_eq!(accent(true), Color::Rgb(0xA4, 0x9E, 0xFF));
    }

    #[test]
    fn gradient_title_is_bold_and_two_stop() {
        let spans = gradient_title("ab", true);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "a");
        assert_eq!(spans[1].content, "b");
        assert_eq!(spans[0].style.fg, Some(Color::Rgb(0xC9, 0xC2, 0xFF)));
        assert_eq!(spans[1].style.fg, Some(Color::Rgb(0x5B, 0x50, 0xE6)));
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn gradient_title_stays_legible_on_light_backgrounds() {
        let spans = gradient_title("ab", false);
        // The indigo stop already passes WCAG contrast on white, so it is kept.
        assert_eq!(spans[1].style.fg, Some(Color::Rgb(0x5B, 0x50, 0xE6)));
    }

    #[test]
    fn help_bar_joins_key_hint_pairs() {
        let line = help_bar(
            &[("↑/↓", "move"), ("Enter", "open")],
            Color::White,
            Color::Rgb(0x6B, 0x72, 0x80),
        );
        let text = span_text(&line.spans);
        assert!(text.contains("↑/↓"));
        assert!(text.contains("move"));
        assert!(text.contains("Enter"));
        assert!(text.contains("open"));
        assert!(text.contains(" · "));
    }
}
