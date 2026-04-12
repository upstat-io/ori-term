//! Doc-only stub for tack's `P) test printer` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! Pressing `P` from the begin-testing menu sends MC4/MC5
//! (`mc4`/`mc5` — printer-on / printer-off) and other printer
//! capability sequences and waits for the user to verify that the
//! attached printer produces output. ori_term does not implement
//! a virtual printer endpoint, so there is nothing to assert
//! against. The screen has no `Done` terminator that the runner
//! could wait on — the test is fundamentally interactive.
//!
//! # Where the equivalent coverage lives
//!
//! Printer caps (`mc0`, `mc4`, `mc5`, `mc5p`) are NOT declared
//! in `extra/ori_term.info` because ori_term is a GPU-accelerated
//! terminal emulator with no printer integration. The 05.5
//! cap-coverage matrix should record these as exempt-by-design
//! via Section 05's `CapCoverageContribution::exempt` slice.
