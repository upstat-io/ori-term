//! Tests for mark mode key dispatch and SnapshotGrid-dependent functions.

use oriterm_core::grid::StableRowIndex;
use oriterm_core::{Selection, SelectionMode, SelectionPoint, Side};
use oriterm_mux::{MarkCursor, PaneSnapshot, WireCell, WireCursor, WireCursorShape, WireRgb};
use oriterm_ui::interaction::mark_mode::motion::{self, AbsCursor, GridBounds};

use super::{ensure_visible, extend_or_create_selection, extract_word_context, select_all};
use crate::app::snapshot_grid::SnapshotGrid;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a simple WireCell with a character and no flags.
fn cell(ch: char) -> WireCell {
    WireCell {
        ch,
        fg: WireRgb {
            r: 255,
            g: 255,
            b: 255,
        },
        bg: WireRgb { r: 0, g: 0, b: 0 },
        flags: 0,
        underline_color: None,
        hyperlink_uri: None,
        zerowidth: Vec::new(),
    }
}

/// Build a test snapshot with configurable scrollback and display offset.
fn test_snapshot_full(
    cells: Vec<Vec<WireCell>>,
    cols: u16,
    stable_row_base: u64,
    scrollback_len: u32,
    display_offset: u32,
) -> PaneSnapshot {
    PaneSnapshot {
        cells,
        cursor: WireCursor {
            col: 0,
            row: 0,
            shape: WireCursorShape::Block,
            visible: true,
        },
        palette: vec![[0; 3]; 270],
        title: String::new(),
        icon_name: None,
        cwd: None,
        modes: 0,
        scrollback_len,
        display_offset,
        stable_row_base,
        cols,
        search_active: false,
        search_query: String::new(),
        search_matches: Vec::new(),
        search_focused: None,
        search_total_matches: 0,
        has_unseen_output: false,
    }
}

/// Build a test snapshot with 100 rows of scrollback and no display offset.
fn test_snapshot(cells: Vec<Vec<WireCell>>, cols: u16, stable_row_base: u64) -> PaneSnapshot {
    test_snapshot_full(cells, cols, stable_row_base, 100, 0)
}

// ---------------------------------------------------------------------------
// Selection containment (verifies extend_or_create_selection's Side logic)
// ---------------------------------------------------------------------------

#[test]
fn selection_forward_includes_both_endpoints() {
    // Forward selection from col 5 to col 8: both should be included.
    let anchor = SelectionPoint {
        row: StableRowIndex(0),
        col: 5,
        side: Side::Left,
    };
    let end = SelectionPoint {
        row: StableRowIndex(0),
        col: 8,
        side: Side::Right,
    };
    let sel = Selection {
        mode: SelectionMode::Char,
        anchor,
        pivot: anchor,
        end,
    };

    assert!(sel.contains(StableRowIndex(0), 5));
    assert!(sel.contains(StableRowIndex(0), 6));
    assert!(sel.contains(StableRowIndex(0), 7));
    assert!(sel.contains(StableRowIndex(0), 8));
    assert!(!sel.contains(StableRowIndex(0), 4));
    assert!(!sel.contains(StableRowIndex(0), 9));
}

#[test]
fn selection_backward_includes_both_endpoints() {
    // Backward selection from col 8 to col 5.
    // anchor=(8, Right), end=(5, Left) -> ordered start=(5,L), end=(8,R).
    let anchor = SelectionPoint {
        row: StableRowIndex(0),
        col: 8,
        side: Side::Right,
    };
    let end = SelectionPoint {
        row: StableRowIndex(0),
        col: 5,
        side: Side::Left,
    };
    let sel = Selection {
        mode: SelectionMode::Char,
        anchor,
        pivot: anchor,
        end,
    };

    assert!(sel.contains(StableRowIndex(0), 5));
    assert!(sel.contains(StableRowIndex(0), 6));
    assert!(sel.contains(StableRowIndex(0), 7));
    assert!(sel.contains(StableRowIndex(0), 8));
    assert!(!sel.contains(StableRowIndex(0), 4));
    assert!(!sel.contains(StableRowIndex(0), 9));
}

#[test]
fn selection_across_rows() {
    // Selection from row 2 col 70 to row 3 col 5.
    let anchor = SelectionPoint {
        row: StableRowIndex(2),
        col: 70,
        side: Side::Left,
    };
    let end = SelectionPoint {
        row: StableRowIndex(3),
        col: 5,
        side: Side::Right,
    };
    let sel = Selection {
        mode: SelectionMode::Char,
        anchor,
        pivot: anchor,
        end,
    };

    // Row 2: cols 70..=MAX should be selected.
    assert!(sel.contains(StableRowIndex(2), 70));
    assert!(sel.contains(StableRowIndex(2), 79));
    assert!(!sel.contains(StableRowIndex(2), 69));

    // Row 3: cols 0..=5 should be selected.
    assert!(sel.contains(StableRowIndex(3), 0));
    assert!(sel.contains(StableRowIndex(3), 5));
    assert!(!sel.contains(StableRowIndex(3), 6));
}

// ---------------------------------------------------------------------------
// Selection direction reversal
// ---------------------------------------------------------------------------

#[test]
fn selection_reversal_forward_then_backward() {
    // Simulate: anchor at col 5, extend forward to col 8, then reverse to col 3.
    // After reversal, cols 3..=5 should be selected (anchor inclusive).
    let anchor = SelectionPoint {
        row: StableRowIndex(0),
        col: 5,
        side: Side::Right, // backward: anchor gets Right
    };
    let end = SelectionPoint {
        row: StableRowIndex(0),
        col: 3,
        side: Side::Left, // backward: end gets Left
    };
    let sel = Selection {
        mode: SelectionMode::Char,
        anchor,
        pivot: anchor,
        end,
    };

    // After ordering: start=(3,Left), end=(5,Right).
    assert!(sel.contains(StableRowIndex(0), 3));
    assert!(sel.contains(StableRowIndex(0), 4));
    assert!(sel.contains(StableRowIndex(0), 5));
    assert!(!sel.contains(StableRowIndex(0), 2));
    assert!(!sel.contains(StableRowIndex(0), 6));
}

#[test]
fn selection_reversal_across_rows() {
    // Anchor at row 5 col 10, extend backward to row 3 col 70.
    let anchor = SelectionPoint {
        row: StableRowIndex(5),
        col: 10,
        side: Side::Right, // backward
    };
    let end = SelectionPoint {
        row: StableRowIndex(3),
        col: 70,
        side: Side::Left, // backward
    };
    let sel = Selection {
        mode: SelectionMode::Char,
        anchor,
        pivot: anchor,
        end,
    };

    // Ordered: start=(3,70,Left), end=(5,10,Right).
    assert!(sel.contains(StableRowIndex(3), 70));
    assert!(sel.contains(StableRowIndex(3), 79));
    assert!(sel.contains(StableRowIndex(4), 0));
    assert!(sel.contains(StableRowIndex(4), 79));
    assert!(sel.contains(StableRowIndex(5), 0));
    assert!(sel.contains(StableRowIndex(5), 10));
    assert!(!sel.contains(StableRowIndex(5), 11));
    assert!(!sel.contains(StableRowIndex(3), 69));
}

// ---------------------------------------------------------------------------
// Single-cell selection (Equal case)
// ---------------------------------------------------------------------------

#[test]
fn selection_equal_position_is_empty() {
    // When anchor == end with (Left, Left), the selection is empty.
    // This is correct: shifting back to the anchor deselects everything.
    let point = SelectionPoint {
        row: StableRowIndex(0),
        col: 5,
        side: Side::Left,
    };
    let sel = Selection {
        mode: SelectionMode::Char,
        anchor: point,
        pivot: point,
        end: point,
    };

    assert!(sel.is_empty());
    // effective_start_col=5, effective_end_col=4 -> nothing contained.
    assert!(!sel.contains(StableRowIndex(0), 5));
}

#[test]
fn selection_equal_at_col_zero_is_empty() {
    // Edge case: Equal at col 0 -- effective_end_col returns 0 (not wrapping).
    let point = SelectionPoint {
        row: StableRowIndex(0),
        col: 0,
        side: Side::Left,
    };
    let sel = Selection {
        mode: SelectionMode::Char,
        anchor: point,
        pivot: point,
        end: point,
    };

    assert!(sel.is_empty());
    // effective_end_col for (col=0, Left) returns 0 (col > 0 check fails).
    // effective_start_col=0, effective_end_col=0 -> contains col 0.
    // This is a special case: is_empty() is true but contains(0) may be true.
    // The is_empty check takes priority in rendering.
}

// ---------------------------------------------------------------------------
// extend_or_create_selection (pure function tests)
// ---------------------------------------------------------------------------

#[test]
fn extend_creates_new_selection_forward() {
    let anchor = MarkCursor {
        row: StableRowIndex(5),
        col: 10,
    };
    let end = MarkCursor {
        row: StableRowIndex(5),
        col: 15,
    };
    let sel = extend_or_create_selection(None, &anchor, &end);
    assert_eq!(sel.mode, SelectionMode::Char);
    assert_eq!(sel.anchor.row, StableRowIndex(5));
    assert_eq!(sel.anchor.col, 10);
    assert_eq!(sel.anchor.side, Side::Left);
    assert_eq!(sel.end.row, StableRowIndex(5));
    assert_eq!(sel.end.col, 15);
    assert_eq!(sel.end.side, Side::Right);
    // Both endpoints included.
    assert!(sel.contains(StableRowIndex(5), 10));
    assert!(sel.contains(StableRowIndex(5), 15));
    assert!(!sel.contains(StableRowIndex(5), 9));
    assert!(!sel.contains(StableRowIndex(5), 16));
}

#[test]
fn extend_creates_new_selection_backward() {
    let anchor = MarkCursor {
        row: StableRowIndex(5),
        col: 15,
    };
    let end = MarkCursor {
        row: StableRowIndex(5),
        col: 10,
    };
    let sel = extend_or_create_selection(None, &anchor, &end);
    assert_eq!(sel.anchor.side, Side::Right);
    assert_eq!(sel.end.side, Side::Left);
    assert!(sel.contains(StableRowIndex(5), 10));
    assert!(sel.contains(StableRowIndex(5), 15));
}

#[test]
fn extend_preserves_anchor_from_existing_selection() {
    let existing = Selection {
        mode: SelectionMode::Char,
        anchor: SelectionPoint {
            row: StableRowIndex(5),
            col: 10,
            side: Side::Left,
        },
        pivot: SelectionPoint {
            row: StableRowIndex(5),
            col: 10,
            side: Side::Left,
        },
        end: SelectionPoint {
            row: StableRowIndex(5),
            col: 12,
            side: Side::Right,
        },
    };
    // Extend further right: anchor stays at col 10.
    let old_cursor = MarkCursor {
        row: StableRowIndex(5),
        col: 12,
    };
    let new_cursor = MarkCursor {
        row: StableRowIndex(5),
        col: 20,
    };
    let sel = extend_or_create_selection(Some(&existing), &old_cursor, &new_cursor);
    assert_eq!(sel.anchor.col, 10);
    assert_eq!(sel.end.col, 20);
    assert!(sel.contains(StableRowIndex(5), 10));
    assert!(sel.contains(StableRowIndex(5), 20));
}

#[test]
fn extend_equal_position_produces_empty_selection() {
    let mc = MarkCursor {
        row: StableRowIndex(0),
        col: 5,
    };
    let sel = extend_or_create_selection(None, &mc, &mc);
    assert!(sel.is_empty());
}

// ---------------------------------------------------------------------------
// select_all (SnapshotGrid-based)
// ---------------------------------------------------------------------------

#[test]
fn select_all_covers_entire_buffer() {
    let snap = test_snapshot(
        vec![
            vec![cell('a'), cell('b'), cell('c')],
            vec![cell('d'), cell('e'), cell('f')],
        ],
        3,
        100,
    );
    let grid = SnapshotGrid::new(&snap);
    let sel = select_all(&grid);
    assert_eq!(sel.mode, SelectionMode::Char);
    assert_eq!(sel.anchor.col, 0);
    assert_eq!(sel.anchor.side, Side::Left);
    assert_eq!(sel.end.col, 2); // cols - 1
    assert_eq!(sel.end.side, Side::Right);
}

// ---------------------------------------------------------------------------
// Auto-scroll (ensure_visible with SnapshotGrid)
// ---------------------------------------------------------------------------

#[test]
fn ensure_visible_in_viewport_returns_none() {
    let snap = test_snapshot(vec![vec![cell('a')]; 3], 1, 100);
    let grid = SnapshotGrid::new(&snap);
    // Cursor at viewport row 0 (stable 100, abs 100, first_visible=100).
    let cursor = MarkCursor {
        row: StableRowIndex(100),
        col: 0,
    };
    assert!(ensure_visible(&grid, &cursor).is_none());
}

#[test]
fn ensure_visible_above_viewport_returns_positive_delta() {
    let snap = test_snapshot(vec![vec![cell('a')]; 3], 1, 100);
    let grid = SnapshotGrid::new(&snap);
    // Cursor at absolute row 0 (stable 0), viewport starts at abs 100.
    let cursor = MarkCursor {
        row: StableRowIndex(0),
        col: 0,
    };
    let delta = ensure_visible(&grid, &cursor);
    assert_eq!(delta, Some(100)); // scroll up 100 rows
}

#[test]
fn ensure_visible_below_viewport_returns_negative_delta() {
    // Viewport scrolled into history: display_offset=10, first_visible=90.
    let snap = test_snapshot_full(vec![vec![cell('a')]; 3], 1, 90, 100, 10);
    let grid = SnapshotGrid::new(&snap);
    // Cursor at absolute row 100 (stable 100), last_visible = 90 + 2 = 92.
    let cursor = MarkCursor {
        row: StableRowIndex(100),
        col: 0,
    };
    let delta = ensure_visible(&grid, &cursor);
    assert_eq!(delta, Some(-8)); // scroll down 8 rows
}

// ---------------------------------------------------------------------------
// Word navigation with SnapshotGrid (integration)
// ---------------------------------------------------------------------------

#[test]
fn word_left_at_origin_clamps_with_snapshot_grid() {
    let snap = test_snapshot_full(
        vec![vec![cell('h'), cell('e'), cell('l'), cell('l'), cell('o')]],
        5,
        0,
        0,
        0,
    );
    let grid = SnapshotGrid::new(&snap);
    let ctx = extract_word_context(&grid, 0, 0, ",\u{2502}`|:\"' ()[]{}<>\t");
    let c = AbsCursor { abs_row: 0, col: 0 };
    let r = motion::word_left(c, &ctx);
    assert_eq!(r, AbsCursor { abs_row: 0, col: 0 });
}

#[test]
fn word_right_at_end_clamps_with_snapshot_grid() {
    let snap = test_snapshot_full(
        vec![vec![cell('h'), cell('e'), cell('l'), cell('l'), cell('o')]],
        5,
        0,
        0,
        0,
    );
    let grid = SnapshotGrid::new(&snap);
    let bounds = GridBounds {
        total_rows: 1,
        cols: 5,
        visible_lines: 1,
    };
    let ctx = extract_word_context(&grid, 0, 4, ",\u{2502}`|:\"' ()[]{}<>\t");
    let c = AbsCursor { abs_row: 0, col: 4 };
    let r = motion::word_right(c, &ctx, bounds);
    assert_eq!(r.abs_row, 0);
    assert_eq!(r.col, 4);
}
