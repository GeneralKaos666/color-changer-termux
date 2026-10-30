# Hardening and Maintenance Implementation Plan

> **Historical note:** this plan is kept as written. Its `1.85.0` MSRV figures
> were corrected to `1.88.0` during execution — the source already used
> let-chains, so `1.85.0` never actually built — and phase 2 later replaced
> `ttf-parser` with `skrifa`. See `CHANGELOG.md` and
> `docs/specs/2026-10-30-dependency-modernization-design.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the four phase‑1 gaps from the audit: unbounded font downloads, a silently permissive CLI, CI/release blind spots, and missing repository documentation.

**Architecture:** Six independent tasks on one branch. Two touch Rust code (downloads, CLI), one is a small refactor (`Paths`), two are GitHub workflow changes, one is documentation plus package metadata. Every Rust task is test-driven and verified with `cargo test`, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings`. Every workflow task is verified with `actionlint`.

**Tech Stack:** Rust 2024 (MSRV 1.85.0), ratatui 0.29, crossterm 0.28, ureq 2.12, ttf-parser 0.25; GitHub Actions; `actionlint`, `jq`, `curl`, `shellcheck` (all present locally).

**Spec:** `docs/specs/2026-10-30-hardening-and-maintenance-design.md`

## Global Constraints

- MSRV is `1.85.0` (`rust-version` in `Cargo.toml`); edition is 2024.
- Download cap is exactly `20 * 1024 * 1024` bytes (20 MB).
- Nerd Fonts catalog is pinned to tag `v3.5.1`.
- Exit codes: `0` success, `1` runtime error, `2` usage error.
- Unit tests must not touch the network.
- `cargo test`, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings` must pass before any task is called done.
- Conventional Commits, every commit GPG-signed with `-S`.
- No dependency version changes in this phase.

## Review Focus

- A font host that sends **no `Content-Length`**: the download must still succeed when the body is under the cap — only the streaming guard applies (Task 1).
- A body **exactly at the cap**: must be accepted, not rejected by an off-by-one (Task 1).
- `--help` combined with an **invalid flag**: help wins, exit `0`, no error (Task 2).
- A tag whose version is a **string prefix** of another (`v1.1.1` vs `v1.1.10`): the release gate must compare full strings and fail, not prefix-match (Task 5).
- A `Cargo.toml` with **more than one `version` key** (e.g. a dependency table): the gate must read only the `[package]` version (Task 5).

---

### Task 1: Bound font downloads

**Files:**
- Modify: `src/fonts/downloader.rs` (constant, new `read_capped`, refactor `download`, `NF_BASE`, tests)
- Test: inline `#[cfg(test)] mod tests` at the bottom of `src/fonts/downloader.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `const MAX_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;` (module-private)
  - `pub fn read_capped(reader: impl std::io::Read, announced_total: Option<u64>, on_progress: impl FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String>`
  - `download` keeps its current signature and delegates the body to `read_capped`.

- [ ] **Step 1: Write the failing tests**

Add to `src/fonts/downloader.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{MAX_DOWNLOAD_BYTES, NF_BASE, nerd_fonts_catalog, read_capped};
    use std::io::{Cursor, Read};

    #[test]
    fn read_capped_accepts_a_body_under_the_cap() {
        let mut progress = Vec::new();
        let bytes = read_capped(Cursor::new(b"font".to_vec()), Some(4), |received, total| {
            progress.push((received, total));
        })
        .unwrap();
        assert_eq!(bytes, b"font");
        assert_eq!(progress, vec![(4, Some(4))]);
    }

    #[test]
    fn read_capped_rejects_an_announced_size_over_the_cap() {
        let mut called = false;
        let result = read_capped(std::io::empty(), Some(MAX_DOWNLOAD_BYTES + 1), |_, _| {
            called = true
        });
        let message = result.unwrap_err();
        assert!(message.contains("larger than 20 MB"), "{message}");
        assert!(!called, "body must not be read when Content-Length is too large");
    }

    #[test]
    fn read_capped_rejects_a_stream_that_exceeds_the_cap() {
        let body = std::io::repeat(b'x').take(MAX_DOWNLOAD_BYTES + 1);
        assert!(read_capped(body, None, |_, _| {}).is_err());
    }

    #[test]
    fn read_capped_accepts_a_body_exactly_at_the_cap() {
        let body = std::io::repeat(b'x').take(MAX_DOWNLOAD_BYTES);
        assert_eq!(
            read_capped(body, None, |_, _| {}).unwrap().len() as u64,
            MAX_DOWNLOAD_BYTES
        );
    }

    #[test]
    fn catalog_is_pinned_to_v3_5_1_and_well_formed() {
        assert_eq!(
            NF_BASE,
            "https://github.com/ryanoasis/nerd-fonts/raw/v3.5.1/patched-fonts"
        );
        let catalog = nerd_fonts_catalog();
        assert!(!catalog.is_empty());
        for (name, url) in &catalog {
            assert!(!name.is_empty());
            assert!(url.starts_with(NF_BASE), "{name}: {url}");
            assert!(url.ends_with(".ttf"), "{name}: {url}");
        }
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --offline --lib fonts::downloader`
Expected: FAIL — `read_capped` and `MAX_DOWNLOAD_BYTES` not found.

- [ ] **Step 3: Implement `read_capped` and refactor `download`**

In `src/fonts/downloader.rs`, add the constant next to `DOWNLOAD_TIMEOUT` and the function next to `download`:

```rust
/// Largest font body the downloader will accept.
const MAX_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;

pub fn read_capped(
    reader: impl Read,
    announced_total: Option<u64>,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, String> {
    // Reject up front when the announced size is already too large.
    // While reading, count bytes and abort once the cap is exceeded.
    // Report progress after each chunk as (received, announced_total).
}
```

The body: return `Err(format!("download failed: file is larger than 20 MB"))` for both cap violations; map read errors to `format!("download failed: {e}")`; read in 8 KiB chunks; accept a body of exactly `MAX_DOWNLOAD_BYTES`.

Rewrite `download` to keep the agent, timeout, request, and `Content-Length` parsing, then `read_capped(response.into_reader(), total, on_progress)`.

- [ ] **Step 4: Run the cap tests to verify they pass**

Run: `cargo test --offline --lib read_capped`
Expected: 4 passed. (The catalog test still pins `v3.2.1` until Step 5.)

- [ ] **Step 5: Bump the catalog and verify every path resolves**

Change `NF_BASE` to `https://github.com/ryanoasis/nerd-fonts/raw/v3.5.1/patched-fonts`, then check every entry:

```bash
grep -o 'format!("{NF_BASE}[^"]*"' src/fonts/downloader.rs \
  | sed 's|^format!("{NF_BASE}|https://github.com/ryanoasis/nerd-fonts/raw/v3.5.1/patched-fonts|; s|"$||' \
  | while read -r url; do
      code=$(curl -s -o /dev/null -w '%{http_code}' -L "$url")
      printf '%s %s\n' "$code" "$url"
    done
```

Expected: every line starts with `200`. For any line that does not, correct or remove that catalog entry and re-run the loop. If more than a handful have moved, stop and report back rather than guessing paths.

- [ ] **Step 6: Run the full verification**

Run: `cargo test --offline && cargo fmt --check && cargo clippy --offline --all-targets -- -D warnings`
Expected: all tests pass; no formatting or lint diffs.

- [ ] **Step 7: Commit**

```bash
git add src/fonts/downloader.rs
git commit -S -m "fix(fonts): cap downloads at 20 MB and refresh the catalog"
git verify-commit HEAD
```

---

### Task 2: CLI help and strict parsing

**Files:**
- Modify: `src/cli.rs` (`Args`, `HELP`, `parse_from`, `run`)
- Modify: `tests/cli.rs` (new tests)
- Modify: `README.md` (CLI block and exit-code paragraph)

**Interfaces:**
- Consumes: `FontSlot::try_from(&str)` and `FontSlot::ALL` (existing).
- Produces:
  - `pub const HELP: &str`
  - `Args` gains `pub help: bool` (still `Debug + Default`)
  - `Args::parse_from` returns `Err(String)` for unknown options, unexpected positionals, and missing values.

- [ ] **Step 1: Write the failing tests**

Append to `tests/cli.rs`:

```rust
#[test]
fn help_wins_over_other_flags() {
    let args = Args::parse_from(["--version".into(), "--bogus".into(), "--help".into()]).unwrap();
    assert!(args.help);
    assert!(!args.version);
}

#[test]
fn short_help_alias_is_accepted() {
    assert!(Args::parse_from(["-h".into()]).unwrap().help);
}

#[test]
fn help_text_names_slots_version_and_exit_codes() {
    assert!(termux_settings::cli::HELP.contains(env!("CARGO_PKG_VERSION")));
    assert!(termux_settings::cli::HELP.contains("bold-italic"));
    assert!(termux_settings::cli::HELP.contains("Exit codes"));
}

#[test]
fn unknown_option_is_rejected() {
    let error = Args::parse_from(["--hlep".into()]).unwrap_err();
    assert!(error.contains("unknown option: --hlep"), "{error}");
}

#[test]
fn missing_flag_values_are_rejected() {
    for flag in ["--apply-font", "--slot"] {
        let error = Args::parse_from([flag.to_owned()]).unwrap_err();
        assert!(error.contains(flag), "{error}");
        assert!(error.contains("needs a value"), "{error}");
    }
}

#[test]
fn unexpected_positional_is_rejected() {
    let error = Args::parse_from(["foo".to_owned()]).unwrap_err();
    assert!(error.contains("unexpected argument: foo"), "{error}");
}

#[test]
fn help_run_returns_success() {
    let dir = tempfile::tempdir().unwrap();
    let args = Args {
        help: true,
        ..Default::default()
    };
    assert_eq!(
        run(args, &Paths::new(dir.path())).unwrap(),
        ExitCode::SUCCESS
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --offline --test cli`
Expected: FAIL — `Args` has no field `help`; `cli::HELP` not found.

- [ ] **Step 3: Implement the parser and help**

In `src/cli.rs`:

- Add `pub help: bool` to `Args`.
- Add the help text verbatim:

```rust
pub const HELP: &str = concat!(
    "termux-settings ", env!("CARGO_PKG_VERSION"), "\n",
    "A keyboard-driven TUI for Termux colours, app theme and fonts.\n\n",
    "Usage:\n  termux-settings [options]\n\n",
    "Options:\n",
    "  -h, --help              print this help and exit\n",
    "      --version           print the version and exit\n",
    "      --list-fonts        list fonts in ~/.termux/fonts/\n",
    "      --apply-font <name> install a font from the library\n",
    "      --slot <slot>       regular, bold, italic, bold-italic (default regular)\n",
    "      --ascii             plain-ASCII glyphs\n\n",
    "Environment:\n",
    "  TERMUX_SETTINGS_ASCII   same as --ascii when set to anything but 0\n",
    "  TERMUX_HOME             override ~/.termux (mainly for tests)\n\n",
    "Exit codes:\n  0  success\n  1  runtime error\n  2  usage error\n",
);
```
- In `parse_from`, collect the iterator into a `Vec<String>` first. If any element is `--help` or `-h`, return `Ok(Args { help: true, ..Default::default() })` immediately (after applying the ASCII environment), so help wins over every other flag.
- Otherwise parse strictly: `--apply-font` and `--slot` consume the next element or return `Err(format!("{flag} needs a value"))`; any other argument starting with `-` returns `Err(format!("unknown option: {arg}"))`; any other argument returns `Err(format!("unexpected argument: {arg}"))`.
- In `run`, handle `args.help` first: `println!("{HELP}")` and return `Ok(ExitCode::SUCCESS)`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --offline --test cli`
Expected: 10 passed (3 existing + 7 new).

- [ ] **Step 5: Update the README**

In the "Command line" block add `termux-settings --help                   # print usage` as the first line, and extend the exit-code paragraph to say that unknown options, unexpected arguments, and missing option values exit `2`.

- [ ] **Step 6: Run the full verification**

Run: `cargo test --offline && cargo fmt --check && cargo clippy --offline --all-targets -- -D warnings`
Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add src/cli.rs tests/cli.rs README.md
git commit -S -m "feat(cli): add --help and reject unknown options"
git verify-commit HEAD
```

---

### Task 3: Remove the duplicate `Paths` accessor

**Files:**
- Modify: `src/paths.rs` (remove `backups()`)
- Modify: `src/screens/app_theme_editor.rs:226`, `src/screens/terminal_editor.rs:322`
- Modify: `tests/paths.rs:17`, `tests/ui_editor_save.rs:25`

**Interfaces:**
- Consumes: existing `Paths::backups_dir()`.
- Produces: `backups_dir()` as the only backups accessor.

- [ ] **Step 1: Replace the call sites**

Change every `paths.backups()` / `p.backups()` to `paths.backups_dir()` / `p.backups_dir()`, then delete `Paths::backups()`.

- [ ] **Step 2: Verify no reference remains and the tests pass**

Run: `grep -rn "backups()" src tests; cargo test --offline`
Expected: `grep` prints nothing; all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/paths.rs src/screens/app_theme_editor.rs src/screens/terminal_editor.rs tests/paths.rs tests/ui_editor_save.rs
git commit -S -m "refactor(paths): drop the duplicate backups accessor"
git verify-commit HEAD
```

---

### Task 4: CI matrix, Android build, advisory scan, dependabot

**Files:**
- Modify: `.github/workflows/ci.yml`
- Create: `.github/dependabot.yml`

**Interfaces:**
- Consumes: the existing `Cargo.toml` MSRV (`1.85.0`) and release targets.
- Produces: job ids `test`, `android`, `audit` used by branch protection if configured later.

- [ ] **Step 1: Rewrite `.github/workflows/ci.yml`**

Keep `name: CI`, `env.CARGO_TERM_COLOR`, `push`, and `pull_request`; add `schedule: - cron: "0 6 * * 1"` (weekly advisory run). Then:

```yaml
jobs:
  test:
    if: github.event_name != 'schedule'
    strategy:
      fail-fast: false
      matrix:
        rust: ["1.85.0", stable]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --locked
      - if: matrix.rust == 'stable'
        run: cargo fmt --check
      - if: matrix.rust == 'stable'
        run: cargo clippy --all-targets -- -D warnings

  android:
    if: github.event_name != 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android,armv7-linux-androideabi
      - uses: nttld/setup-ndk@v1
        with:
          ndk-version: r26d
          link-to-system-path: true
      - uses: Swatinem/rust-cache@v2
      - run: cargo install cargo-ndk --version 4.1.2 --locked
      - run: cargo ndk -t arm64-v8a -t armeabi-v7a --platform 24 build --locked

  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

- [ ] **Step 2: Create `.github/dependabot.yml`**

`version: 2`; one `cargo` update entry for `/` and one `github-actions` entry for `/`, both `schedule.interval: weekly`, each with a `groups` entry that matches `*` so a week's updates arrive as one PR per ecosystem.

- [ ] **Step 3: Validate the workflows**

Run: `actionlint .github/workflows/ci.yml .github/dependabot.yml`
Expected: no output, exit `0`. (`actionlint` may warn that `dependabot.yml` is not a workflow; if so, run it on `ci.yml` only and validate `dependabot.yml` with `yq e '.' .github/dependabot.yml > /dev/null`.)

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml .github/dependabot.yml
git commit -S -m "ci: test the MSRV, build Android on PRs and scan advisories"
git verify-commit HEAD
```

---

### Task 5: Release version gate, pinned cargo-ndk, provenance

**Files:**
- Create: `scripts/check-release-version.sh` (mode 755)
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `Cargo.toml` `[package] version`.
- Produces: `scripts/check-release-version.sh <tag>` — exits `0` on an exact match, `1` otherwise.

- [ ] **Step 1: Write the script**

```sh
#!/bin/sh
# Fail unless the pushed tag exactly matches the [package] version in Cargo.toml.
set -eu
tag=${1:?usage: check-release-version.sh <tag>}
version=$(awk '
  /^\[package\]/ { in_package = 1; next }
  /^\[/ { in_package = 0 }
  in_package && /^version[[:space:]]*=/ {
    gsub(/^version[[:space:]]*=[[:space:]]*"|".*$/, "")
    print
    exit
  }
' Cargo.toml)
if [ "v$version" != "$tag" ]; then
  printf 'tag %s does not match crate version v%s\n' "$tag" "$version" >&2
  exit 1
fi
```

Make it executable and check it with `shellcheck scripts/check-release-version.sh`.

- [ ] **Step 2: Verify the gate behaves correctly**

```sh
sh scripts/check-release-version.sh "v$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"  # exit 0
sh scripts/check-release-version.sh v1.1.10  # exit 1 (prefix must not match)
sh scripts/check-release-version.sh v0.0.0   # exit 1
```

Expected: exit codes `0`, `1`, `1`; the failing runs print a mismatch message on stderr.

Then confirm the gate reads only the `[package]` version:

```sh
tmp=$(mktemp -d)
printf '[package]\nname = "x"\nversion = "2.0.0"\n\n[dependencies]\nfoo = { version = "9.9.9" }\n' > "$tmp/Cargo.toml"
(cd "$tmp" && sh "$OLDPWD/scripts/check-release-version.sh" v2.0.0)
status=$?
rm -rf "$tmp"
exit $status
```

Expected: exit `0` — a dependency's `version` key must not be picked up.

- [ ] **Step 3: Wire the gate and provenance into `.github/workflows/release.yml`**

- Add `id-token: write` and `attestations: write` to the existing `permissions` block (keep `contents: write`).
- Add a `verify` job that checks out, installs stable, caches, runs `sh scripts/check-release-version.sh "$GITHUB_REF_NAME"`, then `cargo test --locked`.
- Add `needs: verify` to the `android` job.
- Change the install step to `cargo install cargo-ndk --version 4.1.2 --locked` and the build to `cargo ndk -t arm64-v8a -t armeabi-v7a --platform 24 build --release --locked`.
- After the checksum step add:

```yaml
      - uses: actions/attest-build-provenance@v2
        continue-on-error: true
        with:
          subject-path: |
            termux-settings-aarch64-linux-android
            termux-settings-armv7-linux-androideabi
```

- [ ] **Step 4: Validate**

Run: `actionlint .github/workflows/release.yml`
Expected: no output, exit `0`.

- [ ] **Step 5: Commit**

```bash
git add scripts/check-release-version.sh .github/workflows/release.yml
git commit -S -m "ci(release): gate on tests and tag version, add provenance"
git verify-commit HEAD
```

---

### Task 6: Documentation and package metadata

**Files:**
- Create: `docs/structure.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`
- Modify: `README.md`, `Cargo.toml`

**Interfaces:**
- Consumes: the changes from Tasks 1–5 (they are the `Unreleased` entries).
- Produces: nothing code-facing.

- [ ] **Step 1: Write `docs/structure.md`**

Sections: a short intro, a tree covering `.github/`, `docs/{specs,plans}/`, `scripts/`, `src/{app,cli,library,paths,presets,runtime,terminal,theme,chrome}.rs`, `src/{fonts,model,persist,screens,ui}/`, and `tests/`; then a purpose table with one row per top-level file/directory. State that `docs/specs` holds designs, `docs/plans` holds implementation plans, and that new features go spec → plan → branch.

- [ ] **Step 2: Write `CHANGELOG.md`**

Keep a Changelog format, SemVer. Seed the released section with the date from `git log -1 --format=%ad --date=short v1.1.1` and the MIT-licence entry from `v1.1.1`. `[Unreleased]` must record, under the correct headings:

- Added: `--help`/`-h`; MSRV, Android and advisory CI jobs; release version gate and provenance; `docs/structure.md`, `CONTRIBUTING.md`, `SECURITY.md`.
- Changed: unknown options, unexpected arguments and missing option values are now usage errors (exit `2`); the Nerd Fonts catalog moved to v3.5.1.
- Fixed: font downloads are capped at 20 MB, including when `Content-Length` lies.

- [ ] **Step 3: Write `CONTRIBUTING.md`**

Sections: Requirements (Rust 1.85.0+, `pkg install rust`), Build, Test (including `TERMUX_HOME` as the test seam and the no-network rule for unit tests), Lint (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`), Commit conventions (Conventional Commits, signed commits), and Process (spec in `docs/specs`, plan in `docs/plans`, branch, PR).

- [ ] **Step 4: Write `SECURITY.md`**

Sections: Supported versions (latest release only), Reporting (GitHub private security advisories, no public issues), Scope (config writes and backups under `~/.termux`, font downloads, `termux.properties` editing), and What is out of scope (Termux app itself, upstream Nerd Fonts artifacts).

- [ ] **Step 5: Add README badges and MSRV note**

Directly under the `# termux-settings` heading add:

```markdown
[![CI](https://github.com/GeneralKaos666/termux-settings/actions/workflows/ci.yml/badge.svg)](https://github.com/GeneralKaos666/termux-settings/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/GeneralKaos666/termux-settings)](https://github.com/GeneralKaos666/termux-settings/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
```

In "From source" and "Development", state that Rust 1.85.0 or newer is required.

- [ ] **Step 6: Complete the package metadata in `Cargo.toml`**

Under `[package]`, alongside the existing keys, add:

```toml
description = "A keyboard-driven TUI for Termux terminal colours, app theme and fonts"
repository = "https://github.com/GeneralKaos666/termux-settings"
readme = "README.md"
keywords = ["termux", "tui", "terminal", "colors", "fonts"]
categories = ["command-line-utilities", "config"]
```

- [ ] **Step 7: Verify**

```sh
cargo metadata --no-deps --format-version 1 | jq -e '.packages[0].description != null'
cargo test --offline && cargo fmt --check && cargo clippy --offline --all-targets -- -D warnings
```

Expected: `true`; all tests and lints pass.

- [ ] **Step 8: Commit**

```bash
git add docs/structure.md CHANGELOG.md CONTRIBUTING.md SECURITY.md README.md Cargo.toml
git commit -S -m "docs: add structure map, changelog, contributing and security policy"
git verify-commit HEAD
```

---

## Self-review notes

- **Spec coverage:** download cap and catalog (Task 1), CLI help and strict parsing (Task 2), `Paths` cleanup (Task 3), CI matrix/Android/audit/dependabot (Task 4), release gate/pinning/provenance (Task 5), all documentation and metadata (Task 6). Every acceptance criterion maps to a step.
- **Deferred:** phase 2 dependency upgrades, pinned hashes, theme CLI parity, `clap`, coverage floor — all explicitly out of scope in the spec.
- **Type consistency:** `read_capped`'s signature is used verbatim in Task 1's tests and in the `download` refactor; `Args::help` is used by both the parser tests and `run`.
