"""Fallback themes used when the user has no colour library installed."""

from __future__ import annotations

from pathlib import Path

from ..models.colors import TerminalColors

BUILT_IN: dict[str, TerminalColors] = {
    "dracula": TerminalColors(
        background="#282a36",
        foreground="#f8f8f2",
        cursor="#f8f8f2",
        palette=[
            "#21222c", "#ff5555", "#50fa7b", "#f1fa8c",
            "#bd93f9", "#ff79c6", "#8be9fd", "#f8f8f2",
            "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5",
            "#d6acff", "#ff92df", "#a4ffff", "#ffffff",
        ],
    ),
    "solarized-dark": TerminalColors(
        background="#002b36",
        foreground="#839496",
        cursor="#839496",
        palette=[
            "#073642", "#dc322f", "#859900", "#b58900",
            "#268bd2", "#d33682", "#2aa198", "#eee8d5",
            "#002b36", "#cb4b16", "#586e75", "#657b83",
            "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
        ],
    ),
    "solarized-light": TerminalColors(
        background="#fdf6e3",
        foreground="#657b83",
        cursor="#657b83",
        palette=[
            "#073642", "#dc322f", "#859900", "#b58900",
            "#268bd2", "#d33682", "#2aa198", "#eee8d5",
            "#002b36", "#cb4b16", "#586e75", "#657b83",
            "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
        ],
    ),
    "nord": TerminalColors(
        background="#2e3440",
        foreground="#d8dee9",
        cursor="#d8dee9",
        palette=[
            "#3b4252", "#bf616a", "#a3be8c", "#ebcb8b",
            "#81a1c1", "#b48ead", "#88c0d0", "#e5e9f0",
            "#4c566a", "#bf616a", "#a3be8c", "#ebcb8b",
            "#81a1c1", "#b48ead", "#8fbcbb", "#eceff4",
        ],
    ),
    "monokai": TerminalColors(
        background="#272822",
        foreground="#f8f8f2",
        cursor="#f8f8f0",
        palette=[
            "#272822", "#f92672", "#a6e22e", "#f4bf75",
            "#66d9ef", "#ae81ff", "#a1efe4", "#f8f8f2",
            "#75715e", "#f92672", "#a6e22e", "#f4bf75",
            "#66d9ef", "#ae81ff", "#a1efe4", "#f9f8f5",
        ],
    ),
    "gruvbox-dark": TerminalColors(
        background="#282828",
        foreground="#ebdbb2",
        cursor="#ebdbb2",
        palette=[
            "#282828", "#cc241d", "#98971a", "#d79921",
            "#458588", "#b16286", "#689d6a", "#a89984",
            "#928374", "#fb4934", "#b8bb26", "#fabd2f",
            "#83a598", "#d3869b", "#8ec07c", "#ebdbb2",
        ],
    ),
}


def built_in_themes() -> dict[str, TerminalColors]:
    """Return a copy of the built-in fallback themes."""
    return {name: colors.copy() for name, colors in BUILT_IN.items()}


def materialize(destination: Path) -> list[Path]:
    """Write the built-in themes into *destination*, returning file paths."""
    from ..persistence.colors_io import render_colors

    destination.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for name, colors in BUILT_IN.items():
        path = destination / f"{name}.properties"
        if not path.exists():
            path.write_text(render_colors(colors), encoding="utf-8")
        written.append(path)
    return written
