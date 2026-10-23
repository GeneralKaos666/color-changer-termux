//! A thin wrapper around [`tui_input::Input`] with screen-friendly key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::{Input, InputRequest};

/// An editable single-line text value.
#[derive(Clone, Debug, Default)]
pub struct Field {
    /// The underlying buffer and cursor.
    pub input: Input,
    /// The width last used to render the field, for scroll calculations.
    width: usize,
}

impl Field {
    /// Build a field holding `value` with the cursor at the end.
    pub fn new(value: &str) -> Self {
        Self {
            input: Input::new(value.to_owned()),
            width: 0,
        }
    }

    /// The current text.
    pub fn value(&self) -> &str {
        self.input.value()
    }

    /// Replace the text, moving the cursor to the end.
    pub fn set_value(&mut self, value: &str) {
        self.input = Input::new(value.to_owned());
    }

    /// Record the width the field is drawn at.
    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    /// The horizontal scroll offset needed to keep the cursor visible.
    pub fn display_offset(&self) -> usize {
        self.input.visual_scroll(self.width.max(1))
    }

    /// Apply `key` to the field, returning whether it was consumed.
    ///
    /// `Tab`, `BackTab`, `Esc` and `Enter` are never consumed so the owning
    /// screen can always move focus, cancel or submit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let Some(request) = request_for(key) else {
            return false;
        };
        self.input.handle(request).is_some()
    }
}

/// Translate a key press into a [`tui_input::InputRequest`].
///
/// Mirrors the crate's own crossterm backend so the dependency can stay free
/// of optional features.
fn request_for(key: KeyEvent) -> Option<InputRequest> {
    use InputRequest::*;
    use KeyCode::*;
    if key.kind == crossterm::event::KeyEventKind::Release {
        return None;
    }
    match (key.code, key.modifiers) {
        (Backspace, KeyModifiers::NONE) | (Char('h'), KeyModifiers::CONTROL) => {
            Some(DeletePrevChar)
        }
        (Delete, KeyModifiers::NONE) => Some(DeleteNextChar),
        (Tab, KeyModifiers::NONE) | (BackTab, KeyModifiers::NONE) => None,
        (Esc | Enter, _) => None,
        (Left, KeyModifiers::NONE) | (Char('b'), KeyModifiers::CONTROL) => Some(GoToPrevChar),
        (Left, KeyModifiers::CONTROL) => Some(GoToPrevWord),
        (Right, KeyModifiers::NONE) | (Char('f'), KeyModifiers::CONTROL) => Some(GoToNextChar),
        (Right, KeyModifiers::CONTROL) => Some(GoToNextWord),
        (Char('u'), KeyModifiers::CONTROL) => Some(DeleteLine),
        (Char('w'), KeyModifiers::CONTROL) | (Backspace, KeyModifiers::ALT) => Some(DeletePrevWord),
        (Delete, KeyModifiers::CONTROL) => Some(DeleteNextWord),
        (Char('k'), KeyModifiers::CONTROL) => Some(DeleteTillEnd),
        (Char('a'), KeyModifiers::CONTROL) | (Home, KeyModifiers::NONE) => Some(GoToStart),
        (Char('e'), KeyModifiers::CONTROL) | (End, KeyModifiers::NONE) => Some(GoToEnd),
        (Char(character), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(InsertChar(character)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn typing_and_editing_round_trip() {
        let mut field = Field::new("ab");
        assert!(field.handle_key(press(KeyCode::Char('c'))));
        assert_eq!(field.value(), "abc");
        assert!(field.handle_key(press(KeyCode::Backspace)));
        assert_eq!(field.value(), "ab");
    }

    #[test]
    fn navigation_keys_are_left_to_the_screen() {
        let mut field = Field::new("ab");
        assert!(!field.handle_key(press(KeyCode::Tab)));
        assert!(!field.handle_key(press(KeyCode::Esc)));
        assert!(!field.handle_key(press(KeyCode::Enter)));
    }
}
