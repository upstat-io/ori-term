//! Visual regression test infrastructure.
//!
//! Renders known terminal content to offscreen targets, reads back pixels,
//! and compares against reference PNGs with per-pixel fuzzy tolerance.
//!
//! # Workflow
//!
//! 1. First run (no reference): renders and saves as the reference PNG.
//! 2. Subsequent runs: renders, compares against reference with tolerance.
//! 3. On mismatch: saves `*_actual.png` and `*_diff.png` for inspection.
//! 4. `ORITERM_UPDATE_GOLDEN=1`: overwrites references with current output.
//!
//! # Running
//!
//! ```text
//! cargo test -p oriterm -- visual_regression
//! ```

mod decoration_tests;
mod dialog_helpers;
mod edge_case_tests;
mod meta_tests;
mod multi_size;
mod reference_tests;
mod settings_dialog;

use std::path::PathBuf;

use image::{ImageBuffer, Rgba, RgbaImage};
use oriterm_core::{CellFlags, Column, CursorShape, Rgb};

use super::frame_input::{FrameInput, ViewportSize};
use super::pipelines::GpuPipelines;
use super::state::GpuState;
use super::window_renderer::WindowRenderer;
use crate::font::{FontCollection, FontSet, GlyphFormat, HintingMode};

/// Per-channel tolerance for pixel comparison. Accounts for anti-aliasing
/// differences and minor rasterization variance across GPU drivers.
pub(super) const PIXEL_TOLERANCE: u8 = 2;

/// Maximum percentage of pixels allowed to differ before a test fails.
/// 99.5% of pixels must match (at most 0.5% may differ).
pub(super) const MAX_MISMATCH_PERCENT: f64 = 0.5;

/// Default test font parameters.
const TEST_FONT_WEIGHT: u16 = 400;
const TEST_FONT_SIZE_PT: f32 = 12.0;
const TEST_DPI: f32 = 96.0;

/// Directory for reference PNG files, relative to the crate root.
pub(super) fn reference_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/references")
}

/// Attempt to create a headless rendering environment with embedded font.
///
/// Uses `FontSet::embedded()` for deterministic output regardless of system
/// fonts. Returns `None` if no GPU adapter is available.
pub(super) fn headless_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_with_config(TEST_FONT_SIZE_PT, TEST_DPI)
}

/// Headless rendering environment with configurable font size and DPI.
pub(super) fn headless_env_with_config(
    size_pt: f32,
    dpi: f32,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_full(size_pt, dpi, GlyphFormat::Alpha)
}

/// Headless rendering environment with configurable font size, DPI, and glyph format.
pub(super) fn headless_env_full(
    size_pt: f32,
    dpi: f32,
    format: GlyphFormat,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_with_hinting(size_pt, dpi, format, HintingMode::Full)
}

/// Headless rendering environment with full control over font size, DPI,
/// glyph format, and hinting mode.
pub(super) fn headless_env_with_hinting(
    size_pt: f32,
    dpi: f32,
    format: GlyphFormat,
    hinting: HintingMode,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        FontSet::embedded(),
        size_pt,
        dpi,
        format,
        TEST_FONT_WEIGHT,
        hinting,
    )
    .ok()?;
    let renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, None);
    Some((gpu, pipelines, renderer))
}

/// Render a `FrameInput` to RGBA pixels via the headless pipeline.
pub(super) fn render_to_pixels(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    input: &FrameInput,
) -> Vec<u8> {
    render_to_pixels_with_origin(gpu, pipelines, renderer, input, (0.0, 0.0))
}

/// Render a `FrameInput` to RGBA pixels with a custom grid origin offset.
///
/// The `origin` shifts all cell positions, simulating chrome height.
pub(super) fn render_to_pixels_with_origin(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    input: &FrameInput,
    origin: (f32, f32),
) -> Vec<u8> {
    let w = input.viewport.width;
    let h = input.viewport.height;
    let target = gpu.create_render_target(w, h);
    renderer.prepare(input, gpu, pipelines, origin, true, true);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

/// Compare rendered pixels against a reference PNG.
///
/// - `ORITERM_UPDATE_GOLDEN=1`: overwrites the reference and returns Ok.
/// - If reference doesn't exist: saves `pixels` as the reference and passes.
/// - If reference exists: compares with `PIXEL_TOLERANCE` and
///   `MAX_MISMATCH_PERCENT`. On failure, saves `*_actual.png` and
///   `*_diff.png` alongside the reference.
///
/// Returns `Ok(())` on match, `Err(message)` on mismatch.
pub(super) fn compare_with_reference(
    name: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let ref_dir = reference_dir();
    let ref_path = ref_dir.join(format!("{name}.png"));
    let actual_path = ref_dir.join(format!("{name}_actual.png"));
    let diff_path = ref_dir.join(format!("{name}_diff.png"));

    let actual: RgbaImage =
        ImageBuffer::from_raw(width, height, pixels.to_vec()).expect("pixel buffer size mismatch");

    // Regeneration mode: overwrite reference with current output.
    if std::env::var("ORITERM_UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::create_dir_all(&ref_dir).expect("failed to create reference dir");
        actual
            .save(&ref_path)
            .expect("failed to save reference PNG");
        eprintln!(
            "golden updated: {} ({}×{})",
            ref_path.display(),
            width,
            height,
        );
        // Clean up stale artifacts.
        let _ = std::fs::remove_file(&actual_path);
        let _ = std::fs::remove_file(&diff_path);
        return Ok(());
    }

    if !ref_path.exists() {
        std::fs::create_dir_all(&ref_dir).expect("failed to create reference dir");
        actual
            .save(&ref_path)
            .expect("failed to save reference PNG");
        eprintln!(
            "reference saved: {} ({}×{}). Re-run to compare.",
            ref_path.display(),
            width,
            height,
        );
        return Ok(());
    }

    let reference = image::open(&ref_path)
        .expect("failed to open reference PNG")
        .to_rgba8();

    if reference.width() != width || reference.height() != height {
        actual
            .save(&actual_path)
            .expect("failed to save actual PNG");
        return Err(format!(
            "size mismatch: reference is {}×{}, actual is {width}×{height}. Actual saved to {}",
            reference.width(),
            reference.height(),
            actual_path.display(),
        ));
    }

    let (mismatches, diff_img) = pixel_diff(&reference, &actual, PIXEL_TOLERANCE);

    if mismatches > 0 {
        let total = (width * height) as usize;
        let pct = mismatches as f64 / total as f64 * 100.0;

        if pct > MAX_MISMATCH_PERCENT {
            actual
                .save(&actual_path)
                .expect("failed to save actual PNG");
            diff_img.save(&diff_path).expect("failed to save diff PNG");

            Err(format!(
                "{mismatches}/{total} pixels differ ({pct:.2}%, threshold {MAX_MISMATCH_PERCENT}%). \
                 tolerance=±{PIXEL_TOLERANCE}\n\
                 actual: {}\n\
                 diff:   {}",
                actual_path.display(),
                diff_path.display(),
            ))
        } else {
            // Within threshold — clean up stale artifacts.
            let _ = std::fs::remove_file(&actual_path);
            let _ = std::fs::remove_file(&diff_path);
            Ok(())
        }
    } else {
        // Clean up any stale actual/diff from previous failures.
        let _ = std::fs::remove_file(&actual_path);
        let _ = std::fs::remove_file(&diff_path);
        Ok(())
    }
}

/// Compute per-pixel diff between two images.
///
/// Returns the number of mismatched pixels and a diff image where:
/// - Matching pixels are transparent black.
/// - Mismatched pixels are red with full alpha.
pub(super) fn pixel_diff(
    reference: &RgbaImage,
    actual: &RgbaImage,
    tolerance: u8,
) -> (usize, RgbaImage) {
    let w = reference.width();
    let h = reference.height();
    let mut diff = RgbaImage::new(w, h);
    let mut count = 0;

    for y in 0..h {
        for x in 0..w {
            let r = reference.get_pixel(x, y);
            let a = actual.get_pixel(x, y);

            let matches =
                r.0.iter()
                    .zip(a.0.iter())
                    .all(|(&rv, &av)| (rv as i16 - av as i16).unsigned_abs() <= tolerance as u16);

            if !matches {
                diff.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                count += 1;
            }
        }
    }

    (count, diff)
}

// Existing visual regression tests.

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
