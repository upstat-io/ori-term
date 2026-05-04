//! Unit tests for tab management helpers and wrap_index arithmetic.

use super::wrap_index;

// -- wrap_index tests --

#[test]
fn wrap_forward_within_range() {
    assert_eq!(wrap_index(0, 1, 3), 1);
    assert_eq!(wrap_index(1, 1, 3), 2);
}

#[test]
fn wrap_forward_wraps_around() {
    assert_eq!(wrap_index(2, 1, 3), 0);
}

#[test]
fn wrap_backward_within_range() {
    assert_eq!(wrap_index(1, -1, 3), 0);
    assert_eq!(wrap_index(2, -1, 3), 1);
}

#[test]
fn wrap_backward_wraps_around() {
    assert_eq!(wrap_index(0, -1, 3), 2);
}

#[test]
fn wrap_forward_by_two() {
    assert_eq!(wrap_index(1, 2, 3), 0);
}

#[test]
fn wrap_backward_by_two() {
    assert_eq!(wrap_index(0, -2, 3), 1);
}

#[test]
fn wrap_single_tab() {
    // With one tab, any delta returns 0.
    assert_eq!(wrap_index(0, 1, 1), 0);
    assert_eq!(wrap_index(0, -1, 1), 0);
}

#[test]
fn wrap_large_delta() {
    // Large forward delta wraps multiple times.
    assert_eq!(wrap_index(0, 7, 3), 1);
    // Large backward delta wraps multiple times.
    assert_eq!(wrap_index(0, -7, 3), 2);
}

// -- Window tab management tests --

use crate::session::{TabId, Window, WindowId};

/// Create a window with N tabs for testing.
fn window_with_tabs(n: usize) -> Window {
    let mut win = Window::new(WindowId::from_raw(1));
    for i in 0..n {
        win.add_tab(TabId::from_raw(100 + i as u64));
    }
    win
}

#[test]
fn create_three_tabs_unique_ids() {
    let win = window_with_tabs(3);
    let tabs = win.tabs();
    assert_eq!(tabs.len(), 3);
    // All unique.
    assert_ne!(tabs[0], tabs[1]);
    assert_ne!(tabs[1], tabs[2]);
    assert_ne!(tabs[0], tabs[2]);
}

#[test]
fn close_middle_tab_preserves_order() {
    let mut win = window_with_tabs(3);
    let t0 = win.tabs()[0];
    let t1 = win.tabs()[1];
    let t2 = win.tabs()[2];

    win.set_active_tab_idx(0);
    win.remove_tab(t1);

    assert_eq!(win.tabs(), &[t0, t2]);
    assert_eq!(win.active_tab_idx(), 0);
}

#[test]
fn close_active_tab_adjusts_index() {
    let mut win = window_with_tabs(3);
    // Active tab is last.
    win.set_active_tab_idx(2);
    let t2 = win.tabs()[2];
    win.remove_tab(t2);
    // Active should clamp to new last.
    assert_eq!(win.active_tab_idx(), 1);
}

#[test]
fn cycle_wrap_forward() {
    let win = window_with_tabs(3);
    // tab 2 of 3 → next → tab 0.
    let next = wrap_index(2, 1, win.tabs().len());
    assert_eq!(next, 0);
}

#[test]
fn cycle_wrap_backward() {
    let win = window_with_tabs(3);
    // tab 0 of 3 → prev → tab 2.
    let next = wrap_index(0, -1, win.tabs().len());
    assert_eq!(next, 2);
}

#[test]
fn reorder_tabs() {
    let mut win = window_with_tabs(4);
    let tabs: Vec<TabId> = win.tabs().to_vec();
    // Move tab 0 to position 2.
    assert!(win.reorder_tab(0, 2));
    assert_eq!(win.tabs()[0], tabs[1]);
    assert_eq!(win.tabs()[1], tabs[2]);
    assert_eq!(win.tabs()[2], tabs[0]);
    assert_eq!(win.tabs()[3], tabs[3]);
}

#[test]
fn closing_last_tab_leaves_empty() {
    let mut win = window_with_tabs(1);
    let t0 = win.tabs()[0];
    win.remove_tab(t0);
    assert!(win.tabs().is_empty());
}

// -- clear_tab_bells_impl regression pins --
//
// The clear_tab_bells sweep is the load-bearing fix for the
// split-pane secondary cause of stuck bell icons: focus-change paths
// used to clear only the active pane's bell, leaving non-active
// splits with stuck has_bell=true. These tests pin "every pane in
// tab.all_panes() gets clear_bell + mark_output_seen" against any
// future regression that re-narrows the scope to active_pane only.
//
// Exercised against a real `EmbeddedMux` because EmbeddedMux's
// `set_bell`/`clear_bell` only mutate `bell_panes` (no spawned-pane
// requirement) — no MockMuxBackend infrastructure needed.

use std::sync::Arc;

use oriterm_mux::EmbeddedMux;
use oriterm_mux::PaneId;
use oriterm_mux::backend::MuxBackend;

use crate::session::SessionRegistry;
use crate::session::SplitDirection;
use crate::session::Tab as SessionTab;
use crate::session::split_tree::SplitTree;

use super::clear_tab_bells_impl;

/// `clear_tab_bells_impl` on a single-pane tab clears that pane's bell
/// and marks output seen. Smoke test for the trivial topology.
#[test]
fn clear_tab_bells_impl_single_pane_tab_clears_one_pane() {
    let mut session = SessionRegistry::new();
    let tab_id = session.alloc_tab_id();
    let pane = PaneId::from_raw(1);
    session.add_tab(SessionTab::new(tab_id, pane));

    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    mux.set_bell(pane);
    assert!(mux.has_bell(pane), "precondition: bell set on pane");

    clear_tab_bells_impl(&session, &mut mux, tab_id);

    assert!(!mux.has_bell(pane), "single-pane tab: bell cleared");
}

/// `clear_tab_bells_impl` on a SPLIT tab (2 panes) clears BOTH bells —
/// not just the tab's `active_pane`. Pinned because the pre-fix
/// behavior was active-pane-only, leaving non-active splits with stuck
/// bell icons after focus-change.
#[test]
fn clear_tab_bells_impl_split_tab_clears_all_panes_in_tree() {
    let mut session = SessionRegistry::new();
    let tab_id = session.alloc_tab_id();
    let active_pane = PaneId::from_raw(1);
    let inactive_pane = PaneId::from_raw(2);

    let mut tab = SessionTab::new(tab_id, active_pane);
    let split_tree = SplitTree::Split {
        direction: SplitDirection::Vertical,
        ratio: 0.5,
        first: Arc::new(SplitTree::leaf(active_pane)),
        second: Arc::new(SplitTree::leaf(inactive_pane)),
    };
    tab.replace_layout(split_tree);
    session.add_tab(tab);

    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    mux.set_bell(active_pane);
    mux.set_bell(inactive_pane);
    assert!(mux.has_bell(active_pane));
    assert!(mux.has_bell(inactive_pane));

    clear_tab_bells_impl(&session, &mut mux, tab_id);

    assert!(!mux.has_bell(active_pane), "active split pane bell cleared");
    assert!(
        !mux.has_bell(inactive_pane),
        "non-active split pane bell ALSO cleared — pinned against \
         the active-pane-only regression"
    );
}

/// `clear_tab_bells_impl` on a 3-deep nested split (left + right-top +
/// right-bottom) clears all 3 panes. Matrix coverage for non-trivial
/// trees — `Tab::all_panes()` traverses depth-first so the iteration
/// order is `[left, right-top, right-bottom]`; correctness depends
/// only on the SET of pane IDs, not the order.
#[test]
fn clear_tab_bells_impl_deep_nested_split_clears_every_leaf() {
    let mut session = SessionRegistry::new();
    let tab_id = session.alloc_tab_id();
    let p1 = PaneId::from_raw(1);
    let p2 = PaneId::from_raw(2);
    let p3 = PaneId::from_raw(3);

    let mut tab = SessionTab::new(tab_id, p1);
    let nested = SplitTree::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.5,
        first: Arc::new(SplitTree::leaf(p1)),
        second: Arc::new(SplitTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Arc::new(SplitTree::leaf(p2)),
            second: Arc::new(SplitTree::leaf(p3)),
        }),
    };
    tab.replace_layout(nested);
    session.add_tab(tab);

    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    for &p in &[p1, p2, p3] {
        mux.set_bell(p);
    }

    clear_tab_bells_impl(&session, &mut mux, tab_id);

    for &p in &[p1, p2, p3] {
        assert!(
            !mux.has_bell(p),
            "every leaf in the nested split tree gets cleared (pane {p:?})",
        );
    }
}

/// `clear_tab_bells_impl` on an UNKNOWN tab id (not registered in the
/// session) is a no-op — does not panic, does not mutate any other
/// pane's bell state. Defensive pin for the registry-miss path that
/// `Tab::all_panes()` returns `Vec::new()` for.
#[test]
fn clear_tab_bells_impl_unknown_tab_id_is_noop() {
    let session = SessionRegistry::new();
    let bogus_tab = TabId::from_raw(999);
    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    let unrelated_pane = PaneId::from_raw(42);
    mux.set_bell(unrelated_pane);

    clear_tab_bells_impl(&session, &mut mux, bogus_tab);

    assert!(
        mux.has_bell(unrelated_pane),
        "unknown-tab no-op must not touch unrelated pane's bell"
    );
}

/// Regression guard against an active-pane-only regression: even when
/// `Tab::active_pane` points at one specific split, the sweep MUST
/// touch every leaf, not just the active one. Pinned by setting the
/// bell on a non-active split and asserting it gets cleared.
#[test]
fn clear_tab_bells_impl_clears_non_active_split_pane_specifically() {
    let mut session = SessionRegistry::new();
    let tab_id = session.alloc_tab_id();
    let active_pane = PaneId::from_raw(1);
    let non_active_pane = PaneId::from_raw(2);

    let mut tab = SessionTab::new(tab_id, active_pane);
    tab.set_active_pane(active_pane);
    let split_tree = SplitTree::Split {
        direction: SplitDirection::Vertical,
        ratio: 0.5,
        first: Arc::new(SplitTree::leaf(active_pane)),
        second: Arc::new(SplitTree::leaf(non_active_pane)),
    };
    tab.replace_layout(split_tree);
    session.add_tab(tab);

    let mut mux = EmbeddedMux::new(Arc::new(|| {}));
    // ONLY the non-active split has a bell set. If a regression
    // re-narrows clear_tab_bells to `active_pane` only, this bell
    // would NOT be cleared and the test fails.
    mux.set_bell(non_active_pane);

    clear_tab_bells_impl(&session, &mut mux, tab_id);

    assert!(
        !mux.has_bell(non_active_pane),
        "non-active split pane bell MUST be cleared — \
         active-pane-only behavior would leak this bell forever"
    );
}
