"""Textual application entry point."""

from __future__ import annotations

from textual.app import App

from .screens.app_theme_editor import AppThemeEditorScreen
from .screens.main_menu import MainMenuScreen
from .screens.preset_picker import PresetPickerScreen
from .screens.terminal_editor import TerminalEditorScreen

CSS = """
Screen {
    background: $surface;
}

#menu {
    width: 64;
    max-width: 90%;
    height: auto;
    margin: 1 2;
}

#title {
    text-style: bold;
    padding: 1 0 0 0;
}

#subtitle {
    color: $text-muted;
    padding-bottom: 1;
}

#menu Button {
    width: 100%;
    margin-bottom: 1;
}

.panel-title {
    text-style: bold;
    background: $boost;
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
    border: round $primary;
    padding: 0 1;
}

#picker-preview {
    width: 58%;
    border: round $secondary;
    padding: 1;
}

#editor {
    height: 1fr;
}

#editor-form {
    width: 55%;
    border: round $primary;
    padding: 0 1;
}

#editor-form Label {
    padding-top: 1;
    color: $text-muted;
}

#editor-preview {
    width: 45%;
    border: round $secondary;
    padding: 1;
}

#actions {
    height: auto;
    padding: 1 0;
}

#actions Button {
    margin: 0 1;
}

ConfirmScreen, MessageScreen, PromptScreen {
    align: center middle;
    background: $background 60%;
}

#dialog {
    width: 64;
    max-width: 90%;
    height: auto;
    padding: 1 2;
    background: $panel;
    border: thick $primary;
}

#dialog-title {
    text-style: bold;
    padding-bottom: 1;
}

#dialog-message {
    padding-bottom: 1;
}

#dialog-buttons {
    height: auto;
    align-horizontal: right;
}

#dialog-buttons Button {
    margin-left: 1;
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
        self.push_screen("main")


def main() -> None:
    """Console-script entry point."""
    TermuxColorsApp().run()


if __name__ == "__main__":
    main()
