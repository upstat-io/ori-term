//! Sixel visual pilot — Scenario B: palette-switch mid-image.
//!
//! Catalog row: SIXEL-DCS-unhook (§12.4 GPU-apex)
//! Apex: GoldenImage
//!
//! Emits two color definitions + two color selects on live sixel placement:
//! `#0;2;100;0;0<red-row>` then `-` (NL) then `#1;2;0;100;0<green-row>`.
//! Forces `#` dispatch on live placement — catches the parser-correct /
//! decoder-wrong seam where a `#` selector would reach the parser but fail
//! to switch the emission color on the pixel buffer.
//!
//! Pin: the golden shows a red sixel band on top and a green sixel band
//! below. If `#1` selection is swallowed, the golden would show two red
//! bands (or two green) — a different pixel layout that fails the 0-pixel
//! diff gate.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Two-band palette-switch DCS sequence:
/// - `#0;2;100;0;0` define color 0 = red
/// - `#0!10~` select 0, emit 10 cols of value `~` (full column)
/// - `-` sixel NL (advance y by 6)
/// - `#1;2;0;100;0` define color 1 = green
/// - `#1!10~` select 1, emit 10 cols of value `~`
const SIXEL_BYTES: &[u8] = b"\x1bPq#0;2;100;0;0#0!10~-#1;2;0;100;0#1!10~\x1b\\";

/// Scenario B — palette-switch mid-image pilot.
///
/// Pins that a `#N` color selector that appears mid-DCS actually changes
/// the emission color for subsequent data bytes, not just the parser
/// state. Regression guard for `SixelParser::apply_color` vs
/// `SixelParser::feed` seam.
#[test]
fn sixel_palette_switch_mid_image_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-DCS-unhook",
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
                golden_name: Some("sixel_palette_switch"),
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
