//! Kitty unicode-placeholder visual pilot — minimal `U=1` transmit +
//! `U+10EEEE` cell rendering through every rung.
//!
//! Catalog row: KG-UNICODE-PLACEHOLDER-CELL-RESOLVE
//! Apex: GoldenImage
//!
//! Drives a `U=1,a=T` transmit followed by a single placeholder cell
//! with diacritic-encoded row/col + fg color carrying `image_id_low`.
//! Establishes that the GPU pipeline can resolve U=1 placeholder cells
//! back to the cached image bytes and emit an image quad.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Transmit `a=T,U=1,i=1` with a 1×1 red RGBA pixel, then write a single
/// placeholder cell at the cursor (fg=Red=Indexed(1), row + col both
/// diacritic 0).
///
/// Breakdown:
/// - `\x1b_G` — APC introducer for kitty graphics
/// - `a=T,U=1,i=1,f=24,s=1,v=1,c=1,r=1,q=2` — transmit-and-place virtual
///   1×1 image, suppress all replies
/// - `;AP8A` — base64 for RGB `0x00 0xFF 0x00` (green pixel); `f=24` =
///   24-bit RGB packed
/// - `\x1b\\` — ST
/// - `\x1b[38;5;1m\u{10EEEE}\u{0305}\u{0305}\x1b[39m` — placeholder cell
///   with fg color = palette 1 (matching `i=1` low bits), row diacritic
///   = 0, col diacritic = 0.
const PLACEHOLDER_BYTES: &[u8] = b"\x1b_Ga=T,U=1,i=1,f=24,s=1,v=1,c=1,r=1,q=2;AP8A\x1b\\\x1b[38;5;1m\xf4\x8e\xbb\xae\xcc\x85\xcc\x85\x1b[39m";

/// Pilot test: drives transmit + placeholder cell through every rung.
#[test]
fn kitty_placeholder_basic_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "KG-UNICODE-PLACEHOLDER-CELL-RESOLVE",
        bytes: PLACEHOLDER_BYTES,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            // At least one image quad emitted from the placeholder cell.
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some("kitty_placeholder_basic"),
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
