import asyncio

from termux_colors.app import TermuxColorsApp
from termux_colors.screens.app_theme_editor import AppThemeEditorScreen
from termux_colors.screens.main_menu import MainMenuScreen
from termux_colors.screens.preset_picker import PresetPickerScreen
from termux_colors.screens.terminal_editor import TerminalEditorScreen

THEME = "background=#000000\nforeground=#ffffff\ncursor=#ffffff\n"


def _seed(home):
    (home / "colors" / "dark").mkdir(parents=True)
    (home / "colors" / "dark" / "Dracula.properties").write_text(THEME, encoding="utf-8")
    (home / "colors" / "dark" / "Nord.properties").write_text(THEME, encoding="utf-8")
    (home / "termux.properties").write_text("use-black-ui = true\n", encoding="utf-8")


def test_import_does_not_require_termux(tmp_path, monkeypatch):
    monkeypatch.setenv("TERMUX_HOME", str(tmp_path))
    app = TermuxColorsApp()
    assert app is not None


def test_navigation_and_composition(tmp_path, monkeypatch):
    monkeypatch.setenv("TERMUX_HOME", str(tmp_path))
    _seed(tmp_path)

    async def scenario():
        app = TermuxColorsApp()
        async with app.run_test() as pilot:
            await pilot.pause()
            assert isinstance(app.screen, MainMenuScreen)

            # Activate the first menu button (Preset Themes).
            await pilot.press("enter")
            await pilot.pause()
            assert isinstance(app.screen, PresetPickerScreen)
            assert len(app.screen.themes) == 2

            app.push_screen("terminal")
            await pilot.pause()
            assert isinstance(app.screen, TerminalEditorScreen)

            app.push_screen("app-theme")
            await pilot.pause()
            assert isinstance(app.screen, AppThemeEditorScreen)
            assert app.screen._collect()["use-black-ui"] == "true"

    asyncio.run(scenario())


def test_preset_picker_seeds_builtins_when_empty(tmp_path, monkeypatch):
    monkeypatch.setenv("TERMUX_HOME", str(tmp_path))

    async def scenario():
        app = TermuxColorsApp()
        async with app.run_test() as pilot:
            await pilot.pause()
            app.push_screen("presets")
            await pilot.pause()
            assert isinstance(app.screen, PresetPickerScreen)
            assert len(app.screen.themes) >= 1
            assert any("dracula" in theme.name.lower() for theme in app.screen.themes)

    asyncio.run(scenario())


def test_preset_filter_narrows_the_list(tmp_path, monkeypatch):
    from textual.widgets import Input

    monkeypatch.setenv("TERMUX_HOME", str(tmp_path))
    _seed(tmp_path)

    async def scenario():
        app = TermuxColorsApp()
        async with app.run_test() as pilot:
            await pilot.pause()
            app.push_screen("presets")
            await pilot.pause()
            screen = app.screen
            assert len(screen.themes) == 2

            screen.query_one("#theme-filter", Input).value = "nord"
            await pilot.pause()
            assert len(screen.themes) == 1
            assert "nord" in screen.themes[0].name.lower()

            screen.query_one("#theme-filter", Input).value = "zzz"
            await pilot.pause()
            assert screen.themes == []

    asyncio.run(scenario())
