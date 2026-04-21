//! Test-only VT340 palette-rebuild bypass for the sixel decoder.
//!
//! Exclusively consumed by the negative-pin palette-leak test in
//! `sixel/tests.rs`. The guard flips a thread-local flag so
//! `SixelParser::new` skips the `VT340_PALETTE` copy loop; the `Drop`
//! impl restores the flag even on panic, preventing state leakage into
//! other tests on the same worker thread per `.claude/rules/impl-
//! hygiene.md §Temporal Coupling & RAII Guards`.

use std::cell::Cell;

thread_local! {
    pub(super) static BYPASS_VT340_RESET: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard for the test-only VT340 palette-rebuild bypass.
///
/// Construct via [`BypassVt340ResetGuard::enable`] at the start of a
/// test that wants to verify what happens when `SixelParser::new` skips
/// the VT340 default copy. The guard flips the thread-local flag to
/// `true` on construction and restores it to `false` on `Drop`.
pub(super) struct BypassVt340ResetGuard {
    _priv: (),
}

impl BypassVt340ResetGuard {
    pub(super) fn enable() -> Self {
        BYPASS_VT340_RESET.with(|cell| cell.set(true));
        Self { _priv: () }
    }
}

impl Drop for BypassVt340ResetGuard {
    fn drop(&mut self) {
        BYPASS_VT340_RESET.with(|cell| cell.set(false));
    }
}
