# Dependency modernization: design

- **Status:** proposed (design)
- **Date:** 2026-10-30
- **Scope:** phase 2 of the two-phase maintenance pass on `termux-settings`
- **Applies to:** version `1.1.1` and later
- **Follows:** `docs/specs/2026-10-30-hardening-and-maintenance-design.md`

## Context

Phase 1 hardened downloads and the CLI, rebuilt CI and the release gate, and
added the repository documentation (merged in PR #2, branch
`chore/hardening-and-maintenance`, MSRV `1.88`). It deliberately left two things
behind:

1. Five direct dependencies are behind their current releases. Three of them
   move together, one is a ground-up major rewrite, and one has no successor
   release because upstream stopped.
2. Its whole-branch review found seven minor issues that were recorded but not
   fixed.

Measured against the registries on 2026-10-30:

| Crate | Current | Latest | Latest MSRV |
| --- | --- | --- | --- |
| `ratatui` | 0.29 | 0.30.2 | 1.88.0 |
| `crossterm` | 0.28 | 0.29.0 | 1.63.0 |
| `tui-input` | 0.11 | 0.15.5 | not declared |
| `ureq` | 2.12 | 3.5.0 | 1.85 |
| `ttf-parser` | 0.25 | 0.25.1 | 1.63.0 |
| `skrifa` (replacement) | — | 0.49.0 | 1.85 |

`ttf-parser` is unmaintained (RUSTSEC-2026-0192) and the advisory names
`skrifa` as the replacement. It runs on bytes downloaded from the network, so
it is the one dependency where being unmaintained is a security concern rather
than a housekeeping one.

## Goals

- Every direct dependency is on a maintained release.
- No unmaintained crate parses attacker-controlled bytes.
- The declared MSRV stays at `1.88`, which the new graph still satisfies.
- A single ratatui/crossterm/tui-input version of each crate is in the tree.
- The seven deferred review minors are closed.
- Behaviour is unchanged: same screens, same key handling, same error strings
  visible to users, same on-disk formats.

## Non-goals

- New features, screens, editors, themes, or CLI flags.
- Adopting `ratatui::init()` / `ratatui::restore()` or a different terminal
  lifecycle.
- Replacing the hand-written argument parser with `clap`.
- Rewriting the HTTP client on a runtime-based stack (`reqwest`).
- Changing the download cap, the catalog, or the release process.
- Adding a coverage floor, `cargo-deny`, or a shell-test framework.

## Decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| `ratatui`, `crossterm`, `tui-input` | Bump together, in one commit | `tui-input 0.15` requires `ratatui ^0.30.2`; `ratatui-crossterm` 0.1.2 defaults to `crossterm 0.29`. Split apart, the tree would hold two `ratatui` or two `crossterm` copies, and `crossterm::event` types would stop matching the backend's. |
| `ttf-parser` | Replace with `skrifa 0.49` | Unmaintained parser in the validation path for untrusted bytes. `skrifa` is the named successor, is maintained, and its `FontRef::new` is the direct equivalent of `Face::parse`. |
| MSRV | Stays `1.88` | `ratatui 0.30.2` requires exactly 1.88; `ureq` and `skrifa` need 1.85; `crossterm` needs 1.63. Phase 1's MSRV work needs no revision. |
| `ureq` TLS | Default features (`rustls` + `ring`) | The 2.x `tls` feature no longer exists. 3.x defaults to rustls with the ring provider, which is the backend already shipped, so no new native dependency appears on Android. |
| `layout::Alignment` | Rename imports to `HorizontalAlignment` | A compatibility type alias exists, but relying on it means the next rename is a silent break. Renaming 5 import sites is cheap now. |
| Release-gate test | Rust integration test in `tests/` | Runs on both CI legs through `cargo test --locked`, needs no new workflow step, and keeps the test next to the other integration tests. |
| Phase-1 plan snippet | Fix in place | The plan is a historical document, but a snippet a reader will copy must not lie. |

## Design

### 1. `ratatui 0.30`, `crossterm 0.29`, `tui-input 0.15`

Files: `Cargo.toml`, `src/app.rs`, `src/ui/mod.rs`, `src/ui/dialog.rs`,
`src/screens/{app_theme_editor,main_menu}.rs`.

- `Cargo.toml`: `ratatui = "0.30"`, `crossterm = "0.29"`, `tui-input = "0.15"`.
  Default features stay as they are; `ratatui`'s default features keep the
  crossterm backend, and `tui-input`'s default feature is `ratatui/crossterm`.
- Rename `ratatui::layout::Alignment` to `ratatui::layout::HorizontalAlignment`
  at all import and use sites. `Block::title_alignment` and
  `Paragraph::alignment` accept it unchanged.
- `List::highlight_symbol` now takes `Into<Line>`; the four call sites pass a
  `String` from `ui.glyphs.marker()`, which already satisfies it. No change.
- The application owns no custom `Backend` implementation and never names
  `Backend::Error`, so the new associated error type requires no code change.
  `ratatui::backend::CrosstermBackend` stays the concrete backend.
- No use of the other 0.30 removals: no `block::Title`, no exhaustive match on
  `Marker`, no `From` conversion between crossterm and ratatui types, no
  `Frame::size`, no `Flex`.
- `tui-input 0.15`: `Input`, `InputRequest`, `handle` and `visual_scroll` are
  unchanged; `src/ui/input.rs` should compile as is. New `InputRequest`
  variants are additive and are not adopted.
- `crossterm 0.29`: the only breaking change is the `KeyModifiers` `Display`
  format, which the application never renders.

### 2. `ureq 3.5`

Files: `Cargo.toml`, `src/fonts/downloader.rs`.

- `Cargo.toml`: `ureq = "3.5"` with default features; the `tls` feature is
  removed because it no longer exists.
- `download(url, on_progress) -> Result<Vec<u8>, String>` keeps its signature
  and its observable behaviour:
  - one agent per call, with a 60-second overall timeout (`DOWNLOAD_TIMEOUT`);
  - `Content-Length` from the response headers becomes `announced_total`,
    `None` when absent or unparseable;
  - the body is streamed through `read_capped`, which remains the sole
    authority on the 20 MB cap;
  - any transport or status error is flattened into the existing
    `download failed: {e}` string. No new error type crosses the worker
    boundary.
- Exact builder and reader calls (`Config`/`Agent` construction, response body
  access) are settled against the `ureq 3.5.0` documentation during
  implementation. The spec fixes behaviour, not call shapes.

### 3. `ttf-parser` → `skrifa`

Files: `Cargo.toml`, `src/fonts/validate.rs`, `src/fonts/scan.rs`.

- `Cargo.toml`: remove `ttf-parser`, add `skrifa = "0.49"`.
- `validate.rs`: replace `ttf_parser::Face::parse(data, 0)` with
  `skrifa::FontRef::new(data)`. The four-magic pre-check (`0x00010000`,
  `OTTO`, `true`, `typ1`) and the user-visible reasons stay:
  `unknown font signature: …` and `cannot parse font: {e}`.
- `scan.rs`: same replacement in the check that decides whether a library file
  is a usable font.
- The test fixtures decide whether the swap is behaviour-preserving. If
  `skrifa` rejects the synthetic 156-byte TTF the tests build, the fixture is
  replaced with a real minimal font; validation is not loosened to accommodate
  it.

### 4. Deferred review minors

1. **Release-gate regression test.** New `tests/release_version.rs` writes a
   `Cargo.toml` into a temporary directory and runs
   `sh scripts/check-release-version.sh <tag>` with that directory as `cwd`.
   Cases: exact match succeeds; `v1.1.1` against `1.1.10` fails; an unrelated
   tag fails; a dependency's `version` key outside `[package]` is ignored.
   The script path comes from `CARGO_MANIFEST_DIR`.
2. **Derived cap message.** `too_large()` formats the limit from
   `MAX_DOWNLOAD_BYTES`, so the message and the cap cannot drift. The rendered
   message stays `download failed: file is larger than 20 MB`; a test asserts
   the derived number appears.
3. **Locked clippy.** CI runs `cargo clippy --locked --all-targets -- -D warnings`
   so linting resolves the same graph as the release build.
4. **Release permissions.** `release.yml` declares `permissions: contents: read`
   at workflow level. `contents: write`, `id-token: write` and
   `attestations: write` move onto the `android` job, the only job that
   attests or publishes. `verify` keeps read-only.
5. **Catalog-verification snippet.** The `curl`/`sed` loop in
   `docs/plans/2026-10-30-hardening-and-maintenance.md` drops its trailing
   quote so URLs resolve instead of returning 404 for every entry.
6. **`Paths::backups()` removal.** `CHANGELOG.md` records the removal of the
   public accessor (kept `backups_dir()`) under `Unreleased`, so the interface
   change is documented like the CLI change was.
7. **Pre-sized buffer.** `read_capped` starts with
   `Vec::with_capacity(min(announced_total, MAX_DOWNLOAD_BYTES))` when the
   header is present, avoiding repeated reallocation on a multi-megabyte body.

### 5. Documentation

- `docs/structure.md`: add `tests/release_version.rs`, and describe
  `fonts/validate.rs` and `fonts/scan.rs` as using `skrifa`.
- `CHANGELOG.md`: `Unreleased` records the dependency modernization (no
  user-visible behaviour change), the `Paths::backups()` removal, and the CI
  permission narrowing.
- `README.md` / `CONTRIBUTING.md`: MSRV stays `1.88`; no edit expected. If any
  wording mentions `ttf-parser`, it is updated.

## Testing

- The existing suite (194 tests) passes unchanged apart from the renames.
- New `tests/release_version.rs` covers the four cases above; it must pass on
  both the `1.88` and `stable` CI legs, and locally in Termux.
- A unit test asserts the download-cap message is derived from
  `MAX_DOWNLOAD_BYTES`.
- Existing `read_capped` tests keep covering under-cap, over-advertised,
  mid-stream overflow and progress monotonicity.
- Font validation tests keep covering valid, wrong-magic, truncated and
  empty inputs under `skrifa`.
- `cargo tree -d` shows no duplicate `ratatui`, `crossterm`, `tui-input` or
  `ttf-parser`/`skrifa` pair.
- One manual TUI run on the device: open every screen, type in a field, move
  the cursor, open the font catalog, download one font (the only end-to-end
  exercise of `ureq 3`), and quit with the terminal restored.
- No automated test requires the network.

## Acceptance criteria

1. `cargo test --locked` passes, including `tests/release_version.rs`.
2. `cargo fmt --check` and
   `cargo clippy --locked --all-targets -- -D warnings` pass.
3. `actionlint .github/workflows/*.yml` and `shellcheck scripts/*.sh` pass.
4. CI is green on `1.88.0` and `stable`, with no duplicate `ratatui` or
   `crossterm` in the resolved graph.
5. The advisory scan no longer reports `ttf-parser`; no new advisory appears.
6. `release.yml` grants no write permission to `verify`, and the release job
   still attests and publishes.
7. The download error message for an oversized body is identical to today's
   and is derived from the constant.
8. `CHANGELOG.md` records the `backups()` removal; `docs/structure.md` matches
   the tree and the new parser.
9. A manual TUI run, including one catalog download, works and leaves the
   terminal in a normal state.

## Risks

- **`ureq 3` call shapes differ from expectation.** This is the one area not
  quoted from a verified signature. The behavioural requirements above are
  stable, so implementation only has to find the right builder calls; if the
  body API cannot stream through our reader, `Body::with_config().limit()`
  becomes the enforcement point and `read_capped` keeps receiving a `Read`.
- **`skrifa` strictness versus the synthetic fixture.** It may accept less (or
  more) than `ttf-parser` on the 156-byte test font. Mitigated by running the
  existing invalid-font tests first and replacing the fixture with a real
  minimal font if needed rather than weakening validation.
- **Hidden 0.30 rendering change.** `Flex::SpaceAround` semantics changed, but
  the application uses no `Flex`; `Alignment` is renamed with an alias. The
  manual run covers the layout risk.
- **Two `crossterm` versions.** If `ratatui`'s `crossterm_0_28` feature were
  selected anywhere, `crossterm::event` types would diverge. Mitigated by the
  single-version bump and the `cargo tree -d` check in the acceptance criteria.
- **Android build regressions from new TLS/parse dependencies.** `ureq 3` still
  uses ring-based rustls and `skrifa` is pure Rust, but the pull-request
  `android` job is the gate; a failure there blocks the phase rather than being
  skipped.

## Rejected alternatives

- **Keep `ttf-parser 0.25.1`.** Smallest diff and CI stays green because the
  advisory is informational, but it leaves an unmaintained parser on untrusted
  bytes and defers the same work to the next phase.
- **Pin `crossterm 0.28` and enable `ratatui`'s `crossterm_0_28` feature.**
  Possible, but it keeps a compatibility shim alive for no benefit and makes the
  next crossterm bump the same work again.
- **`reqwest` for the HTTP rewrite.** Brings a runtime and a much larger graph
  to a 20 MB download on a phone.
- **`assert_cmd`/`trycmd` for the script test.** A dependency and a new
  convention for one four-case test that `std::process::Command` already
  covers.
- **`ratatui 0.31`/git.** Not a published stable target for this crate's
  dependency policy.
