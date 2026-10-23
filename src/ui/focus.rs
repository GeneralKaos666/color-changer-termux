//! Selection movement shared by lists.

use ratatui::widgets::ListState;

/// A list movement request.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Move the selection inside a list of `len` items, clamping at both ends.
///
/// An empty list clears the selection; a list with no selection starts from the
/// first item. `page` is the number of rows a page key jumps.
pub fn list_move(state: &mut ListState, len: usize, movement: Move, page: usize) {
    if len == 0 {
        *state = ListState::default();
        return;
    }
    let last = len - 1;
    let current = state.selected().unwrap_or(0).min(last);
    let next = match movement {
        Move::Up => current.saturating_sub(1),
        Move::Down => (current + 1).min(last),
        Move::Home => 0,
        Move::End => last,
        Move::PageUp => current.saturating_sub(page.max(1)),
        Move::PageDown => current.saturating_add(page.max(1)).min(last),
    };
    state.select(Some(next));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_clamps_at_both_ends() {
        let mut state = ListState::default();
        list_move(&mut state, 3, Move::Up, 2);
        assert_eq!(state.selected(), Some(0));
        list_move(&mut state, 3, Move::End, 2);
        assert_eq!(state.selected(), Some(2));
        list_move(&mut state, 3, Move::Down, 2);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn paging_and_home() {
        let mut state = ListState::default();
        list_move(&mut state, 10, Move::PageDown, 4);
        assert_eq!(state.selected(), Some(4));
        list_move(&mut state, 10, Move::PageDown, 4);
        assert_eq!(state.selected(), Some(8));
        list_move(&mut state, 10, Move::PageUp, 4);
        assert_eq!(state.selected(), Some(4));
        list_move(&mut state, 10, Move::Home, 4);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn an_empty_list_clears_the_selection() {
        let mut state = ListState::default();
        state.select(Some(2));
        list_move(&mut state, 0, Move::Down, 1);
        assert_eq!(state.selected(), None);
    }
}
