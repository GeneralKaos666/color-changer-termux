//! ASCII-aware glyphs, resolved from the global ASCII mode.

use crate::ui::ascii::is_ascii;

/// The selection marker (`▶ ` / `> `).
pub fn marker() -> &'static str {
    if is_ascii() { "> " } else { "▶ " }
}

/// The dotted rule character (`┄` / `-`).
pub fn rule_char() -> char {
    if is_ascii() { '-' } else { '┄' }
}

/// The badge/slot chip delimiters (`⟨ ⟩` / `< >`).
pub fn chip() -> (&'static str, &'static str) {
    if is_ascii() {
        ("<", ">")
    } else {
        ("⟨", "⟩")
    }
}

/// The active-badge dot (`●` / `*`).
pub fn dot() -> &'static str {
    if is_ascii() { "*" } else { "●" }
}

/// The ellipsis (`…` / `...`).
pub fn ellipsis() -> &'static str {
    if is_ascii() { "..." } else { "…" }
}

/// The filter glyph (`⌕ ` / `? `).
pub fn filter() -> &'static str {
    if is_ascii() { "? " } else { "⌕ " }
}

/// The breadcrumb separator (`›` / `>`).
pub fn breadcrumb() -> &'static str {
    if is_ascii() { ">" } else { "›" }
}

/// The success tick used in mock shell output (`✓ ` / `v `).
pub fn tick() -> &'static str {
    if is_ascii() { "v " } else { "✓ " }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ascii::set_ascii;

    #[test]
    fn glyphs_switch_with_ascii_mode() {
        set_ascii(false);
        assert_eq!(marker(), "▶ ");
        assert_eq!(rule_char(), '┄');
        assert_eq!(chip(), ("⟨", "⟩"));
        assert_eq!(dot(), "●");
        assert_eq!(ellipsis(), "…");
        assert_eq!(filter(), "⌕ ");
        assert_eq!(breadcrumb(), "›");
        assert_eq!(tick(), "✓ ");
        set_ascii(true);
        assert_eq!(marker(), "> ");
        assert_eq!(rule_char(), '-');
        assert_eq!(chip(), ("<", ">"));
        assert_eq!(dot(), "*");
        assert_eq!(ellipsis(), "...");
        assert_eq!(filter(), "? ");
        assert_eq!(breadcrumb(), ">");
        assert_eq!(tick(), "v ");
        set_ascii(false);
    }
}
