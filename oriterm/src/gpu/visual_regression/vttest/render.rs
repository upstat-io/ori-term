//! GPU-render helpers for vttest golden image tests.
//!
//! These free functions adapt a shared [`PtySession`] (from
//! `oriterm_test_support`) into a [`FrameInput`] for the GPU pipeline,
//! then run the headless render and compare the result against a
//! reference PNG. Pre-deduplication these were methods on a local
//! `VtTestSession` struct that owned its own PTY plumbing — see
//! `plans/tack-conformance/section-01-shared-pty-session.md`.

use oriterm_core::{Rgb, TermMode};
use oriterm_test_support::PtySession;

use crate::font::CellMetrics;
use crate::gpu::frame_input::{FrameInput, FramePalette, ViewportSize};
use crate::gpu::pipelines::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::visual_regression::{compare_with_reference, render_to_pixels};
use crate::gpu::window_renderer::WindowRenderer;

/// Build a [`FrameInput`] from the current `PtySession` grid state.
pub(super) fn frame_input(session: &PtySession, cell: CellMetrics) -> FrameInput {
    let cols = session.cols() as usize;
    let rows = session.rows() as usize;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let content = session.term().renderable_content();

    let fg = Rgb {
        r: 211,
        g: 215,
        b: 207,
    };
    // Palette bg must differ from the cell bg so the prepare phase emits
    // bg quads. Cells have bg=(0,0,0) from the terminal, so use a slightly
    // different palette bg. The renderer clears to palette bg, then draws
    // cell bg quads on top, then glyphs.
    let palette_bg = Rgb { r: 1, g: 1, b: 1 };

    let reverse_video = content.mode.contains(TermMode::REVERSE_VIDEO);

    // When DECSCNM is active, cell colors are already resolved against the
    // swapped palette in `renderable_content_into()`. The FramePalette
    // fg/bg must also be swapped so the clear color (screen background)
    // matches the swapped default background.
    let (frame_fg, frame_bg) = if reverse_video {
        (palette_bg, fg)
    } else {
        (fg, palette_bg)
    };
    let palette = FramePalette {
        background: frame_bg,
        foreground: frame_fg,
        cursor_color: Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
        opacity: 1.0,
        selection_fg: None,
        selection_bg: None,
    };

    FrameInput {
        content,
        viewport: ViewportSize::new(w, h),
        cell_size: cell,
        content_cols: cols,
        content_rows: rows,
        palette,
        selection: None,
        search: None,
        hovered_cell: None,
        hovered_url_segments: Vec::new(),
        mark_cursor: None,
        window_focused: true,
        reverse_video,
        fg_dim: 1.0,
        text_blink_opacity: 1.0,
        subpixel_positioning: true,
        prompt_marker_rows: Vec::new(),
    }
}

/// Build a [`FrameInput`] with a custom `text_blink_opacity` override.
pub(super) fn frame_input_with_blink(
    session: &PtySession,
    cell: CellMetrics,
    text_blink_opacity: f32,
) -> FrameInput {
    let mut input = frame_input(session, cell);
    input.text_blink_opacity = text_blink_opacity;
    input
}

/// Render the current `PtySession` grid through the GPU pipeline and
/// compare against a golden reference PNG.
pub(super) fn assert_golden(
    session: &PtySession,
    name: &str,
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
) {
    let cell = renderer.cell_metrics();
    let input = frame_input(session, cell);
    let w = input.viewport.width;
    let h = input.viewport.height;

    let pixels = render_to_pixels(gpu, pipelines, renderer, &input);
    if let Err(msg) = compare_with_reference(name, &pixels, w, h) {
        panic!("vttest visual regression ({name}): {msg}");
    }
}

/// Compute the average RGB brightness across all pixels in a grid cell.
///
/// Sums R+G+B for every pixel within the cell bounds and divides by
/// pixel count. This avoids false readings from sampling glyph counters
/// (holes inside letters like 'b', 'e', 'o').
pub(super) fn cell_brightness(
    pixels: &[u8],
    width: u32,
    col: usize,
    row: usize,
    cw: f32,
    ch: f32,
) -> u32 {
    let x0 = (col as f32 * cw) as u32;
    let y0 = (row as f32 * ch) as u32;
    let x1 = ((col + 1) as f32 * cw).ceil() as u32;
    let y1 = ((row + 1) as f32 * ch).ceil() as u32;

    let mut total: u64 = 0;
    let mut count: u64 = 0;
    for py in y0..y1 {
        for px in x0..x1 {
            let idx = ((py * width + px) * 4) as usize;
            total +=
                u64::from(pixels[idx]) + u64::from(pixels[idx + 1]) + u64::from(pixels[idx + 2]);
            count += 1;
        }
    }
    if count == 0 {
        return 0;
    }
    (total / count) as u32
}
