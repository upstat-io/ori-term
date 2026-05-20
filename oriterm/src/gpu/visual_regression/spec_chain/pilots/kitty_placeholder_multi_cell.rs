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

/// Sample the dominant RGB color of the cell at `(col, row)` in the
/// rendered texture. Averages a small inner box of pixels to avoid
/// picking up bilinear-sampling artifacts at cell edges. Returns
/// `(r, g, b)` as `u8` averages.
fn sample_cell_color(pixels: &[u8], width: u32, cell_w: f32, cell_h: f32, col: u32, row: u32) -> (u8, u8, u8) {
    // Inner sample box: 25%-75% of the cell, both axes. Stays well
    // inside the cell so bilinear sampling against the placeholder
    // quad's UV slice is exercised at full strength.
    let x0 = (col as f32 * cell_w + cell_w * 0.25) as u32;
    let x1 = ((col as f32 * cell_w + cell_w * 0.75) as u32).max(x0 + 1);
    let y0 = (row as f32 * cell_h + cell_h * 0.25) as u32;
    let y1 = ((row as f32 * cell_h + cell_h * 0.75) as u32).max(y0 + 1);
    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut n: u64 = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let idx = (y as usize * width as usize + x as usize) * 4;
            r_sum += pixels[idx] as u64;
            g_sum += pixels[idx + 1] as u64;
            b_sum += pixels[idx + 2] as u64;
            n += 1;
        }
    }
    let n = n.max(1);
    (
        (r_sum / n) as u8,
        (g_sum / n) as u8,
        (b_sum / n) as u8,
    )
}

/// Programmatic corner pin (a) of the §13.4 BOTH-gates contract.
///
/// Drives the same 2×2 scenario as the rung-completeness test above,
/// then resolves the four corner cells `(0,0)`, `(1,0)`, `(0,1)`,
/// `(1,1)` to pixel coords via the live `cell_metrics()` and asserts
/// the dominant inner-box color matches the expected per-quadrant
/// payload (RED, GREEN, BLUE, WHITE).
///
/// Companion distinctness assertion:
/// `kitty_placeholder_multi_cell_renders_distinct_corner_colors`.
/// Catalog row `KG-UNICODE-PLACEHOLDER-CELL-RESOLVE` flips from
/// `verified-partial` to `verified` only when this pin AND operator-
/// visual verification of `oriterm/tests/references/
/// kitty_placeholder_multi_cell.png` are both green.
#[test]
fn kitty_placeholder_multi_cell_corners_paint_expected_quadrants_programmatically() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
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
    let _ = harness.run_visual_scenario(&scenario);
    let (pixels, width, _height) = harness
        .last_rendered_pixels()
        .expect("visual scenario must produce a rendered pixel buffer");
    let pixels = pixels.to_vec();

    let cw = cell.width;
    let ch = cell.height;

    // Bilinear sampling within a 2×2 source texture interpolates across
    // texel boundaries (UV = 0.5 is exactly between the two texels per
    // axis), so the sample-box average for a cell whose UV slice
    // includes the boundary will pick up some signal from the
    // neighbouring quadrant. The assertion form is therefore
    // dominance + gap-from-other-channels rather than per-channel hard
    // thresholds — robust to filter bleed but tight enough to catch
    // the regression where every cell renders the bilinearly-blended
    // full image (every channel ≈ similar magnitude).
    const DOM_MIN: u8 = 150; // dominant channel floor
    const GAP: u8 = 60; // minimum lead over other channels

    // Cell (0, 0) → RED quadrant.
    let (r, g, b) = sample_cell_color(&pixels, width, cw, ch, 0, 0);
    assert!(
        r > DOM_MIN && r.saturating_sub(g) > GAP && r.saturating_sub(b) > GAP,
        "cell (0,0) MUST render RED-dominant quadrant — got (r={r}, g={g}, b={b})"
    );
    // Cell (1, 0) → GREEN quadrant.
    let (r, g, b) = sample_cell_color(&pixels, width, cw, ch, 1, 0);
    assert!(
        g > DOM_MIN && g.saturating_sub(r) > GAP && g.saturating_sub(b) > GAP,
        "cell (1,0) MUST render GREEN-dominant quadrant — got (r={r}, g={g}, b={b})"
    );
    // Cell (0, 1) → BLUE quadrant.
    let (r, g, b) = sample_cell_color(&pixels, width, cw, ch, 0, 1);
    assert!(
        b > DOM_MIN && b.saturating_sub(r) > GAP && b.saturating_sub(g) > GAP,
        "cell (0,1) MUST render BLUE-dominant quadrant — got (r={r}, g={g}, b={b})"
    );
    // Cell (1, 1) → WHITE quadrant (all three channels high).
    let (r, g, b) = sample_cell_color(&pixels, width, cw, ch, 1, 1);
    assert!(
        r > 200 && g > 200 && b > 200,
        "cell (1,1) MUST render WHITE quadrant — got (r={r}, g={g}, b={b})"
    );
}

/// Programmatic distinctness assertion (b) of the §13.4 BOTH-gates
/// contract.
///
/// Asserts the four corner cells are NOT all the same color — catches
/// an upstream regression that collapses the UV slicing back to the
/// full-image case (every cell would render the SAME bilinearly-
/// averaged blend of all 4 quadrant pixels).
///
/// Companion expected-quadrant assertion:
/// `kitty_placeholder_multi_cell_corners_paint_expected_quadrants_programmatically`.
#[test]
fn kitty_placeholder_multi_cell_renders_distinct_corner_colors() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
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
    let _ = harness.run_visual_scenario(&scenario);
    let (pixels, width, _height) = harness
        .last_rendered_pixels()
        .expect("visual scenario must produce a rendered pixel buffer");
    let pixels = pixels.to_vec();

    let cw = cell.width;
    let ch = cell.height;
    let c00 = sample_cell_color(&pixels, width, cw, ch, 0, 0);
    let c10 = sample_cell_color(&pixels, width, cw, ch, 1, 0);
    let c01 = sample_cell_color(&pixels, width, cw, ch, 0, 1);
    let c11 = sample_cell_color(&pixels, width, cw, ch, 1, 1);
    let unique: std::collections::HashSet<_> = [c00, c10, c01, c11].into_iter().collect();
    assert!(
        unique.len() >= 3,
        "multi-cell placeholder rendering MUST produce ≥3 distinct corner colors — collapsing to the full-image case would yield 1. Sampled corners: (0,0)={c00:?} (1,0)={c10:?} (0,1)={c01:?} (1,1)={c11:?}"
    );
}
