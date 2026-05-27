//! §15.4 — cell-level alpha DEFAULT-branch blend-matrix pixel pins.
//!
//! Catalog row: DFCT-CELL-ALPHA (cell-level per-channel alpha, de-facto)
//! Apex: TextureRender (direct pixel-value assertion, no golden bless)
//!
//! The pre-flatten contract (§15.0, Decision 09) proves ori_term receives
//! flattened concrete colors. These pins validate ori_term's COMPOSITING of a
//! flattened translucent cell over a KNOWN solid background across the
//! alpha matrix `a ∈ {1.0, 0.5, 0.0}`, driving the PRODUCTION cached render
//! path (`render_frame_cached()` via the visual rung chain) and asserting the
//! cell-center pixel byte values directly:
//!
//! - `a=1.0` → pixel == cell color (cell fully replaces the bg; no blend).
//! - `a=0.0` → pixel == background (the underlying surface shows through).
//! - `a=0.5` → pixel == the correct straight-over linear blend (≈161 red),
//!   NOT the over-bright/over-dark value the double-premultiply regression
//!   (reverting §15.3's straight `rgb_to_floats`) would produce (≈117 red).
//!
//! The `a=0.5` regression-rejecting assertion is the load-bearing guard: it asserts the rendered
//! red lands near the correct single-premultiply composite and far from the
//! double-premultiply value. Both are python-verified (`/tmp/check-cell-alpha.py`):
//! sRGB→linear (IEC 61966-2-1), `cell_lin*a + bg_lin*(1-a)`, linear→sRGB encode.
//! A sample-count ghost guard rejects a vacuous pass.

use oriterm_test_support::spec_chain::ScenarioExpectations;

use super::super::visual_harness::VisualSpecHarness;
use crate::gpu::visual_regression::GoldenLaneConfig;

/// Sampled cell — row 3, col 4 — chosen away from the home-row cursor so the
/// cursor block never confounds the cell-center readback (same seam as §15.3).
const SAMPLE_COL: usize = 4;
const SAMPLE_ROW: usize = 3;

/// Move to (row 3, col 5) via `CSI row;col H` (1-based) → 0-based (3,4), set a
/// bright-red FULLY-OPAQUE background via SGR mode-6 alpha=255, emit a space so
/// the cell carries only the bg (no glyph at the sampled center), reset SGR,
/// then home the cursor to (0,0), far from the sample.
const RED_BG_A255_BYTES: &[u8] = b"\x1b[4;5H\x1b[48:6::220:30:30:255m \x1b[m\x1b[H";

/// Same placement, bright-red background at SGR mode-6 alpha=128 (≈0.502) — the
/// 50% straight-over blend case (the negative-pin discriminant cell).
const RED_BG_A128_BYTES: &[u8] = b"\x1b[4;5H\x1b[48:6::220:30:30:128m \x1b[m\x1b[H";

/// Same placement, bright-red background at SGR mode-6 alpha=0 — the cell bg is
/// non-default (red) so the bg-quad gate still fires, but at alpha=0 it
/// contributes nothing and the underlying surface shows through.
const RED_BG_A0_BYTES: &[u8] = b"\x1b[4;5H\x1b[48:6::220:30:30:0m \x1b[m\x1b[H";

/// Correct single-premultiply straight-over blend, red channel at `a=0.5`
/// (python-verified, `/tmp/check-cell-alpha.py`: `cell_lin*a + bg_lin*(1-a)`,
/// linear→sRGB). Cell red `(220,…)` over near-black golden lane `(1,…)`.
const A50_CORRECT_RED: i32 = 161;

/// Double-premultiply regression red channel at `a=0.5` (`cell_lin*a^2 + …`) —
/// the over-dark value reverting §15.3's straight `rgb_to_floats` produces.
const A50_DOUBLE_PREMUL_RED: i32 = 117;

/// Drive the current harness state through the visual rung chain and return
/// `(pixels, width, height)` from the production cached render path.
fn drive_render(harness: &mut VisualSpecHarness, catalog_row: &str) -> (Vec<u8>, u32, u32) {
    let expectations = ScenarioExpectations::default();
    let _results = harness.render_visual_rungs(catalog_row, &expectations);
    let (slice, w, h) = harness
        .last_rendered_pixels()
        .expect("render_visual_rungs must populate last_rendered_pixels");
    (slice.to_vec(), w, h)
}

/// Resolve cell (col, row) center to a pixel offset into the RGBA8 buffer.
fn cell_center_offset(harness: &VisualSpecHarness, col: usize, row: usize, w: u32) -> usize {
    let cell = harness.renderer().cell_metrics();
    let px_x = (cell.width * (col as f32 + 0.5)) as usize;
    let px_y = (cell.height * (row as f32 + 0.5)) as usize;
    (px_y * w as usize + px_x) * 4
}

/// Count in-bounds RGBA pixels covered by the sample cell's rect. A geometry
/// ghost guard: confirms the cell (col,row)→pixel mapping lands inside the
/// rendered buffer so the center-pixel offset the assertions read is valid and
/// not out of bounds. The separate "renderer actually drew content" guard is
/// the a=1.0 assertion — red ≈220 cannot occur unless the cell bg was
/// rasterized into the sampled cell.
fn cell_sample_count(harness: &VisualSpecHarness, pixels: &[u8], w: u32) -> usize {
    let cell = harness.renderer().cell_metrics();
    let x0 = (cell.width * SAMPLE_COL as f32) as usize;
    let y0 = (cell.height * SAMPLE_ROW as f32) as usize;
    let x1 = (cell.width * (SAMPLE_COL as f32 + 1.0)) as usize;
    let y1 = (cell.height * (SAMPLE_ROW as f32 + 1.0)) as usize;
    let mut count = 0usize;
    for py in y0..y1 {
        for px in x0..x1 {
            let off = (py * w as usize + px) * 4;
            if px < w as usize && off + 3 < pixels.len() {
                count += 1;
            }
        }
    }
    count
}

/// §15.4 — flattened-color blend matrix at `a ∈ {1.0, 0.5, 0.0}` on the
/// SPEC_DEFAULT (dual-source-when-available) lane, bg-only cell (no glyph).
///
/// Proves the cell-bg-quad compositing across the full alpha range:
/// - `a=1.0` → cell red replaces bg exactly (≈220).
/// - `a=0.5` → correct ~50% straight-over blend (≈161), strictly between the
///   double-premultiply over-dark value (≈117) and the opaque cell color.
/// - `a=0.0` → the underlying surface shows through (== bg-only baseline).
#[test]
fn flattened_color_blend_matrix_default_lane() {
    let Some(harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    run_blend_matrix(harness);
}

/// §15.4 — same blend matrix on the NON-dual subpixel lane.
///
/// The bg-quad path is shared, but pinning under `force_non_dual_subpixel`
/// exercises the same compositing seam the §15.3 (F2) subpixel fix touches,
/// proving the blend holds regardless of the subpixel-pipeline selection.
#[test]
fn flattened_color_blend_matrix_non_dual_lane() {
    let Some(harness) = VisualSpecHarness::with_config(GoldenLaneConfig::NON_DUAL_DEFAULT) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    run_blend_matrix(harness);
}

/// Shared blend-matrix body for both lanes. Renders the bg-only baseline, then
/// the red cell bg at `a ∈ {1.0, 0.5, 0.0}`, asserting the cell-center pixel.
fn run_blend_matrix(mut harness: VisualSpecHarness) {
    // Baseline: empty grid, bg-only. The a=0.0 case must reproduce THIS pixel
    // (underlying surface shows through). Sampling it (rather than hardcoding
    // the golden-lane near-black) keeps the pin robust to clear-color details.
    let (base_pixels, base_w, _bh) = drive_render(&mut harness, "DFCT-CELL-ALPHA-BASELINE");
    let base_off = cell_center_offset(&harness, SAMPLE_COL, SAMPLE_ROW, base_w);
    let base_red = i32::from(base_pixels[base_off]);
    let base_count = cell_sample_count(&harness, &base_pixels, base_w);

    // Ghost guard: the cell rect must actually contain pixels, else every
    // center-pixel assertion below could pass vacuously.
    assert!(
        base_count > 0,
        "§15.4: the sample-cell rect contained NO pixels — the center-pixel \
         assertions would pass vacuously; cell metrics or viewport sizing is wrong"
    );

    // a=1.0 — fully-opaque red bg replaces the underlying surface.
    harness.core_mut().feed(b"\x1b[2J\x1b[H");
    harness.core_mut().feed(RED_BG_A255_BYTES);
    let (a255_pixels, a255_w, _h) = drive_render(&mut harness, "DFCT-CELL-ALPHA-A255");
    let a255_off = cell_center_offset(&harness, SAMPLE_COL, SAMPLE_ROW, a255_w);
    let a255_red = i32::from(a255_pixels[a255_off]);
    assert!(
        a255_red > 200,
        "§15.4 (a=1.0): an opaque (alpha=255) red bg cell must render the cell \
         color at full strength (≈220); got {a255_red}. A value ≤200 means the \
         opaque cell bg was wrongly attenuated"
    );

    // a=0.5 — correct ~50% straight-over blend; the negative-pin discriminant.
    harness.core_mut().feed(b"\x1b[2J\x1b[H");
    harness.core_mut().feed(RED_BG_A128_BYTES);
    let (a128_pixels, a128_w, _h) = drive_render(&mut harness, "DFCT-CELL-ALPHA-A128");
    let a128_off = cell_center_offset(&harness, SAMPLE_COL, SAMPLE_ROW, a128_w);
    let a128_red = i32::from(a128_pixels[a128_off]);

    // Regression guard (rejects the DOUBLE-premultiply value). The midpoint
    // between correct (161) and the double-premul value (117) is 139. The lower
    // bound 145 sits strictly above 117 and below 161 → correct passes, the
    // over-dark double-premul value fails. The upper bound 180 sits strictly
    // below the opaque cell red (≈220) → a partial blend, not full strength.
    assert!(
        a128_red > A50_DOUBLE_PREMUL_RED + 28,
        "§15.4 (a=0.5 regression guard): translucent (alpha=128) red bg cell-center \
         red must be the correct single-premultiply straight-over blend (≈{}), \
         strictly above the double-premultiply regression value (≈{}); got \
         {a128_red}. A value ≤145 means rgb_to_floats is premultiplying on top \
         of the shader's premultiply (the §15.3 fix was reverted). It must also \
         stay above the bg-only baseline (baseline={base_red})",
        A50_CORRECT_RED,
        A50_DOUBLE_PREMUL_RED
    );
    assert!(
        a128_red < 180,
        "§15.4 (a=0.5): translucent (alpha=128) red bg cell-center red must be a \
         PARTIAL blend (≈{}), strictly below the fully-opaque cell red (≈{a255_red}); \
         got {a128_red}. A value ≥180 means alpha did not attenuate the cell color",
        A50_CORRECT_RED
    );
    assert!(
        a128_red > base_red,
        "§15.4 (a=0.5): the translucent blend (≈{}) must be brighter than the \
         bg-only baseline ({base_red}); if equal the bg-quad alpha collapsed to ~0",
        A50_CORRECT_RED
    );

    // a=0.0 — underlying surface shows through (== bg-only baseline). The cell
    // bg is non-default red so the bg-quad gate fires, but at alpha=0 it
    // contributes nothing, leaving the surface beneath unchanged.
    harness.core_mut().feed(b"\x1b[2J\x1b[H");
    harness.core_mut().feed(RED_BG_A0_BYTES);
    let (a0_pixels, a0_w, _h) = drive_render(&mut harness, "DFCT-CELL-ALPHA-A0");
    let a0_off = cell_center_offset(&harness, SAMPLE_COL, SAMPLE_ROW, a0_w);
    let a0_red = i32::from(a0_pixels[a0_off]);
    assert!(
        (a0_red - base_red).abs() <= 2,
        "§15.4 (a=0.0): a fully-transparent (alpha=0) red bg cell must show the \
         underlying surface (the bg-only baseline red={base_red}); got {a0_red}. A \
         red far from the baseline means the alpha=0 cell wrongly contributed \
         color (it must contribute NOTHING — the surface beneath shows through)"
    );
    assert!(
        a0_red < 60,
        "§15.4 (a=0.0): the alpha=0 cell-center red must be near the near-black \
         golden-lane surface, NOT the red cell color; got {a0_red}. A high red \
         means the alpha=0 bg quad was composited as if opaque"
    );
}
