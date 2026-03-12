//! Row-level dirty skip for incremental prepare.
//!
//! When only a few rows changed since the last frame, the prepare phase
//! copies cached instances for clean rows and only regenerates dirty rows.
//! This avoids the per-cell shaping, atlas lookup, and decoration work
//! for unchanged rows — the dominant cost in the prepare phase.

use std::ops::Range;

use oriterm_core::{CellFlags, CursorShape};

use super::decorations::DecorationContext;
use super::emit::{GlyphEmitter, build_cursor, draw_prompt_markers, draw_url_hover_underline};
use super::shaped_frame::ShapedFrame;
use super::{AtlasLookup, FrameInput, resolve_cell_colors, resolve_cursor};
use crate::gpu::instance_writer::ScreenRect;
use crate::gpu::prepared_frame::PreparedFrame;

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
    pub backgrounds: Vec<u8>,
    /// Previous frame's mono glyph instance bytes.
    pub glyphs: Vec<u8>,
    /// Previous frame's subpixel glyph instance bytes.
    pub subpixel_glyphs: Vec<u8>,
    /// Previous frame's color glyph instance bytes.
    pub color_glyphs: Vec<u8>,
    /// Per-row instance ranges into the above buffers.
    pub row_ranges: Vec<RowInstanceRanges>,
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
    pub backgrounds: usize,
    pub glyphs: usize,
    pub subpixel_glyphs: usize,
    pub color_glyphs: usize,
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

/// Build a fast dirty-row lookup from `RenderableContent` damage info.
///
/// Returns a `Vec<bool>` indexed by viewport line, where `true` means
/// the row needs regeneration. When `all_dirty` is set, all rows are dirty.
///
/// `prev_selection` is the selection line range from the previous frame.
/// When the selection changes between frames, the affected rows are marked
/// dirty so their instances are regenerated with correct selection colors.
pub fn build_dirty_set(
    input: &FrameInput,
    num_rows: usize,
    prev_selection: Option<(usize, usize)>,
) -> Vec<bool> {
    if input.content.all_dirty {
        return vec![true; num_rows];
    }

    let mut dirty = vec![false; num_rows];
    for d in &input.content.damage {
        if d.line < num_rows {
            dirty[d.line] = true;
        }
    }

    // The cursor row is always dirty (cursor blink state may have changed).
    if input.content.cursor.visible && input.content.cursor.line < num_rows {
        dirty[input.content.cursor.line] = true;
    }

    // Selection damage: mark rows that changed selection state.
    let new_selection = input
        .selection
        .as_ref()
        .and_then(|s| s.viewport_line_range(num_rows));
    mark_selection_damage(&mut dirty, prev_selection, new_selection);

    dirty
}

/// Mark rows dirty that changed selection state between frames.
///
/// Computes the symmetric difference of old and new selection line ranges.
/// A row needs instance regeneration if it was selected before but not now,
/// or is selected now but wasn't before. Boundary lines (first/last of each
/// range) are always marked dirty because their column extent may differ even
/// when the line range overlaps.
pub(crate) fn mark_selection_damage(
    dirty: &mut [bool],
    old: Option<(usize, usize)>,
    new: Option<(usize, usize)>,
) {
    if old == new {
        return;
    }
    let num_rows = dirty.len();
    if num_rows == 0 {
        return;
    }
    let max_line = num_rows - 1;

    match (old, new) {
        (None, None) => {}
        (Some((s, e)), None) => {
            // Selection cleared: damage all previously-selected lines.
            for d in &mut dirty[s..=e.min(max_line)] {
                *d = true;
            }
        }
        (None, Some((s, e))) => {
            // New selection: damage all newly-selected lines.
            for d in &mut dirty[s..=e.min(max_line)] {
                *d = true;
            }
        }
        (Some((os, oe)), Some((ns, ne))) => {
            // Selection changed. Mark symmetric difference lines dirty.
            let min_s = os.min(ns);
            let max_e = oe.max(ne).min(max_line);
            for (line, d) in dirty.iter_mut().enumerate().take(max_e + 1).skip(min_s) {
                let was = line >= os && line <= oe;
                let is = line >= ns && line <= ne;
                if was != is {
                    *d = true;
                }
            }
            // Boundary lines always dirty — column extent may differ
            // even when the line is in both old and new ranges.
            if os <= max_line {
                dirty[os] = true;
            }
            if oe <= max_line {
                dirty[oe] = true;
            }
            if ns <= max_line {
                dirty[ns] = true;
            }
            if ne <= max_line {
                dirty[ne] = true;
            }
        }
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
#[expect(
    clippy::too_many_lines,
    reason = "mirrors fill_frame_shaped structure with dirty-skip branching"
)]
pub(crate) fn fill_frame_incremental(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    shaped: &ShapedFrame,
    frame: &mut PreparedFrame,
    origin: (f32, f32),
    cursor_blink_visible: bool,
) {
    let cw = input.cell_size.width;
    let ch = input.cell_size.height;
    let baseline = input.cell_size.baseline;
    let fg_dim = input.fg_dim;
    let (ox, oy) = origin;
    let sel = input.selection.as_ref();
    let search = input.search.as_ref();
    let cursor = resolve_cursor(&input.content.cursor, input.mark_cursor.as_ref());

    let viewport_h = input.viewport.height as f32;
    let num_rows = input.rows();
    let prev_sel = frame.prev_selection_range;
    let dirty_rows = build_dirty_set(input, num_rows, prev_sel);

    // Track row boundaries for row_ranges.
    let mut current_row = usize::MAX;
    let mut row_start = BufferLengths::capture(frame);
    let mut row_is_clean = false;
    let mut row_off_screen = false;

    let cells = &input.content.cells;
    let mut i = 0;
    while i < cells.len() {
        let cell = &cells[i];

        let row = cell.line;

        // Row transition: record previous row and decide skip/process.
        if row != current_row {
            // Record the previous row's range.
            if current_row != usize::MAX && !row_is_clean {
                let now = BufferLengths::capture(frame);
                let ranges = now.range_since(&row_start);
                while frame.row_ranges.len() < current_row {
                    frame.row_ranges.push(RowInstanceRanges::default());
                }
                frame.row_ranges.push(ranges);
            }

            current_row = row;
            let is_dirty = dirty_rows.get(row).copied().unwrap_or(true);

            if !is_dirty {
                // Clean row: copy cached instances and skip all cells.
                // Borrow saved_tier fields and frame writers separately to
                // satisfy the borrow checker (can't pass &self + &mut frame).
                row_start = BufferLengths::capture(frame);
                if let Some(ranges) = frame.saved_tier.row_ranges(row).cloned() {
                    let saved = &frame.saved_tier;
                    frame.backgrounds.extend_from_byte_range(
                        &saved.backgrounds,
                        ranges.backgrounds.start,
                        ranges.backgrounds.end,
                    );
                    frame.glyphs.extend_from_byte_range(
                        &saved.glyphs,
                        ranges.glyphs.start,
                        ranges.glyphs.end,
                    );
                    frame.subpixel_glyphs.extend_from_byte_range(
                        &saved.subpixel_glyphs,
                        ranges.subpixel_glyphs.start,
                        ranges.subpixel_glyphs.end,
                    );
                    frame.color_glyphs.extend_from_byte_range(
                        &saved.color_glyphs,
                        ranges.color_glyphs.start,
                        ranges.color_glyphs.end,
                    );
                }
                let now = BufferLengths::capture(frame);
                let ranges = now.range_since(&row_start);
                while frame.row_ranges.len() < row {
                    frame.row_ranges.push(RowInstanceRanges::default());
                }
                frame.row_ranges.push(ranges);
                row_is_clean = true;

                // Skip all cells in this row.
                while i < cells.len() && cells[i].line == row {
                    i += 1;
                }
                continue;
            }

            row_start = BufferLengths::capture(frame);
            row_is_clean = false;

            // Skip rows entirely outside the render target.
            let row_y = oy + row as f32 * ch;
            row_off_screen = row_y + ch < 0.0 || row_y > viewport_h;
        }

        i += 1;

        if row_off_screen {
            continue;
        }

        // Skip spacer cells (handled by the base wide char cell).
        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let col = cell.column.0;
        let x = ox + col as f32 * cw;
        let y = oy + row as f32 * ch;

        let (fg, bg) = resolve_cell_colors(
            cell,
            sel,
            search,
            &cursor,
            cursor_blink_visible,
            &input.palette,
        );

        let bg_w = if cell.flags.contains(CellFlags::WIDE_CHAR) {
            2.0 * cw
        } else {
            cw
        };
        frame.backgrounds.push_rect(
            ScreenRect {
                x,
                y,
                w: bg_w,
                h: ch,
            },
            bg,
            1.0,
        );

        let is_hovered = input.hovered_cell == Some((row, col));
        DecorationContext {
            backgrounds: &mut frame.backgrounds,
            glyphs: &mut frame.glyphs,
            atlas,
            size_q6: shaped.size_q6(),
            metrics: &input.cell_size,
        }
        .draw(
            cell.flags,
            cell.underline_color,
            fg,
            x,
            y,
            bg_w,
            cell.has_hyperlink,
            is_hovered,
        );

        if crate::font::is_builtin(cell.ch) {
            let key = crate::gpu::builtin_glyphs::raster_key(cell.ch, shaped.size_q6());
            if let Some(entry) = atlas.lookup_key(key) {
                let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
                let rect = ScreenRect {
                    x,
                    y,
                    w: entry.width as f32,
                    h: entry.height as f32,
                };
                frame.glyphs.push_glyph(rect, uv, fg, fg_dim, entry.page);
            }
            continue;
        }

        if row >= shaped.rows() || col >= shaped.cols() {
            continue;
        }
        if let Some(start_idx) = shaped.col_map(row, col) {
            let row_glyphs = shaped.row_glyphs(row);
            let row_col_starts = shaped.row_col_starts(row);
            GlyphEmitter {
                baseline,
                size_q6: shaped.size_q6(),
                hinted: shaped.hinted(),
                fg_dim,
                atlas,
                frame,
            }
            .emit(row_glyphs, row_col_starts, start_idx, col, x, y, fg, bg);
        }
    }

    // Record the final row's range.
    if current_row != usize::MAX && !row_is_clean {
        let now = BufferLengths::capture(frame);
        let ranges = now.range_since(&row_start);
        while frame.row_ranges.len() < current_row {
            frame.row_ranges.push(RowInstanceRanges::default());
        }
        frame.row_ranges.push(ranges);
    }

    // URL hover, prompt markers, cursor, and images always regenerated.
    draw_url_hover_underline(input, frame, ox, oy);
    draw_prompt_markers(input, frame, ox, oy);

    if cursor.visible && cursor_blink_visible {
        let shape = if input.window_focused {
            cursor.shape
        } else {
            CursorShape::HollowBlock
        };
        build_cursor(
            frame,
            shape,
            cursor.column.0,
            cursor.line,
            cw,
            ch,
            ox,
            oy,
            input.palette.cursor_color,
        );
    }

    super::emit::emit_image_quads(input, frame, ox, oy);
}

#[cfg(test)]
mod tests;
