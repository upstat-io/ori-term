use crate::id::{PaneId, TabId, WindowId};
use crate::layout::SplitDirection;

use super::{MuxTab, MuxWindow};

// --- MuxTab tests ---

#[test]
fn new_tab_has_single_pane() {
    let tab = MuxTab::new(TabId::from_raw(1), PaneId::from_raw(10));
    assert_eq!(tab.id(), TabId::from_raw(1));
    assert_eq!(tab.active_pane(), PaneId::from_raw(10));
    assert_eq!(tab.all_panes(), vec![PaneId::from_raw(10)]);
}

#[test]
fn set_tree_pushes_undo() {
    let p1 = PaneId::from_raw(1);
    let p2 = PaneId::from_raw(2);
    let mut tab = MuxTab::new(TabId::from_raw(1), p1);

    let new_tree = tab.tree().split_at(p1, SplitDirection::Vertical, p2, 0.5);
    tab.set_tree(new_tree);

    assert_eq!(tab.all_panes().len(), 2);
    assert!(tab.tree().contains(p1));
    assert!(tab.tree().contains(p2));
}

#[test]
fn undo_restores_previous_tree() {
    let p1 = PaneId::from_raw(1);
    let p2 = PaneId::from_raw(2);
    let mut tab = MuxTab::new(TabId::from_raw(1), p1);

    let new_tree = tab.tree().split_at(p1, SplitDirection::Vertical, p2, 0.5);
    tab.set_tree(new_tree);
    assert_eq!(tab.all_panes().len(), 2);

    assert!(tab.undo_tree());
    assert_eq!(tab.all_panes(), vec![p1]);
}

#[test]
fn undo_empty_stack_returns_false() {
    let mut tab = MuxTab::new(TabId::from_raw(1), PaneId::from_raw(1));
    assert!(!tab.undo_tree());
}

#[test]
fn undo_stack_capped_at_32() {
    let p1 = PaneId::from_raw(1);
    let mut tab = MuxTab::new(TabId::from_raw(1), p1);

    // Push 40 tree mutations.
    for i in 2..42u64 {
        let p = PaneId::from_raw(i);
        let new_tree = tab.tree().split_at(p1, SplitDirection::Horizontal, p, 0.5);
        tab.set_tree(new_tree);
    }

    // Undo stack should be capped at 32.
    let mut count = 0;
    while tab.undo_tree() {
        count += 1;
    }
    assert_eq!(count, 32);
}

#[test]
fn set_active_pane() {
    let p1 = PaneId::from_raw(1);
    let p2 = PaneId::from_raw(2);
    let mut tab = MuxTab::new(TabId::from_raw(1), p1);
    tab.set_active_pane(p2);
    assert_eq!(tab.active_pane(), p2);
}

// --- MuxWindow tests ---

#[test]
fn new_window_is_empty() {
    let w = MuxWindow::new(WindowId::from_raw(1));
    assert_eq!(w.id(), WindowId::from_raw(1));
    assert!(w.tabs().is_empty());
    assert_eq!(w.active_tab_idx(), 0);
    assert!(w.active_tab().is_none());
}

#[test]
fn add_tab_appends() {
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(10));
    w.add_tab(TabId::from_raw(20));
    assert_eq!(w.tabs(), &[TabId::from_raw(10), TabId::from_raw(20)]);
}

#[test]
fn active_tab_after_add() {
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(10));
    assert_eq!(w.active_tab(), Some(TabId::from_raw(10)));
}

#[test]
fn remove_tab_adjusts_active_before() {
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(1));
    w.add_tab(TabId::from_raw(2));
    w.add_tab(TabId::from_raw(3));
    w.set_active_tab_idx(2); // tab 3 is active

    // Remove tab before active — active should shift left.
    assert!(w.remove_tab(TabId::from_raw(1)));
    assert_eq!(w.active_tab_idx(), 1);
    assert_eq!(w.active_tab(), Some(TabId::from_raw(3)));
}

#[test]
fn remove_active_tab_clamps() {
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(1));
    w.add_tab(TabId::from_raw(2));
    w.set_active_tab_idx(1);

    // Remove the active (last) tab — index should clamp to new last.
    assert!(w.remove_tab(TabId::from_raw(2)));
    assert_eq!(w.active_tab_idx(), 0);
    assert_eq!(w.active_tab(), Some(TabId::from_raw(1)));
}

#[test]
fn remove_nonexistent_tab_returns_false() {
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(1));
    assert!(!w.remove_tab(TabId::from_raw(99)));
}

#[test]
fn remove_last_tab_resets() {
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(1));
    assert!(w.remove_tab(TabId::from_raw(1)));
    assert!(w.tabs().is_empty());
    assert_eq!(w.active_tab_idx(), 0);
    assert!(w.active_tab().is_none());
}

#[test]
fn set_active_tab_idx_clamps() {
    let mut w = MuxWindow::new(WindowId::from_raw(1));
    w.add_tab(TabId::from_raw(1));
    w.add_tab(TabId::from_raw(2));
    w.set_active_tab_idx(999);
    assert_eq!(w.active_tab_idx(), 1);
}
