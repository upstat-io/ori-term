//! Spec_chain visual tests for every Unicode subcell-glyph family (§11.2).
//!
//! Families covered:
//! - Box drawing (U+2500..=U+257F)
//! - Block elements incl. half blocks + eighths + shades + quadrants (U+2580..=U+259F)
//! - Sextants (U+1FB00..=U+1FB3B)
//! - Octants (U+1CD00..=U+1CDE5)
//! - Braille (U+2800..=U+28FF)
//!
//! Every test runs through `VisualSpecHarness` at the `GoldenImage` apex
//! and uses `render_frame_cached()` — the production render path — per
//! cached render testing`.
//!
//! Catalog row IDs map to `plans/spec-conformance/catalog/unicode-subcell.md`.

pub mod adjacency;
pub mod precedence;
pub mod semantic_raster;
pub mod sparse_goldens;

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    RungResult, ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::visual_harness::VisualSpecHarness;

/// Helper: drive a single-codepoint scenario through every visual rung and
/// assert the `GoldenImage` apex is reached.
///
/// Emits the UTF-8 bytes for `ch` at the current cursor position, then runs
/// the scenario with `GoldenImage` apex. The golden PNG is stored at
/// `oriterm/tests/references/<golden_name>.png` and regenerated via
/// `ORITERM_UPDATE_GOLDEN=1`.
///
/// Returns the per-rung results vec so callers can make additional
/// assertions (e.g., non-zero pixels).
#[must_use]
pub(super) fn run_glyph_scenario(
    harness: &mut VisualSpecHarness,
    ch: char,
    golden_name: &'static str,
    catalog_row_id: &'static str,
    bytes: &'static [u8],
) -> Vec<RungResult> {
    let scenario = SpecScenario {
        catalog_row_id,
        bytes,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            // One cell drawn, non-zero glyph instances expected.
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 1)),
            texture: Some(TextureExpectation {
                // Floor guard: cell must have rendered SOMETHING (built-in
                // Canvas ink counts). If the texture is completely zero,
                // the built-in dispatch did not fire.
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some(golden_name),
            }),
            ..ScenarioExpectations::default()
        },
    };

    let results = harness.run_visual_scenario(&scenario);

    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed for codepoint U+{:04X}: {}",
            r.rung_name,
            ch as u32,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    // Confirm we reached the GoldenImage apex.
    let last_rung = results.last().map(|r| r.rung_name);
    assert_eq!(
        last_rung,
        Some(RungName::GoldenImage),
        "sparse-golden scenario must drive through GoldenImage apex \
         (codepoint U+{:04X}, golden={})",
        ch as u32,
        golden_name
    );

    results
}
