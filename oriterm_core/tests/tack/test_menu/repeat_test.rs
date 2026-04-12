//! Doc-only stub for tack's `r) repeat test` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! `r) repeat test` is a control verb on the begin-testing menu
//! itself, not a separate test screen. Pressing `r` re-runs the
//! LAST executed test (whichever of `x`/`a`/`c`/`m`/`p` was last
//! invoked). With no prior test, `r` is a no-op. There is nothing
//! to capture or pin — `r` is purely a navigation aid for human
//! operators iterating on a single test.
//!
//! # Where the equivalent coverage lives
//!
//! Re-running scenarios deterministically is the responsibility
//! of the 05.6 determinism gate, which runs each Section 05
//! scenario 10 times in sequence and asserts every grid_text is
//! byte-identical. That gate exercises the same "repeat the test"
//! semantic that `r` provides interactively, but as an automated
//! invariant.
