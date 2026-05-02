//! Module-level helpers for `mux_pump` notification handling.
//!
//! Extracted from `mod.rs` to keep that file under the 500-line hard
//! limit and to consolidate the two `impl App` blocks per
//! `code-hygiene.md §File Organization` ordering (inherent impls before
//! free functions).

use oriterm_mux::MuxNotification;

/// In-place collapse of `ClearPendingDesktopNotifications` against
/// preceding `DesktopNotification` entries in the same staging
/// buffer. For each clear marker at position `i` for pane `P`,
/// removes every `DesktopNotification { pane_id: P, .. }` at
/// positions `< i`. Iteration order preserves remaining markers.
pub(super) fn purge_pending_desktop_notifications(buf: &mut Vec<MuxNotification>) {
    let mut i = 0;
    while i < buf.len() {
        if let MuxNotification::ClearPendingDesktopNotifications(target_pane) = buf[i] {
            let mut j = 0;
            while j < i {
                let drop_it = matches!(
                    &buf[j],
                    MuxNotification::DesktopNotification { pane_id, .. }
                        if *pane_id == target_pane
                );
                if drop_it {
                    buf.remove(j);
                    i -= 1;
                } else {
                    j += 1;
                }
            }
        }
        i += 1;
    }
}

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
