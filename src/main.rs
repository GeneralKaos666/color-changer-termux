//! The `termux-settings` binary.

use std::process::ExitCode;

use termux_settings::cli::Args;
use termux_settings::paths::Paths;

fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };
    let paths = Paths::from_env();
    match termux_settings::cli::run(args, &paths) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}
