//! Cell-by-cell text reflow for column-width changes.
//!
//! Private helpers `reflow_cells` and `reflow_row_cells`, called from
//! `Grid::resize` in the sibling `mod.rs` when the column count changes.
//! `reflow_cells` walks every old cell in row-major order; `reflow_row_cells`
//! handles a single source row, emitting wrapped continuation rows as the
//! new column width demands.

use crate::cell::{Cell, CellFlags};
use crate::index::Column;

use super::super::row::Row;

/// Pre-reflow column dimensions, cursor position, and history boundary.
///
/// Bundles the scalar inputs [`reflow_cells`] needs alongside the source
/// rows: the `old_cols` / `new_cols` widths, the cursor's absolute row +
/// column, and the `history_boundary` (source row index where real
/// scrollback history ends).
#[derive(Debug, Clone, Copy)]
pub(super) struct ReflowParams {
    /// Column width before reflow.
    pub old_cols: usize,
    /// Column width after reflow.
    pub new_cols: usize,
    /// Cursor absolute row index (scrollback-relative) before reflow.
    pub cursor_abs: usize,
    /// Cursor column before reflow.
    pub cursor_col: usize,
    /// Source row index where real scrollback history ends.
    pub history_boundary: usize,
}

/// Reflow outcome: post-reflow cursor position, history boundary, and
/// the per-source-row first-landing map.
///
/// `first_output_row[i]` is the output row index where source row `i`'s
/// first cell landed — always populated with one entry per source row
/// for use by `ReflowMapping`.
#[derive(Debug)]
pub(super) struct ReflowOutcome {
    /// Reflowed rows (scrollback + visible, undistributed).
    pub rows: Vec<Row>,
    /// Cursor absolute row index after reflow.
    pub new_cursor_abs: usize,
    /// Cursor column after reflow.
    pub new_cursor_col: usize,
    /// Output row index where real scrollback history ends.
    pub new_history_boundary: usize,
    /// Per source row: output row index where its first cell landed.
    pub first_output_row: Vec<usize>,
}

/// Read-only inputs for reflowing a single source row.
#[derive(Clone, Copy)]
struct RowReflowInput<'a> {
    /// The source row being reflowed.
    src_row: &'a Row,
    /// Index of `src_row` within the full row list.
    src_idx: usize,
    /// Number of content cells to copy from `src_row`.
    content_len: usize,
    /// Column width after reflow.
    new_cols: usize,
    /// Cursor absolute row index (scrollback-relative) before reflow.
    cursor_abs: usize,
    /// Cursor column before reflow.
    cursor_col: usize,
}

/// Mutable output sinks updated while reflowing a single source row.
///
/// Each field borrows a distinct local in [`reflow_cells`], so the
/// disjoint `&mut` borrows are valid at the call site.
struct RowReflowSink<'a> {
    /// Finalized output rows accumulated so far.
    result: &'a mut Vec<Row>,
    /// The in-progress output row being filled.
    out_row: &'a mut Row,
    /// Current write column within `out_row`.
    out_col: &'a mut usize,
    /// Tracked cursor absolute row index in the output.
    new_cursor_abs: &'a mut usize,
    /// Tracked cursor column in the output.
    new_cursor_col: &'a mut usize,
}

/// Reflow all rows from old column width to new column width.
pub(super) fn reflow_cells(all_rows: &[Row], params: ReflowParams) -> ReflowOutcome {
    let ReflowParams {
        old_cols,
        new_cols,
        cursor_abs,
        cursor_col,
        history_boundary,
    } = params;
    let mut new_cursor_abs = 0usize;
    let mut new_cursor_col = 0usize;
    let mut new_history_boundary = 0usize;
    let mut history_tracked = false;
    let mut result: Vec<Row> = Vec::with_capacity(all_rows.len());
    let mut first_output_row: Vec<usize> = Vec::with_capacity(all_rows.len());
    let mut out_row = Row::new(new_cols);
    let mut out_col = 0usize;

    for (src_idx, src_row) in all_rows.iter().enumerate() {
        // Track where the history boundary maps in the output.
        if !history_tracked && src_idx >= history_boundary {
            new_history_boundary = result.len();
            history_tracked = true;
        }

        let wrapped = old_cols > 0
            && src_row.cols() >= old_cols
            && src_row[Column(old_cols - 1)]
                .flags
                .contains(CellFlags::WRAP);

        let content_len = if wrapped {
            old_cols
        } else {
            src_row.content_len()
        };

        // Record where this source row's first cell will land in the
        // output. The pending `out_row` becomes `result[result.len()]`
        // when pushed. If `out_col == new_cols`, the pending row is
        // full and the first cell of this source row will trigger an
        // immediate push, so the first cell actually lands at
        // `result.len() + 1`. Empty source rows (`content_len == 0`)
        // write no cells, so they map to `result.len()` — the index
        // where the pending out_row (still being shared) will land
        // when pushed by the end-of-row finalize.
        let first_row = if content_len == 0 || out_col < new_cols {
            result.len()
        } else {
            result.len() + 1
        };
        first_output_row.push(first_row);

        reflow_row_cells(
            RowReflowInput {
                src_row,
                src_idx,
                content_len,
                new_cols,
                cursor_abs,
                cursor_col,
            },
            RowReflowSink {
                result: &mut result,
                out_row: &mut out_row,
                out_col: &mut out_col,
                new_cursor_abs: &mut new_cursor_abs,
                new_cursor_col: &mut new_cursor_col,
            },
        );

        // Track cursor when it's past content on this source row.
        if src_idx == cursor_abs && cursor_col >= content_len {
            new_cursor_abs = result.len();
            new_cursor_col = if wrapped {
                out_col.min(new_cols.saturating_sub(1))
            } else {
                cursor_col.min(new_cols.saturating_sub(1))
            };
        }

        // End of source row: finalize if not wrapped.
        if !wrapped {
            result.push(out_row);
            out_row = Row::new(new_cols);
            out_col = 0;
        }
    }

    // If all rows are real history (boundary at or past end).
    if !history_tracked {
        new_history_boundary = result.len() + usize::from(out_col > 0);
    }

    if out_col > 0 {
        result.push(out_row);
    }

    ReflowOutcome {
        rows: result,
        new_cursor_abs,
        new_cursor_col,
        new_history_boundary,
        first_output_row,
    }
}

/// Reflow cells from a single source row into the output.
fn reflow_row_cells(input: RowReflowInput<'_>, sink: RowReflowSink<'_>) {
    let RowReflowInput {
        src_row,
        src_idx,
        content_len,
        new_cols,
        cursor_abs,
        cursor_col,
    } = input;
    let RowReflowSink {
        result,
        out_row,
        out_col,
        new_cursor_abs,
        new_cursor_col,
    } = sink;
    for src_col in 0..content_len {
        let cell = &src_row[Column(src_col)];

        // Skip spacer cells (regenerated at new positions).
        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            if src_idx == cursor_abs && src_col == cursor_col {
                *new_cursor_abs = result.len();
                *new_cursor_col = out_col.saturating_sub(1);
            }
            continue;
        }

        let is_wide = cell.flags.contains(CellFlags::WIDE_CHAR) && new_cols >= 2;
        let cell_width = if is_wide { 2 } else { 1 };

        // Wrap to next output row if cell doesn't fit.
        if *out_col + cell_width > new_cols {
            if *out_col > 0 {
                let boundary = &mut out_row[Column(new_cols - 1)];
                boundary.flags.insert(CellFlags::WRAP);
                // Wide char at boundary with a gap cell: the cell at
                // new_cols - 1 is padding, not content. Mark it so
                // reflow/selection/search skips it. Carries DRAWN
                // because it structurally participates in the wide
                // char's on-screen presence ().
                if is_wide && *out_col < new_cols {
                    boundary.ch = ' ';
                    boundary
                        .flags
                        .insert(CellFlags::LEADING_WIDE_CHAR_SPACER | CellFlags::DRAWN);
                }
            }
            out_row.set_occ(new_cols);
            result.push(std::mem::replace(out_row, Row::new(new_cols)));
            *out_col = 0;
        }

        // Track cursor position.
        if src_idx == cursor_abs && src_col == cursor_col {
            *new_cursor_abs = result.len();
            *new_cursor_col = *out_col;
        }

        // Write cell (strip old WRAP and LEADING_WIDE_CHAR_SPACER flags).
        let mut new_cell = cell.clone();
        new_cell
            .flags
            .remove(CellFlags::WRAP | CellFlags::LEADING_WIDE_CHAR_SPACER);
        if !is_wide && cell.flags.contains(CellFlags::WIDE_CHAR) {
            new_cell.flags.remove(CellFlags::WIDE_CHAR);
        }
        out_row[Column(*out_col)] = new_cell;
        *out_col += 1;

        // Write wide char spacer in next column. Carries DRAWN because
        // it structurally participates in the wide char's on-screen
        // presence ().
        if is_wide {
            let mut spacer = Cell::default();
            spacer
                .flags
                .insert(CellFlags::WIDE_CHAR_SPACER | CellFlags::DRAWN);
            spacer.fg = cell.fg;
            spacer.bg = cell.bg;
            out_row[Column(*out_col)] = spacer;
            *out_col += 1;
        }
        out_row.set_occ(*out_col);
    }
}
