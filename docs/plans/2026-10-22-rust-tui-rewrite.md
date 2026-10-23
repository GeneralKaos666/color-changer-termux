# Rust TUI Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Textual implementation of `termux-colors` with a single ratatui binary that preserves every behaviour and file-format guarantee.

**Architecture:** One `App` value owns a screen stack, an optional dialog, a derived `Palette` and a status line. Screens are enum variants with their own focus/selection state that return `ScreenAction` values; `App` applies them. Multi-step dialog flows are explicit `pending` enums on the screen that asked.

**Tech Stack:** Rust 2024, ratatui, crossterm, tui-input, time (`local-offset`), tempfile (dev).

**Spec:** `docs/specs/2026-10-22-rust-tui-rewrite-design.md`

## Global Constraints

- Edition 2024, `rust-version = "1.85"`, stable toolchain.
- Allowed dependencies: `ratatui`, `crossterm`, `tui-input`, `time` with `local-offset`, `tempfile` (dev only). No `regex`, `serde`, `anyhow`, `thiserror`, `walkdir`, `chrono`, `clap`, `log`.
- Unix-only is acceptable. Pure-Rust dependencies only; no C libraries.
- Never read `TERMUX_HOME`/`HOME` outside `Paths::from_env`. Every other function takes `&Paths` or a `&Path`.
- Tests must never touch the real `~/.termux`; use `tempfile::tempdir()`.
- Managed colour keys (`background`, `foreground`, `cursor`, `color0`..`color15`) are written as lowercase `#rrggbb`. Comments, unmanaged keys, ordering, indentation and line endings are preserved.
- Terminal is 24-bit colour; no quantisation layer.
- Keyboard only.
- `cargo fmt` clean and `cargo clippy -- -D warnings` clean at every commit.

## Review Focus

The inputs and conditions most likely to bite a user, and the task that pins each:

1. A `termux.properties` with an `extra-keys` array or comments — must survive a save byte-for-byte (Task 6, Task 11).
2. `colors.properties` that is a plain file edited in place, not a symlink — applying its own path must not create a self-referential link (Task 8).
3. A CRLF `colors.properties` without a trailing newline — line endings and the missing newline must be preserved (Task 3, Task 11).
4. An empty or backup-littered theme library — built-ins are seeded once, backups and dot-directories never listed (Task 9, Task 15).
5. A terminal narrower or shorter than the layout needs, or a resize mid-session — a guard, never a panic (Task 20).

Also pinned: invalid hex while saving (Task 18), unreadable theme file during a scan (Task 8), backup collisions inside one second (Task 7), a prompt answered with blank input (Task 18).

---

### Task 1: Crate skeleton, `Paths`, `Error`

**Files:**
- Create: `Cargo.toml`, `src/lib.rs`, `src/main.rs`, `src/error.rs`, `src/paths.rs`, `.gitignore` (modify)
- Test: `tests/paths.rs`

**Interfaces:**
- Produces: `Error { Io(std::io::Error), Parse(String), NotFound(PathBuf), Invalid(String) }`, `type Result<T> = std::result::Result<T, Error>`; `#[derive(Clone, Debug)] Paths` with `Paths::new(root) -> Paths`, `Paths::from_env() -> Paths`, `Paths::from_parts(termux_home: Option<PathBuf>, home: Option<PathBuf>) -> Paths`, `Paths::{root, colors_root, colors_properties, termux_properties, backups}`.

- [ ] **Step 1: Create `Cargo.toml` with the full dependency set**

```toml
[package]
name = "termux-colors"
version = "0.2.0"
edition = "2024"
rust-version = "1.85"
license = "MIT"

[lib]
name = "termux_colors"

[[bin]]
name = "termux-colors"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tui-input = "0.11"
time = { version = "0.3", features = ["formatting", "local-offset"] }

[dev-dependencies]
tempfile = "3"

[profile.release]
opt-level = "s"
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"
```

Add `/target` to `.gitignore`. Note: crate versions are floors, not exact pins; if a newer minor is required by the compiler, bump it and record the version in the commit message.

- [ ] **Step 2: Write the failing test**

```rust
// tests/paths.rs
use std::path::PathBuf;
use termux_colors::paths::Paths;

#[test]
fn accessors_join_under_root() {
    let p = Paths::new("/tmp/home/.termux");
    assert_eq!(p.colors_root(), PathBuf::from("/tmp/home/.termux/colors"));
    assert_eq!(p.colors_properties(), PathBuf::from("/tmp/home/.termux/colors.properties"));
    assert_eq!(p.termux_properties(), PathBuf::from("/tmp/home/.termux/termux.properties"));
    assert_eq!(p.backups(), PathBuf::from("/tmp/home/.termux/backups"));
}

#[test]
fn termux_home_wins_over_home() {
    let p = Paths::from_parts(Some(PathBuf::from("/custom")), Some(PathBuf::from("/home/u")));
    assert_eq!(p.colors_root(), PathBuf::from("/custom/colors"));
    assert_eq!(p.root(), std::path::Path::new("/custom"));
}

#[test]
fn home_is_the_fallback() {
    let p = Paths::from_parts(None, Some(PathBuf::from("/home/u")));
    assert_eq!(p.colors_properties(), PathBuf::from("/home/u/.termux/colors.properties"));
}
```

- [ ] **Step 3: Run it and confirm it fails**

Run: `cargo test --test paths`
Expected: FAIL — crate does not compile, `paths` module missing.

- [ ] **Step 4: Implement `src/error.rs` and `src/paths.rs`, declare both from `src/lib.rs`**

`lib.rs` starts as `pub mod error; pub mod paths;`. `main.rs` starts as `fn main() {}` (Task 13 replaces it). `Paths::from_env` is a three-line lookup delegating to `from_parts`; do not test it directly, because it is the only function allowed to touch process environment and tests would race.

- [ ] **Step 5: Run the tests**

Run: `cargo test --test paths`
Expected: PASS (3 tests).

- [ ] **Step 6: Confirm the crossterm versions agree**

Run: `cargo tree -d`
Expected: exactly one `crossterm` version. If `tui-input` and `ratatui` pull different ones, change the `tui-input` version to the release that matches ratatui's `crossterm`, and record the pinned pair in the commit message.

- [ ] **Step 7: Format, lint, commit**

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings
git add Cargo.toml Cargo.lock .gitignore src/lib.rs src/main.rs src/error.rs src/paths.rs tests/paths.rs
git commit -S -m "feat: scaffold Rust crate with paths and error types"
```

---

### Task 2: `model::colors`

**Files:**
- Create: `src/model/mod.rs`, `src/model/colors.rs`
- Modify: `src/lib.rs`
- Test: `src/model/colors.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Error`, `Result` from Task 1.
- Produces: `Rgb { r, g, b }` with `const fn new(u8,u8,u8)`, `Rgb::parse(&str) -> Option<Rgb>`, `to_hex() -> String`, `Display`; constants `PALETTE_SIZE: usize = 16`, `PALETTE_KEYS: [&str; 16]`, `ANSI_NAMES: [&str; 16]`, `DEFAULT_PALETTE: [Rgb; 16]`; `TerminalColors { background, foreground, cursor, palette: [Rgb; 16], extra: Vec<(String, String)> }` with `Default`; `ColorField { Background, Foreground, Cursor, Palette(usize) }` with `ALL: [ColorField; 19]`, `key() -> String`, `label() -> String`; `TerminalColors::{field, set_field, is_managed}`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn parses_short_and_long_hex() {
    assert_eq!(Rgb::parse("#0f0c1a"), Some(Rgb::new(0x0f, 0x0c, 0x1a)));
    assert_eq!(Rgb::parse("#ABC"), Some(Rgb::new(0xaa, 0xbb, 0xcc)));
    assert_eq!(Rgb::parse("#abc").unwrap().to_hex(), "#aabbcc");
}

#[test]
fn rejects_malformed_hex() {
    for bad in ["abc", "#abcd", "#gggggg", "", "#12 345", "0xffffff"] {
        assert_eq!(Rgb::parse(bad), None, "{bad}");
    }
}

#[test]
fn defaults_match_the_xterm_palette() {
    let c = TerminalColors::default();
    assert_eq!(c.palette.len(), PALETTE_SIZE);
    assert_eq!(c.palette[0].to_hex(), "#000000");
    assert_eq!(c.palette[1].to_hex(), "#cc0000");
    assert_eq!(c.palette[15].to_hex(), "#eeeeee");
    assert_eq!(c.extra.len(), 0);
}

#[test]
fn field_round_trip_covers_all_nineteen() {
    let mut c = TerminalColors::default();
    assert_eq!(ColorField::ALL.len(), 19);
    assert_eq!(ColorField::ALL[0].key(), "background");
    assert_eq!(ColorField::ALL[3].key(), "color0");
    assert_eq!(ColorField::ALL[18].label(), "color15  (bright white)");
    c.set_field(ColorField::Palette(4), Rgb::new(1, 2, 3));
    assert_eq!(c.field(ColorField::Palette(4)), Rgb::new(1, 2, 3));
}

#[test]
fn managed_keys_exclude_extras() {
    let c = TerminalColors { extra: vec![("selection-background".into(), "#333333".into())], ..Default::default() };
    assert!(c.is_managed("cursor"));
    assert!(c.is_managed("color15"));
    assert!(!c.is_managed("selection-background"));
}
```

- [ ] **Step 2: Run and confirm failure**

Run: `cargo test model::colors`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `Rgb`, the constants and `TerminalColors`**

`PALETTE_KEYS` is `["color0", ..., "color15"]`. `ANSI_NAMES` and `DEFAULT_PALETTE` copy `models/colors.py` verbatim. `ColorField::label` returns `"background"`, `"foreground"`, `"cursor"` for the first three and `format!("color{i}  ({})", ANSI_NAMES[i])` for palette entries.

- [ ] **Step 4: Run and confirm pass, then commit**

```bash
cargo test model::colors && cargo fmt --check && cargo clippy --all-targets -- -D warnings
git add src/model src/lib.rs
git commit -S -m "feat: add colour model with hex parsing and field accessors"
```

---

### Task 3: `persist::properties`

The highest-risk module: it decides whether users lose their comments.

**Files:**
- Create: `src/persist/mod.rs`, `src/persist/properties.rs`
- Modify: `src/lib.rs`
- Test: `src/persist/properties.rs`

**Interfaces:**
- Produces: `parse_active(&str) -> Vec<(String, String)>` (file order, active keys only), `set_value(&str, key, value) -> String`, `unset_value(&str, key) -> String`.

- [ ] **Step 1: Write the failing tests**

```rust
const SAMPLE: &str = "extra-keys = [['ESC','-'],\\ ['TAB',':']]\n\nallow-external-apps = true\n\n# fullscreen = true\n\nterminal-transcript-rows = 50000\nuse-black-ui = true\n";

#[test]
fn parses_only_active_keys_in_order() {
    let pairs = parse_active(SAMPLE);
    assert_eq!(pairs[0], ("extra-keys".into(), "[['ESC','-'],\\ ['TAB',':']]".into()));
    assert_eq!(pairs[1].0, "allow-external-apps");
    assert!(!pairs.iter().any(|(k, _)| k == "fullscreen"));
    assert!(parse_active("").is_empty());
}

#[test]
fn uncomments_in_place_keeping_indent_and_spacing() {
    let out = set_value("# fullscreen = true\nx=1\n", "fullscreen", "true");
    assert_eq!(out, "fullscreen = true\nx=1\n");
}

#[test]
fn keeps_separator_style_of_existing_line() {
    assert_eq!(set_value("use-black-ui = true\n", "use-black-ui", "false"), "use-black-ui = false\n");
    assert_eq!(set_value("use-black-ui=true\n", "use-black-ui", "false"), "use-black-ui=false\n");
}

#[test]
fn appends_new_keys_compactly() {
    assert_eq!(set_value("a=1\n", "b", "2"), "a=1\nb=2\n");
    assert_eq!(set_value("", "b", "2"), "b=2\n");
}

#[test]
fn preserves_crlf_and_missing_trailing_newline() {
    assert_eq!(set_value("a=1\r\n", "a", "2"), "a=2\r\n");
    assert_eq!(set_value("a=1", "a", "2"), "a=2");
}

#[test]
fn unset_comments_out_without_deleting() {
    assert_eq!(unset_value("a=1\nb=2\n", "a"), "# a = 1\nb=2\n");
    assert_eq!(unset_value("a=1\n", "missing"), "a=1\n");
}

#[test]
fn non_key_lines_are_untouched() {
    let text = "extra-keys = [\n  [\"TAB\", \":\"]\n]\n";
    assert_eq!(set_value(text, "cursor", "#ffffff"), "extra-keys = [\n  [\"TAB\", \":\"]\n]\ncursor=#ffffff\n");
}
```

- [ ] **Step 2: Run and confirm failure**

Run: `cargo test persist::properties`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement the three functions**

Hand-scan lines without `regex`: detect `\r\n` first for output, split on `\n`, strip a trailing `\r` per line, and match `^(indent)(#?)(key)(\s*=\s*)(value)$` where the key charset is `[A-Za-z0-9_.-]`. Keep the original separator and indent when a key is found; append `key=value` when it is not. Preserve a trailing newline only when the input had one (or was empty).

- [ ] **Step 4: Run and confirm pass, then commit**

```bash
cargo test persist::properties
git add src/persist src/lib.rs
git commit -S -m "feat: add line-preserving properties editor"
```

---

### Task 4: `persist::colors_io`

**Files:**
- Create: `src/persist/colors_io.rs`
- Modify: `src/persist/mod.rs`
- Test: `src/persist/colors_io.rs`, `tests/colors_io.rs` (symlink case)

**Interfaces:**
- Consumes: `parse_active`, `set_value` (Task 3); `TerminalColors`, `ColorField`, `DEFAULT_PALETTE`, `PALETTE_KEYS` (Task 2).
- Produces: `MANAGED_KEYS: [&str; 19]`; `parse_colors(&str) -> TerminalColors`, `render_colors(&TerminalColors) -> String`, `update_colors_text(&str, &TerminalColors) -> String`, `read_colors(&Path) -> Result<TerminalColors>`, `write_colors(&Path, &TerminalColors, preserve: bool) -> Result<()>`.

- [ ] **Step 1: Write the failing tests**

```rust
const PARTIAL: &str = "background=#0f0c1a\nforeground=#f8f8f2\ncolor0=#0f0c1a\ncolor1=#ff3366\nselection-background=#333333\n";

#[test]
fn partial_files_fall_back_to_defaults() {
    let c = parse_colors(PARTIAL);
    assert_eq!(c.background.to_hex(), "#0f0c1a");
    assert_eq!(c.cursor.to_hex(), "#f8f8f2");          // falls back to foreground
    assert_eq!(c.palette[2], DEFAULT_PALETTE[2]);
}

#[test]
fn cursor_falls_back_to_white_without_foreground() {
    assert_eq!(parse_colors("").cursor.to_hex(), "#ffffff");
}

#[test]
fn extras_keep_file_order() {
    let c = parse_colors("background=#000000\nzzz=1\naaa=2\n");
    assert_eq!(c.extra, vec![("zzz".into(), "1".into()), ("aaa".into(), "2".into())]);
}

#[test]
fn render_writes_header_and_key_order() {
    let out = render_colors(&TerminalColors::default());
    assert!(out.starts_with("# Generated by termux-colors\nbackground=#000000\nforeground=#ffffff\ncursor=#ffffff\n\ncolor0=#000000\n"));
    assert!(out.ends_with("color15=#eeeeee\n"));
}

#[test]
fn update_keeps_comments_and_extras() {
    let text = "# mine\nbackground=#000000\ncolor3=#111111\nselection-background=#333333\n";
    let out = update_colors_text(text, &TerminalColors::default());
    assert!(out.starts_with("# mine\n"));
    assert!(out.contains("selection-background=#333333\n"));
    assert!(out.contains("color3=#c4a000\n"));
    assert!(out.contains("background=#000000\n"));
}

#[test]
fn blank_text_renders_fresh() {
    assert_eq!(update_colors_text("  \n", &TerminalColors::default()), render_colors(&TerminalColors::default()));
}
```

- [ ] **Step 2: Add the integration test for write-through-symlink**

```rust
// tests/colors_io.rs
#[test]
fn writing_the_link_edits_the_target_and_keeps_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("Theme.properties");
    std::fs::write(&target, "background=#000000\n").unwrap();
    let link = dir.path().join("colors.properties");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let mut colors = read_colors(&link).unwrap();
    colors.background = Rgb::new(1, 2, 3);
    write_colors(&link, &colors, true).unwrap();

    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert!(std::fs::read_to_string(&target).unwrap().contains("background=#010203"));
}

#[test]
fn write_creates_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("custom/deep/Theme.properties");
    write_colors(&path, &TerminalColors::default(), false).unwrap();
    assert!(path.is_file());
}
```

- [ ] **Step 3: Run and confirm failure**

Run: `cargo test colors_io`
Expected: FAIL — functions missing.

- [ ] **Step 4: Implement the module**

`parse_colors` reads `background`/`foreground` as strings first and only then parses them, so `cursor` can fall back to `foreground` before the fallback to white. Anything that fails `Rgb::parse` falls back to its default. `write_colors` writes with `std::fs::write`, which follows symlinks.

- [ ] **Step 5: Run, lint, commit**

```bash
cargo test colors_io && cargo clippy --all-targets -- -D warnings
git add src/persist/colors_io.rs src/persist/mod.rs tests/colors_io.rs
git commit -S -m "feat: read and write colors.properties with preservation"
```

---

### Task 5: `model::app_theme`

**Files:**
- Create: `src/model/app_theme.rs`
- Modify: `src/model/mod.rs`
- Test: `src/model/app_theme.rs`

**Interfaces:**
- Produces: `FieldKind { Text, Bool, Choice, Int }`; `FieldSpec { key: &'static str, label: &'static str, kind: FieldKind, choices: &'static [&'static str], min: Option<i64>, max: Option<i64>, help: &'static str, section: &'static str }`; `CURATED_FIELDS: [FieldSpec; 9]`; `spec(key) -> Option<&'static FieldSpec>`; `validate_value(&FieldSpec, &str) -> std::result::Result<String, String>`; `AppTheme { values: HashMap<String, String> }` with `get`, `set`, `is_empty`.

- [ ] **Step 1: Write the failing tests**

```rust
fn field(key: &str) -> &'static FieldSpec { spec(key).unwrap() }

#[test]
fn curated_fields_match_the_python_schema() {
    assert_eq!(CURATED_FIELDS.len(), 9);
    assert_eq!(CURATED_FIELDS[0].key, "use-black-ui");
    assert_eq!(CURATED_FIELDS[0].kind, FieldKind::Bool);
    assert_eq!(CURATED_FIELDS[8].key, "allow-external-apps");
    assert_eq!(field("terminal-cursor-style").choices, &["block", "bar", "underline"]);
    assert_eq!(field("terminal-transcript-rows").min, Some(100));
    assert_eq!(field("terminal-transcript-rows").max, Some(50000));
}

#[test]
fn validates_with_the_documented_messages() {
    assert_eq!(validate_value(field("use-black-ui"), " TRUE "), Ok("true".into()));
    assert_eq!(validate_value(field("use-black-ui"), "yes"),
               Err("Black UI must be true or false".into()));
    assert_eq!(validate_value(field("terminal-cursor-style"), "blink"),
               Err("Cursor style must be one of: block, bar, underline".into()));
    assert_eq!(validate_value(field("terminal-margin-horizontal"), "51"),
               Err("Margin horizontal must be <= 50".into()));
    assert_eq!(validate_value(field("terminal-transcript-rows"), "x"),
               Err("Transcript rows must be a whole number".into()));
    assert_eq!(validate_value(field("use-black-ui"), ""), Ok(String::new()));
}
```

- [ ] **Step 2: Run and confirm failure**

Run: `cargo test model::app_theme` — FAIL, module missing.

- [ ] **Step 3: Implement the schema and validation**

Copy keys, labels, sections, help text and bounds from `models/app_theme.py`. `validate_value` trims, returns `Ok("")` for empty, lowercases bools, and returns canonical integers.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test model::app_theme && cargo clippy --all-targets -- -D warnings
git add src/model/app_theme.rs src/model/mod.rs
git commit -S -m "feat: add curated termux.properties schema and validation"
```

---

### Task 6: `persist::theme_io`

**Files:**
- Create: `src/persist/theme_io.rs`
- Modify: `src/persist/mod.rs`
- Test: `src/persist/theme_io.rs`

**Interfaces:**
- Consumes: `parse_active`, `set_value`, `unset_value` (Task 3); `AppTheme`, `CURATED_FIELDS` (Task 5).
- Produces: `read_app_theme(&Path) -> Result<AppTheme>`; `update_theme_text(&str, &AppTheme) -> String`; `write_app_theme(&Path, &AppTheme, preserve: bool) -> Result<()>`.

- [ ] **Step 1: Write the failing tests**

```rust
const SAMPLE: &str = "extra-keys = [['ESC','-']]\n\nallow-external-apps = true\n\n# fullscreen = true\n\nterminal-transcript-rows = 50000\nuse-black-ui = true\nbackground=background.jpg\n";

#[test]
fn reads_only_curated_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("termux.properties");
    std::fs::write(&path, SAMPLE).unwrap();
    let theme = read_app_theme(&path).unwrap();
    assert_eq!(theme.get("use-black-ui"), "true");
    assert_eq!(theme.get("terminal-transcript-rows"), "50000");
    assert_eq!(theme.get("fullscreen"), "");
    assert_eq!(theme.get("background"), "");
}

#[test]
fn missing_file_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert!(read_app_theme(&dir.path().join("termux.properties")).unwrap().is_empty());
}

#[test]
fn write_preserves_unmanaged_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("termux.properties");
    std::fs::write(&path, SAMPLE).unwrap();
    let mut theme = read_app_theme(&path).unwrap();
    theme.set("terminal-transcript-rows", "1000");
    write_app_theme(&path, &theme, true).unwrap();
    let out = std::fs::read_to_string(&path).unwrap();
    assert!(out.contains("extra-keys = [['ESC','-']]\n"));
    assert!(out.contains("background=background.jpg\n"));
    assert!(out.contains("terminal-transcript-rows = 1000\n"));
}

#[test]
fn empty_value_comments_the_key_out() {
    let mut theme = AppTheme::default();
    theme.set("use-black-ui", "");
    assert_eq!(update_theme_text("use-black-ui = true\n", &theme), "# use-black-ui = true\n");
}

#[test]
fn non_curated_keys_are_ignored() {
    let mut theme = AppTheme::default();
    theme.set("background", "#000000");
    assert_eq!(update_theme_text("a=1\n", &theme), "a=1\n");
}

#[test]
fn absent_file_starts_from_the_header() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("termux.properties");
    let mut theme = AppTheme::default();
    theme.set("use-black-ui", "true");
    write_app_theme(&path, &theme, true).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(),
               "# Termux properties managed by termux-colors\nuse-black-ui=true\n");
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test persist::theme_io`, FAIL.

- [ ] **Step 3: Implement the module**

`update_theme_text` iterates `CURATED_FIELDS` in schema order and applies `set_value`/`unset_value`, ignoring keys absent from `AppTheme::values`.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test persist::theme_io && cargo clippy --all-targets -- -D warnings
git add src/persist/theme_io.rs src/persist/mod.rs
git commit -S -m "feat: read and write curated termux.properties settings"
```

---

### Task 7: `persist::backup`

**Files:**
- Create: `src/persist/backup.rs`
- Modify: `src/persist/mod.rs`
- Test: `src/persist/backup.rs`

**Interfaces:**
- Produces: `Backup { path: PathBuf, utc_fallback: bool }`; `format_timestamp(at: OffsetDateTime, offset: Option<UtcOffset>) -> String` (format `%Y-%m-%d-%H%M%S`); `make_backup(&Path, &Path, prefix: &str) -> Result<Option<Backup>>`; `list_backups(&Path) -> Result<Vec<PathBuf>>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn formats_in_the_given_offset() {
    let at = OffsetDateTime::from_unix_timestamp(1_767_225_600).unwrap(); // 2026-01-01T00:00:00Z
    let offset = UtcOffset::from_hms(-8, 0, 0).unwrap();
    assert_eq!(format_timestamp(at, Some(offset)), "2025-12-31-160000");
    assert_eq!(format_timestamp(at, None), "2026-01-01-000000");
}

#[test]
fn copies_content_and_follows_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("Theme.properties");
    std::fs::write(&target, "foreground=#ffffff\n").unwrap();
    let link = dir.path().join("colors.properties");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let backups = dir.path().join("backups");
    let backup = make_backup(&link, &backups, "colors").unwrap().unwrap();

    assert!(backup.path.starts_with(&backups));
    assert!(backup.path.file_name().unwrap().to_str().unwrap().starts_with("colors-"));
    assert_eq!(std::fs::read_to_string(&backup.path).unwrap(), "foreground=#ffffff\n");
}

#[test]
fn collisions_get_a_counter_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("colors.properties");
    std::fs::write(&source, "a=1\n").unwrap();
    let backups = dir.path().join("backups");
    let first = make_backup(&source, &backups, "colors").unwrap().unwrap().path;
    let second = make_backup(&source, &backups, "colors").unwrap().unwrap().path;
    assert_ne!(first, second);
    assert!(second.file_name().unwrap().to_str().unwrap().contains("-1.properties"));
}

#[test]
fn absent_source_is_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(make_backup(&dir.path().join("nope"), &dir.path().join("b"), "x").unwrap().is_none());
}

#[test]
fn lists_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("old"), b"x").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(dir.path().join("new"), b"x").unwrap();
    let listed = list_backups(dir.path()).unwrap();
    assert_eq!(listed[0].file_name().unwrap(), "new");
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test persist::backup`, FAIL.

- [ ] **Step 3: Implement the module**

`make_backup` captures the timestamp once, calls `OffsetDateTime::now_local()` and passes the result to `format_timestamp`; on `Err(IndeterminateOffset)` pass `None` and set `utc_fallback = true`. Loop the counter while the destination exists.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test persist::backup && cargo clippy --all-targets -- -D warnings
git add src/persist/backup.rs src/persist/mod.rs
git commit -S -m "feat: add timestamped config backups"
```

---

### Task 8: `library`

**Files:**
- Create: `src/library.rs`
- Modify: `src/lib.rs`
- Test: `src/library.rs`, `tests/library.rs`

**Interfaces:**
- Consumes: `TerminalColors`, `read_colors`, `Paths`.
- Produces: `ThemeRef { name: String, path: PathBuf, group: String }` with `label() -> String`; `scan_themes(&Path) -> Vec<ThemeRef>`; `active_theme_path(&Path) -> Option<PathBuf>`; `active_colors(&Paths) -> TerminalColors`; `apply_theme(&Path, &Path) -> Result<()>`; `reload_termux() -> bool` and `#[doc(hidden)] pub fn reload_with_path_env(path_env: Option<&str>) -> bool` (public so integration tests can exercise it without touching the process environment).

- [ ] **Step 1: Write the failing tests**

```rust
const BODY: &str = "background=#000000\n";

fn library(root: &Path) {
    for (dir, name) in [("dark", "Dracula"), ("dark", "Nord"), ("light", "Solarized")] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
        std::fs::write(root.join(dir).join(format!("{name}.properties")), BODY).unwrap();
    }
    std::fs::write(root.join("cyberpunk.properties"), BODY).unwrap();
    std::fs::write(root.join("colors.properties"), BODY).unwrap();
    std::fs::write(root.join("Dracula.properties.backup.20260101"), BODY).unwrap();
    std::fs::write(root.join("old.bak"), BODY).unwrap();
    std::fs::create_dir_all(root.join(".hidden")).unwrap();
    std::fs::write(root.join(".hidden/x.properties"), BODY).unwrap();
}

#[test]
fn scans_skipping_backups_and_the_active_link() {
    let dir = tempfile::tempdir().unwrap();
    library(dir.path());
    let labels: Vec<String> = scan_themes(dir.path()).iter().map(|t| t.label()).collect();
    assert_eq!(labels, vec!["dark/Dracula", "dark/Nord", "light/Solarized", "cyberpunk"]);
}

#[test]
fn missing_root_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert!(scan_themes(&dir.path().join("nope")).is_empty());
}

#[test]
fn active_path_resolves_links() {
    let dir = tempfile::tempdir().unwrap();
    library(dir.path());
    let link = dir.path().join("colors.properties");
    std::os::unix::fs::symlink(dir.path().join("dark/Dracula.properties"), &link).unwrap();
    assert_eq!(active_theme_path(&link), Some(dir.path().join("dark/Dracula.properties")));
    assert_eq!(active_theme_path(&dir.path().join("absent")), None);
}

#[test]
fn apply_creates_a_link_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    library(dir.path());
    let theme = dir.path().join("dark/Dracula.properties");
    let link = dir.path().join("colors.properties");
    std::fs::remove_file(&link).unwrap();
    apply_theme(&theme, &link).unwrap();
    apply_theme(&theme, &link).unwrap();
    assert_eq!(active_theme_path(&link), Some(theme));
}

#[test]
fn apply_to_the_file_itself_does_not_self_link() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("colors.properties");
    std::fs::write(&file, BODY).unwrap();
    apply_theme(&file, &file).unwrap();
    assert!(!file.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn apply_rejects_a_missing_theme() {
    let dir = tempfile::tempdir().unwrap();
    assert!(apply_theme(&dir.path().join("nope.properties"), &dir.path().join("colors.properties")).is_err());
}

#[test]
fn reload_returns_false_without_the_binary() {
    assert!(!reload_with_path_env(Some("/nonexistent")));
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test library`, FAIL.

- [ ] **Step 3: Implement the module**

Walk with `read_dir` recursively, collecting `*.properties`; skip unreadable directories silently. Sort with `(group != "root", group.to_lowercase(), name.to_lowercase())`. `apply_theme` compares `std::fs::canonicalize` of both sides and returns early on a match. `reload_termux` searches `PATH` for `termux-reload-settings`, spawns it, polls `try_wait` every 25 ms up to 15 s, and kills on timeout.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test library && cargo clippy --all-targets -- -D warnings
git add src/library.rs src/lib.rs tests/library.rs
git commit -S -m "feat: add theme library scanning and activation"
```

---

### Task 9: `presets`

**Files:**
- Create: `src/presets.rs`
- Modify: `src/lib.rs`
- Test: `src/presets.rs`

**Interfaces:**
- Consumes: `TerminalColors`, `render_colors`.
- Produces: `built_in() -> Vec<(&'static str, TerminalColors)>` in the order `dracula`, `solarized-dark`, `solarized-light`, `nord`, `monokai`, `gruvbox-dark`; `materialize(&Path) -> Result<Vec<PathBuf>>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn all_six_built_ins_are_present() {
    let names: Vec<&str> = built_in().iter().map(|(n, _)| *n).collect();
    assert_eq!(names, vec!["dracula", "solarized-dark", "solarized-light", "nord", "monokai", "gruvbox-dark"]);
    assert_eq!(built_in()[0].1.background.to_hex(), "#282a36");
}

#[test]
fn materialize_writes_missing_files_only() {
    let dir = tempfile::tempdir().unwrap();
    let written = materialize(dir.path()).unwrap();
    assert_eq!(written.len(), 6);
    let dracula = dir.path().join("dracula.properties");
    assert!(std::fs::read_to_string(&dracula).unwrap().contains("background=#282a36"));

    std::fs::write(&dracula, "background=#111111\n").unwrap();
    materialize(dir.path()).unwrap();
    assert!(std::fs::read_to_string(&dracula).unwrap().contains("#111111"));
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test presets`, FAIL.

- [ ] **Step 3: Implement the built-in table verbatim and `materialize`**

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test presets && cargo clippy --all-targets -- -D warnings
git add src/presets.rs src/lib.rs
git commit -S -m "feat: add built-in fallback themes"
```

---

### Task 10: `theme` math and `Palette`

**Files:**
- Create: `src/theme.rs`
- Modify: `src/lib.rs`
- Test: `src/theme.rs`

**Interfaces:**
- Consumes: `TerminalColors`, `Rgb`, `ensure_contrast` inputs.
- Produces: `relative_luminance(Rgb) -> f64`, `is_dark(Rgb) -> bool`, `contrast_ratio(Rgb, Rgb) -> f64`, `blend(Rgb, Rgb, f64) -> Rgb`, `ensure_contrast(Rgb, Rgb, f64) -> Rgb`; `#[derive(Clone, Debug, PartialEq)] Palette { background, foreground, primary, secondary, accent, error, success, warning, surface, panel, boost, muted: Color, dark: bool }` and `Palette::from_colors(&TerminalColors) -> Palette`; `to_rgb(Color) -> Rgb`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn luminance_extremes() {
    assert_eq!(relative_luminance(Rgb::new(0, 0, 0)), 0.0);
    assert!((relative_luminance(Rgb::new(255, 255, 255)) - 1.0).abs() < 1e-9);
    assert!(is_dark(Rgb::new(16, 16, 16)));
    assert!(!is_dark(Rgb::new(240, 240, 240)));
}

#[test]
fn contrast_ratio_bounds() {
    assert!((contrast_ratio(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)) - 21.0).abs() < 1e-9);
    assert!((contrast_ratio(Rgb::new(0x12, 0x34, 0x56), Rgb::new(0x12, 0x34, 0x56)) - 1.0).abs() < 1e-9);
}

#[test]
fn ensure_contrast_pushes_apart() {
    let bg = Rgb::new(0, 0, 0);
    let fixed = ensure_contrast(Rgb::new(13, 13, 13), bg, 3.0);
    assert!(contrast_ratio(fixed, bg) >= 3.0);
}

#[test]
fn blend_hits_both_ends() {
    let a = Rgb::new(0, 0, 0);
    let b = Rgb::new(255, 255, 255);
    assert_eq!(blend(a, b, 0.0), a);
    assert_eq!(blend(a, b, 1.0), b);
}

#[test]
fn palette_derives_from_colours() {
    let colors = TerminalColors::default();
    let p = Palette::from_colors(&colors);
    assert!(p.dark);
    assert!(contrast_ratio(to_rgb(p.foreground), colors.background) >= 4.5);
    assert_ne!(p.panel, p.background);
    assert_ne!(p.muted, p.foreground);
}
```

Add `to_rgb(Color) -> Rgb` in `theme.rs` so the last test can compare. `Palette` derives `Clone, Debug, PartialEq`; `Color` already implements `PartialEq`.

- [ ] **Step 2: Run and confirm failure** — `cargo test theme`, FAIL.

- [ ] **Step 3: Implement the math and palette construction**

Port `_channel_luminance`, `relative_luminance`, `contrast_ratio`, `blend`, `ensure_contrast` exactly, including the 20-step nudge. `Palette::from_colors` maps `Rgb` to `Color::Rgb` and derives `muted = blend(foreground, background, 0.35)`.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test theme && cargo clippy --all-targets -- -D warnings
git add src/theme.rs src/lib.rs
git commit -S -m "feat: add palette derivation and contrast maths"
```

---

### Task 11: Fixture round-trip tests

**Files:**
- Create: `tests/fixtures/colors_with_extras.properties`, `tests/fixtures/colors_with_extras.expected`, `tests/fixtures/termux.properties`, `tests/fixtures/termux.expected`, `tests/roundtrip.rs`

**Interfaces:**
- Consumes: `read_colors`, `write_colors`, `update_colors_text`, `read_app_theme`, `write_app_theme`.

- [ ] **Step 1: Create the fixtures from the Python suite**

`colors_with_extras.properties` is CRLF, has no trailing newline, a header comment, managed keys with mixed casing, extras, and a commented key:

```
# SilkCircuit Vibrant\r\n# Do not edit by hand.\r\n\r\nbackground=#0F0C1A\r\nforeground=#f8f8f2\r\ncursor=#00ffcc\r\n\r\ncolor0=#0f0c1a\r\ncolor1=#ff3366\r\nselection-background=#333333\r\n# background=#111111
```

`colors_with_extras.expected` is what `read_colors` plus `write_colors(preserve = true)` must produce: the header comments, blank line and `selection-background` keep their positions and bytes; `background`, `foreground`, `cursor`, `color0` and `color1` are rewritten lowercase; the commented `# background=#111111` line is untouched (an active `background` line exists, so the comment is not the match); the missing `color2`..`color15` are appended in index order at the end; the file stays CRLF and still has no trailing newline.

`termux.properties` contains the real `extra-keys` one-liner, a commented `# fullscreen = true`, `terminal-transcript-rows = 50000`, `use-black-ui = true`, and `background=background.jpg`. `termux.expected` equals it byte-for-byte after a no-op write.

- [ ] **Step 2: Write the failing tests**

```rust
#[test]
fn colors_round_trip_matches_the_expected_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("colors.properties");
    std::fs::copy("tests/fixtures/colors_with_extras.properties", &path).unwrap();
    let colors = read_colors(&path).unwrap();
    write_colors(&path, &colors, true).unwrap();
    let actual = std::fs::read(&path).unwrap();
    let expected = std::fs::read("tests/fixtures/colors_with_extras.expected").unwrap();
    assert_eq!(String::from_utf8_lossy(&actual), String::from_utf8_lossy(&expected));
}

#[test]
fn termux_properties_round_trip_is_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("termux.properties");
    std::fs::copy("tests/fixtures/termux.properties", &path).unwrap();
    let theme = read_app_theme(&path).unwrap();
    write_app_theme(&path, &theme, true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), std::fs::read("tests/fixtures/termux.expected").unwrap());
}

#[test]
fn extra_keys_survives_a_setting_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("termux.properties");
    std::fs::copy("tests/fixtures/termux.properties", &path).unwrap();
    let mut theme = read_app_theme(&path).unwrap();
    theme.set("use-black-ui", "false");
    write_app_theme(&path, &theme, true).unwrap();
    let out = std::fs::read_to_string(&path).unwrap();
    let original = std::fs::read_to_string("tests/fixtures/termux.properties").unwrap();
    let extra = original.lines().find(|l| l.starts_with("extra-keys")).unwrap();
    assert!(out.lines().any(|l| l == extra));
    assert!(out.contains("use-black-ui = false"));
}
```

- [ ] **Step 3: Run and confirm failure** — `cargo test --test roundtrip`, FAIL until fixtures exist.

- [ ] **Step 4: Materialise the fixtures and make the tests pass**

If the output diverges, decide whether the implementation or the expected file is wrong; the expected file encodes the spec, so fix the implementation unless the spec is ambiguous — then stop and report.

- [ ] **Step 5: Commit**

```bash
cargo test --test roundtrip
git add tests/fixtures tests/roundtrip.rs
git commit -S -m "test: pin properties round-trip behaviour with fixtures"
```

---

### Task 12: `ui::text` primitives

**Files:**
- Create: `src/ui/mod.rs`, `src/ui/text.rs`
- Modify: `src/lib.rs`
- Test: `src/ui/text.rs`

**Interfaces:**
- Consumes: `TerminalColors`, `Rgb`, `ensure_contrast`, `relative_luminance`.
- Produces: `banner(&TerminalColors, width: u16, stacked: bool, compact: bool) -> Vec<Line<'static>>`; `palette_strip(&TerminalColors) -> Line<'static>`; `swatch_grid(&TerminalColors) -> Vec<Line<'static>>`; `mock_session(&TerminalColors) -> Vec<Line<'static>>`; `swatch(Rgb, width: usize) -> Span<'static>`; `contrast_color(Rgb) -> Rgb`.

- [ ] **Step 1: Write the failing tests**

```rust
fn all_text(lines: &[Line<'static>]) -> String {
    lines.iter().map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>()).collect::<Vec<_>>().join("\n")
}

#[test]
fn banner_has_five_rows_unstacked_and_ten_stacked() {
    let c = TerminalColors::default();
    assert_eq!(banner(&c, 100, false, false).len(), 5);
    assert_eq!(banner(&c, 60, true, false).len(), 10);
    assert_eq!(banner(&c, 40, false, true).len(), 5);
    assert!(all_text(&banner(&c, 100, false, false)).contains("█"));
}

#[test]
fn strip_has_sixteen_coloured_blocks() {
    let line = palette_strip(&TerminalColors::default());
    assert_eq!(line.spans.len(), 16);
    assert_eq!(line.spans[0].style.bg, Some(Color::Rgb(0, 0, 0)));
}

#[test]
fn grid_has_two_chip_rows_and_three_samples() {
    let lines = swatch_grid(&TerminalColors::default());
    assert_eq!(lines.len(), 5);
    assert!(all_text(&lines).contains("15"));
    assert!(all_text(&lines).contains("background"));
}

#[test]
fn mock_session_has_seven_lines_and_a_prompt() {
    let lines = mock_session(&TerminalColors::default());
    assert_eq!(lines.len(), 7);
    assert!(all_text(&lines).starts_with("user@termux"));
}

#[test]
fn contrast_color_flips_at_the_threshold() {
    assert_eq!(contrast_color(Rgb::new(255, 255, 255)), Rgb::new(0, 0, 0));
    assert_eq!(contrast_color(Rgb::new(0, 0, 0)), Rgb::new(255, 255, 255));
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test ui::text`, FAIL.

- [ ] **Step 3: Implement the primitives**

Port `_GLYPHS`, `_word`, `banner_text`, `palette_strip`, `colors_preview_text`, `swatch_text`, `mock_lines` and `contrast_color`. Return owned `Line`/`Span` values; apply `ensure_contrast` per word letter as today.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test ui::text && cargo clippy --all-targets -- -D warnings
git add src/ui src/lib.rs
git commit -S -m "feat: add banner, swatch and mock-session text primitives"
```

---

### Task 13: Terminal lifecycle, `App` shell, main menu

**Files:**
- Create: `src/terminal.rs`, `src/app.rs`, `src/ui/input.rs`, `src/ui/focus.rs`, `src/screens/mod.rs`, `src/screens/main_menu.rs`, `tests/support/mod.rs`, `tests/ui_menu.rs`
- Modify: `src/lib.rs`, `src/main.rs`

**Interfaces:**
- Consumes: everything from Tasks 1-12.
- Produces: `Field { input: tui_input::Input }` with `new(&str)`, `value() -> &str`, `set_value(&str)`, `handle_key(KeyEvent) -> bool`, `display_offset() -> usize`; `Move { Up, Down, Home, End, PageUp, PageDown }` and `list_move(&mut ListState, len: usize, m: Move, page: usize)`; `ScreenKind { Main, Presets, Terminal, AppTheme }`; `Screen { Main(MainMenu) }` (later tasks add variants); `Screen::{handle_key(&mut self, KeyEvent, &Paths) -> ScreenAction, render(&mut self, &mut Frame, Rect, &Palette)}`; `ScreenAction { None, Quit, Push(ScreenKind), Pop, RefreshPalette(TerminalColors), Status(String) }`; `App::{new(Paths), handle_key(KeyEvent), render(&mut Frame), tick(Instant), current() -> &Screen, dialog() -> Option<&Dialog>, palette() -> &Palette, should_quit() -> bool}`.

`Command` is introduced in Task 18, where the first screen needs deterministic save/revert entry points. Defining it earlier would create `-D warnings` dead-code failures.
- Test support: `tests/support/mod.rs` exposing `key(KeyCode) -> KeyEvent` and `render_text(&mut App, w, h) -> String` (draws into a `TestBackend` and returns the buffer as text). Later tasks append helpers to the same file; every helper listed in a task is used by that task's tests, so `-D warnings` stays clean.

- [ ] **Step 1: Write the failing test**

```rust
mod support;
use support::{key, render_text};
use termux_colors::app::App;
use termux_colors::paths::Paths;
use crossterm::event::KeyCode;

#[test]
fn menu_renders_its_four_items_and_active_theme() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    let rendered = render_text(&mut app, 100, 30);
    assert!(rendered.contains("Preset Themes"));
    assert!(rendered.contains("active theme"));
}

#[test]
fn arrows_move_the_selection_and_quit_item_exits() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    for _ in 0..3 { app.handle_key(key(KeyCode::Down)); }
    app.handle_key(key(KeyCode::Enter));
    assert!(app.should_quit());
}

#[test]
fn tab_cycles_focus_and_q_quits() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    let before = render_text(&mut app, 100, 30);
    app.handle_key(key(KeyCode::Tab));
    assert_ne!(before, render_text(&mut app, 100, 30));
    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.should_quit());
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test --test ui_menu`, FAIL.

- [ ] **Step 3: Implement the shell**

`main.rs` installs the panic hook, enables raw mode plus the alternate screen, runs `App`, and restores on both paths. `App::new` sets `palette = Palette::from_colors(&active_colors(&paths))`. `App::handle_key` short-circuits `Ctrl+C`, then delegates to the top screen. `MainMenu::handle_key` implements `Up`/`Down`/`j`/`k`, `Tab`, `Enter`, `q`, `Esc`. Render with `Layout` + `Paragraph`, drawing the banner, tagline, strip, status and a bordered menu box.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_menu && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: add app shell, terminal lifecycle and main menu"
```

---

### Task 14: Dialogs and routing

**Files:**
- Create: `src/ui/dialog.rs`, `tests/ui_dialog.rs`
- Modify: `src/app.rs`, `src/ui/mod.rs`, `src/screens/mod.rs`

**Interfaces:**
- Modifies: `ScreenAction` gains `OpenDialog(Dialog)`; `Screen` gains `on_dialog(&mut self, DialogResult, &Paths) -> ScreenAction`.
- Produces: `Dialog::Confirm { title, message, confirm_label, cancel_label, hint, focus_confirm }`, `Dialog::Message { title, message }`, `Dialog::Prompt { title, message, field: Field, placeholder }` with constructors `confirm(message, confirm_label, cancel_label)`, `message(message, title)`, `prompt(message, initial)`; `DialogResult { Confirmed(bool), Acknowledged, Text(Option<String>) }`; `Dialog::handle_key(&mut self, KeyEvent) -> Option<DialogResult>`; `Dialog::render(&self, Rect, &mut Buffer, &Palette)`; `App::open_dialog(Dialog)`, `App::resolve_dialog(DialogResult)`.

Screens own their multi-step state (`pending` fields); `App` only carries the dialog and delivers one result at a time. There is no app-level `Pending` type.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn confirm_enter_is_true_and_esc_is_false() {
    let mut d = Dialog::confirm("Back up?", "Back up", "Skip");
    assert_eq!(d.handle_key(key(KeyCode::Enter)), Some(DialogResult::Confirmed(true)));
    let mut d = Dialog::confirm("Back up?", "Back up", "Skip");
    assert_eq!(d.handle_key(key(KeyCode::Esc)), Some(DialogResult::Confirmed(false)));
}

#[test]
fn confirm_tab_moves_to_cancel() {
    let mut d = Dialog::confirm("x", "Yes", "No");
    d.handle_key(key(KeyCode::Tab));
    assert_eq!(d.handle_key(key(KeyCode::Enter)), Some(DialogResult::Confirmed(false)));
}

#[test]
fn prompt_returns_trimmed_text_or_none() {
    let mut d = Dialog::prompt("Name", "custom");
    d.handle_key(key(KeyCode::Backspace));
    for ch in "moon".chars() { d.handle_key(key(KeyCode::Char(ch))); }
    d.handle_key(key(KeyCode::Enter));
    let mut d = Dialog::prompt("Name", "");
    assert_eq!(d.handle_key(key(KeyCode::Enter)), Some(DialogResult::Text(None)));
}

#[test]
fn app_draws_and_closes_dialogs() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.open_dialog(Dialog::message("Something failed.", "Notice"));
    assert!(render_text(&mut app, 100, 30).contains("Something failed."));
    app.handle_key(key(KeyCode::Esc));
    assert!(app.dialog().is_none());
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test --test ui_dialog`, FAIL.

- [ ] **Step 3: Implement the dialog widget and the app slot**

Render as a centred rect of `min(64, area.width * 9 / 10)` width, cleared to `panel`, bordered `primary`, title in the border, buttons right-aligned. `Esc` yields cancel/`None`; `Tab`/`BackTab` move button focus; the prompt delegates typing to `Field`.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_dialog && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: add modal dialogs and result routing"
```

---

### Task 15: Preset picker — data, list, filter

**Files:**
- Create: `src/screens/preset_picker.rs`, `tests/ui_picker.rs`
- Modify: `src/screens/mod.rs`, `src/app.rs`

**Interfaces:**
- Produces: `PresetPicker::new(&Paths) -> PresetPicker`; fields `all: Vec<ThemeRef>`, `visible: Vec<usize>`, `filter: Field`, `list: ListState`, `cache: HashMap<PathBuf, TerminalColors>`, `active: Option<PathBuf>`; `load_themes(&Paths) -> Vec<ThemeRef>` (seeds built-ins when empty); `apply_filter(&mut self, needle: &str)`; `selected(&self) -> Option<&ThemeRef>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn empty_library_is_seeded_with_builtins() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let themes = load_themes(&paths);
    assert_eq!(themes.len(), 6);
    assert!(dir.path().join("colors/builtin/dracula.properties").is_file());
}

#[test]
fn filter_is_case_insensitive_and_keeps_a_visible_selection() {
    let mut picker = picker_with(&["Dracula", "Nord", "Solarized"]);
    picker.apply_filter("dra");
    assert_eq!(picker.visible.len(), 1);
    assert_eq!(picker.selected().unwrap().name, "Dracula");
    picker.apply_filter("");
    assert_eq!(picker.visible.len(), 3);
}

#[test]
fn movement_clamps_and_pages() {
    let mut picker = picker_with(&["a", "b", "c", "d"]);
    list_move(&mut picker.list, 4, Move::Down, 2);
    assert_eq!(picker.list.selected(), Some(1));
    for _ in 0..10 { list_move(&mut picker.list, 4, Move::Down, 2); }
    assert_eq!(picker.list.selected(), Some(3));
    list_move(&mut picker.list, 4, Move::Home, 2);
    assert_eq!(picker.list.selected(), Some(0));
    list_move(&mut picker.list, 4, Move::PageDown, 2);
    assert_eq!(picker.list.selected(), Some(2));
}

#[test]
fn picker_renders_labels_and_the_count() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    open_picker(&mut app);                 // support helper: Down, Down, Enter on the menu
    let text = render_text(&mut app, 100, 30);
    assert!(text.contains("themes (6)"));
    assert!(text.contains("dracula"));
}
```

Add the helpers this task needs to `tests/support/mod.rs`: `open_picker(&mut App)` (two `Down` presses and `Enter` from the menu) and `picker_with(names: &[&str]) -> PresetPicker` (builds a picker over a tempdir library).

- [ ] **Step 2: Run and confirm failure** — `cargo test --test ui_picker`, FAIL.

- [ ] **Step 3: Implement the picker's data layer and left pane**

`load_themes` scans, and when the result is empty calls `materialize(paths.colors_root().join("builtin"))` and rescans. `apply_filter` rebuilds `visible` by case-insensitive `label().contains`, resetting the selection to 0 only when the current selection is filtered out. Render the left pane with `List` (bordered title `themes (N)`) and the filter `Field` above it.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_picker && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: add preset picker list, filter and navigation"
```

---

### Task 16: Preset picker — preview, live palette, apply

**Files:**
- Modify: `src/screens/preset_picker.rs`, `src/app.rs`
- Test: `tests/ui_picker.rs`

**Interfaces:**
- Consumes: `read_colors`, `cache`, `ScreenAction::RefreshPalette`, `ScreenAction::Status`, `apply_theme`, `reload_termux`.
- Produces: `PresetPicker::colors_for(&mut self, index: usize) -> TerminalColors` (cached read, defaults on error); `PresetPicker::apply(&mut self, &Paths) -> ScreenAction`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn moving_the_selection_retints_the_ui() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    open_picker(&mut app);
    let before = app.palette().clone();
    app.handle_key(key(KeyCode::Down));
    assert_ne!(before, *app.palette());
    assert!(app.palette().background != Color::Rgb(0, 0, 0));
}

#[test]
fn enter_links_colors_properties_to_the_theme() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let mut app = App::new(paths.clone());
    open_picker(&mut app);
    app.handle_key(key(KeyCode::Enter));
    let link = paths.colors_properties();
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert!(render_text(&mut app, 100, 30).contains("Saved"));
}

#[test]
fn apply_failure_reports_and_keeps_the_old_link() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let mut app = App::new(paths.clone());
    open_picker(&mut app);
    app.handle_key(key(KeyCode::Enter));            // first apply succeeds
    let applied = std::fs::canonicalize(paths.colors_properties()).unwrap();

    let theme = picker_theme_path(&mut app);
    std::fs::remove_file(theme).unwrap();
    app.handle_key(key(KeyCode::Enter));            // second apply fails

    assert!(render_text(&mut app, 100, 30).contains("not found"));
    assert_eq!(std::fs::canonicalize(paths.colors_properties()).unwrap(), applied);
}
```

- [ ] **Step 2: Run and confirm failure** — new tests FAIL.

- [ ] **Step 3: Implement the preview pane, palette refresh and apply**

On selection change emit `RefreshPalette(colors_for(index))`. Apply calls `apply_theme` and `reload_termux`; on success set the active path, status `Saved <label>.` plus the reload warning when reload fails; on failure status `theme not found: <path>` and leave the existing link. A theme that exists but cannot be parsed falls back to default colours for the preview rather than failing the apply.

Extend `tests/support/mod.rs` with `picker_theme_path(&mut App) -> PathBuf` (the selected theme's path).

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_picker && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: add live preview and theme activation to the picker"
```

---

### Task 17: Terminal editor — form, validation, preview

**Files:**
- Create: `src/screens/terminal_editor.rs`, `src/ui/form.rs`, `tests/ui_editor.rs`
- Modify: `src/screens/mod.rs`, `src/app.rs`

**Interfaces:**
- Produces: `TerminalEditor::new(&Paths) -> TerminalEditor` with `fields: [Field; 19]`, `valid: [bool; 19]`, `focus: usize`, `scroll: usize`, `target: Option<PathBuf>`, `base: TerminalColors`; `TerminalEditor::{handle_key, command, render, revalidate, current_colors, collect}`; `current_colors(&self) -> TerminalColors` ignores invalid fields; `revalidate(&mut self)` refreshes `valid` and moves `scroll` to keep `focus` visible; `collect(&self) -> std::result::Result<TerminalColors, Vec<String>>` returns the invalid keys; `ui::form::field_row(...)` for label + swatch + input rendering.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn fields_initialise_from_the_active_theme() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    std::fs::create_dir_all(paths.colors_root().join("dark")).unwrap();
    std::fs::write(paths.colors_root().join("dark/T.properties"), "background=#112233\n").unwrap();
    std::os::unix::fs::symlink(paths.colors_root().join("dark/T.properties"), paths.colors_properties()).unwrap();
    let mut app = App::new(paths.clone());
    open_editor(&mut app);
    // assert through the rendered buffer
    assert!(render_text(&mut app, 100, 30).contains("#112233"));
}

#[test]
fn invalid_hex_is_flagged_and_ignored_by_the_preview() {
    let mut editor = editor_with("background=#000000\n");
    editor.fields[0].set_value("#zzz");
    editor.revalidate();
    assert!(!editor.valid[0]);
    assert_eq!(editor.current_colors().background.to_hex(), "#000000");
}

#[test]
fn typing_a_valid_hex_emits_a_palette_refresh() {
    let mut app = editor_app();
    focus_background_cleared(&mut app);
    for ch in "#112233".chars() { app.handle_key(key(KeyCode::Char(ch))); }
    assert_eq!(app.palette().background, Color::Rgb(0x11, 0x22, 0x33));
}

#[test]
fn tab_and_arrows_move_focus_and_scroll_keeps_it_visible() {
    let mut editor = editor_with("");
    assert_eq!(editor.focus, 0);
    editor.handle_key(key(KeyCode::Tab));
    assert_eq!(editor.focus, 1);
    for _ in 0..18 { editor.handle_key(key(KeyCode::Tab)); }
    assert!(editor.scroll > 0);
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test --test ui_editor`, FAIL.

- [ ] **Step 3: Implement the editor form**

Render two panes (52%/48%), section rules, per-field label + 3-cell swatch + `Field`. Delegation: `Tab`/`BackTab` always move focus; other keys go to the focused `Field`; valid text emits `RefreshPalette(current_colors())`. Track `scroll` so `focus` is inside the visible rows.

Extend `tests/support/mod.rs` with `editor_app() -> App` (editor open over an empty tempdir), `editor_with(contents: &str) -> TerminalEditor`, `open_editor(&mut App)` and `focus_background_cleared(&mut App)`, which focuses field 0 and blanks it so typed characters start from empty.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_editor && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: add terminal colour editor form and live preview"
```

---

### Task 18: Terminal editor — save, save-as, revert

**Files:**
- Modify: `src/screens/terminal_editor.rs`, `src/screens/mod.rs`, `src/app.rs`
- Test: `tests/ui_editor_save.rs`

**Interfaces:**
- Produces: `Command { Save, SaveAs, Revert }`; `App::command(Command)`; `Screen::command(&mut self, Command, &Paths) -> ScreenAction` (screens that do not handle it return `ScreenAction::None`); `TerminalPending { WriteAfterBackup { target: PathBuf, colors: TerminalColors }, NameForSaveAs { colors: TerminalColors }, Overwrite { target: PathBuf, colors: TerminalColors }, BackupThenWriteAs { target: PathBuf, colors: TerminalColors } }` held in `TerminalEditor::pending: Option<TerminalPending>`; `TerminalEditor::{start_save, start_save_as, start_revert, on_dialog, command}`; `sanitize_name(&str) -> String`.

- [ ] **Step 1: Write the failing tests**

```rust
use termux_colors::app::Command;

#[test]
fn sanitize_replaces_unsafe_runs() {
    assert_eq!(sanitize_name("my theme"), "my-theme");
    assert_eq!(sanitize_name("../../etc/passwd"), "etc-passwd");
    assert_eq!(sanitize_name("   "), "custom");
    assert_eq!(sanitize_name("ok_Name.1"), "ok_Name.1");
}

#[test]
fn save_backs_up_then_writes_through_the_link() {
    let (mut app, paths) = editor_with_active("#000000");
    set_field(&mut app, 0, "112233");
    app.command(Command::Save);
    assert!(app.dialog().is_some());
    app.resolve_dialog(DialogResult::Confirmed(true));
    assert_eq!(paths.backups().read_dir().unwrap().count(), 1);
    assert!(std::fs::read_to_string(paths.colors_root().join("dark/T.properties"))
        .unwrap().contains("background=#112233"));
}

#[test]
fn save_without_an_active_theme_asks_for_a_name() {
    let (mut app, paths) = editor_with_colors(None);
    app.command(Command::Save);
    assert!(matches!(app.dialog(), Some(Dialog::Prompt { .. })));
    app.resolve_dialog(DialogResult::Text(Some("neon theme".into())));
    assert!(paths.colors_root().join("custom/neon-theme.properties").is_file());
}

#[test]
fn declining_overwrite_leaves_the_file_alone() {
    let (mut app, paths) = editor_with_active("#000000");
    std::fs::write(paths.colors_root().join("custom/exists.properties"), "background=#111111\n").unwrap();
    save_as(&mut app, "exists");
    app.resolve_dialog(DialogResult::Confirmed(false));
    assert_eq!(std::fs::read_to_string(paths.colors_root().join("custom/exists.properties")).unwrap(),
               "background=#111111\n");
}

#[test]
fn invalid_value_opens_a_message_naming_the_keys() {
    let (mut app, _paths) = editor_with_active("#000000");
    set_field(&mut app, 0, "#zzz");
    app.command(Command::Save);
    assert!(render_text(&mut app, 100, 30).contains("Invalid hex colour for: background"));
}

#[test]
fn revert_restores_the_saved_values() {
    let (mut app, _paths) = editor_with_active("#000000");
    set_field(&mut app, 0, "#111111");
    app.command(Command::Revert);
    assert!(render_text(&mut app, 100, 30).contains("#000000"));
}
```

Extend `tests/support/mod.rs` with `editor_with_active(background: &str) -> (App, Paths)` (builds a `dark/T.properties` theme, links it, opens the editor), `editor_with_colors(Option<&str>)`, `set_field(&mut App, index: usize, value: &str)` and `save_as(&mut App, name: &str)` (runs Save As and answers the prompt).

- [ ] **Step 2: Run and confirm failure** — `cargo test --test ui_editor_save`, FAIL.

- [ ] **Step 3: Implement the flows from spec §7.8**

`collect` returns `Err(vec![keys])` when any field is non-empty and invalid. `start_save` → `Message` on error; `Prompt` when `target` is `None`; otherwise `Confirm` for backup when `colors_properties` exists. `on_dialog` advances one step per result and emits `Status`/`RefreshPalette` as appropriate. Backup failures are surfaced as a `Message` and abort the write.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_editor_save && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: add save, save-as and revert flows to the terminal editor"
```

---

### Task 19: App theme editor

**Files:**
- Create: `src/screens/app_theme_editor.rs`, `tests/ui_app_theme.rs`
- Modify: `src/screens/mod.rs`, `src/app.rs`

**Interfaces:**
- Produces: `AppThemeEditor::new(&Paths) -> AppThemeEditor` with `rows: [Row; 9]` where `Row { spec: &'static FieldSpec, value: Option<String>, field: Field, invalid: bool }`; `ThemePending { BackupThenWrite { values: AppTheme } }` held in `AppThemeEditor::pending: Option<ThemePending>`; `AppThemeEditor::{start_save, start_revert, on_dialog, collect}`.

- [ ] **Step 1: Write the failing tests**

```rust
use termux_colors::app::Command;

#[test]
fn rows_initialise_from_the_file_in_schema_order() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    std::fs::write(paths.termux_properties(), "terminal-cursor-style = bar\nuse-black-ui = true\n").unwrap();
    let app = App::new(paths);
    open_app_theme(&app);
    let text = render_text(&mut app, 100, 34);
    assert!(text.contains("Cursor style"));
    assert!(text.contains("bar"));
    assert!(text.contains("Black UI"));
}

#[test]
fn invalid_number_blocks_the_save_with_a_message() {
    let (mut app, _paths) = app_theme_with("terminal-margin-horizontal = 5\n");
    set_row(&mut app, "terminal-margin-horizontal", "999");
    app.command(Command::Save);
    assert!(render_text(&mut app, 100, 34).contains("Margin horizontal must be <= 50"));
}

#[test]
fn save_backs_up_and_preserves_unmanaged_lines() {
    let (mut app, paths) = app_theme_with("use-black-ui = true\nextra-keys = [['ESC','-']]\n");
    set_row(&mut app, "use-black-ui", "false");
    app.command(Command::Save);
    app.resolve_dialog(DialogResult::Confirmed(true));
    let out = std::fs::read_to_string(paths.termux_properties()).unwrap();
    assert!(out.contains("use-black-ui = false"));
    assert!(out.contains("extra-keys = [['ESC','-']]"));
}

#[test]
fn selectors_cycle_through_choices_and_unset() {
    let (mut app, _paths) = app_theme_with("");
    let row = row_index(&mut app, "bell-character");
    focus_row(&mut app, row);
    app.handle_key(key(KeyCode::Right));            // beep
    app.handle_key(key(KeyCode::Right));            // vibrate
    app.handle_key(key(KeyCode::Right));            // ignore
    app.handle_key(key(KeyCode::Right));            // unset
    assert!(render_text(&mut app, 100, 34).contains("bell-character"));
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test --test ui_app_theme`, FAIL.

- [ ] **Step 3: Implement the screen**

`Bool`/`Choice` rows cycle `None → choices… → None`; text/int rows delegate to `Field`. `collect` validates every non-empty row and returns `Err((key, message))` for the first failure (the dialog names that field). Save offers the backup confirm when the file exists, then writes with preservation and reloads.

Extend `tests/support/mod.rs` with `app_theme_with(&str) -> (App, Paths)`, `set_row(&mut App, key: &str, value: &str)`, `row_index(&mut App, key: &str) -> usize` and `focus_row(&mut App, index: usize)`.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_app_theme && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: add app theme editor with validation and backup flow"
```

---

### Task 20: Small-terminal guard and status expiry

**Files:**
- Modify: `src/app.rs`, `src/ui/mod.rs`
- Test: `tests/ui_small.rs`

**Interfaces:**
- Produces: `App::tick(&mut self, now: Instant)`, `App::set_status(&mut self, &str)`; `MIN_WIDTH: u16 = 40`, `MIN_HEIGHT: u16 = 10`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn tiny_terminals_get_a_guard_not_a_panic() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    let text = render_text(&mut app, 20, 5);
    assert!(text.contains("too small"));
    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.should_quit());
}

#[test]
fn status_expires_after_four_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.set_status("Saved thing.");
    assert!(render_text(&mut app, 100, 30).contains("Saved thing."));
    app.tick(Instant::now() + Duration::from_secs(5));
    assert!(!render_text(&mut app, 100, 30).contains("Saved thing."));
}

#[test]
fn a_resize_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    for (w, h) in [(10, 4), (40, 10), (120, 40), (39, 9)] {
        let _ = render_text(&mut app, w, h);
    }
}
```

- [ ] **Step 2: Run and confirm failure** — `cargo test --test ui_small`, FAIL.

- [ ] **Step 3: Implement the guard and expiry**

In `App::render`, short-circuit to a centred "terminal too small (40×10 minimum)" paragraph when the area is smaller. Handle `q`, `Esc` and `Ctrl+C` in that state. `tick` clears a status older than 4 s.

- [ ] **Step 4: Run, lint, commit**

```bash
cargo test --test ui_small && cargo clippy --all-targets -- -D warnings
git add src tests
git commit -S -m "feat: guard small terminals and expire status messages"
```

---

### Task 21: README and CI

**Files:**
- Modify: `README.md`, `.gitignore`
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`

- [ ] **Step 1: Rewrite `README.md`**

Keep the existing structure and tone. Replace the install section with: download the release binary (aarch64 or armv7) to `$PREFIX/bin` and `chmod +x`, or `pkg install rust && cargo build --release`. Add an upgrade note (`pip uninstall termux-colors` first), the truecolor requirement, the key map (including `j`/`k`, `PageUp`/`PageDown`, `Home`/`End`), and a statement that the `colors.sh`/library interoperability and the backup behaviour are unchanged.

- [ ] **Step 2: Add the workflows**

`ci.yml`: on `push` and `pull_request`, Ubuntu, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
`release.yml`: on `v*` tags, install the Android NDK via `nttld/setup-ndk` or the preinstalled `ANDROID_NDK_HOME`, `cargo install cargo-ndk`, build `-t arm64-v8a -t armeabi-v7a -p 24`, strip, `sha256sum`, upload with `softprops/action-gh-release`.

- [ ] **Step 3: Verify**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
Expected: all three exit 0. For the workflow YAML, run
`python3 -c "import yaml,sys; [yaml.safe_load(open(p)) for p in sys.argv[1:]]" .github/workflows/*.yml`
if PyYAML happens to be installed; otherwise review both files by eye. Do not add a dependency just to parse them.

- [ ] **Step 4: Commit**

```bash
git add README.md .gitignore .github/workflows
git commit -S -m "docs: document the Rust build and add CI workflows"
```

---

### Task 22: Remove the Python implementation

**Files:**
- Delete: `src/termux_colors/`, `src/termux_colors.egg-info/`, `tests/test_*.py`, `pyproject.toml`, `build/`, `dist/`, `.pytest_cache/`
- Modify: `.gitignore` (drop Python entries, keep `/target`)

- [ ] **Step 1: Confirm the removal criteria from spec §2**

Run and record the output of each:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
./target/release/termux-colors --help 2>/dev/null; echo "exit: $?"
```

The binary must start and exit cleanly on this Termux device. If it cannot be run here, note that and stop before deleting anything.

- [ ] **Step 2: Delete the Python tree and artifacts**

```bash
git rm -r src/termux_colors src/termux_colors.egg-info tests/test_app_smoke.py tests/test_backup.py tests/test_colors_io.py tests/test_library.py tests/test_theme.py tests/test_theme_io.py pyproject.toml
rm -rf build dist .pytest_cache src/termux_colors.egg-info
```

Do not commit any uncommitted `pyproject.toml` edits made by the user; check `git status` before `git rm` and report what is being discarded.

- [ ] **Step 3: Verify nothing Python remains**

```bash
git ls-files | grep -E '\.py$|pyproject|pytest' || echo "clean"
cargo test
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -S -m "chore: remove the Python implementation

Parity is covered by the Rust test suite: properties round-trip fixtures,
colour and app-theme IO, library scanning and activation, palette maths,
and UI flows for all four screens."
```

---

## Self-Review Notes

- **Spec coverage:** §6.1-6.9 → Tasks 1-10; §6.3 fixtures → Task 11; §7.10 rendering → Task 12; §7.1-7.4 → Task 13; §7.8 dialogs → Tasks 14, 18, 19; §7.5 → Tasks 15-16; §7.6 → Tasks 17-18; §7.7 → Task 19; §7.9 → Task 20; §9 → every task's tests plus Task 11; §10 → Task 21; §11 → the task order; §2 criterion 6 → Task 22.
- **Ordering:** the picker/editor tasks depend on `Screen`/`ScreenAction` from Task 13 and dialogs from Task 14; no task references a later task's types.
- **Test helpers** (`key`, `render_text`, `picker_with`, `editor_with`) live in `tests/support/mod.rs` and are extended as screens arrive; each extension is part of the task that needs it.
