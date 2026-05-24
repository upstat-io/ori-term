//! iTerm2 inline image visual pilot — INLINE mode-row at the cursor.
//!
//! Catalog row: `ITERM2-1337-FILE-INLINE`
//! Apex: `GoldenImage`
//!
//! Drives `OSC 1337 ; File=inline=1:<minimal-png-b64> ST` through every
//! visual rung and asserts each rung passes. Establishes the §14.2
//! mode-row texture-render dual-gate for the INLINE arm (distinct from
//! the format-row dual-gates in `iterm2_image_{png,jpeg,bmp}_at_cursor`
//! per Decision 05 §Consequences 11-texture-render pilot enumeration).
//!
//! Mirrors the PNG pilot's byte fixture but anchors the INLINE
//! catalog row so a regression to the inline decision branch
//! (`iterm2.rs:46-49`) surfaces against this row regardless of which
//! per-format row is being driven.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// `OSC 1337 ; File=inline=1:<b64> ST` with the b64 of a 1×1 opaque-red
/// RGBA PNG produced by the `image` crate (encoded=73 bytes, b64.len=100).
///
/// Same fixture bytes as
/// `pilots/iterm2_image_png_at_cursor.rs::ITERM2_PNG_BYTES`; the PNG
/// payload is the simplest format that exercises the INLINE
/// (`inline=1`) decision branch end-to-end through the texture-render
/// apex. The format dimension is owned by the per-format pilots; this
/// pilot's load-bearing dimension is the mode (`inline=1` vs
/// silent-drop variants in `osc1337_file_inline_0_*` /
/// `osc1337_file_inline_absent_*` / `osc1337_file_inline_invalid_*`).
const ITERM2_INLINE_BYTES: &[u8] = b"\x1b]1337;File=inline=1:iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAEElEQVR4AQEFAPr/AP8AAP8FAAH/+lyI0QAAAABJRU5ErkJggg==\x1b\\";

/// Pilot test: drives the minimal iTerm2 inline-mode payload through
/// every visual rung. Captures golden at
/// `oriterm/tests/references/iterm2_image_inline_at_cursor.png` via
/// `ORITERM_UPDATE_GOLDEN=1`.
#[test]
fn iterm2_image_inline_at_cursor_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id: "ITERM2-1337-FILE-INLINE",
        bytes: ITERM2_INLINE_BYTES,
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
                golden_name: Some("iterm2_image_inline_at_cursor"),
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
