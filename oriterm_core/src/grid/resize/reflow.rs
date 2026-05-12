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

/// Reflow all rows from old column width to new column width.
///
/// Returns (reflowed rows, new cursor abs, new cursor col, new history
/// boundary, `first_output_row`). `history_boundary` is the source row
/// index where real scrollback history ends. `first_output_row[i]` is
/// the output row index where source row `i`'s first cell landed —
/// always populated with one entry per source row for use by
/// `ReflowMapping`.
#[expect(
    clippy::too_many_arguments,
    reason = "reflow state: source rows, dimensions, cursor position, history boundary"
)]
pub(super) fn reflow_cells(
    all_rows: &[Row],
    old_cols: usize,
    new_cols: usize,
    cursor_abs: usize,
    cursor_col: usize,
    history_boundary: usize,
) -> (Vec<Row>, usize, usize, usize, Vec<usize>) {
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
            src_row,
            src_idx,
            content_len,
            new_cols,
            cursor_abs,
            cursor_col,
            &mut result,
            &mut out_row,
            &mut out_col,
            &mut new_cursor_abs,
            &mut new_cursor_col,
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

    (
        result,
        new_cursor_abs,
        new_cursor_col,
        new_history_boundary,
        first_output_row,
    )
}

/// Reflow cells from a single source row into the output.
#[expect(
    clippy::too_many_arguments,
    reason = "cell-by-cell reflow: source context, output state, cursor tracking"
)]
fn reflow_row_cells(
    src_row: &Row,
    src_idx: usize,
    content_len: usize,
    new_cols: usize,
    cursor_abs: usize,
    cursor_col: usize,
    result: &mut Vec<Row>,
    out_row: &mut Row,
    out_col: &mut usize,
    new_cursor_abs: &mut usize,
    new_cursor_col: &mut usize,
) {
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
