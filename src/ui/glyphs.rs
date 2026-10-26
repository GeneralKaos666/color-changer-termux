//! ASCII-aware glyphs, resolved from an explicit [`Glyphs`] value rather than
//! a global flag.

use ratatui::symbols::border::{ROUNDED, Set};

/// Which set of glyphs the UI draws with.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Glyphs {
    ascii: bool,
}

impl Glyphs {
    /// Unicode glyphs when `ascii` is false, box-drawing-free ASCII otherwise.
    pub const fn new(ascii: bool) -> Self {
        Self { ascii }
    }

    /// The selection marker (`▶ ` / `> `).
    pub fn marker(self) -> &'static str {
        if self.ascii { "> " } else { "▶ " }
    }

    /// The dotted rule character (`┄` / `-`).
    pub fn rule_char(self) -> char {
        if self.ascii { '-' } else { '┄' }
    }

    /// The badge/slot chip delimiters (`⟨ ⟩` / `< >`).
    pub fn chip(self) -> (&'static str, &'static str) {
        if self.ascii {
            ("<", ">")
        } else {
            ("⟨", "⟩")
        }
    }

    /// The active-badge dot (`●` / `*`).
    pub fn dot(self) -> &'static str {
        if self.ascii { "*" } else { "●" }
    }

    /// The ellipsis (`…` / `...`).
    pub fn ellipsis(self) -> &'static str {
        if self.ascii { "..." } else { "…" }
    }

    /// The filter glyph (`⌕ ` / `? `).
    pub fn filter(self) -> &'static str {
        if self.ascii { "? " } else { "⌕ " }
    }

    /// The breadcrumb separator (`›` / `>`).
    pub fn breadcrumb(self) -> &'static str {
        if self.ascii { ">" } else { "›" }
    }

    /// The success tick used in mock shell output (`✓ ` / `v `).
    pub fn tick(self) -> &'static str {
        if self.ascii { "v " } else { "✓ " }
    }

    /// The border every pane and dialog uses: rounded box-drawing characters,
    /// or a pure-ASCII `+ - |` set.
    pub fn border_set(self) -> Set {
        if self.ascii {
            Set {
                top_left: "+",
                top_right: "+",
                bottom_left: "+",
                bottom_right: "+",
                vertical_left: "|",
                vertical_right: "|",
                horizontal_top: "-",
                horizontal_bottom: "-",
            }
        } else {
            ROUNDED
        }
    }

    /// The up/down movement hint (`↑/↓` / `up/dn`).
    pub fn arrow(self) -> &'static str {
        if self.ascii { "up/dn" } else { "↑/↓" }
    }

    /// The left/right cycling hint (`←/→` / `l/r`).
    pub fn lr(self) -> &'static str {
        if self.ascii { "l/r" } else { "←/→" }
    }

    /// The help-bar separator (` · ` / ` | `).
    pub fn separator(self) -> &'static str {
        if self.ascii { " | " } else { " · " }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_switch_with_ascii() {
        assert_eq!(Glyphs::new(false).marker(), "▶ ");
        assert_eq!(Glyphs::new(true).marker(), "> ");
        assert_eq!(Glyphs::new(false).chip(), ("⟨", "⟩"));
        assert_eq!(Glyphs::new(true).chip(), ("<", ">"));
        assert_eq!(Glyphs::new(false).dot(), "●");
        assert_eq!(Glyphs::new(true).dot(), "*");
        assert_eq!(Glyphs::new(false).border_set().top_left, "╭");
        assert_eq!(Glyphs::new(true).border_set().top_left, "+");
        assert_eq!(Glyphs::new(false).arrow(), "↑/↓");
        assert_eq!(Glyphs::new(true).arrow(), "up/dn");
        assert_eq!(Glyphs::new(false).lr(), "←/→");
        assert_eq!(Glyphs::new(true).lr(), "l/r");
    }

    #[test]
    fn pane_uses_ascii_border() {
        assert_eq!(Glyphs::new(true).border_set().horizontal_top, "-");
        assert_eq!(Glyphs::new(true).border_set().vertical_left, "|");
    }
}
