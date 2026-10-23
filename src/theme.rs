//! Contrast/luminance maths and the concrete [`Palette`] colour set.

use ratatui::style::Color;

use crate::model::colors::{Rgb, TerminalColors};

/// The per-channel linearisation step of the WCAG luminance formula.
///
/// Ported verbatim from the Python authority's `_channel_luminance`.
fn channel_luminance(channel: u8) -> f64 {
    let normalised = f64::from(channel) / 255.0;
    if normalised <= 0.03928 {
        normalised / 12.92
    } else {
        ((normalised + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG relative luminance of `color` (0 = black, 1 = white).
pub fn relative_luminance(color: Rgb) -> f64 {
    0.2126 * channel_luminance(color.r)
        + 0.7152 * channel_luminance(color.g)
        + 0.0722 * channel_luminance(color.b)
}

/// Whether `color` reads as dark.
///
/// Uses the same 0.35 luminance threshold as the Python authority's `is_dark`.
pub fn is_dark(color: Rgb) -> bool {
    relative_luminance(color) < 0.35
}

/// WCAG contrast ratio between two colours (1..=21).
pub fn contrast_ratio(first: Rgb, second: Rgb) -> f64 {
    let a = relative_luminance(first);
    let b = relative_luminance(second);
    let (light, dark) = if a >= b { (a, b) } else { (b, a) };
    (light + 0.05) / (dark + 0.05)
}

/// Mix `first` toward `second` by `amount` (0..1) in sRGB.
pub fn blend(first: Rgb, second: Rgb, amount: f64) -> Rgb {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| {
        let value = f64::from(a) + (f64::from(b) - f64::from(a)) * amount;
        value.round().clamp(0.0, 255.0) as u8
    };
    Rgb::new(
        mix(first.r, second.r),
        mix(first.g, second.g),
        mix(first.b, second.b),
    )
}

/// Return `color`, nudged toward white/black until it contrasts enough.
///
/// Tries 20 equal blend steps toward the contrasting extreme, returning the
/// first candidate that meets `minimum`, or the extreme itself.
pub fn ensure_contrast(color: Rgb, against: Rgb, minimum: f64) -> Rgb {
    if contrast_ratio(color, against) >= minimum {
        return color;
    }
    let target = if is_dark(against) {
        Rgb::new(0xff, 0xff, 0xff)
    } else {
        Rgb::new(0, 0, 0)
    };
    for step in 1..=20 {
        let candidate = blend(color, target, f64::from(step) / 20.0);
        if contrast_ratio(candidate, against) >= minimum {
            return candidate;
        }
    }
    target
}

/// Convert a [`Color`] back to an [`Rgb`].
///
/// Only [`Color::Rgb`] carries channel data; every other variant (named ANSI
/// colours, indexed colours, reset) maps to black, since this module never
/// produces such colours.
pub fn to_rgb(color: Color) -> Rgb {
    match color {
        Color::Rgb(r, g, b) => Rgb::new(r, g, b),
        _ => Rgb::new(0, 0, 0),
    }
}

/// Wrap an [`Rgb`] as a [`Color::Rgb`].
fn rgb_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.r, rgb.g, rgb.b)
}

/// The concrete colour set derived from a managed terminal palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub surface: Color,
    pub panel: Color,
    pub boost: Color,
    pub muted: Color,
    pub dark: bool,
}

impl Palette {
    /// Derive a [`Palette`] from a managed terminal palette.
    ///
    /// Mirrors the Python authority's `theme_from_colors` slot derivations,
    /// plus the Rust-specific `muted = blend(foreground, background, 0.35)`.
    pub fn from_colors(colors: &TerminalColors) -> Self {
        let background = colors.background;
        let foreground = ensure_contrast(colors.foreground, background, 4.5);
        let dark = is_dark(background);
        let toward = if dark {
            Rgb::new(0xff, 0xff, 0xff)
        } else {
            Rgb::new(0, 0, 0)
        };

        let pick = |index: usize, minimum: f64| {
            ensure_contrast(colors.palette[index], background, minimum)
        };

        let surface = blend(background, toward, if dark { 0.05 } else { 0.04 });

        Self {
            background: rgb_color(background),
            foreground: rgb_color(foreground),
            primary: rgb_color(pick(6, 3.0)),
            secondary: rgb_color(pick(5, 2.5)),
            accent: rgb_color(pick(4, 2.5)),
            error: rgb_color(pick(1, 2.5)),
            success: rgb_color(pick(2, 2.5)),
            warning: rgb_color(pick(3, 2.5)),
            surface: rgb_color(surface),
            panel: rgb_color(blend(background, toward, 0.10)),
            boost: rgb_color(blend(background, toward, 0.17)),
            muted: rgb_color(blend(foreground, background, 0.35)),
            dark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_extremes() {
        assert_eq!(relative_luminance(Rgb::new(0, 0, 0)), 0.0);
        assert!((relative_luminance(Rgb::new(255, 255, 255)) - 1.0).abs() < 1e-9);
        assert!(is_dark(Rgb::new(16, 16, 16)));
        assert!(!is_dark(Rgb::new(240, 240, 240)));
    }

    #[test]
    fn contrast_ratio_bounds() {
        assert!((contrast_ratio(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)) - 21.0).abs() < 1e-9);
        assert!(
            (contrast_ratio(Rgb::new(0x12, 0x34, 0x56), Rgb::new(0x12, 0x34, 0x56)) - 1.0).abs()
                < 1e-9
        );
    }

    #[test]
    fn ensure_contrast_pushes_apart() {
        let bg = Rgb::new(0, 0, 0);
        let fixed = ensure_contrast(Rgb::new(13, 13, 13), bg, 3.0);
        assert!(contrast_ratio(fixed, bg) >= 3.0);
    }

    #[test]
    fn blend_hits_both_ends() {
        let a = Rgb::new(0, 0, 0);
        let b = Rgb::new(255, 255, 255);
        assert_eq!(blend(a, b, 0.0), a);
        assert_eq!(blend(a, b, 1.0), b);
    }

    #[test]
    fn palette_derives_from_colours() {
        let colors = TerminalColors::default();
        let p = Palette::from_colors(&colors);
        assert!(p.dark);
        assert!(contrast_ratio(to_rgb(p.foreground), colors.background) >= 4.5);
        assert_ne!(p.panel, p.background);
        assert_ne!(p.muted, p.foreground);
    }
}
