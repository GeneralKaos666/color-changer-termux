"""Filesystem locations used by termux-colors.

Every path funnels through :func:`termux_dir` so tests can point the whole
application at a temporary directory via the ``TERMUX_HOME`` env var.
"""

from __future__ import annotations

import os
from pathlib import Path


def termux_dir() -> Path:
    """Return the Termux configuration directory (``~/.termux``)."""
    override = os.environ.get("TERMUX_HOME")
    if override:
        return Path(override).expanduser()
    return Path(os.environ.get("HOME", str(Path.home()))) / ".termux"


def colors_root() -> Path:
    """Directory holding the user's theme library (``~/.termux/colors``)."""
    return termux_dir() / "colors"


def colors_properties_path() -> Path:
    """The active colour scheme file (``~/.termux/colors.properties``)."""
    return termux_dir() / "colors.properties"


def termux_properties_path() -> Path:
    """The main Termux app settings file."""
    return termux_dir() / "termux.properties"


def backups_dir() -> Path:
    """Where timestamped backups are written."""
    return termux_dir() / "backups"
