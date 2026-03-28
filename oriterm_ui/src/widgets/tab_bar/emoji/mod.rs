//! Tab icon extraction from OSC icon name strings.
//!
//! Extracts the leading grapheme cluster from an icon name (set by OSC 0/1)
//! when it starts with a non-alphanumeric, non-whitespace character. This
//! captures emoji, braille spinners, symbols, and other visual indicators
//! that terminals use as process icons.

use unicode_segmentation::UnicodeSegmentation;

use super::widget::TabIcon;

/// Extract a leading icon character from an icon name for use as a tab icon.
///
/// Returns `Some(TabIcon::Emoji(grapheme))` when the first grapheme cluster
/// starts with a character that is not alphanumeric and not whitespace.
/// This covers emoji, braille patterns, symbols, and other visual indicators.
/// Returns `None` for plain text, empty strings, paths, and other
/// alphanumeric-leading strings.
pub fn extract_emoji_icon(icon_name: &str) -> Option<TabIcon> {
    let grapheme = icon_name.graphemes(true).next()?;
    let first_cp = grapheme.chars().next()?;
    if !first_cp.is_alphanumeric() && !first_cp.is_whitespace() {
        Some(TabIcon::Emoji(grapheme.to_owned()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
