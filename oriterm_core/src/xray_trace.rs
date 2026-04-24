//! DEBUG-ONLY: limited-count event trace for BUG-06-019 (xray scene).
//!
//! Gated by the `ORITERM_XRAY_TRACE` env var at process start (checked
//! once via `OnceLock`). When enabled, the first `LIMIT` events emit
//! `log::info!` lines tagged with `target: "xray"`; subsequent events
//! are no-ops (single `AtomicUsize::fetch_add`).
//!
//! This module is intended to be deleted after BUG-06-019 diagnosis
//! lands; the branch `debug/bug-06-019-trace` owns these edits.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

const LIMIT: usize = 2000;

static COUNT: AtomicUsize = AtomicUsize::new(0);

fn enabled() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| std::env::var_os("ORITERM_XRAY_TRACE").is_some())
}

/// Returns `true` if the caller should emit a trace line for this event.
/// Returns `false` after `LIMIT` events (hard cap to prevent log flooding).
pub fn next() -> bool {
    if !enabled() {
        return false;
    }
    COUNT.fetch_add(1, Ordering::Relaxed) < LIMIT
}
