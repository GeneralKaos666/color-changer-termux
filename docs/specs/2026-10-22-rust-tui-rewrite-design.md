# Rust TUI rewrite of termux-colors

- **Date:** 2026-10-22
- **Status:** approved design, awaiting spec review
- **Supersedes:** the Textual implementation described in `README.md`

## 1. Context

`termux-colors` is a keyboard-driven TUI that manages the Termux colour scheme
(`~/.termux/colors.properties`) and a curated subset of the Termux app settings
(`~/.termux/termux.properties`). The current implementation is ~2,100 lines of
Python built on Textual and Rich, installed with `pip install .`.

The application works. The problems motivating a rewrite are distribution and
runtime weight: users need Python 3.10+ and a working `textual` install before
the tool can change a hex value, and the command only exists while that
environment does.

This spec describes replacing the Python implementation with a Rust binary
using ratatui, preserving behaviour, file formats and keyboard conventions.

## 2. Goal

Ship a single self-contained binary named `termux-colors` that performs every
operation the Python version performs, reads and writes the same files with the
same preservation guarantees, and needs no interpreter at runtime.

### Success criteria

1. For each fixture in `tests/fixtures/`, writing unchanged data produces the
   documented output (see §6.3). Comments, unmanaged keys, line order, indentation
   and line endings survive.
2. All four screens are reachable and every documented action works: preset
   listing, filtering and activation; the 19-field terminal colour editor with
   Save, Save As and Revert; the 9-field app-theme editor with Save and Revert.
3. The preset library layout keeps working interchangeably with `colors.sh`:
   activation is a symlink from `~/.termux/colors.properties` into
   `~/.termux/colors/`, followed by `termux-reload-settings`.
4. `cargo test` and `cargo clippy -- -D warnings` pass.
5. The release binary runs on Termux aarch64 with no Python installed.
6. The Python source tree, pytest suite, `pyproject.toml` and build artifacts are
   removed from the repository.

## 3. Non-goals

- New features the Python version does not have. Parity only, plus the small
  additions listed in §7.9.
- Mouse support.
- Persisting UI preferences (window layout, last selected theme).
- Supporting terminals without 24-bit colour. Termux always has it; other
  terminals get a documented caveat rather than a quantisation layer.
- Windows or macOS. Unix-only assumptions (symlinks, `PATH` lookup) are allowed.
- A Termux package-manager recipe (`pkg install`). Release binaries plus
  on-device `cargo build` are the two supported install paths.

## 4. Constraints

- **Platform:** aarch64 Android/bionic under Termux. Pure-Rust dependencies only;
  no C or system libraries beyond libc.
- **Dependencies:** `ratatui`, `crossterm`, `tui-input`, `time` (with
  `local-offset`), and `tempfile` for dev/test. No `regex`, `serde`, `anyhow`,
  `thiserror`, `walkdir`, `chrono`, or `clap`. Adding one requires amending this
  spec.
- **Toolchain:** stable Rust, edition 2024, `rust-version = "1.85"`.
- **Compatibility:** `~/.termux/*` file formats and the `TERMUX_HOME` override
  behave as they do today.
- **Keyboard only.** No interaction may require a pointer.

## 5. Architecture

A single application value owns all mutable state. There are no trait objects and
no retained widget tree: ratatui is immediate-mode, and the UI is drawn from
plain data each frame.

```rust
pub struct App {
    paths: Paths,
    palette: Palette,           // derived from the colours currently previewed
    stack: Vec<Screen>,         // last = visible
    dialog: Option<Dialog>,     // drawn over the top screen
    status: Option<(String, Instant)>,
    should_quit: bool,
}

pub enum Screen {
    Main(MainMenu),
    Presets(PresetPicker),
    Terminal(TerminalEditor),
    AppTheme(AppThemeEditor),
}
```

Each screen is a struct with its own focus/selection/scroll state and a `pending`
field (§7.6). `App` owns the event loop, the screen stack, the dialog slot and
status expiry. Screens never read the environment, the clock, or the filesystem
directly at render time; they act through `Paths` and the persistence modules on
key events.

Rationale for this shape is recorded in §13.

## 6. Module contracts

### 6.1 `paths`

```rust
pub struct Paths { root: PathBuf }

impl Paths {
    pub fn from_env() -> Self;              // $TERMUX_HOME, else $HOME/.termux
    pub fn root(&self) -> &Path;
    pub fn colors_root(&self) -> PathBuf;   // root/colors
    pub fn colors_properties(&self) -> PathBuf;
    pub fn termux_properties(&self) -> PathBuf;
    pub fn backups(&self) -> PathBuf;       // root/backups
}
```

`from_env` is the only place `TERMUX_HOME` or `HOME` is read. Everything else
receives `&Paths`, which makes integration tests parallel-safe without mutating
process-global state.

### 6.2 `model::colors`

```rust
pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }
pub struct TerminalColors {
    pub background: Rgb,
    pub foreground: Rgb,
    pub cursor: Rgb,
    pub palette: [Rgb; 16],
    pub extra: Vec<(String, String)>,   // order preserved, as in the file
}
```

- `Rgb::parse` accepts `#rgb` and `#rrggbb` (case-insensitive) and rejects
  anything else. `Display` emits lowercase `#rrggbb`.
- `DEFAULT_PALETTE` and `ANSI_NAMES` are ported verbatim from
  `models/colors.py`.
- `extra` is ordered. It exists so unmanaged keys survive a write; a `HashMap`
  would reorder a user's file.

### 6.3 `persist::properties` and the preservation contract

`parse_active(text) -> Vec<(String, String)>` returns active `key=value` pairs.
`set_value(text, key, value) -> String` and `unset_value(text, key) -> String`
edit one key without disturbing anything else.

Preservation guarantees, asserted by fixtures:

| Property | Required behaviour |
|---|---|
| Line endings | CRLF stays CRLF; LF stays LF |
| Trailing newline | Present if present in input; added for empty input |
| Existing key | Keeps its indentation and its `=` surrounding whitespace |
| Commented key | `# key = value` is replaced in place, not duplicated |
| Unknown keys | Untouched, same position |
| Unset | Key is commented out as `# key = value`, never deleted |
| Non-key lines | Array fragments, continuation lines and any line that is not `key=value` are never rewritten |

The last row is the honest limit of line-by-line editing: a key that exists
*inside* a multi-line `extra-keys` array and happens to look like `key=value`
would be matched. Matching the Python behaviour is preferred to inventing a
different rule here; the fixture tests pin the real one-line form Termux writes.

**Canonicalisation (intentional change).** Managed keys — `background`,
`foreground`, `cursor`, `color0`..`color15` — are re-rendered as lowercase
six-digit hex when written. A file containing `background=#FFF` will save as
`background=#ffffff`. The colour is identical; the spelling is normalised. Keys
the app does not manage are never reformatted.

Parsing is hand-written: scan each line for optional indent, an optional `#`
comment marker, a key of `[A-Za-z0-9_.-]+`, optional whitespace, `=`, optional
whitespace, and a value. This replaces two Python regexes and keeps `regex` out
of the dependency tree.

### 6.4 `persist::colors_io`

- `parse_colors(text) -> TerminalColors`: missing keys fall back to defaults;
  `cursor` falls back to `foreground`, then `#ffffff`; every active key outside
  the managed set is collected into `extra`.
- `render_colors(&TerminalColors) -> String`: `# Generated by termux-colors`
  header, then `background`, `foreground`, `cursor`, a blank line, `color0`..
  `color15`, then extras, each `key=value`, trailing newline.
- `update_colors_text(text, &colors) -> String`: blank input renders fresh;
  otherwise set each managed key in the same order, then extras.
- `read_colors(&Path)`: absent file yields defaults; I/O errors propagate.
- `write_colors(&Path, &colors, preserve: bool)`: creates parent directories,
  writes through symlinks (so editing the active theme edits the library file).

### 6.5 `persist::theme_io` and `model::app_theme`

`FieldSpec { key, label, kind, choices, min, max, help, section }` with
`kind ∈ {Text, Bool, Choice, Int}` and the same nine curated fields, labels,
sections and bounds as `models/app_theme.py`.

`validate_value(spec, raw) -> Result<String, String>` returns the canonical value
or the user-facing message. Messages are ported verbatim so the UI reads the
same:

- `"{label} must be true or false"`
- `"{label} must be one of: {choices}"`
- `"{label} must be a whole number"`
- `"{label} must be >= {min}"` / `"{label} must be <= {max}"`

An empty input is always valid and means "unset" (the key is commented out).
`read_app_theme` keeps only curated keys; `write_app_theme` preserves unmanaged
lines and, when the file is absent, starts from
`# Termux properties managed by termux-colors`.

### 6.6 `persist::backup`

`make_backup(path, backups_dir, prefix) -> Option<PathBuf>`: `None` when the
source is absent; follows symlinks; creates the directory; names the file
`{prefix}-{timestamp}.properties` where `timestamp` is local time formatted
`%Y-%m-%d-%H%M%S` and is captured once per call; appends `-1`, `-2`, … on
collision; copies bytes.
`list_backups` returns files sorted newest-first.

If the local UTC offset cannot be determined (Android builds without timezone
data), fall back to UTC and keep the same format. This is surfaced once in the
status line rather than silently mislabelling, and is covered by a test.

### 6.7 `library`

```rust
pub struct ThemeRef { pub name: String, pub path: PathBuf, pub group: String }
```

- `scan_themes(root) -> Vec<ThemeRef>`: recursive `read_dir`, `*.properties`
  only; skips any file named `colors.properties`, files whose name contains `backup`
  (case-insensitive), ends with `.bak`, or contains `.bak-`, and directories
  whose first component starts with `.`. Sort: root-group themes last, then
  group (case-insensitive), then name (case-insensitive).
- `active_theme_path(colors_properties) -> Option<PathBuf>`: resolves the link to
  its real target, `None` if absent.
- `active_colors(&Paths) -> TerminalColors`: the active file when readable,
  otherwise defaults.
- `apply_theme(theme_path, colors_properties)`: error if the target is not a
  file; no-op if the properties path already resolves to it (prevents a
  self-symlink when the user edits `colors.properties` in place); otherwise
  create the parent, unlink the existing file or link, and symlink.
- `reload_termux() -> bool`: locate `termux-reload-settings` on `PATH` and run it,
  polling `try_wait` every 25 ms with a 15 s deadline, killing the child on
  timeout. Returns whether it ran successfully. Blocks the UI, as today.

### 6.8 `presets`

The built-in theme table — `dracula`, `solarized-dark`, `solarized-light`,
`nord`, `monokai`, `gruvbox-dark` — is ported verbatim from
`presets/built_in.py`.
`materialize(dir)` writes a theme only when the file is absent.

### 6.9 `theme`

Ports `_parse`, `to_hex`, `_channel_luminance`, `relative_luminance`, `is_dark`
(threshold `0.35`), `contrast_ratio`, `blend`, and `ensure_contrast` unchanged,
including the 20-step nudge toward white or black.

```rust
pub struct Palette {
    pub background: Color, pub foreground: Color,
    pub primary: Color, pub secondary: Color, pub accent: Color,
    pub error: Color, pub success: Color, pub warning: Color,
    pub surface: Color, pub panel: Color, pub boost: Color,
    pub muted: Color, pub dark: bool,
}
```

Construction from `TerminalColors` mirrors `theme_from_colors`: foreground is
contrasted to 4.5:1; `primary` from index 6 at 3:1; `secondary` from 5 at 2.5:1;
`accent` from 4 at 2.5:1; error/success/warning from 1/2/3 at 2.5:1; `surface`,
`panel` and `boost` are 5%, 10% and 17% blends toward white (dark) or black
(light). `muted` — which has no Textual source — is
`blend(foreground, background, 0.35)`, replacing `$text-muted`.

## 7. UI specification

### 7.1 Runtime lifecycle

`main` enables raw mode, enters the alternate screen and hides the cursor. A
guard restores the terminal on drop, and a panic hook restores it before the
default hook prints. `panic = "abort"` still runs the hook. No arguments, no
subcommands.

### 7.2 Event loop

Block on `event::poll(250 ms)`. Redraw on key, resize, dialog/status change, or
status expiry. Read keys as `KeyEvent`; ignore key-release events and plain
modifier events. The 250 ms deadline exists only so the status line can expire;
there are no animations.

### 7.3 Focus

Every screen exposes an ordered list of focusable regions. `Tab` and `BackTab`
move between regions; `Up`/`Down`/`j`/`k` move inside a list or form; `Home`,
`End`, `PageUp`, `PageDown` apply where a list exists; `Enter` activates; `Esc`
pops the screen and quits from the main menu, which has nothing to pop; `Ctrl+C`
quits from anywhere including inside a dialog.

Editable text is delegated to `tui-input`, which consumes character, `Backspace`,
`Delete`, `Left`, `Right`, `Home`, `End`, and word-movement keys. `Tab`,
`BackTab`, `Esc` and `Enter` are handled by the screen before delegation, so a
field can never trap the keyboard.

### 7.4 Screen: main menu

Banner (5-row block wordmark, "TERMUX" / "COLORS" coloured from the active
palette), tagline `change terminal & app colours, live`, palette strip, status
line `active theme  <name>`, and a bordered 4-item box: Preset Themes, Terminal
Colors, App Theme, Quit. `q` or `Esc` quits; `Up`/`Down`/`j`/`k` move; `Enter`
opens.

Responsive banner modes, same thresholds as today:

- width ≥ 88 and height ≥ 14: both words on one row
- width ≥ 40 and height ≥ 24: two rows, stacked
- otherwise: compact "TERMUX" only

### 7.5 Screen: preset picker

Two columns: filter field plus list (42% width, min 26) and preview (58%).
The header shows the active theme's library-relative name. Footer lists the keys.

- The list shows `group/name` for grouped themes and `name` for root themes, with
  a selection marker and a scrollbar when needed. Its border title shows
  `themes (N)`.
- Typing in the filter narrows the list case-insensitively by label; clearing it
  restores the full list and keeps the selection if still present.
- `Up`/`Down`/`j`/`k`, `PageUp`, `PageDown`, `Home`, `End` move the selection.
- `Enter` applies the selected theme; `r` rescans; `Esc` returns.
- The preview shows the theme name, the first five lines of the mock shell
  session, the 16-chip swatch grid, the palette strip, and details (path, group,
  `active` marker).
- Moving the selection re-themes the entire UI from that theme's palette. Parsed
  colours are cached per path so key repeat does not re-read files.
- An empty library is seeded from the built-ins into `colors/builtin/` and
  rescanned, exactly as today. Apply failures (missing file, failed reload) are
  reported in the status line and never leave a dangling symlink.

### 7.6 Screen: terminal colours

Form on the left (52%, min 32) with a section rule `basics` (background,
foreground, cursor) and `ansi palette` (16 rows labelled `colorN  (name)`), each
row a label, a 3-cell swatch, and a hex field; preview on the right (48%);
action row: Save, Save As…, Revert, Back.

- Inline validation: a non-empty invalid value paints the field border in
  `error` and leaves the swatch blank. A valid value updates the swatch.
- Any valid change updates the mock session, swatch grid, strip, and the live
  palette.
- The form scrolls so the focused field stays visible.
- Save, Save As and Revert perform the flows in §7.8.
- When no theme is active, Save behaves as Save As.

### 7.7 Screen: app theme

Single scrolling form: `Editing <path>`, then sections `appearance`, `terminal`,
`behaviour` with the curated fields. `Bool` and `Choice` render as a selector
(`Left`/`Right` or `Enter` cycles through values, plus a blank "unset" state);
`Int` and `Text` render as fields. Each change revalidates that field only.
Actions: Save, Revert, Back. Saving offers the backup prompt when
`termux.properties` exists, writes, reloads, and reports the result.

### 7.8 Dialog state machines

`Dialog` is one of `Confirm { title, message, confirm_label, cancel_label,
hint }`, `Message { title, message }`, or `Prompt { title, message, input,
placeholder }`. `DialogResult` is `Confirmed(bool)`, `Acknowledged`, or
`Text(Option<String>)`. The dialog is drawn as a centred bordered box (max 64
columns, 90% of the screen) with the title in the border, a hint line, and
right-aligned buttons; the first button has focus; `Esc` cancels.

Screens hold `pending: Option<...>`, set immediately before opening a dialog.
`App` delivers the result to the top screen's `on_dialog`, which advances the
flow and may open the next dialog.

Terminal editor:

1. **Save**, active theme exists and `colors.properties` exists →
   `Confirm("Back up the current config before saving?")` with
   `pending = WriteAfterBackup { target, colors }`. On confirm, back up; then
   write with `preserve = true`; if the target is not `colors.properties`, apply
   the symlink; reload; status `Saved <name>.` plus
   ` Run termux-reload-settings to refresh.` when the reload failed.
2. **Save** with no active theme → Save As.
3. **Save As** → `Prompt("Name for the new theme:")` with initial `custom` and
   placeholder `my-theme`, `pending = NameForSaveAs { colors }`. A blank or
   cancelled answer aborts. The name is sanitised by replacing runs of
   characters outside `[A-Za-z0-9._-]` with `-` and trimming `-`; an empty result
   becomes `custom`. The target is `colors/custom/<name>.properties`.
4. Target exists → `Confirm("<name> already exists. Overwrite?")` with
   `pending = Overwrite { target, colors }`; declining returns to the editor
   with unsaved changes intact.
5. Before steps 3 and 4 can write, if `colors.properties` exists →
   `Confirm("Back up the current config before saving?")` with
   `pending = BackupThenWriteAs { target, colors }`.
6. Any invalid managed value during Save or Save As →
   `Message("Invalid hex colour for: <keys>")`, `Invalid colour`.
7. **Revert** reloads from the active file (or defaults) and clears field errors.

App theme editor: **Save** with `termux.properties` present →
`Confirm("Back up termux.properties before saving?")` with
`pending = BackupThenWrite { values }`; an invalid value instead opens
`Message(<validation message>, "Invalid setting")`. **Revert** reloads from disk
and clears error state.

The preset picker opens no dialogs.

### 7.9 Additions beyond parity

- A "terminal too small" screen when the area is below 40×10.
- Status messages expire after 4 seconds; Python's `notify` toasts used their own
  timer.
- A scroll offset is tracked explicitly for long forms and lists (Textual
  supplied this silently).

### 7.10 Rendering

All colours come from `Palette`; no hardcoded hex values outside `theme` and
`presets`. Banners, swatches, the mock session and the strip are built as
`Line`/`Span` values with explicit foreground and background colours. The
mock session and gradient details match `widgets.py`: five preview lines in the
picker, seven in the editor, palette strip of 3-cell blocks, 16-chip grid with
index labels contrasted against each chip, and background/foreground/cursor
samples at the bottom.

## 8. Error handling

No `anyhow`/`thiserror`. A small `enum Error { Io(std::io::Error), Parse(String),
NotFound(PathBuf), Invalid(String) }` implements `Display` and `From<io::Error>`.

I/O failures during a user action become a `Message` dialog or a status line —
never a panic. The only panics permitted are invariant violations inside the
renderer, which the panic hook reports. Paths that fail to read as colours fall
back to defaults; individual theme files that fail to parse are skipped during a
scan with their name shown in the status line.

## 9. Testing

1. **Unit tests**, co-located per module:
   - properties: CRLF, trailing newline, indent/separator preservation,
     commented-key replacement, `unset_value`, continuation lines untouched,
     blank input
   - colours IO: defaults for partial files, `cursor` fallback, extras kept in
     order, header on fresh render, write-through-symlink
   - theme IO: curated-only read, unmanaged lines preserved, absent file header,
     empty value comments out
   - backup: naming, collision suffixes, symlink following, UTC fallback
   - library: scan order (root last), skip rules, `apply_theme` no-op and
     self-link guard, `active_theme_path`
   - theme: luminance `0`/`1`, contrast `1`/`21`, `ensure_contrast` moves apart,
     `blend` endpoints, `is_dark` threshold
   - app theme: every validation message
2. **Fixture tests** in `tests/`: the sample files from the Python suite
   (including `extra-keys` with a backslash continuation and a commented
   `fullscreen`) round-trip to the documented output.
3. **UI tests** with `ratatui::backend::TestBackend`: each screen renders at
   80×24 and 100×30 and contains its expected labels; `App::handle_key` drives
   menu → picker → apply, editor save with backup prompt, Save As with an
   existing name, and invalid hex, all against a `tempfile` `Paths`.

Tests never touch the real `~/.termux`. No network. `cargo test` runs on Linux
CI.

## 10. Build and release

`Cargo.toml`: package `termux-colors`, library `termux_colors`, binary
`termux-colors`; edition 2024; `rust-version = "1.85"`.
`[profile.release]`: `opt-level = "s"`, `lto = "thin"`, `codegen-units = 1`,
`strip = true`, `panic = "abort"`.

`.github/workflows/ci.yml`: `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test`.

`.github/workflows/release.yml`: on `v*` tags, `rustup target add
aarch64-linux-android armv7-linux-androideabi`, build with `cargo-ndk` at API
level 24, strip, emit `.sha256`, upload binaries to the release.

Install paths in the README: download the release binary to `$PREFIX/bin` and
`chmod +x`, or `pkg install rust && cargo build --release`. Upgrade note:
`pip uninstall termux-colors` first so the old entry point cannot shadow the
binary.

## 11. Migration and staging

Eight commits, each green:

1. Crate skeleton, `paths`, `model`, `persist` + tests
2. `library`, `presets`, `theme` + tests
3. App shell: terminal guard, event loop, `Palette`, main menu
4. Dialog primitives and pending routing
5. Preset picker
6. Terminal editor
7. App theme editor
8. Parity pass, README, CI workflows, remove `src/termux_colors/`, `tests/*.py`,
   `pyproject.toml`, `build/`, `dist/`, `.pytest_cache/` and the egg-info

Python is deleted only when the removal criteria in §2 are met. Git history
retains it.

## 12. Risks

| Risk | Mitigation |
|---|---|
| `time` cannot resolve a local offset on some Android setups | Fall back to UTC, same filename format, covered by a test |
| `tui-input` API churn across versions | Pin the minor version; the delegation surface is one struct and one `handle_event` call per screen |
| `termux-reload-settings` blocks or hangs | 15 s deadline with `kill`, matching today's timeout |
| Terminal left in raw mode after a crash | Panic hook plus drop guard, both restoring |
| Users with the pip install keep a stale entry point | Explicit `pip uninstall` note in the release |
| Widget-layer underestimation (focus, scrolling, dialogs) | Staged commits 3-7 keep the app runnable at every step |

## 13. Decision log

- **Full replacement, staged.** Rust becomes the project; Python is removed in
  the final commit rather than shipped alongside.
- **Both distribution paths.** CI binaries for aarch64/armv7 plus documented
  on-device builds, because `pkg install rust` cannot be assumed.
- **`ratatui` + `crossterm` + `tui-input`.** Text editing is the one widget worth
  a dependency; lists, forms, focus and dialogs are hand-written to keep
  behaviour predictable.
- **Approach A: enum screens with continuations.** Rejected `dyn Screen` routing
  (same dialog boilerplate, no benefit at this size) and a retained widget tree
  (building a toolkit to use a library).
- **Hybrid navigation.** `Tab` between panes as today, arrows/`j`/`k` inside
  lists, so muscle memory survives but long lists stop being plodding.
- **Canonical hex on save.** Accepted over carrying raw spellings, because it
  only affects managed keys and removes a second representation of every colour.
- **No `regex`, `serde`, `anyhow`, `thiserror`, `walkdir`, `chrono` or `clap`.**
  Each has a small hand-written replacement in the module that needs it.
