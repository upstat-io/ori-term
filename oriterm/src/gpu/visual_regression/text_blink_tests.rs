//! GPU visual regression tests for text blink opacity at different fade levels.
//!
//! Verifies the text blink alpha pipeline: `text_blink_opacity` on `FrameInput`
//! modulates `fg_dim` for cells with `CellFlags::BLINK`, flowing through
//! `fill_frame_shaped()` → `GlyphEmitter` → `push_glyph(alpha)`.

use oriterm_core::CellFlags;

use crate::gpu::frame_input::FrameInput;

use super::{compare_with_reference, headless_env, render_to_pixels};

/// Grid dimensions for text blink tests.
const COLS: usize = 10;
const ROWS: usize = 3;

/// Column of the BLINK cell.
const BLINK_COL: usize = 0;
/// Column of the non-BLINK reference cell.
const NORMAL_COL: usize = 5;

/// Build a test frame with one BLINK cell at col 0 and one normal cell at col 5.
fn blink_input(cell: crate::font::CellMetrics, text_blink_opacity: f32) -> FrameInput {
    use crate::gpu::frame_input::ViewportSize;

    let w = (cell.width * COLS as f32).ceil() as u32;
    let h = (cell.height * ROWS as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(COLS, ROWS, "");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.text_blink_opacity = text_blink_opacity;
    input.content.cursor.visible = false;

    // Place 'A' in the BLINK cell and the normal cell.
    input.content.cells[BLINK_COL].ch = 'A';
    input.content.cells[BLINK_COL].flags = CellFlags::BLINK;
    input.content.cells[NORMAL_COL].ch = 'A';

    input
}

/// Extract the RGBA value at the center of a cell.
fn cell_pixel(pixels: &[u8], width: u32, col: usize, cell_w: f32, cell_h: f32) -> [u8; 4] {
    let cx = (col as f32 * cell_w + cell_w / 2.0) as u32;
    let cy = (cell_h / 2.0) as u32;
    let idx = ((cy * width + cx) * 4) as usize;
    [
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    ]
}

#[test]
fn text_blink_visible() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let input = blink_input(cell, 1.0);

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    let w = input.viewport.width;
    let h = input.viewport.height;

    if let Err(msg) = compare_with_reference("text_blink_visible", &pixels, w, h) {
        panic!("visual regression (text_blink_visible): {msg}");
    }

    // At opacity 1.0, BLINK cell should look the same as the normal cell.
    let blink_px = cell_pixel(&pixels, w, BLINK_COL, cell.width, cell.height);
    let normal_px = cell_pixel(&pixels, w, NORMAL_COL, cell.width, cell.height);
    let diff: i32 = (0..3)
        .map(|i| (blink_px[i] as i32 - normal_px[i] as i32).abs())
        .sum();
    assert!(
        diff < 15,
        "blink cell at opacity=1.0 should match normal cell, blink={blink_px:?} normal={normal_px:?}",
    );
}

#[test]
fn text_blink_hidden() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let input = blink_input(cell, 0.0);
    let bg = input.content.cells[BLINK_COL].bg;

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    let w = input.viewport.width;
    let h = input.viewport.height;

    if let Err(msg) = compare_with_reference("text_blink_hidden", &pixels, w, h) {
        panic!("visual regression (text_blink_hidden): {msg}");
    }

    // At opacity 0.0, BLINK cell's glyph should be invisible (matches bg).
    let blink_px = cell_pixel(&pixels, w, BLINK_COL, cell.width, cell.height);
    // Background is dark — pixel should be near the cell bg color.
    assert!(
        blink_px[0] < bg.r.saturating_add(30)
            && blink_px[1] < bg.g.saturating_add(30)
            && blink_px[2] < bg.b.saturating_add(30),
        "blink cell at opacity=0.0 should match background, got {blink_px:?} (bg={bg:?})",
    );

    // Normal cell should still be visible.
    let normal_px = cell_pixel(&pixels, w, NORMAL_COL, cell.width, cell.height);
    let brightness: u32 = normal_px[0] as u32 + normal_px[1] as u32 + normal_px[2] as u32;
    assert!(
        brightness > 100,
        "normal cell should still be visible, got {normal_px:?}",
    );
}

#[test]
fn text_blink_half() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let input = blink_input(cell, 0.5);

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    let w = input.viewport.width;
    let h = input.viewport.height;

    if let Err(msg) = compare_with_reference("text_blink_half", &pixels, w, h) {
        panic!("visual regression (text_blink_half): {msg}");
    }

    // At opacity 0.5, BLINK cell should be dimmer than the normal cell.
    let blink_px = cell_pixel(&pixels, w, BLINK_COL, cell.width, cell.height);
    let normal_px = cell_pixel(&pixels, w, NORMAL_COL, cell.width, cell.height);
    let blink_brightness: u32 = blink_px[0] as u32 + blink_px[1] as u32 + blink_px[2] as u32;
    let normal_brightness: u32 = normal_px[0] as u32 + normal_px[1] as u32 + normal_px[2] as u32;
    assert!(
        blink_brightness < normal_brightness,
        "blink cell at 0.5 should be dimmer than normal: blink={blink_brightness} normal={normal_brightness}",
    );
}

/// End-to-end blink test through `WindowRenderer::prepare()` with the
/// cursor-only fast path active (`content_changed = false`).
///
/// Renders 5 frames stepping opacity from 1.0 → 0.0, each with
/// `content_changed = false` (except frame 1). Proves:
/// 1. The fast path detects opacity changes and does a full rebuild.
/// 2. Intermediate opacities (fade) produce intermediate brightness.
/// 3. Non-BLINK cells stay constant throughout.
#[test]
fn text_blink_survives_fast_path() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let opacities = [1.0_f32, 0.75, 0.5, 0.25, 0.0];
    let mut blink_brightness: Vec<u32> = Vec::new();
    let mut normal_brightness: Vec<u32> = Vec::new();

    for (i, &opacity) in opacities.iter().enumerate() {
        let input = blink_input(cell, opacity);
        let w = input.viewport.width;
        let content_changed = i == 0; // Only first frame is "new content".
        let target = gpu.create_render_target(w, input.viewport.height);
        renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, content_changed);
        renderer.render_frame(&gpu, &pipelines, target.view());
        let pixels = gpu
            .read_render_target(&target)
            .expect("readback should succeed");

        let blink_px = cell_pixel(&pixels, w, BLINK_COL, cell.width, cell.height);
        let normal_px = cell_pixel(&pixels, w, NORMAL_COL, cell.width, cell.height);
        blink_brightness.push(blink_px[0] as u32 + blink_px[1] as u32 + blink_px[2] as u32);
        normal_brightness.push(normal_px[0] as u32 + normal_px[1] as u32 + normal_px[2] as u32);
    }

    // BLINK brightness must decrease monotonically across all 5 frames.
    for i in 0..opacities.len() - 1 {
        assert!(
            blink_brightness[i] > blink_brightness[i + 1],
            "BLINK cell must dim monotonically: opacity {} br={} should be > opacity {} br={}. \
             If equal, fast path skipped the rebuild or fade not applied.",
            opacities[i],
            blink_brightness[i],
            opacities[i + 1],
            blink_brightness[i + 1],
        );
    }

    // Non-BLINK cell must stay constant across all frames.
    for i in 0..opacities.len() - 1 {
        let diff = (normal_brightness[i] as i32 - normal_brightness[i + 1] as i32).abs();
        assert!(
            diff < 5,
            "non-BLINK cell should be constant: frame {} br={} vs frame {} br={}",
            i,
            normal_brightness[i],
            i + 1,
            normal_brightness[i + 1],
        );
    }
}

/// Cross-frame test: renders at 3 opacity levels in one function and asserts
/// that non-BLINK cell brightness is constant while BLINK cell brightness
/// decreases monotonically.
#[test]
fn text_blink_cross_frame_consistency() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();

    // Render 3 frames at different text_blink_opacity values.
    let input_1_0 = blink_input(cell, 1.0);
    let pixels_1_0 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_1_0);
    let input_0_5 = blink_input(cell, 0.5);
    let pixels_0_5 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_0_5);
    let input_0_0 = blink_input(cell, 0.0);
    let pixels_0_0 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_0_0);

    let w = input_1_0.viewport.width;

    // Extract non-BLINK cell pixel from each frame.
    let n_1_0 = cell_pixel(&pixels_1_0, w, NORMAL_COL, cell.width, cell.height);
    let n_0_5 = cell_pixel(&pixels_0_5, w, NORMAL_COL, cell.width, cell.height);
    let n_0_0 = cell_pixel(&pixels_0_0, w, NORMAL_COL, cell.width, cell.height);

    // Non-BLINK cell RGB should be constant across all 3 frames.
    // text_blink_opacity only affects BLINK-flagged cells in the prepare pipeline.
    for (a, b, label) in [
        (n_1_0, n_0_5, "1.0 vs 0.5"),
        (n_0_5, n_0_0, "0.5 vs 0.0"),
        (n_1_0, n_0_0, "1.0 vs 0.0"),
    ] {
        let diff: i32 = (0..3).map(|i| (a[i] as i32 - b[i] as i32).abs()).sum();
        assert!(
            diff < 5,
            "non-BLINK cell should be constant across frames ({label}): \
             {a:?} vs {b:?}, diff={diff}",
        );
    }

    // Extract BLINK cell pixel from each frame and compute brightness.
    let b_1_0 = cell_pixel(&pixels_1_0, w, BLINK_COL, cell.width, cell.height);
    let b_0_5 = cell_pixel(&pixels_0_5, w, BLINK_COL, cell.width, cell.height);
    let b_0_0 = cell_pixel(&pixels_0_0, w, BLINK_COL, cell.width, cell.height);

    let br_1_0: u32 = b_1_0[0] as u32 + b_1_0[1] as u32 + b_1_0[2] as u32;
    let br_0_5: u32 = b_0_5[0] as u32 + b_0_5[1] as u32 + b_0_5[2] as u32;
    let br_0_0: u32 = b_0_0[0] as u32 + b_0_0[1] as u32 + b_0_0[2] as u32;

    // BLINK cell brightness must decrease monotonically: 1.0 > 0.5 > 0.0.
    assert!(
        br_1_0 > br_0_5,
        "BLINK brightness should decrease: 1.0={br_1_0} should be > 0.5={br_0_5}",
    );
    assert!(
        br_0_5 > br_0_0,
        "BLINK brightness should decrease: 0.5={br_0_5} should be > 0.0={br_0_0}",
    );
}
