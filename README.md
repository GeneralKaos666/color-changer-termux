# termux-colors

A keyboard-driven TUI for [Termux](https://termux.dev) that changes your
terminal colour scheme and your Termux app theme.

```
┌─────────────────────────────┐
│  Termux Color Changer       │
│  [ Preset Themes ]          │
│  [ Terminal Colors ]        │
│  [ App Theme ]              │
│  [ Quit ]                   │
└─────────────────────────────┘
```

## Features

- **Themes itself from your palette** — the TUI re-colours itself to match the
  active terminal theme, and updates live as you highlight presets.
- **Preset Themes** — browses the themes in `~/.termux/colors/` (the same
  library used by `colors.sh`), with a filter box, a mock shell session
  rendered in each theme, a swatch grid, and a colour strip. Activating a theme
  points `~/.termux/colors.properties` at it and runs `termux-reload-settings`.
- **Terminal Colors** — edits background, foreground, cursor and the 16 ANSI
  colours with inline swatches, live validation and a live preview. Save in
  place or `Save As...` a new theme.
- **App Theme** — edits a safe subset of `~/.termux/termux.properties`
  (`use-black-ui`, cursor style, margins, transcript rows, etc.) in sectioned
  groups, without touching anything it does not manage.
- **Safe by default** — before overwriting an existing config it asks whether
  to write a timestamped backup to `~/.termux/backups/`.

## Install

Requires Python 3.10+ (Termux: `pkg install python`).

```sh
pip install .
```

This installs a `termux-colors` command on your `PATH`.

## Usage

```sh
termux-colors
```

Navigation: `Tab`/`Shift+Tab` moves focus, `Enter` activates a button,
`Esc` goes back, `q` quits from the main menu. In Preset Themes, focus the
filter box and type to search the library.

## Relationship to `colors.sh`

`termux-colors` reads the same library as `~/.termux/colors.sh`
(`~/.termux/colors/<group>/*.properties`) and activates themes the same way:
by symlinking `~/.termux/colors.properties` and calling
`termux-reload-settings`. The two can be used interchangeably. If the library
is empty, a small set of built-in themes is written to
`~/.termux/colors/builtin/`.

`termux.properties` is edited line-by-line, so complex settings such as
`extra-keys`, background images and comments are left untouched.

## Development

```sh
python -m venv .venv
.venv/bin/pip install -e '.[dev]'
.venv/bin/python -m pytest
```

Tests point the app at a temporary directory through the `TERMUX_HOME`
environment variable, so they never touch your real config.

## Backups

Backups are plain copies named `colors-YYYY-MM-DD-HHMMSS.properties` or
`termux-YYYY-MM-DD-HHMMSS.properties` under `~/.termux/backups/`.
