//! Sixel visual pilot — Scenario F: SIXEL_SCROLLING OFF (DECRST 80).
//!
//! Catalog row: SIXEL-MODE-80 (§12.4 GPU-apex)
//! Apex: GoldenImage
//!
//! Visually pins §12.3's behavioral bullet: with DECRST 80
//! (SIXEL_SCROLLING off), the cursor stays where the sixel started and
//! subsequent text overwrites the image cells. Programmatic pin lives in
//! `sixel_scrolling_off_via_decrst_80_keeps_cursor_at_home`
//! (`oriterm_core/tests/spec_chain/sixel/grid_integration.rs`); this
//! pilot adds a golden-image check that the overwrite is visible at the
//! rendered pixel level — not just the cursor position.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Setup: DECRST 80 (SIXEL_SCROLLING off) + cursor to (10,5).
const SETUP: &[u8] = b"\x1b[?80l\x1b[10;5H";

/// Sixel payload followed by "OVERWRITE" — since cursor stays at (10,5)
/// after sixel with scrolling off, the text overwrites image cells.
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;100;100#0!10~-#0!10~\x1b\\OVERWRITE";

#[test]
fn sixel_scrolling_off_decrst_80_overwrite_pinned_visually() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-MODE-80",
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
                golden_name: Some("sixel_scrolling_off"),
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
