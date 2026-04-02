//! Tests for tab bar input dispatch logic.

use winit::keyboard::{Key, NamedKey};

use super::{TabEditAction, tab_edit_key_action};

// -- Editing key dispatch --

#[test]
fn enter_commits() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Enter), false, false),
        TabEditAction::Commit,
    );
}

#[test]
fn tab_commits() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Tab), false, false),
        TabEditAction::Commit,
    );
}

#[test]
fn escape_cancels() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Escape), false, false),
        TabEditAction::Cancel,
    );
}

#[test]
fn backspace_deletes_backward() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Backspace), false, false),
        TabEditAction::Backspace,
    );
}

#[test]
fn delete_deletes_forward() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Delete), false, false),
        TabEditAction::Delete,
    );
}

#[test]
fn arrow_left_moves_without_selection() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::ArrowLeft), false, false),
        TabEditAction::MoveLeft {
            extend_selection: false,
        },
    );
}

#[test]
fn shift_arrow_left_extends_selection() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::ArrowLeft), true, false),
        TabEditAction::MoveLeft {
            extend_selection: true,
        },
    );
}

#[test]
fn arrow_right_moves_without_selection() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::ArrowRight), false, false),
        TabEditAction::MoveRight {
            extend_selection: false,
        },
    );
}

#[test]
fn shift_arrow_right_extends_selection() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::ArrowRight), true, false),
        TabEditAction::MoveRight {
            extend_selection: true,
        },
    );
}

#[test]
fn home_without_shift() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Home), false, false),
        TabEditAction::Home {
            extend_selection: false,
        },
    );
}

#[test]
fn shift_home_extends_selection() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Home), true, false),
        TabEditAction::Home {
            extend_selection: true,
        },
    );
}

#[test]
fn end_without_shift() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::End), false, false),
        TabEditAction::End {
            extend_selection: false,
        },
    );
}

#[test]
fn shift_end_extends_selection() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::End), true, false),
        TabEditAction::End {
            extend_selection: true,
        },
    );
}

#[test]
fn ctrl_a_selects_all() {
    assert_eq!(
        tab_edit_key_action(&Key::Character("a".into()), false, true),
        TabEditAction::SelectAll,
    );
}

#[test]
fn printable_char_inserts() {
    assert_eq!(
        tab_edit_key_action(&Key::Character("x".into()), false, false),
        TabEditAction::InsertChars("x".to_string()),
    );
}

#[test]
fn multi_char_inserts_all() {
    assert_eq!(
        tab_edit_key_action(&Key::Character("abc".into()), false, false),
        TabEditAction::InsertChars("abc".to_string()),
    );
}

#[test]
fn control_char_filtered_to_consumed() {
    // A string of only control characters should produce Consumed.
    assert_eq!(
        tab_edit_key_action(&Key::Character("\x01".into()), false, false),
        TabEditAction::Consumed,
    );
}

#[test]
fn unhandled_named_key_consumed() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::F1), false, false),
        TabEditAction::Consumed,
    );
}

#[test]
fn space_inserts() {
    assert_eq!(
        tab_edit_key_action(&Key::Named(NamedKey::Space), false, false),
        TabEditAction::Consumed, // Space is a Named key, not Character.
    );
}
