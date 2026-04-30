//! Unit tests for mux pump helpers.

use oriterm_core::effect::NotificationSource;
use oriterm_mux::{MuxNotification, PaneId};

use oriterm_core::color::Rgb;

use super::{purge_pending_desktop_notifications, resolve_host_color_query};

fn dn(pane_id: PaneId, title: &str) -> MuxNotification {
    MuxNotification::DesktopNotification {
        pane_id,
        source: NotificationSource::Osc9,
        title: title.to_owned(),
        body: String::new(),
    }
}

/// Regression: `[high]` — staging-buffer purge for
/// `ClearPendingDesktopNotifications` must remove preceding
/// `DesktopNotification` entries for the same pane that landed in
/// earlier IO-thread batches and accumulated in `notification_buf`.
#[test]
fn purge_drops_preceding_desktop_notifications_for_same_pane() {
    let pane = PaneId::from_raw(1);
    let mut buf = vec![
        dn(pane, "A"),
        dn(pane, "B"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "C"),
    ];
    purge_pending_desktop_notifications(&mut buf);
    assert_eq!(buf.len(), 2, "got {buf:?}");
    assert!(matches!(
        &buf[0],
        MuxNotification::ClearPendingDesktopNotifications(p) if *p == pane
    ));
    assert!(matches!(
        &buf[1],
        MuxNotification::DesktopNotification { title, .. } if title == "C"
    ));
}

#[test]
fn purge_only_targets_matching_pane() {
    let pane_a = PaneId::from_raw(1);
    let pane_b = PaneId::from_raw(2);
    let mut buf = vec![
        dn(pane_a, "A1"),
        dn(pane_b, "B1"),
        MuxNotification::ClearPendingDesktopNotifications(pane_a),
    ];
    purge_pending_desktop_notifications(&mut buf);
    assert_eq!(buf.len(), 2);
    assert!(matches!(
        &buf[0],
        MuxNotification::DesktopNotification { title, .. } if title == "B1"
    ));
    assert!(matches!(
        &buf[1],
        MuxNotification::ClearPendingDesktopNotifications(p) if *p == pane_a
    ));
}

#[test]
fn purge_handles_multiple_clear_markers() {
    let pane = PaneId::from_raw(1);
    let mut buf = vec![
        dn(pane, "A"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "B"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "C"),
    ];
    purge_pending_desktop_notifications(&mut buf);
    // A dropped (precedes first clear). B dropped (precedes second clear).
    // Both clear markers retained. C survives (follows both clears).
    assert_eq!(buf.len(), 3, "got {buf:?}");
    assert!(matches!(
        &buf[0],
        MuxNotification::ClearPendingDesktopNotifications(_)
    ));
    assert!(matches!(
        &buf[1],
        MuxNotification::ClearPendingDesktopNotifications(_)
    ));
    assert!(matches!(
        &buf[2],
        MuxNotification::DesktopNotification { title, .. } if title == "C"
    ));
}

/// Effect-cutover §01.1 success-criterion 24 canonical-name pin:
/// `ClearPendingDesktopNotifications` purges every staging buffer in
/// the pipeline. Matrix that combines same-pane purge, cross-pane
/// filter, and multi-marker scenarios in one assertion — extends the
/// granular sibling pins (`purge_drops_preceding_desktop_notifications_for_same_pane`,
/// `purge_only_targets_matching_pane`, `purge_handles_multiple_clear_markers`)
/// with an interaction matrix that exercises all three cases together.
#[test]
fn clear_pending_notifications_purges_all_staging_buffers() {
    let pane_a = PaneId::from_raw(1);
    let pane_b = PaneId::from_raw(2);
    let mut buf = vec![
        dn(pane_a, "A1"),
        dn(pane_b, "B1"),
        dn(pane_a, "A2"),
        MuxNotification::ClearPendingDesktopNotifications(pane_a),
        dn(pane_a, "A3"),
        dn(pane_b, "B2"),
        MuxNotification::ClearPendingDesktopNotifications(pane_b),
        dn(pane_b, "B3"),
    ];
    purge_pending_desktop_notifications(&mut buf);

    // pane_a clear at original index 3: drops preceding A1 and A2;
    //   pane_b's B1 untouched (cross-pane filter).
    // pane_b clear at original index 6 (now shifted): drops preceding
    //   B1 and B2; pane_a's A3 untouched.
    // Both clear markers retained; A3 and B3 (post-clear) survive.
    let titles: Vec<String> = buf
        .iter()
        .filter_map(|n| match n {
            MuxNotification::DesktopNotification { title, .. } => Some(title.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        titles,
        vec!["A3".to_string(), "B3".to_string()],
        "only post-clear notifications for each pane must survive"
    );

    let clear_count = buf
        .iter()
        .filter(|n| matches!(n, MuxNotification::ClearPendingDesktopNotifications(_)))
        .count();
    assert_eq!(clear_count, 2, "both clear markers must be retained");
}

// ── BUG-11-013: resolve_host_color_query helper tests ──────────────
//
// The helper resolves OSC 4 / OSC 10 / OSC 11 / OSC 12 color queries
// against the pane's palette snapshot. Layer 1 of the BUG-11-013 TDD
// matrix per `bug-tracker/plans/BUG-11-013/section-03-tdd-matrix.md`.
// VTE dispatch index space:
//   - OSC 4 ; Pn ; ?      → index = u8 (0..=255), per
//                           crates/vte/src/ansi/colors.rs:197-209
//   - OSC 10 ; ?          → index = NamedColor::Foreground = 256
//   - OSC 11 ; ?           → index = NamedColor::Background = 257
//   - OSC 12 ; ?           → index = NamedColor::Cursor    = 258
// Indices 259..=269 (DimBlack..DimForeground) are NOT reachable via
// current OSC dispatch but the helper handles them defensively.

fn palette_with(index: usize, color: [u8; 3]) -> Vec<[u8; 3]> {
    let mut p = vec![[0u8; 3]; 270];
    p[index] = color;
    p
}

/// Regression: BUG-11-013 — semantic pin for OSC 10 returning configured
/// foreground color (NamedColor::Foreground = 256). Before BUG-11-013
/// the consumer arm always returned black.
#[test]
fn resolve_host_color_query_returns_named_foreground_when_index_is_256() {
    let palette = palette_with(256, [0xab, 0xcd, 0xef]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 256);
    assert_eq!(
        result,
        Rgb {
            r: 0xab,
            g: 0xcd,
            b: 0xef
        }
    );
}

/// Regression: BUG-11-013 — OSC 11 (background) query.
#[test]
fn resolve_host_color_query_returns_named_background_when_index_is_257() {
    let palette = palette_with(257, [0x10, 0x20, 0x30]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 257);
    assert_eq!(
        result,
        Rgb {
            r: 0x10,
            g: 0x20,
            b: 0x30
        }
    );
}

/// Regression: BUG-11-013 — OSC 12 (cursor) query.
#[test]
fn resolve_host_color_query_returns_named_cursor_when_index_is_258() {
    let palette = palette_with(258, [0xff, 0xa5, 0x00]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 258);
    assert_eq!(
        result,
        Rgb {
            r: 0xff,
            g: 0xa5,
            b: 0x00
        }
    );
}

/// Regression: BUG-11-013 — OSC 4 indexed query at u8 minimum (boundary).
#[test]
fn resolve_host_color_query_returns_indexed_color_for_osc_4_zero_index() {
    let palette = palette_with(0, [0x00, 0x00, 0x01]); // not literal black, to detect the lookup
    let result = resolve_host_color_query(Some(palette.as_slice()), 0);
    assert_eq!(
        result,
        Rgb {
            r: 0x00,
            g: 0x00,
            b: 0x01
        }
    );
}

/// Regression: BUG-11-013 — OSC 4 indexed query at low index (xterm white).
#[test]
fn resolve_host_color_query_returns_indexed_color_for_osc_4_low_index() {
    let palette = palette_with(7, [0xe5, 0xe5, 0xe5]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 7);
    assert_eq!(
        result,
        Rgb {
            r: 0xe5,
            g: 0xe5,
            b: 0xe5
        }
    );
}

/// Regression: BUG-11-013 — OSC 4 indexed query in the 6×6×6 cube.
#[test]
fn resolve_host_color_query_returns_indexed_color_for_osc_4_mid_index() {
    let palette = palette_with(200, [0x12, 0x34, 0x56]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 200);
    assert_eq!(
        result,
        Rgb {
            r: 0x12,
            g: 0x34,
            b: 0x56
        }
    );
}

/// Regression: BUG-11-013 — OSC 4 indexed query at u8 maximum (boundary
/// at parse_number's u8 cap).
#[test]
fn resolve_host_color_query_returns_indexed_color_for_osc_4_max_index() {
    let palette = palette_with(255, [0xff, 0xff, 0xff]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 255);
    assert_eq!(
        result,
        Rgb {
            r: 0xff,
            g: 0xff,
            b: 0xff
        }
    );
}

/// Regression: BUG-11-013 — defensive pin at the last valid palette
/// slot (NamedColor::DimForeground = 269). Not reachable via current
/// OSC dispatch but the helper must handle it correctly if VTE
/// dispatch ever changes.
#[test]
fn resolve_host_color_query_returns_palette_value_for_index_269() {
    let palette = palette_with(269, [0x80, 0x80, 0x80]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 269);
    assert_eq!(
        result,
        Rgb {
            r: 0x80,
            g: 0x80,
            b: 0x80
        }
    );
}

/// Regression: BUG-11-013 — out-of-range index (one past palette
/// length) falls back to black per Palette::color() contract at
/// `oriterm_core/src/color/palette/mod.rs:286-296`.
#[test]
fn resolve_host_color_query_returns_black_when_index_is_out_of_range() {
    let palette = vec![[0xffu8, 0xff, 0xff]; 270];
    let result = resolve_host_color_query(Some(palette.as_slice()), 270);
    assert_eq!(result, Rgb { r: 0, g: 0, b: 0 });
}

/// Regression: BUG-11-013 — None palette (snapshot unavailable, e.g.
/// pane closed mid-query) falls back to black. Helper owns the None
/// case so callers can use `map_or` without re-stating the fallback.
#[test]
fn resolve_host_color_query_returns_black_when_palette_is_none() {
    let result = resolve_host_color_query(None, 7);
    assert_eq!(result, Rgb { r: 0, g: 0, b: 0 });
}

/// Regression: BUG-11-013 — empty palette slice (defensive — never
/// expected in practice).
#[test]
fn resolve_host_color_query_returns_black_when_palette_is_empty() {
    let palette: Vec<[u8; 3]> = Vec::new();
    let result = resolve_host_color_query(Some(palette.as_slice()), 0);
    assert_eq!(result, Rgb { r: 0, g: 0, b: 0 });
}

/// Regression: BUG-11-013 — negative pin. Configures palette[256] to a
/// non-black color and asserts the helper does NOT return the
/// pre-fix placeholder black. Catches regressions that re-introduce
/// hardcoded `Rgb { r:0, g:0, b:0 }` at the consumer arm.
#[test]
fn resolve_host_color_query_does_not_return_black_for_valid_in_range_index() {
    let palette = palette_with(256, [0x10, 0x20, 0x30]);
    let result = resolve_host_color_query(Some(palette.as_slice()), 256);
    assert_eq!(
        result,
        Rgb {
            r: 0x10,
            g: 0x20,
            b: 0x30
        }
    );
    assert_ne!(
        result,
        Rgb { r: 0, g: 0, b: 0 },
        "must reject placeholder regression"
    );
}
