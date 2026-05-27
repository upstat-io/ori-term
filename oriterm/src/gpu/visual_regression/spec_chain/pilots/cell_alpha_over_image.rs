//! §15.4 — cell-vs-image coexistence pin under the shared `PREMUL_ALPHA_BLEND`.
//!
//! Catalog rows: DFCT-CELL-ALPHA, ECMA48-SGR-48-RGBA, KG-ACTION-TRANSMIT-AND-PLACE
//! Apex: TextureRender (direct pixel-value + cell-rect scan, no golden bless).
//!
//! NOT a cell-over-image z-interleaving test: Decision 09 §"Plane ordering"
//! REJECTS arbitrary cell-over-image z-interleaving for §15 (notcurses
//! pre-flattens producer-side; the fixed 3-tier painter's order
//! `cell-bg → images-below → glyphs → images-above` paints cell-bg BEFORE
//! any image so cell-bg-over-image-quad is architecturally unrepresentable in
//! current code — any test that placed the SGR cell + kitty image at the SAME
//! cell would simply assert "image overwrites bg," which trivially passes and
//! proves nothing about blend math). The pin instead places the two cells
//! at NON-OVERLAPPING positions so each pixel sees ONE pipeline's output,
//! then asserts BOTH pipelines independently composite correctly while
//! sharing the `PREMUL_ALPHA_BLEND` constant in the same rendered frame.
//!
//! - `DFCT-CELL-ALPHA` — cell-level per-channel alpha, de-facto. Cell-bg pin
//!   exercises the shared `PREMUL_ALPHA_BLEND` single-premultiply convention.
//! - `ECMA48-SGR-48-RGBA` — `CSI 48:6::r:g:b:a m`, on-wire concrete bg-alpha
//!   producer. Cell-bg pin verifies translucent bg composites correctly when
//!   the image-pipeline also runs in the frame.
//! - `KG-ACTION-TRANSMIT-AND-PLACE` — kitty `a=T`. Image-cell pin verifies the
//!   image quad survives to GPU emit alongside the SGR cell.
//!
//! Out of scope: cross-pipeline GEOMETRY pollution (cell-bg quad tinting the
//! image cell, or vice versa). The non-overlapping cell placement makes this
//! pin geometry-blind by construction — a future cross-pipeline geometry pin
//! lands as a sibling pilot that overlaps the cells and asserts per-channel
//! purity (anchored in §15.R for follow-up).
//!
//! Decision 09 names the cell-vs-image reconciliation: cell-bg and image quads
//! BOTH bind `PREMUL_ALPHA_BLEND` (`pipeline/mod.rs:117-128`; image binding at
//! `pipeline/image.rs:143`), so a translucent SGR cell + an opaque kitty image
//! in the SAME frame must each composite correctly without one pipeline's math
//! corrupting the other. Painter's order (`render_helpers/record_passes.rs`:
//! cell-bg → images-below → glyphs → images-above) keeps the two cells
//! non-overlapping by design — each pixel sees ONE pipeline's output:
//!
//! - Cell (3,4): SGR mode-6 `48:6::220:30:30:128m` (translucent red, a≈0.502)
//!   over the near-black golden-lane surface → ≈161 red (single-premul blend,
//!   identical to `cell_alpha.rs` `A50_CORRECT_RED`).
//! - Cell (0,0): opaque kitty 1×1 RGBA `FF 00 00 FF` via `a=T` at native pixel
//!   size — paints a small red region inside the cell. Same shape as
//!   `kitty_cure_confirmation`'s item-14 pin.
//!
//! The pin is per-pipeline coexistence in one frame: the cell-bg center pixel
//! must reject the double-premul regression (≈117) AND the kitty image cell
//! must contain ≥4 red pixels — so introducing the image quad does NOT
//! disturb the cell-bg blend math, and introducing the SGR cell does NOT
//! suppress the image-pipeline emit. The image-quad count must be ≥1 (proves
//! `prepare` actually emitted the image — guards against a vacuous "0 red, 0
//! quads" no-image silent pass).
//!
//! Expected values python-verified (`/tmp/check-cell-image-coexist.py`:
//! sRGB→linear, `cell_lin*a + bg_lin*(1-a)`, linear→sRGB encode). Geometry
//! ghost guard rejects out-of-bounds cell rects.

use oriterm_test_support::spec_chain::{
    FrameInputExpectation, GpuInstanceExpectation, ScenarioExpectations,
};

use super::super::visual_harness::VisualSpecHarness;
use crate::gpu::visual_regression::GoldenLaneConfig;

/// Sample-A — translucent cell-bg cell (row 3, col 4). Same seam as
/// `cell_alpha.rs` so the discriminant values stay aligned.
const CELL_BG_COL: usize = 4;
const CELL_BG_ROW: usize = 3;

/// Sample-B — kitty image cell (row 0, col 0). Native-pixel-size placement at
/// the home cell; same seam as `kitty_cure_confirmation`'s item-14 pin.
const IMAGE_COL: usize = 0;
const IMAGE_ROW: usize = 0;

/// Kitty transmit-and-place at the current cursor: 1×1 opaque red RGBA.
///
/// - `f=32,s=1,v=1` — RGBA, 1×1 source.
/// - `a=T` — transmit and place at cursor.
/// - `i=42` — image id (per-harness fresh).
/// - `q=2` — suppress responses.
/// - Payload `/wAA/w==` = base64(`FF 00 00 FF`) = opaque red.
///
/// Cursor is at (0,0) when this is fed (default Term home), so the image
/// places at cell `(IMAGE_COL, IMAGE_ROW)` = (0,0).
const OPAQUE_KITTY_IMAGE_BYTES: &[u8] = b"\x1b_Gf=32,s=1,v=1,a=T,i=42,q=2;/wAA/w==\x1b\\";

/// Move to (row 3, col 5) via `CSI row;col H` (1-based) → 0-based (3,4), set a
/// translucent red bg via SGR mode-6 alpha=128 (≈0.502), emit a space so the
/// cell carries only the bg (no glyph at the sampled center), reset SGR.
///
/// No trailing `\x1b[H` — the cursor sits at (3,5) after the space, away from
/// BOTH the SGR-cell sample (3,4) and the image cell (0,0). Bringing cursor
/// HOME would land it at (0,0) directly on the image, and `record_cursor_pass`
/// (`render_helpers/record_passes.rs:197-212`) runs AFTER `record_image_draws`,
/// so the cursor block would overdraw the kitty image — the exact failure mode
/// `kitty_cure_confirmation`'s item-14 pin documents.
const TRANSLUCENT_RED_BG_BYTES: &[u8] = b"\x1b[4;5H\x1b[48:6::220:30:30:128m \x1b[m";

/// Correct single-premultiply straight-over blend red at the cell-bg cell.
/// python-verified — identical to `cell_alpha.rs` `A50_CORRECT_RED`.
const CELL_BG_CORRECT_RED: i32 = 161;

/// Double-premultiply regression red — the over-dark value reverting §15.3's
/// straight `rgb_to_floats` would produce on the cell-bg cell.
const CELL_BG_DOUBLE_PREMUL_RED: i32 = 117;

/// Minimum red-pixel count inside the kitty image cell. The 1×1 native-pixel
/// kitty image lands as a small red region in cell (0,0) (kitty stretches to
/// at least the source-pixel-size, sub-cell). Same ≥4 threshold as
/// `kitty_cure_confirmation`'s item-14 cell-pinned pin.
const IMAGE_MIN_RED_PIXELS: usize = 4;

/// Drive the current harness state through the visual rung chain. Caller
/// chooses the expectation shape: baseline (no image) vs image-bearing.
fn drive_render(
    harness: &mut VisualSpecHarness,
    catalog_row: &str,
    expectations: ScenarioExpectations,
) -> (Vec<u8>, u32, u32) {
    let _results = harness.render_visual_rungs(catalog_row, &expectations);
    let (slice, w, h) = harness
        .last_rendered_pixels()
        .expect("render_visual_rungs must populate last_rendered_pixels");
    (slice.to_vec(), w, h)
}

/// Image-bearing expectations — mirrors `kitty_cure_confirmation`'s
/// `drive_render`: `frame_input` + `gpu_instance.with_images(1)` matches the
/// known-good kitty-image rung chain so the same pin shape applies here.
fn image_bearing_expectations() -> ScenarioExpectations {
    ScenarioExpectations {
        frame_input: Some(FrameInputExpectation::default_grid()),
        gpu_instance: Some(GpuInstanceExpectation::at_least(0, 0).with_images(1)),
        ..ScenarioExpectations::default()
    }
}

/// Resolve cell (col, row) center to a pixel offset into the RGBA8 buffer.
fn cell_center_offset(harness: &VisualSpecHarness, col: usize, row: usize, w: u32) -> usize {
    let cell = harness.renderer().cell_metrics();
    let px_x = (cell.width * (col as f32 + 0.5)) as usize;
    let px_y = (cell.height * (row as f32 + 0.5)) as usize;
    (px_y * w as usize + px_x) * 4
}

/// Count in-bounds RGBA pixels covered by a sample cell's rect — geometry
/// ghost guard. The two-pipeline pin reads at TWO cells; without this guard
/// either center read could pass vacuously on a viewport-sizing regression.
fn cell_sample_count(
    harness: &VisualSpecHarness,
    pixels: &[u8],
    w: u32,
    col: usize,
    row: usize,
) -> usize {
    let cell = harness.renderer().cell_metrics();
    let x0 = (cell.width * col as f32) as usize;
    let y0 = (cell.height * row as f32) as usize;
    let x1 = (cell.width * (col as f32 + 1.0)) as usize;
    let y1 = (cell.height * (row as f32 + 1.0)) as usize;
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

/// Count red-ish pixels (`r > 200 && g < 50 && b < 50`) inside the kitty
/// image cell's rect — same tolerance as `kitty_cure_confirmation`'s
/// `is_red_ish` predicate.
fn red_pixels_in_cell(
    harness: &VisualSpecHarness,
    pixels: &[u8],
    w: u32,
    h: u32,
    col: usize,
    row: usize,
) -> usize {
    let cell = harness.renderer().cell_metrics();
    let px_x = (cell.width * col as f32) as usize;
    let px_y = (cell.height * row as f32) as usize;
    let cell_w = cell.width as usize;
    let cell_h = cell.height as usize;
    let mut count = 0usize;
    for y in px_y..(px_y + cell_h).min(h as usize) {
        for x in px_x..(px_x + cell_w).min(w as usize) {
            let off = (y * w as usize + x) * 4;
            if off + 4 <= pixels.len()
                && pixels[off] > 200
                && pixels[off + 1] < 50
                && pixels[off + 2] < 50
            {
                count += 1;
            }
        }
    }
    count
}

/// §15.4 — translucent SGR cell + opaque kitty image coexist correctly in the
/// SAME frame on the SPEC_DEFAULT (dual-source-when-available) lane.
#[test]
fn flattened_cell_and_image_coexist_default_lane() {
    let Some(harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    run_coexistence(harness);
}

/// §15.4 — same coexistence pin on the NON-dual subpixel lane. The cell-bg
/// quad and image-quad pipelines are shared across lanes; pinning under
/// `force_non_dual_subpixel` proves no subpixel-lane regression breaks the
/// reconciliation seam.
#[test]
fn flattened_cell_and_image_coexist_non_dual_lane() {
    let Some(harness) = VisualSpecHarness::with_config(GoldenLaneConfig::NON_DUAL_DEFAULT) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    run_coexistence(harness);
}

/// Regression: BUG-06-107 — per-frame image-emit cache poisoning when a
/// prior no-image `render_visual_rungs` call precedes an image-bearing one.
/// See: bug-tracker/section-06-rendering-perf.md
///
/// Shared body — render translucent SGR + kitty image in one frame and assert
/// both pipeline outputs.
///
/// No separate baseline render is used: a baseline-then-scene render pair
/// leaves `prepared_image_quad_counts()` reporting the prepared quad but
/// `record_image_draws` emits 0 visible pixels; single-render emits the
/// expected red region. The pilot ships via the single-render workaround.
/// Any future fix that "removes the workaround" must first close the
/// regression named in the doc comment above — the workaround is the
/// regression guard, not a paper-over. The cell-bg regression-guard pin
/// (>145) is already a strict rejection of the double-premultiply value
/// (≈117), so no baseline-vs-translucent comparison is needed for the pin
/// to be sound.
fn run_coexistence(mut harness: VisualSpecHarness) {
    // Frame under test — feed kitty image FIRST (places at home (0,0) where
    // the cursor starts), then SGR translucent cell (positions cursor away to
    // (3,4)). One render: both pipelines exercised in the same frame.
    harness.core_mut().feed(OPAQUE_KITTY_IMAGE_BYTES);
    harness.core_mut().feed(TRANSLUCENT_RED_BG_BYTES);
    let (pixels, w, h) = drive_render(
        &mut harness,
        "DFCT-CELL-ALPHA-OVER-IMAGE",
        image_bearing_expectations(),
    );

    let cell_off = cell_center_offset(&harness, CELL_BG_COL, CELL_BG_ROW, w);
    let cell_red = i32::from(pixels[cell_off]);
    let image_red_count = red_pixels_in_cell(&harness, &pixels, w, h, IMAGE_COL, IMAGE_ROW);
    let (img_below, img_above) = harness.prepared_image_quad_counts();

    let cell_bg_count = cell_sample_count(&harness, &pixels, w, CELL_BG_COL, CELL_BG_ROW);
    let image_cell_count = cell_sample_count(&harness, &pixels, w, IMAGE_COL, IMAGE_ROW);

    // Geometry ghost guards: BOTH sample cells must contain pixels, else the
    // center-pixel + red-count assertions would pass vacuously.
    assert!(
        cell_bg_count > 0,
        "§15.4: the cell-bg sample cell ({CELL_BG_COL},{CELL_BG_ROW}) contained NO \
         pixels — the cell-bg center-pixel assertion would pass vacuously; cell \
         metrics or viewport sizing is wrong"
    );
    assert!(
        image_cell_count > 0,
        "§15.4: the image sample cell ({IMAGE_COL},{IMAGE_ROW}) contained NO \
         pixels — the image red-count assertion would pass vacuously; cell \
         metrics or viewport sizing is wrong"
    );

    // Cell-bg cell: translucent SGR must still composite at the correct
    // single-premul brightness (≈161) with the image quad in the same frame.
    // The 145 lower bound sits strictly above the double-premul regression
    // (≈117) and below the correct value — a §15.3 `rgb_to_floats` revert
    // (double-premul) trips this guard.
    assert!(
        cell_red > CELL_BG_DOUBLE_PREMUL_RED + 28,
        "§15.4 cell-vs-image: translucent (alpha=128) red SGR bg at cell \
         ({CELL_BG_COL},{CELL_BG_ROW}) must composite at the correct single-premul \
         brightness (≈{CELL_BG_CORRECT_RED}) even with an image quad in the same \
         frame; got {cell_red}. A value ≤{} means `rgb_to_floats` is premultiplying \
         on top of the shader's premultiply (the §15.3 fix was reverted, or the \
         image pipeline's introduction of a second quad disturbed the cell-bg blend \
         math).",
        CELL_BG_DOUBLE_PREMUL_RED + 28
    );
    assert!(
        cell_red < 180,
        "§15.4 cell-vs-image: translucent (alpha=128) red SGR bg at cell \
         ({CELL_BG_COL},{CELL_BG_ROW}) must be a PARTIAL blend (≈{CELL_BG_CORRECT_RED}), \
         strictly below the fully-opaque cell red (≈220); got {cell_red}. A value \
         ≥180 means alpha did not attenuate the cell color — image-pipeline setup \
         may be polluting the cell-bg path."
    );

    // Image quad count: prepare MUST emit ≥1 image quad. Guards against a
    // vacuous pass where the kitty bytes were dropped silently (then both the
    // red-count below and the cell-bg assertion above would pass with NO
    // image-pipeline interaction at all, hiding a real regression).
    assert!(
        img_below + img_above >= 1,
        "§15.4 cell-vs-image: the prepared frame must contain ≥1 image quad \
         (below={img_below}, above={img_above}); got 0 quads. The kitty `a=T` \
         transmit-and-place was either parser-rejected, dropped at snapshot, or \
         silently filtered out at prepare — the image-pipeline half of the \
         coexistence pin never exercised."
    );

    // Image cell: ≥4 red pixels inside cell (0,0). Same threshold as
    // `kitty_cure_confirmation`'s item-14 pin (1×1 native-pixel kitty image
    // paints a small red region). If 0, the image rendered to a quad but did
    // not actually emit visible pixels — `record_image_draws` regression or
    // texture upload failure.
    assert!(
        image_red_count >= IMAGE_MIN_RED_PIXELS,
        "§15.4 cell-vs-image: the kitty image at cell ({IMAGE_COL},{IMAGE_ROW}) \
         must paint ≥{IMAGE_MIN_RED_PIXELS} red-ish pixels (r>200,g<50,b<50); got \
         {image_red_count}. {img_below}+{img_above} image quads were prepared, so \
         the image survived to GPU emit, but `record_image_draws` produced no \
         visible red pixels in the cell rect — either the texture upload failed, \
         the bind group is empty, or the image-quad coordinates miss the cell."
    );
}
