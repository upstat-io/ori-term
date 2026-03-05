use super::Window;
use crate::session::id::{TabId, WindowId};

fn tid(n: u64) -> TabId {
    TabId::from_raw(n)
}

fn wid(n: u64) -> WindowId {
    WindowId::from_raw(n)
}

#[test]
fn new_window_is_empty() {
    let win = Window::new(wid(1));
    assert_eq!(win.id(), wid(1));
    assert!(win.tabs().is_empty());
    assert_eq!(win.active_tab_idx(), 0);
    assert!(win.active_tab().is_none());
}

#[test]
fn add_tab() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    assert_eq!(win.tabs(), &[tid(10), tid(20)]);
    assert_eq!(win.active_tab(), Some(tid(10)));
}

#[test]
fn set_active_tab_idx() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.set_active_tab_idx(1);
    assert_eq!(win.active_tab(), Some(tid(20)));
}

#[test]
fn set_active_tab_idx_clamps() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.set_active_tab_idx(100);
    assert_eq!(win.active_tab_idx(), 0);
}

#[test]
fn remove_tab_basic() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    assert!(win.remove_tab(tid(10)));
    assert_eq!(win.tabs(), &[tid(20)]);
}

#[test]
fn remove_tab_not_found() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    assert!(!win.remove_tab(tid(99)));
}

#[test]
fn remove_tab_adjusts_active_when_before() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.add_tab(tid(30));
    win.set_active_tab_idx(2); // active = tid(30)
    win.remove_tab(tid(10));
    // Active should still point to tid(30), now at index 1.
    assert_eq!(win.active_tab(), Some(tid(30)));
    assert_eq!(win.active_tab_idx(), 1);
}

#[test]
fn remove_tab_clamps_active_when_at_end() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.set_active_tab_idx(1); // active = tid(20)
    win.remove_tab(tid(20));
    // Active should clamp to last valid index.
    assert_eq!(win.active_tab_idx(), 0);
    assert_eq!(win.active_tab(), Some(tid(10)));
}

#[test]
fn remove_last_tab_resets_active() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.remove_tab(tid(10));
    assert!(win.tabs().is_empty());
    assert_eq!(win.active_tab_idx(), 0);
    assert!(win.active_tab().is_none());
}

#[test]
fn insert_tab_at_beginning() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.set_active_tab_idx(0); // active = tid(10)
    win.insert_tab_at(0, tid(5));
    assert_eq!(win.tabs(), &[tid(5), tid(10), tid(20)]);
    // Active should shift to keep tracking tid(10).
    assert_eq!(win.active_tab(), Some(tid(10)));
    assert_eq!(win.active_tab_idx(), 1);
}

#[test]
fn insert_tab_at_end_appends() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.insert_tab_at(100, tid(20));
    assert_eq!(win.tabs(), &[tid(10), tid(20)]);
}

#[test]
fn reorder_tab_basic() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.add_tab(tid(30));
    assert!(win.reorder_tab(0, 2));
    assert_eq!(win.tabs(), &[tid(20), tid(30), tid(10)]);
}

#[test]
fn reorder_tab_active_follows_moved() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.add_tab(tid(30));
    win.set_active_tab_idx(0); // active = tid(10)
    win.reorder_tab(0, 2);
    // Active tab (tid(10)) moved to index 2.
    assert_eq!(win.active_tab(), Some(tid(10)));
    assert_eq!(win.active_tab_idx(), 2);
}

#[test]
fn reorder_tab_out_of_bounds() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    assert!(!win.reorder_tab(0, 5));
    assert!(!win.reorder_tab(5, 0));
}

#[test]
fn replace_tabs_preserves_active() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.set_active_tab_idx(1); // active = tid(20)
    win.replace_tabs(&[tid(30), tid(20), tid(10)]);
    // tid(20) still exists, now at index 1.
    assert_eq!(win.active_tab(), Some(tid(20)));
    assert_eq!(win.active_tab_idx(), 1);
}

#[test]
fn replace_tabs_resets_when_active_gone() {
    let mut win = Window::new(wid(1));
    win.add_tab(tid(10));
    win.add_tab(tid(20));
    win.set_active_tab_idx(1); // active = tid(20)
    win.replace_tabs(&[tid(30), tid(40)]);
    // tid(20) is gone, should reset to 0.
    assert_eq!(win.active_tab_idx(), 0);
    assert_eq!(win.active_tab(), Some(tid(30)));
}
