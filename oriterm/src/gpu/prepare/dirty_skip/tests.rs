//! Tests for row-level dirty skip logic.

use oriterm_core::term::renderable::DamageLine;
use oriterm_core::{Column, CursorShape, RenderableCursor};

use crate::gpu::frame_input::FrameInput;

use super::{BufferLengths, RowInstanceRanges, build_dirty_set};

#[test]
fn all_dirty_marks_every_row() {
    let mut input = FrameInput::test_grid(10, 5, "");
    input.content.all_dirty = true;
    let dirty = build_dirty_set(&input, 5);
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
    let dirty = build_dirty_set(&input, 5);
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
    let dirty = build_dirty_set(&input, 5);
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
    let dirty = build_dirty_set(&input, 5);
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
