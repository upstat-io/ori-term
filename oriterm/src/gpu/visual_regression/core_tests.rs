//! Core visual regression tests.

use oriterm_core::{CellFlags, Column, CursorShape, Rgb};

use super::{compare_with_reference, headless_env, render_to_pixels};
use crate::gpu::frame_input::{FrameInput, ViewportSize};

#[test]
fn basic_grid() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 80usize;
    let rows = 24usize;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let text: String = (0..(cols * rows))
        .map(|i| {
            let ch = b' ' + (i % 95) as u8;
            ch as char
        })
        .collect();

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("basic_grid", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn colors_16() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 16usize;
    let rows = 2usize;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let ansi_colors: [Rgb; 16] = [
        Rgb { r: 0, g: 0, b: 0 },
        Rgb { r: 205, g: 0, b: 0 },
        Rgb { r: 0, g: 205, b: 0 },
        Rgb {
            r: 205,
            g: 205,
            b: 0,
        },
        Rgb { r: 0, g: 0, b: 238 },
        Rgb {
            r: 205,
            g: 0,
            b: 205,
        },
        Rgb {
            r: 0,
            g: 205,
            b: 205,
        },
        Rgb {
            r: 229,
            g: 229,
            b: 229,
        },
        Rgb {
            r: 127,
            g: 127,
            b: 127,
        },
        Rgb { r: 255, g: 0, b: 0 },
        Rgb { r: 0, g: 255, b: 0 },
        Rgb {
            r: 255,
            g: 255,
            b: 0,
        },
        Rgb {
            r: 92,
            g: 92,
            b: 255,
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
            r: 255,
            g: 255,
            b: 255,
        },
    ];

    let mut input = FrameInput::test_grid(cols, rows, "");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    for i in 0..16 {
        input.content.cells[i].bg = ansi_colors[i];
        input.content.cells[i].ch = ' ';

        let row1_idx = cols + i;
        input.content.cells[row1_idx].fg = ansi_colors[i];
        input.content.cells[row1_idx].bg = Rgb { r: 0, g: 0, b: 0 };
        input.content.cells[row1_idx].ch = '#';
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("colors_16", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn cursor_shapes() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 20usize;
    let rows = 5usize;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let shapes = [
        CursorShape::Block,
        CursorShape::Bar,
        CursorShape::Underline,
        CursorShape::HollowBlock,
    ];

    for (i, &shape) in shapes.iter().enumerate() {
        let mut input = FrameInput::test_grid(cols, rows, "");
        input.viewport = ViewportSize::new(w, h);
        input.cell_size = cell;
        input.content.cursor.column = Column(5);
        input.content.cursor.line = i;
        input.content.cursor.shape = shape;
        input.content.cursor.visible = true;
        input.palette.cursor_color = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };

        let name = format!("cursor_{shape:?}").to_lowercase();
        let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
        if let Err(msg) = compare_with_reference(&name, &pixels, w, h) {
            panic!("visual regression ({name}): {msg}");
        }
    }
}

#[test]
fn bold_italic() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 20usize;
    let rows = 4usize;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let text = "Regular text here   Bold text here     Italic text here    BoldItalic here     ";
    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    for col in 0..cols {
        input.content.cells[cols + col].flags = CellFlags::BOLD;
    }
    for col in 0..cols {
        input.content.cells[2 * cols + col].flags = CellFlags::ITALIC;
    }
    for col in 0..cols {
        input.content.cells[3 * cols + col].flags = CellFlags::BOLD | CellFlags::ITALIC;
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("bold_italic", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}
