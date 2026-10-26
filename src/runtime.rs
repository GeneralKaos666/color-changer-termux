//! The message channel and worker spawners the event loop drains.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;

use crate::fonts::scan::FontEntry;
use crate::paths::Paths;

/// How long [`crate::app::App`] blocks waiting for input before ticking.
pub const TICK: Duration = Duration::from_millis(250);

/// Everything that can change the app: input, a clock tick, or a worker result.
pub enum Msg {
    Tick(Instant),
    Key(KeyEvent),
    Resize(u16, u16),
    Reloaded(bool),
    PromptCaptured(Vec<String>),
    FontsScanned(std::io::Result<Vec<FontEntry>>),
    DownloadProgress {
        name: String,
        received: u64,
        total: Option<u64>,
    },
    Downloaded(Result<PathBuf, String>),
}

/// A one-shot channel workers post [`Msg`]s into.
pub struct Runtime {
    rx: Receiver<Msg>,
    tx: Sender<Msg>,
}

impl Runtime {
    /// Build an empty runtime with no jobs in flight.
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { rx, tx }
    }

    /// A sender for spawning a worker.
    pub fn sender(&self) -> Sender<Msg> {
        self.tx.clone()
    }

    /// Every message that has arrived since the last drain.
    pub fn drain(&self) -> Vec<Msg> {
        self.rx.try_iter().collect()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Run `termux-reload-settings` off the input thread and report the outcome.
pub fn spawn_reload(tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let ok = crate::library::reload_termux();
        let _ = tx.send(Msg::Reloaded(ok));
    });
}

/// Capture the shell prompt off the input thread.
pub fn spawn_prompt(tx: Sender<Msg>) {
    std::thread::spawn(move || {
        let lines = crate::fonts::prompt::capture_prompt_lines(None);
        let _ = tx.send(Msg::PromptCaptured(lines));
    });
}

/// Scan the font library off the input thread.
pub fn spawn_scan(tx: Sender<Msg>, paths: Paths) {
    std::thread::spawn(move || {
        let result = crate::fonts::scan::list_library(&paths);
        let _ = tx.send(Msg::FontsScanned(result));
    });
}

/// Download and install a font off the input thread.
pub fn spawn_download(tx: Sender<Msg>, paths: Paths, name: String, url: String) {
    std::thread::spawn(move || {
        let name_for_progress = name.clone();
        let progress_tx = tx.clone();
        let progress = move |received: u64, total: Option<u64>| {
            let _ = progress_tx.send(Msg::DownloadProgress {
                name: name_for_progress.clone(),
                received,
                total,
            });
        };
        let result = crate::fonts::downloader::download(&url, progress).and_then(|bytes| {
            crate::fonts::downloader::install_bytes(&paths, &name, &bytes)
                .map_err(|e| e.to_string())
        });
        let _ = tx.send(Msg::Downloaded(result));
    });
}
