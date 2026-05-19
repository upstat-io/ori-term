//! §13.6.1 multi-cell unicode-placeholder UV slicing pilot.
//!
//! Catalog row: `KG-UNICODE-PLACEHOLDER-CELL-RESOLVE`
//! Apex: `GoldenImage`
//!
//! Drives a `a=T,U=1,c=2,r=2` transmit + 2×2 placeholder cell write. The
//! cache records `(cols, rows) = (2, 2)`; the snapshot propagates that
//! grid onto each `RenderablePlaceholderCell`; emit slices the UV per
//! cell so each cell renders its corresponding quadrant of the source
//! image.
//!
//! Without the §13.6.1 multi-cell UV slicing, every cell would render
//! the full image — the §13.4 single-cell baseline behavior. The
//! prepare-layer tests at
//! `oriterm/src/gpu/prepare/tests.rs::placeholder_uv_slicing::placeholder_multi_cell_renders_image_slice_per_cell`
//! pin the UV math directly; this pilot is the GPU-rung evidence pin —
//! the full Parser→Dispatch→State→Renderable→FrameInput→GpuInstance→
//! TextureRender→GoldenImage chain.
//!
//! Image payload: 2×2 RGBA with one solid colored pixel per quadrant —
//! top-left RED, top-right GREEN, bottom-left BLUE, bottom-right WHITE.
//! Each placeholder cell should render its mapped quadrant; the operator
//! visual-verifies the golden PNG matches that layout per
//! `feedback_visual_bugs_need_operator_verification.md`.

use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Transmit `a=T,U=1,i=1,c=2,r=2` carrying a 2×2 RGBA image (one
/// solid-color pixel per quadrant), then write four placeholder cells
/// with the appropriate row/col diacritics.
///
/// Image payload (4 pixels × 4 bytes = 16 bytes raw RGBA):
/// - TL (0,0): 0xFF 0x00 0x00 0xFF (red)
/// - TR (0,1): 0x00 0xFF 0x00 0xFF (green)
/// - BL (1,0): 0x00 0x00 0xFF 0xFF (blue)
/// - BR (1,1): 0xFF 0xFF 0xFF 0xFF (white)
///
/// Base64-encoded: `/wAA/wD/AP8AAP///////w==` (24 chars incl. padding).
///
/// Diacritic-encoded placeholder cells:
/// - Cell (col=0, row=0): U+10EEEE + U+0305 + U+0305 → image (0,0) — RED
/// - Cell (col=1, row=0): U+10EEEE + U+0305 + U+030D → image (0,1) — GREEN
/// - CR/LF advances cursor to next row.
/// - Cell (col=0, row=1): U+10EEEE + U+030D + U+0305 → image (1,0) — BLUE
/// - Cell (col=1, row=1): U+10EEEE + U+030D + U+030D → image (1,1) — WHITE
///
/// Each diacritic-encoded cell carries fg=palette(1) so `image_id_low=1`
/// (matches transmit's `i=1`).
const MULTICELL_BYTES: &[u8] = b"\x1b_Ga=T,U=1,i=1,f=32,s=2,v=2,c=2,r=2,q=2;/wAA/wD/AP8AAP///////w==\x1b\\\x1b[38;5;1m\xf4\x8e\xbb\xae\xcc\x85\xcc\x85\xf4\x8e\xbb\xae\xcc\x85\xcc\x8d\r\n\xf4\x8e\xbb\xae\xcc\x8d\xcc\x85\xf4\x8e\xbb\xae\xcc\x8d\xcc\x8d\x1b[39m";

/// Pilot test: drives `a=T,U=1,c=2,r=2` + 2×2 placeholder cells through
/// every rung. Validates: (a) emit produces 4 image quads (one per
/// cell); (b) each quad's image_id resolves to the transmitted image;
/// (c) the rendered texture has non-zero pixels (the visual rung
/// chain didn't skip emit); (d) the golden PNG captures the per-cell
/// slice layout (operator visual-verifies).
#[test]
fn kitty_placeholder_multi_cell_drives_every_rung_green() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Sync Term cell metrics with the GPU renderer — without this, image
    // placements compute pixel coords against `Term::cell_pixel_*` (default
    // 8×16) while the renderer's font-derived metrics differ. The
    // production GUI calls set_cell_dimensions on every font/resize event;
    // visual harness skips that step by default.
    let cell = harness.renderer().cell_metrics();
    harness
        .core_mut()
        .term_mut()
        .set_cell_dimensions(cell.width as u16, cell.height as u16);

    let scenario = SpecScenario {
        catalog_row_id: "KG-UNICODE-PLACEHOLDER-CELL-RESOLVE",
        bytes: MULTICELL_BYTES,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            // 4 placeholder cells → 4 image quads in image_quads_above.
            gpu_instance: Some(GpuInstanceExpectation::at_least(4, 0).with_images(4)),
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some("kitty_placeholder_multi_cell"),
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
    assert_eq!(*rung_names.last().unwrap(), RungName::GoldenImage);
}
