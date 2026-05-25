//! iTerm2 inline image visual pilot — `N%` terminal-fraction sizing.
//!
//! Catalog row: `ITERM2-1337-FILE-DIM-PERCENT`
//! Apex: `GoldenImage`
//!
//! Drives `OSC 1337 ; File=inline=1;width=25%;height=25%:<png-b64> ST`
//! and asserts every visual rung passes. Pairs with the state-snapshot
//! pins `osc1337_file_width_percent_is_terminal_fraction` /
//! `osc1337_file_height_percent_is_terminal_fraction` per Decision 05
//! §Consequences dual-gate. Exercises `SizeSpec::Percent` at
//! `iterm2/mod.rs:160-162` and the terminal-fraction multiplication
//! at `iterm2.rs:263` (term_size * pct / 100).

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// `OSC 1337 ; File=inline=1;width=25%;height=25%:<b64> ST` — 32×32
/// PNG forced into a 25%-of-terminal placement.
const ITERM2_DIM_PERCENT_BYTES: &[u8] = b"\x1b]1337;File=inline=1;width=25%;height=25%:iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAAK0lEQVR42u3OIQEAAAwEoetfeovxBoGnq1tKQEBAQEBAQEBAQEBAQEBgHXhUDfhqeP5ugAAAAABJRU5ErkJggg==\x1b\\";

#[test]
fn iterm2_image_dim_percent_terminal_fraction_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "ITERM2-1337-FILE-DIM-PERCENT",
        bytes: ITERM2_DIM_PERCENT_BYTES,
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
                golden_name: Some("iterm2_image_dim_percent_at_cursor"),
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
