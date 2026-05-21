//! Test wrapper for the tack padding-and-string-capabilities scenario.
//!
//! Const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::padding`.
//! This file defines a `#[test] fn` wrapper at 80x24 only —
//! padding is intrinsically size-independent (the test does not
//! depend on viewport dimensions for the cap probes).
//!
//! # Empirical reality (tack v1.08 + ori_term.info)
//!
//! Pressing `p` from the begin-testing menu first triggers an
//! interactive ENQ/ACK / DA1 handshake — tack writes
//! `Testing ENQ/ACK, standby...\x1B[c` and waits for the terminal
//! to respond. The framework's `PtySession` answers automatically
//! via `oriterm_core::Term`'s `Event::PtyWrite` handler. After the
//! handshake, tack reports `ACK terminating character: c`, enters
//! the `tack/test/pad [n] >` sub-menu, and on `n` runs the standard
//! padding test.
//!
//! Against `extra/ori_term.info` the captured grid after the test is:
//!
//! ```text
//! (rs1) reset_1string, not present. (rs1) Done
//! ```
//!
//! Tack only probes `rs1` and reports it as `not present` because
//! `extra/ori_term.info` declares NO reset-string capabilities at
//! all (neither `rs1`, `rs2`, nor `rs3`). The previous version of
//! this comment incorrectly said "ori_term.info declares `rs2`
//! instead" (Codex review-work iteration 4 of M2)
//! correctly noted that this was a factual error against the
//! pinned terminfo source. Whether `extra/ori_term.info` should
//! declare any reset string is a Section 05.5 cap-coverage matrix
//! decision; this wrapper does NOT pin the `not present` substring
//! because that depends on the terminfo state, not on tack itself.
//! See `crates/oriterm_test_support/src/tack_framework/scenarios/padding/mod.rs`
//! rustdoc for the full empirical evidence and the hybrid coverage
//! strategy.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::padding::TACK_PADDING;

#[test]
fn tack_padding() {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic unavailable, skipping tack_padding");
        return;
    }
    let outcome = ScenarioRunner::run(&TACK_PADDING);

    // End-to-end pin: tack ran the standard padding test and
    // reported its terminator. Catches regressions in the spawn
    // / DA1-handshake / navigate / capture / quit pipeline.
    assert!(
        outcome.grid_text.contains("Done"),
        "expected captured grid to contain 'Done' terminator, got:\n{}",
        outcome.grid_text
    );

    // SEMANTIC PINS: tack v1.08's padding test surfaces the
    // (rs1) reset_1string cap probe. These two assertions are
    // the canonical semantic claims for 05.4b cap-coverage:
    // 1. "(rs1)" — proves tack referenced the rs1 cap by its
    // terminfo short name (the canonical tack output format
    // matching the (am)/(os)/(bel)/(colors)/(pairs)/(clear)
    // pattern from prior screens). This is the cap-coverage
    // pin for `rs1` in 05.5.
    // 2. "reset_1string" — proves tack referenced the cap by its
    // terminfo full name. Catches a regression where tack
    // swaps to a different cap probe.
    // The `not present` part of tack's output is NOT pinned
    // because that's a property of the current ori_term.info
    // (which declares NO reset-string caps at all — neither rs1,
    // rs2, nor rs3), not of the padding test itself. If a future
    // ori_term.info adds rs1, the wrapper should still pass.
    // (fix: previous version incorrectly said
    // "declares rs2 but not rs1".)
    assert!(
        outcome.grid_text.contains("(rs1)"),
        "expected captured grid to contain '(rs1)' parenthesized cap, got:\n{}",
        outcome.grid_text
    );
    assert!(
        outcome.grid_text.contains("reset_1string"),
        "expected captured grid to contain 'reset_1string' full cap name, got:\n{}",
        outcome.grid_text
    );

    // Insta snapshot of the full grid for visual regression.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
