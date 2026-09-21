"""Read and write the curated subset of ``termux.properties``."""

from __future__ import annotations

from pathlib import Path

from ..models.app_theme import CURATED_KEYS, AppTheme
from .properties import parse_active, set_value, unset_value


def read_app_theme(path: Path) -> AppTheme:
    """Read the curated keys from *path* (empty theme when missing)."""
    if not path.exists():
        return AppTheme({})
    pairs = parse_active(path.read_text(encoding="utf-8", errors="replace"))
    return AppTheme({key: pairs[key] for key in CURATED_KEYS if key in pairs})


def update_theme_text(text: str, values: dict[str, str]) -> str:
    """Apply *values* to *text* line-by-line, preserving unmanaged lines.

    A value of ``None`` is not expected; pass an empty string to remove a key
    (it is commented out rather than deleted).
    """
    result = text
    for key, value in values.items():
        if key not in CURATED_KEYS:
            continue
        if value == "":
            result = unset_value(result, key)
        else:
            result = set_value(result, key, value)
    return result


def write_app_theme(path: Path, values: dict[str, str], preserve: bool = True) -> None:
    """Write curated *values* to *path*, preserving other lines by default."""
    path.parent.mkdir(parents=True, exist_ok=True)
    if preserve and path.exists():
        text = path.read_text(encoding="utf-8", errors="replace")
    else:
        text = "# Termux properties managed by termux-colors\n"
    text = update_theme_text(text, values)
    path.write_text(text, encoding="utf-8")
