//! Doc-only stub for tack's `e) edit terminfo` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! Pressing `e` from the begin-testing menu launches an interactive
//! terminfo editor that blocks waiting for the user to type cap
//! names, edit values, and confirm changes. There is no way to
//! drive this screen end-to-end through `ScenarioRunner` without
//! either (a) sending arbitrary editor commands and asserting
//! against editor state — which couples the test to tack's
//! internal editor, not its terminfo behavior — or (b) sending a
//! quit sequence immediately, which produces no observable test
//! output.
//!
//! # Where the equivalent coverage lives
//!
//! Terminfo correctness for `extra/ori_term.info` is covered by:
//! - The 05.5 cap-coverage matrix (parses `extra/ori_term.info`
//!   directly and asserts every cap is exercised by some scenario
//!   or is on a per-section exemption list).
//! - Section 03's `tic`-roundtrip tests
//!   (`crates/oriterm_test_support/src/terminfo/`) which compile
//!   the terminfo source via `tic` and assert the binary form
//!   round-trips back to the source via `infocmp`.
//!
//! The terminfo editor itself is a tack feature, not an ori_term
//! feature, and is intentionally not under test here.
