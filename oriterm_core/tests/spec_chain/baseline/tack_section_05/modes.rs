//! Spec_chain conversion of the tack `modes` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/modes/mod.rs`
//! and the empirical capture against tack v1.08 (2026-04-08), the modes
//! sweep tests several terminfo capabilities that map to DEC private
//! modes:
//!
//! - `am` (auto_right_margin) → `DEC-DECAWM` (mode 7)
//! - `bw` (auto_left_margin) → `DEC-DECREVWRAP` (mode 45)
//!
//! Tack v1.08 does NOT surface per-cap `(am)` / `(bw)` labels (the modes
//! sweep only emits `(os)` at the end — see the scenario module's
//! rustdoc for the empirical evidence). Tack DOES exercise these caps
//! INTERNALLY by constructing screens that depend on auto-wrap and
//! reverse-wrap behavior. The protocol-level conversion below verifies
//! the DECSET/DECRST flag transitions that those caps depend on.
//!
//! # Catalog rows verified
//!
//! - `DEC-DECAWM` (mode 7) — autowrap mode
//! - `DEC-DECREVWRAP` (mode 45) — reverse wraparound
//!
//! Other modes-sweep caps (`bce`, `km`, `mir`, `msgr`, `xenl`, `os`)
//! are terminfo BEHAVIOR caps (back-color-erase, has-meta-key, etc.)
//! that are not 1:1 mapped to DEC private modes, so they have no
//! catalog rows in `dec-private-modes.md`. They remain owned by the
//! Section 23 cross-stack regression sweep.

use oriterm_core::TermMode;
use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, ParserExpectation, ScenarioExpectations, SpecHarness,
    SpecScenario,
};

/// DECRST mode 7 (`\E[?7l`) clears `TermMode::LINE_WRAP`.
///
/// Default `TermMode` includes `LINE_WRAP`, so the assertion proves the
/// reset path actually transitions the flag; without the regression guard
/// below, a no-op handler would still satisfy "LINE_WRAP unset" via the
/// initial state.
#[test]
fn decrst_mode_7_clears_line_wrap() {
    let scenario = SpecScenario {
        catalog_row_id: "DEC-DECAWM",
        bytes: b"\x1b[?7l",
        apex_layer: ApexLayer::Dispatch,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'l',
                params: &[7],
                intermediates: b"?",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("unset_private_mode")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    assert!(
        harness.term().mode().contains(TermMode::LINE_WRAP),
        "default TermMode must include LINE_WRAP"
    );

    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert!(
        !harness.term().mode().contains(TermMode::LINE_WRAP),
        "after DECRST 7, LINE_WRAP must be cleared"
    );
}

/// DECSET mode 7 (`\E[?7h`) restores `TermMode::LINE_WRAP` after reset.
///
/// Round-trip pin: reset then set must return the flag to its starting
/// state. Catches regressions where DECSET is a no-op on already-unset
/// flags (it must always set, regardless of prior state).
#[test]
fn decset_mode_7_restores_line_wrap_after_reset() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?7l"); // reset
    assert!(!harness.term().mode().contains(TermMode::LINE_WRAP));
    harness.feed(b"\x1b[?7h"); // set
    assert!(
        harness.term().mode().contains(TermMode::LINE_WRAP),
        "DECSET 7 after DECRST 7 must restore LINE_WRAP"
    );
}

/// DECSET mode 45 (`\E[?45h`) sets `TermMode::REVERSE_WRAP`.
///
/// Default `TermMode` does NOT include `REVERSE_WRAP`, so this is the
/// transition direction that proves the handler routes mode 45 to
/// `REVERSE_WRAP` (not to some other flag).
#[test]
fn decset_mode_45_sets_reverse_wrap() {
    let scenario = SpecScenario {
        catalog_row_id: "DEC-DECREVWRAP",
        bytes: b"\x1b[?45h",
        apex_layer: ApexLayer::Dispatch,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'h',
                params: &[45],
                intermediates: b"?",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("set_private_mode")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    assert!(
        !harness.term().mode().contains(TermMode::REVERSE_WRAP),
        "default TermMode must NOT include REVERSE_WRAP"
    );

    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert!(
        harness.term().mode().contains(TermMode::REVERSE_WRAP),
        "after DECSET 45, REVERSE_WRAP must be set"
    );
}

/// Regression guard: DECSET mode 45 must NOT touch the LINE_WRAP flag.
///
/// Without this pin, a handler that bulk-flipped multiple wrap-related
/// flags on any wrap-mode DECSET would silently pass the positive
/// REVERSE_WRAP pin above.
#[test]
fn decset_mode_45_leaves_line_wrap_unchanged() {
    let mut harness = SpecHarness::new();
    let initial_line_wrap = harness.term().mode().contains(TermMode::LINE_WRAP);
    harness.feed(b"\x1b[?45h");
    assert_eq!(
        harness.term().mode().contains(TermMode::LINE_WRAP),
        initial_line_wrap,
        "DECSET 45 must NOT modify LINE_WRAP"
    );
}
