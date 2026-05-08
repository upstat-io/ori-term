//! Tests for row-level dirty skip logic.

use oriterm_core::index::Side;
use oriterm_core::selection::SelectionMode;
use oriterm_core::term::renderable::DamageLine;
use oriterm_core::{Column, CursorShape, RenderableCursor};

use crate::gpu::frame_input::{FrameInput, SelectionDamageSnapshot};

use super::{BufferLengths, RowInstanceRanges, build_dirty_set, mark_selection_damage};

/// Build a snapshot covering lines `start..=end` with default column extents.
///
/// Uses char mode with column 0..80 to represent a typical full-line selection.
fn snap(start: usize, end: usize) -> SelectionDamageSnapshot {
    SelectionDamageSnapshot {
        start_line: start,
        end_line: end,
        start_col: 0,
        start_side: Side::Left,
        end_col: 80,
        end_side: Side::Right,
        mode: SelectionMode::Char,
    }
}

/// Build a `PrevFrameState` from individual fields for terse test setup.
fn prev_state_of(
    selection_snapshot: Option<SelectionDamageSnapshot>,
    prev_cursor_line: Option<usize>,
    prev_hovered_cell: Option<(usize, usize)>,
) -> super::selection_damage::PrevFrameState {
    // The visibility-canonicalized storage rule means Some(line) at the
    // call site implies a visible cursor; reconstruct a RenderableCursor
    // with visible=true so build_dirty_set's cursor_line() yields the line.
    let resolved_cursor = prev_cursor_line.map(|line| RenderableCursor {
        line,
        column: Column(0),
        shape: CursorShape::Block,
        visible: true,
    });
    super::selection_damage::PrevFrameState {
        selection_snapshot,
        resolved_cursor,
        hovered_cell: prev_hovered_cell,
    }
}

/// Helper: call `build_dirty_set` with a reusable scratch buffer and return it.
/// Uses the raw `input.content.cursor` as the resolved cursor (no mark mode
/// override applies in these tests).
fn dirty_set(
    input: &FrameInput,
    num_rows: usize,
    prev_sel: Option<SelectionDamageSnapshot>,
) -> Vec<bool> {
    let mut buf = Vec::new();
    let resolved = input.content.cursor;
    build_dirty_set(
        input,
        num_rows,
        &resolved,
        prev_state_of(prev_sel, None, None),
        &mut buf,
    );
    buf
}

/// Helper: call `build_dirty_set` with the previous-cursor-row argument.
fn dirty_set_with_prev_cursor(
    input: &FrameInput,
    num_rows: usize,
    prev_sel: Option<SelectionDamageSnapshot>,
    prev_cursor_line: Option<usize>,
) -> Vec<bool> {
    let mut buf = Vec::new();
    let resolved = input.content.cursor;
    build_dirty_set(
        input,
        num_rows,
        &resolved,
        prev_state_of(prev_sel, prev_cursor_line, None),
        &mut buf,
    );
    buf
}

/// Helper: call `build_dirty_set` with the previous-hovered-cell argument.
fn dirty_set_with_prev_hover(
    input: &FrameInput,
    num_rows: usize,
    prev_hovered_cell: Option<(usize, usize)>,
) -> Vec<bool> {
    let mut buf = Vec::new();
    let resolved = input.content.cursor;
    build_dirty_set(
        input,
        num_rows,
        &resolved,
        prev_state_of(None, None, prev_hovered_cell),
        &mut buf,
    );
    buf
}

/// Regression: BUG-06-027 — when the cursor moves between frames, the
/// previous cursor row must also be marked dirty so its cells regenerate
/// without inheriting the "with cursor" per-cell colors that were baked
/// into `saved_tier`.
#[test]
fn cursor_move_dirties_previous_cursor_row() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor = RenderableCursor {
        line: 2,
        column: Column(0),
        visible: true,
        shape: CursorShape::Block,
    };
    input.content.damage.clear();

    // Previous frame had cursor on row 0; current frame moved it to row 2.
    let dirty = dirty_set_with_prev_cursor(&input, 5, None, Some(0));
    assert!(dirty[0], "previous cursor row (0) marked dirty after move");
    assert!(dirty[2], "current cursor row (2) always dirty");
    assert!(!dirty[1], "non-cursor row stays clean");
}

/// Regression: BUG-06-027 — when the hovered cell moves, both the
/// previous and current hover rows must be dirtied so the hyperlink
/// solid-underline decoration migrates with the mouse without falling
/// back to a full rebuild.
#[test]
fn hover_move_dirties_previous_and_current_hover_rows() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    input.hovered_cell = Some((3, 4));

    // Previous hover at row 1, new hover at row 3. Both rows must be
    // dirty so the hover decoration is regenerated correctly.
    let dirty = dirty_set_with_prev_hover(&input, 5, Some((1, 2)));
    assert!(dirty[1], "previous hover row dirty");
    assert!(dirty[3], "current hover row dirty");
    assert!(!dirty[0]);
    assert!(!dirty[2]);
    assert!(!dirty[4]);
}

/// Regression: BUG-06-027 negative pin — when the hovered cell does not
/// change, `build_dirty_set` must not dirty hover rows.
#[test]
fn hover_stationary_does_not_dirty_hover_rows() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage.clear();
    input.hovered_cell = Some((2, 3));

    let dirty = dirty_set_with_prev_hover(&input, 5, Some((2, 3)));
    assert!(dirty.iter().all(|&d| !d), "no rows dirtied");
}

/// Regression: BUG-06-027 negative pin — when the cursor stays put,
/// `build_dirty_set` must not gratuitously dirty extra rows.
#[test]
fn cursor_stationary_does_not_dirty_previous_cursor_row() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor = RenderableCursor {
        line: 2,
        column: Column(0),
        visible: true,
        shape: CursorShape::Block,
    };
    input.content.damage.clear();

    let dirty = dirty_set_with_prev_cursor(&input, 5, None, Some(2));
    assert!(dirty[2], "current cursor row dirty");
    assert!(!dirty[0]);
    assert!(!dirty[1]);
    assert!(!dirty[3]);
    assert!(!dirty[4]);
}

#[test]
fn all_dirty_marks_every_row() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = true;
    let dirty = dirty_set(&input, 5, None);
    assert!(dirty.iter().all(|&d| d));
}

#[test]
fn damage_marks_specific_rows() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    input.content.damage = vec![
        DamageLine {
            line: 1,
            left: Column(0),
            right: Column(9),
        },
        DamageLine {
            line: 3,
            left: Column(0),
            right: Column(9),
        },
    ];
    let dirty = dirty_set(&input, 5, None);
    assert!(!dirty[0]);
    assert!(dirty[1]);
    assert!(!dirty[2]);
    assert!(dirty[3]);
    assert!(!dirty[4]);
}

#[test]
fn cursor_row_always_dirty() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor = RenderableCursor {
        line: 2,
        column: Column(0),
        shape: CursorShape::Block,
        visible: true,
    };
    let dirty = dirty_set(&input, 5, None);
    assert!(dirty[2]);
    assert!(!dirty[0]);
}

#[test]
fn invisible_cursor_not_dirty() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor = RenderableCursor {
        line: 2,
        column: Column(0),
        shape: CursorShape::Block,
        visible: false,
    };
    let dirty = dirty_set(&input, 5, None);
    assert!(!dirty[2]);
}

#[test]
fn buffer_lengths_range_since() {
    let before = BufferLengths {
        backgrounds: 0,
        glyphs: 160,
        subpixel_glyphs: 0,
        color_glyphs: 0,
    };
    let after = BufferLengths {
        backgrounds: 800,
        glyphs: 480,
        subpixel_glyphs: 80,
        color_glyphs: 0,
    };
    let ranges = after.range_since(&before);
    assert_eq!(ranges.backgrounds, 0..800);
    assert_eq!(ranges.glyphs, 160..480);
    assert_eq!(ranges.subpixel_glyphs, 0..80);
    assert_eq!(ranges.color_glyphs, 0..0);
}

#[test]
fn empty_row_range_is_default() {
    let r = RowInstanceRanges::default();
    assert!(r.backgrounds.is_empty());
    assert!(r.glyphs.is_empty());
    assert!(r.subpixel_glyphs.is_empty());
    assert!(r.color_glyphs.is_empty());
}

// Selection damage tracking tests.

#[test]
fn new_selection_damages_selected_lines() {
    let mut dirty = vec![false; 10];
    mark_selection_damage(&mut dirty, None, Some(snap(3, 7)));
    assert!(!dirty[0]);
    assert!(!dirty[2]);
    assert!(dirty[3]);
    assert!(dirty[4]);
    assert!(dirty[5]);
    assert!(dirty[6]);
    assert!(dirty[7]);
    assert!(!dirty[8]);
}

#[test]
fn clear_selection_damages_previously_selected_lines() {
    let mut dirty = vec![false; 10];
    mark_selection_damage(&mut dirty, Some(snap(2, 5)), None);
    assert!(!dirty[0]);
    assert!(!dirty[1]);
    assert!(dirty[2]);
    assert!(dirty[3]);
    assert!(dirty[4]);
    assert!(dirty[5]);
    assert!(!dirty[6]);
}

#[test]
fn extend_selection_damages_new_lines_and_boundary() {
    let mut dirty = vec![false; 10];
    // Extend from lines 3-5 to lines 3-8.
    mark_selection_damage(&mut dirty, Some(snap(3, 5)), Some(snap(3, 8)));
    // Interior lines 4 stayed selected — not dirty.
    assert!(!dirty[4]);
    // Boundary lines dirty (old end=5, new end=8).
    assert!(dirty[5]);
    assert!(dirty[8]);
    // Newly-covered lines 6-7 dirty.
    assert!(dirty[6]);
    assert!(dirty[7]);
    // Old start=3 is a boundary → dirty.
    assert!(dirty[3]);
    // Outside range: clean.
    assert!(!dirty[0]);
    assert!(!dirty[9]);
}

#[test]
fn shrink_selection_damages_uncovered_lines() {
    let mut dirty = vec![false; 10];
    // Shrink from lines 2-7 to lines 2-4.
    mark_selection_damage(&mut dirty, Some(snap(2, 7)), Some(snap(2, 4)));
    // Uncovered lines 5-7 dirty.
    assert!(dirty[5]);
    assert!(dirty[6]);
    assert!(dirty[7]);
    // Interior line 3 stayed selected — not dirty.
    assert!(!dirty[3]);
    // Boundary lines dirty.
    assert!(dirty[2]);
    assert!(dirty[4]);
}

#[test]
fn same_selection_no_damage() {
    let mut dirty = vec![false; 10];
    mark_selection_damage(&mut dirty, Some(snap(3, 7)), Some(snap(3, 7)));
    assert!(dirty.iter().all(|&d| !d));
}

#[test]
fn selection_damage_integrated_with_build_dirty_set() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    // No content damage, but previous selection covered lines 1-3
    // and current frame has no selection (cleared).
    let dirty = dirty_set(&input, 5, Some(snap(1, 3)));
    assert!(!dirty[0]);
    assert!(dirty[1]);
    assert!(dirty[2]);
    assert!(dirty[3]);
    assert!(!dirty[4]);
}

#[test]
fn selection_damage_clamped_to_viewport() {
    let mut dirty = vec![false; 5];
    // Selection extends beyond viewport (lines 3-20 but only 5 rows).
    mark_selection_damage(&mut dirty, None, Some(snap(3, 20)));
    assert!(!dirty[0]);
    assert!(!dirty[2]);
    assert!(dirty[3]);
    assert!(dirty[4]);
}

// Intra-line selection change detection (regression test for the drag bug).

#[test]
fn same_line_range_different_columns_marks_boundary_dirty() {
    let mut dirty = vec![false; 10];
    // Previous: selection on line 5, columns 3..10.
    let old = SelectionDamageSnapshot {
        start_line: 5,
        end_line: 5,
        start_col: 3,
        start_side: Side::Left,
        end_col: 10,
        end_side: Side::Right,
        mode: SelectionMode::Char,
    };
    // New: selection on line 5, columns 3..20 (drag extended).
    let new = SelectionDamageSnapshot {
        start_line: 5,
        end_line: 5,
        start_col: 3,
        start_side: Side::Left,
        end_col: 20,
        end_side: Side::Right,
        mode: SelectionMode::Char,
    };
    // old != new because end_col differs, so `if old == new` doesn't bail.
    // Line 5 is a boundary of both old and new → marked dirty.
    mark_selection_damage(&mut dirty, Some(old), Some(new));
    assert!(dirty[5], "same-line drag must mark the row dirty");
    // Non-selected lines stay clean.
    assert!(!dirty[0]);
    assert!(!dirty[4]);
    assert!(!dirty[6]);
}

#[test]
fn zero_area_to_real_selection_on_same_line_marks_dirty() {
    let mut dirty = vec![false; 10];
    // Zero-area click: start_col > end_col after effective_start/end logic.
    let old = SelectionDamageSnapshot {
        start_line: 3,
        end_line: 3,
        start_col: 5,
        start_side: Side::Right,
        end_col: 5,
        end_side: Side::Right,
        mode: SelectionMode::Char,
    };
    // Drag: extends to column 15 on same line.
    let new = SelectionDamageSnapshot {
        start_line: 3,
        end_line: 3,
        start_col: 5,
        start_side: Side::Right,
        end_col: 15,
        end_side: Side::Right,
        mode: SelectionMode::Char,
    };
    mark_selection_damage(&mut dirty, Some(old), Some(new));
    assert!(dirty[3], "drag from zero-area must mark the row dirty");
}

#[test]
fn block_mode_column_change_dirties_all_interior_rows() {
    let mut dirty = vec![false; 10];
    // Block selection: rows 2-5, cols 3-4.
    let old = SelectionDamageSnapshot {
        start_line: 2,
        end_line: 5,
        start_col: 3,
        start_side: Side::Left,
        end_col: 4,
        end_side: Side::Right,
        mode: SelectionMode::Block,
    };
    // Block widens: rows 2-5, cols 3-8.
    let new = SelectionDamageSnapshot {
        start_line: 2,
        end_line: 5,
        start_col: 3,
        start_side: Side::Left,
        end_col: 8,
        end_side: Side::Right,
        mode: SelectionMode::Block,
    };
    mark_selection_damage(&mut dirty, Some(old), Some(new));
    // All rows in the block must be dirty — not just boundaries.
    assert!(dirty[2], "block start row");
    assert!(dirty[3], "block interior row");
    assert!(dirty[4], "block interior row");
    assert!(dirty[5], "block end row");
    // Outside range stays clean.
    assert!(!dirty[0]);
    assert!(!dirty[1]);
    assert!(!dirty[6]);
}

#[test]
fn mode_change_dirties_entire_union() {
    let mut dirty = vec![false; 10];
    // Linear char selection on rows 3-6.
    let old = SelectionDamageSnapshot {
        start_line: 3,
        end_line: 6,
        start_col: 5,
        start_side: Side::Left,
        end_col: 10,
        end_side: Side::Right,
        mode: SelectionMode::Char,
    };
    // Switch to block mode on same rows.
    let new = SelectionDamageSnapshot {
        start_line: 3,
        end_line: 6,
        start_col: 5,
        start_side: Side::Left,
        end_col: 10,
        end_side: Side::Right,
        mode: SelectionMode::Block,
    };
    mark_selection_damage(&mut dirty, Some(old), Some(new));
    // Mode change means visual meaning is completely different for all rows.
    assert!(dirty[3]);
    assert!(dirty[4]);
    assert!(dirty[5]);
    assert!(dirty[6]);
    assert!(!dirty[2]);
    assert!(!dirty[7]);
}

#[test]
fn linear_shrink_from_top_dirties_uncovered_rows() {
    let mut dirty = vec![false; 10];
    // Shrink from top: old=2-7, new=5-7 (start moves down).
    mark_selection_damage(&mut dirty, Some(snap(2, 7)), Some(snap(5, 7)));
    // Uncovered top rows 2-4 must be dirty.
    assert!(dirty[2]);
    assert!(dirty[3]);
    assert!(dirty[4]);
    // New start boundary dirty.
    assert!(dirty[5]);
    // Interior rows that stayed selected — line 6 is interior.
    assert!(!dirty[6]);
    // Old end=7, new end=7 is boundary → dirty.
    assert!(dirty[7]);
    // Outside range stays clean.
    assert!(!dirty[0]);
    assert!(!dirty[8]);
}

#[test]
fn block_selection_edge_at_wide_char_boundary_dirties_rows() {
    let mut dirty = vec![false; 10];
    // Block selection where column edge sits at a wide-char boundary.
    // The damage logic operates on lines, not columns, so the key assertion
    // is that column changes in block mode dirty ALL rows — even when the
    // change is a single column shift (e.g. from col 5 to col 6 which
    // might split a wide character).
    let old = SelectionDamageSnapshot {
        start_line: 1,
        end_line: 4,
        start_col: 5,
        start_side: Side::Left,
        end_col: 5,
        end_side: Side::Right,
        mode: SelectionMode::Block,
    };
    let new = SelectionDamageSnapshot {
        start_line: 1,
        end_line: 4,
        start_col: 5,
        start_side: Side::Left,
        end_col: 6,
        end_side: Side::Right,
        mode: SelectionMode::Block,
    };
    mark_selection_damage(&mut dirty, Some(old), Some(new));
    // Block mode with column change → all rows in range dirty.
    assert!(dirty[1]);
    assert!(dirty[2]);
    assert!(dirty[3]);
    assert!(dirty[4]);
    assert!(!dirty[0]);
    assert!(!dirty[5]);
}
