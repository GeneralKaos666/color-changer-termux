from termux_colors.models.colors import TerminalColors
from termux_colors.persistence.colors_io import (
    parse_colors,
    render_colors,
    update_colors_text,
    write_colors,
)

SAMPLE = """\
# SilkCircuit Vibrant for Termux
# Do not edit by hand.

background=#0f0c1a
foreground=#f8f8f2
cursor=#00ffcc

color0=#0f0c1a
color1=#ff3366
color15=#ffffff

selection-background=#333333
"""


def test_parse_keeps_managed_and_extra_keys():
    colors = parse_colors(SAMPLE)
    assert colors.background == "#0f0c1a"
    assert colors.foreground == "#f8f8f2"
    assert colors.cursor == "#00ffcc"
    assert colors.palette[0] == "#0f0c1a"
    assert colors.palette[1] == "#ff3366"
    assert colors.palette[15] == "#ffffff"
    assert colors.extra == {"selection-background": "#333333"}


def test_parse_uses_defaults_for_missing_colors():
    colors = parse_colors("background=#000000\n")
    assert len(colors.palette) == 16
    assert colors.foreground == "#ffffff"


def test_update_preserves_comments_and_extra_keys():
    colors = parse_colors(SAMPLE)
    colors.background = "#123456"
    colors.palette[1] = "#abcdef"

    updated = update_colors_text(SAMPLE, colors)

    assert "# SilkCircuit Vibrant for Termux" in updated
    assert "background=#123456" in updated
    assert "color1=#abcdef" in updated
    assert "selection-background=#333333" in updated


def test_update_appends_missing_key():
    colors = TerminalColors(background="#010203")
    updated = update_colors_text("foreground=#ffffff\n", colors)
    assert "background=#010203" in updated
    assert "foreground=#ffffff" in updated


def test_render_round_trips():
    original = parse_colors(SAMPLE)
    reparsed = parse_colors(render_colors(original))
    assert reparsed.background == original.background
    assert reparsed.palette == original.palette
    assert reparsed.extra == original.extra


def test_write_colors_preserves_existing_file(tmp_path):
    path = tmp_path / "colors.properties"
    path.write_text(SAMPLE, encoding="utf-8")
    colors = parse_colors(SAMPLE)
    colors.cursor = "#ff0000"

    write_colors(path, colors)

    text = path.read_text(encoding="utf-8")
    assert "cursor=#ff0000" in text
    assert "# SilkCircuit Vibrant for Termux" in text
