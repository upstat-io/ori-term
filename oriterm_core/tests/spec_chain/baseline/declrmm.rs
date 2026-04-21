//! Spec_chain conversion of DECLRMM (mode 69) through parser → dispatch → state.
//!
//! DECLRMM is a composite feature: the mode 69 flag gates cursor-movement
//! constraints (CUF/CUB stop at `left_margin`/`right_margin` instead of
//! screen edges). This test drives the flag + margin set + CUF movement
//! through all three apex layers, verifying the state observer sees the
//! cursor clamped to the right margin rather than the right screen edge.
//!
//! # Catalog rows verified
//!
//! - `DEC-DECLRMM` (mode 69) — left/right margin mode
//!
//! Catalog rows: XT-DECSLRM
//!
//! # Why state apex (not renderable)
//!
//! `observe_renderable` in `crates/oriterm_test_support/src/spec_chain/observers/renderable.rs`
//! is currently a stub that unconditionally returns pass. Using the
//! Renderable apex would produce a false-green rung 4. `ApexLayer::State`
//! verifies the observable cursor position change caused by the margin
//! constraint, which is the load-bearing behavior change this mode
//! enables.

use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, ParserExpectation, ScenarioExpectations, SpecHarness,
    SpecScenario, StateExpectation,
};

/// CUF under DECLRMM clamps cursor to `right_margin`, not screen edge.
///
/// Setup:
/// - DECSET ?69 enables DECLRMM (mode 69)
/// - DECSLRM `\x1b[6;41s` sets margins to left=5, right=40 (0-based)
/// - CUP `\x1b[1;10H` positions cursor at line 0, col 9 (inside band)
///
/// Test bytes: `\x1b[100C` — CUF by 100. Without DECLRMM cursor would
/// clamp to col 79. With DECLRMM active and cursor inside the margin
/// band, cursor clamps to `right_margin` = 40.
#[test]
fn declrmm_cuf_clamps_to_right_margin() {
    let scenario = SpecScenario {
        catalog_row_id: "DEC-DECLRMM",
        bytes: b"\x1b[100C",
        apex_layer: ApexLayer::State,
        setup: b"\x1b[?69h\x1b[6;41s\x1b[1;10H",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('C', &[100])),
            dispatch: Some(DispatchExpectation::method("move_forward")),
            state: Some(StateExpectation::cursor_at(0, 40)),
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
