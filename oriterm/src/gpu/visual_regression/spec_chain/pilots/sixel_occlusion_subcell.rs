//! Sixel + subcell-glyphs occlusion pilot (§12.3 §11-occlusion rung).
//!
//! Catalog row: SIXEL-Z-ORDER (subcell-glyph variant)
//! Apex: GoldenImage
//!
//! Writes a mix of half-block / quadrant / sextant subcell glyphs
//! (`▀▌▞▙` — U+2580 U+258C U+259E U+2599) at row 0, then emits a sixel
//! image overlapping those cells. The §11 subcell-glyph families are
//! rendered as built-in glyphs via `oriterm/src/gpu/builtin_glyphs`,
//! so this pilot pins z-order across the custom glyph-rendering path
//! (rather than the swash-font glyph path the wide-CJK + ZWJ pilots
//! exercise).

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Setup: write subcell glyphs + reset cursor to home.
///
/// UTF-8: `▀` = E2 96 80 (upper half-block), `▌` = E2 96 8C (left
/// half-block), `▞` = E2 96 9E (diagonal quadrant), `▙` = E2 96 99
/// (3-quadrant). 12 bytes + 3 bytes for `ESC [ H`.
const SETUP: &[u8] = b"\xE2\x96\x80\xE2\x96\x8C\xE2\x96\x9E\xE2\x96\x99\x1b[H";

/// Same 20×12 pixel magenta sixel as the other occlusion pilots.
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;0;100#0!20~-#0!20~\x1b\\";

#[test]
fn sixel_occlusion_subcell_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-Z-ORDER",
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
                golden_name: Some("sixel_occlusion_subcell"),
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
    assert_eq!(results.len(), 8);
}
