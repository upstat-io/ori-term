//! Doc-only stub for tack's `s) skip to next test` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! `s) skip to next test` is a control verb on the begin-testing
//! menu, not a separate test screen. It is meaningful only inside
//! a sequence (`n) run standard tests` runs every test in order;
//! `s` advances to the next one without waiting for the current
//! to complete). Outside that sequence, `s` is a no-op. There is
//! nothing to capture or pin.
//!
//! # Where the equivalent coverage lives
//!
//! Each individual scenario in 05.1–05.4b drives ONE test directly
//! via its own menu_path, bypassing the `n) run standard tests`
//! sequencer entirely. Per-test isolation is stronger than the
//! `s)` control flow because each test runs in a fresh tack
//! invocation with no inter-test state leakage.
