//! Colour values stored in `colors.properties`.

use std::fmt;

/// Number of entries in a terminal ANSI palette.
pub const PALETTE_SIZE: usize = 16;

/// The `colors.properties` keys for the 16 palette entries.
pub const PALETTE_KEYS: [&str; PALETTE_SIZE] = [
    "color0", "color1", "color2", "color3", "color4", "color5", "color6", "color7", "color8",
    "color9", "color10", "color11", "color12", "color13", "color14", "color15",
];

/// Human-readable names for the 16 ANSI palette entries.
pub const ANSI_NAMES: [&str; PALETTE_SIZE] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright black",
    "bright red",
    "bright green",
    "bright yellow",
    "bright blue",
    "bright magenta",
    "bright cyan",
    "bright white",
];

/// Fallback palette based on the common xterm defaults.
pub const DEFAULT_PALETTE: [Rgb; PALETTE_SIZE] = [
    Rgb::new(0x00, 0x00, 0x00),
    Rgb::new(0xcc, 0x00, 0x00),
    Rgb::new(0x4e, 0x9a, 0x06),
    Rgb::new(0xc4, 0xa0, 0x00),
    Rgb::new(0x34, 0x65, 0xa4),
    Rgb::new(0x75, 0x50, 0x7b),
    Rgb::new(0x06, 0x98, 0x9a),
    Rgb::new(0xd3, 0xd7, 0xcf),
    Rgb::new(0x55, 0x57, 0x53),
    Rgb::new(0xef, 0x29, 0x29),
    Rgb::new(0x8a, 0xe2, 0x34),
    Rgb::new(0xfc, 0xe9, 0x4f),
    Rgb::new(0x72, 0x9f, 0xcf),
    Rgb::new(0xad, 0x7f, 0xa8),
    Rgb::new(0x34, 0xe2, 0xe2),
    Rgb::new(0xee, 0xee, 0xee),
];

/// A 24-bit colour as stored in `colors.properties`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Build a colour from its three channels.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse a `#rgb` or `#rrggbb` colour, ignoring case and surrounding
    /// whitespace. Short forms expand each digit into a byte.
    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.trim().strip_prefix('#')?;
        let hex = match digits.len() {
            3 => digits.chars().flat_map(|digit| [digit, digit]).collect(),
            6 => digits.to_string(),
            _ => return None,
        };
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self::new(r, g, b))
    }

    /// Render the colour as a lowercase `#rrggbb` string.
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// The colour values stored in `colors.properties`.
///
/// Unknown keys found in a parsed file are kept in [`TerminalColors::extra`]
/// so a round-trip through the app never silently drops user data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalColors {
    pub background: Rgb,
    pub foreground: Rgb,
    pub cursor: Rgb,
    pub palette: [Rgb; PALETTE_SIZE],
    pub extra: Vec<(String, String)>,
}

impl Default for TerminalColors {
    fn default() -> Self {
        Self {
            background: Rgb::new(0x00, 0x00, 0x00),
            foreground: Rgb::new(0xff, 0xff, 0xff),
            cursor: Rgb::new(0xff, 0xff, 0xff),
            palette: DEFAULT_PALETTE,
            extra: Vec::new(),
        }
    }
}

impl TerminalColors {
    /// Read one managed colour by its field.
    ///
    /// # Panics
    ///
    /// Panics when `field` is a [`ColorField::Palette`] index of
    /// [`PALETTE_SIZE`] or greater.
    pub fn field(&self, field: ColorField) -> Rgb {
        match field {
            ColorField::Background => self.background,
            ColorField::Foreground => self.foreground,
            ColorField::Cursor => self.cursor,
            ColorField::Palette(index) => self.palette[index],
        }
    }

    /// Write one managed colour by its field.
    ///
    /// # Panics
    ///
    /// Panics when `field` is a [`ColorField::Palette`] index of
    /// [`PALETTE_SIZE`] or greater.
    pub fn set_field(&mut self, field: ColorField, color: Rgb) {
        match field {
            ColorField::Background => self.background = color,
            ColorField::Foreground => self.foreground = color,
            ColorField::Cursor => self.cursor = color,
            ColorField::Palette(index) => self.palette[index] = color,
        }
    }

    /// Report whether `key` is one of the colours this model owns, as opposed
    /// to an entry preserved in [`TerminalColors::extra`].
    pub fn is_managed(&self, key: &str) -> bool {
        matches!(key, "background" | "foreground" | "cursor") || PALETTE_KEYS.contains(&key)
    }
}

/// One selectable colour of [`TerminalColors`].
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ColorField {
    Background,
    Foreground,
    Cursor,
    Palette(usize),
}

impl ColorField {
    /// Every field, in the order the editor presents them.
    pub const ALL: [ColorField; 3 + PALETTE_SIZE] = [
        ColorField::Background,
        ColorField::Foreground,
        ColorField::Cursor,
        ColorField::Palette(0),
        ColorField::Palette(1),
        ColorField::Palette(2),
        ColorField::Palette(3),
        ColorField::Palette(4),
        ColorField::Palette(5),
        ColorField::Palette(6),
        ColorField::Palette(7),
        ColorField::Palette(8),
        ColorField::Palette(9),
        ColorField::Palette(10),
        ColorField::Palette(11),
        ColorField::Palette(12),
        ColorField::Palette(13),
        ColorField::Palette(14),
        ColorField::Palette(15),
    ];

    /// The `colors.properties` key this field is stored under.
    ///
    /// # Panics
    ///
    /// Panics when the palette index is [`PALETTE_SIZE`] or greater.
    pub fn key(&self) -> String {
        match self {
            Self::Background => "background".to_string(),
            Self::Foreground => "foreground".to_string(),
            Self::Cursor => "cursor".to_string(),
            Self::Palette(index) => PALETTE_KEYS[*index].to_string(),
        }
    }

    /// The human-readable label shown in the editor.
    ///
    /// # Panics
    ///
    /// Panics when the palette index is [`PALETTE_SIZE`] or greater.
    pub fn label(&self) -> String {
        match self {
            Self::Background => "background".to_string(),
            Self::Foreground => "foreground".to_string(),
            Self::Cursor => "cursor".to_string(),
            Self::Palette(index) => format!("color{index}  ({})", ANSI_NAMES[*index]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_short_and_long_hex() {
        assert_eq!(Rgb::parse("#0f0c1a"), Some(Rgb::new(0x0f, 0x0c, 0x1a)));
        assert_eq!(Rgb::parse("#ABC"), Some(Rgb::new(0xaa, 0xbb, 0xcc)));
        assert_eq!(Rgb::parse("#abc").unwrap().to_hex(), "#aabbcc");
    }

    #[test]
    fn displays_as_lowercase_hex() {
        assert_eq!(Rgb::new(0x0f, 0x0c, 0x1a).to_string(), "#0f0c1a");
        assert_eq!(Rgb::parse("#0F0C1A"), Some(Rgb::new(0x0f, 0x0c, 0x1a)));
    }

    #[test]
    fn rejects_malformed_hex() {
        for bad in ["abc", "#abcd", "#gggggg", "", "#12 345", "0xffffff"] {
            assert_eq!(Rgb::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn defaults_match_the_xterm_palette() {
        let c = TerminalColors::default();
        assert_eq!(c.palette.len(), PALETTE_SIZE);
        assert_eq!(c.palette[0].to_hex(), "#000000");
        assert_eq!(c.palette[1].to_hex(), "#cc0000");
        assert_eq!(c.palette[15].to_hex(), "#eeeeee");
        assert_eq!(c.extra.len(), 0);
    }

    #[test]
    fn field_round_trip_covers_all_nineteen() {
        let mut c = TerminalColors::default();
        assert_eq!(ColorField::ALL.len(), 19);
        assert_eq!(ColorField::ALL[0].key(), "background");
        assert_eq!(ColorField::ALL[3].key(), "color0");
        assert_eq!(ColorField::ALL[18].label(), "color15  (bright white)");
        c.set_field(ColorField::Palette(4), Rgb::new(1, 2, 3));
        assert_eq!(c.field(ColorField::Palette(4)), Rgb::new(1, 2, 3));
    }

    #[test]
    fn managed_keys_exclude_extras() {
        let c = TerminalColors {
            extra: vec![("selection-background".into(), "#333333".into())],
            ..Default::default()
        };
        assert!(c.is_managed("cursor"));
        assert!(c.is_managed("color15"));
        assert!(!c.is_managed("selection-background"));
    }
}
