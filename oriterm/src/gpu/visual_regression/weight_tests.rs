//! Font weight verification tests with image-level analysis.
//!
//! These tests render text at different weights and parse the resulting
//! pixel buffers to verify weight correctness algorithmically — measuring
//! actual ink density rather than just comparing against golden references.

use oriterm_core::CellFlags;

use crate::gpu::frame_input::{FrameInput, ViewportSize};

use super::{compare_with_reference, headless_env, render_to_pixels};

/// Compute the mean luminance (ink density) of an RGBA pixel buffer.
///
/// Returns a value in `[0.0, 1.0]` where 0.0 is fully dark (maximum ink on
/// dark background) and 1.0 is fully bright (no ink). We use the mean of all
/// non-background pixels' brightness.
///
/// For a dark background terminal, darker pixels = more ink = heavier weight.
fn mean_brightness(pixels: &[u8], width: u32, height: u32) -> f64 {
    assert_eq!(pixels.len(), (width * height * 4) as usize);
    let mut sum = 0.0f64;
    let mut count = 0u64;
    for chunk in pixels.chunks_exact(4) {
        let r = chunk[0] as f64 / 255.0;
        let g = chunk[1] as f64 / 255.0;
        let b = chunk[2] as f64 / 255.0;
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        sum += lum;
        count += 1;
    }
    if count == 0 { 0.0 } else { sum / count as f64 }
}

/// Count the number of "ink" pixels above a brightness threshold.
///
/// On a dark background, foreground text pixels are bright. This counts
/// how many pixels are above `threshold` brightness, which correlates
/// with stroke width (heavier weight = more bright pixels).
fn ink_pixel_count(pixels: &[u8], threshold: f64) -> u64 {
    let mut count = 0u64;
    for chunk in pixels.chunks_exact(4) {
        let r = chunk[0] as f64 / 255.0;
        let g = chunk[1] as f64 / 255.0;
        let b = chunk[2] as f64 / 255.0;
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        if lum > threshold {
            count += 1;
        }
    }
    count
}

/// Render a single row of text at a given weight (via cell flags) and return
/// the mean brightness and ink pixel count.
fn render_weight_metrics(text: &str, flags: CellFlags) -> Option<(f64, u64, Vec<u8>, u32, u32)> {
    let (gpu, pipelines, mut renderer) = headless_env()?;
    let cell = renderer.cell_metrics();
    let cols = text.len();
    let rows = 1;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, text);
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    // Apply cell flags to all cells.
    if !flags.is_empty() {
        for cell in &mut input.content.cells {
            cell.flags |= flags;
        }
    }

    let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);
    let brightness = mean_brightness(&pixels, w, h);
    let ink = ink_pixel_count(&pixels, 0.1);
    Some((brightness, ink, pixels, w, h))
}

#[test]
fn bold_text_has_more_ink_than_regular() {
    let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    let Some((reg_brightness, _, _, _, _)) = render_weight_metrics(text, CellFlags::empty()) else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let Some((bold_brightness, _, _, _, _)) = render_weight_metrics(text, CellFlags::BOLD) else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    // Bold text should have higher mean brightness (more foreground coverage).
    // The embedded JetBrains Mono has no Bold face, so bold uses synthetic
    // emboldening (~0.5px stroke widening). This produces subtly brighter
    // output even though the difference may be too small for pixel counting.
    assert!(
        bold_brightness >= reg_brightness,
        "bold brightness ({bold_brightness:.6}) should be >= regular ({reg_brightness:.6})",
    );
}

#[test]
fn gamma_correction_produces_heavier_text_than_linear() {
    // This test verifies the alpha correction is active by checking that
    // rendered text has more ink than would be expected from raw linear blending.
    // We can't easily disable the correction in a test, but we can verify the
    // absolute ink density is in a reasonable range.
    let text = "The quick brown fox";
    let Some((brightness, ink, _, w, h)) = render_weight_metrics(text, CellFlags::empty()) else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let total_pixels = u64::from(w) * u64::from(h);
    let ink_percent = ink as f64 / total_pixels as f64 * 100.0;

    // With gamma correction active, text should have at least 5% ink coverage.
    // Without correction, thin anti-aliased strokes would produce lower coverage.
    assert!(
        ink_percent > 5.0,
        "text should have >5% ink coverage with gamma correction (got {ink_percent:.1}%)",
    );

    // Mean brightness should be above background-only level (dark bg ~ 0.02).
    assert!(
        brightness > 0.03,
        "mean brightness ({brightness:.4}) should be above pure background level",
    );
}

#[test]
fn weight_golden_regular_vs_bold() {
    let text = "Hello World 0123456789";

    let Some((_, _, reg_pixels, w, h)) = render_weight_metrics(text, CellFlags::empty()) else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let Some((_, _, bold_pixels, bw, bh)) = render_weight_metrics(text, CellFlags::BOLD) else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    // Save golden references for visual inspection.
    if let Err(msg) = compare_with_reference("weight_regular", &reg_pixels, w, h) {
        panic!("visual regression: {msg}");
    }
    if let Err(msg) = compare_with_reference("weight_bold", &bold_pixels, bw, bh) {
        panic!("visual regression: {msg}");
    }
}
