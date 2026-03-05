//! Text decoration visual regression tests.
//!
//! Tests underline variants, strikethrough, dim text, and inverse video
//! through the full GPU rendering pipeline.

use oriterm_core::{CellFlags, Rgb};

use crate::gpu::frame_input::{FrameInput, ViewportSize};

use super::{compare_with_reference, headless_env, render_to_pixels};

#[test]
fn underline_styles() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 20;
    let rows = 5;
    let full_text = format!(
        "{:<cols$}{:<cols$}{:<cols$}{:<cols$}{:<cols$}",
        "Single underline",
        "Double underline",
        "Curly underline",
        "Dotted underline",
        "Dashed underline",
    );
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &full_text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Row 0: single underline.
    for col in 0..cols {
        input.content.cells[col].flags = CellFlags::UNDERLINE;
    }
    // Row 1: double underline.
    for col in 0..cols {
        input.content.cells[cols + col].flags = CellFlags::DOUBLE_UNDERLINE;
    }
    // Row 2: curly underline.
    for col in 0..cols {
        input.content.cells[2 * cols + col].flags = CellFlags::CURLY_UNDERLINE;
    }
    // Row 3: dotted underline.
    for col in 0..cols {
        input.content.cells[3 * cols + col].flags = CellFlags::DOTTED_UNDERLINE;
    }
    // Row 4: dashed underline.
    for col in 0..cols {
        input.content.cells[4 * cols + col].flags = CellFlags::DASHED_UNDERLINE;
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("underline_styles", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn strikethrough() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 30;
    let rows = 2;
    let text = format!(
        "{:<cols$}{:<cols$}",
        "Normal text for comparison", "Strikethrough text here",
    );
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Row 1: strikethrough.
    for col in 0..cols {
        input.content.cells[cols + col].flags = CellFlags::STRIKETHROUGH;
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("strikethrough", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn underline_with_strikethrough() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 30;
    let rows = 1;
    let text = format!("{:<cols$}", "Underline + Strikethrough");
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    for col in 0..cols {
        input.content.cells[col].flags = CellFlags::UNDERLINE | CellFlags::STRIKETHROUGH;
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("underline_with_strikethrough", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn bold_strikethrough() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 30;
    let rows = 1;
    let text = format!("{:<cols$}", "Bold + Strikethrough text");
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    for col in 0..cols {
        input.content.cells[col].flags = CellFlags::BOLD | CellFlags::STRIKETHROUGH;
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("bold_strikethrough", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn underline_color() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 30;
    let rows = 2;
    let text = format!(
        "{:<cols$}{:<cols$}",
        "Default underline color", "Custom underline color (red)",
    );
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Row 0: underline with default fg color.
    for col in 0..cols {
        input.content.cells[col].flags = CellFlags::UNDERLINE;
    }
    // Row 1: underline with explicit red color (SGR 58).
    let red = Rgb { r: 255, g: 0, b: 0 };
    for col in 0..cols {
        input.content.cells[cols + col].flags = CellFlags::UNDERLINE;
        input.content.cells[cols + col].underline_color = Some(red);
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("underline_color", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn dim_text() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 30;
    let rows = 2;
    let text = format!(
        "{:<cols$}{:<cols$}",
        "Normal brightness text", "Dimmed brightness text",
    );
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Row 1: simulate dimmed text by reducing fg brightness.
    // The extract phase resolves DIM to a dimmed RGB value; since test_grid
    // bypasses extract, we set the resolved dimmed color directly.
    let dim_fg = Rgb {
        r: 105,
        g: 107,
        b: 103,
    };
    for col in 0..cols {
        input.content.cells[cols + col].fg = dim_fg;
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("dim_text", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn inverse_video() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 30;
    let rows = 2;
    let text = format!(
        "{:<cols$}{:<cols$}",
        "Normal video mode", "Inverse video mode",
    );
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Row 1: simulate inverse by swapping fg/bg.
    // The extract phase resolves INVERSE to swapped colors; since test_grid
    // bypasses extract, we swap the colors directly.
    for col in 0..cols {
        let cell = &mut input.content.cells[cols + col];
        std::mem::swap(&mut cell.fg, &mut cell.bg);
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("inverse_video", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}
