"""Edit the individual colours of the active theme."""

from __future__ import annotations

import re

from rich.text import Text
from textual import work
from textual.app import ComposeResult
from textual.containers import Horizontal, VerticalScroll
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Input, Label, Static

from ..library import active_theme_path, apply_theme, reload_termux
from ..models.colors import (
    ANSI_NAMES,
    PALETTE_KEYS,
    TerminalColors,
    is_valid_hex,
    normalize_hex,
)
from ..paths import backups_dir, colors_properties_path, colors_root
from ..persistence.backup import make_backup
from ..persistence.colors_io import read_colors, write_colors
from ..theme import apply_app_theme
from ..widgets import (
    colors_preview_text,
    mock_lines,
    palette_strip,
    swatch_text,
)
from .dialogs import ConfirmScreen, MessageScreen, PromptScreen

BASIC_KEYS = ("background", "foreground", "cursor")


def _sanitize(name: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9._-]+", "-", name.strip()).strip("-")
    return cleaned or "custom"


class TerminalEditorScreen(Screen):
    """Edit background, foreground, cursor and the 16 ANSI colours."""

    BINDINGS = [("escape", "app.pop_screen", "Back")]

    def __init__(self, title: str = "Terminal Colors") -> None:
        super().__init__()
        self.screen_title = title
        active = active_theme_path(colors_properties_path())
        self.active_path = active
        self.base = read_colors(active) if active else TerminalColors()
        self._inputs: dict[str, Input] = {}

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with Horizontal(id="editor"):
            with VerticalScroll(id="editor-form"):
                yield Static("basics", classes="section-rule")
                for key in BASIC_KEYS:
                    yield Label(key, classes="field-label")
                    with Horizontal(classes="field-row"):
                        yield Static(id=f"swatch-{key}", classes="field-swatch")
                        yield Input(value=self.base.as_dict().get(key, ""), id=f"field-{key}")
                yield Static("ansi palette", classes="section-rule")
                for index, key in enumerate(PALETTE_KEYS):
                    yield Label(f"{key}  ({ANSI_NAMES[index]})", classes="field-label")
                    with Horizontal(classes="field-row"):
                        yield Static(id=f"swatch-{key}", classes="field-swatch")
                        yield Input(value=self.base.palette[index], id=f"field-{key}")
            with VerticalScroll(id="editor-preview"):
                yield Static(id="preview-name")
                yield Static(id="preview-mock")
                yield Static(id="preview-swatches")
                yield Static(id="preview-strip")
        with Horizontal(id="actions"):
            yield Button("Save", variant="primary", id="save")
            yield Button("Save As…", id="save-as")
            yield Button("Revert", id="revert")
            yield Button("Back", id="back")
        yield Footer()

    def on_mount(self) -> None:
        for key in self._field_keys():
            self._inputs[key] = self.query_one(f"#field-{key}", Input)
        self.query_one("#editor-form", VerticalScroll).border_title = self.screen_title.lower()
        self.query_one("#editor-preview", VerticalScroll).border_title = "preview"
        self._update_preview()

    def _field_keys(self) -> list[str]:
        return [*BASIC_KEYS, *PALETTE_KEYS]

    def on_input_changed(self, event: Input.Changed) -> None:
        self._update_preview()

    def _current_colors(self) -> TerminalColors:
        colors = self.base.copy()
        for key, widget in self._inputs.items():
            value = widget.value.strip()
            if not is_valid_hex(value):
                continue
            if key in BASIC_KEYS:
                setattr(colors, key, value)
            else:
                colors.palette[PALETTE_KEYS.index(key)] = value
        return colors

    def _update_preview(self) -> None:
        colors = self._current_colors()
        apply_app_theme(self.app, colors)

        for key, widget in self._inputs.items():
            value = widget.value.strip()
            valid = is_valid_hex(value)
            widget.set_class(bool(value) and not valid, "input-error")
            swatch = self.query_one(f"#swatch-{key}", Static)
            swatch.update(swatch_text(value) if valid else Text(" "))

        self.query_one("#preview-name", Static).update("[b]live preview[/]")
        session = mock_lines(colors)[:5]
        mock = Text()
        for line in session:
            mock.append_text(line)
            mock.append("\n")
        self.query_one("#preview-mock", Static).update(mock)
        self.query_one("#preview-swatches", Static).update(colors_preview_text(colors))
        self.query_one("#preview-strip", Static).update(palette_strip(colors))

    def _collect(self) -> TerminalColors:
        colors = self.base.copy()
        invalid: list[str] = []
        for key in self._field_keys():
            value = self._inputs[key].value.strip()
            try:
                normalized = normalize_hex(value)
            except ValueError:
                invalid.append(key)
                continue
            if key in BASIC_KEYS:
                setattr(colors, key, normalized)
            else:
                colors.palette[PALETTE_KEYS.index(key)] = normalized
        if invalid:
            raise ValueError("Invalid hex colour for: " + ", ".join(invalid))
        return colors

    def on_button_pressed(self, event: Button.Pressed) -> None:
        button_id = event.button.id
        if button_id == "save":
            self.action_save()
        elif button_id == "save-as":
            self.action_save_as()
        elif button_id == "revert":
            self.action_revert()
        elif button_id == "back":
            self.app.pop_screen()

    def action_revert(self) -> None:
        self.base = read_colors(self.active_path) if self.active_path else TerminalColors()
        values = self.base.as_dict()
        for key, widget in self._inputs.items():
            widget.value = values.get(key, "")
        self._update_preview()
        self.notify("Reverted to the saved values.")

    async def _maybe_backup(self) -> None:
        colors_properties = colors_properties_path()
        if colors_properties.exists() or colors_properties.is_symlink():
            confirmed = await self.app.push_screen_wait(
                ConfirmScreen(
                    "Back up the current config before saving?",
                    confirm_label="Back up",
                    cancel_label="Skip",
                )
            )
            if confirmed:
                backup = make_backup(colors_properties, backups_dir(), prefix="colors")
                if backup:
                    self.notify(f"Backed up to {backup.name}")

    def _write_and_apply(self, target, colors: TerminalColors) -> None:
        write_colors(target, colors, preserve=True)
        colors_properties = colors_properties_path()
        if target.resolve() != colors_properties.resolve():
            apply_theme(target, colors_properties)
        reloaded = reload_termux()
        message = f"Saved {target.name}."
        if not reloaded:
            message += " Run termux-reload-settings to refresh."
        self.notify(message)

    async def _save_as_flow(self, colors: TerminalColors) -> None:
        name = await self.app.push_screen_wait(
            PromptScreen("Name for the new theme:", initial="custom", placeholder="my-theme")
        )
        if not name:
            return
        target = colors_root() / "custom" / f"{_sanitize(name)}.properties"
        if target.exists():
            confirmed = await self.app.push_screen_wait(
                ConfirmScreen(f"{target.name} already exists. Overwrite?", confirm_label="Overwrite")
            )
            if not confirmed:
                return
        await self._maybe_backup()
        self._write_and_apply(target, colors)
        self.active_path = target.resolve()

    @work(exclusive=True)
    async def action_save(self) -> None:
        try:
            colors = self._collect()
        except ValueError as error:
            await self.app.push_screen_wait(MessageScreen(str(error), "Invalid colour"))
            return
        if self.active_path is None:
            await self._save_as_flow(colors)
            return
        await self._maybe_backup()
        self._write_and_apply(self.active_path, colors)

    @work(exclusive=True)
    async def action_save_as(self) -> None:
        try:
            colors = self._collect()
        except ValueError as error:
            await self.app.push_screen_wait(MessageScreen(str(error), "Invalid colour"))
            return
        await self._save_as_flow(colors)
