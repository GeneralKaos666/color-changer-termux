"""Modal dialogs shared by the screens."""

from __future__ import annotations

from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical
from textual.screen import ModalScreen
from textual.widgets import Button, Input, Static


class ConfirmScreen(ModalScreen[bool]):
    """A yes/no confirmation dialog."""

    BINDINGS = [("escape", "dismiss(False)", "Cancel")]

    def __init__(self, message: str, confirm_label: str = "Yes", cancel_label: str = "No") -> None:
        super().__init__()
        self.message = message
        self.confirm_label = confirm_label
        self.cancel_label = cancel_label

    def compose(self) -> ComposeResult:
        with Vertical(id="dialog"):
            yield Static(self.message, id="dialog-message")
            with Horizontal(id="dialog-buttons"):
                yield Button(self.confirm_label, variant="primary", id="confirm")
                yield Button(self.cancel_label, variant="default", id="cancel")

    def on_button_pressed(self, event: Button.Pressed) -> None:
        self.dismiss(event.button.id == "confirm")


class MessageScreen(ModalScreen[None]):
    """An informational dialog with a single dismiss button."""

    BINDINGS = [("escape", "dismiss", "Close")]

    def __init__(self, message: str, title: str = "Notice") -> None:
        super().__init__()
        self.message = message
        self.title = title

    def compose(self) -> ComposeResult:
        with Vertical(id="dialog"):
            yield Static(self.title, id="dialog-title")
            yield Static(self.message, id="dialog-message")
            with Horizontal(id="dialog-buttons"):
                yield Button("OK", variant="primary", id="ok")

    def on_button_pressed(self, event: Button.Pressed) -> None:
        self.dismiss(None)


class PromptScreen(ModalScreen[str | None]):
    """Ask the user for a single line of text."""

    BINDINGS = [("escape", "dismiss(None)", "Cancel")]

    def __init__(self, message: str, initial: str = "", placeholder: str = "") -> None:
        super().__init__()
        self.message = message
        self.initial = initial
        self.placeholder = placeholder

    def compose(self) -> ComposeResult:
        with Vertical(id="dialog"):
            yield Static(self.message, id="dialog-message")
            yield Input(value=self.initial, placeholder=self.placeholder, id="prompt-input")
            with Horizontal(id="dialog-buttons"):
                yield Button("OK", variant="primary", id="ok")
                yield Button("Cancel", variant="default", id="cancel")

    def on_mount(self) -> None:
        self.query_one("#prompt-input", Input).focus()

    def on_input_submitted(self, event: Input.Submitted) -> None:
        self.dismiss(event.value.strip() or None)

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "ok":
            value = self.query_one("#prompt-input", Input).value.strip()
            self.dismiss(value or None)
        else:
            self.dismiss(None)
