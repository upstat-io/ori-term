//! Private helpers for mouse selection: drag snapping, spacer redirect,
//! and auto-scroll.
//!
//! Extracted from `mod.rs` to keep file sizes under the 500-line limit.

use winit::dpi::PhysicalPosition;

use oriterm_core::grid::{Grid, StableRowIndex};
use oriterm_core::selection::{logical_line_end, logical_line_start, word_boundaries};
use oriterm_core::{CellFlags, Column, SelectionMode, SelectionPoint, Side};

use oriterm_mux::pane::Pane;

use super::{GridCtx, pixel_to_side};

/// Update the selection endpoint during drag, respecting mode-aware snapping.
pub(super) fn update_drag_endpoint(
    pane: &mut Pane,
    col: usize,
    line: usize,
    side: Side,
    word_delimiters: &str,
) {
    // Read selection state before locking the terminal.
    let (sel_mode, sel_anchor) = match pane.selection() {
        Some(s) => (Some(s.mode), Some(s.anchor)),
        None => (None, None),
    };

    let new_end = {
        let term = pane.terminal().lock();
        let g = term.grid();
        let col = col.min(g.cols().saturating_sub(1));
        let line = line.min(g.lines().saturating_sub(1));
        let abs_row = g.scrollback().len().saturating_sub(g.display_offset()) + line;
        let col = redirect_spacer(g, abs_row, col);
        let stable_row = StableRowIndex::from_absolute(g, abs_row);

        match sel_mode {
            Some(SelectionMode::Word) => {
                let (ws, we) = word_boundaries(g, abs_row, col, word_delimiters);
                let start_pt = SelectionPoint {
                    row: stable_row,
                    col: ws,
                    side: Side::Left,
                };
                let end_pt = SelectionPoint {
                    row: stable_row,
                    col: we,
                    side: Side::Right,
                };
                // Snap to word boundary in the drag direction.
                if sel_anchor.is_some_and(|a| start_pt < a) {
                    start_pt
                } else {
                    end_pt
                }
            }
            Some(SelectionMode::Line) => {
                let ls = logical_line_start(g, abs_row);
                let le = logical_line_end(g, abs_row);
                let grid_cols = g.cols();
                // Snap to line boundary in the drag direction.
                if sel_anchor.is_some_and(|a| stable_row < a.row) {
                    SelectionPoint {
                        row: StableRowIndex::from_absolute(g, ls),
                        col: 0,
                        side: Side::Left,
                    }
                } else {
                    SelectionPoint {
                        row: StableRowIndex::from_absolute(g, le),
                        col: grid_cols.saturating_sub(1),
                        side: Side::Right,
                    }
                }
            }
            Some(_) => SelectionPoint {
                row: stable_row,
                col,
                side,
            },
            None => return,
        }
    };

    pane.update_selection_end(new_end);
}

/// Redirect a column to the base cell if it lands on a wide char spacer.
///
/// Wide characters occupy two cells: the base cell and a trailing spacer.
/// Clicking on the spacer should act as if the user clicked on the base cell.
pub(crate) fn redirect_spacer(grid: &Grid, abs_row: usize, col: usize) -> usize {
    if col == 0 {
        return col;
    }
    let Some(row) = grid.absolute_row(abs_row) else {
        return col;
    };
    if col < row.cols() && row[Column(col)].flags.contains(CellFlags::WIDE_CHAR_SPACER) {
        col - 1
    } else {
        col
    }
}

/// Auto-scroll the viewport when the mouse is above or below the grid.
///
/// After scrolling, updates the selection endpoint to the visible edge row
/// at the mouse's X column so the highlight extends with the scroll.
pub(super) fn handle_auto_scroll(pane: &mut Pane, pos: PhysicalPosition<f64>, ctx: &GridCtx<'_>) {
    let Some(bounds) = ctx.widget.bounds() else {
        return;
    };
    let y = pos.y;
    let grid_top = f64::from(bounds.y());
    let ch = f64::from(ctx.cell.height);
    if ch <= 0.0 {
        return;
    }

    let side = pixel_to_side(pos, ctx);
    let scrolling_up = y < grid_top;

    // Determine scroll direction; bail if mouse is inside the grid or
    // already at the bottom of history.
    if scrolling_up {
        pane.scroll_display(1);
    } else {
        let (lines, offset) = {
            let term = pane.terminal().lock();
            let g = term.grid();
            (g.lines(), g.display_offset())
        };
        let grid_bottom = grid_top + lines as f64 * ch;
        if y < grid_bottom || offset == 0 {
            return;
        }
        pane.scroll_display(-1);
    }

    // After scrolling, compute endpoint for the visible edge row.
    let cw = f64::from(ctx.cell.width);
    let endpoint = {
        let term = pane.terminal().lock();
        let g = term.grid();
        let edge_line = if scrolling_up {
            0
        } else {
            g.lines().saturating_sub(1)
        };
        let abs = g.scrollback().len().saturating_sub(g.display_offset()) + edge_line;
        let col = if cw > 0.0 {
            ((pos.x - f64::from(bounds.x())) / cw) as usize
        } else {
            0
        };
        let col = col.min(g.cols().saturating_sub(1));
        let col = redirect_spacer(g, abs, col);
        let stable = StableRowIndex::from_absolute(g, abs);
        SelectionPoint {
            row: stable,
            col,
            side,
        }
    };

    pane.update_selection_end(endpoint);
}
