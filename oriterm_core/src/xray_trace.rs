//! DEBUG-ONLY: limited-count event trace for BUG-06-018 (whiteout scene).
//!
//! Gated by the `ORITERM_XRAY_TRACE` env var at process start.
//!
//! No row filter (whiteout uses the full screen). Up to `LIMIT` events
//! emit `log::info!` lines tagged with `target: "oriterm_core::xray"`;
//! subsequent events are no-ops. Branch `debug/bug-06-018-trace` owns
//! these edits.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

const LIMIT: usize = 30_000;

static COUNT: AtomicUsize = AtomicUsize::new(0);

fn enabled() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| std::env::var_os("ORITERM_XRAY_TRACE").is_some())
}

/// Returns `true` if the caller should emit a trace line.
///
/// Returns `false` after `LIMIT` events.
pub fn next() -> bool {
    if !enabled() {
        return false;
    }
    COUNT.fetch_add(1, Ordering::Relaxed) < LIMIT
}
