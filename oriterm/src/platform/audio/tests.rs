//! Construction-only smoke tests for the bell-sound dispatcher.
//!
//! `play_bell()` is fire-and-forget — its sound output is unobservable
//! from Rust. These tests pin the behavioral contract that the function
//! is callable on every supported platform without panicking. The
//! load-bearing verification of audible output is the manual user test
//! per `bug-tracker/plans//section-07-completion-checklist.md`.

use super::play_bell;

/// Regression: `play_bell()` must be callable on the
/// current host without panicking. Catches FFI-link failures and `cfg`
/// misconfigurations before they reach a user.
#[test]
fn play_bell_does_not_panic_on_current_platform() {
    play_bell();
}

/// Regression: `play_bell()` must be safe to call multiple
/// times in a row. Catches FFI-lifetime / thread-safety bugs (e.g. an
/// X11 display handle that's not properly closed between calls).
#[test]
fn play_bell_is_idempotent_when_called_multiple_times() {
    for _ in 0..5 {
        play_bell();
    }
}
