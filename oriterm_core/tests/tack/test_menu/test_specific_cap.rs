//! Doc-only stub for tack's `/) test a specific capability` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! Pressing `/` from the begin-testing menu prompts the user to
//! type a cap name (e.g. `cup`, `setaf`) and runs an ad-hoc test
//! against that single cap. The cap name input is required from
//! a human operator — there is no programmatic interface to drive
//! the prompt with a fixed set of caps for automated coverage.
//!
//! # Where the equivalent coverage lives
//!
//! Per-cap coverage is the responsibility of the 05.5 cap-coverage
//! matrix, which iterates over `extra/ori_term.info` and asserts
//! every cap is exercised by at least one scenario or is on a
//! per-section exemption list. The matrix is more rigorous than
//! tack's interactive `/) test specific cap` flow because it
//! enforces COMPLETE coverage rather than ad-hoc spot checks.
