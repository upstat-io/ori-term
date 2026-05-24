//! iTerm2 inline image visual pilot — `width=auto` native sizing.
//!
//! Catalog row: `ITERM2-1337-FILE-DIM-AUTO`
//! Apex: `GoldenImage`
//!
//! Drives `OSC 1337 ; File=inline=1:<32x32-png-b64> ST` (no width=) and
//! asserts every visual rung passes. Pairs with the state-snapshot pin
//! `osc1337_file_width_auto_uses_native_size` /
//! `osc1337_file_height_auto_uses_native_size` per Decision 05
//! §Consequences dual-gate.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// `OSC 1337 ; File=inline=1:<b64> ST` with the b64 of a 32×32 opaque-red
/// RGBA PNG. Native size drives the auto-sizing arm at
/// `iterm2.rs:209-211` (resolve_one_dimension SizeSpec::Auto -> native).
const ITERM2_DIM_AUTO_BYTES: &[u8] = b"\x1b]1337;File=inline=1:iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAAK0lEQVR42u3OIQEAAAwEoetfeovxBoGnq1tKQEBAQEBAQEBAQEBAQEBgHXhUDfhqeP5ugAAAAABJRU5ErkJggg==\x1b\\";

#[test]
fn iterm2_image_dim_auto_native_size_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "ITERM2-1337-FILE-DIM-AUTO",
        bytes: ITERM2_DIM_AUTO_BYTES,
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
                golden_name: Some("iterm2_image_dim_auto_at_cursor"),
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
