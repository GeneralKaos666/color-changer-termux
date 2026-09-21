"""Rich-text helpers for rendering colour previews."""

from __future__ import annotations

from rich.text import Text

from .models.colors import TerminalColors


def _rgb(hex_color: str) -> tuple[int, int, int] | None:
    value = hex_color.strip().lstrip("#")
    if len(value) == 3:
        value = "".join(ch * 2 for ch in value)
    if len(value) != 6:
        return None
    try:
        return int(value[0:2], 16), int(value[2:4], 16), int(value[4:6], 16)
    except ValueError:
        return None


def contrast_color(hex_color: str) -> str:
    """Return black or white, whichever reads better on *hex_color*."""
    rgb = _rgb(hex_color)
    if rgb is None:
        return "#ffffff"
    r, g, b = rgb
    luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255
    return "#000000" if luminance > 0.6 else "#ffffff"


def colors_preview_text(colors: TerminalColors) -> Text:
    """Build a Rich ``Text`` swatch preview of *colors*."""
    text = Text()
    for index in range(16):
        background = colors.palette[index]
        foreground = contrast_color(background)
        text.append(f"{index:>2}", style=f"bold {foreground} on {background}")
        text.append(" ")
        if index % 8 == 7:
            text.append("\n")
    text.append("\n")
    text.append("  background  ", style=f"{contrast_color(colors.background)} on {colors.background}")
    text.append("  ")
    text.append("  foreground  ", style=f"{colors.foreground} on {colors.background}")
    text.append("\n")
    text.append("  cursor      ", style=f"{contrast_color(colors.cursor)} on {colors.cursor}")
    text.append("   " + colors.background + " / " + colors.foreground, style="dim")
    return text
