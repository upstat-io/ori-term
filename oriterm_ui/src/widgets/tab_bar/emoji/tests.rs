//! Tests for tab icon extraction.

use super::*;
use crate::widgets::tab_bar::widget::TabIcon;

#[test]
fn emoji_prefix_extracted() {
    assert_eq!(
        extract_emoji_icon("🐍python"),
        Some(TabIcon::Emoji("🐍".to_owned()))
    );
}

#[test]
fn plain_text_returns_none() {
    assert_eq!(extract_emoji_icon("python"), None);
}

#[test]
fn empty_string_returns_none() {
    assert_eq!(extract_emoji_icon(""), None);
}

#[test]
fn flag_sequence_extracted() {
    assert_eq!(
        extract_emoji_icon("🇺🇸USA"),
        Some(TabIcon::Emoji("🇺🇸".to_owned()))
    );
}

#[test]
fn zwj_sequence_extracted() {
    assert_eq!(
        extract_emoji_icon("👨\u{200D}💻code"),
        Some(TabIcon::Emoji("👨\u{200D}💻".to_owned()))
    );
}

#[test]
fn standalone_emoji() {
    assert_eq!(
        extract_emoji_icon("🔥"),
        Some(TabIcon::Emoji("🔥".to_owned()))
    );
}

#[test]
fn digit_prefix_returns_none() {
    assert_eq!(extract_emoji_icon("42foo"), None);
}

#[test]
fn alpha_prefix_returns_none() {
    assert_eq!(extract_emoji_icon("claude"), None);
}

#[test]
fn whitespace_prefix_returns_none() {
    assert_eq!(extract_emoji_icon(" hello"), None);
}

#[test]
fn braille_spinner_extracted() {
    assert_eq!(
        extract_emoji_icon("⠂ Claude Code"),
        Some(TabIcon::Emoji("⠂".to_owned()))
    );
}

#[test]
fn braille_dot_extracted() {
    assert_eq!(
        extract_emoji_icon("⠐ working"),
        Some(TabIcon::Emoji("⠐".to_owned()))
    );
}

#[test]
fn star_symbol_extracted() {
    assert_eq!(
        extract_emoji_icon("✳ Claude Code"),
        Some(TabIcon::Emoji("✳".to_owned()))
    );
}

#[test]
fn hash_symbol_extracted() {
    // Non-alphanumeric symbols are valid icons.
    assert_eq!(
        extract_emoji_icon("#channel"),
        Some(TabIcon::Emoji("#".to_owned()))
    );
}

#[test]
fn path_prefix_returns_none() {
    // Paths starting with letters return None.
    assert_eq!(extract_emoji_icon("orc"), None);
}

#[test]
fn dot_prefix_extracted() {
    // Dots are not alphanumeric — extracted as icon.
    assert_eq!(
        extract_emoji_icon("..c/Users"),
        Some(TabIcon::Emoji(".".to_owned()))
    );
}
