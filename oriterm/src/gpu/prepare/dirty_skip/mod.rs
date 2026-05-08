//! Row-level dirty skip for incremental prepare.
//!
//! When only a few rows changed since the last frame, the prepare phase
//! copies cached instances for clean rows and only regenerates dirty rows.
//! This avoids the per-cell shaping, atlas lookup, and decoration work
//! for unchanged rows — the dominant cost in the prepare phase.

mod selection_damage;

use std::ops::Range;

use log::trace;
use oriterm_core::{CellFlags, RenderableCell};

use super::emit::{draw_prompt_markers, draw_url_hover_underline};
use super::emit_cell::EmitCtx;
use super::shaped_frame::ShapedFrame;
use super::{AtlasLookup, FrameInput};
use crate::gpu::prepared_frame::PreparedFrame;

pub use selection_damage::build_dirty_set;
#[cfg(test)]
pub(crate) use selection_damage::mark_selection_damage;

/// Per-row byte ranges in the terminal-tier instance buffers.
///
/// Tracks where each visible row's instances begin and end in the
/// `backgrounds`, `glyphs`, `subpixel_glyphs`, and `color_glyphs`
/// buffers. Built during prepare, consumed during incremental updates.
#[derive(Debug, Clone, Default)]
pub struct RowInstanceRanges {
    /// Byte range `[start..end)` in `backgrounds`.
    pub backgrounds: Range<usize>,
    /// Byte range `[start..end)` in `glyphs`.
    pub glyphs: Range<usize>,
    /// Byte range `[start..end)` in `subpixel_glyphs`.
    pub subpixel_glyphs: Range<usize>,
    /// Byte range `[start..end)` in `color_glyphs`.
    pub color_glyphs: Range<usize>,
}

/// Saved terminal-tier buffer data from the previous frame.
///
/// Swapped out of `PreparedFrame` at the start of an incremental update.
/// Clean rows' instances are copied from these buffers using the saved
/// [`RowInstanceRanges`]; dirty rows are regenerated fresh.
pub struct SavedTerminalTier {
    /// Previous frame's background instance bytes.
    pub(crate) backgrounds: Vec<u8>,
    /// Previous frame's mono glyph instance bytes.
    pub(crate) glyphs: Vec<u8>,
    /// Previous frame's subpixel glyph instance bytes.
    pub(crate) subpixel_glyphs: Vec<u8>,
    /// Previous frame's color glyph instance bytes.
    pub(crate) color_glyphs: Vec<u8>,
    /// Per-row instance ranges into the above buffers.
    pub(crate) row_ranges: Vec<RowInstanceRanges>,
}

impl SavedTerminalTier {
    /// Create empty saved tier (no cached data).
    pub fn new() -> Self {
        Self {
            backgrounds: Vec::new(),
            glyphs: Vec::new(),
            subpixel_glyphs: Vec::new(),
            color_glyphs: Vec::new(),
            row_ranges: Vec::new(),
        }
    }

    /// Whether this saved tier has valid cached data for incremental updates.
    pub fn has_cached_rows(&self) -> bool {
        !self.row_ranges.is_empty()
    }

    /// Get the cached row ranges for a given row, if available.
    pub fn row_ranges(&self, row: usize) -> Option<&RowInstanceRanges> {
        self.row_ranges.get(row)
    }
}

/// Snapshot of current buffer byte lengths, used to compute row ranges.
#[derive(Debug, Clone, Copy)]
pub struct BufferLengths {
    backgrounds: usize,
    glyphs: usize,
    subpixel_glyphs: usize,
    color_glyphs: usize,
}

impl BufferLengths {
    /// Capture current byte lengths from a frame's terminal-tier writers.
    pub fn capture(frame: &PreparedFrame) -> Self {
        Self {
            backgrounds: frame.backgrounds.byte_len(),
            glyphs: frame.glyphs.byte_len(),
            subpixel_glyphs: frame.subpixel_glyphs.byte_len(),
            color_glyphs: frame.color_glyphs.byte_len(),
        }
    }

    /// Compute the row range from a before-snapshot to a current snapshot.
    pub fn range_since(&self, before: &Self) -> RowInstanceRanges {
        RowInstanceRanges {
            backgrounds: before.backgrounds..self.backgrounds,
            glyphs: before.glyphs..self.glyphs,
            subpixel_glyphs: before.subpixel_glyphs..self.subpixel_glyphs,
            color_glyphs: before.color_glyphs..self.color_glyphs,
        }
    }
}

/// Row-tracking state returned from the cell processing loop.
struct RowLoopResult {
    current_row: usize,
    row_start: BufferLengths,
    row_is_clean: bool,
}

/// Record a dirty row's instance range into `frame.row_ranges`.
fn push_dirty_row_range(frame: &mut PreparedFrame, current_row: usize, row_start: &BufferLengths) {
    let now = BufferLengths::capture(frame);
    let ranges = now.range_since(row_start);
    while frame.row_ranges.len() < current_row {
        frame.row_ranges.push(RowInstanceRanges::default());
    }
    frame.row_ranges.push(ranges);
}

/// Copy saved-tier instances for a clean row and record its range.
///
/// Borrows `frame.saved_tier` (shared) and terminal-tier writers (mutable) as
/// disjoint field projections — see the borrow note in `fill_frame_incremental`.
fn replay_clean_row(frame: &mut PreparedFrame, row: usize) {
    let row_start = BufferLengths::capture(frame);
    // Copy out byte-range starts/ends as scalars before the disjoint-field
    // borrow split below — avoids per-clean-row `RowInstanceRanges.cloned()`
    // (32-byte struct copy in the hot path) by capturing only the 8 usize
    // values we actually need.
    let saved_ranges = frame.saved_tier.row_ranges(row).map(|r| {
        (
            r.backgrounds.start,
            r.backgrounds.end,
            r.glyphs.start,
            r.glyphs.end,
            r.subpixel_glyphs.start,
            r.subpixel_glyphs.end,
            r.color_glyphs.start,
            r.color_glyphs.end,
        )
    });
    if let Some((bg_s, bg_e, g_s, g_e, sp_s, sp_e, c_s, c_e)) = saved_ranges {
        let saved = &frame.saved_tier;
        frame
            .backgrounds
            .extend_from_byte_range(&saved.backgrounds, bg_s, bg_e);
        frame.glyphs.extend_from_byte_range(&saved.glyphs, g_s, g_e);
        frame
            .subpixel_glyphs
            .extend_from_byte_range(&saved.subpixel_glyphs, sp_s, sp_e);
        frame
            .color_glyphs
            .extend_from_byte_range(&saved.color_glyphs, c_s, c_e);
    }
    let now = BufferLengths::capture(frame);
    let ranges = now.range_since(&row_start);
    while frame.row_ranges.len() < row {
        frame.row_ranges.push(RowInstanceRanges::default());
    }
    frame.row_ranges.push(ranges);
}

/// Core cell-iteration loop for the incremental path.
///
/// Iterates `cells`, maintaining row-transition state, skipping clean rows by
/// replaying their saved-tier instances, and calling [`emit_cell`] for each
/// dirty cell. Returns the final row-tracking state for post-loop range recording.
fn process_incremental_cells(
    ctx: &mut EmitCtx<'_>,
    cells: &[RenderableCell],
    ox: f32,
    oy: f32,
) -> RowLoopResult {
    let cw = ctx.cell_size.width;
    let ch = ctx.cell_size.height;
    let viewport_h = ctx.frame.viewport.height as f32;

    let mut current_row = usize::MAX;
    let mut row_start = BufferLengths::capture(ctx.frame);
    let mut row_is_clean = false;
    let mut row_off_screen = false;

    let mut clean_replay_rows = 0usize;
    let mut dirty_emitted_rows = 0usize;
    let mut emitted_cells = 0usize;

    let mut i = 0;
    while i < cells.len() {
        let cell = &cells[i];
        let row = cell.line;

        if row != current_row {
            if current_row != usize::MAX && !row_is_clean {
                push_dirty_row_range(ctx.frame, current_row, &row_start);
            }
            current_row = row;
            let is_dirty = ctx.frame.scratch_dirty.get(row).copied().unwrap_or(true);
            if !is_dirty {
                replay_clean_row(ctx.frame, row);
                clean_replay_rows += 1;
                row_is_clean = true;
                while i < cells.len() && cells[i].line == row {
                    i += 1;
                }
                continue;
            }
            dirty_emitted_rows += 1;
            row_start = BufferLengths::capture(ctx.frame);
            row_is_clean = false;
            let row_y = super::snapped_row_y(oy, row, ch);
            row_off_screen = row_y + ch < 0.0 || row_y > viewport_h;
        }

        i += 1;

        if row_off_screen {
            continue;
        }
        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let col = cell.column.0;
        let x = ox + col as f32 * cw;
        // Round Y to integer pixels (see prepare/mod.rs for rationale).
        let y = super::snapped_row_y(oy, row, ch);
        super::emit_cell::emit_cell(cell, x, y, ctx);
        emitted_cells += 1;
    }

    trace!(
        "process_incremental_cells clean_rows={clean_replay_rows} dirty_rows={dirty_emitted_rows} emitted_cells={emitted_cells}"
    );

    RowLoopResult {
        current_row,
        row_start,
        row_is_clean,
    }
}

/// Incremental prepare: skip clean rows, copy cached instances, regenerate dirty.
///
/// Iterates cells row-by-row. For each row:
/// - **Clean**: copies the previous frame's instances from `saved_tier`.
/// - **Dirty**: processes cells normally (color resolution, shaping, decoration).
///
/// Cursor, URL hover, prompt markers, and images are always regenerated since
/// they depend on frame-level state (blink, hover, search) not just cell content.
#[expect(
    clippy::too_many_arguments,
    reason = "origin + cursor blink are pipeline context passed from renderer"
)]
pub(crate) fn fill_frame_incremental(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    shaped: &ShapedFrame,
    frame: &mut PreparedFrame,
    origin: (f32, f32),
    cursor_opacity: f32,
) {
    let (ox, oy) = origin;

    // Setup that must read frame fields before frame is moved into ctx.
    let num_rows = input.rows();
    // Snapshot prev-frame row-state inputs into a Copy struct so the
    // immutable borrow of `frame` ends before `&mut frame.scratch_dirty`.
    let prev_state = selection_damage::PrevFrameState::from_frame(frame);
    // Resolve cursor BEFORE build_dirty_set so the dirty-set tracks the
    // actually-rendered cursor row (which may be the mark-mode cursor)
    // rather than the raw terminal cursor — otherwise the wrong row's
    // per-cell colors get the cursor-cell exclusion treatment.
    let resolved_cursor = super::resolve_cursor_state(input);
    build_dirty_set(
        input,
        num_rows,
        &resolved_cursor,
        prev_state,
        &mut frame.scratch_dirty,
    );

    let mut ctx = EmitCtx {
        fg_dim: input.fg_dim,
        text_blink_opacity: input.text_blink_opacity,
        subpixel_positioning: input.subpixel_positioning,
        palette: &input.palette,
        sel: input.selection.as_ref(),
        search: input.search.as_ref(),
        cursor: resolved_cursor,
        cursor_opacity,
        hovered_cell: input.hovered_cell,
        cell_size: &input.cell_size,
        atlas,
        size_q6: shaped.size_q6(),
        frame,
        shaped: Some((shaped, shaped.hinted())),
    };

    let RowLoopResult {
        current_row,
        row_start,
        row_is_clean,
    } = process_incremental_cells(&mut ctx, &input.content.cells, ox, oy);

    if current_row != usize::MAX && !row_is_clean {
        push_dirty_row_range(ctx.frame, current_row, &row_start);
    }

    draw_url_hover_underline(input, ctx.frame, ox, oy);
    draw_prompt_markers(input, ctx.frame, ox, oy);

    super::emit::emit_cursor_for_frame(input, ctx.frame, origin, cursor_opacity);

    super::emit::emit_image_quads(input, ctx.frame, ox, oy);
}

#[cfg(test)]
mod tests;
