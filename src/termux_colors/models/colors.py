"""Data models for Termux colour schemes and app settings."""

from __future__ import annotations

import re

HEX_RE = re.compile(r"^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$")

PALETTE_SIZE = 16
PALETTE_KEYS = [f"color{i}" for i in range(PALETTE_SIZE)]

# Fallback palette based on the common xterm defaults.
DEFAULT_PALETTE = [
    "#000000",
    "#cc0000",
    "#4e9a06",
    "#c4a000",
    "#3465a4",
    "#75507b",
    "#06989a",
    "#d3d7cf",
    "#555753",
    "#ef2929",
    "#8ae234",
    "#fce94f",
    "#729fcf",
    "#ad7fa8",
    "#34e2e2",
    "#eeeeee",
]

ANSI_NAMES = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright black",
    "bright red",
    "bright green",
    "bright yellow",
    "bright blue",
    "bright magenta",
    "bright cyan",
    "bright white",
]


def is_valid_hex(value: str) -> bool:
    """Return True when *value* is a ``#rgb`` or ``#rrggbb`` colour."""
    return bool(HEX_RE.match(value.strip()))


def normalize_hex(value: str) -> str:
    """Validate and canonicalise a hex colour, raising ``ValueError``."""
    cleaned = value.strip()
    if not is_valid_hex(cleaned):
        raise ValueError(f"invalid hex colour: {value!r}")
    return cleaned.lower()


class TerminalColors:
    """The colour values stored in ``colors.properties``.

    Unknown keys found in a parsed file are kept in :attr:`extra` so a
    round-trip through the app never silently drops user data.
    """

    def __init__(
        self,
        background: str = "#000000",
        foreground: str = "#ffffff",
        cursor: str = "#ffffff",
        palette: list[str] | None = None,
        extra: dict[str, str] | None = None,
    ) -> None:
        self.background = background
        self.foreground = foreground
        self.cursor = cursor
        self.palette = list(palette) if palette is not None else list(DEFAULT_PALETTE)
        if len(self.palette) != PALETTE_SIZE:
            raise ValueError(f"palette must have {PALETTE_SIZE} entries")
        self.extra = dict(extra or {})

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, TerminalColors):
            return NotImplemented
        return (
            self.background == other.background
            and self.foreground == other.foreground
            and self.cursor == other.cursor
            and self.palette == other.palette
            and self.extra == other.extra
        )

    def __repr__(self) -> str:
        return (
            f"TerminalColors(background={self.background!r}, "
            f"foreground={self.foreground!r}, cursor={self.cursor!r}, "
            f"palette={self.palette!r})"
        )

    def as_dict(self) -> dict[str, str]:
        data = {"background": self.background, "foreground": self.foreground,
               "cursor": self.cursor}
        for key, value in zip(PALETTE_KEYS, self.palette):
            data[key] = value
        data.update(self.extra)
        return data

    def copy(self) -> "TerminalColors":
        return TerminalColors(
            background=self.background,
            foreground=self.foreground,
            cursor=self.cursor,
            palette=list(self.palette),
            extra=dict(self.extra),
        )
