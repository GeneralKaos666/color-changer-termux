"""Browse the theme library and activate a theme."""

from __future__ import annotations

import asyncio

from rich.text import Text
from textual import work
from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Input, Label, ListItem, ListView, Static

from ..library import (
    Theme,
    active_theme_path,
    apply_theme,
    reload_termux,
    scan_themes,
)
from ..paths import backups_dir, colors_properties_path, colors_root
from ..persistence.backup import make_backup
from ..persistence.colors_io import read_colors
from ..presets.built_in import materialize
from ..theme import apply_app_theme
from ..widgets import colors_preview_text, mock_lines, palette_strip, theme_color
from .dialogs import ConfirmScreen, MessageScreen


def load_themes() -> list[Theme]:
    """Return library themes, seeding built-ins when the library is empty."""
    root = colors_root()
    themes = scan_themes(root)
    if not themes:
        materialize(root / "builtin")
        themes = scan_themes(root)
    return themes


class PresetPickerScreen(Screen):
    """List every theme with a live preview and an Apply action."""

    BINDINGS = [
        ("escape", "app.pop_screen", "Back"),
        ("r", "refresh", "Refresh"),
    ]

    def __init__(self) -> None:
        super().__init__()
        self.all_themes: list[Theme] = load_themes()
        self.themes: list[Theme] = list(self.all_themes)
        self._active_path = active_theme_path(colors_properties_path())

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with Horizontal(id="picker"):
            with Vertical(id="picker-list"):
                yield Input(placeholder="filter themes…", id="theme-filter")
                with ListView(id="theme-list"):
                    for theme in self.themes:
                        yield ListItem(Label(self._label_for(theme)))
            with VerticalScroll(id="picker-preview"):
                yield Static(id="preview-name")
                yield Static(id="preview-mock")
                yield Static(id="preview-swatches")
                yield Static(id="preview-strip")
                yield Static(id="details")
        with Horizontal(id="actions"):
            yield Button("Apply", variant="primary", id="apply")
            yield Button("Refresh", id="refresh")
            yield Button("Back", id="back")
        yield Footer()

    def on_mount(self) -> None:
        self._list_view.border_title = f"themes ({len(self.themes)})"
        self._preview.border_title = "preview"
        if self.themes:
            self._list_view.index = 0
            self._show_theme(self.themes[0])
        else:
            self.query_one("#details", Static).update("No themes found.")
        self._list_view.focus()

    # -- helpers ---------------------------------------------------------
    @property
    def _list_view(self) -> ListView:
        return self.query_one("#theme-list", ListView)

    @property
    def _preview(self) -> VerticalScroll:
        return self.query_one("#picker-preview", VerticalScroll)

    def _label_for(self, theme: Theme) -> Text:
        label = Text(theme.label)
        if self._active_path is not None and self._active_path == theme.path.resolve():
            label.append("  ✓ active", style=f"bold {theme_color(self.app, 'success', '#3ecf6d')}")
        return label

    def selected_theme(self) -> Theme | None:
        index = self._list_view.index
        if index is None or not (0 <= index < len(self.themes)):
            return None
        return self.themes[index]

    def _show_theme(self, theme: Theme) -> None:
        try:
            colors = read_colors(theme.path)
        except OSError as error:
            self.query_one("#preview-name", Static).update("")
            self.query_one("#preview-mock", Static).update("")
            self.query_one("#preview-swatches", Static).update("")
            self.query_one("#preview-strip", Static).update("")
            self.query_one("#details", Static).update(f"Could not read: {error}")
            return

        apply_app_theme(self.app, colors)

        is_active = self._active_path is not None and self._active_path == theme.path.resolve()
        badge = "  [b]✓ active[/]" if is_active else ""
        self.query_one("#preview-name", Static).update(f"[b]{theme.label}[/]{badge}")
        self.query_one("#preview-swatches", Static).update(colors_preview_text(colors))
        self.query_one("#preview-strip", Static).update(palette_strip(colors))
        self.query_one("#details", Static).update(f"{theme.group} · {theme.path}")

        if self.size.height >= 20 and self.size.width >= 60:
            self._animate_mock(colors)
        else:
            self.query_one("#preview-mock", Static).update(self._mock_text(colors, None))

    @staticmethod
    def _mock_text(colors, limit: int | None) -> Text:
        session = mock_lines(colors)
        if limit is not None:
            session = session[:limit]
        text = Text()
        for line in session:
            text.append_text(line)
            text.append("\n")
        return text

    @work(exclusive=True, group="mock")
    async def _animate_mock(self, colors) -> None:
        """Type the mock shell session out one line at a time."""
        target = self.query_one("#preview-mock", Static)
        lines = mock_lines(colors)
        text = Text()
        target.update(text)
        for line in lines:
            text.append_text(line)
            text.append("\n")
            target.update(text.copy())
            await asyncio.sleep(0.05)

    # -- events ----------------------------------------------------------
    async def on_input_changed(self, event: Input.Changed) -> None:
        needle = event.value.strip().lower()
        if needle:
            self.themes = [
                theme
                for theme in self.all_themes
                if needle in theme.label.lower()
            ]
        else:
            self.themes = list(self.all_themes)
        await self._list_view.clear()
        for theme in self.themes:
            self._list_view.append(ListItem(Label(self._label_for(theme))))
        self._list_view.border_title = f"themes ({len(self.themes)})"
        if self.themes:
            self._list_view.index = 0
            self._show_theme(self.themes[0])
        else:
            self.query_one("#details", Static).update("No themes match the filter.")

    def on_list_view_highlighted(self, event: ListView.Highlighted) -> None:
        theme = self.selected_theme()
        if theme is not None:
            self._show_theme(theme)

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "apply":
            self.action_apply()
        elif event.button.id == "refresh":
            self.action_refresh()
        elif event.button.id == "back":
            self.app.pop_screen()

    @work(exclusive=True)
    async def action_refresh(self) -> None:
        self.all_themes = load_themes()
        self.themes = list(self.all_themes)
        self._active_path = active_theme_path(colors_properties_path())
        self.query_one("#theme-filter", Input).value = ""
        await self._list_view.clear()
        for theme in self.themes:
            self._list_view.append(ListItem(Label(self._label_for(theme))))
        self._list_view.border_title = f"themes ({len(self.themes)})"
        if self.themes:
            self._list_view.index = 0
            self._show_theme(self.themes[0])
        self.notify(f"Loaded {len(self.themes)} themes.")

    @work(exclusive=True)
    async def action_apply(self) -> None:
        theme = self.selected_theme()
        if theme is None:
            self.notify("Select a theme first.", severity="warning")
            return
        colors_properties = colors_properties_path()
        if colors_properties.exists() or colors_properties.is_symlink():
            confirmed = await self.app.push_screen_wait(
                ConfirmScreen(
                    "Back up the current config before applying?",
                    confirm_label="Back up",
                    cancel_label="Skip",
                )
            )
            if confirmed:
                backup = make_backup(colors_properties, backups_dir(), prefix="colors")
                if backup:
                    self.notify(f"Backed up to {backup.name}")
        try:
            apply_theme(theme.path, colors_properties)
        except OSError as error:
            await self.app.push_screen_wait(MessageScreen(f"Could not apply theme:\n{error}", "Error"))
            return
        reloaded = reload_termux()
        self._active_path = theme.path.resolve()
        self.query_one("#theme-list", ListView).refresh()
        self._show_theme(theme)
        message = f"Applied {theme.label}."
        if not reloaded:
            message += " Run termux-reload-settings to refresh."
        self.notify(message)
