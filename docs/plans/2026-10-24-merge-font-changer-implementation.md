# Merge font-changer into termux-colors - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task.

**Goal:** Fold the standalone Go `font-changer` application into this Rust `termux-colors` app so that one binary changes terminal colors, the Termux app theme, and the terminal font, through a tabbed interface.

**Architecture:** Replace the screen stack with a tab bar (`Colors`, `Fonts`, `App Theme`). Port all font-changer functionality (scan, validate, apply, import, downloader, prompt capture) to Rust. Make event loop event-driven (no unconditional 250ms redraw). Cache palette and colors in App. Keep all existing functionality working.

**Tech Stack:** Rust 2024, ratatui 0.29, crossterm 0.28, tui-input 0.11, time 0.3, tempfile 3 (dev), ttf-parser (new), ureq with rustls (new)

---

## Task 1: Update dependencies and add CLI argument parsing

**Files:**
- Modify: `Cargo.toml`
- Create: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

**Step 1: Add dependencies to Cargo.toml**

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tui-input = "0.11"
time = { version = "0.3", features = ["formatting", "local-offset"] }
ttf-parser = "0.25"
ureq = { version = "2.10", features = ["tls"] }

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create cli.rs**

```rust
//! Command-line argument parsing for termux-colors.

use std::env;

#[derive(Debug, Default)]
pub struct Args {
    pub list_fonts: bool,
    pub apply_font: Option<String>,
    pub slot: String,
    pub version: bool,
    pub ascii: bool,
}

impl Args {
    pub fn parse() -> Self {
        let mut args = Args {
            slot: "regular".to_string(),
            ..Default::default()
        };
        let mut argv = env::args().skip(1);
        while let Some(arg) = argv.next() {
            match arg.as_str() {
                "--list-fonts" => args.list_fonts = true,
                "--apply-font" => {
                    if let Some(name) = argv.next() {
                        args.apply_font = Some(name);
                    }
                }
                "--slot" => {
                    if let Some(s) = argv.next() {
                        args.slot = s;
                    }
                }
                "--version" => args.version = true,
                "--ascii" => args.ascii = true,
                _ => {}
            }
        }
        // Check env vars for ascii
        if env::var("TERMUX_COLORS_ASCII").is_ok() && env::var("TERMUX_COLORS_ASCII").unwrap() != "0" {
            args.ascii = true;
        } else if env::var("FONT_CHANGER_ASCII").is_ok() && env::var("FONT_CHANGER_ASCII").unwrap() != "0" {
            args.ascii = true;
        }
        args
    }
}
```

**Step 3: Update lib.rs to add cli module**

**Step 4: Update main.rs to handle CLI flags**

**Step 5: Test compilation**

```bash
cargo test 2>&1 | tail -20
```

**Commit:** `feat: add CLI argument parsing and font dependencies`

---

## Task 2: Extend paths module for font support

**Files:**
- Modify: `src/paths.rs`

**Step 1: Add font-related methods to Paths**

Add slot files mapping, font dirs, slot paths etc.

```rust
use std::collections::HashMap;

impl Paths {
    pub fn fonts_dir(&self) -> std::path::PathBuf {
        self.root.join("fonts")
    }
    
    pub fn backups_dir(&self) -> std::path::PathBuf {
        self.root.join("backups")
    }
    
    pub fn font_slot_path(&self, slot: &str) -> Result<std::path::PathBuf> {
        let name = match slot {
            "regular" => "font.ttf",
            "bold" => "font-bold.ttf",
            "italic" => "font-italic.ttf",
            "bold-italic" => "font-bold-italic.ttf",
            _ => return Err(crate::error::Error::Message(format!("unknown font slot: {:?}", slot))),
        };
        Ok(self.root.join(name))
    }
    
    pub fn slot_names() -> Vec<&'static str> {
        vec!["regular", "bold", "italic", "bold-italic"]
    }
}
```

**Step 2: Test**

```bash
cargo test 2>&1 | tail -10
```

**Commit:** `feat: extend paths for font slots and directories`

---

## Task 3: Create font validation module

**Files:**
- Create: `src/fonts/mod.rs`
- Create: `src/fonts/validate.rs`
- Modify: `src/lib.rs`

Port from Go's validate package.

**Commit:** `feat: add font validation with magic bytes and ttf-parser`

---

## Task 4: Create font scanning module

**Files:**
- Create: `src/fonts/scan.rs`

Port from Go's scan package.

**Commit:** `feat: add font library scanning`

---

## Task 5: Create font apply/preview/backup module

**Files:**
- Create: `src/fonts/apply.rs`

Port from Go's apply package with SessionState, PreviewFont, CommitPreview, RestoreOriginal.

**Commit:** `feat: add font apply, preview and backup logic`

---

## Task 6: Create font importer module

**Files:**
- Create: `src/fonts/importer.rs`

Port from Go's importer package.

**Commit:** `feat: add font import with clash handling`

---

## Task 7: Create font downloader module

**Files:**
- Create: `src/fonts/downloader.rs`

Port from Go's downloader package with NerdFonts catalog.

**Commit:** `feat: add Nerd Fonts downloader`

---

## Task 8: Create font prompt capture module

**Files:**
- Create: `src/fonts/prompt.rs`

Port prompt capture logic.

**Commit:** `feat: add shell prompt capture for font preview`

---

## Task 9: Update lib.rs to export fonts module

**Files:**
- Modify: `src/lib.rs`

Add `pub mod fonts;`

**Commit:** `feat: export fonts module`

---

## Task 10: Add ASCII mode support

**Files:**
- Create: `src/ui/ascii.rs` or modify existing
- Update all UI components to respect ASCII mode

**Commit:** `feat: add ASCII borders mode`

---

## Task 11: Refactor App for tabs (replace screen stack with tabs)

**Files:**
- Modify: `src/app.rs`
- Modify: `src/screens/mod.rs`
- Modify: `src/main.rs`

This is a big change - replace stack with Tab enum, make event-driven.

**Commit:** `refactor: replace screen stack with tab bar`

---

## Task 12: Create Fonts screen

**Files:**
- Create: `src/screens/fonts.rs`
- Modify: `src/screens/mod.rs`

**Commit:** `feat: add Fonts tab screen`

---

## Task 13: Merge Colors tab (combine preset picker and terminal editor)

**Files:**
- Modify: `src/screens/preset_picker.rs` or create new combined screen
- Modify: `src/screens/terminal_editor.rs`

**Commit:** `feat: merge Colors tab`

---

## Task 14: Update App Theme tab

**Files:**
- Modify: `src/screens/app_theme_editor.rs`

**Commit:** `refactor: integrate App Theme as tab`

---

## Task 15: Update tests

**Files:**
- Modify: `tests/support/mod.rs`
- Update all UI tests for tab-based navigation

**Commit:** `test: update tests for tab-based UI`

---

## Task 16: Add font fixtures and tests

**Files:**
- Copy fixtures from font-changer
- Add tests for all font modules

**Commit:** `test: add font tests and fixtures`

---

## Task 17: Final integration and cleanup

**Files:**
- Update README
- Run all tests
- Verify CLI flags work

**Commit:** `feat: complete font-changer merge`

---

**Testing Strategy:** Run `cargo test` after each task. All existing tests must continue to pass (with updates where necessary per the design spec).
