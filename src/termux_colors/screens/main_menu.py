"""The landing screen."""

from __future__ import annotations

from textual.app import ComposeResult
from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Static


class MainMenuScreen(Screen):
    """Top-level navigation between the app's tools."""

    BINDINGS = [("q", "app.quit", "Quit"), ("escape", "app.quit", "Quit")]

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with Vertical(id="menu"):
            yield Static("Termux Color Changer", id="title")
            yield Static(
                "Change your terminal colours and Termux app theme.",
                id="subtitle",
            )
            yield Button("Preset Themes", variant="primary", id="presets")
            yield Button("Terminal Colors", id="terminal")
            yield Button("App Theme", id="app-theme")
            yield Button("Quit", variant="error", id="quit")
        yield Footer()

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
