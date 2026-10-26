//! The full-screen event loop.

use std::io;
use std::time::Instant;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use termux_settings::app::App;
use termux_settings::cli::Args;
use termux_settings::fonts;
use termux_settings::paths::Paths;
use termux_settings::terminal;

fn main() -> io::Result<()> {
    let args = Args::parse();
    if args.ascii {
        termux_settings::ui::ascii::set_ascii(true);
    }
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
        let entries = fonts::scan::list_library(&paths).unwrap_or_default();
        // find entry
        if let Some(entry) = entries
            .iter()
            .find(|e| e.name == *name || e.name.to_lowercase() == name.to_lowercase())
        {
            if let Err(e) = fonts::apply::install_font(&paths, &entry.path, &args.slot) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            println!(
                "Applied {} to {} ({})",
                entry.name,
                args.slot,
                paths
                    .font_slot_path(&args.slot)
                    .unwrap_or_default()
                    .display()
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
    let mut app = App::new(paths);

    loop {
        app.tick(Instant::now());
        terminal.draw(|frame| app.render(frame))?;
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => app.handle_key(key),
            _ => {}
        }
        if app.should_quit() {
            break;
        }
    }
    Ok(())
}
