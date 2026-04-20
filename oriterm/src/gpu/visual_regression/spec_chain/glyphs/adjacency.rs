//! Braille/octant adjacency test — proves no visual or stateful interference
//! between the two 2×4 subcell renderers.
//!
//! Braille (U+2800..=U+28FF) and octants (U+1CD00..=U+1CDE5) both divide the
//! cell into a 2×4 grid but use different bit orderings AND different visual
//! idioms: braille draws small dots; octants fill subcell rectangles. A
//! renderer that inadvertently shared state, reused a cached bitmap, or
//! routed one family through the other's dispatch would produce a visually
//! wrong pair when both codepoints are rendered side-by-side.
//!
//! The test renders U+28FF (braille with all 8 dots) in column 0 and
//! U+1CDE5 (octant end-of-range) in column 1 and pins the resulting frame
//! against a committed golden PNG. Pixel-exact equality proves the two
//! renderers produce their canonical patterns in the same frame without
//! interference.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// U+28FF then U+1CDE5 — adjacent in row 0, columns 0 and 1.
const ADJACENCY_BYTES: &[u8] = "\u{28FF}\u{1CDE5}".as_bytes();

#[test]
fn braille_and_octant_adjacent_in_same_row_pixel_exact_golden() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "USC-LEGACY-OCTANT",
        bytes: ADJACENCY_BYTES,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            // Two cells with built-in glyphs → at least 2 glyph instances.
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 2)),
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some("subcell_braille_octant_adjacency"),
            }),
            ..ScenarioExpectations::default()
        },
    };

    let results = harness.run_visual_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "adjacency rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)"),
        );
    }
    assert_eq!(
        results.last().map(|r| r.rung_name),
        Some(RungName::GoldenImage),
        "adjacency scenario must drive through GoldenImage apex",
    );
}
