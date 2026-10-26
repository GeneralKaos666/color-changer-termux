//! Typed font slots for the four Termux font files.
//!
//! Replaces the `&str` slot names (`"regular"`, `"bold"`, …) with an enum so a
//! bad slot is a compile error at the call site instead of a runtime `match`.

use std::fmt;

use crate::error::Error;

/// One of the four Termux font files.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontSlot {
    /// `font.ttf`.
    Regular,
    /// `font-bold.ttf`.
    Bold,
    /// `font-italic.ttf`.
    Italic,
    /// `font-bold-italic.ttf`.
    BoldItalic,
}

impl FontSlot {
    /// Every slot, in the order the UI cycles through them.
    pub const ALL: [FontSlot; 4] = [
        FontSlot::Regular,
        FontSlot::Bold,
        FontSlot::Italic,
        FontSlot::BoldItalic,
    ];

    /// The user-facing slot name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::BoldItalic => "bold-italic",
        }
    }

    /// The Termux font file the slot writes to.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Regular => "font.ttf",
            Self::Bold => "font-bold.ttf",
            Self::Italic => "font-italic.ttf",
            Self::BoldItalic => "font-bold-italic.ttf",
        }
    }
}

impl fmt::Display for FontSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for FontSlot {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "regular" => Ok(Self::Regular),
            "bold" => Ok(Self::Bold),
            "italic" => Ok(Self::Italic),
            "bold-italic" => Ok(Self::BoldItalic),
            _ => Err(Error::Invalid(format!("unknown font slot: {value:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FontSlot;

    #[test]
    fn all_slots_round_trip() {
        for slot in FontSlot::ALL {
            assert_eq!(FontSlot::try_from(slot.as_str()).unwrap(), slot);
        }
    }

    #[test]
    fn file_names_are_the_termux_originals() {
        assert_eq!(FontSlot::Regular.file_name(), "font.ttf");
        assert_eq!(FontSlot::Bold.file_name(), "font-bold.ttf");
        assert_eq!(FontSlot::Italic.file_name(), "font-italic.ttf");
        assert_eq!(FontSlot::BoldItalic.file_name(), "font-bold-italic.ttf");
    }

    #[test]
    fn rejects_unknown_slot() {
        assert!(FontSlot::try_from("nope").is_err());
    }
}
