//! DEBUG-ONLY: limited-count event trace for BUG-06-019 (xray scene).
//!
//! Gated by the `ORITERM_XRAY_TRACE` env var at process start.
//!
//! Trace events from non-marquee rows are filtered out (xray's marquee
//! sits at y=1..11; we trace rows 0..=11). Up to `LIMIT` events emit
//! `log::info!` lines tagged with `target: "oriterm_core::xray"`;
//! subsequent events are no-ops.
//!
//! This module is intended to be deleted after BUG-06-019 diagnosis
//! lands; the branch `debug/bug-06-019-trace` owns these edits.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

const LIMIT: usize = 20_000;
const MAX_ROW: usize = 11;

static COUNT: AtomicUsize = AtomicUsize::new(0);

fn enabled() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| std::env::var_os("ORITERM_XRAY_TRACE").is_some())
}

/// Returns `true` if the caller should emit a trace line for a cell-write.
///
/// Filters out rows beyond the xray marquee band (y=1..11). Returns
/// `false` after `LIMIT` events.
pub fn next_at_row(row: usize) -> bool {
    if !enabled() || row > MAX_ROW {
        return false;
    }
    COUNT.fetch_add(1, Ordering::Relaxed) < LIMIT
}

/// Returns `true` if the caller should emit a trace line for an SGR event.
///
/// No row context — SGR sets cursor template state. Returns `false` after
/// `LIMIT` events.
pub fn next_sgr() -> bool {
    if !enabled() {
        return false;
    }
    COUNT.fetch_add(1, Ordering::Relaxed) < LIMIT
}
