//! Coalescing wakeup helper shared by every [`crate::backend::MuxBackend`] implementation.
//!
//! [`guarded_wakeup`] wraps a raw wakeup callback with an [`AtomicBool`]
//! gate so flood-rate calls collapse to a single notification per
//! poll cycle. The returned flag is the SSOT for `has_pending_wakeup` /
//! `clear_wakeup_pending` queries on the owning backend.
//!
//! The (closure, flag) tuple is the algorithmic SSOT for the wakeup-
//! coalescing protocol; the 1-line `load(Acquire)` / `store(false,
//! Release)` operations on the returned flag remain in each backend's
//! `MuxBackend` impl since they are single-statement field accesses,
//! not multi-step algorithms.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Wrap a raw wakeup callback with an [`AtomicBool`] coalescing gate.
///
/// Returns `(guarded, pending)` where:
/// - `guarded` invokes `raw` only on the leading edge of each clear
///   cycle. Subsequent calls before `pending` is cleared are absorbed.
/// - `pending` is the SSOT flag — store `false` (Release) on poll to
///   re-arm the gate.
///
/// The 1-line `pending.load(Ordering::Acquire)` and
/// `pending.store(false, Ordering::Release)` operations remain in
/// each backend's `MuxBackend` impl since they are single-statement
/// field accesses, not multi-step algorithms.
#[must_use = "discarding the (guarded_wakeup, wakeup_pending) tuple silently breaks the coalescing protocol"]
pub(super) fn guarded_wakeup(
    raw: Arc<dyn Fn() + Send + Sync>,
) -> (Arc<dyn Fn() + Send + Sync>, Arc<AtomicBool>) {
    let pending = Arc::new(AtomicBool::new(false));
    let pending_clone = pending.clone();
    let guarded = Arc::new(move || {
        if !pending_clone.swap(true, Ordering::Release) {
            (raw)();
        }
    }) as Arc<dyn Fn() + Send + Sync>;
    (guarded, pending)
}

#[cfg(test)]
mod tests;
