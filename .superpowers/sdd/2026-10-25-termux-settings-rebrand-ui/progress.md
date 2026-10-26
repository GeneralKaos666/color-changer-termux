# SDD ledger — plan: docs/plans/2026-10-25-termux-settings-rebrand-ui.md

Execution: Native (inline). Branch: feat/termux-settings (off main).

Pre-flight scan (shared interfaces):
- Task 3 Produces `chrome::{accent, gradient_title, help_bar}` and `glyphs::*`; Tasks 4–11 Consume them. Signatures match (verified against plan text). Clean.
- Task 4 Produces `Screen::help()` + per-screen `help()`; no later task re-declares the signature differently. Clean.
- Task 11 deletes `ColorsTab`/`AppThemeTab`; Task 1 does not touch them. Clean.

Pre-flight: no shared-interface conflicts.
