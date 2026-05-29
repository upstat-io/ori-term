//! Unit tests for mux pump helpers.

use oriterm_core::effect::NotificationSource;
use oriterm_mux::{MuxNotification, PaneId};

use oriterm_core::color::Rgb;

use super::notification_purge::purge_pending_desktop_notifications;
use super::resolve_host_color_query;

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
    // pane_b's B1 untouched (cross-pane filter).
    // pane_b clear at original index 6 (now shifted): drops preceding
    // B1 and B2; pane_a's A3 untouched.
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

// ── : resolve_host_color_query helper tests ──────────────
// The helper resolves OSC 4 / OSC 10 / OSC 11 / OSC 12 color queries
// against the pane's palette snapshot. Layer 1 of the TDD
// matrix per .
// VTE dispatch index space:
// - OSC 4 ; Pn ; ? → index = u8 (0..=255), per
// crates/vte/src/ansi/colors.rs:197-209
// - OSC 10 ; ? → index = NamedColor::Foreground = 256
// - OSC 11 ; ? → index = NamedColor::Background = 257
// - OSC 12 ; ? → index = NamedColor::Cursor = 258
// Indices 259..=269 (DimBlack..DimForeground) are NOT reachable via
// current OSC dispatch but the helper handles them defensively.

fn palette_with(index: usize, color: [u8; 3]) -> Vec<[u8; 3]> {
    let mut p = vec![[0u8; 3]; 270];
    p[index] = color;
    p
}

/// Regression: property for OSC 10 returning configured
/// foreground color (NamedColor::Foreground = 256). Before
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

/// Regression: OSC 11 (background) query.
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

/// Regression: OSC 12 (cursor) query.
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

/// Regression: OSC 4 indexed query at u8 minimum (boundary).
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

/// Regression: OSC 4 indexed query at low index (xterm white).
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

/// Regression: OSC 4 indexed query in the 6×6×6 cube.
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

/// Regression: OSC 4 indexed query at u8 maximum (boundary
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

/// Regression: defensive pin at the last valid palette
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

/// Regression: out-of-range index (one past palette
/// length) falls back to black per Palette::color() contract at
/// `oriterm_core/src/color/palette/mod.rs:286-296`.
#[test]
fn resolve_host_color_query_returns_black_when_index_is_out_of_range() {
    let palette = vec![[0xffu8, 0xff, 0xff]; 270];
    let result = resolve_host_color_query(Some(palette.as_slice()), 270);
    assert_eq!(result, Rgb { r: 0, g: 0, b: 0 });
}

/// Regression: None palette (snapshot unavailable, e.g.
/// pane closed mid-query) falls back to black. Helper owns the None
/// case so callers can use `map_or` without re-stating the fallback.
#[test]
fn resolve_host_color_query_returns_black_when_palette_is_none() {
    let result = resolve_host_color_query(None, 7);
    assert_eq!(result, Rgb { r: 0, g: 0, b: 0 });
}

/// Regression: empty palette slice (defensive — never
/// expected in practice).
#[test]
fn resolve_host_color_query_returns_black_when_palette_is_empty() {
    let palette: Vec<[u8; 3]> = Vec::new();
    let result = resolve_host_color_query(Some(palette.as_slice()), 0);
    assert_eq!(result, Rgb { r: 0, g: 0, b: 0 });
}

/// Regression: regression guard. Configures palette[256] to a
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

// -- apply_bell_focus_decision regression pins --
// The PaneBell + DesktopNotification arms each gate `set_bell` /
// `clear_bell` on `is_pane_in_focused_tab`. The decision step is
// extracted as `apply_bell_focus_decision` so the gate composition
// (focus decision × per-pane mux mutation) has a behavioral pin
// without requiring a full App fixture. Exercised against a real
// `EmbeddedMux` because EmbeddedMux's set_bell / clear_bell only
// mutate `bell_panes` (no spawned-pane requirement).
// Naming pinned per the SSOT mapping:
// focused-tab input ⇒ clear_bell only (silent bell convention)
// background-tab input ⇒ set_bell only (light the tab indicator)
// These are the load-bearing cells the bell-icon-on-focused-tab
// regression would have to break. Either branch firing the wrong
// method (or both methods) would fail one of the asserts below.

use std::sync::Arc;

use oriterm_mux::EmbeddedMux;
use oriterm_mux::backend::MuxBackend;

use super::apply_bell_focus_decision;

#[test]
fn apply_bell_focus_decision_focused_tab_clears_only() {
    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    let pane = PaneId::from_raw(1);

    // Pre-state: bell already set so we can pin "clear takes effect".
    mux.set_bell(pane);
    assert!(mux.has_bell(pane), "precondition: bell pre-set");

    apply_bell_focus_decision(true, &mut mux, pane);

    assert!(
        !mux.has_bell(pane),
        "focused-tab input must call clear_bell — pinned against \
 a regression that swapped the branches",
    );
}

#[test]
fn apply_bell_focus_decision_background_tab_sets_only() {
    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    let pane = PaneId::from_raw(1);

    assert!(!mux.has_bell(pane), "precondition: bell not set");

    apply_bell_focus_decision(false, &mut mux, pane);

    assert!(
        mux.has_bell(pane),
        "background-tab input must call set_bell — pinned against \
 a regression that suppressed the persistent tab indicator",
    );
}

#[test]
fn apply_bell_focus_decision_focused_overrides_pre_existing_bell() {
    // The PaneBell arm receives the notification AFTER the IO thread
    // has emitted it. If a previous burst left the bell set and the
    // pane is NOW focused, the focused-tab branch MUST clear — not
    // leave the stale bell. Pinned against an active-pane-only
    // regression that read the bell as "already silenced, do nothing".
    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    let pane = PaneId::from_raw(7);
    mux.set_bell(pane);

    apply_bell_focus_decision(true, &mut mux, pane);

    assert!(
        !mux.has_bell(pane),
        "focused-tab branch must clear pre-existing stale bell",
    );
}

#[test]
fn apply_bell_focus_decision_background_idempotent_under_repeat() {
    // PaneBell can fire multiple times for repeated bell escapes.
    // Background-tab → set_bell idempotent (set on already-set pane
    // stays set; pin defends against a regression that toggled).
    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    let pane = PaneId::from_raw(2);

    apply_bell_focus_decision(false, &mut mux, pane);
    apply_bell_focus_decision(false, &mut mux, pane);
    apply_bell_focus_decision(false, &mut mux, pane);

    assert!(
        mux.has_bell(pane),
        "set_bell on already-set pane must stay set (idempotent)",
    );
}

#[test]
fn apply_bell_focus_decision_does_not_mutate_unrelated_panes() {
    // The decision targets ONE pane id. A regression that drained or
    // bulk-mutated bell_panes for a focused-tab decision would fail
    // this clamp — the unrelated background pane keeps its bell.
    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    let focused_pane = PaneId::from_raw(1);
    let unrelated_pane = PaneId::from_raw(99);
    mux.set_bell(unrelated_pane);

    apply_bell_focus_decision(true, &mut mux, focused_pane);

    assert!(
        mux.has_bell(unrelated_pane),
        "clear_bell on focused_pane must not touch unrelated pane",
    );
    assert!(
        !mux.has_bell(focused_pane),
        "focused_pane bell IS cleared (positive pair to the unrelated assertion)",
    );
}

// -- pump_mux_events_core regression pins --
// `pump_mux_events_core` carves the side-effect protocol out of
// `App::pump_mux_events` so the gate check + daemon-connectivity check +
// `poll_events → drain_notifications → empty-check → purge` ordering can
// be tested against a recording `MuxBackend` without an `App` fixture.
// `RecordingMuxBackend` implements only the 5 methods `pump_mux_events_core`
// actually calls; every other trait method is `unimplemented!()` because
// reaching it would mean the helper deviated from its documented protocol.

use std::cell::RefCell;
use std::io;
use std::sync::mpsc::Sender;

use oriterm_core::{CursorShape, Palette, RenderableContent, Selection, Theme as CoreTheme};
use oriterm_mux::in_process::ClosePaneResult;
use oriterm_mux::mux_event::MuxEvent;
use oriterm_mux::{DomainId, HostReply, ImageConfig, PaneEntry, PaneSnapshot, SpawnConfig};

use super::{PumpResult, pump_mux_events_core};

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordedCall {
    IsDaemonMode,
    IsConnected,
    HasPendingWakeup,
    PollEvents,
    DrainNotifications,
}

struct RecordingMuxBackend {
    is_daemon_mode_value: bool,
    is_connected_value: bool,
    has_pending_wakeup_value: bool,
    drain_returns: Vec<MuxNotification>,
    calls: RefCell<Vec<RecordedCall>>,
}

impl RecordingMuxBackend {
    fn new() -> Self {
        Self {
            is_daemon_mode_value: false,
            is_connected_value: true,
            has_pending_wakeup_value: false,
            drain_returns: Vec::new(),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<RecordedCall> {
        self.calls.borrow().clone()
    }
}

#[expect(
    clippy::unimplemented,
    reason = "RecordingMuxBackend implements only the 5 methods pump_mux_events_core calls; \
 reaching any other method means the helper deviated from its protocol"
)]
impl MuxBackend for RecordingMuxBackend {
    fn is_daemon_mode(&self) -> bool {
        self.calls.borrow_mut().push(RecordedCall::IsDaemonMode);
        self.is_daemon_mode_value
    }
    fn is_connected(&self) -> bool {
        self.calls.borrow_mut().push(RecordedCall::IsConnected);
        self.is_connected_value
    }
    fn has_pending_wakeup(&self) -> bool {
        self.calls.borrow_mut().push(RecordedCall::HasPendingWakeup);
        self.has_pending_wakeup_value
    }
    fn poll_events(&mut self) {
        self.calls.borrow_mut().push(RecordedCall::PollEvents);
    }
    fn drain_notifications(&mut self, out: &mut Vec<MuxNotification>) {
        self.calls
            .borrow_mut()
            .push(RecordedCall::DrainNotifications);
        // Match production semantics: InProcessMux::drain_notifications
        // (`oriterm_mux/src/in_process/event_pump.rs`) clears the caller's
        // buffer THEN swaps in the new notifications. A naive `extend`
        // would let pre-existing entries leak through, masking real
        // production behavior.
        out.clear();
        out.extend(self.drain_returns.drain(..));
    }

    fn discard_notifications(&mut self) {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call discard_notifications"
        )
    }
    fn get_pane_entry(&self, _: PaneId) -> Option<PaneEntry> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call get_pane_entry")
    }
    fn spawn_pane(&mut self, _: &SpawnConfig, _: CoreTheme) -> io::Result<PaneId> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call spawn_pane")
    }
    fn close_pane(&mut self, _: PaneId) -> ClosePaneResult {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call close_pane")
    }
    fn resize_pane_grid(&mut self, _: PaneId, _: u16, _: u16) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call resize_pane_grid")
    }
    fn pane_mode(&self, _: PaneId) -> Option<u64> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call pane_mode")
    }
    fn pane_dec_locator_active(&self, _: PaneId) -> bool {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call pane_dec_locator_active"
        )
    }
    fn set_pane_theme(&mut self, _: PaneId, _: CoreTheme, _: Palette) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call set_pane_theme")
    }
    fn set_cursor_shape(&mut self, _: PaneId, _: CursorShape) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call set_cursor_shape")
    }
    fn set_bold_is_bright(&mut self, _: PaneId, _: bool) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call set_bold_is_bright")
    }
    fn set_answerback(&mut self, _: PaneId, _: Vec<u8>) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call set_answerback")
    }
    fn mark_all_dirty(&mut self, _: PaneId) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call mark_all_dirty")
    }
    fn set_image_config(&mut self, _: PaneId, _: ImageConfig) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call set_image_config")
    }
    fn set_cell_dimensions(&mut self, _: PaneId, _: u16, _: u16) {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call set_cell_dimensions"
        )
    }
    fn scroll_display(&mut self, _: PaneId, _: isize) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call scroll_display")
    }
    fn scroll_to_bottom(&mut self, _: PaneId) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call scroll_to_bottom")
    }
    fn scroll_to_previous_prompt(&mut self, _: PaneId) {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call scroll_to_previous_prompt"
        )
    }
    fn scroll_to_next_prompt(&mut self, _: PaneId) {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call scroll_to_next_prompt"
        )
    }
    fn open_search(&mut self, _: PaneId) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call open_search")
    }
    fn close_search(&mut self, _: PaneId) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call close_search")
    }
    fn search_set_query(&mut self, _: PaneId, _: String) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call search_set_query")
    }
    fn search_next_match(&mut self, _: PaneId) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call search_next_match")
    }
    fn search_prev_match(&mut self, _: PaneId) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call search_prev_match")
    }
    fn is_search_active(&self, _: PaneId) -> bool {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call is_search_active")
    }
    fn extract_text(&mut self, _: PaneId, _: &Selection) -> Option<String> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call extract_text")
    }
    fn extract_html(
        &mut self,
        _: PaneId,
        _: &Selection,
        _: &str,
        _: f32,
    ) -> Option<(String, String)> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call extract_html")
    }
    fn send_input(&mut self, _: PaneId, _: &[u8]) {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call send_input")
    }
    fn pane_ids(&self) -> Vec<PaneId> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call pane_ids")
    }
    fn event_tx(&self) -> Option<&Sender<MuxEvent>> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call event_tx")
    }
    fn default_domain(&self) -> DomainId {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call default_domain")
    }
    fn pane_snapshot(&self, _: PaneId) -> Option<&PaneSnapshot> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call pane_snapshot")
    }
    fn is_pane_snapshot_dirty(&self, _: PaneId) -> bool {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call is_pane_snapshot_dirty"
        )
    }
    fn refresh_pane_snapshot(&mut self, _: PaneId) -> Option<&PaneSnapshot> {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call refresh_pane_snapshot"
        )
    }
    fn sync_pane_snapshot(&mut self, _: PaneId) -> Option<PaneSnapshot> {
        unimplemented!("RecordingMuxBackend: pump_mux_events_core does not call sync_pane_snapshot")
    }
    fn clear_pane_snapshot_dirty(&mut self, _: PaneId) {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call clear_pane_snapshot_dirty"
        )
    }
    fn swap_renderable_content(&mut self, _: PaneId, _: &mut RenderableContent) -> bool {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call swap_renderable_content"
        )
    }
    fn fulfill_host_request(&mut self, _: PaneId, _: HostReply) -> io::Result<()> {
        unimplemented!(
            "RecordingMuxBackend: pump_mux_events_core does not call fulfill_host_request"
        )
    }
}

// -- Decision-matrix coverage --

/// Regression: None mux short-circuits to NoMux without
/// touching any backend trait method.

#[test]
fn pump_mux_events_core_mux_is_none_returns_no_mux() {
    let mut buf: Vec<MuxNotification> = Vec::new();
    let result = pump_mux_events_core(None, &mut buf);
    assert_eq!(result, PumpResult::NoMux);
    assert!(
        buf.is_empty(),
        "buffer must be untouched; len={}",
        buf.len()
    );
}

/// Regression: daemon-mode disconnected backend returns
/// DaemonDisconnect after recording IsDaemonMode + IsConnected only.
#[test]
fn pump_mux_events_core_daemon_disconnected_returns_daemon_disconnect() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = false;
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::DaemonDisconnect);
    assert!(buf.is_empty());
    let calls = backend.calls();
    assert_eq!(
        calls,
        vec![RecordedCall::IsDaemonMode, RecordedCall::IsConnected]
    );
}

/// Regression: daemon-mode connected backend with closed gate
/// returns NoPendingWakeup.
#[test]
fn pump_mux_events_core_daemon_connected_no_wakeup_returns_no_pending_wakeup() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = true;
    backend.has_pending_wakeup_value = false;
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::NoPendingWakeup);
    assert!(buf.is_empty());
}

/// Regression: embedded backend never consults is_connected
/// (daemon-disconnect arm short-circuits via is_daemon_mode==false).
#[test]
fn pump_mux_events_core_embedded_no_wakeup_returns_no_pending_wakeup() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = false;
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::NoPendingWakeup);
    let calls = backend.calls();
    assert!(
        !calls.contains(&RecordedCall::IsConnected),
        "embedded mode must not consult is_connected; recorded: {calls:?}"
    );
}

/// Regression: embedded mode with `is_connected=false` and
/// gate closed still returns NoPendingWakeup; the daemon-disconnect arm
/// short-circuits via is_daemon_mode==false BEFORE is_connected is consulted.
#[test]
fn pump_mux_events_core_embedded_disconnected_value_irrelevant_returns_no_pending_wakeup() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.is_connected_value = false; // would matter if daemon mode
    backend.has_pending_wakeup_value = false;
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::NoPendingWakeup);
}

/// Regression: embedded mode, gate open, drain produces
/// zero notifications → EmptyDrain. Buffer untouched after drain.
#[test]
fn pump_mux_events_core_embedded_gate_open_drain_empty_returns_empty_drain() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = Vec::new();
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::EmptyDrain);
    assert!(buf.is_empty());
    let calls = backend.calls();
    assert!(calls.contains(&RecordedCall::PollEvents));
    assert!(calls.contains(&RecordedCall::DrainNotifications));
    assert!(
        !calls.contains(&RecordedCall::IsConnected),
        "embedded must not consult is_connected; recorded: {calls:?}"
    );
}

/// Regression: daemon mode, gate open, drain produces zero
/// notifications → EmptyDrain. Pins daemon-mode call sequence.
#[test]
fn pump_mux_events_core_daemon_gate_open_drain_empty_returns_empty_drain() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = true;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = Vec::new();
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::EmptyDrain);
    let calls = backend.calls();
    assert_eq!(
        calls,
        vec![
            RecordedCall::IsDaemonMode,
            RecordedCall::IsConnected,
            RecordedCall::HasPendingWakeup,
            RecordedCall::PollEvents,
            RecordedCall::DrainNotifications,
        ]
    );
}

/// Regression: embedded mode, drain yields one notification
/// → HasNotifications. Buffer carries the notification.
#[test]
fn pump_mux_events_core_embedded_gate_open_drain_yields_one_returns_has_notifications() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = vec![MuxNotification::PaneOutput(PaneId::from_raw(1))];
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::HasNotifications);
    assert_eq!(buf.len(), 1);
    let calls = backend.calls();
    assert!(
        !calls.contains(&RecordedCall::IsConnected),
        "embedded must not consult is_connected; recorded: {calls:?}"
    );
}

/// Regression: daemon mode, drain yields one notification
/// → HasNotifications. is_connected MUST appear in call sequence between
/// is_daemon_mode and has_pending_wakeup.
#[test]
fn pump_mux_events_core_daemon_gate_open_drain_yields_one_returns_has_notifications() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = true;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = vec![MuxNotification::PaneOutput(PaneId::from_raw(1))];
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::HasNotifications);
    assert_eq!(buf.len(), 1);
    let calls = backend.calls();
    let is_daemon_idx = calls.iter().position(|c| *c == RecordedCall::IsDaemonMode);
    let is_connected_idx = calls.iter().position(|c| *c == RecordedCall::IsConnected);
    let has_pending_idx = calls
        .iter()
        .position(|c| *c == RecordedCall::HasPendingWakeup);
    assert!(
        is_daemon_idx.is_some()
            && is_connected_idx.is_some()
            && has_pending_idx.is_some()
            && is_daemon_idx < is_connected_idx
            && is_connected_idx < has_pending_idx,
        "daemon-mode sequence must be IsDaemonMode → IsConnected → HasPendingWakeup; got: {calls:?}"
    );
}

/// Regression: embedded mode, drain yields multiple
/// notifications including a ClearPendingDesktopNotifications mid-batch
/// → HasNotifications. Purge runs; preceding DesktopNotifications dropped.
#[test]
fn pump_mux_events_core_embedded_gate_open_drain_yields_many_returns_has_notifications() {
    let pane = PaneId::from_raw(1);
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = vec![
        dn(pane, "A"),
        dn(pane, "B"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "C"),
    ];
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::HasNotifications);
    // Purge ran: A and B (preceding the clear) dropped; clear marker + C survive.
    assert_eq!(buf.len(), 2, "got {buf:?}");
    assert!(matches!(
        &buf[0],
        MuxNotification::ClearPendingDesktopNotifications(_)
    ));
    assert!(matches!(
    &buf[1],
    MuxNotification::DesktopNotification { title, .. } if title == "C"
    ));
}

/// Regression: daemon mode, drain yields multiple
/// notifications with mid-batch clear marker → HasNotifications. Pins
/// that purge runs in daemon mode and that is_connected appears in the
/// recorded sequence.
#[test]
fn pump_mux_events_core_daemon_gate_open_drain_yields_many_returns_has_notifications() {
    let pane = PaneId::from_raw(1);
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = true;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = vec![
        dn(pane, "A"),
        MuxNotification::ClearPendingDesktopNotifications(pane),
        dn(pane, "B"),
    ];
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::HasNotifications);
    assert_eq!(buf.len(), 2);
    assert!(matches!(
        &buf[0],
        MuxNotification::ClearPendingDesktopNotifications(_)
    ));
    assert!(matches!(
    &buf[1],
    MuxNotification::DesktopNotification { title, .. } if title == "B"
    ));
    let calls = backend.calls();
    let idx = calls.iter().position(|c| *c == RecordedCall::IsConnected);
    assert!(
        idx.is_some(),
        "daemon mode MUST consult is_connected; recorded: {calls:?}"
    );
}

// -- Order pin --

/// Regression: embedded happy-path call sequence.
/// Reordering ANY pair (e.g., draining before polling) fails.
#[test]
fn pump_mux_events_core_embedded_mode_records_calls_in_canonical_order() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = vec![MuxNotification::PaneOutput(PaneId::from_raw(1))];
    let mut buf = Vec::new();
    let _ = pump_mux_events_core(Some(&mut backend), &mut buf);
    let calls = backend.calls();
    assert_eq!(
        calls,
        vec![
            RecordedCall::IsDaemonMode,
            RecordedCall::HasPendingWakeup,
            RecordedCall::PollEvents,
            RecordedCall::DrainNotifications,
        ]
    );
}

/// Regression: daemon happy-path call sequence with the
/// extra IsConnected step between IsDaemonMode and HasPendingWakeup.
#[test]
fn pump_mux_events_core_daemon_mode_records_calls_in_canonical_order() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = true;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = vec![MuxNotification::PaneOutput(PaneId::from_raw(1))];
    let mut buf = Vec::new();
    let _ = pump_mux_events_core(Some(&mut backend), &mut buf);
    let calls = backend.calls();
    assert_eq!(
        calls,
        vec![
            RecordedCall::IsDaemonMode,
            RecordedCall::IsConnected,
            RecordedCall::HasPendingWakeup,
            RecordedCall::PollEvents,
            RecordedCall::DrainNotifications,
        ]
    );
}

// -- Negative pins --

/// Regression: None mux records ZERO calls and leaves buf empty.
#[test]
fn pump_mux_events_core_no_mux_records_zero_calls() {
    let mut buf = Vec::new();
    let result = pump_mux_events_core(None, &mut buf);
    assert_eq!(result, PumpResult::NoMux);
    assert!(buf.is_empty());
}

/// Regression: daemon-disconnect skips poll_events,
/// drain_notifications, AND has_pending_wakeup (the entire post-gate
/// branch). Pins the EXACT short-circuit boundary.
#[test]
fn pump_mux_events_core_daemon_disconnect_skips_poll_and_drain() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = false;
    backend.has_pending_wakeup_value = true; // value irrelevant — must not be consulted
    let mut buf = Vec::new();
    let _ = pump_mux_events_core(Some(&mut backend), &mut buf);
    let calls = backend.calls();
    assert!(!calls.contains(&RecordedCall::HasPendingWakeup));
    assert!(!calls.contains(&RecordedCall::PollEvents));
    assert!(!calls.contains(&RecordedCall::DrainNotifications));
}

/// Regression: embedded gate-closed records exactly
/// IsDaemonMode + HasPendingWakeup; no PollEvents or DrainNotifications.
#[test]
fn pump_mux_events_core_embedded_no_pending_wakeup_skips_poll_and_drain() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = false;
    let mut buf = Vec::new();
    let _ = pump_mux_events_core(Some(&mut backend), &mut buf);
    let calls = backend.calls();
    assert_eq!(
        calls,
        vec![RecordedCall::IsDaemonMode, RecordedCall::HasPendingWakeup]
    );
}

/// Regression: daemon-connected gate-closed records
/// IsDaemonMode + IsConnected + HasPendingWakeup; no PollEvents or DrainNotifications.
#[test]
fn pump_mux_events_core_daemon_connected_no_pending_wakeup_skips_poll_and_drain() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = true;
    backend.has_pending_wakeup_value = false;
    let mut buf = Vec::new();
    let _ = pump_mux_events_core(Some(&mut backend), &mut buf);
    let calls = backend.calls();
    assert_eq!(
        calls,
        vec![
            RecordedCall::IsDaemonMode,
            RecordedCall::IsConnected,
            RecordedCall::HasPendingWakeup
        ]
    );
}

// -- Precedence pins --

/// Regression: None mux short-circuits regardless of any
/// hypothetical other field state. The early-return at the
/// `Option<&mut dyn MuxBackend>` match arm fires unconditionally.
#[test]
fn pump_mux_events_core_no_mux_short_circuits_other_fields() {
    let mut buf = Vec::new();
    let result = pump_mux_events_core(None, &mut buf);
    assert_eq!(result, PumpResult::NoMux);
}

/// Regression: daemon-disconnect short-circuits BEFORE the
/// gate check, even when has_pending_wakeup would also return false.
/// Pins that the daemon-disconnect early-return fires first; swapping
/// the two early-return blocks would not fail any other test.
#[test]
fn pump_mux_events_core_daemon_disconnect_short_circuits_pending_wakeup() {
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = false;
    backend.has_pending_wakeup_value = false; // would also yield NoPendingWakeup
    let mut buf = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(
        result,
        PumpResult::DaemonDisconnect,
        "daemon-disconnect arm MUST fire before the gate check"
    );
}

// -- Buffer-state matrix --

/// Regression: pre-populated notification_buf is preserved when mux=None
/// (early-exit at NoMux must not clear or mutate the buf — production
/// invariant: caller owns buffer lifecycle).
#[test]
fn pump_mux_events_core_no_mux_preserves_pre_existing_buf() {
    let pane = PaneId::from_raw(99);
    let mut buf = vec![dn(pane, "PRE_EXISTING")];
    let result = pump_mux_events_core(None, &mut buf);
    assert_eq!(result, PumpResult::NoMux);
    assert_eq!(
        buf.len(),
        1,
        "buf must retain its 1 pre-existing entry; got len={}",
        buf.len()
    );
    assert!(matches!(
    &buf[0],
    MuxNotification::DesktopNotification { title, .. } if title == "PRE_EXISTING"
    ));
}

/// Regression: pre-populated notification_buf is preserved when gate is
/// closed (NoPendingWakeup early-exit must not clear or mutate buf).
#[test]
fn pump_mux_events_core_no_pending_wakeup_preserves_pre_existing_buf() {
    let pane = PaneId::from_raw(99);
    let mut buf = vec![dn(pane, "PRE_EXISTING")];
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = false;
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::NoPendingWakeup);
    assert_eq!(buf.len(), 1, "buf preserved on gate-closed early-exit");
    assert!(matches!(
    &buf[0],
    MuxNotification::DesktopNotification { title, .. } if title == "PRE_EXISTING"
    ));
}

/// Regression: pre-populated notification_buf is REPLACED (not
/// appended-to) when drain runs and yields entries. Pins the
/// production invariant from `oriterm_mux/src/in_process/event_pump.rs`
/// `drain_notifications`: `out.clear(); std::mem::swap(...)`. Pre-
/// existing entries are LOST — only post-drain entries survive.
#[test]
fn pump_mux_events_core_drain_replaces_pre_existing_buf() {
    let pane = PaneId::from_raw(99);
    let mut buf = vec![dn(pane, "PRE_EXISTING")];
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = vec![dn(pane, "FROM_DRAIN")];
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::HasNotifications);
    // PRE_EXISTING was cleared by drain_notifications's `out.clear()`;
    // only FROM_DRAIN survives. Defends against a regression that
    // would let pre-existing entries leak through.
    assert_eq!(buf.len(), 1, "drain replaced pre-existing buf; got {buf:?}");
    assert!(matches!(
    &buf[0],
    MuxNotification::DesktopNotification { title, .. } if title == "FROM_DRAIN"
    ));
}

/// Regression: pre-populated notification_buf is preserved when daemon-
/// disconnect arm fires (drain not called → buf untouched).
#[test]
fn pump_mux_events_core_daemon_disconnect_preserves_pre_existing_buf() {
    let pane = PaneId::from_raw(99);
    let mut buf = vec![dn(pane, "PRE_EXISTING")];
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = true;
    backend.is_connected_value = false;
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::DaemonDisconnect);
    assert_eq!(buf.len(), 1, "daemon-disconnect early-exit preserves buf");
    assert!(matches!(
    &buf[0],
    MuxNotification::DesktopNotification { title, .. } if title == "PRE_EXISTING"
    ));
}

/// Regression: EmptyDrain branch — gate open, drain runs and yields
/// zero entries. The pre-existing PRE_EXISTING is LOST because
/// drain_notifications cleared the buf before swap (production semantics).
/// Buf is empty post-call, returns PumpResult::EmptyDrain.
#[test]
fn pump_mux_events_core_empty_drain_clears_pre_existing_buf() {
    let pane = PaneId::from_raw(99);
    let mut buf = vec![dn(pane, "PRE_EXISTING")];
    let mut backend = RecordingMuxBackend::new();
    backend.is_daemon_mode_value = false;
    backend.has_pending_wakeup_value = true;
    backend.drain_returns = Vec::new();
    let result = pump_mux_events_core(Some(&mut backend), &mut buf);
    assert_eq!(result, PumpResult::EmptyDrain);
    assert!(
        buf.is_empty(),
        "drain_notifications cleared pre-existing buf; got {buf:?}"
    );
}
