//! Spec_chain conversion of the tack `acs` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/acs/mod.rs`
//! and the empirical capture against tack v1.08 (2026-04-08), the
//! "alternate character set" test on tack v1.08 actually probes only the
//! `bel` capability. The captured output is:
//!
//! ```text
//! \x1B[H\x1B[2JTesting bell (bel)
//! If you did not hear the Bell then (bel) has failed.  (bel) Done
//! ```
//!
//! The `(bel)` parenthesized cap label means tack emitted a literal `BEL`
//! (`0x07`) byte and verified the terminal handled it. This module replays
//! that probe at the protocol layer through `SpecHarness`.
//!
//! # Catalog rows verified
//!
//! - `ECMA48-C0-BEL` — Bell control character (`0x07`) → `Effect::Host(HostEffect::Bell)`
//!
//! ACS line-drawing rendering itself (`ESC ( 0` charset designation) is
//! NOT exercised by tack v1.08's ACS test (the parser preserves a
//! line-drawing-char counter as forward-compatible infrastructure but the
//! count is always 0 against tack v1.08). Charset designation rows
//! (`ECMA48-ESC-0`, `ECMA48-ESC-B`) are owned by Section 18 (Charsets).

use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, EffectExpectation, ParserExpectation, ScenarioExpectations,
    SpecHarness, SpecScenario,
};

/// BEL drives the parser, dispatch, and Host effect rungs.
///
/// `tack` section 05 ACS scenario emits `\x07` (BEL) during its `(bel)`
/// probe; the spec_chain replay verifies the terminal:
/// 1. Tokenizes the byte as a ground execute (parser rung)
/// 2. Routes to `Handler::bell` (dispatch rung)
/// 3. Emits `Effect::Host(HostEffect::Bell)` (effect rung)
#[test]
fn bel_drives_to_host_effect_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-C0-BEL",
        bytes: b"\x07",
        apex_layer: ApexLayer::EffectHostNotification,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation {
                action: '\x07',
                params: &[],
                intermediates: &[],
                osc_command: None,
            }),
            dispatch: Some(DispatchExpectation::method("bell")),
            effect: Some(EffectExpectation::host()),
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

/// Negative pin: a non-BEL byte must NOT emit a Host effect.
///
/// Proves the dispatch routing is BEL-specific, not "any C0 control
/// emits Host". Without this, a regression that fired Host on every
/// C0 byte would silently pass the positive pin above.
#[test]
fn non_bel_c0_does_not_emit_host_effect() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x08"); // BS — explicitly NOT bell

    let host_effects: Vec<_> = harness
        .outcome()
        .effects_emitted
        .iter()
        .filter(|eff| matches!(eff, oriterm_core::effect::Effect::Host(_)))
        .collect();

    assert!(
        host_effects.is_empty(),
        "BS (0x08) must not emit a Host effect; got: {host_effects:?}"
    );
}
