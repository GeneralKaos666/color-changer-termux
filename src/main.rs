//! The full-screen event loop.

use std::io;
use std::time::Instant;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use termux_settings::app::App;
use termux_settings::cli::Args;
use termux_settings::fonts;
use termux_settings::model::font_slot::FontSlot;
use termux_settings::paths::Paths;
use termux_settings::runtime::{Msg, TICK};
use termux_settings::terminal;
use termux_settings::ui::Glyphs;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let glyphs = Glyphs::new(args.ascii);
    if args.version {
        println!("termux-settings {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let paths = Paths::from_env();
    if args.list_fonts {
        let entries = fonts::scan::list_library(&paths).unwrap_or_default();
        for e in entries {
            println!("{}", e.name);
        }
        return Ok(());
    }
    if let Some(ref name) = args.apply_font {
        let slot = args.slot.unwrap_or(FontSlot::Regular);
        let entries = fonts::scan::list_library(&paths).unwrap_or_default();
        // find entry
        if let Some(entry) = entries
            .iter()
            .find(|e| e.name == *name || e.name.to_lowercase() == name.to_lowercase())
        {
            if let Err(e) = fonts::apply::install_font(&paths, &entry.path, slot) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            println!(
                "Applied {} to {} ({})",
                entry.name,
                slot,
                paths.font_slot_path(slot).display()
            );
            return Ok(());
        } else {
            eprintln!("unknown font: {name:?}");
            std::process::exit(2);
        }
    }

    terminal::install_panic_hook();
    let _guard = terminal::Terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::with_glyphs(paths, glyphs);

    loop {
        for msg in app.drain() {
            app.update(msg);
        }
        terminal.draw(|frame| app.render(frame))?;
        if app.should_quit() {
            break;
        }
        if event::poll(TICK)? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => app.update(Msg::Key(key)),
                Event::Resize(width, height) => app.update(Msg::Resize(width, height)),
                _ => {}
            }
        } else {
            app.update(Msg::Tick(Instant::now()));
        }
    }
    Ok(())
}
