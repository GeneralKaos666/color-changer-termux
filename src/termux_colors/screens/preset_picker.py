"""Browse the theme library and activate a theme."""

from __future__ import annotations

from textual import work
from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Label, ListItem, ListView, Static

from ..library import Theme, active_theme_path, apply_theme, reload_termux, scan_themes
from ..paths import backups_dir, colors_properties_path, colors_root
from ..persistence.backup import make_backup
from ..persistence.colors_io import read_colors
from ..presets.built_in import materialize
from ..widgets import colors_preview_text
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
        self.themes: list[Theme] = load_themes()

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with Horizontal(id="picker"):
            with Vertical(id="picker-list"):
                yield Static("Themes", classes="panel-title")
                with ListView(id="theme-list"):
                    for theme in self.themes:
                        yield ListItem(Label(theme.label))
            with VerticalScroll(id="picker-preview"):
                yield Static("Preview", classes="panel-title")
                yield Static(id="preview")
                yield Static(id="details")
        with Horizontal(id="actions"):
            yield Button("Apply", variant="primary", id="apply")
            yield Button("Refresh", id="refresh")
            yield Button("Back", id="back")
        yield Footer()

    def on_mount(self) -> None:
        if self.themes:
            self.query_one("#theme-list", ListView).index = 0
            self._show_theme(self.themes[0])
        else:
            self.query_one("#details", Static).update("No themes found.")

    def selected_theme(self) -> Theme | None:
        index = self.query_one("#theme-list", ListView).index
        if index is None or not (0 <= index < len(self.themes)):
            return None
        return self.themes[index]

    def _show_theme(self, theme: Theme) -> None:
        try:
            colors = read_colors(theme.path)
        except OSError as error:
            self.query_one("#preview", Static).update("")
            self.query_one("#details", Static).update(f"Could not read: {error}")
            return
        self.query_one("#preview", Static).update(colors_preview_text(colors))
        active = active_theme_path(colors_properties_path())
        is_active = active is not None and active == theme.path.resolve()
        status = "  (active)" if is_active else ""
        self.query_one("#details", Static).update(f"{theme.label}{status}\n{theme.path}")

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
        self.themes = load_themes()
        list_view = self.query_one("#theme-list", ListView)
        await list_view.clear()
        for theme in self.themes:
            list_view.append(ListItem(Label(theme.label)))
        if self.themes:
            list_view.index = 0
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
        message = f"Applied {theme.label}."
        if not reloaded:
            message += " Run a new session or termux-reload-settings to see it."
        self.notify(message, severity="information")
        self._show_theme(theme)
