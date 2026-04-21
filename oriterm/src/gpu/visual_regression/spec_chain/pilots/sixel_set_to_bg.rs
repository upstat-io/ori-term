//! Sixel visual pilot — P2=2 SetToBg terminal-background fill.
//!
//! Catalog row: SIXEL-BG-SetToBg (§12.4 GPU-apex)
//! Apex: GoldenImage
//!
//! Emits a DCS q with P2=2 (SetToBg — undrawn pixels filled with the
//! terminal background color snapshotted at DCS-hook time). Paired
//! with a partial-coverage payload (10 filled cols + 10 empty cols +
//! 10 filled cols) so both the emitted color and the terminal-bg fill
//! appear in the golden. Bracketing rationale alone is not enough —
//! this pilot directly drives the `SixelParser::new(params, terminal_bg)`
//! + `SixelBgMode::SetToBg` + `finish` path through the full visual
//! rung chain.
//!
//! Regression guard: if `Term::handle_sixel_start` regresses to the
//! pre-§12.2 DeviceDefault-collapse behavior (undrawn pixels filled with
//! `[0,0,0,255]` regardless of terminal bg), the golden's partial-
//! coverage band shows opaque black instead of the deterministic lane's
//! `PALETTE_BG = Rgb(1,1,1)` — 0-pixel diff fails.

use oriterm_test_support::spec_chain::{
    ApexLayer, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations, SpecScenario,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// DCS with P1=0, P2=2 (SetToBg / fill with terminal bg):
/// - `#0;2;100;100;100` define white
/// - `#0!10~` 10 cols filled white
/// - `!10?` 10 cols empty (value 0) — filled with `terminal_bg` under P2=2
/// - `#0!10~` 10 more cols filled white
const SIXEL_BYTES: &[u8] = b"\x1bP0;2q#0;2;100;100;100#0!10~!10?#0!10~\x1b\\";

/// Pilot: P2=2 SetToBg pins the golden with a terminal-background fill.
///
/// The deterministic lane's `PALETTE_BG = Rgb(1,1,1)` lands on undrawn
/// pixels through the SetToBg branch — distinct from DeviceDefault's
/// opaque `[0,0,0,255]`. If §12.2's `Term::effective_background` +
/// `SixelParser::new(params, bg)` plumbing regresses, the golden image
/// diff fails on the 0-pixel gate.
#[test]
fn sixel_set_to_bg_p2_two_fills_undrawn_with_terminal_background() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "SIXEL-BG-SetToBg",
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
                golden_name: Some("sixel_set_to_bg"),
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
