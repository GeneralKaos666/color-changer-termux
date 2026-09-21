from pathlib import Path

from termux_colors.persistence.backup import list_backups, make_backup


def test_make_backup_copies_content(tmp_path):
    source = tmp_path / "colors.properties"
    source.write_text("background=#000000\n", encoding="utf-8")
    backups = tmp_path / "backups"

    result = make_backup(source, backups, prefix="colors")

    assert result is not None
    assert result.parent == backups
    assert result.read_text(encoding="utf-8") == "background=#000000\n"
    assert result.name.startswith("colors-")
    assert result.suffix == ".properties"


def test_make_backup_follows_symlink(tmp_path):
    real = tmp_path / "theme.properties"
    real.write_text("foreground=#ffffff\n", encoding="utf-8")
    link = tmp_path / "colors.properties"
    link.symlink_to(real)

    result = make_backup(link, tmp_path / "backups", prefix="colors")

    assert result is not None
    assert result.read_text(encoding="utf-8") == "foreground=#ffffff\n"


def test_make_backup_returns_none_when_missing(tmp_path):
    assert make_backup(tmp_path / "nope.properties", tmp_path / "backups") is None


def test_make_backup_does_not_collide(tmp_path):
    source = tmp_path / "colors.properties"
    source.write_text("background=#000000\n", encoding="utf-8")
    backups = tmp_path / "backups"

    first = make_backup(source, backups, prefix="colors")
    second = make_backup(source, backups, prefix="colors")

    assert first is not None and second is not None
    assert first != second
    assert len(list_backups(backups)) == 2


def test_list_backups_empty_when_missing(tmp_path):
    assert list_backups(tmp_path / "backups") == []
