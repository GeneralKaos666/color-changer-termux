//! Shared UI context: the derived palette and the active glyph set.

pub mod dialog;
pub mod focus;
pub mod form;
pub mod glyphs;
pub mod input;
pub mod text;

use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

use crate::chrome;
use crate::theme::Palette;

pub use glyphs::Glyphs;

/// Everything a screen needs to draw: the palette and the glyph set.
#[derive(Copy, Clone)]
pub struct Ui {
    pub palette: Palette,
    pub glyphs: Glyphs,
}

/// A bordered pane with the shared accent border, surface fill, and the
/// glyph set's border (ASCII `+ - |` or rounded box-drawing).
pub fn pane<'a>(ui: &Ui, title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(ui.glyphs.border_set())
        .title(title)
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(chrome::accent(ui.palette.dark)))
        .style(Style::default().bg(ui.palette.surface))
}
