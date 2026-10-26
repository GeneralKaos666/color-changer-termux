mod support;

use std::time::{Duration, Instant};

use support::{ctrl, render_text};
use termux_settings::app::App;
use termux_settings::paths::Paths;

#[test]
fn tiny_terminals_get_a_guard_not_a_panic() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    let text = render_text(&mut app, 20, 5);
    assert!(text.contains("too small"));
    app.handle_key(ctrl('c'));
    assert!(app.should_quit());
}

#[test]
fn status_expires_after_four_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    app.set_status("Saved thing.");
    assert!(render_text(&mut app, 100, 30).contains("Saved thing."));
    app.tick(Instant::now() + Duration::from_secs(5));
    assert!(!render_text(&mut app, 100, 30).contains("Saved thing."));
}

#[test]
fn a_resize_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Paths::new(dir.path()));
    for (w, h) in [(10, 4), (40, 10), (120, 40), (39, 9)] {
        let _ = render_text(&mut app, w, h);
    }
}
