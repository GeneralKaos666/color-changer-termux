"""Line-preserving helpers for ``*.properties`` files.

Termux property files contain comments and keys we do not manage (for
example ``extra-keys`` with backslash continuations).  These helpers edit a
single key at a time without disturbing any other line.
"""

from __future__ import annotations

import re

_ACTIVE_RE = re.compile(
    r"^(?P<indent>[ \t]*)(?P<key>[A-Za-z0-9_.\-]+)(?P<sep>[ \t]*=[ \t]*)(?P<value>.*)$"
)
_COMMENTED_RE = re.compile(
    r"^(?P<indent>[ \t]*)#[ \t]*(?P<key>[A-Za-z0-9_.\-]+)(?P<sep>[ \t]*=[ \t]*)(?P<value>.*)$"
)


def parse_active(text: str) -> dict[str, str]:
    """Return the active (non-commented) ``key=value`` pairs in *text*."""
    pairs: dict[str, str] = {}
    for line in text.splitlines():
        match = _ACTIVE_RE.match(line)
        if match:
            pairs[match.group("key")] = match.group("value").strip()
    return pairs


def _find(lines: list[str], key: str) -> tuple[int, re.Match[str]] | None:
    for index, line in enumerate(lines):
        match = _ACTIVE_RE.match(line)
        if match and match.group("key") == key:
            return index, match
        commented = _COMMENTED_RE.match(line)
        if commented and commented.group("key") == key:
            return index, commented
    return None


def set_value(text: str, key: str, value: str) -> str:
    """Return *text* with *key* set to *value*, preserving all other lines.

    An existing line (active or commented) keeps its separator style, so
    ``background=#000`` stays compact while ``use-black-ui = true`` keeps its
    spaces.  New keys are appended in Termux's compact ``key=value`` form.
    """
    newline = "\r\n" if "\r\n" in text else "\n"
    trailing_newline = text.endswith(("\n", "\r"))
    lines = text.splitlines()
    found = _find(lines, key)
    if found:
        match = found[1]
        lines[found[0]] = f"{match.group('indent')}{key}{match.group('sep')}{value}"
    else:
        lines.append(f"{key}={value}")
    result = newline.join(lines)
    if trailing_newline or not text:
        result += newline
    return result


def unset_value(text: str, key: str) -> str:
    """Comment out *key* if present, leaving the value in place."""
    newline = "\r\n" if "\r\n" in text else "\n"
    trailing_newline = text.endswith(("\n", "\r"))
    lines = text.splitlines()
    found = _find(lines, key)
    if found and _ACTIVE_RE.match(lines[found[0]]):
        match = found[1]
        lines[found[0]] = f"{match.group('indent')}# {key} = {match.group('value').strip()}"
    result = newline.join(lines)
    if trailing_newline or not text:
        result += newline
    return result
