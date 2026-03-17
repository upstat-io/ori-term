use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers};
use crate::layout::BoxContent;
use crate::sense::Sense;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{LayoutCtx, OnInputResult, Widget, WidgetAction};

use super::{TextInputStyle, TextInputWidget};

fn bounds() -> Rect {
    Rect::new(0.0, 0.0, 200.0, 28.0)
}

fn key_down(k: Key) -> InputEvent {
    InputEvent::KeyDown {
        key: k,
        modifiers: Modifiers::NONE,
    }
}

fn shift_key_down(k: Key) -> InputEvent {
    InputEvent::KeyDown {
        key: k,
        modifiers: Modifiers::SHIFT_ONLY,
    }
}

fn ctrl_key_down(k: Key) -> InputEvent {
    InputEvent::KeyDown {
        key: k,
        modifiers: Modifiers::CTRL_ONLY,
    }
}

fn char_down(ch: char) -> InputEvent {
    key_down(Key::Character(ch))
}

/// Helper: calls `on_input` with keyboard event and returns the result.
fn input(ti: &mut TextInputWidget, event: &InputEvent) -> OnInputResult {
    ti.on_input(event, bounds())
}

// -- Construction and state --

#[test]
fn default_state() {
    let ti = TextInputWidget::new();
    assert_eq!(ti.text(), "");
    assert_eq!(ti.cursor(), 0);
    assert!(ti.selection_anchor().is_none());
    assert!(!ti.is_disabled());
    assert!(ti.is_focusable());
}

// -- Sense and controllers --

#[test]
fn sense_returns_click_drag_focusable() {
    let ti = TextInputWidget::new();
    assert_eq!(
        ti.sense(),
        Sense::click_and_drag().union(Sense::focusable())
    );
}

#[test]
fn has_two_controllers() {
    let ti = TextInputWidget::new();
    assert_eq!(ti.controllers().len(), 2);
}

#[test]
fn has_visual_state_animator() {
    let ti = TextInputWidget::new();
    assert!(ti.visual_states().is_some());
}

// -- Text editing --

#[test]
fn type_characters() {
    let mut ti = TextInputWidget::new();

    input(&mut ti, &char_down('h'));
    input(&mut ti, &char_down('i'));
    assert_eq!(ti.text(), "hi");
    assert_eq!(ti.cursor(), 2);
}

#[test]
fn type_emits_text_changed() {
    let mut ti = TextInputWidget::new();

    let r = input(&mut ti, &char_down('a'));
    assert!(r.handled);
    assert_eq!(
        r.action,
        Some(WidgetAction::TextChanged {
            id: ti.id(),
            text: "a".to_string(),
        })
    );
}

#[test]
fn backspace_deletes() {
    let mut ti = TextInputWidget::new();

    input(&mut ti, &char_down('a'));
    input(&mut ti, &char_down('b'));
    assert_eq!(ti.text(), "ab");

    let r = input(&mut ti, &key_down(Key::Backspace));
    assert_eq!(ti.text(), "a");
    assert_eq!(ti.cursor(), 1);
    assert!(r.action.is_some());
}

#[test]
fn backspace_at_start_no_op() {
    let mut ti = TextInputWidget::new();

    let r = input(&mut ti, &key_down(Key::Backspace));
    assert_eq!(ti.text(), "");
    assert!(r.action.is_none());
}

#[test]
fn delete_forward() {
    let mut ti = TextInputWidget::new();

    ti.set_text("abc");
    ti.cursor = 1; // After 'a'.

    let r = input(&mut ti, &key_down(Key::Delete));
    assert_eq!(ti.text(), "ac");
    assert_eq!(ti.cursor(), 1);
    assert!(r.action.is_some());
}

#[test]
fn delete_at_end_no_op() {
    let mut ti = TextInputWidget::new();

    ti.set_text("abc");
    // Cursor is at end after set_text.

    let r = input(&mut ti, &key_down(Key::Delete));
    assert_eq!(ti.text(), "abc");
    assert!(r.action.is_none());
}

#[test]
fn arrow_keys_move_cursor() {
    let mut ti = TextInputWidget::new();

    ti.set_text("abc");
    ti.cursor = 3;

    input(&mut ti, &key_down(Key::ArrowLeft));
    assert_eq!(ti.cursor(), 2);

    input(&mut ti, &key_down(Key::ArrowLeft));
    assert_eq!(ti.cursor(), 1);

    input(&mut ti, &key_down(Key::ArrowRight));
    assert_eq!(ti.cursor(), 2);
}

#[test]
fn home_end_keys() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.cursor = 2;

    input(&mut ti, &key_down(Key::Home));
    assert_eq!(ti.cursor(), 0);

    input(&mut ti, &key_down(Key::End));
    assert_eq!(ti.cursor(), 5);
}

// -- Selection --

#[test]
fn shift_arrow_selects() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.cursor = 2;

    input(&mut ti, &shift_key_down(Key::ArrowRight));
    assert_eq!(ti.cursor(), 3);
    assert_eq!(ti.selection_anchor(), Some(2));
    assert_eq!(ti.selection_range(), Some((2, 3)));

    input(&mut ti, &shift_key_down(Key::ArrowRight));
    assert_eq!(ti.selection_range(), Some((2, 4)));
}

#[test]
fn ctrl_a_selects_all() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.cursor = 2;

    input(&mut ti, &ctrl_key_down(Key::Character('a')));
    assert_eq!(ti.selection_anchor(), Some(0));
    assert_eq!(ti.cursor(), 5);
    assert_eq!(ti.selection_range(), Some((0, 5)));
}

#[test]
fn type_replaces_selection() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.selection_anchor = Some(1);
    ti.cursor = 4; // Select "ell".

    input(&mut ti, &char_down('X'));
    assert_eq!(ti.text(), "hXo");
    assert_eq!(ti.cursor(), 2);
    assert!(ti.selection_anchor().is_none());
}

#[test]
fn backspace_deletes_selection() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.selection_anchor = Some(1);
    ti.cursor = 4;

    input(&mut ti, &key_down(Key::Backspace));
    assert_eq!(ti.text(), "ho");
    assert_eq!(ti.cursor(), 1);
}

#[test]
fn arrow_left_collapses_selection_to_start() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.selection_anchor = Some(1);
    ti.cursor = 4;

    // Left arrow without shift collapses selection to start.
    input(&mut ti, &key_down(Key::ArrowLeft));
    assert_eq!(ti.cursor(), 1);
    assert!(ti.selection_anchor().is_none());
}

#[test]
fn arrow_right_collapses_selection_to_end() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.selection_anchor = Some(1);
    ti.cursor = 4;

    // Right arrow without shift collapses selection to end.
    input(&mut ti, &key_down(Key::ArrowRight));
    assert_eq!(ti.cursor(), 4);
    assert!(ti.selection_anchor().is_none());
}

#[test]
fn ctrl_a_then_type_replaces_all() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    input(&mut ti, &ctrl_key_down(Key::Character('a')));
    assert_eq!(ti.selection_range(), Some((0, 5)));

    input(&mut ti, &char_down('X'));
    assert_eq!(ti.text(), "X");
    assert_eq!(ti.cursor(), 1);
    assert!(ti.selection_anchor().is_none());
}

#[test]
fn shift_home_selects_to_start() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.cursor = 3;

    input(&mut ti, &shift_key_down(Key::Home));
    assert_eq!(ti.cursor(), 0);
    assert_eq!(ti.selection_anchor(), Some(3));
    assert_eq!(ti.selection_range(), Some((0, 3)));
}

#[test]
fn shift_end_selects_to_end() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.cursor = 1;

    input(&mut ti, &shift_key_down(Key::End));
    assert_eq!(ti.cursor(), 5);
    assert_eq!(ti.selection_anchor(), Some(1));
    assert_eq!(ti.selection_range(), Some((1, 5)));
}

#[test]
fn delete_with_selection_removes_selected() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.selection_anchor = Some(1);
    ti.cursor = 4;

    input(&mut ti, &key_down(Key::Delete));
    assert_eq!(ti.text(), "ho");
    assert_eq!(ti.cursor(), 1);
    assert!(ti.selection_anchor().is_none());
}

#[test]
fn shift_left_then_right_cancels_selection() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.cursor = 3;

    // Select one char left.
    input(&mut ti, &shift_key_down(Key::ArrowLeft));
    assert_eq!(ti.selection_range(), Some((2, 3)));

    // Select one char right — cursor back to anchor.
    input(&mut ti, &shift_key_down(Key::ArrowRight));
    // Anchor is still 3, cursor is 3 — selection is (3,3) which is empty.
    assert_eq!(ti.cursor(), 3);
    assert_eq!(ti.selection_anchor(), Some(3));
}

// -- Disabled --

#[test]
fn disabled_ignores_keyboard() {
    let mut ti = TextInputWidget::new().with_disabled(true);

    assert!(!ti.is_focusable());

    let r = input(&mut ti, &char_down('a'));
    assert!(!r.handled);
    assert_eq!(ti.text(), "");
}

// -- Layout --

#[test]
fn layout_uses_min_width() {
    let ti = TextInputWidget::new();
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = ti.layout(&ctx);
    let s = TextInputStyle::default();

    if let BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        // Empty text -> placeholder empty -> min_width applies.
        assert!(*intrinsic_width >= s.min_width);
    } else {
        panic!("expected leaf layout");
    }
}

#[test]
fn placeholder_layout_measures_placeholder() {
    let ti = TextInputWidget::new().with_placeholder("Type here...");
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout = ti.layout(&ctx);

    if let BoxContent::Leaf {
        intrinsic_width, ..
    } = &layout.content
    {
        // "Type here..." = 12 chars * 8px = 96 + padding 16 = 112,
        // but min_width = 120 so it should be at least 120.
        assert!(*intrinsic_width >= 120.0);
    } else {
        panic!("expected leaf layout");
    }
}

// -- Unicode --

#[test]
fn unicode_editing() {
    let mut ti = TextInputWidget::new();

    // Type multi-byte chars.
    input(&mut ti, &char_down('\u{e9}'));
    input(&mut ti, &char_down('\u{e0}'));
    assert_eq!(ti.text(), "\u{e9}\u{e0}");
    assert_eq!(ti.cursor(), 4); // 2 bytes each.

    input(&mut ti, &key_down(Key::Backspace));
    assert_eq!(ti.text(), "\u{e9}");
    assert_eq!(ti.cursor(), 2);
}

#[test]
fn four_byte_unicode_editing() {
    let mut ti = TextInputWidget::new();

    // Emoji are 4 bytes each in UTF-8.
    input(&mut ti, &char_down('\u{1F600}')); // Grinning face.
    input(&mut ti, &char_down('\u{1F680}')); // Rocket.
    assert_eq!(ti.text().len(), 8); // 4 bytes each.
    assert_eq!(ti.cursor(), 8);

    input(&mut ti, &key_down(Key::Backspace));
    assert_eq!(ti.text(), "\u{1F600}");
    assert_eq!(ti.cursor(), 4);
}

// -- Edge cases --

#[test]
fn left_arrow_at_start_stays() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    ti.cursor = 0;

    input(&mut ti, &key_down(Key::ArrowLeft));
    assert_eq!(ti.cursor(), 0);
}

#[test]
fn right_arrow_at_end_stays() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    // cursor already at end from set_text.

    input(&mut ti, &key_down(Key::ArrowRight));
    assert_eq!(ti.cursor(), 5);
}

#[test]
fn set_text_moves_cursor_to_end() {
    let mut ti = TextInputWidget::new();
    ti.set_text("abc");
    assert_eq!(ti.cursor(), 3);
    assert!(ti.selection_anchor().is_none());
}

#[test]
fn escape_key_ignored() {
    let mut ti = TextInputWidget::new();

    ti.set_text("hello");
    let r = input(&mut ti, &key_down(Key::Escape));
    assert!(!r.handled);
    assert_eq!(ti.text(), "hello");
}

#[test]
fn ctrl_a_on_empty_text() {
    let mut ti = TextInputWidget::new();

    let r = input(&mut ti, &ctrl_key_down(Key::Character('a')));
    // Should still set selection (0,0) — anchor=0, cursor=0.
    assert_eq!(ti.selection_anchor(), Some(0));
    assert_eq!(ti.cursor(), 0);
    assert!(r.handled);
}

// -- Click-to-cursor (via cached char offsets) --

#[test]
fn click_positions_cursor() {
    let mut ti = TextInputWidget::new();
    ti.set_text("hello");

    // Populate char offsets via layout.
    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    ti.layout(&ctx);

    // Click near the start — should position at 0.
    let click = InputEvent::MouseDown {
        pos: crate::geometry::Point::new(7.0, 14.0), // Just inside padding.
        button: crate::input::MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let r = ti.on_input(&click, Rect::new(0.0, 0.0, 200.0, 28.0));
    assert!(r.handled);
    assert_eq!(ti.cursor(), 0);
    assert!(ti.selection_anchor().is_none());
}

#[test]
fn click_at_end_positions_cursor_at_end() {
    let mut ti = TextInputWidget::new();
    ti.set_text("hi");

    let m = MockMeasurer::new();
    let ctx = LayoutCtx {
        measurer: &m,
        theme: &super::super::tests::TEST_THEME,
    };
    ti.layout(&ctx);

    // Click far right — should position at end (byte 2).
    let click = InputEvent::MouseDown {
        pos: crate::geometry::Point::new(190.0, 14.0),
        button: crate::input::MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let r = ti.on_input(&click, Rect::new(0.0, 0.0, 200.0, 28.0));
    assert!(r.handled);
    assert_eq!(ti.cursor(), 2);
}

#[test]
fn disabled_ignores_click() {
    let mut ti = TextInputWidget::new().with_disabled(true);
    ti.set_text("hello");

    let click = InputEvent::MouseDown {
        pos: crate::geometry::Point::new(50.0, 14.0),
        button: crate::input::MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let r = ti.on_input(&click, bounds());
    assert!(!r.handled);
}
