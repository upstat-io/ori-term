//! Sixel visual pilot — minimal raster fill through every rung.
//!
//! Catalog row: SIXEL-DCS-q
//! Apex: GoldenImage
//!
//! Drives a minimal sixel sequence (DCS q ... ST that paints a solid
//! rectangle) through every visual rung and asserts each rung passes.
//! This pilot establishes the harness MVP for visual sequences.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, ParserExpectation,
    RungName, ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Minimal sixel DCS sequence: paint a 10×12 light gray rectangle.
///
/// Breakdown:
/// - `\x1bP` — DCS introducer
/// - `q`     — Sixel graphics mode
/// - `#0;2;100;100;100` — Set color 0 to RGB(100%, 100%, 100%) = white
/// - `#0`    — Select color 0
/// - `!10~`  — Repeat sixel `~` (0b111111) 10 times → 10 cols × 6 rows
/// - `-`     — Graphics new line
/// - `#0!10~`— Same pattern on next sixel row
/// - `\x1b\\` — ST (String Terminator)
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;100;100#0!10~-#0!10~\x1b\\";

/// Pilot test: drives the minimal sixel through every rung (1-8).
///
/// Uses the deterministic golden lane (Section 05) via `VisualSpecHarness`.
/// The golden image is captured with `ORITERM_UPDATE_GOLDEN=1` and
/// committed under `oriterm/tests/references/`.
#[test]
fn sixel_minimal_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-DCS-q",
        bytes: SIXEL_BYTES,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            // Per-rung expectations strengthen the property beyond
            // the golden apex. Rungs with stub expectation types
            // (dispatch, state, renderable) pass unconditionally until
            // their types gain assertion fields in later sections.
            parser: Some(ParserExpectation::dcs('q', &[0])),
            frame_input: Some(FrameInputExpectation::default_grid()),
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
            texture: Some(TextureExpectation {
                // Floor guard: catches uninitialized render targets (all
                // zeros). Note: with PALETTE_BG = Rgb(1,1,1), background
                // pixels ARE non-zero — this guards against a completely
                // blank (transparent) target, not against missing content.
                // The golden comparison is the content assertion.
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some("sixel_minimal"),
            }),
            ..ScenarioExpectations::default()
        },
    };

    let results = harness.run_visual_scenario(&scenario);

    // Every rung must pass.
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    // Verify we drove through all 8 rungs to GoldenImage apex.
    let rung_names: Vec<_> = results.iter().map(|r| r.rung_name).collect();
    assert_eq!(
        rung_names.len(),
        8,
        "GoldenImage apex should produce exactly 8 rung results, got: {rung_names:?}"
    );
    assert_eq!(
        *rung_names.last().unwrap(),
        RungName::GoldenImage,
        "last rung should be GoldenImage"
    );
}
