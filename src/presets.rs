//! Built-in fallback themes materialised when the library is empty.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::model::colors::{Rgb, TerminalColors};
use crate::persist::colors_io::render_colors;

/// Parses a built-in colour, which is a compile-time constant we control.
fn rgb(hex: &str) -> Rgb {
    Rgb::parse(hex).expect("built-in colour must be valid hex")
}

/// Builds a [`TerminalColors`] from its hex components.
fn colors(background: &str, foreground: &str, cursor: &str, palette: [&str; 16]) -> TerminalColors {
    TerminalColors {
        background: rgb(background),
        foreground: rgb(foreground),
        cursor: rgb(cursor),
        palette: palette.map(rgb),
        extra: Vec::new(),
    }
}

/// The six fallback themes shipped with the application, in display order.
///
/// Every value is copied verbatim from the Python authority
/// (`termux_colors/presets/built_in.py`).
pub fn built_in() -> Vec<(&'static str, TerminalColors)> {
    vec![
        (
            "dracula",
            colors(
                "#282a36",
                "#f8f8f2",
                "#f8f8f2",
                [
                    "#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd",
                    "#f8f8f2", "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff", "#ff92df",
                    "#a4ffff", "#ffffff",
                ],
            ),
        ),
        (
            "solarized-dark",
            colors(
                "#002b36",
                "#839496",
                "#839496",
                [
                    "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198",
                    "#eee8d5", "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4",
                    "#93a1a1", "#fdf6e3",
                ],
            ),
        ),
        (
            "solarized-light",
            colors(
                "#fdf6e3",
                "#657b83",
                "#657b83",
                [
                    "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198",
                    "#eee8d5", "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4",
                    "#93a1a1", "#fdf6e3",
                ],
            ),
        ),
        (
            "nord",
            colors(
                "#2e3440",
                "#d8dee9",
                "#d8dee9",
                [
                    "#3b4252", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0",
                    "#e5e9f0", "#4c566a", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead",
                    "#8fbcbb", "#eceff4",
                ],
            ),
        ),
        (
            "monokai",
            colors(
                "#272822",
                "#f8f8f2",
                "#f8f8f0",
                [
                    "#272822", "#f92672", "#a6e22e", "#f4bf75", "#66d9ef", "#ae81ff", "#a1efe4",
                    "#f8f8f2", "#75715e", "#f92672", "#a6e22e", "#f4bf75", "#66d9ef", "#ae81ff",
                    "#a1efe4", "#f9f8f5",
                ],
            ),
        ),
        (
            "gruvbox-dark",
            colors(
                "#282828",
                "#ebdbb2",
                "#ebdbb2",
                [
                    "#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a",
                    "#a89984", "#928374", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b",
                    "#8ec07c", "#ebdbb2",
                ],
            ),
        ),
    ]
}

/// Writes every built-in theme into `destination`, returning the files written.
///
/// Files that already exist are left untouched, matching the Python
/// implementation's `materialize`. The directory is created when it is absent.
pub fn materialize(destination: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(destination)?;
    let mut written = Vec::new();
    for (name, colors) in built_in() {
        let path = destination.join(format!("{name}.properties"));
        if path.exists() {
            continue;
        }
        std::fs::write(&path, render_colors(&colors))?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_built_ins_are_present() {
        let names: Vec<&str> = built_in().iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            vec![
                "dracula",
                "solarized-dark",
                "solarized-light",
                "nord",
                "monokai",
                "gruvbox-dark"
            ]
        );
        assert_eq!(built_in()[0].1.background.to_hex(), "#282a36");
    }

    #[test]
    fn materialize_writes_missing_files_only() {
        let dir = tempfile::tempdir().unwrap();
        let written = materialize(dir.path()).unwrap();
        assert_eq!(written.len(), 6);
        let dracula = dir.path().join("dracula.properties");
        assert!(
            std::fs::read_to_string(&dracula)
                .unwrap()
                .contains("background=#282a36")
        );

        std::fs::write(&dracula, "background=#111111\n").unwrap();
        materialize(dir.path()).unwrap();
        assert!(
            std::fs::read_to_string(&dracula)
                .unwrap()
                .contains("#111111")
        );
    }
}
