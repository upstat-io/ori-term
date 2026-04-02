//! GPU visual regression tests for cursor opacity at different fade levels.
//!
//! Verifies the fade-blink alpha pipeline: `cursor_opacity` passes through
//! `build_cursor()` → `push_cursor(alpha)` → bg shader premultiplied blending.
//! Tests render at opacity 1.0, 0.5, and 0.0, then compare cursor pixel
//! brightness against expectations.

use oriterm_core::{Column, CursorShape, Rgb};

use crate::gpu::frame_input::{FrameInput, ViewportSize};

use super::{compare_with_reference, headless_env, render_to_pixels_with_opacity};

/// Cursor pixel position in the rendered image (column 5, row 0).
const CURSOR_COL: usize = 5;

/// Test grid dimensions.
const COLS: usize = 20;
const ROWS: usize = 5;

/// Set up a frame input with a visible white block cursor at a known position.
fn cursor_input(cell: crate::font::CellMetrics) -> FrameInput {
    let w = (cell.width * COLS as f32).ceil() as u32;
    let h = (cell.height * ROWS as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(COLS, ROWS, "");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = true;
    input.content.cursor.column = Column(CURSOR_COL);
    input.content.cursor.line = 0;
    input.content.cursor.shape = CursorShape::Block;
    input.palette.cursor_color = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    input
}

/// Extract the RGBA value at the center of the cursor cell.
fn cursor_pixel(pixels: &[u8], width: u32, cell_w: f32, cell_h: f32) -> [u8; 4] {
    let cx = (CURSOR_COL as f32 * cell_w + cell_w / 2.0) as u32;
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
fn cursor_opacity_full() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let input = cursor_input(cell);

    let pixels = render_to_pixels_with_opacity(&gpu, &pipelines, &mut renderer, &input, 1.0);

    // Golden image comparison.
    let w = input.viewport.width;
    let h = input.viewport.height;
    if let Err(msg) = compare_with_reference("cursor_opacity_full", &pixels, w, h) {
        panic!("visual regression (cursor_opacity_full): {msg}");
    }

    // White cursor on black background at full opacity → pixel near white.
    let [r, g, b, _a] = cursor_pixel(&pixels, w, cell.width, cell.height);
    assert!(
        r > 200 && g > 200 && b > 200,
        "cursor pixel at opacity=1.0 should be near white, got ({r}, {g}, {b})",
    );
}

#[test]
fn cursor_opacity_zero() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let input = cursor_input(cell);

    let pixels = render_to_pixels_with_opacity(&gpu, &pipelines, &mut renderer, &input, 0.0);

    // Golden image comparison.
    let w = input.viewport.width;
    let h = input.viewport.height;
    if let Err(msg) = compare_with_reference("cursor_opacity_zero", &pixels, w, h) {
        panic!("visual regression (cursor_opacity_zero): {msg}");
    }

    // No cursor emitted at opacity=0.0 → pixel matches default palette background.
    let [r, g, b, _a] = cursor_pixel(&pixels, w, cell.width, cell.height);
    assert!(
        r < 60 && g < 60 && b < 60,
        "cursor pixel at opacity=0.0 should match background, got ({r}, {g}, {b})",
    );
}

#[test]
fn cursor_opacity_half() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let input = cursor_input(cell);

    let pixels = render_to_pixels_with_opacity(&gpu, &pipelines, &mut renderer, &input, 0.5);

    // Golden image comparison.
    let w = input.viewport.width;
    let h = input.viewport.height;
    if let Err(msg) = compare_with_reference("cursor_opacity_half", &pixels, w, h) {
        panic!("visual regression (cursor_opacity_half): {msg}");
    }

    // Premultiplied alpha: white * 0.5 blended with dark background.
    // Expected: roughly halfway between background (~30) and white (~255).
    let [r, g, b, _a] = cursor_pixel(&pixels, w, cell.width, cell.height);
    assert!(
        r > 100 && r < 220 && g > 100 && g < 220 && b > 100 && b < 220,
        "cursor pixel at opacity=0.5 should be intermediate, got ({r}, {g}, {b})",
    );
}
