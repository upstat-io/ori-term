//! Pure mark-mode cursor motion functions.
//!
//! All functions are pure: no locks, no side effects, no grid access.
//! The caller extracts grid bounds under terminal lock, calls a motion
//! function, then stores the result.

/// Grid dimensions extracted under terminal lock.
///
/// Passed to pure motion functions so they never touch the grid directly.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GridBounds {
    /// Total rows in the grid (scrollback + visible).
    pub(crate) total_rows: usize,
    /// Number of columns.
    pub(crate) cols: usize,
    /// Number of visible lines in the viewport.
    pub(crate) visible_lines: usize,
}

/// Absolute cursor position for motion arithmetic.
///
/// Converted from/to [`oriterm_mux::pane::MarkCursor`] (which uses
/// `StableRowIndex`) under terminal lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbsCursor {
    /// Absolute row (0 = oldest scrollback row).
    pub(crate) abs_row: usize,
    /// Column (0-based).
    pub(crate) col: usize,
}

/// Move cursor left by one cell, wrapping to previous row end.
pub(crate) fn move_left(c: AbsCursor, b: GridBounds) -> AbsCursor {
    if c.col > 0 {
        AbsCursor {
            col: c.col - 1,
            ..c
        }
    } else if c.abs_row > 0 {
        AbsCursor {
            abs_row: c.abs_row - 1,
            col: b.cols.saturating_sub(1),
        }
    } else {
        c
    }
}

/// Move cursor right by one cell, wrapping to next row start.
pub(crate) fn move_right(c: AbsCursor, b: GridBounds) -> AbsCursor {
    if c.col + 1 < b.cols {
        AbsCursor {
            col: c.col + 1,
            ..c
        }
    } else if c.abs_row + 1 < b.total_rows {
        AbsCursor {
            abs_row: c.abs_row + 1,
            col: 0,
        }
    } else {
        c
    }
}

/// Move cursor up by one row, clamping at the top.
pub(crate) fn move_up(c: AbsCursor) -> AbsCursor {
    AbsCursor {
        abs_row: c.abs_row.saturating_sub(1),
        ..c
    }
}

/// Move cursor down by one row, clamping at the bottom.
pub(crate) fn move_down(c: AbsCursor, b: GridBounds) -> AbsCursor {
    let last = b.total_rows.saturating_sub(1);
    if c.abs_row < last {
        AbsCursor {
            abs_row: c.abs_row + 1,
            ..c
        }
    } else {
        c
    }
}

/// Move cursor up by one page (viewport height).
pub(crate) fn page_up(c: AbsCursor, b: GridBounds) -> AbsCursor {
    AbsCursor {
        abs_row: c.abs_row.saturating_sub(b.visible_lines),
        ..c
    }
}

/// Move cursor down by one page (viewport height).
pub(crate) fn page_down(c: AbsCursor, b: GridBounds) -> AbsCursor {
    let last = b.total_rows.saturating_sub(1);
    AbsCursor {
        abs_row: (c.abs_row + b.visible_lines).min(last),
        ..c
    }
}

/// Move cursor to the start of the current line.
pub(crate) fn line_start(c: AbsCursor) -> AbsCursor {
    AbsCursor { col: 0, ..c }
}

/// Move cursor to the end of the current line.
pub(crate) fn line_end(c: AbsCursor, b: GridBounds) -> AbsCursor {
    AbsCursor {
        col: b.cols.saturating_sub(1),
        ..c
    }
}

/// Move cursor to the start of the buffer.
pub(crate) fn buffer_start() -> AbsCursor {
    AbsCursor { abs_row: 0, col: 0 }
}

/// Move cursor to the end of the buffer.
pub(crate) fn buffer_end(b: GridBounds) -> AbsCursor {
    AbsCursor {
        abs_row: b.total_rows.saturating_sub(1),
        col: b.cols.saturating_sub(1),
    }
}

/// Pre-extracted word boundary data for pure word motion functions.
///
/// Computed under terminal lock so that the motion functions themselves
/// remain pure (no grid access, no locks).
#[derive(Debug, Clone, Copy)]
pub(crate) struct WordContext {
    /// Word start column at the current cursor position.
    pub(crate) ws: usize,
    /// Word end column at the current cursor position.
    pub(crate) we: usize,
    /// Word start at `(abs_row, ws - 1)`, if `ws > 0`.
    pub(crate) prev_same_row_ws: Option<usize>,
    /// Word start at `(abs_row - 1, cols - 1)`, if `abs_row > 0`.
    pub(crate) prev_row_ws: Option<usize>,
    /// Word end at `(abs_row, we + 1)`, if `we + 1 < cols`.
    pub(crate) next_same_row_we: Option<usize>,
    /// Word end at `(abs_row + 1, 0)`, if `abs_row + 1 < total_rows`.
    pub(crate) next_row_we: Option<usize>,
}

/// Move cursor to the start of the current or previous word.
pub(crate) fn word_left(c: AbsCursor, ctx: &WordContext) -> AbsCursor {
    if c.col > ctx.ws {
        AbsCursor {
            abs_row: c.abs_row,
            col: ctx.ws,
        }
    } else if let Some(ws) = ctx.prev_same_row_ws {
        AbsCursor {
            abs_row: c.abs_row,
            col: ws,
        }
    } else if let Some(ws) = ctx.prev_row_ws {
        AbsCursor {
            abs_row: c.abs_row.saturating_sub(1),
            col: ws,
        }
    } else {
        AbsCursor { abs_row: 0, col: 0 }
    }
}

/// Move cursor to the end of the current or next word.
pub(crate) fn word_right(c: AbsCursor, ctx: &WordContext, b: GridBounds) -> AbsCursor {
    if c.col < ctx.we {
        AbsCursor {
            abs_row: c.abs_row,
            col: ctx.we,
        }
    } else if let Some(we) = ctx.next_same_row_we {
        AbsCursor {
            abs_row: c.abs_row,
            col: we,
        }
    } else if let Some(we) = ctx.next_row_we {
        AbsCursor {
            abs_row: c.abs_row + 1,
            col: we,
        }
    } else {
        AbsCursor {
            abs_row: c.abs_row,
            col: b.cols.saturating_sub(1),
        }
    }
}
