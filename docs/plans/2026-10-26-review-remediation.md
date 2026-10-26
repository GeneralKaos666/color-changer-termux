# Review Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every finding from the 2026-10-26 review — make the loop non-blocking, every view pure, the Fonts tab behave as documented, all writes safe, and the suite green under `clippy -D warnings`.

**Architecture:** A `poll(TICK)` loop drains one `mpsc` channel into a single `App::update(Msg)` reducer; long jobs (reload, prompt capture, font scan, download) run on detached one-shot threads and post messages back. Views become `render(&self, frame, area, &Ui)`; geometry and caches move to `on_resize`/`opened`. `FontSlot` and `Glyphs` replace `&str` slots and the global ASCII flag; all file writes go through atomic temp+rename.

**Tech Stack:** Rust edition 2024, rust-version 1.85; ratatui 0.29, crossterm 0.28, tui-input 0.11, ttf-parser 0.25, ureq 2.10, time 0.3. No new dependencies.

**Spec:** `docs/specs/2026-10-26-review-remediation-design.md`

## Global Constraints

- No `unsafe`, no `static mut`, no `process::exit` below `main`'s `run()`.
- No `fs::`/`Command::`/`read_colors` call inside any `render` body — views are pure.
- `Palette` becomes `Copy`; `Ui { palette: Palette, glyphs: Glyphs }` is `Copy` and is the single render context.
- Every bordered block uses `ui::pane(ui, title)` so `--ascii` reaches all borders and the help bar.
- `q` quits from anywhere (restoring any uncommitted font preview first); `Esc` restores+pops on sub-screens, quits at the menu.
- `FontSlot` is the only font-slot type; `Paths::slot_names` and `&str` slots are gone.
- All config/font writes are atomic (temp + rename in the same directory); symlink targets are written through, never replaced.
- Commit GPG-signed (`git commit -S`) and verify with `git verify-commit HEAD` (must show `G`); run `sh "$HOME/.config/opencode/skills/gpg-commit/scripts/unlock-gpg.sh"` once per session before the first commit.
- Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` — all green before the branch is done.

## Review Focus

1. **A config file that is a symlink** (`colors.properties` → theme file). Writes must edit the link target, not clobber the link. → Task 2 test `write_atomic_follows_symlink`.
2. **`colors.properties` is a directory, or an `apply_theme` target is a directory.** Must error, never `remove_file` a directory. → Task 4 test `directory_target_is_rejected`.
3. **Previewing several slots then quitting** leaves no slot previewed and restores all originals. → Task 5 test `restore_covers_every_previewed_slot`.
4. **A font library that exists but is unreadable** must exit 1 on `--list-fonts` and show a status in the TUI, not panic or silently list nothing. → Task 1 test `unreadable_library_is_an_error`, Task 11 exit-code test.
5. **A download whose body is not a valid font** (HTTP 200, wrong bytes) must be rejected and its temp file removed. → Task 9 test `install_bytes_rejects_invalid_font`.

---

## File map

- `src/model/font_slot.rs` (new) — `FontSlot`.
- `src/runtime.rs` (new) — `Msg`, `Runtime`, worker spawners.
- `src/ui/mod.rs` — `Ui`, `pane`, re-exports.
- `src/ui/glyphs.rs` — `Glyphs` struct (replaces free fns + `ui::ascii`).
- `src/ui/ascii.rs` — deleted.
- `src/persist/fs.rs` (new) — `write_atomic`, `read_text`.
- `src/fonts/slot_apply` — no; `FontSlot` lives in `model`.
- `src/screens/font_catalog.rs` (new) — Nerd Font download screen.

---

### Task 1: `FontSlot` enum, `Paths` slot paths, library-scan errors

**Files:**
- Create: `src/model/font_slot.rs`
- Modify: `src/model/mod.rs`, `src/paths.rs`, `src/cli.rs`, `src/main.rs` (slot usages only), `src/fonts/apply.rs`, `src/fonts/scan.rs`, `src/screens/fonts_tab.rs`
- Test: `src/model/font_slot.rs` (`#[cfg(test)]`), `tests/paths.rs`

**Interfaces:**
- Produces:
  ```rust
  pub enum FontSlot { Regular, Bold, Italic, BoldItalic }
  impl FontSlot {
      pub const ALL: [FontSlot; 4];
      pub fn as_str(self) -> &'static str;      // "regular","bold","italic","bold-italic"
      pub fn file_name(self) -> &'static str;   // "font.ttf","font-bold.ttf","font-italic.ttf","font-bold-italic.ttf"
  }
  impl TryFrom<&str> for FontSlot { type Error = crate::error::Error; }
  impl std::fmt::Display for FontSlot;          // same as as_str
  ```
  `PathBuf font_slot_path(&self, slot: FontSlot)` (was `Result<PathBuf>`; no longer fallible). `Paths::slot_names()` deleted.
  `cli::Args.slot: Option<FontSlot>` (default `None`).
  `scan::list_library(&Paths) -> std::io::Result<Vec<FontEntry>>` — `NotFound` → `Ok(vec![])`, other read errors → `Err`.
- Consumes: nothing.

- [ ] **Step 1: Write failing tests**

`src/model/font_slot.rs` unit tests:
```rust
#[test] fn all_slots_round_trip() { for slot in FontSlot::ALL { assert_eq!(FontSlot::try_from(slot.as_str()).unwrap(), slot); } }
#[test] fn file_names_are_the_termux_originals() { assert_eq!(FontSlot::Bold.file_name(), "font-bold.ttf"); }
#[test] fn rejects_unknown_slot() { assert!(FontSlot::try_from("nope").is_err()); }
```
`tests/paths.rs`: `font_slot_path(FontSlot::Bold)` ends with `font-bold.ttf`; `font_slot_path(FontSlot::Regular)` ends with `font.ttf`.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets font_slot`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement `FontSlot` and rewire `Paths`**

Write `FontSlot` as above. In `paths.rs`, replace the `&str` `match` with `slot.file_name()`; `font_slot_path` returns `PathBuf`. Update every caller that used `Paths::slot_names()` or `&str` slots: `fonts/apply.rs` (`snapshot_originals`, `restore_original`), `fonts/scan.rs` (`read_active` — delete this fn here or in Task 12; for now keep compiling by mapping `FontSlot::ALL`), `screens/fonts_tab.rs` (slot cycling + preview calls), `cli.rs` (`--slot` parsing via `FontSlot::try_from`), `main.rs` (`install_font(..., slot)`).

- [ ] **Step 4: Implement `scan::list_library` error distinction**

Change the `let Ok(read_dir) = fs::read_dir(&dir) else { return Ok(Vec::new()) }` branch to return `Err` for anything but `NotFound`, and add the unit/integration test `unreadable_library_is_an_error` (chmod `000` a dir under the temp root, assert `list_library` is `Err`).

- [ ] **Step 5: Run tests**

Run: `cargo test --all-targets`
Expected: PASS (full suite green).

- [ ] **Step 6: Commit**

```bash
git add src/model/font_slot.rs src/model/mod.rs src/paths.rs src/cli.rs src/main.rs src/fonts src/screens/fonts_tab.rs tests/paths.rs
git commit -S -m "refactor: typed FontSlot replaces string font slots"
```

---

### Task 2: Atomic writes + shared `read_text`

**Files:**
- Create: `src/persist/fs.rs`
- Modify: `src/persist/mod.rs`, `src/persist/colors_io.rs`, `src/persist/theme_io.rs`
- Test: `src/persist/fs.rs` (`#[cfg(test)]`), plus existing round-trip tests as the regression net.

**Interfaces:**
- Produces:
  ```rust
  pub fn write_atomic(path: &Path, bytes: &[u8]) -> crate::error::Result<()>;
  pub fn read_text(path: &Path) -> crate::error::Result<String>;
  ```
  `write_atomic` resolves `path` via `canonicalize` when it is a symlink, writes `<dir>/.<name>.tmp-<pid>`, `sync_all`, copies the existing file's permissions when one exists, then renames over the destination.
- Consumes: `crate::error::Result`.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn write_atomic_follows_symlink();      // write to a link edits the target, link still a symlink
#[test] fn write_atomic_preserves_permissions(); // 0o644 target keeps 0o644
#[test] fn write_atomic_leaves_old_bytes_on_error(); // simulate failure (read-only dir) -> original intact
#[test] fn read_text_replaces_invalid_utf8();   // "\xFF" -> U+FFFD, no panic
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets write_atomic`
Expected: FAIL.

- [ ] **Step 3: Implement `write_atomic` and `read_text` in `src/persist/fs.rs`**

Add `pub mod fs;` to `src/persist/mod.rs`. Move the two `read_text` bodies here and re-export; delete the private copies in `colors_io.rs` and `theme_io.rs`.

- [ ] **Step 4: Route `write_colors`/`write_app_theme` through `write_atomic`**

Replace the trailing `std::fs::write(path, content)` in both with `crate::persist::fs::write_atomic(path, content.as_bytes())`, keeping `preserve` and `create_dir_all` behaviour. This is the atomicity fix (spec §5.1).

- [ ] **Step 5: Run tests**

Run: `cargo test --all-targets`
Expected: PASS (existing round-trip/CRLF fixtures stay byte-identical).

- [ ] **Step 6: Commit**

```bash
git add src/persist
git commit -S -m "feat: atomic config writes and shared read_text"
```

---

### Task 3: Generalise backups (extension + stamp predicate)

**Files:**
- Modify: `src/persist/backup.rs`
- Test: `src/persist/backup.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  ```rust
  pub fn make_unique_backup(source: &Path, dir: &Path, prefix: &str, ext: &str) -> Result<Option<Backup>>;
  pub fn is_stamped_name(name: &str, stem: &str, ext: &str) -> bool;
  ```
  `is_stamped_name` accepts exactly `<stem>-YYYY-MM-DD-HHMMSS<ext>` (dashes at offsets 4/7/10, digits elsewhere). `make_backup` delegates with `ext = "properties"`.
- Consumes: `format_timestamp`.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn stamped_name_accepts_17_char_stamp();  // "colors-2026-10-25-123456.properties" true
#[test] fn stamped_name_rejects_wrong_length();   // 15-char middle false; "colors-2026-10-25.properties" false
#[test] fn stamped_name_rejects_misplaced_dashes(); // "colors-2026-1-25-123456.properties" false
#[test] fn make_unique_backup_writes_custom_ext(); // font.ttf -> "font-...ttf" exists with same bytes
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets stamped_name`
Expected: FAIL.

- [ ] **Step 3: Implement both functions**

Extract the body of `make_backup` into `make_unique_backup`, parameterising the extension and reusing the collision counter; `make_backup` becomes a one-line delegate. Implement `is_stamped_name` against the 17-char layout.

- [ ] **Step 4: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/persist/backup.rs
git commit -S -m "feat: generalise backups with extension and stamp predicate"
```

---

### Task 4: `apply_theme` atomic swap + directory rejection

**Files:**
- Modify: `src/library.rs`
- Test: `tests/library.rs`

**Interfaces:**
- Consumes: `crate::error::{Error, Result}`.
- Produces: unchanged `pub fn apply_theme(theme: &Path, colors_properties: &Path) -> Result<()>`.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn directory_target_is_rejected();       // colors_properties as a dir -> Err, dir still exists
#[test] fn theme_resolving_to_directory_is_rejected(); // theme is a dir -> Err(Invalid)
#[test] fn failed_swap_leaves_old_link();        // force rename failure (target parent read-only) -> old symlink intact
#[test] fn self_link_is_a_noop();                // theme already == resolved target -> Ok, no error
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets directory_target_is_rejected`
Expected: FAIL (current code `remove_file`s the dir path and errors or worse).

- [ ] **Step 3: Implement the atomic swap**

Rewrite `apply_theme`: (1) `canonicalize(theme)` with `NotFound`/`Invalid` for a directory; (2) no-op when `colors_properties` resolves to the same file; (3) `symlink_metadata(colors_properties)` — if it exists and `is_dir()`, `Err(Invalid)`; (4) `symlink(resolved, tmp)` at `.<name>.tmp-<pid>` in the same dir, then `rename(tmp, colors_properties)`; on any error remove the temp link and leave the old one. No `remove_file` before the new link exists.

- [ ] **Step 4: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/library.rs tests/library.rs
git commit -S -m "fix: activate themes atomically and reject directory targets"
```

---

### Task 5: Rewrite `fonts::apply` (per-slot restore, correct backups, no `static mut`)

**Files:**
- Modify: `src/fonts/apply.rs`
- Test: `src/fonts/apply.rs` (`#[cfg(test)]`), plus `tests/ui_fonts.rs` (created here with the restore/quit cases)

**Interfaces:**
- Produces:
  ```rust
  pub struct PreviewRef { pub src: PathBuf, pub slot: FontSlot }
  pub struct SessionState {
      originals: HashMap<FontSlot, Option<Vec<u8>>>,
      previewed: BTreeMap<FontSlot, PreviewRef>,
      backed_up: BTreeSet<FontSlot>,
  }
  pub fn new_session_state(paths: &Paths) -> SessionState;
  pub fn preview_font(paths: &Paths, src: &Path, slot: FontSlot, st: &mut SessionState) -> crate::error::Result<PathBuf>;
  pub fn commit_preview(paths: &Paths, st: &mut SessionState) -> bool;   // clears previewed; takes &Paths (no from_env)
  pub fn restore_original(paths: &Paths, st: &mut SessionState) -> std::io::Result<bool>; // restores every previewed slot
  pub fn install_font(paths: &Paths, src: &Path, slot: FontSlot) -> crate::error::Result<PathBuf>;
  pub fn is_dirty(st: &SessionState) -> bool;                             // !previewed.is_empty()
  ```
  Deleted: `LAST_RELOAD_OK`, `last_reload_ok`, `is_preview_dirty`, `reload_settings`, `format_timestamp`, `PreviewRef.slot: String`.
- Consumes: `backup::make_unique_backup`, `backup::is_stamped_name`, `library::reload_termux`, `FontSlot`.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn restore_covers_every_previewed_slot();   // preview regular then bold, restore -> both reverted
#[test] fn commit_keeps_both_and_leaves_nothing_to_restore();
#[test] fn ensure_backup_once_writes_once_per_slot(); // same slot twice -> one backup
#[test] fn backup_name_has_a_real_date();            // filename parses to the current local date (regression for 16-month bug)
#[test] fn reload_result_is_returned_not_global();    // no last_reload_ok symbol (compile-time), reload returns bool
```
`tests/ui_fonts.rs` (support-driven): `preview_then_quit_restores` — drive `App`, open Fonts tab, preview into two slots, send `q`, assert both slot files equal their originals.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets restore_covers_every_previewed_slot`
Expected: FAIL.

- [ ] **Step 3: Implement the rewrite**

Rebuild `SessionState` around the three maps; `preview_font` inserts (never overwrites) into `previewed` and dedupes `ensure_backup_once` via `backed_up` + `is_stamped_name`; `commit_preview(&paths, st)` clears `previewed` and returns `true` when something was previewed; `restore_original` walks all of `previewed` (write original bytes, or remove the file when the slot had none) then clears. Replace the hand-rolled timestamp with `backup::make_unique_backup(target, &paths.backups_dir(), stem, ".ttf")`. Call `library::reload_termux()` (delete `reload_settings`); surface `MANUAL_RESTART_HINT` to callers via a status/dialog.

- [ ] **Step 4: Add `App::on_exit` + `Screen::on_quit` so `q` restores previews**

In `src/app.rs` add `fn on_exit(&mut self, paths: &Paths)` called by `handle_key` before setting `quit` and by `main` before breaking; in `src/screens/mod.rs` add `pub fn on_quit(&mut self, paths: &Paths) -> ScreenAction` (FontsTab restores; others `None`). Wire `q`/`Ctrl+C`/`Ctrl+Q` to quit-with-restore.

- [ ] **Step 5: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/fonts/apply.rs src/app.rs src/screens/mod.rs tests/ui_fonts.rs
git commit -S -m "fix: per-slot font restore, correct backups, no global reload state"
```

---

### Task 6: `Glyphs` + `Ui` context, `Palette: Copy`, shared panes

**Files:**
- Modify: `src/ui/glyphs.rs`, `src/ui/mod.rs`, `src/theme.rs`, `src/chrome.rs`, `src/app.rs`, all `src/screens/*.rs`
- Delete: `src/ui/ascii.rs`
- Test: `src/ui/glyphs.rs` (`#[cfg(test)]`), `tests/ui_ascii.rs` (new)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Copy, Clone)] pub struct Glyphs { ascii: bool }
  impl Glyphs {
      pub const fn new(ascii: bool) -> Self;
      pub fn marker(self) -> &'static str; pub fn rule_char(self) -> char;
      pub fn chip(self) -> (&'static str, &'static str); pub fn dot(self) -> &'static str;
      pub fn ellipsis(self) -> &'static str; pub fn filter(self) -> &'static str;
      pub fn breadcrumb(self) -> &'static str; pub fn tick(self) -> &'static str;
      pub fn border(self) -> ratatui::widgets::BorderType;   // Plain when ascii, Rounded otherwise
      pub fn arrow(self) -> &'static str;   // "↑/↓" / "up/dn"
      pub fn lr(self) -> &'static str;      // "←/→" / "l/r"
  }
  #[derive(Copy, Clone)] pub struct Ui { pub palette: Palette, pub glyphs: Glyphs }
  pub fn pane<'a>(ui: &Ui, title: &'a str) -> Block<'a>;    // bordered, border=ui.glyphs.border(), accent border, surface bg
  ```
  `Palette` derives `Copy`. `chrome::gradient_title(text, dark) -> Line<'static>`; `chrome::help_bar(pairs, accent, muted, glyphs) -> Line<'static>` (maps `↑/↓`/`←/→` via glyphs, joins with `glyphs`-aware ` · `).
- Consumes: `theme::Palette`, `chrome`.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn glyphs_switch_with_ascii();      // marker/chip/border/arrow flip between modes
#[test] fn pane_uses_ascii_border();        // Glyphs::new(true).border() == Plain
```
`tests/ui_ascii.rs`: build an `App` with `Glyphs::new(true)`, render the menu, assert the frame contains no `─`/`│` box-drawing chars and the help bar shows `up/dn` not `↑/↓`.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets glyphs_switch_with_ascii`
Expected: FAIL.

- [ ] **Step 3: Implement `Glyphs`, `Ui`, `pane`, `Palette: Copy`, chrome changes**

Replace the free fns in `glyphs.rs` with the struct; delete `ui/ascii.rs` and its `AtomicBool`. Add `pub use`/`Ui` in `ui/mod.rs`. Change every `render(&mut self, frame, area, palette: &Palette)` to `render(&mut self, frame, area, ui: &Ui)` and every `glyphs::marker()`-style call to `ui.glyphs.marker()`. Replace the nine inline `Block::bordered().border_type(Rounded)` panes with `ui::pane(ui, " …")`. `App` gains `glyphs: Glyphs` (built from `cli::Args.ascii`); `main` passes it. `App::render` builds `Ui { palette, glyphs }` (Copy) and passes `&ui`.

- [ ] **Step 4: Run tests**

Run: `cargo test --all-targets`
Expected: PASS (all 133 + new).

- [ ] **Step 5: Commit**

```bash
git add src/ui src/theme.rs src/chrome.rs src/app.rs src/screens src/main.rs tests/ui_ascii.rs
git commit -S -m "refactor: Glyphs/Ui render context, Copy palette, ASCII borders everywhere"
```

---

### Task 7: Runtime + reducer loop, `App::update`, resize/tick, quit-restore

**Files:**
- Create: `src/runtime.rs`
- Modify: `src/lib.rs`, `src/main.rs`, `src/app.rs`, `src/screens/mod.rs`
- Test: `tests/ui_runtime.rs` (new)

**Interfaces:**
- Produces:
  ```rust
  pub const TICK: std::time::Duration;   // 250ms
  pub enum Msg {
      Tick(Instant), Key(KeyEvent), Resize(u16, u16),
      Reloaded(bool), PromptCaptured(Vec<String>),
      FontsScanned(std::io::Result<Vec<FontEntry>>),
      DownloadProgress { name: String, received: u64, total: Option<u64> },
      Downloaded(Result<PathBuf, String>),
  }
  pub struct Runtime { rx: Receiver<Msg>, tx: Sender<Msg> }
  impl Runtime { pub fn new() -> Self; pub fn sender(&self) -> Sender<Msg>; pub fn drain(&self) -> Vec<Msg>; }
  pub fn spawn_reload(tx: Sender<Msg>); pub fn spawn_prompt(tx: Sender<Msg>);
  pub fn spawn_scan(tx: Sender<Msg>, paths: Paths);
  pub fn spawn_download(tx: Sender<Msg>, paths: Paths, name: String, url: String);
  ```
  `App::update(&mut self, msg: Msg)` dispatches; `App::set_area(Rect)` stores the size; `Screen::on_msg(&mut self, msg: &Msg, paths: &Paths) -> ScreenAction`.
- Consumes: `App`, `fonts::scan::FontEntry`, `Paths`.

- [ ] **Step 1: Write failing tests**

`tests/ui_runtime.rs`:
```rust
#[test] fn tick_expires_status_within_one_tick();  // set_status, update(Tick(now+4s+250ms)) -> status gone
#[test] fn resize_sets_too_small();                 // update(Resize(10,10)) -> render shows guard, no panic
#[test] fn q_quits_from_a_sub_screen();             // open picker, key('q') -> should_quit
#[test] fn msg_prompt_updates_the_fonts_tab();      // update(PromptCaptured(vec!["$> "])) reflected in preview
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets tick_expires_status_within_one_tick`
Expected: FAIL.

- [ ] **Step 3: Implement `Runtime` and the loop**

`main`:
```rust
let guard = terminal::Terminal::enter()?;
let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
let mut app = App::new(paths, glyphs);
let rt = Runtime::new();
loop {
    for msg in rt.drain() { app.update(msg); }
    terminal.draw(|f| app.render(f))?;
    if app.should_quit() { break; }
    if event::poll(TICK)? {
        match event::read()? { Event::Key(k) if k.kind != Release => app.update(Msg::Key(k)),
                               Event::Resize(w, h) => app.update(Msg::Resize(w, h)), _ => {} }
    } else { app.update(Msg::Tick(Instant::now())); }
}
```

- [ ] **Step 4: Wire `App::update`**

`update` matches the loop msgs; `Msg::Key` → `handle_key`, `Tick` → `tick`, `Resize` → `set_area` + `current_mut().on_resize(area)`, worker msgs → `current_mut().on_msg(...)`. Add `Screen::on_resize` and `Screen::on_msg` delegating per variant (fonts screens handle worker msgs, others no-op).

- [ ] **Step 5: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/runtime.rs src/lib.rs src/main.rs src/app.rs src/screens/mod.rs tests/ui_runtime.rs
git commit -S -m "feat: poll-driven runtime with a single App::update reducer"
```

---

### Task 8: Pure `render(&self)` + `on_resize`, `Dialog` widget, picker header + scrollbar

**Files:**
- Modify: `src/app.rs`, `src/screens/mod.rs`, all `src/screens/*.rs`, `src/ui/dialog.rs`, `src/ui/input.rs`, `src/ui/form.rs`
- Test: `tests/ui_dialog.rs`, `tests/ui_picker.rs`

**Interfaces:**
- Produces: `Screen::render(&self, frame, area, ui: &Ui)` and per-screen `render(&self, …)`; `Screen::on_resize(&mut self, area: Rect)`; `Field::visible(&self, width: usize) -> &str` (display window, no `set_width` in draw); `Dialog::widget<'a>(&'a self, ui: &'a Ui) -> DialogWidget<'a>` with `impl Widget for DialogWidget`; `PresetPicker` header shows the active theme's library-relative name and a local `ScrollbarState` when the list overflows.
- Consumes: everything from Tasks 6–7.

- [ ] **Step 1: Write failing tests**

`tests/ui_dialog.rs`:
```rust
#[test] fn multiline_confirm_renders_every_line();  // long message -> all words present in rendered buffer
```
`tests/ui_picker.rs`:
```rust
#[test] fn header_shows_active_theme_label();
#[test] fn scrollbar_appears_when_list_overflows(); // 40 themes, 10 rows -> scrollbar glyph present
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets multiline_confirm_renders_every_line`
Expected: FAIL.

- [ ] **Step 3: Drop `&mut` from every `render`**

Move the geometry-derived mutation out of draw into `on_resize`/`update`: `too_small` (computed locally + stored), `PresetPicker::memoize_canonical` (call in `opened`/`reload`), `colors_for` cache (eager fill in `opened`), `TerminalEditor::visible_rows`/`scroll_to_focus` (in `on_resize`), `Field::set_width` (replaced by `visible(width)`), `FontsTab`/`AppThemeEditor` widths likewise. `App::render` no longer mutates `self` except via the stored `area`.

- [ ] **Step 4: Implement the `Dialog` widget**

`Dialog::height()` computes wrapped line count greedily (split on whitespace, pack up to width, count lines) + 1 for buttons + 1 for the hint row on `Confirm`; `DialogWidget::render` draws over `Clear` (the caller renders `Clear` first). Update `App::render` to `frame.render_widget(Clear, area); frame.render_widget(dialog.widget(&ui), area);`.

- [ ] **Step 5: Add picker header + scrollbar**

In `render_list`, add a title line showing `active` library-relative name (derive from `active_theme_path` + `strip_prefix`), and render `Scrollbar::default()` with a locally built `ScrollbarState::new(visible_len).position(selected)` when `visible.len()` exceeds the pane rows.

- [ ] **Step 6: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs src/screens src/ui
git commit -S -m "refactor: pure render views, widget-based dialogs, picker header and scrollbar"
```

---

### Task 9: Fonts tab behaves as documented (filter, import, download, prompt worker)

**Files:**
- Modify: `src/screens/fonts_tab.rs`, `src/fonts/prompt.rs`, `src/fonts/downloader.rs`, `src/fonts/importer.rs`, `src/screens/mod.rs`
- Create: `src/screens/font_catalog.rs`
- Test: `tests/ui_fonts.rs` (extend), `src/fonts/downloader.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  ```rust
  // prompt.rs
  pub fn capture_prompt_lines(shell: Option<&str>) -> Vec<String>;  // $SHELL; zsh print -P, bash ${PS1@P}, else "$ "; 3s cap
  // downloader.rs
  pub fn sorted_catalog() -> Vec<(String, String)>;                 // name, url — sorted by name
  pub fn download(url: &str, on_progress: impl Fn(u64, Option<u64>)) -> Result<Vec<u8>, String>;
  pub fn install_bytes(paths: &Paths, name: &str, bytes: &[u8]) -> crate::error::Result<PathBuf>;
  // font_catalog.rs
  pub struct FontCatalog { /* entries: Vec<(String,String)>, visible, filter, state, progress */ }
  ```
  `FontsTab::new` no longer copies `Current.ttf`; it starts `prompt = vec!["$ ".into()]` and spawns `spawn_prompt`. `handle_key` `d` pushes `ScreenKind::FontCatalog`. Filter narrows `visible` by name/family/style; `Esc` clears the filter, then pops.
- Consumes: `Runtime` senders, `Msg`, `importer::import_file`, `Dialog`.

- [ ] **Step 1: Write failing tests**

```rust
// downloader.rs unit
#[test] fn install_bytes_rejects_invalid_font();     // garbage bytes -> Err, no file created
#[test] fn install_bytes_writes_atomically();         // valid ttf bytes -> library file, name-derived
#[test] fn sorted_catalog_is_sorted();                // names ascending
```
`tests/ui_fonts.rs`:
```rust
#[test] fn filter_narrows_the_list_and_esc_clears(); // type in filter -> fewer visible; Esc -> filter empty
#[test] fn esc_restores_then_pops();                 // dirty preview -> Esc restores + pops
#[test] fn import_flow_offers_clash_dialog();        // i, path prompt, clash -> keep-both creates -1
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets install_bytes_rejects_invalid_font`
Expected: FAIL.

- [ ] **Step 3: Implement prompt, downloader split, catalog screen, tab wiring**

`capture_prompt_lines` per spec §4.6 (poll `try_wait` every 25 ms, kill at 3 s, read stdout only after exit). Split `fetch` into `download` (returns bytes, reports progress with `Content-Length` when present; delete the discarded `HEAD` request) and `install_bytes` (validate + `write_atomic` into the library). Build `font_catalog.rs` as a filterable list; `Enter` spawns `spawn_download`; the footer shows `name NN%` from `DownloadProgress`; `Downloaded` refreshes and statuses. Wire `i` → prompt → `import_file` → clash dialog → re-run with keep-both/replace. Remove the `Current.ttf` seeding.

- [ ] **Step 4: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/fonts src/screens
git commit -S -m "feat: working font import, download, filter, and prompt capture"
```

---

### Task 10: App-theme read errors + `Row` refactor

**Files:**
- Modify: `src/screens/app_theme_editor.rs`, `src/screens/mod.rs`
- Test: `tests/ui_app_theme.rs`

**Interfaces:**
- Produces: `AppThemeEditor` stores `load_error: Option<String>`; when set, `opened()` returns `ScreenAction::OpenDialog(Dialog::message(…))`, `command(Save)` returns a refusal status, and `write`/`unset_value` are unreachable. `rows: Vec<Row>` (built from `CURATED_FIELDS.iter()`); `Row` drops the mirrored `value: Option<String>` for text/int rows — text/int rows read `field.value()`, selector rows keep a `selection: Option<usize>` index.
- Consumes: `CURATED_FIELDS`, `validate_value`.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn unreadable_theme_shows_dialog_and_blocks_save(); // read error -> dialog visible, Ctrl+S leaves file untouched
#[test] fn selectors_cycle_through_choices_and_unset();     // replace the empty test: Left/Right cycles, reaches unset
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets unreadable_theme_shows_dialog_and_blocks_save`
Expected: FAIL.

- [ ] **Step 3: Implement**

Change `new` to keep the `Result` from `read_app_theme` instead of `unwrap_or_default()`; add the guard paths; rebuild `rows` as a `Vec`; remove the text/int `value` mirror and the `Row::current`/`is_selector`/`choices` duplication is consolidated into one `fn current(&self)` and one `fn cycle`. Update `tests/support/mod.rs` helpers that referenced `row.value`/`row.field` mirrors accordingly.

- [ ] **Step 4: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/screens/app_theme_editor.rs src/screens/mod.rs tests/ui_app_theme.rs tests/support/mod.rs
git commit -S -m "fix: app-theme read errors refuse to save; Row mirrors removed"
```

---

### Task 11: CLI exit codes and `--list-fonts` error surfacing

**Files:**
- Modify: `src/main.rs`, `src/cli.rs`
- Test: `tests/cli.rs` (new, if the binary is testable via `run()`)

**Interfaces:**
- Produces:
  ```rust
  pub fn run(args: Args, paths: &Paths) -> Result<ExitCode, crate::error::Error>;
  ```
  `main` becomes `fn main() -> ExitCode` calling `run`, printing `error: {e}` to stderr on failure. `--list-fonts` propagates `scan::list_library` errors (exit 1); `--apply-font` unknown name exits 2; unknown `--slot` exits 2. No `process::exit`.
- Consumes: Task 1 `scan::list_library`, `FontSlot`.

- [ ] **Step 1: Write failing tests**

`tests/cli.rs`:
```rust
#[test] fn list_fonts_reports_unreadable_library();  // run(Args{list_fonts:true,..}, unreadable dir) -> Err, not Ok(exit 0)
#[test] fn apply_font_unknown_name_exits_2();         // run returns Ok(ExitCode::from(2))
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --all-targets list_fonts_reports_unreadable_library`
Expected: FAIL.

- [ ] **Step 3: Implement `run`**

Move the CLI branches out of `main` into `run`; `Args` carries `slot: Option<FontSlot>` (from Task 1) and `parse` errors on `--slot bogus`. `main` keeps only `Args::parse`, the `--version` early return, `install_panic_hook`, and the TUI loop, delegating the CLI path to `run` and returning its exit code.

- [ ] **Step 4: Run tests**

Run: `cargo test --all-targets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/cli.rs tests/cli.rs
git commit -S -m "fix: CLI propagates errors and returns proper exit codes"
```

---

### Task 12: Dead-code sweep + `ScreenAction::Applied` consolidation + `Screen::new`/`reload`

**Files:**
- Modify: `src/screens/mod.rs`, `src/app.rs`, all `src/screens/*.rs`, `src/model/colors.rs`, `src/fonts/scan.rs`
- Test: full suite (no new behaviour).

**Interfaces:**
- Produces:
  ```rust
  pub enum ScreenAction {
      None, Quit, Push(ScreenKind), Pop, OpenDialog(Dialog),
      Applied { colors: Option<TerminalColors>, message: Option<String> },
  }
  impl ScreenAction { pub fn status(msg: &str) -> Self; pub fn applied(colors: TerminalColors, msg: &str) -> Self; }
  pub fn Screen::new(kind: ScreenKind, paths: &Paths) -> Self;   // no Option, no unreachable!
  pub fn Screen::reload(&mut self, paths: &Paths);               // replaces refresh; every screen has a body
  ```
  Deleted: `ScreenAction::{RefreshPalette, Status, RefreshPaletteAndStatus}`, `Colors::is_managed`, `fonts::scan::{describe, read_active}`, `downloader`'s dead `HEAD` path (already removed in Task 9), `AppThemeEditor`/`TerminalEditor`'s redundant `Paths` clone, `let _ = meta`/`_resolved` leftovers.
- Consumes: Tasks 1–11.

- [ ] **Step 1: Consolidate `ScreenAction`**

Replace the three removed variants with `Applied`; update every `ScreenAction::Status(..)`/`RefreshPalette(..)`/`RefreshPaletteAndStatus(..)` return site to `ScreenAction::status(..)`/`ScreenAction::applied(..)`, and `App::apply` to fold `Applied` into `colors`/`status`. `ScreenAction::applied(colors, "")` = palette-only; `status(msg)` = message-only.

- [ ] **Step 2: Make `Screen::new` infallible; rename `refresh` → `reload`**

Delete the `Option` and the `unreachable!`; give every screen a `reload` body (menu re-reads active theme; picker re-scans + rebuilds caches; editors re-check mtime; fonts re-scan). Update the `Pop` handler to call `reload`.

- [ ] **Step 3: Delete the dead items and redundant clones**

Remove `is_managed` (and its two tests), `describe`, `read_active`, and the editors' stored `Paths`.

- [ ] **Step 4: Run tests and lints**

Run: `cargo test --all-targets` then `cargo clippy --all-targets -- -D warnings`
Expected: PASS / clean (no `unused` warnings).

- [ ] **Step 5: Commit**

```bash
git add src/screens src/app.rs src/model/colors.rs src/fonts/scan.rs
git commit -S -m "refactor: single Applied action, infallible Screen::new, dead code removed"
```

---

### Task 13: Docs + final gates

**Files:**
- Modify: `README.md`
- Test: none (manual read + gates).

- [ ] **Step 1: Rewrite the README keyboard and CLI tables**

Update README to match: Fonts `i`/`d`/`/`/`Esc`/`p`/`Enter`/`s` behaviour; `q`/`Esc`/`Ctrl+C`/`Ctrl+Q`; `--ascii` covering borders and the help bar; `--list-fonts`/`--apply-font`/`--slot` exit codes; the Fonts features are no longer described as stubs.

- [ ] **Step 2: Final gates**

Run: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`
Expected: all green; suite larger than 133 (every Task added named tests).

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -S -m "docs: README matches implemented behaviour and CLI"
```

---

## Self-review notes

- **Spec coverage:** §3.1→T7, §3.2→T8, §3.3→T6, §3.4→T1, §3.5→T8, §4.1–4.6→T5/T9, §5.1→T2, §5.2→T4, §5.3→T10, §6→T12, §7→T11, §8→per-task tests, §9→T13.
- **Type consistency:** `FontSlot` (T1) is consumed by T5/T9/T11; `Ui`/`Glyphs` (T6) by T8; `Msg`/`Runtime` (T7) by T9; `write_atomic` (T2) by T9's `install_bytes`; `make_unique_backup`/`is_stamped_name` (T3) by T5.
- **Review Focus:** all five inputs have named tests in the owning task (T2, T4, T5, T1+T11, T9).
- **Proportion:** steps name signatures, test names, and assertions; bodies are left to the implementer except where the spec fixes an algorithm (atomic rename, wrap-count height, poll-and-kill prompt).
