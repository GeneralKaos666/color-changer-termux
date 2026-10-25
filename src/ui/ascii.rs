use std::sync::atomic::{AtomicBool, Ordering};

static ASCII: AtomicBool = AtomicBool::new(false);

pub fn set_ascii(v: bool) {
    ASCII.store(v, Ordering::Relaxed);
}

pub fn is_ascii() -> bool {
    ASCII.load(Ordering::Relaxed)
}
