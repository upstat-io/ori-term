//! Sixel visual pilot — Scenario D: `$` CR + `-` NL banding interaction.
//!
//! Catalog rows: SIXEL-CR, SIXEL-NL (§12.4 GPU-apex)
//! Apex: GoldenImage
//!
//! Four-band composition exercising both the `$` (graphic carriage
//! return — reset x to 0, keep y) and `-` (graphic newline — reset x to
//! 0, advance y by 6) operators in a single DCS. Forces `$` + `-` into
//! the raster output, not just the state machine.
//!
//! Layout (in sixel-native coordinates, each sixel row = 6 pixels tall):
//!
//! ```text
//! y=0..5: red (cols 0-9) overlaid with green (cols 0-9 via $)
//! y=6..11: blue (cols 0-9) overlaid with yellow (cols 0-9 via $)
//! ```
//!
//! The overlay semantics of `$` are that the *second* color's set bits
//! overwrite the first color's set bits at the same pixel position —
//! pinned as: green-on-red becomes green, yellow-on-blue becomes yellow.
//! A bug where `$` advanced y (instead of resetting x) would split the
//! bands into 4 separate rows — different golden.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Four-band sequence:
/// - `#0;2;100;0;0` red + `#0!10~` 10 cols
/// - `$` CR (reset x=0, keep y=0)
/// - `#1;2;0;100;0` green + `#1!10~` 10 cols (overlays red)
/// - `-` NL (x=0, y=6)
/// - `#2;2;0;0;100` blue + `#2!10~`
/// - `$` CR
/// - `#3;2;100;100;0` yellow + `#3!10~` (overlays blue)
const SIXEL_BYTES: &[u8] = b"\x1bPq\
#0;2;100;0;0#0!10~\
$#1;2;0;100;0#1!10~\
-#2;2;0;0;100#2!10~\
$#3;2;100;100;0#3!10~\
\x1b\\";

/// Scenario D — CR + NL banding pilot.
///
/// Pins that `$` resets x without advancing y and `-` advances y by 6.
/// The golden composition has two bands (one per sixel row), each with
/// the later color's bits on top — regression guard for the `$` vs `-`
/// dispatch in `SixelParser::feed`.
#[test]
fn sixel_cr_nl_banding_interaction_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-CR",
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
                golden_name: Some("sixel_cr_nl_banding"),
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
