//! Tests for row-level dirty skip logic.

use oriterm_core::term::renderable::DamageLine;
use oriterm_core::{Column, CursorShape, RenderableCursor};

use crate::gpu::frame_input::FrameInput;

use super::{BufferLengths, RowInstanceRanges, build_dirty_set, mark_selection_damage};

#[test]
fn all_dirty_marks_every_row() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = true;
    let dirty = build_dirty_set(&input, 5, None);
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
    let dirty = build_dirty_set(&input, 5, None);
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
    let dirty = build_dirty_set(&input, 5, None);
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
    let dirty = build_dirty_set(&input, 5, None);
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
    mark_selection_damage(&mut dirty, None, Some((3, 7)));
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
    mark_selection_damage(&mut dirty, Some((2, 5)), None);
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
    mark_selection_damage(&mut dirty, Some((3, 5)), Some((3, 8)));
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
    mark_selection_damage(&mut dirty, Some((2, 7)), Some((2, 4)));
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
    mark_selection_damage(&mut dirty, Some((3, 7)), Some((3, 7)));
    assert!(dirty.iter().all(|&d| !d));
}

#[test]
fn selection_damage_integrated_with_build_dirty_set() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = false;
    input.content.cursor.visible = false;
    // No content damage, but previous selection covered lines 1-3
    // and current frame has no selection (cleared).
    let dirty = build_dirty_set(&input, 5, Some((1, 3)));
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
    mark_selection_damage(&mut dirty, None, Some((3, 20)));
    assert!(!dirty[0]);
    assert!(!dirty[2]);
    assert!(dirty[3]);
    assert!(dirty[4]);
}
