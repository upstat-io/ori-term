//! Tests for cell-metric broadcast logic.
//!
//! Covers the pure `cell_metric_broadcast_needed` helper, the stateful
//! `try_claim_broadcast` cache, and the `collect_window_pane_ids`
//! session-enumeration helper. These three are the testable layers of
//! `App::broadcast_cell_metrics_to_window`; the mux dispatch and
//! IO-thread handler are tested in `oriterm_mux` (see
//! `backend::embedded::tests` and `pane::io_thread::tests`).
//!
//! See: §07.N

use super::{cell_metric_broadcast_needed, collect_window_pane_ids, try_claim_broadcast};

/// First broadcast (no prior state) must fire.
#[test]
fn first_broadcast_always_fires() {
 assert!(cell_metric_broadcast_needed(None, (8, 16)));
 assert!(cell_metric_broadcast_needed(None, (16, 32)));
}

/// Identical dims short-circuit.
#[test]
fn identical_dims_short_circuit() {
 assert!(!cell_metric_broadcast_needed(Some((8, 16)), (8, 16)));
 assert!(!cell_metric_broadcast_needed(Some((16, 32)), (16, 32)));
}

/// Width change fires a broadcast.
#[test]
fn width_change_fires() {
 assert!(cell_metric_broadcast_needed(Some((8, 16)), (10, 16)));
}

/// Height change fires a broadcast.
#[test]
fn height_change_fires() {
 assert!(cell_metric_broadcast_needed(Some((8, 16)), (8, 20)));
}

/// Both-axis change fires a broadcast.
#[test]
fn both_axis_change_fires() {
 assert!(cell_metric_broadcast_needed(Some((8, 16)), (16, 32)));
}

/// Regression guard: any non-None equal value must NOT fire, even for
/// degenerate `(1, 1)` dims that happen after a font-size clamp.
#[test]
fn negative_pin_degenerate_dims_short_circuit() {
 assert!(!cell_metric_broadcast_needed(Some((1, 1)), (1, 1)));
}

/// Regression: a font-size change that does NOT alter grid cols/rows
/// must still trigger a cell-metric broadcast. This pins the decision
/// helper: changed pixel dims fire even when grid cols/rows are constant.
/// See: §07.N
/// The full broadcast fanout (`broadcast_cell_metrics_to_window` →
/// `mux.set_cell_dimensions` for all panes) is verified at the mux
/// level by `both_split_panes_receive_updated_metrics_after_font_change`
/// in `oriterm_mux::backend::embedded::tests`.
#[test]
fn font_size_change_without_grid_change_still_fires_broadcast() {
 // Simulate: grid stays at 80x24, but font changed 8x16 → 10x20.
 let prior = Some((8, 16));
 assert!(
 cell_metric_broadcast_needed(prior, (10, 20)),
 "cell metric change (font-size change) must fire even when grid cols/rows are unchanged"
 );
}

// ── try_claim_broadcast: decision + state update fused ────────────

/// First broadcast (cache is None) claims the slot AND updates the cache.
/// Pins that `try_claim_broadcast` does the state assignment — a
/// refactor that removes `*cached = Some(new)` fails this test.
#[test]
fn try_claim_broadcast_updates_cache_on_first_call() {
 let mut cached = None;
 assert!(try_claim_broadcast(&mut cached, (8, 16)));
 assert_eq!(
 cached,
 Some((8, 16)),
 "cache must be updated to the claimed dims"
 );
}

/// Second call with identical dims short-circuits AND leaves cache intact.
#[test]
fn try_claim_broadcast_short_circuits_and_leaves_cache() {
 let mut cached = Some((8, 16));
 assert!(!try_claim_broadcast(&mut cached, (8, 16)));
 assert_eq!(
 cached,
 Some((8, 16)),
 "cache must be unchanged on short-circuit"
 );
}

/// Changed dims claim the slot and UPDATE the cache to the new value.
#[test]
fn try_claim_broadcast_updates_cache_on_change() {
 let mut cached = Some((8, 16));
 assert!(try_claim_broadcast(&mut cached, (16, 32)));
 assert_eq!(cached, Some((16, 32)), "cache must update to the new dims");
}

/// Sequence: None → (8,16) → (8,16) → (16,32) — verifies the state
/// machine across multiple calls. A refactor that skips the
/// assignment on "change" path would fail here because the third
/// `try_claim_broadcast` would erroneously claim again.
#[test]
fn try_claim_broadcast_full_sequence() {
 let mut cached = None;
 assert!(try_claim_broadcast(&mut cached, (8, 16)));
 assert_eq!(cached, Some((8, 16)));
 assert!(!try_claim_broadcast(&mut cached, (8, 16)));
 assert_eq!(cached, Some((8, 16)));
 assert!(try_claim_broadcast(&mut cached, (16, 32)));
 assert_eq!(cached, Some((16, 32)));
 assert!(!try_claim_broadcast(&mut cached, (16, 32)));
 assert_eq!(cached, Some((16, 32)));
}

// ── collect_window_pane_ids: session enumeration ──────────────────

/// Regression: `broadcast_cell_metrics_to_window` must reach every pane
/// across all tabs in the target window. This tests the extracted
/// enumeration helper independently of the App fixture.
/// See: §07.N
#[test]
fn collect_window_pane_ids_spans_all_tabs() {
 use oriterm_mux::PaneId;

 use crate::session::{SessionRegistry, Tab, Window};

 let mut session = SessionRegistry::new();
 let wid = session.alloc_window_id();
 let tid1 = session.alloc_tab_id();
 let tid2 = session.alloc_tab_id();

 let p1 = PaneId::from_raw(100);
 let p2 = PaneId::from_raw(101);

 session.add_tab(Tab::new(tid1, p1));
 session.add_tab(Tab::new(tid2, p2));

 let mut win = Window::new(wid);
 win.add_tab(tid1);
 win.add_tab(tid2);
 session.add_window(win);

 let panes = collect_window_pane_ids(&session, wid);
 assert!(panes.contains(&p1), "pane from tab 1 must be included");
 assert!(panes.contains(&p2), "pane from tab 2 must be included");
 assert_eq!(panes.len(), 2, "exactly 2 panes across 2 tabs");
}

/// Regression guard: a window with no tabs returns an empty list.
/// See: §07.N
#[test]
fn collect_window_pane_ids_empty_window() {
 use crate::session::{SessionRegistry, Window};

 let mut session = SessionRegistry::new();
 let wid = session.alloc_window_id();
 session.add_window(Window::new(wid));

 let panes = collect_window_pane_ids(&session, wid);
 assert!(panes.is_empty(), "empty window must yield no panes");
}

/// Regression guard: a non-existent window returns an empty list.
/// See: §07.N
#[test]
fn collect_window_pane_ids_missing_window() {
 use crate::session::SessionRegistry;
 use crate::session::WindowId as SessionWindowId;

 let session = SessionRegistry::new();
 let bogus = SessionWindowId::from_raw(999);

 let panes = collect_window_pane_ids(&session, bogus);
 assert!(panes.is_empty(), "missing window must yield no panes");
}
