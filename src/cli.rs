//! Command-line argument parsing and the process entry point for
//! `termux-settings`.

use std::env;
use std::io;
use std::process::ExitCode;
use std::time::Instant;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;
use crate::error::Result;
use crate::fonts;
use crate::model::font_slot::FontSlot;
use crate::paths::Paths;
use crate::runtime::{Msg, TICK};
use crate::terminal;
use crate::ui::Glyphs;

#[derive(Debug, Default)]
pub struct Args {
    pub list_fonts: bool,
    pub apply_font: Option<String>,
    pub slot: Option<FontSlot>,
    pub version: bool,
    pub help: bool,
    pub ascii: bool,
}

/// Usage text printed by `--help` and `-h`.
pub const HELP: &str = concat!(
    "termux-settings ",
    env!("CARGO_PKG_VERSION"),
    "\n",
    "A keyboard-driven TUI for Termux colours, app theme and fonts.\n\n",
    "Usage:\n  termux-settings [options]\n\n",
    "Options:\n",
    "  -h, --help              print this help and exit\n",
    "      --version           print the version and exit\n",
    "      --list-fonts        list fonts in ~/.termux/fonts/\n",
    "      --apply-font <name> install a font from the library\n",
    "      --slot <slot>       regular, bold, italic, bold-italic (default regular)\n",
    "      --ascii             plain-ASCII glyphs\n\n",
    "Environment:\n",
    "  TERMUX_SETTINGS_ASCII   same as --ascii when set to anything but 0\n",
    "  TERMUX_HOME             override ~/.termux (mainly for tests)\n\n",
    "Exit codes:\n  0  success\n  1  runtime error\n  2  usage error\n",
);

impl Args {
    pub fn parse() -> std::result::Result<Self, String> {
        Self::parse_from(env::args().skip(1))
    }

    /// Parse `argv`, applying the ASCII environment overrides.
    pub fn parse_from<I>(argv: I) -> std::result::Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let argv: Vec<String> = argv.into_iter().collect();
        if argv.iter().any(|arg| arg == "--help" || arg == "-h") {
            let mut args = Self {
                help: true,
                ..Self::default()
            };
            args.apply_ascii_env();
            return Ok(args);
        }

        let mut args = Args::default();
        let mut argv = argv.into_iter();
        while let Some(arg) = argv.next() {
            match arg.as_str() {
                "--list-fonts" => args.list_fonts = true,
                "--apply-font" => {
                    args.apply_font = Some(
                        argv.next()
                            .ok_or_else(|| "--apply-font needs a value".to_owned())?,
                    );
                }
                "--slot" => {
                    let value = argv
                        .next()
                        .ok_or_else(|| "--slot needs a value".to_owned())?;
                    args.slot = Some(
                        FontSlot::try_from(value.as_str()).map_err(|error| error.to_string())?,
                    );
                }
                "--version" => args.version = true,
                "--ascii" => args.ascii = true,
                other if other.starts_with('-') => {
                    return Err(format!("unknown option: {other}"));
                }
                other => return Err(format!("unexpected argument: {other}")),
            }
        }
        args.apply_ascii_env();
        Ok(args)
    }

    /// Honour the legacy and current ASCII environment variables.
    fn apply_ascii_env(&mut self) {
        if env::var("TERMUX_SETTINGS_ASCII").is_ok_and(|v| v != "0")
            || env::var("TERMUX_COLORS_ASCII").is_ok_and(|v| v != "0")
            || env::var("FONT_CHANGER_ASCII").is_ok_and(|v| v != "0")
        {
            self.ascii = true;
        }
    }
}

/// Run the application for `args`, returning the process exit code.
///
/// The CLI paths return before touching the terminal; only a fully interactive
/// run enters the alternate screen.
pub fn run(args: Args, paths: &Paths) -> Result<ExitCode> {
    let glyphs = Glyphs::new(args.ascii);
    if args.help {
        println!("{HELP}");
        return Ok(ExitCode::SUCCESS);
    }
    if args.version {
        println!("termux-settings {}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }
    if args.list_fonts {
        let entries = fonts::scan::list_library(paths)?;
        for entry in entries {
            println!("{}", entry.name);
        }
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(ref name) = args.apply_font {
        let slot = args.slot.unwrap_or(FontSlot::Regular);
        let entries = fonts::scan::list_library(paths)?;
        if let Some(entry) = entries
            .iter()
            .find(|e| e.name == *name || e.name.to_lowercase() == name.to_lowercase())
        {
            fonts::apply::install_font(paths, &entry.path, slot)?;
            if !crate::library::reload_termux() {
                eprintln!("{}", crate::fonts::apply::MANUAL_RESTART_HINT);
            }
            println!(
                "Applied {} to {} ({})",
                entry.name,
                slot,
                paths.font_slot_path(slot).display()
            );
            return Ok(ExitCode::SUCCESS);
        }
        eprintln!("unknown font: {name:?}");
        return Ok(ExitCode::from(2));
    }

    terminal::install_panic_hook();
    let _guard = terminal::Terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::with_glyphs(paths.clone(), glyphs);

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
    Ok(ExitCode::SUCCESS)
}
