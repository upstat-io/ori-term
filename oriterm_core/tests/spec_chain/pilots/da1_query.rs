//! DA1 (Device Attributes Primary) non-visual pilot.
//!
//! Drives `CSI c` through the verification chain and asserts each
//! applicable rung: parser tokenizes the CSI sequence, dispatch routes
//! to `identify_terminal`, and the effect transcript captures
//! `PtyEffect::Write { kind: PtyWriteKind::DeviceAttribute }`.
//!
//! Catalog row: ECMA48-CSI-DA1
//! Apex: EffectPtyWrite

use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, EffectExpectation, ParserExpectation, RungName,
    ScenarioExpectations, SpecHarness, SpecScenario,
};

/// DA1 pilot: `CSI c` → parser → dispatch → effect apex.
///
/// Validates the harness MVP for non-visual sequences with PTY-reply
/// apex. DA1 is chosen because it has a well-defined, stable reply
/// (`ESC [ ? 64 ; 6 ; 4 c`) and exercises the full non-visual chain.
#[test]
fn da1_query_drives_to_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-DA1",
        bytes: b"\x1b[c",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('c', &[0])),
            dispatch: Some(DispatchExpectation::method("identify_terminal")),
            effect: Some(EffectExpectation::pty("DeviceAttribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);

    // All three rungs (Parser, Dispatch, Effect) should pass.
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    // Verify all expected rungs were reached.
    let rung_names: Vec<_> = results.iter().map(|r| r.rung_name).collect();
    assert!(
        rung_names.contains(&RungName::Parser),
        "Parser rung missing from results"
    );
    assert!(
        rung_names.contains(&RungName::Dispatch),
        "Dispatch rung missing from results"
    );
    assert!(
        rung_names.contains(&RungName::Effect),
        "Effect rung missing from results"
    );
}

/// DA1 reply bytes are the VT420 + ANSI color + sixel attribute string.
///
/// Verifies the effect transcript contains the correct reply bytes,
/// not just the correct `PtyWriteKind` discriminator.
#[test]
fn da1_reply_bytes_match_vt420_attributes() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[c");

    let (bytes, kind) = oriterm_test_support::spec_chain::pty_writes(&harness)
        .next()
        .expect("DA1 should emit a Pty effect");
    assert_eq!(
        kind,
        oriterm_core::effect::PtyWriteKind::DeviceAttribute,
        "DA1 reply should be PtyWriteKind::DeviceAttribute"
    );
    // VT420 conformance level (64) with ANSI color (6) and sixel (4).
    assert_eq!(
        bytes, b"\x1b[?64;6;4c",
        "DA1 reply should be ESC [ ? 64 ; 6 ; 4 c"
    );
}

/// DA1 scenario with no parser expectation still passes dispatch + effect.
///
/// Proves the harness correctly skips rungs with `None` expectations.
#[test]
fn da1_skips_parser_rung_when_no_expectation() {
    let scenario = SpecScenario {
        catalog_row_id: "TEST-DA1-SKIP-PARSER",
        bytes: b"\x1b[c",
        apex_layer: ApexLayer::EffectPtyWrite,
        setup: b"",
        expectations: ScenarioExpectations {
            // Parser expectation is None — should pass unconditionally.
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
