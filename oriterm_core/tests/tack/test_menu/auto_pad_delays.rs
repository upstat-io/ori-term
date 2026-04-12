//! Doc-only stub for tack's `t) auto generate pad delays` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::ExcludedInteractive` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # Why this is excluded
//!
//! Pressing `t` from the begin-testing menu runs an iterative
//! auto-tuner that measures the actual hardware padding required
//! for each capability and writes the results back to the
//! terminfo entry. This is a maintenance utility for tack
//! authors, not a test of the terminal — and the result depends
//! on hardware timing, so two runs against the same terminal can
//! produce different padding values. The screen is interactive
//! (the user must accept or reject each measurement) and has no
//! deterministic terminator the runner could wait on.
//!
//! # Where the equivalent coverage lives
//!
//! ori_term is a software terminal emulator running over a PTY,
//! not a hardware terminal — padding delays are zero-cost in our
//! pipeline. The 05.4b padding scenario (`tack_padding`) covers
//! the cap probes that pad timing would have measured, without
//! the iterative tuning loop.
