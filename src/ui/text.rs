//! Pure text building: the banner wordmark, palette strip, swatch grid and
//! the mock shell session.
//!
//! Every function returns owned [`Line`]/[`Span`] values with explicit
//! [`Color::Rgb`] styles, ported from the Python authority's `widgets.py`.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::model::colors::{Rgb, TerminalColors};
use crate::theme::{ensure_contrast, relative_luminance};

/// A minimal 6x5 block font, only the glyphs used by the wordmark.
const GLYPHS: [(char, [&str; 5]); 10] = [
    ('T', ["██████", "  ██  ", "  ██  ", "  ██  ", "  ██  "]),
    ('E', ["██████", "██    ", "████  ", "██    ", "██████"]),
    ('R', ["█████ ", "██  ██", "█████ ", "██ ██ ", "██  ██"]),
    ('M', ["██   ██", "███ ███", "██ █ ██", "██   ██", "██   ██"]),
    ('U', ["██  ██", "██  ██", "██  ██", "██  ██", "██████"]),
    ('X', ["██  ██", " ████ ", "  ██  ", " ████ ", "██  ██"]),
    ('C', [" █████", "██    ", "██    ", "██    ", " █████"]),
    ('O', [" ████ ", "██  ██", "██  ██", "██  ██", " ████ "]),
    ('L', ["██    ", "██    ", "██    ", "██    ", "██████"]),
    ('S', [" █████", "██    ", " ████ ", "    ██", "█████ "]),
];

/// Return black or white, whichever reads better on `color`.
///
/// Ports the Python `contrast_color` rule: black above a 0.45 luminance
/// threshold, white otherwise.
pub fn contrast_color(color: Rgb) -> Rgb {
    if relative_luminance(color) > 0.45 {
        Rgb::new(0, 0, 0)
    } else {
        Rgb::new(255, 255, 255)
    }
}

/// A small inline colour chip, `width` cells wide.
pub fn swatch(color: Rgb, width: usize) -> Span<'static> {
    Span::styled(" ".repeat(width), Style::default().bg(rgb(color)))
}

/// The "TERMUX COLORS" wordmark in the palette's colours.
///
/// `compact` shows just "TERMUX"; `stacked` puts the two words on separate
/// rows; otherwise they share one row when the width allows it.
pub fn banner(
    colors: &TerminalColors,
    width: u16,
    stacked: bool,
    compact: bool,
) -> Vec<Line<'static>> {
    if compact {
        return word("TERMUX", colors, 0);
    }
    if stacked || width < 88 {
        let mut top = word("TERMUX", colors, 0);
        top.extend(word("COLORS", colors, 6));
        return top;
    }
    let top = word("TERMUX", colors, 0);
    let right = word("COLORS", colors, 6);
    top.into_iter()
        .zip(right)
        .map(|(left, right)| {
            let mut spans = left.spans;
            spans.push(Span::raw("  "));
            spans.extend(right.spans);
            Line::from(spans)
        })
        .collect()
}

/// A single-line strip of the 16 palette colours, three cells each.
pub fn palette_strip(colors: &TerminalColors) -> Line<'static> {
    Line::from(
        colors
            .palette
            .iter()
            .map(|color| Span::styled("   ", Style::default().bg(rgb(*color))))
            .collect::<Vec<_>>(),
    )
}

/// The 16-chip swatch grid plus background, foreground and cursor samples.
pub fn swatch_grid(colors: &TerminalColors) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(5);
    for start in [0, 8] {
        let mut spans = Vec::new();
        for index in start..start + 8 {
            let background = colors.palette[index];
            let foreground = contrast_color(background);
            spans.push(Span::styled(
                format!("{index:>2}"),
                Style::default()
                    .fg(rgb(foreground))
                    .bg(rgb(background))
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::raw(""));

    let background = colors.background;
    lines.push(Line::from(vec![
        Span::styled(
            "  background  ",
            Style::default()
                .fg(rgb(contrast_color(background)))
                .bg(rgb(background)),
        ),
        Span::raw("  "),
        Span::styled(
            "  foreground  ",
            Style::default()
                .fg(rgb(colors.foreground))
                .bg(rgb(background)),
        ),
    ]));

    let cursor = colors.cursor;
    lines.push(Line::from(Span::styled(
        "  cursor      ",
        Style::default()
            .fg(rgb(contrast_color(cursor)))
            .bg(rgb(cursor)),
    )));

    lines
}

/// A fake shell session rendered in `colors`, one [`Line`] per screen row.
pub fn mock_session(colors: &TerminalColors) -> Vec<Line<'static>> {
    let palette = colors.palette;
    let background = colors.background;
    let ink = |index: usize| ensure_contrast(palette[index], background, 2.5);

    let prompt = Line::from(vec![
        bold("user@termux", ink(2)),
        plain(":", colors.foreground),
        bold("~", ink(4)),
        plain("$ ", colors.foreground),
        plain("ls", colors.foreground),
    ]);

    let listing = Line::from(vec![
        Span::raw("  "),
        bold("Documents  ", ink(4)),
        bold("Downloads  ", ink(4)),
        bold("projects  ", ink(4)),
        plain("notes.md  ", colors.foreground),
        plain("README  ", colors.foreground),
    ]);

    let git = Line::from(vec![
        bold("user@termux", ink(2)),
        plain(":", colors.foreground),
        bold("~/projects", ink(4)),
        plain("$ ", colors.foreground),
        plain("git status", colors.foreground),
    ]);

    let branch = Line::from(vec![
        plain("  On branch ", colors.foreground),
        bold("main", ink(6)),
    ]);

    let clean = Line::from(plain("  nothing to commit, working tree clean", ink(2)));

    let tests = Line::from(vec![
        Span::raw("  "),
        bold("✓ ", ink(2)),
        plain("31 passed", ink(2)),
        plain(" in 2.5s", colors.foreground),
    ]);

    let error = Line::from(vec![
        Span::raw("  "),
        bold("make: ", ink(1)),
        plain("*** [Makefile:12: build] Error 1", ink(1)),
    ]);

    vec![prompt, listing, git, branch, clean, tests, error]
}

/// Render one word of the wordmark as five rows of styled spans.
///
/// Each letter is coloured by `ensure_contrast` of the palette entry at
/// `(index + offset) % 16`, matching the Python `_word`.
fn word(word: &str, colors: &TerminalColors, offset: usize) -> Vec<Line<'static>> {
    let mut rows = vec![Vec::new(); 5];
    for (index, letter) in word.chars().enumerate() {
        let Some(glyph) = glyph(letter) else {
            continue;
        };
        let color = ensure_contrast(
            colors.palette[(index + offset) % colors.palette.len()],
            colors.background,
            3.0,
        );
        for (row, pattern) in glyph.iter().enumerate() {
            rows[row].push(bold(pattern, color));
            rows[row].push(Span::raw(" "));
        }
    }
    rows.into_iter().map(Line::from).collect()
}

/// The five block-font rows for `letter`, or [`None`] when it has no glyph.
fn glyph(letter: char) -> Option<[&'static str; 5]> {
    let upper = letter.to_ascii_uppercase();
    GLYPHS
        .iter()
        .find(|(key, _)| *key == upper)
        .map(|(_, rows)| *rows)
}

/// A foreground-coloured span, optionally bold.
fn plain(content: &'static str, color: Rgb) -> Span<'static> {
    Span::styled(content, Style::default().fg(rgb(color)))
}

/// A bold foreground-coloured span.
fn bold(content: &'static str, color: Rgb) -> Span<'static> {
    Span::styled(
        content,
        Style::default().fg(rgb(color)).add_modifier(Modifier::BOLD),
    )
}

/// Wrap an [`Rgb`] as a [`Color::Rgb`].
fn rgb(color: Rgb) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn banner_has_five_rows_unstacked_and_ten_stacked() {
        let c = TerminalColors::default();
        assert_eq!(banner(&c, 100, false, false).len(), 5);
        assert_eq!(banner(&c, 60, true, false).len(), 10);
        assert_eq!(banner(&c, 40, false, true).len(), 5);
        assert!(all_text(&banner(&c, 100, false, false)).contains("█"));
    }

    #[test]
    fn strip_has_sixteen_coloured_blocks() {
        let line = palette_strip(&TerminalColors::default());
        assert_eq!(line.spans.len(), 16);
        assert_eq!(line.spans[0].style.bg, Some(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn grid_has_two_chip_rows_and_three_samples() {
        let lines = swatch_grid(&TerminalColors::default());
        assert_eq!(lines.len(), 5);
        assert!(all_text(&lines).contains("15"));
        assert!(all_text(&lines).contains("background"));
    }

    #[test]
    fn mock_session_has_seven_lines_and_a_prompt() {
        let lines = mock_session(&TerminalColors::default());
        assert_eq!(lines.len(), 7);
        assert!(all_text(&lines).starts_with("user@termux"));
    }

    #[test]
    fn contrast_color_flips_at_the_threshold() {
        assert_eq!(contrast_color(Rgb::new(255, 255, 255)), Rgb::new(0, 0, 0));
        assert_eq!(contrast_color(Rgb::new(0, 0, 0)), Rgb::new(255, 255, 255));
    }
}
