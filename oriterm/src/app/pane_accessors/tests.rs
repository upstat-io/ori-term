//! Tests for `is_pane_in_focused_tab_impl` — the pure helper that
//! decides whether a pane lives in the currently-focused window's
//! active tab. Exercised across the full session topology matrix
//! (no active window, single tab, split, multi-tab, multi-window,
//! orphan pane) so a regression in any guard clause surfaces here
//! instead of as a bell-icon glitch.

use oriterm_mux::PaneId;

use super::is_pane_in_focused_tab_impl;
use crate::session::{SessionRegistry, Tab};

/// Build a registry with one window holding `tabs.len()` tabs, each
/// containing the corresponding `PaneId`. Returns
/// `(registry, window_id, tab_ids)`. The FIRST tab is set as the
/// window's active tab regardless of insertion order — `Window::add_tab`
/// activates the most recently added tab, so we explicitly reset to
/// index 0 after the inserts.
fn registry_with_layout(
    tabs: &[PaneId],
) -> (
    SessionRegistry,
    crate::session::WindowId,
    Vec<crate::session::TabId>,
) {
    let mut session = SessionRegistry::new();
    let window_id = session.alloc_window_id();
    let mut window = crate::session::Window::new(window_id);
    let mut tab_ids = Vec::with_capacity(tabs.len());
    for pane in tabs {
        let tab_id = session.alloc_tab_id();
        window.add_tab(tab_id);
        session.add_tab(Tab::new(tab_id, *pane));
        tab_ids.push(tab_id);
    }
    if !tabs.is_empty() {
        window.set_active_tab_idx(0);
    }
    session.add_window(window);
    (session, window_id, tab_ids)
}

#[test]
fn returns_false_when_no_active_window() {
    let pane = PaneId::from_raw(1);
    let (session, _wid, _tabs) = registry_with_layout(&[pane]);
    assert!(
        !is_pane_in_focused_tab_impl(None, &session, pane),
        "no focused window → predicate is false"
    );
}

#[test]
fn returns_false_for_orphan_pane_with_no_owning_window() {
    let known = PaneId::from_raw(1);
    let orphan = PaneId::from_raw(99);
    let (session, wid, _tabs) = registry_with_layout(&[known]);
    assert!(
        !is_pane_in_focused_tab_impl(Some(wid), &session, orphan),
        "pane not registered in any tab → predicate is false"
    );
}

#[test]
fn returns_true_for_pane_in_focused_window_active_tab() {
    let pane = PaneId::from_raw(1);
    let (session, wid, _tabs) = registry_with_layout(&[pane]);
    assert!(
        is_pane_in_focused_tab_impl(Some(wid), &session, pane),
        "single-tab single-pane focused window → predicate is true"
    );
}

#[test]
fn returns_false_for_pane_in_focused_window_inactive_tab() {
    // Two tabs, first is active by default; query the SECOND tab's pane.
    let active_pane = PaneId::from_raw(1);
    let inactive_pane = PaneId::from_raw(2);
    let (session, wid, _tabs) = registry_with_layout(&[active_pane, inactive_pane]);
    assert!(
        is_pane_in_focused_tab_impl(Some(wid), &session, active_pane),
        "active-tab pane is in focused tab"
    );
    assert!(
        !is_pane_in_focused_tab_impl(Some(wid), &session, inactive_pane),
        "inactive-tab pane is NOT in focused tab even when same window"
    );
}

#[test]
fn returns_false_for_pane_in_non_focused_window() {
    // Two windows, each with one tab/pane; focus on window A, query a
    // pane that lives in window B.
    let pane_a = PaneId::from_raw(1);
    let pane_b = PaneId::from_raw(2);

    let mut session = SessionRegistry::new();
    let window_a_id = session.alloc_window_id();
    let window_b_id = session.alloc_window_id();
    let tab_a_id = session.alloc_tab_id();
    let tab_b_id = session.alloc_tab_id();

    let mut window_a = crate::session::Window::new(window_a_id);
    window_a.add_tab(tab_a_id);
    let mut window_b = crate::session::Window::new(window_b_id);
    window_b.add_tab(tab_b_id);

    session.add_tab(Tab::new(tab_a_id, pane_a));
    session.add_tab(Tab::new(tab_b_id, pane_b));
    session.add_window(window_a);
    session.add_window(window_b);

    assert!(
        is_pane_in_focused_tab_impl(Some(window_a_id), &session, pane_a),
        "pane_a in focused window_a → true"
    );
    assert!(
        !is_pane_in_focused_tab_impl(Some(window_a_id), &session, pane_b),
        "pane_b in non-focused window_b → false"
    );
    assert!(
        is_pane_in_focused_tab_impl(Some(window_b_id), &session, pane_b),
        "switching focus to window_b → pane_b is now in focused tab"
    );
    assert!(
        !is_pane_in_focused_tab_impl(Some(window_b_id), &session, pane_a),
        "switching focus to window_b → pane_a is now in non-focused window"
    );
}

#[test]
fn returns_true_for_split_pane_in_focused_tab() {
    // A tab with two panes (the split case): both must register as
    // "in focused tab" regardless of which is the active split.
    use std::sync::Arc;

    use crate::session::SplitDirection;
    use crate::session::split_tree::SplitTree;

    let active_pane = PaneId::from_raw(1);
    let inactive_pane = PaneId::from_raw(2);

    let mut session = SessionRegistry::new();
    let window_id = session.alloc_window_id();
    let tab_id = session.alloc_tab_id();
    let mut tab = Tab::new(tab_id, active_pane);
    let split_tree = SplitTree::Split {
        direction: SplitDirection::Vertical,
        ratio: 0.5,
        first: Arc::new(SplitTree::leaf(active_pane)),
        second: Arc::new(SplitTree::leaf(inactive_pane)),
    };
    tab.replace_layout(split_tree);
    let mut window = crate::session::Window::new(window_id);
    window.add_tab(tab_id);
    session.add_tab(tab);
    session.add_window(window);

    assert!(
        is_pane_in_focused_tab_impl(Some(window_id), &session, active_pane),
        "active split pane is in focused tab"
    );
    assert!(
        is_pane_in_focused_tab_impl(Some(window_id), &session, inactive_pane),
        "non-active split pane in same tab is ALSO in focused tab — \
         the predicate is tab-scoped, not active-pane scoped"
    );
}

#[test]
fn returns_false_when_focused_window_does_not_exist_in_registry() {
    // Stale active_window pointing to a removed window — guard clause
    // for the registry-lookup miss path.
    let pane = PaneId::from_raw(1);
    let (mut session, wid, _tabs) = registry_with_layout(&[pane]);
    let _ = session.remove_window(wid);
    assert!(
        !is_pane_in_focused_tab_impl(Some(wid), &session, pane),
        "stale active_window pointing to removed window → false"
    );
}
