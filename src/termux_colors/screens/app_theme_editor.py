"""Edit the curated ``termux.properties`` settings."""

from __future__ import annotations

from textual import work
from textual.app import ComposeResult
from textual.containers import Horizontal, VerticalScroll
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Input, Label, Select, Static

from ..library import reload_termux
from ..models.app_theme import CURATED_FIELDS, FieldSpec, validate_value
from ..paths import backups_dir, termux_properties_path
from ..persistence.backup import make_backup
from ..persistence.theme_io import read_app_theme, write_app_theme
from .dialogs import ConfirmScreen, MessageScreen


class AppThemeEditorScreen(Screen):
    """Form editor for a safe subset of ``termux.properties``."""

    BINDINGS = [("escape", "app.pop_screen", "Back")]

    def __init__(self) -> None:
        super().__init__()
        self.path = termux_properties_path()
        self.theme = read_app_theme(self.path)
        self._widgets: dict[str, object] = {}

    def _next_value(self, spec: FieldSpec) -> str:
        return self.theme.get(spec.key, "")

    def _build_select(self, spec: FieldSpec, current: str) -> Select:
        options = [("true", "true"), ("false", "false")] if spec.kind == "bool" else [
            (choice, choice) for choice in spec.choices
        ]
        valid = {value for _, value in options}
        value = current if current in valid else Select.NULL
        return Select(options, value=value, allow_blank=True, id=f"field-{spec.key}")

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with VerticalScroll(id="editor-form"):
            yield Static("App Theme", classes="panel-title")
            yield Static(f"Editing {self.path}", classes="hint")
            for spec in CURATED_FIELDS:
                current = self._next_value(spec)
                yield Label(spec.label + (f"  ({spec.help})" if spec.help else ""))
                if spec.kind in ("bool", "choice"):
                    yield self._build_select(spec, current)
                else:
                    yield Input(value=current, id=f"field-{spec.key}")
        with Horizontal(id="actions"):
            yield Button("Save", variant="primary", id="save")
            yield Button("Revert", id="revert")
            yield Button("Back", id="back")
        yield Footer()

    def on_mount(self) -> None:
        for spec in CURATED_FIELDS:
            self._widgets[spec.key] = self.query_one(f"#field-{spec.key}")

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "save":
            self.action_save()
        elif event.button.id == "revert":
            self.action_revert()
        elif event.button.id == "back":
            self.app.pop_screen()

    def _collect(self) -> dict[str, str]:
        values: dict[str, str] = {}
        for spec in CURATED_FIELDS:
            widget = self._widgets[spec.key]
            raw = widget.value if isinstance(widget, Select) else widget.value.strip()
            if not isinstance(raw, str):
                raw = ""
            values[spec.key] = validate_value(spec, raw)
        return values

    def action_revert(self) -> None:
        self.theme = read_app_theme(self.path)
        for spec in CURATED_FIELDS:
            widget = self._widgets[spec.key]
            current = self.theme.get(spec.key, "")
            if isinstance(widget, Select):
                options = {"true", "false"} if spec.kind == "bool" else set(spec.choices)
                widget.value = current if current in options else Select.NULL
            else:
                widget.value = current
        self.notify("Reverted to the file's current values.")

    @work(exclusive=True)
    async def action_save(self) -> None:
        try:
            values = self._collect()
        except ValueError as error:
            await self.app.push_screen_wait(MessageScreen(str(error), "Invalid setting"))
            return
        if self.path.exists():
            confirmed = await self.app.push_screen_wait(
                ConfirmScreen(
                    "Back up termux.properties before saving?",
                    confirm_label="Back up",
                    cancel_label="Skip",
                )
            )
            if confirmed:
                backup = make_backup(self.path, backups_dir(), prefix="termux")
                if backup:
                    self.notify(f"Backed up to {backup.name}")
        write_app_theme(self.path, values)
        reloaded = reload_termux()
        message = "Saved termux.properties."
        if not reloaded:
            message += " Run termux-reload-settings to refresh."
        self.notify(message)
