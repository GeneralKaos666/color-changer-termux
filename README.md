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

## Requirements

- Termux with a truecolor (24-bit colour) terminal. Termux always has one; a
  terminal that only supports 256 colours will render approximated colours.
- `termux-reload-settings` (from `termux-tools`) for live reloading.
- At least a 40×10 terminal; smaller windows show a notice instead of the UI.

## Install

### APT (recommended)

`termux-colors` is available from the [flutter-for-termux](https://github.com/GeneralKaos666/flutter-for-termux) apt repository.

```sh
pkg install -y gnupg
mkdir -p "$PREFIX/etc/apt/keyrings"
curl -fsSL https://GeneralKaos666.github.io/flutter-for-termux/key.gpg \
  | gpg --dearmor > "$PREFIX/etc/apt/keyrings/flutter.gpg"
echo "deb [signed-by=$PREFIX/etc/apt/keyrings/flutter.gpg] https://GeneralKaos666.github.io/flutter-for-termux termux main" \
  > "$PREFIX/etc/apt/sources.list.d/flutter.list"
pkg update && pkg install termux-colors
termux-colors
```

### Release binary

Download the asset that matches your device and install it as
`$PREFIX/bin/termux-colors`:

```sh
pkg install curl
curl -LO https://github.com/GeneralKaos666/color-changer-termux/releases/latest/download/termux-colors-aarch64-linux-android
install -m 755 termux-colors-aarch64-linux-android "$PREFIX/bin/termux-colors"
```

Use `termux-colors-armv7-linux-androideabi` instead on 32-bit devices
(`uname -m` prints `armv7l`). Each release also ships a `.sha256` file:

```sh
curl -LO https://github.com/GeneralKaos666/color-changer-termux/releases/latest/download/termux-colors-aarch64-linux-android.sha256
sha256sum -c termux-colors-aarch64-linux-android.sha256
```

### From source

```sh
pkg install rust git
git clone https://github.com/GeneralKaos666/color-changer-termux.git
cd color-changer-termux
cargo build --release
install -m 755 target/release/termux-colors "$PREFIX/bin/termux-colors"
```

### Upgrading from the Python version

Remove the old entry point first:

```sh
pip uninstall termux-colors
```

Otherwise the Python script in `$PREFIX/bin` keeps shadowing the new binary.
Your themes, `termux.properties` edits and backups are untouched by the
upgrade.

## Usage

```sh
termux-colors
```

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | next / previous focus region |
| `Up` / `Down`, `j` / `k` | move inside a list or form |
| `Home` / `End` | first / last item |
| `PageUp` / `PageDown` | jump ten items |
| `Enter` | activate the focused button or item |
| `Esc` | go back; quits from the main menu |
| `q` | quit from the main menu |
| `Ctrl+C` | quit from anywhere, including dialogs |
| `Ctrl+S` | save (editors) |
| `Ctrl+A` | save as a new theme (Terminal Colors) |
| `Ctrl+R` | revert unsaved edits (editors) |
| `r` | reload the theme library (Preset Themes) |

In Preset Themes, focus the filter box and type to search the library.

## Relationship to `colors.sh`

`termux-colors` reads the same library as `~/.termux/colors.sh`
(`~/.termux/colors/<group>/*.properties`) and activates themes the same way:
by symlinking `~/.termux/colors.properties` and calling
`termux-reload-settings`. The two can be used interchangeably. If the library
is empty, a small set of built-in themes is written to
`~/.termux/colors/builtin/`.

`termux.properties` is edited line-by-line, so complex settings such as
`extra-keys`, background images and comments are left untouched.

The on-disk formats, `colors.sh` interoperability and backup behaviour are the
same as the previous version.

## Development

```sh
pkg install rust
cargo test
```

Lint before committing:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Tests point the app at a temporary directory through the `TERMUX_HOME`
environment variable, so they never touch your real config.

## Backups

Backups are plain copies named `colors-YYYY-MM-DD-HHMMSS.properties` or
`termux-YYYY-MM-DD-HHMMSS.properties` under `~/.termux/backups/`.
