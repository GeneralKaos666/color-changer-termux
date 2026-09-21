"""Rich-text helpers for banners, swatches and the mock terminal preview."""

from __future__ import annotations

from rich.text import Text

from .models.colors import TerminalColors
from .theme import ensure_contrast, relative_luminance


def theme_color(app, name: str, fallback: str = "#ffffff") -> str:
    """Resolve a Textual theme colour to a concrete hex string for Rich.

    Rich spans cannot parse Textual ``$variable`` styles, so callers must
    resolve them first.
    """
    try:
        theme = app.get_theme(app.theme)
        value = getattr(theme, name, None)
    except Exception:  # pragma: no cover - defensive
        return fallback
    if value is None:
        return fallback
    text = str(value)
    return text if text.startswith("#") else fallback

# A minimal 6x5 block font, only the glyphs used by the wordmark.
_GLYPHS: dict[str, list[str]] = {
    "T": ["██████", "  ██  ", "  ██  ", "  ██  ", "  ██  "],
    "E": ["██████", "██    ", "████  ", "██    ", "██████"],
    "R": ["█████ ", "██  ██", "█████ ", "██ ██ ", "██  ██"],
    "M": ["██   ██", "███ ███", "██ █ ██", "██   ██", "██   ██"],
    "U": ["██  ██", "██  ██", "██  ██", "██  ██", "██████"],
    "X": ["██  ██", " ████ ", "  ██  ", " ████ ", "██  ██"],
    "C": [" █████", "██    ", "██    ", "██    ", " █████"],
    "O": [" ████ ", "██  ██", "██  ██", "██  ██", " ████ "],
    "L": ["██    ", "██    ", "██    ", "██    ", "██████"],
    "S": [" █████", "██    ", " ████ ", "    ██", "█████ "],
}


def contrast_color(hex_color: str) -> str:
    """Return black or white, whichever reads better on *hex_color*."""
    return "#000000" if relative_luminance(hex_color) > 0.45 else "#ffffff"


def _word(word: str, colors: TerminalColors, offset: int) -> list[Text]:
    rows = [Text() for _ in range(5)]
    for index, letter in enumerate(word):
        glyph = _GLYPHS.get(letter.upper())
        if glyph is None:
            continue
        color = ensure_contrast(colors.palette[(index + offset) % len(colors.palette)], colors.background, 3.0)
        for row, pattern in enumerate(glyph):
            rows[row].append(pattern, style=f"bold {color}")
            rows[row].append(" ")
    return rows


def banner_text(
    colors: TerminalColors,
    width: int = 80,
    stacked: bool = False,
    compact: bool = False,
) -> Text:
    """Render the "TERMUX COLORS" wordmark in the palette's colours.

    ``compact`` shows just "TERMUX", ``stacked`` puts the two words on
    separate rows, otherwise they share one row when it fits.
    """
    if compact:
        top = _word("TERMUX", colors, 0)
        bottom: list[Text] = []
    elif stacked or width < 88:
        top = _word("TERMUX", colors, 0)
        bottom = _word("COLORS", colors, 6)
    else:
        top = _word("TERMUX", colors, 0)
        right = _word("COLORS", colors, 6)
        top = [Text.assemble(line, "  ", right[index]) for index, line in enumerate(top)]
        bottom = []
    text = Text()
    for line in top:
        text.append_text(line)
        text.append("\n")
    for line in bottom:
        text.append_text(line)
        text.append("\n")
    return text


def palette_strip(colors: TerminalColors, blocks: str = "   ") -> Text:
    """A single-line strip of the 16 palette colours."""
    text = Text()
    for color in colors.palette:
        text.append(blocks, style=f"on {color}")
    return text


def colors_preview_text(colors: TerminalColors) -> Text:
    """Swatch grid plus background/foreground/cursor samples."""
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
    return text


def swatch_text(hex_color: str, width: int = 3) -> Text:
    """A small inline colour chip, used beside hex inputs."""
    return Text(" " * width, style=f"on {hex_color}")


def mock_lines(colors: TerminalColors) -> list[Text]:
    """A fake shell session rendered in *colors* (one ``Text`` per line)."""
    palette = colors.palette
    background = colors.background

    def ink(index: int) -> str:
        return ensure_contrast(palette[index], background, 2.5)

    prompt = Text()
    prompt.append("user@termux", style=f"bold {ink(2)}")
    prompt.append(":", style=f"{colors.foreground}")
    prompt.append("~", style=f"bold {ink(4)}")
    prompt.append("$ ", style=f"{colors.foreground}")
    prompt.append("ls", style=f"{colors.foreground}")

    listing = Text("  ")
    listing.append("Documents  ", style=f"bold {ink(4)}")
    listing.append("Downloads  ", style=f"bold {ink(4)}")
    listing.append("projects  ", style=f"bold {ink(4)}")
    listing.append("notes.md  ", style=f"{colors.foreground}")
    listing.append("README  ", style=f"{colors.foreground}")

    git = Text()
    git.append("user@termux", style=f"bold {ink(2)}")
    git.append(":", style=f"{colors.foreground}")
    git.append("~/projects", style=f"bold {ink(4)}")
    git.append("$ ", style=f"{colors.foreground}")
    git.append("git status", style=f"{colors.foreground}")

    branch = Text("  On branch ", style=f"{colors.foreground}")
    branch.append("main", style=f"bold {ink(6)}")

    clean = Text("  nothing to commit, working tree clean", style=f"{ink(2)}")

    tests = Text("  ")
    tests.append("✓ ", style=f"bold {ink(2)}")
    tests.append("31 passed", style=f"{ink(2)}")
    tests.append(" in 2.5s", style=f"{colors.foreground}")

    error = Text("  ")
    error.append("make: ", style=f"bold {ink(1)}")
    error.append("*** [Makefile:12: build] Error 1", style=f"{ink(1)}")

    return [prompt, listing, git, branch, clean, tests, error]


def mock_terminal_text(colors: TerminalColors, lines: int | None = None) -> Text:
    """The full mock session, optionally limited to the first *lines* lines."""
    text = Text()
    session = mock_lines(colors)
    if lines is not None:
        session = session[:lines]
    for line in session:
        text.append_text(line)
        text.append("\n")
    return text
