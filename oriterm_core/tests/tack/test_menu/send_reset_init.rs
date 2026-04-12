//! Doc-only stub for tack's `i) send reset and init` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! Pressing `i` from the begin-testing menu sends `rs1`/`rs2`/`rs3`
//! and `is1`/`is2`/`is3` reset/init strings directly to the
//! terminal and waits for the user to visually verify the screen
//! state. There is no programmatic completion signal — tack does
//! not emit a `Done` terminator after the reset sequence — so the
//! `ScenarioRunner` (which waits on `ready_anchor`) cannot drive
//! the screen end-to-end. The test relies entirely on visual
//! inspection by a human operator.
//!
//! # Where the equivalent coverage lives
//!
//! - The 05.4b padding scenario (`tack_padding`) probes `rs1` /
//!   `reset_1string` end-to-end via the `p) test padding and
//!   string capabilities` entry, which IS automatable.
//! - Section 03's `tic`-roundtrip tests verify the reset/init cap
//!   strings are syntactically correct in `extra/ori_term.info`.
//! - Section 07's GPU goldens (when they land) will visually
//!   regression-test the rendered output after a reset sequence.
