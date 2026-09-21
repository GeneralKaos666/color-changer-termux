"""Model and schema for the ``termux.properties`` app-theme editor."""

from __future__ import annotations

from dataclasses import dataclass, field

BOOL_CHOICES = ["true", "false"]


@dataclass(frozen=True)
class FieldSpec:
    """Describes one curated ``termux.properties`` key."""

    key: str
    label: str
    kind: str = "text"  # text | bool | choice | int
    choices: tuple[str, ...] = field(default_factory=tuple)
    minimum: int | None = None
    maximum: int | None = None
    help: str = ""


CURATED_FIELDS: tuple[FieldSpec, ...] = (
    FieldSpec("use-black-ui", "Black UI", "bool", help="Use a black background in the app UI."),
    FieldSpec("fullscreen", "Fullscreen", "bool", help="Hide the status bar."),
    FieldSpec(
        "terminal-cursor-style",
        "Cursor style",
        "choice",
        choices=("block", "bar", "underline"),
    ),
    FieldSpec(
        "terminal-cursor-blink-rate",
        "Cursor blink rate (ms)",
        "int",
        minimum=0,
        maximum=2000,
        help="0 disables blinking.",
    ),
    FieldSpec("terminal-margin-horizontal", "Margin horizontal", "int", minimum=0, maximum=50),
    FieldSpec("terminal-margin-vertical", "Margin vertical", "int", minimum=0, maximum=50),
    FieldSpec(
        "terminal-transcript-rows",
        "Transcript rows",
        "int",
        minimum=100,
        maximum=50000,
    ),
    FieldSpec(
        "bell-character",
        "Bell",
        "choice",
        choices=("beep", "vibrate", "ignore"),
        help="What happens when an application rings the bell.",
    ),
    FieldSpec("allow-external-apps", "Allow external apps", "bool"),
)

CURATED_KEYS: tuple[str, ...] = tuple(spec.key for spec in CURATED_FIELDS)
CURATED_BY_KEY: dict[str, FieldSpec] = {spec.key: spec for spec in CURATED_FIELDS}


class AppTheme:
    """Curated ``termux.properties`` values read from the user's file."""

    def __init__(self, values: dict[str, str] | None = None) -> None:
        self.values: dict[str, str] = dict(values or {})

    def get(self, key: str, default: str = "") -> str:
        return self.values.get(key, default)

    def set(self, key: str, value: str) -> None:
        self.values[key] = value

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, AppTheme):
            return NotImplemented
        return self.values == other.values

    def __repr__(self) -> str:
        return f"AppTheme({self.values!r})"


def validate_value(spec: FieldSpec, value: str) -> str:
    """Validate *value* for *spec*, returning the canonical form.

    Raises ``ValueError`` with a human-readable message when invalid.
    """
    value = value.strip()
    if value == "":
        return ""
    if spec.kind == "bool":
        lowered = value.lower()
        if lowered not in BOOL_CHOICES:
            raise ValueError(f"{spec.label} must be true or false")
        return lowered
    if spec.kind == "choice":
        if value not in spec.choices:
            options = ", ".join(spec.choices)
            raise ValueError(f"{spec.label} must be one of: {options}")
        return value
    if spec.kind == "int":
        try:
            number = int(value)
        except ValueError:
            raise ValueError(f"{spec.label} must be a whole number") from None
        if spec.minimum is not None and number < spec.minimum:
            raise ValueError(f"{spec.label} must be >= {spec.minimum}")
        if spec.maximum is not None and number > spec.maximum:
            raise ValueError(f"{spec.label} must be <= {spec.maximum}")
        return str(number)
    return value
