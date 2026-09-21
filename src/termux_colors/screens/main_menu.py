"""The landing screen."""

from __future__ import annotations

from textual.app import ComposeResult
from textual.containers import Vertical, VerticalScroll
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Static

from ..library import active_colors, active_theme_path
from ..paths import colors_properties_path, colors_root
from ..widgets import banner_text, palette_strip


def _active_name() -> str:
    active = active_theme_path(colors_properties_path())
    if active is None:
        return "default"
    try:
        return str(active.relative_to(colors_root()).with_suffix(""))
    except ValueError:
        return active.name


class MainMenuScreen(Screen):
    """Top-level navigation between the app's tools."""

    BINDINGS = [("q", "app.quit", "Quit"), ("escape", "app.quit", "Quit")]

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with VerticalScroll(id="menu-container"):
            yield Static(id="banner")
            yield Static("change terminal & app colours, live", id="tagline")
            yield Static(id="strip")
            yield Static(id="status")
            with Vertical(id="menu"):
                yield Button("Preset Themes", variant="primary", id="presets")
                yield Button("Terminal Colors", id="terminal")
                yield Button("App Theme", id="app-theme")
                yield Button("Quit", variant="error", id="quit")
        yield Footer()

    def on_mount(self) -> None:
        self._refresh()
        self.query_one("#presets", Button).focus()

    def on_resize(self) -> None:
        self._refresh()

    def _refresh(self) -> None:
        colors = active_colors()
        width = self.size.width
        height = self.size.height
        if width >= 88 and height >= 14:
            stacked, compact = False, False
        elif width >= 40 and height >= 24:
            stacked, compact = True, False
        else:
            stacked, compact = False, True

        self.query_one("#banner", Static).update(
            banner_text(colors, width, stacked=stacked, compact=compact)
        )
        self.query_one("#strip", Static).update(palette_strip(colors))

        self.query_one("#status", Static).update(f"active theme  [b]{_active_name()}[/]")
        self.query_one("#menu", Vertical).border_title = "menu"

    def on_button_pressed(self, event: Button.Pressed) -> None:
        button_id = event.button.id
        if button_id == "presets":
            self.app.push_screen("presets")
        elif button_id == "terminal":
            self.app.push_screen("terminal")
        elif button_id == "app-theme":
            self.app.push_screen("app-theme")
        elif button_id == "quit":
            self.app.exit()
