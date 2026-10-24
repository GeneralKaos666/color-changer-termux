# Merge `font-changer` into `termux-colors` — design

Date: 2026-10-24
Status: draft (awaiting review)

## Goal

Fold the standalone Go `font-changer` application into this Rust
`termux-colors` app so that **one binary** changes terminal colors, the Termux
app theme, **and** the terminal font, through a single, calmer, more
discoverable interface.

## Success criteria

- One Rust binary (`termux-colors`), no Go toolchain at install time.
- A top-level **tab bar** — `Colors`, `Fonts`, `App Theme` — each reachable in
  one keystroke; no drill-down menu.
- The full `font-changer` feature set works: browse/apply fonts, live preview
  against the real shell prompt, four font slots, import own fonts, download
  Nerd Fonts, safe backups, and `--list-fonts` / `--apply-font` / `--version`
  / `--ascii` CLI flags.
- The UI feels **smoother**: event-driven redraws, no per-frame disk I/O, no
  visible repaint jank, one consistent look, and an always-on footer key bar.

## Non-goals

- No new language runtime; the Go code is ported, not embedded or shelled out
  to.
- No glyph *rendering* (we parse font metadata, never rasterize glyphs).
- No change to the on-disk formats: `colors.properties`,
  `termux.properties`, and the `~/.termux/fonts` + slot-file layout stay
  byte-compatible with both predecessors.
- No change to backup file naming or location.

---

## 1. Top-level UI structure

Replace the four-item main menu with a **tab bar**:

```
┌ Colors ─┬ Fonts ─┬ App Theme ─┐
│                                │
│          (active tab)          │
│                                │
├────────────────────────────────┤
│  footer: key hints for this tab│
└────────────────────────────────┘
```

- Tabs switch with `Tab` / `Shift+Tab` (and `←` / `→`). Each tab is a full
  screen reached in one step.
- **Colors** merges the existing preset browser and terminal-color editor into
  one pane (browse on the left, edit on the right), removing the
  "Preset Themes" → "Terminal Colors" hop.
- **Fonts** is the ported picker (preview on top, collection list below).
- **App Theme** is the existing `termux.properties` editor, behavior unchanged.
- The **screen push/pop stack is removed**; tabs replace it. Each tab keeps its
  own list/scroll/cursor state, so switching never resets where you were.
- The **dialog system stays** for confirmations (save, backup, clash, quit).
- An **always-on footer hint bar** shows the live key bindings for the current
  tab and focus.

### Key map (unified)

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` or `←` / `→` | switch tab |
| `↑` / `↓`, `j` / `k` | move within the focused pane |
| `Enter` | activate / confirm |
| `Esc` | back / cancel within the current context (then tab bar, then quit) |
| `q`, `Ctrl+C` | quit from anywhere (restoring any uncommitted preview) |
| `s` | (Fonts) cycle font slot |
| `Space` / `p` | (Fonts) preview highlighted font |
| `i` | (Fonts) import a font |
| `d` | (Fonts) download a Nerd Font |
| `Ctrl+S` / `Ctrl+A` / `Ctrl+R` | save / save-as / revert (editors) |

## 2. Rendering & smoothness

- **Event-driven redraws.** Draw only when input arrives or time-driven state
  needs it (status expiry, download progress). Remove the unconditional 250 ms
  repaint loop in `main.rs`.
- **Cache active colors + palette in `App`; no disk I/O in render.** Today
  `main_menu.render` re-reads `colors.properties` every frame. Move all such
  reads to explicit state changes (theme applied/reverted, library reloaded).
  Every screen reads the single cached palette — no drift.
- **State-preserving tabs** and **single consistent key map** (above).
- ratatui's existing differential buffer means identical frames emit no bytes;
  idle therefore costs nothing.

## 3. Font subsystem port

New Rust modules, mirroring the Go packages:

```
src/paths.rs        + font locations (fonts dir, 4 slot files, backups dir)
src/fonts/
  mod.rs
  scan.rs          FontEntry, FontDetails, list_library, family_style
  validate.rs      is_valid_font (magic bytes + parse + forced name read)
  apply.rs         SessionState, preview/commit/restore, backups, reload
  importer.rs      import_file with clash modes (ask/error/keep-both/replace)
  downloader.rs    NerdFonts catalog, fetch with progress
  prompt.rs        capture_prompt_lines (live shell prompt)
src/screens/fonts.rs   the "Fonts" tab (picker + import/download overlays)
```

- **Slot files**: `font.ttf`, `font-bold.ttf`, `font-italic.ttf`,
  `font-bold-italic.ttf` under the Termux root; canonical order `regular`,
  `bold`, `italic`, `bold-italic`; unknown slot is an error.
- **Scan** lists `~/.termux/fonts/*.ttf|.otf`, reads family/style from the name
  table, caches `Describe` details only when the selection changes (no disk
  I/O during render).
- **Preview lifecycle** (exact semantics preserved): highlighting never
  installs; `Space`/`p` previews (install + mark dirty), `Enter` commits,
  `Esc`/quit restores session-start bytes. One timestamped backup per slot per
  session (`EnsureBackupOnce`); atomic temp-file+rename copies.
- **Import** validates then copies into the library; collisions offer
  keep-both / replace / cancel; permission-denied shows the
  `termux-setup-storage` hint.
- **Download** runs on a background thread, reports byte progress through a
  channel drained by the event-loop tick (feeds spinner/progress bar).
- **Prompt capture** reuses the existing poll-with-timeout pattern
  (`reload_with_path_env`): `$SHELL -ic` with `print -P "$PROMPT"` (zsh) or
  `echo "${PS1@P}"` (bash), 3 s cap, up to 2 lines, mock fallback on failure.

### New dependencies (only two)

- `ttf-parser` — pure-Rust sfnt parsing (replaces `x/image/font/sfnt`).
  Manual 4-byte magic check (`00 01 00 00` / `OTTO` / `true` / `typ1`) is kept
  for parity, then `ttf-parser` confirms the tables parse.
- `ureq` (rustls) — small blocking HTTP client (replaces `net/http`), bundled
  roots, works on Android. The 45-entry Nerd Fonts catalog ports over as a
  static map with unchanged URLs, injectable base URL for tests.

### Dropped

- Go's `theme` package — the Fonts tab inherits this app's cached `Palette`.

### CLI flags

Add to `termux-colors` (hand-rolled arg parser, no new dependency):
`--list-fonts`, `--apply-font NAME --slot SLOT`, `--version`, `--ascii`.
No flags → interactive TUI. `--ascii` sets a global ASCII-borders mode that
affects all tabs' borders and markers.

## 4. Testing

Keep the existing pattern: inline unit tests and `tests/` integration tests via
`support/mod.rs` (`TestBackend`, `Paths::new(tempdir)`), hermetic throughout.

- **Fixtures**: copy `font-changer`'s `a.ttf`/`b.ttf` into
  `tests/fixtures/fonts/`.
- **Unit tests per module**: `validate` (valid / corrupt-but-magic / unknown
  signature / missing); `scan` (extension filter, sort, family/style,
  describe, empty dir); `apply` (preview→commit, preview→restore, backup-once,
  atomic copy, reload stubbed via PATH); `importer` (clash modes, invalid
  font, `stem-N.ttf`); `downloader` (catalog shape, fetch against a local
  `TcpListener` with injectable base URL, invalid download); `prompt` (fake
  `$SHELL` script, timeout/unknown-shell fallback).
- **TUI/integration tests**: tab bar render/switch, footer hints, and the full
  Fonts-tab flow (list focus, filter, slot cycle, preview/commit/restore,
  import and download overlays, applied badges).
- **Revise existing UI tests**: `open_picker`/`open_editor`/`open_app_theme`
  helpers change from "Down+Enter on the menu" to "switch to tab N"; the
  `escape_returns_to_the_menu`-style assertions change to the tab model.
