//! Sixel visual pilot — Scenario C: `!` repeat optimization.
//!
//! Catalog row: SIXEL-REPEAT (§12.4 GPU-apex)
//! Apex: GoldenImage
//!
//! Emits `!100~` — repeat value `~` (full 6-bit column) 100 times. Asserts
//! the golden shows 100 equal-valued columns, pinning `emit_sixel(value,
//! count)` against per-column decode drift. A regression where the
//! repeat operator decremented color state, skipped columns, or diverged
//! after column N would produce a visually different band and fail the
//! 0-pixel diff gate.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// `!100~` — 100 equal-valued columns of full-6-bit sixel (`~` = 0b111111).
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;100;100#0!100~\x1b\\";

/// Scenario C — repeat-operator pilot.
///
/// Pins `SixelParser::emit_sixel(value, count)` on live placement: 100
/// columns must be bit-identical. Drift would manifest as a visible
/// discontinuity in the band and fail the strict golden comparison.
#[test]
fn sixel_repeat_emits_one_hundred_equal_columns() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-REPEAT",
        bytes: SIXEL_BYTES,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some("sixel_repeat"),
            }),
            ..ScenarioExpectations::default()
        },
    };

    let results = harness.run_visual_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
    assert_eq!(results.len(), 8, "GoldenImage apex produces 8 rung results");
}
