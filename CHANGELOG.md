# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--help` and `-h`, printing usage, environment variables and exit codes.
- CI runs the test suite on the MSRV (`1.85.0`) as well as stable, builds the
  Android binaries on every pull request, and scans for RustSec advisories on a
  weekly schedule.
- `dependabot` opens grouped weekly updates for Cargo dependencies and GitHub
  Actions.
- Release tags are gated: the pushed tag must match the crate version, the test
  suite must pass, and the binaries get a build provenance attestation.
- `docs/structure.md`, `CONTRIBUTING.md` and `SECURITY.md`.

### Changed

- The minimum supported Rust version is now `1.88.0`. The previous `1.85.0`
  claim was wrong — the source already used let-chains — and the patched `time`
  release needs 1.88 as well.
- Unknown options, unexpected arguments and options missing their value are now
  usage errors and exit `2` instead of being silently ignored.
- The Nerd Fonts catalog points at upstream `v3.5.1`, with every path
  re-resolved against that tag after upstream flattened its variant directories.

### Fixed

- Font downloads are capped at 20 MB, including when the server omits or
  understates `Content-Length`.
- `time` is updated past RUSTSEC-2026-0009 (denial of service via stack
  exhaustion in RFC 2822 parsing).

## [1.1.1] - 2026-10-26

### Added

- MIT licence.

[Unreleased]: https://github.com/GeneralKaos666/termux-settings/compare/v1.1.1...HEAD
[1.1.1]: https://github.com/GeneralKaos666/termux-settings/releases/tag/v1.1.1
