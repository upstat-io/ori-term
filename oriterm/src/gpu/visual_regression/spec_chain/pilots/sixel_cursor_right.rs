//! Sixel visual pilot — Scenario G: SIXEL_CURSOR_RIGHT (DECSET 8452).
//!
//! Catalog row: SIXEL-MODE-8452 (§12.4 GPU-apex)
//! Apex: GoldenImage
//!
//! Visually pins §12.3's behavioral bullet: with DECSET 8452
//! (SIXEL_CURSOR_RIGHT on), the cursor advances to the column right of
//! the image and subsequent text lands to the right of the image band.
//! Programmatic pin lives in
//! `sixel_cursor_right_via_decset_8452_moves_cursor_right_of_image`
//! (`oriterm_core/tests/spec_chain/sixel/grid_integration.rs`); this
//! pilot adds the rendered-pixel proof.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Setup: DECSET 8452 (SIXEL_CURSOR_RIGHT on) + cursor to (10,5).
const SETUP: &[u8] = b"\x1b[?8452h\x1b[10;5H";

/// Sixel payload followed by "FOLLOW" — cursor moves to right of image
/// after sixel, so text lands after the image band on the same row.
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;100;100#0!10~-#0!10~\x1b\\FOLLOW";

#[test]
fn sixel_cursor_right_decset_8452_follow_text_pinned_visually() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-MODE-8452",
        bytes: SIXEL_BYTES,
        apex_layer: ApexLayer::GoldenImage,
        setup: SETUP,
        expectations: ScenarioExpectations {
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some("sixel_cursor_right"),
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
