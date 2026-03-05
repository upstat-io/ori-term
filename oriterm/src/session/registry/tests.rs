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
