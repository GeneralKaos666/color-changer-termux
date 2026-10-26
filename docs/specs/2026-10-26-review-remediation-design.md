# Review remediation — design

- **Date:** 2026-10-26
- **Status:** approved for planning
- **Supersedes nothing.** Amends the three earlier specs where they conflict; the
  2026-10-25 rebrand design remains authoritative for layout and copy.
- **Input:** the two-axis review of the whole tree at `d946599` (9 hard
  violations, 7 modernness gaps, 4 smell classes, 22 spec findings).

## 1. Goal

Close every finding from the review, so that the README, the help bars, and the
code agree, and so the tree reads as a modern ratatui application rather than a
modern skeleton with an unfinished subsystem bolted on.

Success is not "the findings are ticked off". Success is:

1. No operation performed on the input thread can block the UI for longer than
   one frame.
2. Every `render` is a pure function of state; no screen reads or writes the
   filesystem while drawing.
3. No user-visible promise in `README.md` or in a help bar is unimplemented.
4. No path exists where a save, an activation, or a preview can destroy a user
   file without a recoverable copy.
5. `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and
   `cargo fmt --check` are green, and the suite covers the behaviour above
   rather than merely executing it.

Baseline for comparison: 133 tests passing at `d946599`.

## 2. Non-goals

- No new runtime dependencies. In particular **no `tokio`**: four child
  processes and one HTTP GET are served by `std::sync::mpsc` and detached
  threads.
- No theme-library features beyond the findings. The review named none.
- No Windows support work. `apply_theme` is already Unix-only.
- No changes to the colour-derivation maths (`theme.rs`), which the review
  called out as good, beyond making `Palette` `Copy`.
- No redesign of layout; the 2026-10-25 visual design stands.

## 3. Architecture

### 3.1 One reducer, one loop

The event loop stops being "draw, block on `read`, mutate". It becomes a
`poll`-driven loop with a single message type and one worker channel.

```text
src/runtime.rs   Msg, Runtime (channel + worker spawners)
src/main.rs      loop: drain channel -> update -> draw -> poll(250ms)? event : tick
src/app.rs       App::update(Msg) — the only state entry point
```

```rust
pub enum Msg {
    Tick(Instant),
    Key(KeyEvent),
    Resize(u16, u16),
    Reloaded(bool),                          // termux-reload-settings outcome
    PromptCaptured(Vec<String>),             // shell preview lines
    FontsScanned(io::Result<Vec<FontEntry>>),
    DownloadProgress { received: u64, total: Option<u64> },
    Downloaded(Result<PathBuf, String>),
}
```

`App::update` dispatches: `Key` → the existing per-screen key path, `Resize` →
`set_area`, `Tick` → `tick`, workers → the font subsystem. `App::handle_key` and
`App::tick` stay public, so the 133 existing tests keep driving them directly
and none of them need a thread.

Workers are spawned on demand and are short-lived:

| job | spawn site | cap | message |
|---|---|---|---|
| reload settings | font apply/preview | 15 s (reuse `library::reload_termux`) | `Reloaded` |
| capture shell prompt | Fonts tab open | 3 s | `PromptCaptured` |
| scan font library | Fonts tab open | — | `FontsScanned` |
| download + install one font | catalog screen Enter | 60 s | `DownloadProgress`, `Downloaded` |

The Nerd Font catalog itself is a compiled-in table, so it is built once when the
catalog screen is constructed, not per keypress and not on a worker.

Concurrency cap: one in-flight job per kind. A second request is dropped with a
status line, never queued. This keeps at most four short-lived child threads,
which matters on Android.

Because the loop polls at 250 ms, `STATUS_TTL` (4 s) now actually expires while
the user is idle, within 250 ms of its deadline.

### 3.2 Views are pure

`Screen::render` and every screen's `render` take `&self`. Consequences and
where the work moves:

| today | after |
|---|---|
| `App::render` sets `self.too_small` | computed locally in `render` from `frame.area()`; stored copy updated on `Resize` for screens that need it |
| `TerminalEditor::render` calls `Field::set_width` | `Field::set_width` called from `on_resize`; `Field::visible(&self, width)` for the draw |
| `PresetPicker::render` calls `memoize_canonical()` per frame | canonical paths computed in `opened()` and after any palette change |
| `PresetPicker::render` may `read_colors` per frame | every theme's colours read once in `opened()` into `HashMap<PathBuf, TerminalColors>` |
| `App::render` clones the palette per frame | `Palette: Copy` |
| `chrome::gradient_title` rebuilds 15 `String`s per frame | `App` holds a `Line<'static>` rebuilt only when the palette changes |
| `Dialog::render(area, &mut Buffer, ..)` | `impl Widget for &Dialog`, drawn with `frame.render_widget(Clear, area)`; box height computed from the wrapped message so `Wrap` is applied to a box that can hold it |
| `FontsTab::new` runs `sh -ic` inline | placeholder `["$ "]`, replaced by `PromptCaptured` |

A new `Screen::on_resize(&mut self, area: Rect)` is the only place a screen may
derive geometry. `on_resize` is called on `Msg::Resize`, on push, and once at
startup from `terminal.size()`.

### 3.3 One UI context

Signatures grow by one argument, not two, and ASCII mode stops being a global
`AtomicBool`.

```rust
#[derive(Copy, Clone)]
pub struct Glyphs { ascii: bool }   // marker, rule_char, chip, dot, ellipsis,
                                    // filter, breadcrumb, tick, arrow,
                                    // separator, border() -> BorderType

#[derive(Copy, Clone)]
pub struct Ui { pub palette: Palette, pub glyphs: Glyphs }
```

`App` owns `Ui`. `render(&self, frame, area, ui: &Ui)`. `ui::ascii` is deleted;
`cli::Args` builds `Glyphs::new(args.ascii)`. A shared `ui::pane(ui, title)`
builds every bordered block, so `--ascii` reaches all nine panes and the help
bar in one place. Env vars `TERMUX_SETTINGS_ASCII` / `TERMUX_COLORS_ASCII` /
`FONT_CHANGER_ASCII` keep working.

### 3.4 Typed font slots

```rust
pub enum FontSlot { Regular, Bold, Italic, BoldItalic }
impl FontSlot {
    pub const ALL: [Self; 4];
    pub fn as_str(self) -> &'static str;      // "regular", ...
    pub fn file_name(self) -> &'static str;   // "font.ttf", "font-bold.ttf", ...
}
impl TryFrom<&str> for FontSlot { type Error = crate::error::Error; }
```

`Paths::font_slot_path(FontSlot)` replaces the `&str` + `match`; `Paths::slot_names`
is deleted. `Args.slot: Option<FontSlot>` — an unknown `--slot` becomes a CLI
error instead of a silently-ignored `Invalid` at first use.

### 3.5 Picker chrome

The preset picker gains the two elements the 2026-10-25 design promised but that
were never implemented: a header showing the active theme's library-relative
name, and a scrollbar when the list overflows. The scrollbar is a local
`ScrollbarState::new(len).position(offset)` built inside `render` — it mutates
no screen state, so the purity rule holds.

## 4. Fonts subsystem

### 4.1 Session state restores every touched slot

The bug the review ranked first: preview into `regular`, `s`, preview into
`bold`, `Esc` leaves `regular` previewed forever, because only the last
`preview` reference is remembered.

```rust
pub struct SessionState {
    originals: HashMap<FontSlot, Option<Vec<u8>>>,  // session start bytes
    previewed: BTreeMap<FontSlot, PreviewRef>,      // slot -> src
}
```

- `preview_font` inserts into `previewed`; it never overwrites another slot's
  entry.
- `restore_original` walks **all** of `previewed`, writes back the original
  bytes (or removes the file when the slot did not exist), then clears.
- `commit_preview(&Paths, &mut SessionState)` clears `previewed` without
  restoring, and takes `&Paths` — `Paths::from_env()` disappears from the
  preview path.
- `App` gains `fn on_exit(&mut self, paths: &Paths)`, called for each screen on
  the stack before the loop breaks, so `q`/`Ctrl+C`/`Ctrl+Q` restore an
  uncommitted preview instead of leaving the font installed. `Screen::on_quit`
  is the per-screen hook.

### 4.2 Backups: one implementation, correct dates

`fonts::apply::format_timestamp` (1970 + days/365 arithmetic, 16-month error) and
the 15-vs-17-character mismatch both die.

- `persist::backup::make_backup` gains a sibling that takes the extension:
  `make_unique_backup(source, dir, prefix, ext)`. `make_backup` delegates with
  `"properties"`.
- `fonts::apply::ensure_backup_once` calls it, and its dedupe check uses
  `backup::is_stamped_name(name, stem, ext)` — a shared predicate that accepts
  exactly `YYYY-MM-DD-HHMMSS` (17 chars, dashes at 4/7/10, digits elsewhere).
- In-session dedupe additionally short-circuits on `SessionState::backed_up`, so
  a preview never writes a second backup for the same slot.
- `Backup::utc_fallback` is no longer discarded: the editor that triggered the
  save reports it once per session in the status line.

### 4.3 One reload path

`fonts::apply::reload_settings` is deleted along with `LAST_RELOAD_OK`,
`last_reload_ok`, and the `static mut`. All call sites use
`library::reload_termux` (15 s cap, kills on timeout), spawned as a worker on
the TUI path and run synchronously only for the CLI `--apply-font` path. A
failed reload surfaces `MANUAL_RESTART_HINT`.

### 4.4 Fonts tab behaves as advertised

| key | after |
|---|---|
| `↑`/`↓`, `j`/`k` | move within the **filtered** list |
| `p` / `space` | preview into the selected slot; failures shown, not swallowed |
| `Enter` | commit; status reflects success or the reload hint |
| `s` | cycle `FontSlot::ALL` |
| `i` | path prompt → `importer::import_file`; on clash, keep-both / replace / cancel dialog |
| `d` | opens the Nerd Font catalog screen |
| `/` | filter; typing narrows `visible`; `Esc` clears the filter, `Esc` again leaves |
| `Esc` | restore every previewed slot, then pop |
| `q` | quit from anywhere (restoring), as the 10-24 keymap requires |

Filtering: `visible: Vec<usize>` recomputed on each edit from
`name`/`family`/`style`, case-insensitive substring. The filter input is
rendered when `show_filter` is on. `ListState` indexes `visible`.

`FontsTab::new` no longer copies the active font into `fonts/Current.ttf`; that
side effect was in neither spec.

### 4.5 Import and download

- **Import** reuses `importer::import_file` unchanged in behaviour, but is now
  reachable: `ask` mode's error is caught and converted into the clash dialog,
  which re-runs with `keep-both` or `replace`. `STORAGE_HINT` is shown verbatim
  for permission errors.
- **Download** gets a real screen, `screens/font_catalog.rs`: the catalog sorted
  by name, filterable, `Enter` starts a download, footer shows
  `name 42%` from `DownloadProgress`, completion refreshes the library and shows
  a status. `catalog.iter().next()` disappears.
- `downloader::fetch` splits into `download(url, on_progress) -> Result<Vec<u8>>`
  (the `ureq` call, with `Content-Length` when present) and
  `install_bytes(paths, name, &bytes) -> Result<PathBuf>` (validate + atomic
  write into the library). Only the second is unit-tested; the network path is
  covered by a `file://`-style fake through a test-only `Reader` closure.
- The discarded `HEAD` request is deleted.

### 4.6 Shell prompt capture

`capture_prompt_lines(shell: Option<&str>) -> Vec<String>`:

- uses `$SHELL` when set, `sh` otherwise;
- `zsh` → `print -P "$PROMPT"`, `bash` → `printf '%s' "${PS1@P}"`, other → `$ `;
- 3 s cap: poll `try_wait` every 25 ms, then kill and fall back;
- reads stdout only after the child exits, so `read_to_string` cannot block
  forever;
- never panics; on any failure returns `["$ "]`.

It runs on a worker; `FontsTab` starts with the fallback and swaps in the real
lines when `PromptCaptured` arrives.

## 5. Safety-critical file handling

### 5.1 Atomic writes

New `persist::fs::write_atomic(path, bytes)`:

1. resolve `path` — if it is a symlink, write to `canonicalize(path)` so
   activating a theme keeps writing *through* the link (today's behaviour);
2. write `<dir>/.<name>.tmp-<pid>` in the same directory, `sync_all`;
3. copy the existing file's permissions when there is one;
4. `rename` over the destination.

`write_colors` and `write_app_theme` use it. A crash or a full disk can no
longer truncate `colors.properties` / `termux.properties`, whether or not the
user answered "Skip" to the backup prompt. Byte-fidelity behaviour
(`preserve`) is untouched; the round-trip fixtures are the regression test.

### 5.2 Activating a theme cannot destroy it

`library::apply_theme`:

1. `canonicalize(theme)` — `NotFound` when missing; `Invalid` when it resolves
   to a directory;
2. if `colors_properties` resolves to the same file → no-op (kept);
3. reject a `colors_properties` that exists and is a directory;
4. create `.<name>.tmp-<pid>` symlink in the same directory, then `rename` it
   over `colors_properties` — atomic replacement, no window in which the active
   config is missing, and no `remove_file` before a `symlink` that can fail;
5. on any error, remove the temp link and leave the old one in place.

### 5.3 Unreadable app theme is not "delete all settings"

`AppThemeEditor::new` keeps the read error instead of `unwrap_or_default()`. When
set, the screen shows a message dialog on open, refuses to save with a status
line, and `Ctrl+S` never reaches `unset_value`. Colour editing keeps its
default-fallback, which the spec allows.

## 6. Cleanup

Deleted, because the review found no user:

- `fonts::apply::{last_reload_ok, is_preview_dirty, format_timestamp}`,
  `LAST_RELOAD_OK`, `let _ = meta`, `_resolved`
- `fonts::scan::{describe, read_active}`, `model::colors::Colors::is_managed`
- `ui::ascii` (global), `Screen::new -> Option` and the `unreachable!` in `App`
- `ScreenAction::{RefreshPalette, Status, RefreshPaletteAndStatus}` — replaced by
  one `Applied { colors: Option<TerminalColors>, message: Option<String> }`
- `Paths::{backups, slot_names}`, the duplicated `read_text`
  (→ `persist::read_text`), the discarded `HEAD` request
- `downloader`'s unordered catalog access

`ScreenAction` gains constructors (`ScreenAction::status(..)`,
`ScreenAction::applied(..)`) so call sites stay one line.

`Screen::refresh` stops being a one-variant `if let`: it becomes
`Screen::reload(&mut self, paths)`, called for every screen when it is revealed
by a pop, and each screen decides what is stale (the menu's active theme, the
picker's colour cache and canonical paths, the editors' file mtime).

Also folded in: `Row.value` stops mirroring `Row.field` (text rows derive their
value from the field, selector rows keep an index), `[Row; 9]` becomes a `Vec`
built from `CURATED_FIELDS`, and `AppThemeEditor`/`TerminalEditor` no longer hold
a second copy of `Paths` alongside the parameter.

## 7. CLI

`main` becomes `fn main() -> ExitCode` over `fn run() -> Result<ExitCode, Error>`:

| situation | before | after |
|---|---|---|
| unreadable font library with `--list-fonts` | prints nothing, exit 0 | `error: …` on stderr, exit 1 |
| `--apply-font` for an unknown name | exit 2 | exit 2, same message |
| `--slot bogus` | ignored until first apply | `error: unknown font slot: "bogus"`, exit 2 |
| unknown flag | ignored | still ignored (out of scope) |

No `process::exit` below `run`.

So that `--list-fonts` can report an unreadable directory instead of an empty
list, `scan::list_library` distinguishes "no library yet" (`NotFound` →
`Ok(vec![])`) from "cannot read it" (`PermissionDenied`/other → `Err`). The TUI
treats the error as an empty list plus a status line; the CLI exits 1.

## 8. Testing

Existing 133 tests stay green; deliberate behaviour changes get new tests rather
than edited expectations where possible (`q` at root, `Esc` on Fonts, dialog
height, ASCII borders).

New coverage, mapped to the findings:

| area | tests |
|---|---|
| slots | `FontSlot::ALL` ↔ `file_name` ↔ `as_str` round-trip; `TryFrom` rejects garbage |
| backups | stamp predicate accepts 17-char names only; `ensure_backup_once` writes once per slot per session; name parses as a real date (regression for the 16-month bug) |
| restore | two-slot preview then `Esc` restores both; quit during preview restores; commit keeps both and leaves nothing to restore |
| atomic write | symlink target is written through; failed write leaves original bytes; permissions preserved |
| activation | directory target rejected; old link survives a failed swap; self-link no-op; replace keeps working |
| app theme | unreadable file → no save, dialog shown, `unset` never called |
| import | keep-both creates `-1`, replace overwrites, cancel leaves the library untouched, permission hint surfaces |
| download | `install_bytes` validates and writes atomically; progress accounting sums to the total; bad payload rejected |
| prompt | fake shell script → parsed lines; hung shell → fallback after the cap |
| filter | narrows by name/family/style, case-insensitive, selection stays visible, `Esc` clears |
| runtime | `Tick` expires status within one tick of the TTL; `Resize` sets `too_small`; `q` quits at root; `Esc` pops a sub-screen |
| picker chrome | header shows the active theme's library-relative name; scrollbar renders only when the list overflows; `apply_theme` self-link is a no-op |
| contrast | `is_dark` boundary either side of the 0.35 threshold; `Palette` stays `Copy` and derives the same colours as before |
| dialog | a multi-line confirm renders every line (regression for the 1-row wrap box) |
| ascii | every pane uses `BorderType::Plain`; help bar has no non-ASCII glyphs |

Weak tests the review named are replaced, not deleted: `ui_app_theme`'s empty
test asserts selector cycling and the unset state; `ui_picker`'s
`"Browse Presets" || "Colors"` assertion asserts the actual destination;
`ui_dialog`'s unasserted result is asserted.

## 9. Acceptance criteria

1. Zero `unsafe`, zero `static mut`, zero `process::exit` outside `run`.
2. No `fs::`/`Command::`/`read_colors` call inside any `render` body.
3. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test --all-targets` all pass.
4. `README.md` and every `help()` list match the keyboard table in §4.4 and the
   CLI table in §7; no advertised feature is a stub.
5. Every row of §8's table exists as a named test.

## 10. Risks

- **Termux/Android thread churn.** Jobs are one-shot and capped; the app still
  runs exactly one event loop. If a worker is killed by the OS, the message
  simply never arrives and the screen keeps its previous state — no deadlock,
  because the loop never waits on a worker.
- **Eager colour reads on picker open.** A very large library now costs one read
  per theme at open instead of per frame. Bounded by library size; strictly less
  I/O overall.
- **`Palette: Copy`** is mechanical but touches every `&palette` site; the
  compiler finds them all.
- **Behaviour changes are user-visible** (`q` quits, `Esc` leaves Fonts, taller
  dialogs, ASCII everywhere). This is the point of the remediation, and README
  is updated in the same change.
