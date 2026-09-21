"""Derive a Textual theme from the Termux palette being managed.

The application re-colours itself to match the active terminal theme, so the
UI is a live preview of the colours it writes.
"""

from __future__ import annotations

from textual.theme import Theme

from .models.colors import TerminalColors

THEME_NAME = "termux-dynamic"


def _parse(hex_color: str) -> tuple[int, int, int]:
    value = hex_color.strip().lstrip("#")
    if len(value) == 3:
        value = "".join(ch * 2 for ch in value)
    if len(value) != 6:
        raise ValueError(f"invalid colour: {hex_color!r}")
    return int(value[0:2], 16), int(value[2:4], 16), int(value[4:6], 16)


def to_hex(rgb: tuple[float, float, float]) -> str:
    r, g, b = (max(0, min(255, round(channel))) for channel in rgb)
    return f"#{r:02x}{g:02x}{b:02x}"


def _channel_luminance(channel: float) -> float:
    normalised = channel / 255
    return normalised / 12.92 if normalised <= 0.03928 else ((normalised + 0.055) / 1.055) ** 2.4


def relative_luminance(hex_color: str) -> float:
    """WCAG relative luminance of *hex_color* (0 = black, 1 = white)."""
    r, g, b = _parse(hex_color)
    return (
        0.2126 * _channel_luminance(r)
        + 0.7152 * _channel_luminance(g)
        + 0.0722 * _channel_luminance(b)
    )


def is_dark(hex_color: str) -> bool:
    return relative_luminance(hex_color) < 0.35


def contrast_ratio(first: str, second: str) -> float:
    light, dark = sorted((relative_luminance(first), relative_luminance(second)), reverse=True)
    return (light + 0.05) / (dark + 0.05)


def blend(first: str, second: str, amount: float) -> str:
    """Mix *first* toward *second* by *amount* (0..1) in sRGB."""
    amount = max(0.0, min(1.0, amount))
    a = _parse(first)
    b = _parse(second)
    return to_hex(tuple(a[i] + (b[i] - a[i]) * amount for i in range(3)))


def ensure_contrast(color: str, against: str, minimum: float = 3.0) -> str:
    """Return *color*, nudged toward white/black until it contrasts enough."""
    if contrast_ratio(color, against) >= minimum:
        return color
    target = "#ffffff" if is_dark(against) else "#000000"
    for step in range(1, 21):
        candidate = blend(color, target, step / 20)
        if contrast_ratio(candidate, against) >= minimum:
            return candidate
    return target


def palette_key(colors: TerminalColors) -> tuple:
    """A hashable fingerprint used to skip redundant re-themes."""
    return (colors.background, colors.foreground, tuple(colors.palette))


def theme_from_colors(colors: TerminalColors) -> Theme:
    """Build a Textual :class:`Theme` from a :class:`TerminalColors`."""
    background = colors.background
    foreground = ensure_contrast(colors.foreground, background, 4.5)
    dark = is_dark(background)
    toward = "#ffffff" if dark else "#000000"

    def pick(index: int, minimum: float = 3.0) -> str:
        return ensure_contrast(colors.palette[index], background, minimum)

    primary = pick(6)
    secondary = pick(5, 2.5)
    accent = pick(4, 2.5)
    panel = blend(background, toward, 0.10)

    return Theme(
        name=THEME_NAME,
        primary=primary,
        secondary=secondary,
        accent=accent,
        error=ensure_contrast(colors.palette[1], background, 2.5),
        success=ensure_contrast(colors.palette[2], background, 2.5),
        warning=ensure_contrast(colors.palette[3], background, 2.5),
        foreground=foreground,
        background=background,
        surface=blend(background, toward, 0.05 if dark else 0.04),
        panel=panel,
        boost=blend(background, toward, 0.17),
        dark=dark,
        variables={
            "ansi-background": background,
            "ansi-foreground": foreground,
            "border": primary,
            "border-blurred": panel,
            "scrollbar": primary,
            "scrollbar-background": panel,
            "scrollbar-hover": secondary,
            "scrollbar-active": accent,
            "block-cursor-foreground": background,
            "block-cursor-background": primary,
            "input-selection-background": secondary,
            "input-selection-foreground": background,
            "screen-selection-background": primary,
            "screen-selection-foreground": background,
        },
    )


def apply_app_theme(app, colors: TerminalColors) -> bool:
    """Register and activate the theme for *colors*; return whether it changed."""
    key = palette_key(colors)
    if getattr(app, "_termux_palette_key", None) == key:
        return False
    app._termux_palette_key = key
    app.register_theme(theme_from_colors(colors))
    app.theme = THEME_NAME
    return True
