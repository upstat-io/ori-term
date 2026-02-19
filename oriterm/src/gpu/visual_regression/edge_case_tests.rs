//! Grid edge case visual regression tests.
//!
//! Tests boundary conditions: wide characters at terminal edge, pure
//! background cells, and empty grids.

use oriterm_core::{CellFlags, Rgb};

use crate::gpu::frame_input::{FrameInput, ViewportSize};

use super::{compare_with_reference, headless_env, render_to_pixels};

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
