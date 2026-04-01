//! Tests for search key dispatch logic.

use winit::keyboard::{Key, NamedKey};

use super::{SearchAction, search_key_action};

#[test]
fn escape_closes_search() {
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::Escape), false),
        SearchAction::Close,
    );
}

#[test]
fn enter_navigates_next() {
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::Enter), false),
        SearchAction::NextMatch,
    );
}

#[test]
fn shift_enter_navigates_prev() {
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::Enter), true),
        SearchAction::PrevMatch,
    );
}

#[test]
fn backspace_deletes_last_char() {
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::Backspace), false),
        SearchAction::Backspace,
    );
}

#[test]
fn character_appends_text() {
    assert_eq!(
        search_key_action(&Key::Character("a".into()), false),
        SearchAction::AppendText("a".to_string()),
    );
}

#[test]
fn multi_char_appends_full_string() {
    assert_eq!(
        search_key_action(&Key::Character("abc".into()), false),
        SearchAction::AppendText("abc".to_string()),
    );
}

#[test]
fn unhandled_named_key_consumed() {
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::F1), false),
        SearchAction::Consumed,
    );
}

#[test]
fn tab_key_consumed() {
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::Tab), false),
        SearchAction::Consumed,
    );
}

#[test]
fn arrow_keys_consumed() {
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::ArrowUp), false),
        SearchAction::Consumed,
    );
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::ArrowDown), false),
        SearchAction::Consumed,
    );
}

#[test]
fn shift_does_not_affect_escape() {
    // Shift+Escape should still close.
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::Escape), true),
        SearchAction::Close,
    );
}

#[test]
fn shift_does_not_affect_backspace() {
    // Shift+Backspace should still backspace.
    assert_eq!(
        search_key_action(&Key::Named(NamedKey::Backspace), true),
        SearchAction::Backspace,
    );
}
