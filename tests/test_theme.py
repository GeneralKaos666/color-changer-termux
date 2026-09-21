from textual.theme import Theme

from termux_colors.models.colors import DEFAULT_PALETTE, TerminalColors
from termux_colors.theme import (
    THEME_NAME,
    blend,
    contrast_ratio,
    ensure_contrast,
    is_dark,
    palette_key,
    relative_luminance,
    theme_from_colors,
)


def test_luminance_extremes():
    assert relative_luminance("#000000") == 0
    assert relative_luminance("#ffffff") == 1
    assert is_dark("#101010") is True
    assert is_dark("#f0f0f0") is False


def test_contrast_ratio_range():
    assert contrast_ratio("#000000", "#ffffff") == 21
    assert contrast_ratio("#123456", "#123456") == 1


def test_ensure_contrast_pushes_apart():
    low = "#0d0d0d"
    fixed = ensure_contrast(low, "#000000", 3.0)
    assert contrast_ratio(fixed, "#000000") >= 3.0


def test_blend_endpoints():
    assert blend("#000000", "#ffffff", 0.0) == "#000000"
    assert blend("#000000", "#ffffff", 1.0) == "#ffffff"
    assert blend("#000000", "#ffffff", 0.5) == "#808080"


def test_theme_from_colors_dark():
    colors = TerminalColors(
        background="#101018",
        foreground="#eeeeee",
        cursor="#00ffcc",
        palette=list(DEFAULT_PALETTE),
    )
    theme = theme_from_colors(colors)
    assert isinstance(theme, Theme)
    assert theme.name == THEME_NAME
    assert theme.dark is True
    assert theme.background == "#101018"
    assert theme.ansi is False
    assert theme.variables["ansi-background"] == "#101018"


def test_theme_from_colors_light_flag():
    colors = TerminalColors(background="#fdf6e3", foreground="#333333")
    theme = theme_from_colors(colors)
    assert theme.dark is False


def test_theme_primary_contrasts_with_background():
    colors = TerminalColors(background="#0f0c1a", foreground="#f8f8f2")
    theme = theme_from_colors(colors)
    assert contrast_ratio(str(theme.primary), theme.background) >= 3.0


def test_theme_foreground_is_readable():
    colors = TerminalColors(background="#000000", foreground="#111111")
    theme = theme_from_colors(colors)
    assert contrast_ratio(str(theme.foreground), theme.background) >= 4.5


def test_theme_all_colours_are_hex():
    colors = TerminalColors(
        background="#0f0c1a",
        foreground="#f8f8f2",
        cursor="#00ffcc",
        palette=list(DEFAULT_PALETTE),
    )
    theme = theme_from_colors(colors)
    for attribute in ("primary", "secondary", "accent", "error", "success", "warning", "surface", "panel", "boost"):
        value = str(getattr(theme, attribute))
        assert value.startswith("#"), f"{attribute} was {value!r}"


def test_palette_key_changes_with_palette():
    first = TerminalColors(background="#000000")
    second = TerminalColors(background="#000000")
    third = TerminalColors(background="#000000")
    third.palette[0] = "#123456"
    assert palette_key(first) == palette_key(second)
    assert palette_key(first) != palette_key(third)
