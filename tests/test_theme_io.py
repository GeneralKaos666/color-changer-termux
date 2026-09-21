from termux_colors.models.app_theme import CURATED_BY_KEY, validate_value
from termux_colors.persistence.properties import parse_active, set_value, unset_value
from termux_colors.persistence.theme_io import read_app_theme, update_theme_text, write_app_theme

SAMPLE = """\
extra-keys = [['ESC',{key: '/', popup: '~' },'-','BKSP','UP','BACKSLASH',{key: KEYBOARD, popup: {macro: "CTRL d", display: exit}}],\\ ['TAB',':','DRAWER','LEFT','DOWN','RIGHT','ENTER']]

allow-external-apps = true

# fullscreen = true

# use-fullscreen-workaround = true

terminal-transcript-rows = 50000

use-black-ui = true
background=background.jpg
background.opacity=1.0
"""


def test_parse_active_reads_only_active_keys():
    pairs = parse_active(SAMPLE)
    assert pairs["allow-external-apps"] == "true"
    assert pairs["terminal-transcript-rows"] == "50000"
    assert "fullscreen" not in pairs


def test_set_value_uncomments_existing_commented_key():
    updated = set_value(SAMPLE, "fullscreen", "true")
    assert "# fullscreen = true" not in updated
    assert "fullscreen = true" in updated


def test_set_value_preserves_extra_keys_and_background():
    updated = update_theme_text(SAMPLE, {"use-black-ui": "false"})
    assert "extra-keys = [" in updated
    assert "\\ ['TAB'" in updated
    assert "background=background.jpg" in updated
    assert "background.opacity=1.0" in updated
    assert "use-black-ui = false" in updated


def test_set_value_does_not_touch_background_when_editing_dotted_key():
    updated = set_value(SAMPLE, "background.opacity", "0.5")
    assert "background=background.jpg" in updated
    assert "background.opacity=0.5" in updated


def test_unset_comments_out_line():
    updated = unset_value(SAMPLE, "terminal-transcript-rows")
    assert "\nterminal-transcript-rows = 50000" not in updated
    assert "# terminal-transcript-rows = 50000" in updated


def test_read_app_theme_returns_curated_values(tmp_path):
    path = tmp_path / "termux.properties"
    path.write_text(SAMPLE, encoding="utf-8")
    theme = read_app_theme(path)
    assert theme.get("allow-external-apps") == "true"
    assert theme.get("use-black-ui") == "true"
    assert theme.get("fullscreen") == ""


def test_write_app_theme_preserves_unmanaged_lines(tmp_path):
    path = tmp_path / "termux.properties"
    path.write_text(SAMPLE, encoding="utf-8")
    write_app_theme(path, {"terminal-cursor-style": "bar"})
    text = path.read_text(encoding="utf-8")
    assert "terminal-cursor-style=bar" in text
    assert "extra-keys = [" in text
    assert "background=background.jpg" in text


def test_validate_value_rules():
    assert validate_value(CURATED_BY_KEY["use-black-ui"], "TRUE") == "true"
    assert validate_value(CURATED_BY_KEY["terminal-transcript-rows"], "500") == "500"
    assert validate_value(CURATED_BY_KEY["terminal-cursor-style"], "bar") == "bar"

    try:
        validate_value(CURATED_BY_KEY["use-black-ui"], "maybe")
    except ValueError:
        pass
    else:
        raise AssertionError("expected ValueError for invalid bool")

    try:
        validate_value(CURATED_BY_KEY["terminal-cursor-style"], "fancy")
    except ValueError:
        pass
    else:
        raise AssertionError("expected ValueError for invalid choice")

    try:
        validate_value(CURATED_BY_KEY["terminal-transcript-rows"], "abc")
    except ValueError:
        pass
    else:
        raise AssertionError("expected ValueError for invalid int")
