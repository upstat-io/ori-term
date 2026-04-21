//! Sixel + wide-CJK occlusion pilot (§12.3 §11-occlusion rung).
//!
//! Catalog row: SIXEL-Z-ORDER (wide-CJK variant)
//! Apex: GoldenImage
//!
//! Writes two wide CJK glyphs (`你好`, 4 cells) at row 0 col 0, then
//! emits a sixel image over the same region. The sixel placement uses
//! `z_index: 0` (`Term::sixel_create_placement` in
//! `oriterm_core/src/term/handler/image/sixel.rs`) and non-negative
//! `z_index` draws above text in the GPU emit path
//! (`oriterm/src/gpu/prepare/emit.rs`). The golden pins that the
//! sixel occludes the CJK glyphs deterministically.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Setup: write the wide CJK string `你好` (2 wide glyphs = 4 cells)
/// in the default color, then move cursor back to home.
///
/// `你` = U+4F60 (width 2), `好` = U+597D (width 2). UTF-8:
/// E4 BD A0 E5 A5 BD — 6 bytes.
const SETUP: &[u8] = b"\xE4\xBD\xA0\xE5\xA5\xBD\x1b[H";

/// Main payload: emit a sixel (20 pixels wide × 12 tall, 2 bands of
/// repeated `~`) in foreground magenta. At harness cell dimensions
/// (8×16) this covers ceil(20/8) = 3 cells wide × ceil(12/16) = 1
/// cell tall — overlapping the first 3 cells of the CJK row.
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;0;100#0!20~-#0!20~\x1b\\";

#[test]
fn sixel_occlusion_wide_cjk_drives_every_rung_green() {
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
                golden_name: Some("sixel_occlusion_wide_cjk"),
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
