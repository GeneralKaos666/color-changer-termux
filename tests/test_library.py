from termux_colors.library import (
    active_theme_path,
    apply_theme,
    group_themes,
    reload_termux,
    scan_themes,
)

THEME_BODY = "background=#000000\nforeground=#ffffff\ncursor=#ffffff\n"


def _make_library(tmp_path):
    root = tmp_path / "colors"
    (root / "dark").mkdir(parents=True)
    (root / "light").mkdir(parents=True)
    (root / "dark" / "Dracula.properties").write_text(THEME_BODY, encoding="utf-8")
    (root / "dark" / "Nord.properties").write_text(THEME_BODY, encoding="utf-8")
    (root / "light" / "Solarized.properties").write_text(THEME_BODY, encoding="utf-8")
    (root / "cyberpunk.properties").write_text(THEME_BODY, encoding="utf-8")
    (root / "colors.properties").write_text(THEME_BODY, encoding="utf-8")
    (root / "Dracula.properties.backup.20260101").write_text(THEME_BODY, encoding="utf-8")
    (root / "old.bak").write_text(THEME_BODY, encoding="utf-8")
    return root


def test_scan_finds_themes_and_skips_backups(tmp_path):
    root = _make_library(tmp_path)
    themes = scan_themes(root)
    labels = {theme.label for theme in themes}

    assert "dark/Dracula" in labels
    assert "dark/Nord" in labels
    assert "light/Solarized" in labels
    assert "cyberpunk" in labels
    assert all("backup" not in theme.name.lower() for theme in themes)
    assert all(theme.name != "colors" for theme in themes)


def test_group_themes(tmp_path):
    root = _make_library(tmp_path)
    grouped = group_themes(scan_themes(root))
    assert set(grouped) == {"dark", "light", "root"}
    assert len(grouped["dark"]) == 2


def test_scan_missing_root_returns_empty(tmp_path):
    assert scan_themes(tmp_path / "nope") == []


def test_apply_theme_creates_symlink(tmp_path):
    root = _make_library(tmp_path)
    colors_properties = tmp_path / "colors.properties"
    target = root / "dark" / "Dracula.properties"

    apply_theme(target, colors_properties)

    assert colors_properties.is_symlink()
    assert active_theme_path(colors_properties) == target.resolve()


def test_apply_theme_replaces_existing_symlink(tmp_path):
    root = _make_library(tmp_path)
    colors_properties = tmp_path / "colors.properties"
    apply_theme(root / "dark" / "Dracula.properties", colors_properties)
    apply_theme(root / "dark" / "Nord.properties", colors_properties)

    assert colors_properties.is_symlink()
    assert active_theme_path(colors_properties) == (root / "dark" / "Nord.properties").resolve()


def test_apply_theme_replaces_regular_file(tmp_path):
    root = _make_library(tmp_path)
    colors_properties = tmp_path / "colors.properties"
    colors_properties.write_text("background=#111111\n", encoding="utf-8")

    apply_theme(root / "dark" / "Dracula.properties", colors_properties)

    assert colors_properties.is_symlink()


def test_apply_theme_is_idempotent(tmp_path):
    root = _make_library(tmp_path)
    colors_properties = tmp_path / "colors.properties"
    target = root / "dark" / "Dracula.properties"

    apply_theme(target, colors_properties)
    apply_theme(target, colors_properties)

    assert colors_properties.is_symlink()
    assert active_theme_path(colors_properties) == target.resolve()


def test_apply_theme_keeps_plain_file_that_is_its_own_target(tmp_path):
    colors_properties = tmp_path / "colors.properties"
    colors_properties.write_text(THEME_BODY, encoding="utf-8")

    apply_theme(colors_properties, colors_properties)

    assert not colors_properties.is_symlink()
    assert colors_properties.is_file()


def test_reload_termux_without_binary(monkeypatch):
    monkeypatch.setattr("termux_colors.library.shutil.which", lambda name: None)
    assert reload_termux() is False
