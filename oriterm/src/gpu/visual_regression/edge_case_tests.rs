//! Grid edge case visual regression tests.
//!
//! Tests boundary conditions: wide characters at terminal edge, pure
//! background cells, empty grids, and fractional-origin seam detection.

use oriterm_core::{CellFlags, Rgb};

use crate::gpu::frame_input::{FrameInput, ViewportSize};

use super::{compare_with_reference, headless_env, render_to_pixels, render_to_pixels_with_origin};

#[test]
fn wide_char_at_edge() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    // 80-column grid. Place a CJK char at column 78 (occupies 78+79).
    // Then try another at column 79 (only 1 column left — should wrap or
    // truncate). Tests that wide chars near the edge don't overflow.
    let cols = 80;
    let rows = 2;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, "");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Fill row 0 with ASCII up to col 77, then wide char at 78.
    for col in 0..78 {
        input.content.cells[col].ch = 'A';
    }
    input.content.cells[78].ch = '界';
    input.content.cells[78].flags = CellFlags::WIDE_CHAR;
    input.content.cells[79].ch = ' ';
    input.content.cells[79].flags = CellFlags::WIDE_CHAR_SPACER;

    // Row 1: wide char exactly at col 79 — only spacer column remains.
    // In real terminals this wraps. In test grid, just mark the flags.
    for col in 0..79 {
        input.content.cells[cols + col].ch = 'B';
    }
    // Last column gets a narrow char (wide char wouldn't fit).
    input.content.cells[cols + 79].ch = 'X';

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("wide_char_at_edge", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn background_only() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 8;
    let rows = 2;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, "");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Row 0: 8 different background colors, space characters (no glyphs).
    let colors = [
        Rgb { r: 255, g: 0, b: 0 },
        Rgb { r: 0, g: 255, b: 0 },
        Rgb { r: 0, g: 0, b: 255 },
        Rgb {
            r: 255,
            g: 255,
            b: 0,
        },
        Rgb {
            r: 255,
            g: 0,
            b: 255,
        },
        Rgb {
            r: 0,
            g: 255,
            b: 255,
        },
        Rgb {
            r: 128,
            g: 128,
            b: 128,
        },
        Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
    ];
    for (col, &color) in colors.iter().enumerate() {
        input.content.cells[col].ch = ' ';
        input.content.cells[col].bg = color;
    }
    // Row 1: same colors with '#' characters to show glyph-on-background.
    for (col, &color) in colors.iter().enumerate() {
        input.content.cells[cols + col].ch = '#';
        input.content.cells[cols + col].bg = color;
        input.content.cells[cols + col].fg = Rgb { r: 0, g: 0, b: 0 };
    }

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("background_only", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn empty_grid() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 80;
    let rows = 24;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    // Fully blank grid — all space characters, default colors.
    let mut input = FrameInput::test_grid(cols, rows, "");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("empty_grid", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

/// Scan rendered pixels for horizontal seam lines in the grid region.
///
/// A seam is a full scanline where every pixel in the grid columns is
/// near-black (background). Returns the list of y-coordinates with seams.
fn find_block_seams(
    pixels: &[u8],
    viewport_w: u32,
    viewport_h: u32,
    origin_y: f32,
    cell_w: f32,
    cell_h: f32,
    cols: usize,
    rows: usize,
) -> Vec<u32> {
    let grid_y_start = origin_y.floor() as u32;
    let grid_y_end = (origin_y + cell_h * rows as f32).ceil() as u32;
    let grid_x_end = (cell_w * cols as f32).ceil() as u32;
    let stride = viewport_w as usize * 4;

    let mut seams = Vec::new();
    for y in grid_y_start..grid_y_end.min(viewport_h) {
        let row_start = y as usize * stride;
        let mut all_dark = true;
        for x in 0..grid_x_end.min(viewport_w) {
            let offset = row_start + x as usize * 4;
            let r = pixels[offset];
            let g = pixels[offset + 1];
            let b = pixels[offset + 2];
            // Threshold: any pixel brighter than 10 means the block was rendered.
            if r > 10 || g > 10 || b > 10 {
                all_dark = false;
                break;
            }
        }
        if all_dark {
            seams.push(y);
        }
    }
    seams
}

/// Build a FrameInput filled with full-block characters (█) — white on black.
fn full_block_input(
    cols: usize,
    rows: usize,
    cell: crate::font::CellMetrics,
    w: u32,
    h: u32,
) -> FrameInput {
    let block: String = "\u{2588}".repeat(cols).repeat(rows);
    let mut input = FrameInput::test_grid(cols, rows, &block);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let fg = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    let bg = Rgb { r: 0, g: 0, b: 0 };
    for cell_data in &mut input.content.cells {
        cell_data.fg = fg;
        cell_data.bg = bg;
    }
    input.palette.foreground = fg;
    input.palette.background = bg;
    input
}

/// Prove the bug: a raw fractional y-origin produces visible seams between
/// adjacent full-block rows.
///
/// At 125% DPI with 82px logical chrome, the physical origin is
/// `82.0 * 1.25 = 102.5` — a half-pixel boundary. Without rounding, the
/// GPU rasterizes a 1px gap between block rows.
#[test]
fn fractional_origin_produces_block_seams() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 10usize;
    let rows = 6usize;
    let origin_y: f32 = 102.5;

    let grid_pixel_h = (cell.height * rows as f32).ceil() as u32;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = origin_y.ceil() as u32 + grid_pixel_h;
    let input = full_block_input(cols, rows, cell, w, h);

    let pixels = render_to_pixels_with_origin(&gpu, &mut renderer, &input, (0.0, origin_y));
    let seams = find_block_seams(&pixels, w, h, origin_y, cell.width, cell.height, cols, rows);

    assert!(
        !seams.is_empty(),
        "expected seams from fractional origin {origin_y} — if this fails, \
         the GPU driver may not reproduce the issue (test still valid as guard)",
    );
}

/// Sanity baseline: integer y-origin never produces seams.
#[test]
fn integer_origin_no_block_seams() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 10usize;
    let rows = 6usize;
    let origin_y: f32 = 102.0;

    let grid_pixel_h = (cell.height * rows as f32).ceil() as u32;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = origin_y as u32 + grid_pixel_h;
    let input = full_block_input(cols, rows, cell, w, h);

    let pixels = render_to_pixels_with_origin(&gpu, &mut renderer, &input, (0.0, origin_y));
    let seams = find_block_seams(&pixels, w, h, origin_y, cell.width, cell.height, cols, rows);

    assert!(
        seams.is_empty(),
        "seams detected at scanlines {seams:?} — integer origin should never produce seams",
    );
}
