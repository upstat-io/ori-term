//! Reference string visual regression tests.
//!
//! Each test renders a specific character set or style combination and
//! compares against a golden PNG. Covers ASCII, ligatures, box drawing,
//! block elements, braille, CJK, combining marks, powerline, and mixed styles.

use oriterm_core::CellFlags;

use crate::gpu::frame_input::{FrameInput, ViewportSize};

use super::{compare_with_reference, headless_env, render_to_pixels};

/// Set a cell as a wide character with its trailing spacer.
fn set_wide_char(input: &mut FrameInput, cols: usize, row: usize, col: usize, ch: char) {
    let idx = row * cols + col;
    input.content.cells[idx].ch = ch;
    input.content.cells[idx].flags = CellFlags::WIDE_CHAR;
    if col + 1 < cols {
        input.content.cells[idx + 1].ch = ' ';
        input.content.cells[idx + 1].flags = CellFlags::WIDE_CHAR_SPACER;
    }
}

#[test]
fn ascii_regular() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let text = "The quick brown fox jumps over the lazy dog 0123456789";
    let cols = text.len();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("ascii_regular", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn ascii_bold_italic() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let line = "The quick brown fox jumps over the lazy dog 0123456789";
    let cols = line.len();
    let rows = 4;
    let text = format!("{line}{line}{line}{line}",);
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Row 0: Regular (default). Row 1: Bold.
    for col in 0..cols {
        input.content.cells[cols + col].flags = CellFlags::BOLD;
    }
    // Row 2: Italic.
    for col in 0..cols {
        input.content.cells[2 * cols + col].flags = CellFlags::ITALIC;
    }
    // Row 3: BoldItalic.
    for col in 0..cols {
        input.content.cells[3 * cols + col].flags = CellFlags::BOLD | CellFlags::ITALIC;
    }

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("ascii_bold_italic", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn ligatures() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let text = "=> -> != === !== >= <= |> <| :: <<";
    let cols = text.len();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("ligatures", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn box_drawing() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols = 5;
    let rows = 4;
    // 4×5 box: ┌─┬─┐ / │ │ │ / ├─┼─┤ / └─┴─┘
    let text = "┌─┬─┐│ │ │├─┼─┤└─┴─┘";
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("box_drawing", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn block_elements() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let text = "█▓▒░▀▄▌▐▖▗▘▝▚▞";
    let cols = text.chars().count();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("block_elements", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn braille() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let text = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏⣿⠿⡿⢿";
    let cols = text.chars().count();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("braille", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn cjk_notdef() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    // "Hello你好世界" — CJK chars map to .notdef with embedded font.
    // 5 narrow + 4 wide = 13 columns.
    let cols = 13;
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, "Hello");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Manually set CJK wide chars after "Hello" (columns 5..12).
    let cjk = ['你', '好', '世', '界'];
    let mut col = 5;
    for ch in cjk {
        set_wide_char(&mut input, cols, 0, col, ch);
        col += 2;
    }

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("cjk_notdef", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn combining_marks() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    // Base characters with combining marks applied via zerowidth.
    // e + combining acute, n + combining tilde, u + combining diaeresis,
    // a + combining macron, o + combining diaeresis.
    let bases = ['e', ' ', 'n', ' ', 'u', ' ', 'a', ' ', 'o'];
    let marks: [&[char]; 9] = [
        &['\u{0301}'], // e + acute
        &[],
        &['\u{0303}'], // n + tilde
        &[],
        &['\u{0308}'], // u + diaeresis
        &[],
        &['\u{0304}'], // a + macron
        &[],
        &['\u{0308}'], // o + diaeresis
    ];

    let cols = bases.len();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let text: String = bases.iter().collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Attach combining marks via zerowidth.
    for (i, mark) in marks.iter().enumerate() {
        if !mark.is_empty() {
            input.content.cells[i].zerowidth = mark.to_vec();
        }
    }

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("combining_marks", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn powerline() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    // Powerline glyphs: right triangle, right thin, left triangle, left thin.
    let chars = [
        '\u{E0B0}', ' ', '\u{E0B1}', ' ', '\u{E0B2}', ' ', '\u{E0B3}',
    ];
    let cols = chars.len();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let text: String = chars.iter().collect();
    let mut input = FrameInput::test_grid(cols, rows, &text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("powerline", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}

#[test]
fn mixed_styles() {
    let Some((gpu, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let text = "Normal Bold Italic BoldIt Normal";
    let cols = text.len();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // "Normal " (0..7), "Bold " (7..12), "Italic " (12..19),
    // "BoldIt " (19..26), "Normal" (26..32).
    for col in 7..12 {
        input.content.cells[col].flags = CellFlags::BOLD;
    }
    for col in 12..19 {
        input.content.cells[col].flags = CellFlags::ITALIC;
    }
    for col in 19..26 {
        input.content.cells[col].flags = CellFlags::BOLD | CellFlags::ITALIC;
    }

    let pixels = render_to_pixels(&gpu, &mut renderer, &input);
    if let Err(msg) = compare_with_reference("mixed_styles", &pixels, w, h) {
        panic!("visual regression: {msg}");
    }
}
