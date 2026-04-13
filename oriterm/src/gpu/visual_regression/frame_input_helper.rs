//! Shared `PtySession` to `FrameInput` builder for visual regression tests.
//!
//! Both `vttest/render.rs` and `tack/mod.rs` need to construct a
//! [`FrameInput`] from a live [`PtySession`] running a terminal program
//! (vttest or tack) under ori_term's pinned terminfo. The construction
//! is algorithmically identical — same palette constants, same
//! reverse-video handling, same `subpixel_positioning: true`,
//! same unused selection/search/hover/mark fields. This module is
//! the single canonical home for that construction; duplicating it
//! is `LEAK:algorithmic-duplication` and will be caught by
//! `/impl-hygiene-review`.

use oriterm_core::{Rgb, TermMode};
use oriterm_test_support::PtySession;

use crate::font::CellMetrics;
use crate::gpu::frame_input::{FrameInput, FramePalette, ViewportSize};

/// Build a [`FrameInput`] from a live [`PtySession`] with the standard
/// golden-test palette.
///
/// Uses the canonical fg `(211, 215, 207)` / palette_bg `(1, 1, 1)`
/// pair and all overlay fields (`selection`, `search`, `hovered_cell`,
/// etc.) set to their neutral defaults. `subpixel_positioning` is
/// caller-controlled: legacy vttest/tack goldens pass `true`;
/// spec-conformance goldens pass `false` for deterministic pixel output.
///
/// Both vttest and tack GPU goldens consume this — having two copies
/// is `LEAK:algorithmic-duplication` per `impl-hygiene.md`.
pub(in crate::gpu::visual_regression) fn frame_input(
    session: &PtySession,
    cell: CellMetrics,
    subpixel_positioning: bool,
) -> FrameInput {
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
        subpixel_positioning,
        prompt_marker_rows: Vec::new(),
    }
}
