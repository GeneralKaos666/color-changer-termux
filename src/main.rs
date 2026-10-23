//! The full-screen event loop.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use termux_colors::app::App;
use termux_colors::paths::Paths;
use termux_colors::terminal;

/// How long the loop waits for a key before redrawing.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

fn main() -> io::Result<()> {
    terminal::install_panic_hook();
    let _guard = terminal::Terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(Paths::from_env());

    loop {
        terminal.draw(|frame| app.render(frame))?;
        if event::poll(POLL_INTERVAL)? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => app.handle_key(key),
                _ => {}
            }
        }
        app.tick(Instant::now());
        if app.should_quit() {
            break;
        }
    }
    Ok(())
}
