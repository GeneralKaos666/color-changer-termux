//! Command-line argument parsing for termux-settings.

use std::env;

use crate::model::font_slot::FontSlot;

#[derive(Debug, Default)]
pub struct Args {
    pub list_fonts: bool,
    pub apply_font: Option<String>,
    pub slot: Option<FontSlot>,
    pub version: bool,
    pub ascii: bool,
}

impl Args {
    pub fn parse() -> Self {
        let mut args = Args::default();
        let mut argv = env::args().skip(1);
        while let Some(arg) = argv.next() {
            match arg.as_str() {
                "--list-fonts" => args.list_fonts = true,
                "--apply-font" => {
                    if let Some(name) = argv.next() {
                        args.apply_font = Some(name);
                    }
                }
                "--slot" => {
                    if let Some(s) = argv.next() {
                        args.slot = FontSlot::try_from(s.as_str()).ok();
                    }
                }
                "--version" => args.version = true,
                "--ascii" => args.ascii = true,
                _ => {}
            }
        }
        if env::var("TERMUX_SETTINGS_ASCII").is_ok_and(|v| v != "0")
            || env::var("TERMUX_COLORS_ASCII").is_ok_and(|v| v != "0")
            || env::var("FONT_CHANGER_ASCII").is_ok_and(|v| v != "0")
        {
            args.ascii = true;
        }
        args
    }
}
