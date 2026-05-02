//! OSC 4 / OSC 10 / OSC 11 / OSC 12 host color-query resolution.
//!
//! Resolves a queried palette slot against the pane's snapshot
//! palette, falling back to `oriterm_core::color::FALLBACK_COLOR`
//! when the snapshot is unavailable or the index is out of range.
//! Shares the SSOT fallback constant with `Palette::color()`.

/// Resolve an OSC 4 / OSC 10 / OSC 11 / OSC 12 color query against the
/// pane's palette snapshot.
///
/// `palette` is a slice of pre-resolved RGB triplets from
/// `PaneSnapshot::palette` — 270 entries covering 0..=255 (indexed
/// palette) and 256..=269 (named semantic slots: Foreground,
/// Background, Cursor, dim variants, etc.). `index` is the
/// pre-computed slot the VTE OSC dispatch resolved.
///
/// Returns `oriterm_core::color::FALLBACK_COLOR` (black) when the
/// snapshot is missing (`None`) OR the index is out of range —
/// shares the SSOT fallback constant with `Palette::color()`.
pub(super) fn resolve_host_color_query(
    palette: Option<&[[u8; 3]]>,
    index: usize,
) -> oriterm_core::color::Rgb {
    palette
        .and_then(|p| p.get(index).copied())
        .map_or(oriterm_core::color::FALLBACK_COLOR, |[r, g, b]| {
            oriterm_core::color::Rgb { r, g, b }
        })
}
