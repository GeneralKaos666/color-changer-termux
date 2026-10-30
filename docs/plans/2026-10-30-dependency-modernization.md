# Dependency Modernization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every direct dependency to a maintained release (`ratatui 0.30`, `crossterm 0.29`, `tui-input 0.15`, `ureq 3.5`, `skrifa 0.49` replacing `ttf-parser`) and close the seven review minors deferred from phase 1, with no behaviour change.

**Architecture:** Three mechanical migrations in separate commits, ordered by blast radius: the ratatui/crossterm/tui-input triple (one atomic bump, rename-only source changes), the `ureq` rewrite confined to `download()`, and the parser swap confined to two call sites. Then four small, independent maintenance commits. Each migration is guarded by tests that pass before and after, so a failure is always attributable to one commit.

**Tech Stack:** Rust 2024, MSRV `1.88`, ratatui 0.30.2, crossterm 0.29.0, tui-input 0.15.5, ureq 3.5.0, skrifa 0.49.0; GitHub Actions; `actionlint`, `shellcheck`.

**Spec:** `docs/specs/2026-10-30-dependency-modernization-design.md`

## Global Constraints

- MSRV stays `1.88`. Do not raise `rust-version` and do not rewrite code for a newer toolchain.
- `skrifa` is the only new direct dependency; `ttf-parser` is removed. No other dependency may be added.
- No user-visible behaviour change: identical error strings, identical key handling, identical on-disk formats.
- Automated tests must not touch the network. Loopback sockets on `127.0.0.1` are allowed and required for the download tests.
- Exactly one `ratatui`, one `crossterm` and one `tui-input` version in `Cargo.lock` (`cargo tree -d` must not list them).
- Every commit is GPG-signed: `git commit -S`, then confirm `git log --format='%h %G?' -1` reports `G`.
- Conventional Commits, one concern per commit, in the order below.
- Gates before each commit: `cargo test --locked`, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`.
- Workflows must pass `actionlint .github/workflows/*.yml`; scripts must pass `shellcheck scripts/*.sh`.
- Docs live in `docs/specs/` and `docs/plans/` only. Never create `docs/superpowers/`.

## Review Focus

1. A body shorter than its advertised `Content-Length` (dropped connection) must produce an error, never a partial font installed. Pinned by `download_reports_a_truncated_body` in Task 2.
2. A file with a valid font signature but a corrupt table directory must be rejected as `cannot parse font: …` without panicking. Pinned by `rejects_a_valid_signature_with_a_corrupt_table_directory` in Task 3.
3. A non-2xx response must surface `download failed: …` and write nothing. Pinned by `download_reports_a_non_success_status` in Task 2.
4. A `Content-Length` larger than the cap must fail before reading the body, and a `Content-Length` under the cap must still be capped mid-stream. Pinned by the existing `read_capped_*` tests, which must keep passing unchanged.
5. A release tag that is a string prefix of the version (`v1.1.1` against `1.1.10`) must fail the gate. Pinned by `prefix_tag_fails` in Task 4.

---

### Task 1: Bump ratatui, crossterm and tui-input

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Modify: `src/app.rs:7`, `src/ui/mod.rs:10`, `src/ui/dialog.rs:5`,
  `src/screens/app_theme_editor.rs:5`, `src/screens/main_menu.rs:5`
- Test: the existing suite (`src/` unit tests and `tests/ui_*.rs`)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `ratatui::layout::HorizontalAlignment` as the alignment type used by every screen; `ratatui::backend::CrosstermBackend` remains the concrete backend. No public API of this crate changes.

- [ ] **Step 1: Record the baseline**

Run: `cargo test --locked 2>&1 | awk '/^test result/ {s+=$4} END {print s}'`
Expected: `194`.

- [ ] **Step 2: Bump the three crates in `Cargo.toml`**

```toml
ratatui = "0.30"
crossterm = "0.29"
tui-input = "0.15"
```

Leave `ureq`, `ttf-parser` and `time` untouched. Then refresh the lockfile without `--locked`:

Run: `cargo build`
Expected: succeeds or fails only in this crate's source; `Cargo.lock` gains ratatui 0.30.2, crossterm 0.29.0, tui-input 0.15.5, `ratatui-core`, `ratatui-widgets`, `ratatui-crossterm`, `instability`.

- [ ] **Step 3: Prove the graph has no duplicates**

Run: `cargo tree -d`
Expected: no `ratatui`, `crossterm` or `tui-input` line. If `crossterm v0.28` appears, `ratatui-crossterm` selected the wrong feature — stop and report rather than pinning the old version.

- [ ] **Step 4: Compile and record what 0.30 rejects**

Run: `cargo test --no-run`
Expected: `Alignment` still compiles through the compatibility alias; no other breakage. If there are errors, they must be limited to names renamed in 0.30 — report anything else before proceeding.

- [ ] **Step 5: Rename `Alignment` to `HorizontalAlignment`**

In the five files listed above, change the import and every use:

```rust
use ratatui::layout::{HorizontalAlignment, Constraint, Layout, Rect};
```

```rust
.alignment(HorizontalAlignment::Center)
.title_alignment(HorizontalAlignment::Left)
```

Do not add `#[allow(deprecated)]` and do not keep the old name. Do not touch `List::highlight_symbol` call sites: `String` already satisfies the new `Into<Line>` bound.

- [ ] **Step 6: Run the gates**

Run: `cargo test --locked 2>&1 | awk '/^test result/ {s+=$4} END {print s}'`
Expected: `194`.

Run: `cargo fmt --check`
Expected: no output.

Run: `cargo clippy --locked --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/app.rs src/ui/mod.rs src/ui/dialog.rs src/screens/app_theme_editor.rs src/screens/main_menu.rs
git commit -S -m "build(deps): move to ratatui 0.30, crossterm 0.29 and tui-input 0.15"
```

---

### Task 2: Migrate downloads to ureq 3

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Modify: `src/fonts/downloader.rs` (`download`, `download` tests)
- Test: `src/fonts/downloader.rs` (`mod tests`)

**Interfaces:**
- Consumes: `read_capped(reader: impl Read, announced_total: Option<u64>, on_progress: impl FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String>` — signature unchanged by this task.
- Produces: `pub fn download(url: &str, on_progress: impl FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String>` — signature unchanged; same `download failed: …` message prefix.

- [ ] **Step 1: Add a loopback-server helper and three failing-purpose tests**

In `src/fonts/downloader.rs`'s `mod tests`, add:

```rust
/// Serve exactly one HTTP/1.1 response on 127.0.0.1 and return its URL.
fn serve_once(status: &str, content_length: Option<usize>, body: &[u8], send_bytes: usize) -> String
```

It binds `TcpListener::bind("127.0.0.1:0")`, reads the request until `\r\n\r\n`, writes `HTTP/1.1 {status}\r\nConnection: close\r\n` then `Content-Length: {n}\r\n` when `content_length` is `Some`, then the body truncated to `send_bytes`, and drops the stream. The thread is spawned; the function returns `format!("http://{addr}/font.ttf")`.

```rust
#[test]
fn download_reads_a_body_and_reports_progress()
```

Asserts the returned bytes are `b"font"` and the collected progress is `vec![(4, Some(4))]`.

```rust
#[test]
fn download_reports_a_non_success_status()
```

Serves `"500 Internal Server Error"` with body `b"boom"`. Asserts the result is `Err` and the message starts with `"download failed:"`.

```rust
#[test]
fn download_reports_a_truncated_body()
```

Serves `content_length: Some(10)`, `body: b"0123456789"`, `send_bytes: 4`. Asserts the result is `Err` whose message starts with `"download failed:"`.

- [ ] **Step 2: Confirm they pass against the current ureq 2 implementation**

Run: `cargo test download_ -- --nocapture`
Expected: 3 passed. These are characterisation tests: they pin today's behaviour so the rewrite cannot regress it. If `download_reports_a_truncated_body` fails here, record the observed behaviour and keep the test asserting the error requirement from the spec — do not weaken it.

- [ ] **Step 3: Swap the dependency**

```toml
ureq = "3.5"
```

Remove `features = ["tls"]`; 3.x defaults to rustls with the ring provider. Run `cargo build` to refresh `Cargo.lock`; expected: `ureq 3.5.0`, `ureq-proto`, and no `native-tls`.

- [ ] **Step 4: Rewrite `download` against the 3.x API**

```rust
pub fn download(url: &str, on_progress: impl FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let total = response.body().content_length();
    read_capped(response.body_mut().as_reader(), total, on_progress)
}
```

If — and only if — `download_reports_a_truncated_body` shows that ureq returns a short body as `Ok`, add the explicit guard in `download` before returning: compare `bytes.len() as u64` with `total` when `total` is `Some`, and return `Err("download failed: truncated body".to_owned())` on a mismatch. A short body must never reach `install_bytes`.

- [ ] **Step 5: Run the gates**

Run: `cargo test download_ -- --nocapture`
Expected: 3 passed.

Run: `cargo test --locked 2>&1 | awk '/^test result/ {s+=$4} END {print s}'`
Expected: `197` (194 + 3).

Run: `cargo fmt --check && cargo clippy --locked --all-targets -- -D warnings`
Expected: both silent.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/fonts/downloader.rs
git commit -S -m "build(deps): migrate downloads to ureq 3"
```

---

### Task 3: Replace ttf-parser with skrifa

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Modify: `src/fonts/validate.rs:14`
- Modify: `src/fonts/scan.rs:21`
- Modify: `src/fonts/downloader.rs` (move the font fixture out of `mod tests`)
- Test: new `mod tests` at the end of `src/fonts/validate.rs`

**Interfaces:**
- Consumes: `crate::fonts::downloader::minimal_ttf() -> Vec<u8>` (new `#[cfg(test)] pub(crate) fn`).
- Produces: `pub fn is_valid_font_bytes(data: &[u8]) -> (bool, String)` — signature and message wording unchanged; `cannot parse font: {e}` now displays skrifa's `ReadError`.

- [ ] **Step 1: Share the font fixture**

In `src/fonts/downloader.rs`, move `minimal_ttf()` out of `mod tests` to module scope, guarded:

```rust
#[cfg(test)]
pub(crate) fn minimal_ttf() -> Vec<u8>
```

Keep its body byte-for-byte. Delete the copy inside `mod tests`; `use super::*` keeps its callers working.

Run: `cargo test --locked 2>&1 | awk '/^test result/ {s+=$4} END {print s}'`
Expected: `197`, unchanged.

- [ ] **Step 2: Write the failing tests in `src/fonts/validate.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::is_valid_font_bytes;
    use crate::fonts::downloader::minimal_ttf;

    #[test]
    fn rejects_data_too_small() // 3 bytes -> (false, "file is too small to be a font")

    #[test]
    fn rejects_an_unknown_signature() // b"wOFF...." -> (false, "unknown font signature: …")

    #[test]
    fn accepts_a_minimal_valid_font() // minimal_ttf() -> (true, "")

    #[test]
    fn rejects_a_valid_signature_with_a_corrupt_table_directory()
    // minimal_ttf() with bytes[4..6] set to 0xFFFF (numTables) -> (false, reason)
    // assert reason.starts_with("cannot parse font:")
}
```

Assert on the exact `(bool, String)` pairs described in the comments; for the signature case assert `reason.starts_with("unknown font signature:")`.

- [ ] **Step 3: Run them and watch the corrupt-directory case fail**

Run: `cargo test --lib fonts::validate`
Expected: `accepts_a_minimal_valid_font` passes under `ttf-parser`; `rejects_a_valid_signature_with_a_corrupt_table_directory` also passes under `ttf-parser`. The value of this step is that all four tests are green *before* the swap, so the only variable afterwards is the parser. If the corrupt-directory case is accepted by `ttf-parser`, record that as a pre-existing gap and keep the test: the requirement is skrifa rejecting it.

- [ ] **Step 4: Swap the parser**

```toml
skrifa = "0.49"
```

Remove `ttf-parser`. In `validate.rs`, replace
`match ttf_parser::Face::parse(data, 0)` with `match skrifa::FontRef::new(data)`.
In `scan.rs`, replace `ttf_parser::Face::parse(&data, 0)` with `skrifa::FontRef::new(&data)`.
Keep the four-magic pre-check and both message formats exactly as they are.

Run: `cargo build` to refresh `Cargo.lock`; expected: `skrifa 0.49.0` and `read-fonts` present, no `ttf-parser` anywhere in `cargo tree`.

- [ ] **Step 5: Run the gates**

Run: `cargo test --lib fonts::`
Expected: all font tests pass, including the four above and every `downloader` test that uses `minimal_ttf()`.

Run: `cargo test --locked 2>&1 | awk '/^test result/ {s+=$4} END {print s}'`
Expected: `201` (197 + 4).

Run: `cargo fmt --check && cargo clippy --locked --all-targets -- -D warnings && cargo tree -d`
Expected: silent, silent, and no `ratatui`/`crossterm`/`tui-input`/`ttf-parser` duplicates.

If `skrifa` rejects `minimal_ttf()` for a reason unrelated to the directory (for example a missing table it insists on), stop and report with the exact `ReadError`; do not relax validation and do not shrink the test set.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/fonts/validate.rs src/fonts/scan.rs src/fonts/downloader.rs
git commit -S -m "build(deps): replace the unmaintained ttf-parser with skrifa"
```

---

### Task 4: Cover the release tag gate

**Files:**
- Create: `tests/release_version.rs`
- Test: itself

**Interfaces:**
- Consumes: `scripts/check-release-version.sh <tag>`, which reads `Cargo.toml` from the current directory and exits `0` on an exact match, `1` otherwise.
- Produces: nothing other tasks consume.

- [ ] **Step 1: Write the test file**

`tests/release_version.rs` builds a temp dir with `std::env::temp_dir()` + `tempfile::tempdir()`, writes a `Cargo.toml`, and runs:

```rust
fn run_gate(tag: &str, cargo_toml: &str) -> std::process::Output
```

which runs `std::process::Command::new("sh")` with the script path
`concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/check-release-version.sh")` and `tag`, with `current_dir(dir.path())`.

Four tests:

```rust
#[test]
fn exact_version_matches()          // [package] version = "1.1.1", tag "v1.1.1" -> success

#[test]
fn prefix_tag_fails()               // version = "1.1.10", tag "v1.1.1" -> failure

#[test]
fn different_version_fails()        // version = "1.1.1", tag "v2.0.0" -> failure, stderr contains "does not match"

#[test]
fn dependency_version_is_ignored()  // [package] version = "1.2.3" + [dependencies.foo] version = "9.9.9", tag "v1.2.3" -> success
```

- [ ] **Step 2: Run the test**

Run: `cargo test --test release_version`
Expected: 4 passed. The script is already correct; this locks its behaviour, so a rewrite that breaks the string prefix or the `[package]` boundary fails here.

- [ ] **Step 3: Commit**

```bash
git add tests/release_version.rs
git commit -S -m "test(scripts): cover the release tag gate"
```

---

### Task 5: Derive the cap message and pre-size the buffer

**Files:**
- Modify: `src/fonts/downloader.rs` (`too_large`, `read_capped`, `mod tests`)

**Interfaces:**
- Consumes: `MAX_DOWNLOAD_BYTES: u64`, `read_capped` from Task 2.
- Produces: no signature change. `too_large()` keeps the exact text `download failed: file is larger than 20 MB`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn cap_message_is_derived_from_the_constant()
```

Asserts `too_large()` equals the literal
`"download failed: file is larger than 20 MB"` **and** contains
`(MAX_DOWNLOAD_BYTES / (1024 * 1024)).to_string()`. The literal pins the user-visible text; the second assertion pins that it is derived.

```rust
#[test]
fn read_capped_presizes_the_buffer_from_the_announced_total()
```

Reads `Cursor::new(b"font")` with `announced_total: Some(64)` and asserts `bytes.capacity() >= 64`.

- [ ] **Step 2: Run them and watch the presize test fail**

Run: `cargo test --lib cap_message read_capped_presizes`
Expected: `read_capped_presizes_the_buffer_from_the_announced_total` fails because the vector starts at capacity 0. `cap_message_is_derived_from_the_constant` passes both before and after — it exists to pin the user-visible text, not to drive the change.

- [ ] **Step 3: Implement both changes**

`too_large()` formats `MAX_DOWNLOAD_BYTES / (1024 * 1024)`. `read_capped` starts the accumulator with `Vec::with_capacity(announced_total.unwrap_or(0).min(MAX_DOWNLOAD_BYTES) as usize)`.

- [ ] **Step 4: Run the gates**

Run: `cargo test --lib fonts::downloader`
Expected: all pass, including the existing `read_capped_*` cases (under cap, exact cap, over-advertised, mid-stream overflow).

Run: `cargo fmt --check && cargo clippy --locked --all-targets -- -D warnings`
Expected: both silent.

- [ ] **Step 5: Commit**

```bash
git add src/fonts/downloader.rs
git commit -S -m "fix(fonts): derive the download cap message and pre-size the buffer"
```

---

### Task 6: Lock clippy and scope release permissions

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: workflow-level `permissions: contents: read` in `release.yml`; job-scoped write scopes on `android` only.

- [ ] **Step 1: Edit the CI clippy step**

Change `cargo clippy --all-targets -- -D warnings` to
`cargo clippy --locked --all-targets -- -D warnings`.

- [ ] **Step 2: Restructure `release.yml` permissions**

Replace the top-level block with:

```yaml
permissions:
  contents: read
```

Add to the `android` job only:

```yaml
    permissions:
      contents: write
      id-token: write
      attestations: write
```

`verify` gets no `permissions` key and therefore inherits read-only.

- [ ] **Step 3: Run the gates**

Run: `actionlint .github/workflows/*.yml`
Expected: no output.

Run: `cargo clippy --locked --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml
git commit -S -m "ci: lock clippy and scope release permissions"
```

---

### Task 7: Update the documentation

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `docs/structure.md`
- Modify: `docs/plans/2026-10-30-hardening-and-maintenance.md:148-155`
- Modify: `README.md`, `CONTRIBUTING.md` only if a stale claim is found

**Interfaces:**
- Consumes: the code changes from Tasks 1–6.
- Produces: documentation that matches the tree.

- [ ] **Step 1: Fix the catalog-verification snippet**

In `docs/plans/2026-10-30-hardening-and-maintenance.md`, the `sed` expects `")` but the grep stops at `"`, so every URL keeps a trailing quote. Replace the suffix expression with `s|"$||`:

```bash
grep -o 'format!("{NF_BASE}[^"]*"' src/fonts/downloader.rs \
  | sed 's|^format!("{NF_BASE}|https://github.com/ryanoasis/nerd-fonts/raw/v3.5.1/patched-fonts|; s|"$||' \
  | while read -r url; do
      code=$(curl -s -o /dev/null -w '%{http_code}' -L "$url")
      printf '%s %s\n' "$code" "$url"
    done
```

Verify the fix end-to-end once: run the loop and confirm the URLs no longer end in `"` (a `200` for every path is not required; this plan does not move the catalog).

- [ ] **Step 2: Update `CHANGELOG.md`**

Under `[Unreleased]`:

- `Changed`: the dependency modernization (`ratatui 0.30`, `crossterm 0.29`, `tui-input 0.15`, `ureq 3.5`, `skrifa` in place of the unmaintained `ttf-parser`), noting that nothing user-visible changed.
- `Changed`: the release workflow grants write permissions only to the job that publishes.
- `Removed`: the public `Paths::backups()` accessor; `Paths::backups_dir()` remains.
- Fix the stale `Added` bullet that still calls the MSRV `1.85.0`; it is `1.88.0`.

- [ ] **Step 3: Update `docs/structure.md`**

- Add `tests/release_version.rs` to the tree and describe it as the release tag gate test.
- Describe `src/fonts/validate.rs` and `src/fonts/scan.rs` as validating fonts through `skrifa` rather than `ttf-parser`.

- [ ] **Step 4: Check the remaining docs**

Run: `grep -rn "ttf-parser\|ttf_parser" README.md CONTRIBUTING.md SECURITY.md docs/`
Expected: only the historical spec/plan entries remain; no live claim about the dependency.

Run: `grep -rn "1\.85" README.md CONTRIBUTING.md CHANGELOG.md`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add CHANGELOG.md docs/structure.md docs/plans/2026-10-30-hardening-and-maintenance.md README.md CONTRIBUTING.md
git commit -S -m "docs: record the modernization and fix the catalog snippet"
```

---

### Task 8: Whole-branch verification

**Files:** none (verification only; fix in the owning task if anything fails).

**Interfaces:**
- Consumes: every task above.
- Produces: the evidence used to claim the phase is complete.

- [ ] **Step 1: Run the full gate set**

```bash
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
actionlint .github/workflows/*.yml
shellcheck scripts/*.sh
cargo tree -d
git log --format='%h %G? %s' main..HEAD
```

Expected: tests pass (207); fmt and clippy silent; actionlint and shellcheck silent; `cargo tree -d` lists no `ratatui`, `crossterm`, `tui-input` or `ttf-parser`; every commit line shows `G`.

- [ ] **Step 2: Manual TUI run on the device**

Run: `cargo run --locked`

Check, in order: main menu renders with borders and markers; Terminal Colors opens and a colour row edits; App Theme opens and a toggle flips; Fonts tab lists the library; Font Catalog lists entries; open a text field and type, move with Left/Right, delete with Backspace, submit; download one font from the catalog and confirm it appears in the library; quit and confirm the terminal is restored (cursor visible, normal screen).

Expected: no panic, no flicker beyond the redraw, the downloaded font is listed, and the shell is usable immediately after exit. This is the only end-to-end exercise of `ureq 3`; if the download fails, capture the TUI error line and fix it in Task 2, not here.

- [ ] **Step 3: Confirm the audit**

Run: `cargo audit` if installed, otherwise rely on the CI `audit` job.
Expected: no `ttf-parser` entry; the remaining informational advisories (`paste`, `lru`) are unchanged from phase 1.

- [ ] **Step 4: Report**

Record the commands and their output above, plus the manual checklist result, in the completion report for the branch. Do not claim completion from any step that was not actually run.
