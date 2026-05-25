//! iTerm2 inline image visual pilot — `Npx` pixel-suffix sizing.
//!
//! Catalog row: `ITERM2-1337-FILE-DIM-PIXELS`
//! Apex: `GoldenImage`
//!
//! Drives `OSC 1337 ; File=inline=1;width=100px;height=100px:<png-b64> ST`
//! and asserts every visual rung passes. Pairs with the state-snapshot
//! pins `osc1337_file_width_px_suffix_is_pixels` /
//! `osc1337_file_height_px_suffix_is_pixels` per Decision 05
//! §Consequences dual-gate. Exercises `SizeSpec::Pixels` at
//! `iterm2/mod.rs:156-158` and `PlacementSizing::FixedPixels` at
//! `iterm2.rs:165-168`.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// `OSC 1337 ; File=inline=1;width=100px;height=100px:<b64> ST` — 32×32
/// PNG forced into a 100×100 device-pixel placement.
const ITERM2_DIM_PIXELS_BYTES: &[u8] = b"\x1b]1337;File=inline=1;width=100px;height=100px:iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAAK0lEQVR42u3OIQEAAAwEoetfeovxBoGnq1tKQEBAQEBAQEBAQEBAQEBgHXhUDfhqeP5ugAAAAABJRU5ErkJggg==\x1b\\";

#[test]
fn iterm2_image_dim_pixels_suffix_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "ITERM2-1337-FILE-DIM-PIXELS",
        bytes: ITERM2_DIM_PIXELS_BYTES,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 0).with_images(1)),
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some("iterm2_image_dim_pixels_at_cursor"),
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
