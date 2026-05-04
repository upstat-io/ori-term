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
use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, ParserExpectation, ScenarioExpectations, SpecHarness,
    SpecScenario, pty_writes,
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

 // Regression guard: flag must start off.
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

 // Property: flag must be set after DECSET.
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
    let found = pty_writes(&harness).any(|(b, _)| b == expected_response);
    assert!(
        found,
        "DECRQM ?2026 when reset should emit {:?}, got effects: {:?}",
        String::from_utf8_lossy(expected_response),
        harness.outcome().effects_emitted
    );
}

/// DECRQM `?2026` returns `\x1b[?2026;1$y` when mode is set.
///
/// With Mode 2026 no longer buffering bytes (), DECRQM is
/// dispatched inline between BSU and ESU within the same `feed()` call.
/// The mode flag is still set when DECRQM arrives (BSU set it, ESU has
/// not yet been processed), so the response correctly reports `1`.
#[test]
fn mode_2026_decrqm_after_set() {
    let mut harness = SpecHarness::new();

    // BSU + DECRQM + ESU in one call. Inline dispatch order:
    //   1. DECSET ?2026  → sets TermMode::SYNC_UPDATE, arms sync timer
    //   2. DECRQM ?2026  → emits `?2026;1$y` while flag is still set
    //   3. DECRST ?2026  → clears the flag, disarms sync timer
    harness.feed(b"\x1b[?2026h\x1b[?2026$p\x1b[?2026l");

    let expected_response = b"\x1b[?2026;1$y";
    let found = pty_writes(&harness).any(|(b, _)| b == expected_response);
    assert!(
        found,
        "DECRQM ?2026 when set should emit {:?}, got effects: {:?}",
        String::from_utf8_lossy(expected_response),
        harness.outcome().effects_emitted
    );
}

/// Regression guard: DECSET `?2026` must NOT touch unrelated flags.
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

// §03 matrix — device-query bytes inside Mode 2026 dispatch
// inline, not deferred to ESU/timeout.
//
// Each test below feeds `BSU + query` in ONE harness.feed() call with
// NO ESU. The query MUST dispatch INLINE during the sync window — its
// PTY response appears in the harness's effect transcript before ESU
// is even processed. If a regression re-introduced byte buffering, the
// response would be deferred and these assertions would fail.
//
// All 11 query classes from §01 blast radius are covered, plus the
// DECRQM unknown / set / reset axes added by review rounds 0-1.

/// Helper: assert a specific PTY response is in the harness transcript.
///
/// Used by the §03 matrix tests below — each test feeds a
/// device query inside Mode 2026 and asserts the expected response.
fn assert_pty_response(harness: &SpecHarness, expected: &[u8], context: &str) {
    let found = pty_writes(harness).any(|(b, _)| b == expected);
    assert!(
        found,
        "{}: expected PTY response {:?} not found; effects: {:?}",
        context,
        String::from_utf8_lossy(expected),
        harness.outcome().effects_emitted
    );
}

/// Helper: assert any PTY write contains `needle` as a substring.
fn assert_pty_response_contains(harness: &SpecHarness, needle: &[u8], context: &str) {
    let found = pty_writes(harness).any(|(b, _)| b.windows(needle.len()).any(|w| w == needle));
    assert!(
        found,
        "{}: expected PTY response containing {:?} not found; effects: {:?}",
        context,
        String::from_utf8_lossy(needle),
        harness.outcome().effects_emitted
    );
}

/// Regression: §03 — DA1 dispatched inline during Mode 2026 BSU.
///
/// Feed BSU + DA1 in one call (no ESU). The DA1 dispatches inline and
/// the `\x1b[?64;6;4c` response appears in the transcript before the
/// chunk completes — no buffering, no deferred replay.
#[test]
fn da1_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[c");
    assert_pty_response(&harness, b"\x1b[?64;6;4c", "DA1 inline inside Mode 2026");
}

/// Regression: §03 — DA2 dispatched inline during Mode 2026 BSU.
#[test]
fn da2_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[>c");
    // DA2 reply: `\x1b[>0;<version>;1c` — version is dynamic.
    assert_pty_response_contains(
        &harness,
        b"\x1b[>0;",
        "DA2 inline inside Mode 2026 (prefix)",
    );
    assert_pty_response_contains(&harness, b";1c", "DA2 inline inside Mode 2026 (suffix)");
}

/// Regression: §03 — DA3 dispatched inline during Mode 2026 BSU.
#[test]
fn da3_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[=c");
    assert_pty_response(
        &harness,
        b"\x1bP!|00000000\x1b\\",
        "DA3 inline inside Mode 2026",
    );
}

/// Regression: §03 — DSR 5 dispatched inline during Mode 2026 BSU.
#[test]
fn dsr_5_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[5n");
    assert_pty_response(&harness, b"\x1b[0n", "DSR 5 inline inside Mode 2026");
}

/// Regression: §03 — DSR 6 dispatched inline during Mode 2026 BSU.
///
/// Default cursor is at (1,1) so the response reports `\x1b[1;1R`.
#[test]
fn dsr_6_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[6n");
    assert_pty_response(&harness, b"\x1b[1;1R", "DSR 6 inline inside Mode 2026");
}

/// Regression: §03 — DECRQM ANSI mode 4 (Insert, default reset)
/// dispatched inline during Mode 2026 BSU.
#[test]
fn decrqm_ansi_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[4$p");
    // Mode 4 = Insert, default reset → value 2.
    assert_pty_response(
        &harness,
        b"\x1b[4;2$y",
        "DECRQM ANSI mode 4 inline inside Mode 2026",
    );
}

/// Regression: §03 — DECRQM private mode 2026 (set since BSU
/// just set it) dispatched inline during Mode 2026 BSU.
#[test]
fn decrqm_private_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[?2026$p");
    assert_pty_response(
        &harness,
        b"\x1b[?2026;1$y",
        "DECRQM private mode 2026 inline inside Mode 2026",
    );
}

/// Regression: §03 — DECRQM unknown private mode reports value 0
/// when queried inline during Mode 2026 BSU.
///
/// Covers the `unknown` axis of the DECRQM private matrix (Plan TPR
/// round 0 finding).
#[test]
fn decrqm_unknown_private_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[?9999$p");
    assert_pty_response(
        &harness,
        b"\x1b[?9999;0$y",
        "DECRQM unknown private mode inline inside Mode 2026",
    );
}

/// Regression: §03 — DECRQM unknown ANSI mode reports value 0
/// when queried inline during Mode 2026 BSU.
#[test]
fn decrqm_unknown_ansi_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[99$p");
    assert_pty_response(
        &harness,
        b"\x1b[99;0$y",
        "DECRQM unknown ANSI mode inline inside Mode 2026",
    );
}

/// Regression: §03 — DECRQM ANSI **set** axis. Set Insert
/// before BSU so the inline DECRQM inside sync reports value 1 (set).
///
/// Covers the ANSI-set axis (review round 1 finding F4).
#[test]
fn decrqm_ansi_set_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[4h\x1b[?2026h\x1b[4$p");
    assert_pty_response(
        &harness,
        b"\x1b[4;1$y",
        "DECRQM ANSI mode 4 set inline inside Mode 2026",
    );
}

/// Regression: §03 — DECRQM private **reset** axis. Mode 1049
/// (Alternate Screen Buffer Save Cursor) is default reset, so the inline
/// DECRQM inside sync reports value 2.
///
/// Covers the private-reset axis (review round 1 finding F4).
#[test]
fn decrqm_private_reset_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[?1049$p");
    assert_pty_response(
        &harness,
        b"\x1b[?1049;2$y",
        "DECRQM private mode 1049 reset inline inside Mode 2026",
    );
}

/// Regression: §03 — CSI 18t (text-area-size report)
/// dispatched inline during Mode 2026 BSU. Default harness size is
/// 24×80, so the response is `\x1b[8;24;80t`.
#[test]
fn csi_18t_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[18t");
    assert_pty_response(
        &harness,
        b"\x1b[8;24;80t",
        "CSI 18t inline inside Mode 2026",
    );
}

/// Regression: §03 — XTSMGRAPHICS Pi=1 Pa=1 Pv=0 (read color
/// registers) dispatched inline during Mode 2026 BSU.
///
/// Per review round 1 finding F3, the dispatcher requires exactly 3
/// top-level params. Default `image_protocol_enabled = true` and
/// `color_register_count = COLOR_REGISTERS_MAX`, so the response shape
/// is `\x1b[?1;0;<count>S` — match the prefix to keep the test stable
/// across configuration changes to the constant.
#[test]
fn xtsmgraphics_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[?1;1;0S");
    assert_pty_response_contains(
        &harness,
        b"\x1b[?1;0;",
        "XTSMGRAPHICS Pi=1 Pa=1 inline inside Mode 2026 (status=0 prefix)",
    );
}

/// Regression: §03 — DECRQSS SGR query dispatched inline
/// during Mode 2026 BSU. Default cursor template emits SGR `0` (reset),
/// so the response is `\x1bP1$r0m\x1b\\`.
#[test]
fn decrqss_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1bP$qm\x1b\\");
    assert_pty_response(
        &harness,
        b"\x1bP1$r0m\x1b\\",
        "DECRQSS SGR inline inside Mode 2026",
    );
}

/// Regression: §03 — XTVERSION dispatched inline during Mode
/// 2026 BSU. Resolves the cross-ref note (XTVERSION inside a
/// sync block now responds immediately, not at sync end).
#[test]
fn xtversion_inside_mode_2026_emits_response_inline() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[?2026h\x1b[>q");
    assert_pty_response_contains(
        &harness,
        b"\x1bP>|oriterm(",
        "XTVERSION inline inside Mode 2026",
    );
    assert_pty_response_contains(&harness, b")\x1b\\", "XTVERSION ST tail");
}
