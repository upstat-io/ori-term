//! Server-side snapshot wiring tests.
//!
//! Pins the `RenderableContent → PaneSnapshot` wire mapping without
//! needing a full [`crate::pane::Pane`]. The `Pane`-derived half lives in
//! [`super::fill_pane_metadata`] and is covered by pane/backend tests;
//! here we verify the `RenderableContent`-derived half only.

use oriterm_core::RenderableContent;
use oriterm_core::effect::sink::VoidEffectSink;
use oriterm_core::{Term, Theme};
use vte::ansi::Processor;
use vte::ansi::cursor_icon::CursorIcon;

use super::fill_wire_metadata_from_renderable;
use crate::PaneSnapshot;
use crate::protocol::encode_cursor_icon;

/// Section 10.5 server-side pin: firing OSC 22 at the `Term`,
/// producing a `RenderableContent` via `renderable_content_into`, and
/// running it through the server-side wire mapper must surface the
/// encoded `mouse_cursor_icon` on the outgoing `PaneSnapshot`. Without
/// this pin, a regression at `fill_wire_metadata_from_renderable` that
/// silently drops the icon encoding would not be caught until client
/// renderers noticed a missing cursor change.
#[test]
fn osc22_daemon_snapshot_carries_cursor_icon() {
    let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
    let mut processor: Processor = Processor::new();
    processor.advance(&mut term, b"\x1b]22;pointer\x1b\\");
    assert_eq!(
        term.mouse_cursor_icon(),
        Some(CursorIcon::Pointer),
        "sanity: OSC 22 must land on Term::mouse_cursor_icon"
    );

    let mut render_buf = RenderableContent::default();
    term.renderable_content_into(&mut render_buf);
    assert_eq!(
        render_buf.mouse_cursor_icon,
        Some(CursorIcon::Pointer),
        "sanity: RenderableContent must carry the icon from Term"
    );

    let mut snapshot = PaneSnapshot::default();
    fill_wire_metadata_from_renderable(&render_buf, &mut snapshot);

    let expected =
        encode_cursor_icon(CursorIcon::Pointer).expect("Pointer must be wire-transportable");
    assert_eq!(
        snapshot.mouse_cursor_icon,
        Some(expected),
        "PaneSnapshot::mouse_cursor_icon must be the encoded wire index"
    );
}

/// Regression guard: when no OSC 22 fired, `Term::mouse_cursor_icon` stays
/// `None` and the wire field is also `None`. Guards against a future
/// edit that writes a default encoding (e.g. `Some(0)`) instead of the
/// correct absence.
#[test]
fn daemon_snapshot_no_osc22_yields_none() {
    let term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

    let mut render_buf = RenderableContent::default();
    term.renderable_content_into(&mut render_buf);
    assert_eq!(render_buf.mouse_cursor_icon, None);

    let mut snapshot = PaneSnapshot::default();
    fill_wire_metadata_from_renderable(&render_buf, &mut snapshot);
    assert_eq!(snapshot.mouse_cursor_icon, None);
}

/// Matrix pin: every icon in the project-owned wire slice must
/// round-trip through the server-side wire mapping with a stable u8
/// index. A regression here would be silent drift between
/// `OSC22_KNOWN_ICONS` and the server encoding path.
#[test]
fn daemon_snapshot_encodes_every_wire_icon() {
    use crate::protocol::OSC22_KNOWN_ICONS;

    let mut count = 0;
    for (expected_idx, &icon) in OSC22_KNOWN_ICONS.iter().enumerate() {
        let mut render_buf = RenderableContent::default();
        render_buf.mouse_cursor_icon = Some(icon);

        let mut snapshot = PaneSnapshot::default();
        fill_wire_metadata_from_renderable(&render_buf, &mut snapshot);

        let expected_u8 = u8::try_from(expected_idx).expect("OSC22_KNOWN_ICONS index fits in u8");
        assert_eq!(
            snapshot.mouse_cursor_icon,
            Some(expected_u8),
            "icon {icon:?} must encode to wire index {expected_idx}"
        );
        count += 1;
    }
    assert_eq!(
        count,
        OSC22_KNOWN_ICONS.len(),
        "matrix must visit every entry in OSC22_KNOWN_ICONS"
    );
}
