//! Unshaped (per-cell) prepare path — test-only.
//!
//! These functions use per-cell character lookups instead of shaped glyph
//! positions. Production rendering uses the shaped path in `mod.rs`.

use oriterm_core::CellFlags;

use super::super::frame_input::FrameInput;
use super::super::prepared_frame::PreparedFrame;
use super::AtlasLookup;
use super::emit::{draw_prompt_markers, draw_url_hover_underline};
use super::emit_cell::EmitCtx;
use super::resolve_cursor_state;

/// Convert a [`FrameInput`] into a GPU-ready [`PreparedFrame`] using per-cell
/// character lookups (unshaped path).
///
/// Used by tests to verify prepare logic without shaping complexity. Production
/// rendering uses [`prepare_frame_shaped`](super::prepare_frame_shaped) instead.
pub(crate) fn prepare_frame(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    origin: (f32, f32),
) -> PreparedFrame {
    let cols = input.columns();
    let rows = input.rows();
    let opacity = f64::from(input.palette.opacity);
    let mut frame = PreparedFrame::with_capacity(
        input.viewport,
        cols,
        rows,
        input.palette.background,
        opacity,
    );
    fill_frame(input, atlas, &mut frame, origin, 1.0);
    frame
}

/// Convert a [`FrameInput`] into a pre-existing [`PreparedFrame`], reusing
/// its buffer allocations (unshaped path).
///
/// Used by tests. Production rendering uses
/// [`prepare_frame_shaped_into`](super::prepare_frame_shaped_into) instead.
pub(crate) fn prepare_frame_into(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    out: &mut PreparedFrame,
    origin: (f32, f32),
) {
    out.clear();
    out.viewport = input.viewport;
    out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
    fill_frame(input, atlas, out, origin, 1.0);
}

/// Unshaped per-cell rendering: emit instances into `frame`.
///
/// Iterates every visible cell, emits background and glyph instances via
/// character lookup, then builds cursor instances. Used by tests; production
/// rendering uses the shaped path.
fn fill_frame(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    frame: &mut PreparedFrame,
    origin: (f32, f32),
    cursor_opacity: f32,
) {
    let cw = input.cell_size.width;
    let ch = input.cell_size.height;
    let (ox, oy) = origin;

    let mut ctx = EmitCtx {
        fg_dim: input.fg_dim,
        text_blink_opacity: input.text_blink_opacity,
        subpixel_positioning: input.subpixel_positioning,
        palette: &input.palette,
        sel: input.selection.as_ref(),
        search: input.search.as_ref(),
        cursor: resolve_cursor_state(input),
        cursor_opacity,
        hovered_cell: input.hovered_cell,
        cell_size: &input.cell_size,
        atlas,
        size_q6: 0,
        frame,
        shaped: None,
    };

    for cell in &input.content.cells {
        // Spacer cells are handled by their primary cell (or are padding).
        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let col = cell.column.0;
        let x = ox + col as f32 * cw;
        let y = super::snapped_row_y(oy, cell.line, ch);

        super::emit_cell::emit_cell(cell, x, y, &mut ctx);
    }

    draw_url_hover_underline(input, ctx.frame, ox, oy);
    draw_prompt_markers(input, ctx.frame, ox, oy);

    super::emit::emit_cursor_for_frame(input, ctx.frame, origin, cursor_opacity);

    super::emit::emit_image_quads(input, ctx.frame, ox, oy);
}
