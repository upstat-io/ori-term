//! Tests for emoji icon extraction.

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
fn symbol_prefix_returns_none() {
    assert_eq!(extract_emoji_icon("#channel"), None);
}

#[test]
fn eight_spoked_asterisk_is_emoji() {
    // U+2733 has Emoji_Presentation — it's a valid emoji icon.
    assert_eq!(
        extract_emoji_icon("✳ Claude Code"),
        Some(TabIcon::Emoji("✳".to_owned()))
    );
}

#[test]
fn braille_returns_none() {
    assert_eq!(extract_emoji_icon("⠂ Claude Code"), None);
    assert_eq!(extract_emoji_icon("⠐ working"), None);
}

#[test]
fn ascii_punctuation_returns_none() {
    assert_eq!(extract_emoji_icon("..c/Users"), None);
    assert_eq!(extract_emoji_icon("/home/user"), None);
    assert_eq!(extract_emoji_icon("-flag"), None);
}

#[test]
fn path_prefix_returns_none() {
    assert_eq!(extract_emoji_icon("orc"), None);
}
