//! Spec_chain conversion of the tack `status_reports_inventory` family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/status_reports_inventory/mod.rs`,
//! tack's `t -> s) ANSI status reports` sub-submenu walks a 33-entry
//! sequence of DA/DSR/DECRQSS probes. The framework pins a curated
//! 8-entry subset (`da1`, `dsr_status`, `dsr_cpr`, `da2`, `decrqss_first`,
//! `da3`, `decrqss_last`, `decrqde_window`) against ori_term's terminfo;
//! the spec_chain conversion below drives each DA/DSR row the curated
//! subset exercises directly through `SpecHarness`.
//!
//! # Catalog rows verified
//!
//! - `ECMA48-CSI-DA2` — Secondary Device Attributes (`CSI > c`)
//! - `ECMA48-CSI-DA3` — Tertiary Device Attributes (`CSI = c`)
//! - `ECMA48-CSI-DSR-5` — Device Status Report — operating status
//! (`CSI 5 n`)
//! - `ECMA48-CSI-DSR-6` — Cursor Position Report (`CSI 6 n`)
//!
//! DA1 (`ECMA48-CSI-DA1`) is already driven to `verified` by the
//! pilot at `oriterm_core/tests/spec_chain/pilots/da1_query.rs`.
//!
//! The DECRQSS sub-tests in the curated subset (`decrqss_first`,
//! `decrqss_last`, `decrqde_window`) drive `ECMA48-DCS-DECRQSS` — but
//! that row is intentionally left for the DECRQSS per-query coverage
//! in 08.5 (DECLRMM extended operations, which adds the DECSLRM
//! variant) and 08.8b (`ECMA48-DCS-DECRQSS-DECSLRM`). The generic
//! DECRQSS round-trip is covered in-depth by those subsections;
//! duplicating the surface-level round-trip here would be
//! `LEAK:algorithmic-duplication`.

use oriterm_core::effect::PtyWriteKind;
use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, EffectExpectation, ParserExpectation, ScenarioExpectations,
    SpecHarness, SpecScenario, pty_writes_of_kind,
};

// --- DA2 -------------------------------------------------------------------

/// DA2 (`CSI > c`) drives parser → dispatch → effect apex.
///
/// Tack's `da2` sub-test at `status_reports_inventory[3]` emits
/// `CSI > c` and validates the response format. This replay verifies
/// ori_term tokenizes the sequence with the `>` intermediate, routes
/// through `identify_terminal(Some('>'))`, and emits a
/// `PtyWriteKind::DeviceAttribute` effect carrying the DA2 reply.
#[test]
fn da2_query_drives_to_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-DA2",
        bytes: b"\x1b[>c",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'c',
                params: &[0],
                intermediates: b">",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("identify_terminal")),
            effect: Some(EffectExpectation::pty("DeviceAttribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
}

/// DA2 with explicit-zero param (`CSI > 0 c`) drives the same path.
///
/// Tack's `status_reports_inventory` fixtures label this probe as
/// `(CSI > 0 c)` (per `crates/oriterm_test_support/src/tack_framework/scenarios/status_reports/tests.rs:25-27`),
/// not the implicit-zero `CSI > c` form. This matrix cell pins the
/// verbatim tack byte sequence so a regression that accidentally
/// routes explicit-zero to a different handler can't hide behind
/// the implicit-zero green.
#[test]
fn da2_query_explicit_zero_param_drives_to_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-DA2",
        bytes: b"\x1b[>0c",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'c',
                params: &[0],
                intermediates: b">",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("identify_terminal")),
            effect: Some(EffectExpectation::pty("DeviceAttribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
}

/// Explicit-zero and implicit-zero DA2 forms emit identical replies.
///
/// Proves the two forms are behaviorally equivalent at the effect
/// layer — a regression that reported a different value for one
/// form would fail here.
#[test]
fn da2_explicit_and_implicit_zero_replies_match() {
    let mut h_implicit = SpecHarness::new();
    h_implicit.feed(b"\x1b[>c");
    let reply_implicit = pty_writes_of_kind(&h_implicit, PtyWriteKind::DeviceAttribute)
        .next()
        .map(|b| b.to_vec())
        .expect("implicit-zero DA2 should emit a reply");

    let mut h_explicit = SpecHarness::new();
    h_explicit.feed(b"\x1b[>0c");
    let reply_explicit = pty_writes_of_kind(&h_explicit, PtyWriteKind::DeviceAttribute)
        .next()
        .map(|b| b.to_vec())
        .expect("explicit-zero DA2 should emit a reply");

    assert_eq!(
        reply_implicit, reply_explicit,
        "DA2 implicit-0 (`CSI > c`) and explicit-0 (`CSI > 0 c`) must emit identical replies"
    );
}

/// DA2 reply bytes start with `ESC [ > 0 ;` (terminal type 0).
///
/// Confirms the effect transcript contains the DA2-specific prefix,
/// not the DA1 prefix (`ESC [ ?`). Without this pin, a regression
/// that accidentally returned the DA1 reply for `CSI > c` would pass
/// the `DeviceAttribute` variant check above.
#[test]
fn da2_reply_has_secondary_prefix() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[>c");

    let reply = pty_writes_of_kind(&harness, PtyWriteKind::DeviceAttribute)
        .next()
        .map(|b| b.to_vec())
        .expect("DA2 should emit a DeviceAttribute effect");

    assert!(
        reply.starts_with(b"\x1b[>0;"),
        "DA2 reply must start with `ESC [ > 0 ;` (terminal type 0); got: {reply:?}"
    );
    assert!(
        reply.ends_with(b"c"),
        "DA2 reply must terminate with `c`; got: {reply:?}"
    );
}

// --- DA3 -------------------------------------------------------------------

/// DA3 (`CSI = c`) drives parser → dispatch → effect apex.
///
/// Tack's `da3` sub-test at `status_reports_inventory[18]` emits
/// `CSI = c` and expects a DCS `!|` unit-ID response. This replay
/// verifies ori_term tokenizes the `=` intermediate, routes through
/// `identify_terminal(Some('='))`, and emits a `DeviceAttribute`
/// PTY write carrying the DCS response.
#[test]
fn da3_query_drives_to_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-DA3",
        bytes: b"\x1b[=c",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'c',
                params: &[0],
                intermediates: b"=",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("identify_terminal")),
            effect: Some(EffectExpectation::pty("DeviceAttribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
}

/// DA3 with explicit-zero param (`CSI = 0 c`) drives the same path.
///
/// Matrix cell pairing: tack's fixture labels DA3 as `(CSI = 0 c)`
/// (explicit-zero). Pins the verbatim form so regressions in the
/// explicit-zero path can't hide behind the implicit-zero green.
#[test]
fn da3_query_explicit_zero_param_drives_to_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-DA3",
        bytes: b"\x1b[=0c",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: 'c',
                params: &[0],
                intermediates: b"=",
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("identify_terminal")),
            effect: Some(EffectExpectation::pty("DeviceAttribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
}

/// DA3 reply is a DCS frame: `ESC P ! |... ESC \` (eight hex digits).
///
/// Pins the xterm-standard unit-ID format. Regressions that emit a
/// CSI response instead of DCS, or return the wrong digit count,
/// fail here.
#[test]
fn da3_reply_is_dcs_unit_id_frame() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[=c");

    let reply = pty_writes_of_kind(&harness, PtyWriteKind::DeviceAttribute)
        .next()
        .map(|b| b.to_vec())
        .expect("DA3 should emit a DeviceAttribute effect");

    assert_eq!(
        reply, b"\x1bP!|00000000\x1b\\",
        "DA3 must reply with `ESC P ! | 00000000 ESC \\`; got: {reply:?}"
    );
}

// --- DSR 5 (operating status) ----------------------------------------------

/// DSR operating status (`CSI 5 n`) drives parser → dispatch → effect apex.
///
/// Tack's `dsr_status` sub-test at `status_reports_inventory[1]`
/// emits `CSI 5 n` and expects a `CSI 0 n` (ready) response. This
/// replay verifies ori_term routes through `device_status(5)` and
/// emits a `PtyWriteKind::DeviceStatus` effect (distinct from the
/// `DeviceAttribute` kind used for DA replies).
#[test]
fn dsr_5_drives_to_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-DSR-5",
        bytes: b"\x1b[5n",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('n', &[5])),
            dispatch: Some(DispatchExpectation::method("device_status")),
            effect: Some(EffectExpectation::pty("DeviceStatus")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
}

/// DSR 5 reply is literally `ESC [ 0 n` (ready).
#[test]
fn dsr_5_reply_is_ready() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5n");

    let reply = pty_writes_of_kind(&harness, PtyWriteKind::DeviceStatus)
        .next()
        .map(|b| b.to_vec())
        .expect("DSR 5 should emit a DeviceStatus effect");

    assert_eq!(
        reply, b"\x1b[0n",
        "DSR 5 must reply with `ESC [ 0 n` (ready); got: {reply:?}"
    );
}

// --- DSR 6 (cursor position) -----------------------------------------------

/// DSR cursor position report (`CSI 6 n`) drives parser → dispatch → effect apex.
///
/// Tack's `dsr_cpr` sub-test at `status_reports_inventory[2]` emits
/// `CSI 6 n` and validates the response format `ESC [ r ; c R`. This
/// replay verifies ori_term routes through `device_status(6)` and
/// emits a `PtyWriteKind::CursorReport` effect (distinct from the
/// `DeviceStatus` kind used for DSR 5).
#[test]
fn dsr_6_drives_to_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-DSR-6",
        bytes: b"\x1b[6n",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('n', &[6])),
            dispatch: Some(DispatchExpectation::method("device_status")),
            effect: Some(EffectExpectation::pty("CursorReport")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
}

/// DSR 6 at default cursor origin replies with `ESC [ 1 ; 1 R`.
///
/// The default SpecHarness state has the cursor at (0, 0); CPR
/// reports 1-based coordinates per ECMA-48 §8.3.35, so the reply is
/// `ESC [ 1 ; 1 R`. If DECOM were active and the scroll region
/// started elsewhere, this would shift — that origin-aware path is
/// covered by origin-specific tests elsewhere in the test tree.
#[test]
fn dsr_6_reply_is_one_one_at_default_cursor() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[6n");

    let reply = pty_writes_of_kind(&harness, PtyWriteKind::CursorReport)
        .next()
        .map(|b| b.to_vec())
        .expect("DSR 6 should emit a CursorReport effect");

    assert_eq!(
        reply, b"\x1b[1;1R",
        "DSR 6 at default cursor must reply `ESC [ 1 ; 1 R`; got: {reply:?}"
    );
}

// --- Negative pins ---------------------------------------------------------

/// Regression guard: DSR 6 must emit `CursorReport`, NOT `DeviceStatus`.
///
/// Matrix pair with `dsr_5_does_not_emit_cursor_report`. Without
/// this pin, a regression that routed `CSI 6 n` to the
/// `DeviceStatus` kind (instead of `CursorReport`) would pass the
/// positive DSR 6 pin above — the positive pin only checks presence
/// of a `CursorReport`, not absence of a `DeviceStatus`.
#[test]
fn dsr_6_does_not_emit_device_status() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[6n");

    let device_status_replies: Vec<_> = pty_writes_of_kind(&harness, PtyWriteKind::DeviceStatus)
        .map(|b| b.to_vec())
        .collect();

    assert!(
        device_status_replies.is_empty(),
        "DSR 6 must not emit a DeviceStatus effect; got: {device_status_replies:?}"
    );
}

/// Regression guard: DSR 5 must emit `DeviceStatus`, NOT `CursorReport`.
///
/// Without this pin, a regression that routed both DSR variants to
/// the same `PtyWriteKind` (e.g. always `CursorReport`) would pass
/// the positive DSR 5 pin above because the bytes check is separate.
#[test]
fn dsr_5_does_not_emit_cursor_report() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5n");

    let cursor_reports: Vec<_> = pty_writes_of_kind(&harness, PtyWriteKind::CursorReport)
        .map(|b| b.to_vec())
        .collect();

    assert!(
        cursor_reports.is_empty(),
        "DSR 5 must not emit a CursorReport effect; got: {cursor_reports:?}"
    );
}

/// Regression guard: DA1 / DA2 / DA3 must emit different reply bytes.
///
/// Proves the three DA variants dispatch to different response
/// bodies based on intermediate, not a single shared reply. Without
/// this pin, a regression that ignored the intermediate and always
/// returned DA1's reply would pass the per-variant positive pins
/// above (which only check the `DeviceAttribute` kind, not payload
/// distinctness across variants).
#[test]
fn da_variants_emit_distinct_replies() {
    let mut harness_da1 = SpecHarness::new();
    harness_da1.feed(b"\x1b[c");
    let da1_reply = pty_writes_of_kind(&harness_da1, PtyWriteKind::DeviceAttribute)
        .next()
        .map(|b| b.to_vec())
        .expect("DA1 should emit a reply");

    let mut harness_da2 = SpecHarness::new();
    harness_da2.feed(b"\x1b[>c");
    let da2_reply = pty_writes_of_kind(&harness_da2, PtyWriteKind::DeviceAttribute)
        .next()
        .map(|b| b.to_vec())
        .expect("DA2 should emit a reply");

    let mut harness_da3 = SpecHarness::new();
    harness_da3.feed(b"\x1b[=c");
    let da3_reply = pty_writes_of_kind(&harness_da3, PtyWriteKind::DeviceAttribute)
        .next()
        .map(|b| b.to_vec())
        .expect("DA3 should emit a reply");

    assert_ne!(da1_reply, da2_reply, "DA1 and DA2 must differ");
    assert_ne!(da1_reply, da3_reply, "DA1 and DA3 must differ");
    assert_ne!(da2_reply, da3_reply, "DA2 and DA3 must differ");
}
