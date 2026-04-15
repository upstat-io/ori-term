//! Terminal grid: 2D cell storage with cursor, scrollback, and dirty tracking.
//!
//! The `Grid` is the central data structure for terminal emulation. It stores
//! visible rows, manages cursor state, and tracks tab stops. Scrollback,
//! dirty tracking, and editing operations are added in submodules.

pub mod cursor;
pub mod dirty;
pub mod editing;
pub mod navigation;
pub mod resize;
pub mod ring;
pub mod row;
pub mod scroll;
pub mod stable_index;

use std::ops::{Index, IndexMut, Range};
use std::sync::Arc;

use vte::ansi::Color;

use crate::cell::{CellExtra, CellFlags};
use crate::index::Line;

pub use cursor::{Cursor, CursorShape};
pub use dirty::{DirtyIter, DirtyLine, DirtyTracker};
pub use editing::{DisplayEraseMode, LineEraseMode};
pub use navigation::TabClearMode;
pub use resize::ReflowMapping;
pub use ring::ScrollbackBuffer;
pub use row::Row;
pub use stable_index::StableRowIndex;

/// The 2D terminal cell grid.
///
/// Stores visible rows indexed `0..lines` (top to bottom), a cursor,
/// tab stops, scrollback history, and dirty tracking for damage-based
/// rendering.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Visible rows (index 0 = top of screen).
    rows: Vec<Row>,
    /// Number of columns.
    cols: usize,
    /// Number of visible lines.
    lines: usize,
    /// Current cursor position and template.
    cursor: Cursor,
    /// DECSC/DECRC saved cursor.
    ///
    /// Per DEC STD 070 §5.6.1 and cross-verified against wezterm, alacritty,
    /// and ghostty, the DECSC save set is the cursor position, character
    /// attributes, charset state, wrap flag, and DECOM flag. DECLRMM margins
    /// are NOT saved — margin state is scoped to the screen (alt vs primary),
    /// not to the cursor save/restore pair.
    saved_cursor: Option<Cursor>,
    /// Tab stop at each column (true = stop).
    tab_stops: Vec<bool>,
    /// DECSTBM scroll region: top (inclusive) .. bottom (exclusive).
    scroll_region: Range<usize>,
    /// Scrollback history (rows that scrolled off the top).
    scrollback: ScrollbackBuffer,
    /// How many lines scrolled back into history (0 = live view).
    display_offset: usize,
    /// Rows evicted from scrollback (for `StableRowIndex` stability).
    total_evicted: usize,
    /// Reflow overflow: scrollback rows created by column reflow that are
    /// stale copies of visible content (wrapping pushed them). Consumed by
    /// `scroll_up` and `erase_display(All)` to remove them before the
    /// shell redraws after SIGWINCH. Not incremented by height changes.
    resize_pushed: usize,
    /// DECLRMM left margin column (inclusive, 0-based). Default: 0.
    left_margin: usize,
    /// DECLRMM right margin column (inclusive, 0-based). Default: cols - 1.
    right_margin: usize,
    /// Tracks which rows have changed since last drain.
    dirty: DirtyTracker,
    /// XTPUSHSGR/XTPOPSGR attribute stack (max 10 entries per xterm).
    sgr_stack: Vec<SgrSnapshot>,
}

/// Saved SGR state for XTPUSHSGR/XTPOPSGR.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SgrSnapshot {
    flags: CellFlags,
    fg: Color,
    bg: Color,
    extra: Option<Arc<CellExtra>>,
}

impl Grid {
    /// Create a new grid with the given dimensions and default scrollback.
    ///
    /// Initializes all rows as empty, cursor at (0, 0), and tab stops
    /// every 8 columns.
    pub fn new(lines: usize, cols: usize) -> Self {
        Self::with_scrollback(lines, cols, ring::DEFAULT_MAX_SCROLLBACK)
    }

    /// Create a new grid with explicit scrollback capacity.
    pub fn with_scrollback(lines: usize, cols: usize, max_scrollback: usize) -> Self {
        debug_assert!(
            lines >= 1 && cols >= 1,
            "Grid dimensions must be >= 1 (got {lines}x{cols})"
        );
        let rows = (0..lines).map(|_| Row::new(cols)).collect();
        let tab_stops = Self::init_tab_stops(cols);

        Self {
            rows,
            cols,
            lines,
            cursor: Cursor::new(),
            saved_cursor: None,
            tab_stops,
            scroll_region: 0..lines,
            scrollback: ScrollbackBuffer::new(max_scrollback),
            display_offset: 0,
            total_evicted: 0,
            resize_pushed: 0,
            left_margin: 0,
            right_margin: cols.saturating_sub(1),
            dirty: DirtyTracker::new(lines, cols),
            sgr_stack: Vec::new(),
        }
    }

    /// Number of visible lines.
    pub fn lines(&self) -> usize {
        self.lines
    }

    /// Number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Immutable reference to the cursor.
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Mutable reference to the cursor.
    pub fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    /// Immutable reference to tab stops.
    #[cfg(test)]
    pub(crate) fn tab_stops(&self) -> &[bool] {
        &self.tab_stops
    }

    /// Left/right margin bounds (inclusive, 0-based).
    pub fn left_right_margins(&self) -> (usize, usize) {
        (self.left_margin, self.right_margin)
    }

    /// Set DECLRMM left/right margins.
    ///
    /// `left` and `right` are 0-based inclusive columns. Silently ignored
    /// if `left >= right` or `right >= cols`.
    pub fn set_left_right_margins(&mut self, left: usize, right: usize) {
        if left < right && right < self.cols {
            self.left_margin = left;
            self.right_margin = right;
        }
    }

    /// Reset left/right margins to full width.
    pub fn reset_left_right_margins(&mut self) {
        self.left_margin = 0;
        self.right_margin = self.cols.saturating_sub(1);
    }

    /// Whether horizontal margins are active (not full-width).
    pub fn has_horizontal_margins(&self) -> bool {
        self.left_margin > 0 || self.right_margin < self.cols.saturating_sub(1)
    }

    /// Whether the cursor is inside the left/right margin band.
    ///
    /// Wrap-pending state (col == `right_margin` + 1) counts as "in band"
    /// so that auto-wrap targets `left_margin`, not column 0.
    pub fn cursor_in_margin_band(&self) -> bool {
        let col = self.cursor.col().0;
        col >= self.left_margin && col <= self.right_margin + 1
    }

    /// Total lines: visible + scrollback history.
    pub fn total_lines(&self) -> usize {
        self.lines + self.scrollback.len()
    }

    /// How many lines scrolled back into history (0 = live view).
    pub fn display_offset(&self) -> usize {
        self.display_offset
    }

    /// Rows evicted from scrollback history.
    ///
    /// Used by `StableRowIndex` to produce row identities that survive
    /// scrollback eviction.
    pub fn total_evicted(&self) -> usize {
        self.total_evicted
    }

    /// Immutable reference to the scrollback buffer.
    pub fn scrollback(&self) -> &ScrollbackBuffer {
        &self.scrollback
    }

    /// Access a row by absolute index.
    ///
    /// Absolute index 0 is the oldest scrollback row, with visible rows
    /// following at `scrollback.len()..scrollback.len() + lines`.
    pub fn absolute_row(&self, abs_row: usize) -> Option<&Row> {
        let sb_len = self.scrollback.len();
        if abs_row < sb_len {
            // Scrollback: logical 0 = newest, but absolute 0 = oldest.
            self.scrollback.get(sb_len - 1 - abs_row)
        } else {
            let vis = abs_row - sb_len;
            self.rows.get(vis)
        }
    }

    /// The scroll region as a half-open range (top inclusive, bottom exclusive).
    pub fn scroll_region(&self) -> &Range<usize> {
        &self.scroll_region
    }

    /// Immutable reference to the dirty tracker.
    pub fn dirty(&self) -> &DirtyTracker {
        &self.dirty
    }

    /// Mutable reference to the dirty tracker.
    pub fn dirty_mut(&mut self) -> &mut DirtyTracker {
        &mut self.dirty
    }

    /// Adjust display offset (positive = scroll back, negative = scroll forward).
    ///
    /// Clamped to `0..=scrollback.len()`.
    pub fn scroll_display(&mut self, delta: isize) {
        let max = self.scrollback.len();
        let current = self.display_offset as isize;
        let target = (current + delta).clamp(0, max as isize) as usize;

        if target != self.display_offset {
            self.display_offset = target;
            self.dirty.mark_all();
        }
    }

    /// Reset the grid to initial state.
    ///
    /// Clears all rows, resets cursor to (0,0) with default template,
    /// clears saved cursor, resets tab stops and scroll region, clears
    /// scrollback history, and marks everything dirty. Does not affect
    /// scrollback capacity.
    pub fn reset(&mut self) {
        for row in &mut self.rows {
            row.reset(self.cols, &crate::cell::Cell::default());
        }
        self.cursor = Cursor::new();
        self.saved_cursor = None;
        Self::reset_tab_stops(&mut self.tab_stops, self.cols);
        self.scroll_region = 0..self.lines;
        self.left_margin = 0;
        self.right_margin = self.cols.saturating_sub(1);
        self.total_evicted += self.scrollback.len();
        self.scrollback.clear();
        self.display_offset = 0;
        self.sgr_stack.clear();
        self.dirty.mark_all();
    }

    /// Push current cursor template SGR state onto the XTPUSHSGR stack.
    pub fn push_sgr(&mut self) {
        const MAX_SGR_STACK: usize = 10;
        if self.sgr_stack.len() >= MAX_SGR_STACK {
            return;
        }
        let t = &self.cursor.template;
        self.sgr_stack.push(SgrSnapshot {
            flags: t.flags,
            fg: t.fg,
            bg: t.bg,
            extra: t.extra.clone(),
        });
    }

    /// Pop SGR state from the XTPUSHSGR stack and apply to cursor template.
    pub fn pop_sgr(&mut self) {
        if let Some(snap) = self.sgr_stack.pop() {
            let t = &mut self.cursor.template;
            t.flags = snap.flags;
            t.fg = snap.fg;
            t.bg = snap.bg;
            t.extra = snap.extra;
        }
    }

    /// Initialize tab stops every 8 columns.
    fn init_tab_stops(cols: usize) -> Vec<bool> {
        (0..cols).map(|c| c % 8 == 0).collect()
    }

    /// Reset tab stops in-place every 8 columns, reusing the existing allocation.
    fn reset_tab_stops(tab_stops: &mut Vec<bool>, cols: usize) {
        tab_stops.resize(cols, false);
        for (i, stop) in tab_stops.iter_mut().enumerate() {
            *stop = i % 8 == 0;
        }
    }

    /// Mark the current cursor line dirty and move to `new_line`.
    ///
    /// Marks both old and new lines dirty so a damage-aware renderer
    /// redraws the cursor in both its old and new positions.
    pub(crate) fn move_cursor_line(&mut self, new_line: usize) {
        self.dirty.mark(self.cursor.line());
        self.cursor.set_line(new_line);
        self.dirty.mark(self.cursor.line());
    }

    /// Mark the current cursor line dirty and move to `new_col`.
    ///
    /// The cursor stays on the same line, so only the current line
    /// needs to be marked dirty (the cursor's old and new positions
    /// are both on this line).
    pub(crate) fn move_cursor_col(&mut self, new_col: crate::index::Column) {
        self.dirty.mark(self.cursor.line());
        self.cursor.set_col(new_col);
    }
}

impl Index<Line> for Grid {
    type Output = Row;

    fn index(&self, line: Line) -> &Row {
        debug_assert!(line.0 >= 0, "negative Line index on Grid (got {})", line.0);
        &self.rows[line.0 as usize]
    }
}

impl IndexMut<Line> for Grid {
    fn index_mut(&mut self, line: Line) -> &mut Row {
        debug_assert!(line.0 >= 0, "negative Line index on Grid (got {})", line.0);
        &mut self.rows[line.0 as usize]
    }
}

pub mod snapshot;

#[cfg(test)]
mod tests;
