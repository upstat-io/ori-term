//! Sixel + ZWJ-cluster occlusion pilot (§12.3 §11-occlusion rung).
//!
//! Catalog row: SIXEL-Z-ORDER (ZWJ-cluster variant)
//! Apex: GoldenImage
//!
//! Writes a ZWJ-joined emoji cluster (`👨‍💻` — U+1F468 U+200D U+1F4BB,
//! "man technologist") at row 0, then emits a sixel image overlapping
//! the cluster's cells. Complements the wide-CJK pilot by exercising a
//! glyph-cluster path (width-2 base + ZWJ + width-2 trailer collapsing
//! to a single grapheme cluster) rather than a plain wide glyph.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Setup: write the ZWJ emoji cluster then reset cursor to home.
///
/// UTF-8 bytes for U+1F468 (man), U+200D (ZWJ), U+1F4BB (laptop):
/// F0 9F 91 A8 E2 80 8D F0 9F 92 BB — 11 bytes, then `ESC [ H` home.
const SETUP: &[u8] = b"\xF0\x9F\x91\xA8\xE2\x80\x8D\xF0\x9F\x92\xBB\x1b[H";

/// Same 20×12 pixel magenta sixel as the wide-CJK pilot.
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;0;100#0!20~-#0!20~\x1b\\";

#[test]
fn sixel_occlusion_zwj_drives_every_rung_green() {
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
                golden_name: Some("sixel_occlusion_zwj"),
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
