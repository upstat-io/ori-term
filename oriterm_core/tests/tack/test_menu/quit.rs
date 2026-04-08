//! Doc-only stub for tack's `q) quit` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! `q) quit` exits the begin-testing submenu and returns to the
//! main tack menu. It is a navigation verb, not a test screen.
//! There is nothing to capture or pin — the only observable effect
//! is that the prompt changes from `tack/test [n] >` back to
//! `tack [n] >`, which is identical state to before the test
//! menu was entered.
//!
//! # Where the equivalent coverage lives
//!
//! Every Section 05 scenario uses `q` (or its equivalent) at the
//! end of its `quit_path` to exit tack cleanly when the test is
//! complete. The 05.0.b `phase_capture_loop` exit semantics and
//! the `ScenarioRunner::run` quit handling already exercise the
//! quit verb end-to-end on every scenario invocation. A separate
//! `tack_quit` scenario would duplicate that coverage with no
//! incremental signal.
