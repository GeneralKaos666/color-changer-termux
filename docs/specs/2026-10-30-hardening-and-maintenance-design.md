# Hardening and maintenance: design

- **Status:** approved (design)
- **Date:** 2026-10-30
- **Scope:** phase 1 of a two-phase maintenance pass on `termux-settings`
- **Applies to:** version `1.1.1` and later

## Context

`termux-settings` is a keyboard-driven TUI that edits Termux colour, app-theme
and font configuration. It runs as the user, writes to `~/.termux`, and
downloads fonts over the network.

A repository audit found the codebase in good shape — clean module boundaries,
182 tests, atomic writes, symlink-safe restores, CI running `fmt`, `clippy` and
`test` — with a small set of concrete gaps:

1. Font downloads are read into an unbounded buffer; only a 60-second timeout
   protects the process.
2. The CLI has no `--help` and silently ignores unknown flags, so a typo opens
   the full-screen TUI instead of reporting a mistake.
3. CI never builds for the Android targets it releases, never checks the
   declared minimum Rust version, and never audits dependencies. The release
   workflow does not run tests or verify that the tag matches the crate
   version.
4. Repository documentation is incomplete: no structure map, changelog,
   contribution guide, security policy, or package metadata.

Separately, five direct dependencies are behind their current releases. Four of
those are breaking upgrades, so they are handled in phase 2 rather than mixed
into this work.

## Goals

- A hostile or broken font host cannot exhaust memory on the device.
- A mistyped command line fails loudly, with exit code `2`, instead of opening
  the TUI.
- CI catches target-specific breakage, MSRV breakage, and known vulnerable
  dependencies before a tag is pushed.
- A release cannot publish a binary whose version disagrees with the tag.
- A contributor can find the project layout, the change log, the contribution
  rules, and the security-reporting route from the repository root.

## Non-goals

- Changing the on-disk formats or `colors.sh` interoperability.
- Any new screen, editor, or theme feature.
- Pinned SHA-256 checksums for downloads (see "Rejected alternatives").
- Dependency modernization; that is phase 2.
- Replacing the hand-written argument parser with `clap`.

## Decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| Download integrity | Size cap only | TLS plus `ttf-parser` structural validation covers accidental corruption; pinning hashes for ~50 fonts adds a maintenance burden and a large one-time fetch for marginal gain. |
| CLI scope | `--help` and strict parsing only | Fixes the silent-failure class without adding scriptable theme commands in the same change. |
| CI strictness | Pragmatic | MSRV, audit, Android build and release gates; no license allowlist or coverage floor, which tend to block on noise rather than risk. |
| Dependency work | Separate branch and plan | A bad `ratatui` migration must be revertible without reverting the hardening work. |
| Catalog version | `v3.5.1` | Upstream current release; see the verification requirement below. |

## Design

### 1. Bounded font downloads

Files: `src/fonts/downloader.rs`.

- Add `const MAX_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;`. A Nerd Font patched
  TTF is typically 2–10 MB, so 20 MB leaves room without allowing unbounded
  growth.
- Add `read_capped(reader, announced_total, on_progress) -> Result<Vec<u8>, String>`:
  - When `announced_total` exceeds the cap, return an error immediately,
    without reading the body: `download failed: file is larger than 20 MB`.
  - Otherwise read in 8 KiB chunks, calling `on_progress(received, total)` after
    each chunk, and return an error as soon as `received` would exceed the cap.
    This catches a host that omits or understates `Content-Length`.
- `download(url, on_progress)` keeps the ureq agent, the 60-second timeout, the
  request, and `Content-Length` parsing, then delegates the body to
  `read_capped`.
- The worker boundary keeps `Result<Vec<u8>, String>`; no new error enum is
  introduced.
- Bump `NF_BASE` to `https://github.com/ryanoasis/nerd-fonts/raw/v3.5.1/patched-fonts`.
  Upstream renames and moves patched files between releases, so every catalog
  path must be checked against the tag before the bump is committed. Paths
  that no longer resolve are corrected or removed; the tag is not bumped
  partially.
- Add an offline invariant test asserting that every catalog value starts with
  `NF_BASE`, ends with `.ttf`, and that keys are unique.

### 2. CLI help and strict parsing

Files: `src/cli.rs`, `tests/cli.rs`, `README.md`.

- `--help` and `-h` print a usage block covering the flags, the four font
  slots, the `TERMUX_SETTINGS_ASCII` override, and the exit codes, then exit
  `0`. Help takes precedence over every other flag, including `--version`.
- Parsing errors, each printed as `error: <message>` on stderr with exit `2`:
  - unknown option: `unknown option: --hlep`
  - missing value: `--apply-font needs a value`
  - unexpected positional: `unexpected argument: foo`
- `--version` keeps printing `termux-settings <version>` and exiting `0`.
- Existing successful paths and exit codes are unchanged: `--apply-font` with
  an unknown name exits `2`; other runtime failures exit `1`.
- This is a deliberate breaking change for anyone passing flags the parser did
  not previously implement; it is recorded in the changelog.

### 3. CI and release

Files: `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
`.github/dependabot.yml`.

CI (`push`, `pull_request`):

- `test` matrix over `1.88.0` (the declared MSRV) and `stable`, each running
  `cargo test --locked`. `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` run on `stable` only, keeping the
  MSRV job focused on compilation and tests.
- `android` job: `cargo ndk -t arm64-v8a -t armeabi-v7a --platform 24 build --locked`
  in debug mode. This links a real binary for both release ABIs without paying
  the release-profile cost on every pull request.
- `audit` job: `rustsec/audit-check` on pull requests plus a weekly cron.

Release (`v*` tags):

- A new `verify` job runs `cargo test --locked` and asserts that
  `v<crate version>` equals the pushed tag; the Android job `needs: verify`.
- `cargo-ndk` is installed at a pinned version (`4.1.2`) with `--locked`, and
  the build itself uses `--locked`.
- `actions/attest-build-provenance@v2` attests both release binaries, with
  `id-token: write` and `attestations: write` on the job.
- The NDK stays at r26d: a working, reproducible choice, and unrelated to this
  change.

Dependabot: weekly `cargo` and `github-actions` updates, grouped by ecosystem.

### 4. Documentation and package metadata

- `docs/structure.md`: directory map and purpose table, including how
  `docs/specs` and `docs/plans` relate to changes.
- `CHANGELOG.md`: Keep a Changelog format with `Unreleased` seeded from `1.1.1`,
  recording the download cap, strict CLI parsing, `--help`, and the CI gates.
- `CONTRIBUTING.md`: build and test commands, MSRV, lint commands, the
  `TERMUX_HOME` test seam, commit conventions, and the spec → plan → PR flow.
- `SECURITY.md`: supported versions, private reporting through GitHub security
  advisories, and the in-scope surface (config writes, backups, font
  downloads).
- `README.md`: CI, release and licence badges; `--help` in the CLI block; the
  minimum supported Rust version.
- `Cargo.toml`: `description`, `repository`, `readme`, `keywords` and
  `categories`.
- `src/paths.rs`: remove the duplicated `backups()` / `backups_dir()` pair,
  keeping `backups_dir()` and updating the three `backups()` call sites.

## Testing

- Unit tests for `read_capped`: body under the cap, rejection from
  `Content-Length`, rejection mid-stream when the header lies, and progress
  callbacks receiving monotonic byte counts.
- Catalog invariant test, offline.
- CLI tests: help text, `-h` alias, unknown option, missing value, unexpected
  positional, and the existing unknown-slot, unknown-font and unreadable-library
  cases.
- No test requires the network.
- Each change is verified with `cargo test`, `cargo fmt --check`, and
  `cargo clippy --all-targets -- -D warnings` before completion is claimed.

## Acceptance criteria

1. A download advertising more than 20 MB fails before body bytes are read; a
   body that grows past 20 MB fails while streaming.
2. `termux-settings --help` prints usage and exits `0`; `--hlep` prints an
   error and exits `2`; `--apply-font` without a value prints an error and
   exits `2`.
3. Every Nerd Fonts catalog path resolves at the tag the catalog is pinned to
   (`v3.5.1`, unless the bump is dropped because upstream renames make it
   infeasible).
4. CI runs tests on `1.88.0` and `stable`, builds both Android ABIs on pull
   requests, and runs an advisory scan.
5. A release tag whose version differs from `Cargo.toml` fails before building
   or publishing.
6. The repository contains `docs/structure.md`, `CHANGELOG.md`,
   `CONTRIBUTING.md`, `SECURITY.md`, README badges, and complete package
   metadata.
7. `cargo test`, `cargo fmt --check` and
   `cargo clippy --all-targets -- -D warnings` all pass.

## Risks

- **Catalog bump breaks downloads.** Mitigated by resolving every path against
  `v3.5.1` before committing and by keeping the base/extension invariant test.
  If many paths moved, the bump is dropped from this phase and tracked
  separately.
- **MSRV job fails on `1.85.0`.** The declared `rust-version` may already be
  wrong. In that case the job is the finding: raise `rust-version` to the
  actual minimum and record it in the changelog.
  *Outcome:* the job failed. Two `if ... && let` chains in the source need
  Rust 1.88, and the `time` patch for RUSTSEC-2026-0009 (`0.3.47`) also
  requires 1.88, so the MSRV was raised to `1.88.0` rather than the
  let-chains being rewritten.
- **Android job slows pull requests.** Mitigated with the existing Rust cache
  and a debug build; if it proves too slow, it can fall back to
  `cargo ndk ... check`.
- **Attestation permissions.** If the default `GITHUB_TOKEN` cannot attest,
  the step is marked `continue-on-error` rather than blocking a release.

## Rejected alternatives

- **Pinned SHA-256 for every font.** Stronger supply-chain guarantee, but
  requires fetching the whole catalog once and refreshing hashes whenever the
  catalog moves, for a threat already partly covered by TLS and structural
  validation.
- **`clap` for argument parsing.** Generated help and completions, but adds a
  dependency and a meaningful amount of binary size to a stripped release
  build for a five-flag CLI.
- **`cargo-deny` with a licence allowlist and coverage floor.** More policy
  than this repository currently sustains; error-prone to keep green.
- **Bundling dependency upgrades with this work.** A `ratatui` migration
  touches most screens and would obscure review of the hardening changes.

## Phase 2 (separate spec and plan)

Two workstreams on their own branch, each with its own spec and plan.

**Dependency modernization:** `ratatui 0.29 → 0.30`, `crossterm 0.28 → 0.29`,
`tui-input 0.11 → 0.15`, `ureq 2.12 → 3.5`, `ttf-parser 0.25 → 0.25.1`, verified
by the full suite plus a manual TUI run.

**Review follow-ups deferred from phase 1** (found in the phase‑1 whole-branch
review, graded Minor, none blocking):

- An automated regression test for `scripts/check-release-version.sh` covering
  exact match, a string prefix (`v1.1.1` vs `v1.1.10`), a mismatch, and a
  `Cargo.toml` whose only other `version` key belongs to a dependency.
- Derive the "20 MB" in `read_capped`'s error message from
  `MAX_DOWNLOAD_BYTES` so the two cannot drift apart.
- Pass `--locked` to the clippy step in CI so linting covers the same
  dependency graph the release builds.
- Split `permissions` in `release.yml`: `contents: read` on `verify`, with the
  write scopes scoped to the `android` job that needs them.
- Fix the catalog-verification snippet in the phase‑1 plan (its `sed` keeps a
  trailing quote, so every URL reports `404`).
- Record the removal of the public `Paths::backups()` accessor in
  `CHANGELOG.md`.
- Pre-size the download buffer with
  `Vec::with_capacity(min(announced_total, MAX_DOWNLOAD_BYTES))` to avoid
  reallocations on a large body.

**Superseded in part:** phase 2 is specified in
`docs/specs/2026-10-30-dependency-modernization-design.md`. That spec replaces
`ttf-parser 0.25.1` with `skrifa 0.49` (upstream is unmaintained) and keeps the
MSRV at `1.88`; every other item above is carried forward unchanged.

