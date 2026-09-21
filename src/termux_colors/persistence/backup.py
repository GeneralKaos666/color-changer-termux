"""Timestamped backups of Termux config files."""

from __future__ import annotations

import os
from datetime import datetime
from pathlib import Path


def timestamp() -> str:
    """Return a filesystem-safe local timestamp."""
    return datetime.now().strftime("%Y-%m-%d-%H%M%S")


def make_backup(path: Path, backups_dir: Path, prefix: str | None = None) -> Path | None:
    """Copy *path* into *backups_dir* and return the new path.

    Returns ``None`` when *path* does not exist.  Symlinks are followed so the
    backup contains the real colour data rather than a dangling link.
    """
    if not path.exists():
        return None
    name = prefix or path.stem
    suffix = path.suffix or ".properties"
    backups_dir.mkdir(parents=True, exist_ok=True)
    destination = backups_dir / f"{name}-{timestamp()}{suffix}"
    counter = 1
    while destination.exists():
        destination = backups_dir / f"{name}-{timestamp()}-{counter}{suffix}"
        counter += 1
    destination.write_bytes(path.read_bytes())
    return destination


def list_backups(backups_dir: Path) -> list[Path]:
    """Return existing backups, newest first."""
    if not backups_dir.is_dir():
        return []
    files = [p for p in backups_dir.iterdir() if p.is_file()]
    return sorted(files, key=os.path.getmtime, reverse=True)
