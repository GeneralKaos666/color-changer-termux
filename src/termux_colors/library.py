"""Scan the user's theme library and activate themes by symlink."""

from __future__ import annotations

import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Theme:
    """One theme file discovered in the colour library."""

    name: str
    path: Path
    group: str = "root"

    @property
    def label(self) -> str:
        if self.group and self.group != "root":
            return f"{self.group}/{self.name}"
        return self.name


def _is_backup(path: Path) -> bool:
    name = path.name.lower()
    return "backup" in name or name.endswith(".bak") or ".bak-" in name


def scan_themes(root: Path) -> list[Theme]:
    """Find every usable ``*.properties`` theme under *root*.

    The active ``colors.properties`` file, backups, and dotfiles are skipped.
    Results are ordered by group then name.
    """
    if not root.is_dir():
        return []
    themes: list[Theme] = []
    for path in root.rglob("*.properties"):
        if not path.is_file() and not path.is_symlink():
            continue
        if path.name == "colors.properties" or _is_backup(path):
            continue
        relative = path.relative_to(root)
        group = relative.parts[0] if len(relative.parts) > 1 else "root"
        if group.startswith("."):
            continue
        themes.append(Theme(name=path.stem, path=path, group=group))
    themes.sort(key=lambda theme: ((theme.group == "root"), theme.group.lower(), theme.name.lower()))
    return themes


def group_themes(themes: list[Theme]) -> dict[str, list[Theme]]:
    """Group themes by their library sub-directory."""
    grouped: dict[str, list[Theme]] = {}
    for theme in themes:
        grouped.setdefault(theme.group, []).append(theme)
    return grouped


def active_theme_path(colors_properties: Path) -> Path | None:
    """Return the real theme file the active ``colors.properties`` resolves to."""
    if not colors_properties.exists() and not colors_properties.is_symlink():
        return None
    return Path(os.path.realpath(colors_properties))


def apply_theme(theme_path: Path, colors_properties: Path) -> None:
    """Point ``colors.properties`` at *theme_path* and reload Termux.

    An existing file or symlink is replaced.  ``colors.properties`` becomes a
    symlink into the library, matching the layout used by ``colors.sh``.  If
    the target already is ``colors.properties`` itself (a plain file the user
    edits in place) this is a no-op so no self-referential link is created.
    """
    theme_path = theme_path.resolve()
    if not theme_path.is_file():
        raise FileNotFoundError(f"theme not found: {theme_path}")
    if colors_properties.exists() or colors_properties.is_symlink():
        if colors_properties.resolve() == theme_path:
            return
    colors_properties.parent.mkdir(parents=True, exist_ok=True)
    if colors_properties.is_symlink() or colors_properties.exists():
        colors_properties.unlink()
    os.symlink(theme_path, colors_properties)


def reload_termux() -> bool:
    """Run ``termux-reload-settings`` when available; return success."""
    executable = shutil.which("termux-reload-settings")
    if not executable:
        return False
    try:
        subprocess.run([executable], check=True, timeout=15)
    except (subprocess.SubprocessError, OSError):
        return False
    return True
