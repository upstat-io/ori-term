//! Reference string visual regression tests.
//!
//! Each test renders a specific character set or style combination and
//! compares against a golden PNG. Covers ASCII, ligatures, box drawing,
//! block elements, braille, CJK, combining marks, powerline, and mixed styles.

use oriterm_core::CellFlags;

use crate::font::GlyphFormat;
use crate::gpu::frame_input::{FrameInput, ViewportSize};

use crate::font::HintingMode;

use super::{
    TEST_DPI, TEST_FONT_SIZE_PT, compare_with_reference, headless_env, headless_env_full,
    headless_env_with_hinting, render_to_pixels,
};

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

/// Hinting visibly changes pixel patterns by snapping outlines to the grid.
///
/// Renders the same reference string with `HintingMode::Full` and
/// `HintingMode::None`, asserts the outputs differ, and saves the hinted
/// version as a golden image for regression tracking.
#[test]
fn hinted_vs_unhinted() {
    let Some((gpu_hint, mut renderer_hint)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let Some((gpu_none, mut renderer_none)) = headless_env_with_hinting(
        TEST_FONT_SIZE_PT,
        TEST_DPI,
        GlyphFormat::Alpha,
        HintingMode::None,
    ) else {
        eprintln!("skipped: no GPU adapter available for unhinted env");
        return;
    };

    let cell_hint = renderer_hint.cell_metrics();
    let cell_none = renderer_none.cell_metrics();
    let text = "The quick brown fox jumps over the lazy dog 0123456789";
    let cols = text.len();
    let rows = 1;

    // Hinted render.
    let w_h = (cell_hint.width * cols as f32).ceil() as u32;
    let h_h = (cell_hint.height * rows as f32).ceil() as u32;
    let mut input_h = FrameInput::test_grid(cols, rows, text);
    input_h.viewport = ViewportSize::new(w_h, h_h);
    input_h.cell_size = cell_hint;
    input_h.content.cursor.visible = false;
    let pixels_hinted = render_to_pixels(&gpu_hint, &mut renderer_hint, &input_h);

    // Unhinted render.
    let w_n = (cell_none.width * cols as f32).ceil() as u32;
    let h_n = (cell_none.height * rows as f32).ceil() as u32;
    let mut input_n = FrameInput::test_grid(cols, rows, text);
    input_n.viewport = ViewportSize::new(w_n, h_n);
    input_n.cell_size = cell_none;
    input_n.content.cursor.visible = false;
    let pixels_unhinted = render_to_pixels(&gpu_none, &mut renderer_none, &input_n);

    // Both should render non-empty output.
    assert!(
        pixels_hinted.iter().any(|&b| b > 0),
        "hinted render should not be all zeros",
    );
    assert!(
        pixels_unhinted.iter().any(|&b| b > 0),
        "unhinted render should not be all zeros",
    );

    // Hinting snaps outlines to the pixel grid, producing different rasterization
    // patterns than unhinted rendering which preserves outline shape.
    assert_ne!(
        pixels_hinted, pixels_unhinted,
        "hinted and unhinted renders should differ",
    );

    // Save hinted render as golden image for regression tracking.
    if let Err(msg) = compare_with_reference("hinted_full", &pixels_hinted, w_h, h_h) {
        panic!("visual regression: {msg}");
    }
}

/// Subpixel LCD rendering produces visually distinct output from grayscale.
///
/// Renders the same reference string in both SubpixelRgb and Alpha modes,
/// asserts the pixel outputs differ, and saves the subpixel version as a
/// golden image for regression tracking.
#[test]
fn subpixel_vs_grayscale() {
    let Some((gpu_gray, mut renderer_gray)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let Some((gpu_sub, mut renderer_sub)) =
        headless_env_full(TEST_FONT_SIZE_PT, TEST_DPI, GlyphFormat::SubpixelRgb)
    else {
        eprintln!("skipped: no GPU adapter available for subpixel env");
        return;
    };

    let cell_gray = renderer_gray.cell_metrics();
    let cell_sub = renderer_sub.cell_metrics();
    let text = "The quick brown fox jumps over the lazy dog 0123456789";
    let cols = text.len();
    let rows = 1;

    // Grayscale render.
    let w_g = (cell_gray.width * cols as f32).ceil() as u32;
    let h_g = (cell_gray.height * rows as f32).ceil() as u32;
    let mut input_g = FrameInput::test_grid(cols, rows, text);
    input_g.viewport = ViewportSize::new(w_g, h_g);
    input_g.cell_size = cell_gray;
    input_g.content.cursor.visible = false;
    let pixels_gray = render_to_pixels(&gpu_gray, &mut renderer_gray, &input_g);

    // Subpixel render.
    let w_s = (cell_sub.width * cols as f32).ceil() as u32;
    let h_s = (cell_sub.height * rows as f32).ceil() as u32;
    let mut input_s = FrameInput::test_grid(cols, rows, text);
    input_s.viewport = ViewportSize::new(w_s, h_s);
    input_s.cell_size = cell_sub;
    input_s.content.cursor.visible = false;
    let pixels_subpx = render_to_pixels(&gpu_sub, &mut renderer_sub, &input_s);

    // Both should render non-empty output.
    assert!(
        pixels_gray.iter().any(|&b| b > 0),
        "grayscale render should not be all zeros",
    );
    assert!(
        pixels_subpx.iter().any(|&b| b > 0),
        "subpixel render should not be all zeros",
    );

    // Subpixel rendering should produce different pixel data from grayscale.
    // The per-channel coverage in subpixel mode creates distinct anti-aliasing
    // patterns that differ from single-channel alpha blending.
    assert_ne!(
        pixels_gray, pixels_subpx,
        "subpixel and grayscale renders should differ",
    );

    // Save subpixel render as golden image for regression tracking.
    if let Err(msg) = compare_with_reference("subpixel_rgb", &pixels_subpx, w_s, h_s) {
        panic!("visual regression: {msg}");
    }
}
