//! Emoji detection for tab icon extraction.
//!
//! Extracts the leading emoji grapheme cluster from an icon name string
//! (set by OSC 0/1). Only recognizes codepoints in the `Emoji_Presentation`
//! set — alphanumeric or symbol prefixes are not treated as icons.

use unicode_segmentation::UnicodeSegmentation;

use super::widget::TabIcon;

/// Extract a leading emoji from an icon name for use as a tab icon.
///
/// Returns `Some(TabIcon::Emoji(grapheme))` when the first grapheme cluster
/// starts with an `Emoji_Presentation` codepoint. Returns `None` for plain
/// text, empty strings, or non-emoji leading characters.
pub fn extract_emoji_icon(icon_name: &str) -> Option<TabIcon> {
    let grapheme = icon_name.graphemes(true).next()?;
    let first_cp = grapheme.chars().next()?;
    if is_emoji_presentation(first_cp) {
        Some(TabIcon::Emoji(grapheme.to_owned()))
    } else {
        None
    }
}

/// Whether a codepoint has `Emoji_Presentation` — renders as emoji by default.
///
/// Covers the most common pictographic emoji ranges. Variation selectors
/// (U+FE0F) are handled by grapheme clustering, not here.
fn is_emoji_presentation(cp: char) -> bool {
    matches!(cp,
        // Miscellaneous Technical (watch, hourglass, play buttons, etc.).
        '\u{2300}'..='\u{23FF}'
        // Miscellaneous Symbols (sun, cloud, stars, zodiac, etc.).
        | '\u{2600}'..='\u{27BF}'
        // Supplemental arrows / misc symbols (stars, circles).
        | '\u{2B50}'..='\u{2B55}'
        // CJK symbols and Mahjong tiles.
        | '\u{3030}' | '\u{303D}'
        // Enclosed CJK letters.
        | '\u{3297}' | '\u{3299}'
        // Enclosed alphanumeric supplement.
        | '\u{1F100}'..='\u{1F1FF}'
        // Supplementary symbols and pictographs (most emoji live here).
        | '\u{1F200}'..='\u{1FFFF}'
    )
}

#[cfg(test)]
mod tests;
