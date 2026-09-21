"""Textual application entry point."""

from __future__ import annotations

from textual.app import App

from .library import active_colors
from .screens.app_theme_editor import AppThemeEditorScreen
from .screens.main_menu import MainMenuScreen
from .screens.preset_picker import PresetPickerScreen
from .screens.terminal_editor import TerminalEditorScreen
from .theme import apply_app_theme

CSS = """
Screen {
    background: $background;
    color: $foreground;
}

Header {
    background: $panel;
    color: $foreground;
}

Footer {
    background: $panel;
}

#menu-container {
    align: center top;
    padding: 1 2;
}

#banner {
    text-align: center;
    width: 100%;
}

#tagline {
    text-align: center;
    color: $text-muted;
    width: 100%;
    padding-bottom: 1;
}

#strip {
    text-align: center;
    width: 100%;
}

#status {
    text-align: center;
    color: $text-muted;
    width: 100%;
    padding: 1 0;
}

#menu {
    width: 62;
    max-width: 100%;
    height: auto;
    border: round $primary;
    border-title-color: $primary;
    border-title-align: center;
    padding: 1 2;
    background: $surface;
}

#menu Button {
    width: 100%;
    margin-bottom: 1;
    text-style: bold;
}

.panel-title {
    text-style: bold;
    background: $boost;
    color: $foreground;
    width: 100%;
    padding: 0 1;
}

.hint {
    color: $text-muted;
}

#picker {
    height: 1fr;
}

#picker-list {
    width: 42%;
    min-width: 26;
    border: round $primary;
    border-title-color: $primary;
    padding: 0 1;
    background: $surface;
}

#theme-filter {
    margin: 1 0;
}

#picker-preview {
    width: 58%;
    border: round $secondary;
    border-title-color: $secondary;
    padding: 1 2;
    background: $surface;
}

#details {
    color: $text-muted;
    padding-top: 1;
}

#editor {
    height: 1fr;
}

#editor-form {
    width: 52%;
    min-width: 32;
    border: round $primary;
    border-title-color: $primary;
    padding: 0 1 1 1;
    background: $surface;
}

#editor-preview {
    width: 48%;
    border: round $secondary;
    border-title-color: $secondary;
    padding: 1 2;
    background: $surface;
}

.field-label {
    color: $text-muted;
    text-style: bold;
    padding-top: 1;
}

.field-row {
    height: auto;
}

.field-row Input {
    width: 1fr;
}

.field-swatch {
    width: 4;
    height: 1;
    margin: 0 1 0 0;
}

.input-error {
    border: tall $error;
}

.section-rule {
    color: $accent;
    padding-top: 1;
}

#actions {
    height: auto;
    padding: 1 0;
    align-horizontal: center;
}

#actions Button {
    margin: 0 1;
    min-width: 12;
}

ConfirmScreen, MessageScreen, PromptScreen {
    align: center middle;
    background: $background 70%;
}

#dialog {
    width: 64;
    max-width: 90%;
    height: auto;
    padding: 1 2;
    background: $panel;
    border: round $primary;
    border-title-color: $primary;
}

#dialog-title {
    text-style: bold;
    color: $primary;
    padding-bottom: 1;
}

#dialog-message {
    padding-bottom: 1;
}

#dialog-buttons {
    height: auto;
    align-horizontal: right;
    padding-top: 1;
}

#dialog-buttons Button {
    margin-left: 1;
}

Toast {
    background: $panel;
    border: round $primary;
    color: $foreground;
}
"""


class TermuxColorsApp(App):
    """Manage Termux terminal colours and app theme."""

    CSS = CSS
    TITLE = "Termux Colors"
    SCREENS = {
        "main": MainMenuScreen,
        "presets": PresetPickerScreen,
        "terminal": TerminalEditorScreen,
        "app-theme": AppThemeEditorScreen,
    }

    def on_mount(self) -> None:
        apply_app_theme(self, active_colors())
        self.push_screen("main")


def main() -> None:
    """Console-script entry point."""
    TermuxColorsApp().run()


if __name__ == "__main__":
    main()
