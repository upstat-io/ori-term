//! Sixel visual pilot — Scenario E: P2=1 transparency composite.
//!
//! Catalog row: SIXEL-BG-NoChange (§12.4 GPU-apex)
//! Apex: GoldenImage
//!
//! Emits a DCS q with P2=1 (NoChange — undrawn pixels have α=0) and
//! partial pixel coverage: 10 filled cols, 10 transparent cols (`?` =
//! value 0), 10 filled cols. The transparent band must show the
//! deterministic background from §05 through the image. GPU image_render
//! composites RGBA-premultiplied content over the background; α=0 must
//! leave those pixels visually identical to the bg-only baseline.
//!
//! Explicit sub-pixel / AA gate: 0-pixel diff against the committed
//! golden on the deterministic lane. Any non-zero diff IS a bug — either
//! in transparency compositing at `oriterm/src/gpu/image_render/mod.rs`
//! or in §05's cell-metrics pinning.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// DCS with P1=0, P2=1 (NoChange / transparent bg):
/// - `#0;2;100;100;100` define white
/// - `#0!10~` 10 cols filled
/// - `!10?` 10 cols empty (value 0) — stays transparent under P2=1
/// - `#0!10~` 10 more cols filled
const SIXEL_BYTES: &[u8] = b"\x1bP0;1q#0;2;100;100;100#0!10~!10?#0!10~\x1b\\";

/// Scenario E — transparency composite pilot.
///
/// Pins that P2=1 leaves α=0 undrawn pixels visually transparent (bg
/// shows through) after GPU compositing. Regression guard for
/// `image_render/mod.rs` alpha handling and §05 cell-metrics pinning.
#[test]
fn sixel_transparency_p2_one_preserves_background_through_undrawn_pixels() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-BG-NoChange",
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
                golden_name: Some("sixel_transparency"),
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
