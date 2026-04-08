//! Test wrappers for the modes scenarios.
//!
//! Const `ScenarioSpec`s and parsers live in
//! `oriterm_test_support::tack_framework::scenarios::modes`. This
//! file just defines `#[test] fn` wrappers that invoke
//! `ScenarioRunner` against those consts.
//!
//! # Per-cap phase scenarios deliberately NOT added in 05.1
//!
//! Section 05.1 of the tack-conformance plan originally specified
//! 7 per-cap phase scenarios (`tack_modes_phase_am`, `_bce`, `_bw`,
//! `_km`, `_mir`, `_msgr`, `_xenl`) that would each capture
//! tack's emission of the corresponding `(cap)` line during the
//! modes-test sweep. Empirical investigation under tack v1.08
//! against `extra/ori_term.info` AND under tack v1.08 against the
//! host's `xterm-256color` (driven through `expect`) showed that
//! tack v1.08's modes test emits ONLY `(os)` content. The full
//! captured output is:
//!
//! ```text
//! \x1B[H\x1B[2J(os) should be true, not false.
//! (os) should be           false.
//! (os) over-strike is false in the data base.  (os) Done
//! ```
//!
//! No `(am)`, `(bce)`, `(bw)`, `(km)`, `(mir)`, `(msgr)`, or
//! `(xenl)` is ever printed. Tack v1.08 tests the other modes
//! caps INTERNALLY (sets up screens that exercise auto-margins,
//! back-color-erase, etc.) but doesn't emit per-cap visible
//! status — that's been tack's design since 1997. The
//! `(os) Done` line is the test terminator and the only visible
//! signal that the modes test ran successfully.
//!
//! Section 04's `TACK_MODES_AM` (and its `parse_modes_screen`
//! parser with `KNOWN: &[\"os\"]`) is therefore the correct and
//! complete coverage of tack's modes screen. The 05.1 plan's
//! per-cap design was based on a wrong model of tack's output
//! and could not have worked regardless of capture strategy.
//!
//! The 05.0.b `PhaseSpec` / `ScenarioRunner::run_phase` /
//! `PtySession::drain_until` infrastructure is preserved as
//! a speculative future-use primitive for any plan section that
//! does need to capture mid-flow tack content (verified by
//! empirical inspection of that section's tack output). Section
//! 05.1's plan body records the empirical finding so future
//! readers do not re-attempt the per-cap design.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::modes::TACK_MODES_AM;

#[test]
fn tack_modes_am() {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic not installed, skipping tack_modes_am");
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_MODES_AM);

    // Programmatic semantic assertion: the parser found `os`
    // (over-strike) in the modes screen capability list. Tack lists
    // `os` last as the test terminator, so it's always visible in
    // the 24-row viewport at the moment the test reports "Done"
    // (earlier caps like `am`, `bce` scrolled off — Section 05
    // adds per-cap scenarios that capture the right viewport for
    // each). Uses the tokenized `grid_has_paren_token` helper
    // indirectly via `parse_modes_screen` — tack tags every modes
    // result with `(cap_name)` and `grid_has_paren_token` matches
    // exactly that form, so substring collisions cannot false-pass.
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "os"),
        "expected `os` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );

    // Insta snapshot of the full grid for visual regression catching.
    // Use the size-aware snapshot name so size-matrix runs in
    // Section 05 share the snapshot file when the screen is the same.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
