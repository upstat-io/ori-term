//! Section 06.4 test wrapper for tack's
//! `u) test ENQ/ACK (DA1) handshake` tool screen.
//!
//! The const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::enq_ack`. This
//! file is a thin wrapper that invokes `ScenarioRunner::run(..)`,
//! cross-references the `u8`/`u9` cap declarations from
//! `extra/ori_term.info`, and pins the captured grid via insta.
//!
//! See the `enq_ack/mod.rs` module rustdoc for the full empirical
//! record (success-path vs failure-path output, the DA1 round-trip
//! through `oriterm_core/src/term/handler/status.rs:121-148`, and
//! the plan deviation rationale).

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::cap_coverage::{declared_cap_value, parse_declared_caps};
use oriterm_test_support::tack_framework::scenarios::enq_ack::TACK_TOOLS_ENQ_ACK;

#[test]
fn tack_tools_enq_ack_80x24() {
    if !ScenarioRunner::available() {
        eprintln!(
            "tack, tic, or a supported tack version unavailable, \
             skipping tack_tools_enq_ack_80x24"
        );
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_TOOLS_ENQ_ACK);

 // REGRESSION GUARD: empty grid means tack navigation failed before
    // the ENQ/ACK probe ran. The downstream assertions would still
    // pass on an empty grid (no notes → no extraction), so this
    // explicit assertion fires first with a precise diagnostic.
    assert!(
        !outcome.grid_text.is_empty(),
        "TACK_TOOLS_ENQ_ACK returned empty grid — tack navigation \
         failed before render"
    );

    // CAP DECLARATION CROSS-REFERENCE — u9 / u8 must be present in
    // the pinned terminfo. parse_declared_caps() returns
    // BTreeSet<String> of cap NAMES only. The presence check is the
    // first half of the round-trip pin.
    let caps = parse_declared_caps();
    assert!(
        caps.contains("u9"),
        "extra/ori_term.info must declare u9 (ENQ trigger sequence)"
    );
    assert!(
        caps.contains("u8"),
        "extra/ori_term.info must declare u8 (ACK terminator pattern)"
    );

    // CAP VALUE CROSS-REFERENCE — extract the literal u9 value
    // from the pinned terminfo source via the canonical helper in
    // `oriterm_test_support` (fix: use the SSOT helper
    // instead of duplicating the parser locally) and assert it
    // matches what ori_term's DA1 handler is exercised by.
    // ori_term's u9 is \E[c (the DA1 query, per
    // `extra/ori_term.info:115`); the success of the scenario
    // implicitly proves ori_term responded to the byte sequence
    // the cap declares.
    let u9_value =
        declared_cap_value("u9").expect("u9 value must be extractable from pinned terminfo");
    assert_eq!(
        u9_value, "\\E[c",
        "u9 declaration drift — Section 06.4 anchors the success \
         path on ori_term answering DA1 (\\E[c). If u9 changes, \
         the ENQ probe sends a different byte sequence and this \
         scenario must be re-validated against the new behavior."
    );

    // SUCCESS-PATH NOTE EXTRACTION — the parser records exactly
    // one `ack_terminator=` note for the success-path screen. tack
    // captures the trailing `c` of ori_term's DA1 response
    // (\x1b[?64;6;4c), so the terminator is `c`.
    let term_note = outcome
        .parsed
        .notes
        .iter()
        .find(|n| n.starts_with("ack_terminator="))
        .expect("parse_enq_ack_screen must record an ack_terminator note");
    assert_eq!(
        term_note, "ack_terminator=c",
        "expected ACK terminator `c` (the trailing byte of \
         ori_term's DA1 response \\x1b[?64;6;4c — see \
         oriterm_core/src/term/handler/status.rs:131); got \
         {term_note:?}.\nGrid:\n{}",
        outcome.grid_text
    );

    // Insta snapshot of the full grid for visual regression — pins
    // the empirical screen layout produced by tack v1.08 against
    // ori_term's pinned terminfo + DA1 handler.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

// fix: the previous draft of this file had a local
// `extract_cap_value` helper plus an inline
// `#[cfg(test)] mod local_helper_tests` block to pin the
// extraction. promoted that helper to
// `oriterm_test_support::tack_framework::cap_coverage::extract_cap_value`
// (the SSOT for tic-format cap-value extraction); the sibling
// tests for the canonical helper live next to its definition in
// the `oriterm_test_support` crate. The local nested mod was a
// test-organization rule violation (no inline test modules) and
// is no longer needed once the helper has its proper canonical
// home, so the nested mod was deleted along with the local copy.
