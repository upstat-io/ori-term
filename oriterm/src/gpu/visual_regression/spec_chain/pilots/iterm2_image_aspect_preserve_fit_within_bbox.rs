//! iTerm2 inline image visual pilot — `preserveAspectRatio=1` fit-within-bbox.
//!
//! Catalog row: `ITERM2-1337-FILE-ASPECT-PRESERVE`
//! Apex: `GoldenImage`
//!
//! Drives `OSC 1337 ; File=inline=1;width=10;height=5;preserveAspectRatio=1
//! :<100x50-png-b64> ST` and asserts every visual rung passes. Pairs
//! with the state-snapshot pin
//! `osc1337_file_aspect_preserve_explicit_explicit_fits_within_bbox`
//! per Decision 05 §Consequences dual-gate. Exercises the fix at
//! `iterm2.rs:252-275` (false, false) arm — scale-to-fit (NOT stretch)
//! when `preserve_aspect == true` AND both width AND height explicit.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// `OSC 1337 ; File=inline=1;width=10;height=5;preserveAspectRatio=1:<b64>
/// ST` — 100×50 PNG fit within a 10-col × 5-row bbox preserving aspect.
const ITERM2_ASPECT_PRESERVE_BYTES: &[u8] = b"\x1b]1337;File=inline=1;width=10;height=5;preserveAspectRatio=1:iVBORw0KGgoAAAANSUhEUgAAAGQAAAAyCAYAAACqNX6+AAAAYElEQVR42u3RIQEAAAjAsPcvDRWQiIkX+Joa/SkTgAgIEAEBIiBABASIEUAEBIiAABEQIAICREAEBIiAABEQIAICREAEBIiAABEQIAICREAEBIiAABEQIAICREAEBIiuLXnB6yufCAPiAAAAAElFTkSuQmCC\x1b\\";

#[test]
fn iterm2_image_aspect_preserve_fit_within_bbox_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "ITERM2-1337-FILE-ASPECT-PRESERVE",
        bytes: ITERM2_ASPECT_PRESERVE_BYTES,
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
                golden_name: Some("iterm2_image_aspect_preserve_fit_within_bbox"),
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
