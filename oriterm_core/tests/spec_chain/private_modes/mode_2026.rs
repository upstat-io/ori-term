//! Spec-chain tests for Mode 2026 (Synchronized Output) — core-layer
//! DECSET/DECRST flag toggle and DECRQM query/response.
//!
//! Section 09.2 scope: verifies ONLY the core-layer plumbing. The
//! Begin/Commit/Abort apex tests (publication suppression, timeout-abort)
//! are owned by Section 06 at `oriterm_mux/src/pane/io_thread/tests.rs`.
//!
//! # Catalog rows verified
//!
//! - `DEC-SYNC-UPDATE` — flag toggle + DECRQM (core-layer only)

use oriterm_core::TermMode;
use oriterm_core::effect::{Effect, PtyEffect};
use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, ParserExpectation, ScenarioExpectations, SpecHarness,
    SpecScenario,
};

/// DECSET `?2026` sets `TermMode::SYNC_UPDATE`.
///
/// Default `TermMode` does NOT include `SYNC_UPDATE`, so the assertion
/// proves the handler routes mode 2026 to the correct flag.
#[test]
fn mode_2026_decset_toggles_flag() {
    let scenario = SpecScenario {
        catalog_row_id: "DEC-SYNC-UPDATE",
        bytes: b"\x1b[?2026h",
        apex_layer: ApexLayer::Dispatch,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'h',
                params: &[2026],
                intermediates: b"?",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("set_private_mode")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();

    // Negative pin: flag must start off.
    assert!(
        !harness.term().mode().contains(TermMode::SYNC_UPDATE),
        "default TermMode must NOT include SYNC_UPDATE"
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

    // Semantic pin: flag must be set after DECSET.
    assert!(
        harness.term().mode().contains(TermMode::SYNC_UPDATE),
        "after DECSET ?2026, SYNC_UPDATE must be set"
    );
}

/// DECRST `?2026` clears `TermMode::SYNC_UPDATE`.
///
/// Round-trip pin: set then reset must clear the flag.
#[test]
fn mode_2026_decrst_clears_flag() {
    let mut harness = SpecHarness::new();

    // Set the flag first.
    harness.feed(b"\x1b[?2026h");
    assert!(
        harness.term().mode().contains(TermMode::SYNC_UPDATE),
        "SYNC_UPDATE must be set after DECSET"
    );

    // Reset the flag.
    harness.feed(b"\x1b[?2026l");
    assert!(
        !harness.term().mode().contains(TermMode::SYNC_UPDATE),
        "after DECRST ?2026, SYNC_UPDATE must be cleared"
    );
}

/// Round-trip: reset → set → reset must end with the flag cleared.
///
/// Catches regressions where repeated toggles accumulate or drift.
#[test]
fn mode_2026_round_trip_toggle() {
    let mut harness = SpecHarness::new();

    // Start off → set → off → set → off.
    for _ in 0..3 {
        harness.feed(b"\x1b[?2026h");
        assert!(harness.term().mode().contains(TermMode::SYNC_UPDATE));
        harness.feed(b"\x1b[?2026l");
        assert!(!harness.term().mode().contains(TermMode::SYNC_UPDATE));
    }
}

/// DECRQM `?2026` returns `\x1b[?2026;2$y` when mode is reset (default).
#[test]
fn mode_2026_decrqm_default_reset() {
    let mut harness = SpecHarness::new();

    // Query DECRQM for mode 2026 while it is in the default (reset) state.
    harness.feed(b"\x1b[?2026$p");

    let expected_response = b"\x1b[?2026;2$y";
    let found = harness.outcome().effects_emitted.iter().any(
        |e| matches!(e, Effect::Pty(PtyEffect::Write { bytes, .. }) if bytes == expected_response),
    );
    assert!(
        found,
        "DECRQM ?2026 when reset should emit {:?}, got effects: {:?}",
        String::from_utf8_lossy(expected_response),
        harness.outcome().effects_emitted
    );
}

/// DECRQM `?2026` returns `\x1b[?2026;1$y` when mode is set.
///
/// Mode 2026 activates VTE sync buffering — the SpecHarness uses a
/// persistent `Processor`, so a separate `feed()` after DECSET would
/// buffer the DECRQM query instead of processing it. To work around
/// this, we feed BSU + DECRQM + ESU in a single call. The DECRQM is
/// replayed from the sync buffer while the mode flag is still set
/// (before ESU clears it), so the response correctly reports `1`.
#[test]
fn mode_2026_decrqm_after_set() {
    let mut harness = SpecHarness::new();

    // Feed DECSET (BSU) + DECRQM + DECRST (ESU) in one call.
    // Processing order during sync-buffer replay:
    //   1. DECSET ?2026 → sets TermMode::SYNC_UPDATE, starts sync buffer
    //   2. DECRQM ?2026 → buffered in sync buffer
    //   3. DECRST ?2026 → detected by advance_sync_csi → triggers buffer flush
    //   4. During flush: DECRQM fires while flag is still set → response `?2026;1$y`
    //   5. During flush: DECRST fires → clears the flag
    harness.feed(b"\x1b[?2026h\x1b[?2026$p\x1b[?2026l");

    let expected_response = b"\x1b[?2026;1$y";
    let found = harness.outcome().effects_emitted.iter().any(
        |e| matches!(e, Effect::Pty(PtyEffect::Write { bytes, .. }) if bytes == expected_response),
    );
    assert!(
        found,
        "DECRQM ?2026 when set should emit {:?}, got effects: {:?}",
        String::from_utf8_lossy(expected_response),
        harness.outcome().effects_emitted
    );
}

/// Negative pin: DECSET `?2026` must NOT touch unrelated flags.
///
/// Verifies that only the SYNC_UPDATE flag changes — no side effects
/// on other mode bits.
#[test]
fn mode_2026_does_not_touch_unrelated_flags() {
    let mut harness = SpecHarness::new();
    let mode_before = harness.term().mode().bits();

    harness.feed(b"\x1b[?2026h");

    let mode_after = harness.term().mode().bits();
    let changed_bits = mode_before ^ mode_after;

    assert_eq!(
        changed_bits,
        TermMode::SYNC_UPDATE.bits(),
        "DECSET ?2026 must only toggle SYNC_UPDATE; unexpected bits changed: {changed_bits:#b}"
    );
}
