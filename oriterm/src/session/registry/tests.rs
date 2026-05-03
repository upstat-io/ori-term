use oriterm_mux::PaneId;

use super::SessionRegistry;
use crate::session::id::{TabId, WindowId};
use crate::session::tab::Tab;
use crate::session::window::Window;

fn pid(n: u64) -> PaneId {
    PaneId::from_raw(n)
}

fn tid(n: u64) -> TabId {
    TabId::from_raw(n)
}

fn wid(n: u64) -> WindowId {
    WindowId::from_raw(n)
}

#[test]
fn new_registry_is_empty() {
    let reg = SessionRegistry::new();
    assert_eq!(reg.tab_count(), 0);
    assert_eq!(reg.window_count(), 0);
}

#[test]
fn add_and_get_tab() {
    let mut reg = SessionRegistry::new();
    let tab = Tab::new(tid(1), pid(10));
    reg.add_tab(tab);
    assert_eq!(reg.tab_count(), 1);
    assert!(reg.get_tab(tid(1)).is_some());
    assert_eq!(reg.get_tab(tid(1)).unwrap().active_pane(), pid(10));
}

#[test]
fn remove_tab() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(Tab::new(tid(1), pid(10)));
    let removed = reg.remove_tab(tid(1));
    assert!(removed.is_some());
    assert_eq!(reg.tab_count(), 0);
    assert!(reg.get_tab(tid(1)).is_none());
}

#[test]
fn remove_nonexistent_tab() {
    let mut reg = SessionRegistry::new();
    assert!(reg.remove_tab(tid(99)).is_none());
}

#[test]
fn get_tab_mut() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(Tab::new(tid(1), pid(10)));
    let tab = reg.get_tab_mut(tid(1)).unwrap();
    tab.set_active_pane(pid(20));
    assert_eq!(reg.get_tab(tid(1)).unwrap().active_pane(), pid(20));
}

#[test]
fn add_and_get_window() {
    let mut reg = SessionRegistry::new();
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    reg.add_window(win);
    assert_eq!(reg.window_count(), 1);
    assert!(reg.get_window(wid(1)).is_some());
}

#[test]
fn remove_window() {
    let mut reg = SessionRegistry::new();
    reg.add_window(Window::new(wid(1)));
    let removed = reg.remove_window(wid(1));
    assert!(removed.is_some());
    assert_eq!(reg.window_count(), 0);
}

#[test]
fn window_for_tab_found() {
    let mut reg = SessionRegistry::new();
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    reg.add_window(win);

    assert_eq!(reg.window_for_tab(tid(10)), Some(wid(1)));
    assert_eq!(reg.window_for_tab(tid(20)), Some(wid(1)));
}

#[test]
fn window_for_tab_not_found() {
    let reg = SessionRegistry::new();
    assert!(reg.window_for_tab(tid(99)).is_none());
}

#[test]
fn is_last_pane_true() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(Tab::new(tid(1), pid(10)));
    assert!(reg.is_last_pane(pid(10)));
}

#[test]
fn is_last_pane_false_multiple_tabs() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(Tab::new(tid(1), pid(10)));
    reg.add_tab(Tab::new(tid(2), pid(20)));
    assert!(!reg.is_last_pane(pid(10)));
}

#[test]
fn is_last_pane_false_wrong_pane() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(Tab::new(tid(1), pid(10)));
    assert!(!reg.is_last_pane(pid(99)));
}

#[test]
fn alloc_tab_id_monotonic() {
    let mut reg = SessionRegistry::new();
    let a = reg.alloc_tab_id();
    let b = reg.alloc_tab_id();
    assert_eq!(a.raw(), 1);
    assert_eq!(b.raw(), 2);
}

#[test]
fn alloc_window_id_monotonic() {
    let mut reg = SessionRegistry::new();
    let a = reg.alloc_window_id();
    let b = reg.alloc_window_id();
    assert_eq!(a.raw(), 1);
    assert_eq!(b.raw(), 2);
}

#[test]
fn default_matches_new() {
    let from_new = SessionRegistry::new();
    let from_default = SessionRegistry::default();
    assert_eq!(from_new.tab_count(), from_default.tab_count());
    assert_eq!(from_new.window_count(), from_default.window_count());
}

#[test]
fn windows_returns_all() {
    let mut reg = SessionRegistry::new();
    reg.add_window(Window::new(wid(1)));
    reg.add_window(Window::new(wid(2)));
    assert_eq!(reg.windows().len(), 2);
    assert!(reg.windows().contains_key(&wid(1)));
    assert!(reg.windows().contains_key(&wid(2)));
}

// pane_position regression suite — BUG-11-022.
// See bug-tracker/plans/BUG-11-022/section-03-tdd-matrix.md.

/// Edge case: empty registry — pane is registered nowhere.
/// Pins `tab_for_pane` returning None at the head of the chain.
#[test]
fn pane_position_returns_none_when_pane_unknown() {
    let reg = SessionRegistry::new();
    assert!(reg.pane_position(pid(99)).is_none());
}

/// Baseline: one window with one tab containing one pane —
/// resolves to (window_id, tab_index=0).
#[test]
fn pane_position_returns_some_for_single_window_single_tab() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(Tab::new(tid(1), pid(10)));
    let mut win = Window::new(wid(1));
    win.add_tab(tid(1));
    reg.add_window(win);

    let pos = reg.pane_position(pid(10)).expect("pane is registered");
    assert_eq!(pos.window_id, wid(1));
    assert_eq!(pos.tab_index, 0);
}

/// Index correctness within one window: pane in the second tab
/// resolves to tab_index=1 (not 0).
#[test]
fn pane_position_returns_secondary_tab_index() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(Tab::new(tid(1), pid(10)));
    reg.add_tab(Tab::new(tid(2), pid(20)));
    let mut win = Window::new(wid(1));
    win.add_tab(tid(1));
    win.add_tab(tid(2));
    reg.add_window(win);

    let pos = reg.pane_position(pid(20)).expect("pane is in tab 2");
    assert_eq!(pos.window_id, wid(1));
    assert_eq!(pos.tab_index, 1);
}

/// Regression: BUG-11-022 — semantic pin AND negative pin against
/// `active_window`-scoped routing. With two windows registered, a pane
/// in the SECOND window must resolve to that window's id and the correct
/// tab index in that window's tab list. A buggy implementation that
/// scoped the lookup to "active_window" (W1) would return None or
/// `(W1_id, ...)`. Cell #4 in §03 TDD matrix.
#[test]
fn pane_position_resolves_to_owning_window_for_pane_in_secondary_window() {
    let mut reg = SessionRegistry::new();
    // W1 first (would be "active" in App-layer state).
    reg.add_tab(Tab::new(tid(1), pid(10)));
    let mut w1 = Window::new(wid(1));
    w1.add_tab(tid(1));
    reg.add_window(w1);
    // W2 second (would be "background"). Pane P2 is in W2's only tab.
    reg.add_tab(Tab::new(tid(2), pid(20)));
    let mut w2 = Window::new(wid(2));
    w2.add_tab(tid(2));
    reg.add_window(w2);

    let pos = reg
        .pane_position(pid(20))
        .expect("P2 is registered in W2's only tab");

    // Positive: resolves to W2's id and tab index 0.
    assert_eq!(pos.window_id, wid(2));
    assert_eq!(pos.tab_index, 0);
    // Negative: rejects the broken active-window-scoped behavior.
    assert_ne!(pos.window_id, wid(1));
}

/// Two-axis pin: window selection AND tab index in that window. W2's
/// middle tab (index 1) resolves to (W2_id, 1) — confirms the index
/// is computed against the OWNING window's tab list, not W1's.
#[test]
fn pane_position_resolves_correct_tab_index_in_secondary_window_with_multiple_tabs() {
    let mut reg = SessionRegistry::new();
    // W1 with one tab.
    reg.add_tab(Tab::new(tid(1), pid(10)));
    let mut w1 = Window::new(wid(1));
    w1.add_tab(tid(1));
    reg.add_window(w1);
    // W2 with three tabs; pane is in the middle tab.
    reg.add_tab(Tab::new(tid(20), pid(200)));
    reg.add_tab(Tab::new(tid(21), pid(201)));
    reg.add_tab(Tab::new(tid(22), pid(202)));
    let mut w2 = Window::new(wid(2));
    w2.add_tab(tid(20));
    w2.add_tab(tid(21));
    w2.add_tab(tid(22));
    reg.add_window(w2);

    let pos = reg
        .pane_position(pid(201))
        .expect("P201 is in W2's middle tab");
    assert_eq!(pos.window_id, wid(2));
    assert_eq!(pos.tab_index, 1);
}

/// Edge case: pane with no tab — returns None at the first `?` of
/// the resolution chain (`tab_for_pane`).
#[test]
fn pane_position_returns_none_for_pane_with_no_tab() {
    let reg = SessionRegistry::new();
    // No tabs, no windows — the pane is registered nowhere.
    assert!(reg.pane_position(pid(42)).is_none());
}

/// Regression: BUG-11-022 — orphan-tab edge case. Tab containing the pane
/// exists in the registry, but the tab_id is not in any Window's tab list.
/// Cell #7 in §03 TDD matrix.
#[test]
fn pane_position_returns_none_for_pane_in_tab_not_in_any_window() {
    let mut reg = SessionRegistry::new();
    // Tab exists with the pane, but no window claims this tab.
    reg.add_tab(Tab::new(tid(1), pid(10)));
    // Add a different window with a different tab so the registry has windows.
    let mut w = Window::new(wid(1));
    w.add_tab(tid(2));
    reg.add_window(w);

    assert!(reg.pane_position(pid(10)).is_none());
}
