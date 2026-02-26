use crate::id::{DomainId, PaneId, TabId, WindowId};
use crate::session::{MuxTab, MuxWindow};

use super::{PaneEntry, PaneRegistry, SessionRegistry};

// --- PaneRegistry tests ---

fn entry(pane: u64, tab: u64, domain: u64) -> PaneEntry {
    PaneEntry {
        pane: PaneId::from_raw(pane),
        tab: TabId::from_raw(tab),
        domain: DomainId::from_raw(domain),
    }
}

#[test]
fn empty_registry() {
    let reg = PaneRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
    assert!(reg.get(PaneId::from_raw(1)).is_none());
}

#[test]
fn register_and_get() {
    let mut reg = PaneRegistry::new();
    reg.register(entry(1, 10, 1));

    let e = reg.get(PaneId::from_raw(1)).unwrap();
    assert_eq!(e.pane, PaneId::from_raw(1));
    assert_eq!(e.tab, TabId::from_raw(10));
    assert_eq!(e.domain, DomainId::from_raw(1));
}

#[test]
fn unregister_removes_entry() {
    let mut reg = PaneRegistry::new();
    reg.register(entry(1, 10, 1));
    assert_eq!(reg.len(), 1);

    let removed = reg.unregister(PaneId::from_raw(1));
    assert!(removed.is_some());
    assert!(reg.is_empty());
}

#[test]
fn unregister_nonexistent_returns_none() {
    let mut reg = PaneRegistry::new();
    assert!(reg.unregister(PaneId::from_raw(99)).is_none());
}

#[test]
fn panes_in_tab() {
    let mut reg = PaneRegistry::new();
    reg.register(entry(1, 10, 1));
    reg.register(entry(2, 10, 1));
    reg.register(entry(3, 20, 1));

    let mut panes = reg.panes_in_tab(TabId::from_raw(10));
    panes.sort_by_key(|p| p.raw());
    assert_eq!(panes, vec![PaneId::from_raw(1), PaneId::from_raw(2)]);
}

#[test]
fn panes_in_nonexistent_tab_is_empty() {
    let reg = PaneRegistry::new();
    assert!(reg.panes_in_tab(TabId::from_raw(99)).is_empty());
}

// --- SessionRegistry tests ---

#[test]
fn empty_session_registry() {
    let reg = SessionRegistry::new();
    assert_eq!(reg.tab_count(), 0);
    assert_eq!(reg.window_count(), 0);
}

#[test]
fn add_and_get_tab() {
    let mut reg = SessionRegistry::new();
    let tab = MuxTab::new(TabId::from_raw(1), PaneId::from_raw(10));
    reg.add_tab(tab);

    assert_eq!(reg.tab_count(), 1);
    let t = reg.get_tab(TabId::from_raw(1)).unwrap();
    assert_eq!(t.id(), TabId::from_raw(1));
    assert_eq!(t.active_pane(), PaneId::from_raw(10));
}

#[test]
fn remove_tab() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(MuxTab::new(TabId::from_raw(1), PaneId::from_raw(10)));

    let removed = reg.remove_tab(TabId::from_raw(1));
    assert!(removed.is_some());
    assert_eq!(reg.tab_count(), 0);
}

#[test]
fn add_and_get_window() {
    let mut reg = SessionRegistry::new();
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(10));
    reg.add_window(w);

    assert_eq!(reg.window_count(), 1);
    let w = reg.get_window(WindowId::from_raw(1)).unwrap();
    assert_eq!(w.tabs(), &[TabId::from_raw(10)]);
}

#[test]
fn window_for_tab_found() {
    let mut reg = SessionRegistry::new();
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(10));
    w.add_tab(TabId::from_raw(20));
    reg.add_window(w);

    assert_eq!(
        reg.window_for_tab(TabId::from_raw(20)),
        Some(WindowId::from_raw(1))
    );
}

#[test]
fn window_for_tab_not_found() {
    let reg = SessionRegistry::new();
    assert!(reg.window_for_tab(TabId::from_raw(99)).is_none());
}

#[test]
fn get_tab_mut_modifies() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(MuxTab::new(TabId::from_raw(1), PaneId::from_raw(10)));

    let tab = reg.get_tab_mut(TabId::from_raw(1)).unwrap();
    tab.set_active_pane(PaneId::from_raw(20));

    let tab = reg.get_tab(TabId::from_raw(1)).unwrap();
    assert_eq!(tab.active_pane(), PaneId::from_raw(20));
}

// --- Gap analysis tests ---

/// Registering the same pane ID twice overwrites the first entry.
#[test]
fn register_overwrites_existing_entry() {
    let mut reg = PaneRegistry::new();
    reg.register(entry(1, 10, 1));
    reg.register(entry(1, 20, 2));

    assert_eq!(reg.len(), 1);
    let e = reg.get(PaneId::from_raw(1)).unwrap();
    assert_eq!(e.tab, TabId::from_raw(20));
    assert_eq!(e.domain, DomainId::from_raw(2));
}

/// After unregistering a pane, cross-reference queries no longer return it.
#[test]
fn cross_query_consistent_after_unregister() {
    let mut reg = PaneRegistry::new();
    reg.register(entry(1, 10, 1));
    reg.register(entry(2, 10, 1));
    reg.register(entry(3, 10, 1));

    reg.unregister(PaneId::from_raw(2));

    assert_eq!(reg.len(), 2);
    assert!(reg.get(PaneId::from_raw(2)).is_none());
    let mut panes = reg.panes_in_tab(TabId::from_raw(10));
    panes.sort_by_key(|p| p.raw());
    assert_eq!(panes, vec![PaneId::from_raw(1), PaneId::from_raw(3)]);
}

/// Panes from multiple domains coexist and are distinguishable.
#[test]
fn multiple_domains_in_registry() {
    let mut reg = PaneRegistry::new();
    reg.register(entry(1, 10, 1));
    reg.register(entry(2, 10, 2));
    reg.register(entry(3, 20, 1));

    assert_eq!(reg.len(), 3);
    assert_eq!(
        reg.get(PaneId::from_raw(1)).unwrap().domain,
        DomainId::from_raw(1)
    );
    assert_eq!(
        reg.get(PaneId::from_raw(2)).unwrap().domain,
        DomainId::from_raw(2)
    );
    // Both panes in tab 10, despite different domains.
    let mut panes = reg.panes_in_tab(TabId::from_raw(10));
    panes.sort_by_key(|p| p.raw());
    assert_eq!(panes, vec![PaneId::from_raw(1), PaneId::from_raw(2)]);
}

/// Removing a nonexistent tab from SessionRegistry returns None.
#[test]
fn remove_nonexistent_tab_from_session_registry() {
    let mut reg = SessionRegistry::new();
    reg.add_tab(MuxTab::new(TabId::from_raw(1), PaneId::from_raw(10)));
    assert!(reg.remove_tab(TabId::from_raw(99)).is_none());
    assert_eq!(reg.tab_count(), 1);
}

/// Removing a window from SessionRegistry works correctly.
#[test]
fn remove_window_from_session_registry() {
    let mut reg = SessionRegistry::new();
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(10));
    reg.add_window(w);

    let removed = reg.remove_window(WindowId::from_raw(1));
    assert!(removed.is_some());
    assert_eq!(reg.window_count(), 0);
    assert!(reg.get_window(WindowId::from_raw(1)).is_none());
}

/// Registry handles 1000+ entries without issue.
#[test]
fn large_registry_stress() {
    let mut reg = PaneRegistry::new();
    for i in 0..1000u64 {
        reg.register(PaneEntry {
            pane: PaneId::from_raw(i),
            tab: TabId::from_raw(i / 10),
            domain: DomainId::from_raw(1),
        });
    }
    assert_eq!(reg.len(), 1000);

    // Tab 0 should contain panes 0..10.
    let mut panes = reg.panes_in_tab(TabId::from_raw(0));
    panes.sort_by_key(|p| p.raw());
    assert_eq!(panes.len(), 10);
    assert_eq!(panes[0], PaneId::from_raw(0));
    assert_eq!(panes[9], PaneId::from_raw(9));

    // Unregister all even panes.
    for i in (0..1000u64).step_by(2) {
        reg.unregister(PaneId::from_raw(i));
    }
    assert_eq!(reg.len(), 500);
}
