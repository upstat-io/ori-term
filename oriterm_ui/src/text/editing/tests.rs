//! Tests for `TextEditingState`.

use super::TextEditingState;

#[test]
fn empty_state_cursor_at_zero() {
    let s = TextEditingState::new();
    assert_eq!(s.cursor(), 0);
    assert!(s.text().is_empty());
    assert!(s.selection_anchor().is_none());
}

#[test]
fn insert_ascii_updates_text_and_cursor() {
    let mut s = TextEditingState::new();
    s.insert_char('a');
    assert_eq!(s.text(), "a");
    assert_eq!(s.cursor(), 1);
}

#[test]
fn insert_multibyte_unicode() {
    let mut s = TextEditingState::new();
    s.insert_char('\u{00f6}'); // o-umlaut, 2 bytes
    assert_eq!(s.text(), "\u{00f6}");
    assert_eq!(s.cursor(), 2);
    assert!(s.text().is_char_boundary(s.cursor()));

    s.insert_char('\u{4f60}'); // CJK character, 3 bytes
    assert_eq!(s.cursor(), 5);
    assert!(s.text().is_char_boundary(s.cursor()));
}

#[test]
fn backspace_at_start_is_noop() {
    let mut s = TextEditingState::new();
    assert!(!s.backspace());
    assert!(s.text().is_empty());
}

#[test]
fn backspace_removes_previous_char() {
    let mut s = TextEditingState::new();
    s.insert_char('a');
    s.insert_char('b');
    s.insert_char('c');
    assert!(s.backspace());
    assert_eq!(s.text(), "ab");
    assert_eq!(s.cursor(), 2);
}

#[test]
fn delete_at_end_is_noop() {
    let mut s = TextEditingState::new();
    s.insert_char('x');
    assert!(!s.delete());
    assert_eq!(s.text(), "x");
}

#[test]
fn delete_removes_next_char() {
    let mut s = TextEditingState::new();
    s.set_text("abc");
    s.set_cursor(1);
    assert!(s.delete());
    assert_eq!(s.text(), "ac");
    assert_eq!(s.cursor(), 1);
}

#[test]
fn move_left_at_start_is_noop() {
    let mut s = TextEditingState::new();
    s.set_text("hi");
    s.set_cursor(0);
    s.move_left(false);
    assert_eq!(s.cursor(), 0);
}

#[test]
fn move_right_at_end_is_noop() {
    let mut s = TextEditingState::new();
    s.set_text("hi");
    // cursor is at end after set_text
    s.move_right(false);
    assert_eq!(s.cursor(), 2);
}

#[test]
fn select_all_selects_entire_text() {
    let mut s = TextEditingState::new();
    s.set_text("hello");
    s.select_all();
    assert_eq!(s.selection_anchor(), Some(0));
    assert_eq!(s.cursor(), 5);
    assert_eq!(s.selection_range(), Some((0, 5)));
}

#[test]
fn select_all_empty_is_noop() {
    let mut s = TextEditingState::new();
    s.select_all();
    assert!(s.selection_anchor().is_none());
}

#[test]
fn delete_selection_removes_range() {
    let mut s = TextEditingState::new();
    s.set_text("hello");
    s.set_cursor(1);
    s.select_all(); // anchor=0, cursor=5
    // Override to select "ell" (1..4)
    s.set_cursor(1);
    s.move_right(true);
    s.move_right(true);
    s.move_right(true);
    assert_eq!(s.selection_range(), Some((1, 4)));
    assert!(s.delete_selection());
    assert_eq!(s.text(), "ho");
    assert_eq!(s.cursor(), 1);
}

#[test]
fn home_moves_to_start() {
    let mut s = TextEditingState::new();
    s.set_text("abc");
    // cursor at end
    s.home(false);
    assert_eq!(s.cursor(), 0);
    assert!(s.selection_anchor().is_none());
}

#[test]
fn end_moves_to_end() {
    let mut s = TextEditingState::new();
    s.set_text("abc");
    s.set_cursor(0);
    s.end(false);
    assert_eq!(s.cursor(), 3);
    assert!(s.selection_anchor().is_none());
}
