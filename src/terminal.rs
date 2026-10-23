//! Terminal lifecycle: raw mode, the alternate screen and their restoration.

use std::io::{self, Write};

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

/// Owns the terminal in its full-screen mode and restores it on drop.
pub struct Terminal;

impl Terminal {
    /// Enter raw mode and the alternate screen, hiding the cursor.
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore();
    }
}

/// Put the terminal back into its normal state.
///
/// Idempotent and error-tolerant: it runs from `Drop`, from the panic hook and
/// from the normal exit path, any of which may find the terminal already clean.
pub fn restore() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, Show, LeaveAlternateScreen);
    let _ = stdout.flush();
    let _ = disable_raw_mode();
}

/// Restore the terminal before the previous panic hook prints.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}
