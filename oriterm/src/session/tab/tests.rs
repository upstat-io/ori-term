use std::collections::HashSet;

use oriterm_mux::PaneId;

use super::Tab;
use crate::session::id::TabId;

fn pid(n: u64) -> PaneId {
    PaneId::from_raw(n)
}

fn tid(n: u64) -> TabId {
    TabId::from_raw(n)
}

#[test]
fn new_tab_has_single_pane() {
    let tab = Tab::new(tid(1), pid(10));
    assert_eq!(tab.id(), tid(1));
    assert_eq!(tab.active_pane(), pid(10));
    assert_eq!(tab.all_panes(), vec![pid(10)]);
    assert!(tab.zoomed_pane().is_none());
}

#[test]
fn set_active_pane() {
    let mut tab = Tab::new(tid(1), pid(10));
    tab.set_active_pane(pid(20));
    assert_eq!(tab.active_pane(), pid(20));
}

#[test]
fn zoom_state() {
    let mut tab = Tab::new(tid(1), pid(10));
    assert!(tab.zoomed_pane().is_none());

    tab.set_zoomed_pane(Some(pid(10)));
    assert_eq!(tab.zoomed_pane(), Some(pid(10)));

    tab.set_zoomed_pane(None);
    assert!(tab.zoomed_pane().is_none());
}

#[test]
fn set_tree_pushes_undo() {
    let mut tab = Tab::new(tid(1), pid(10));
    let original_tree = tab.tree().clone();

    // Replace tree with a new one.
    let new_tree = crate::session::split_tree::SplitTree::leaf(pid(20));
    tab.set_tree(new_tree.clone());

    assert_eq!(tab.tree().panes(), vec![pid(20)]);

    // Undo should restore the original.
    let live = HashSet::from([pid(10), pid(20)]);
    assert!(tab.undo_tree(&live));
    assert_eq!(tab.tree().panes(), original_tree.panes());
}

#[test]
fn undo_redo_cycle() {
    let mut tab = Tab::new(tid(1), pid(10));
    let live = HashSet::from([pid(10), pid(20)]);

    let tree_a = tab.tree().clone();
    let tree_b = crate::session::split_tree::SplitTree::leaf(pid(20));
    tab.set_tree(tree_b);

    // Undo: back to tree_a.
    assert!(tab.undo_tree(&live));
    assert_eq!(tab.tree().panes(), tree_a.panes());

    // Redo: forward to tree_b.
    assert!(tab.redo_tree(&live));
    assert_eq!(tab.tree().panes(), vec![pid(20)]);
}

#[test]
fn undo_skips_stale_entries() {
    let mut tab = Tab::new(tid(1), pid(10));

    // Push a tree referencing pid(20), then another referencing pid(10).
    let stale_tree = crate::session::split_tree::SplitTree::leaf(pid(20));
    tab.set_tree(stale_tree);
    let current = crate::session::split_tree::SplitTree::leaf(pid(10));
    tab.set_tree(current);

    // Only pid(10) is live — the stale tree (pid(20)) should be skipped.
    let live = HashSet::from([pid(10)]);
    assert!(tab.undo_tree(&live));
    // Should have skipped the stale pid(20) tree and found the original pid(10) tree.
    assert_eq!(tab.tree().panes(), vec![pid(10)]);
}

#[test]
fn undo_returns_false_when_empty() {
    let mut tab = Tab::new(tid(1), pid(10));
    let live = HashSet::from([pid(10)]);
    assert!(!tab.undo_tree(&live));
}

#[test]
fn redo_returns_false_when_empty() {
    let mut tab = Tab::new(tid(1), pid(10));
    let live = HashSet::from([pid(10)]);
    assert!(!tab.redo_tree(&live));
}

#[test]
fn replace_layout_does_not_push_undo() {
    let mut tab = Tab::new(tid(1), pid(10));
    let new_tree = crate::session::split_tree::SplitTree::leaf(pid(20));
    tab.replace_layout(new_tree);

    // Undo stack should be empty.
    let live = HashSet::from([pid(10), pid(20)]);
    assert!(!tab.undo_tree(&live));
}

#[test]
fn set_tree_clears_redo() {
    let mut tab = Tab::new(tid(1), pid(10));
    let live = HashSet::from([pid(10), pid(20), pid(30)]);

    let tree_b = crate::session::split_tree::SplitTree::leaf(pid(20));
    tab.set_tree(tree_b);

    // Undo to create a redo entry.
    assert!(tab.undo_tree(&live));

    // New mutation should clear redo.
    let tree_c = crate::session::split_tree::SplitTree::leaf(pid(30));
    tab.set_tree(tree_c);
    assert!(!tab.redo_tree(&live));
}

#[test]
fn floating_layer_initially_empty() {
    let tab = Tab::new(tid(1), pid(10));
    assert!(tab.floating().is_empty());
    assert!(!tab.is_floating(pid(10)));
}

// --- Undo stack cap (S30 + S33) ---

#[test]
fn undo_stack_capped_at_max_entries() {
    let mut tab = Tab::new(tid(1), pid(0));
    let all_pids: HashSet<PaneId> = (0..=35).map(pid).collect();

    // Push 35 trees (exceeds MAX_UNDO_ENTRIES = 32).
    for i in 1..=35u64 {
        tab.set_tree(crate::session::split_tree::SplitTree::leaf(pid(i)));
    }

    // Undo should succeed at most 32 times (the cap).
    let mut undo_count = 0;
    while tab.undo_tree(&all_pids) {
        undo_count += 1;
    }
    assert_eq!(undo_count, 32);
}

// --- Multi-step undo/redo walk (S33) ---

#[test]
fn multi_step_undo_redo_walk() {
    let mut tab = Tab::new(tid(1), pid(10));
    let live = HashSet::from([pid(10), pid(20), pid(30), pid(40)]);

    let tree_b = crate::session::split_tree::SplitTree::leaf(pid(20));
    let tree_c = crate::session::split_tree::SplitTree::leaf(pid(30));
    let tree_d = crate::session::split_tree::SplitTree::leaf(pid(40));

    tab.set_tree(tree_b);
    tab.set_tree(tree_c);
    tab.set_tree(tree_d);
    assert_eq!(tab.tree().panes(), vec![pid(40)]);

    // Undo 3 times: D → C → B → A(original).
    assert!(tab.undo_tree(&live));
    assert_eq!(tab.tree().panes(), vec![pid(30)]);
    assert!(tab.undo_tree(&live));
    assert_eq!(tab.tree().panes(), vec![pid(20)]);
    assert!(tab.undo_tree(&live));
    assert_eq!(tab.tree().panes(), vec![pid(10)]);

    // Redo 3 times: A → B → C → D.
    assert!(tab.redo_tree(&live));
    assert_eq!(tab.tree().panes(), vec![pid(20)]);
    assert!(tab.redo_tree(&live));
    assert_eq!(tab.tree().panes(), vec![pid(30)]);
    assert!(tab.redo_tree(&live));
    assert_eq!(tab.tree().panes(), vec![pid(40)]);
}

// --- Redo stale pane skip (S33) ---

#[test]
fn redo_skips_stale_entries() {
    let mut tab = Tab::new(tid(1), pid(10));
    let all_live = HashSet::from([pid(10), pid(20), pid(30)]);

    // Push trees B(pid(20)) and C(pid(30)).
    let tree_b = crate::session::split_tree::SplitTree::leaf(pid(20));
    let tree_c = crate::session::split_tree::SplitTree::leaf(pid(30));
    tab.set_tree(tree_b);
    tab.set_tree(tree_c);

    // Undo twice: C → B → A.
    assert!(tab.undo_tree(&all_live));
    assert!(tab.undo_tree(&all_live));
    assert_eq!(tab.tree().panes(), vec![pid(10)]);

    // Now pid(30) is "dead" — redo should skip tree C and apply tree B.
    let live_without_30 = HashSet::from([pid(10), pid(20)]);
    assert!(tab.redo_tree(&live_without_30));
    assert_eq!(tab.tree().panes(), vec![pid(20)]);

    // Next redo references pid(30) which is dead — should skip.
    assert!(!tab.redo_tree(&live_without_30));
}

// --- Zoom tests (S33) ---

#[test]
fn unzoom_clears_zoom() {
    let mut tab = Tab::new(tid(1), pid(10));
    tab.set_zoomed_pane(Some(pid(10)));
    assert_eq!(tab.zoomed_pane(), Some(pid(10)));

    tab.set_zoomed_pane(None);
    assert!(tab.zoomed_pane().is_none());
}

#[test]
fn unzoom_noop_when_not_zoomed() {
    let mut tab = Tab::new(tid(1), pid(10));
    assert!(tab.zoomed_pane().is_none());

    // Setting to None when already None is a no-op.
    tab.set_zoomed_pane(None);
    assert!(tab.zoomed_pane().is_none());
}

#[test]
fn close_zoomed_pane_clears_zoom_on_tree_remove() {
    let mut tab = Tab::new(tid(1), pid(10));
    let tree = tab.tree().split_at(
        pid(10),
        crate::session::SplitDirection::Vertical,
        pid(20),
        0.5,
    );
    tab.set_tree(tree);

    // Zoom pane 20.
    tab.set_zoomed_pane(Some(pid(20)));
    assert_eq!(tab.zoomed_pane(), Some(pid(20)));

    // "Close" pane 20 by removing from tree. The caller (App) clears zoom.
    // At the Tab level, we verify the tree removal works and zoom can be cleared.
    let new_tree = tab.tree().remove(pid(20)).expect("sibling exists");
    tab.set_tree(new_tree);
    tab.set_zoomed_pane(None);

    assert!(tab.zoomed_pane().is_none());
    assert!(!tab.tree().contains(pid(20)));
    assert_eq!(tab.tree().panes(), vec![pid(10)]);
}

// --- Float ↔ tile toggle (S33) ---

#[test]
fn float_to_tiled_removes_from_floating() {
    use crate::session::floating::FloatingPane;
    use crate::session::rect::Rect;

    let mut tab = Tab::new(tid(1), pid(10));
    let avail = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let fp = FloatingPane::centered(pid(20), &avail, 1);
    let layer = tab.floating().add(fp);
    tab.set_floating(layer);

    assert!(tab.is_floating(pid(20)));

    // Move from floating → tiled.
    let new_layer = tab.floating().remove(pid(20));
    tab.set_floating(new_layer);
    let new_tree = tab.tree().split_at(
        pid(10),
        crate::session::SplitDirection::Vertical,
        pid(20),
        0.5,
    );
    tab.set_tree(new_tree);

    assert!(!tab.is_floating(pid(20)));
    assert!(tab.tree().contains(pid(20)));
}

#[test]
fn tiled_to_floating_removes_from_tree() {
    use crate::session::floating::FloatingPane;
    use crate::session::rect::Rect;

    let mut tab = Tab::new(tid(1), pid(10));
    let tree = tab.tree().split_at(
        pid(10),
        crate::session::SplitDirection::Vertical,
        pid(20),
        0.5,
    );
    tab.set_tree(tree);
    assert!(tab.tree().contains(pid(20)));

    // Move from tiled → floating.
    let new_tree = tab.tree().remove(pid(20)).expect("has sibling");
    tab.set_tree(new_tree);
    let avail = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let fp = FloatingPane::centered(pid(20), &avail, 1);
    let layer = tab.floating().add(fp);
    tab.set_floating(layer);

    assert!(!tab.tree().contains(pid(20)));
    assert!(tab.is_floating(pid(20)));
}

#[test]
fn move_last_tiled_pane_to_floating_rejected() {
    let tab = Tab::new(tid(1), pid(10));

    // Removing the only tiled pane returns None (can't have empty tree).
    assert!(tab.tree().remove(pid(10)).is_none());
}

// --- Auto-unzoom on split (S33, Tab-level) ---

#[test]
fn split_after_zoom_produces_valid_tree() {
    let mut tab = Tab::new(tid(1), pid(10));
    tab.set_zoomed_pane(Some(pid(10)));

    // Caller (App) clears zoom before split. Simulate:
    tab.set_zoomed_pane(None);
    let new_tree = tab.tree().split_at(
        pid(10),
        crate::session::SplitDirection::Horizontal,
        pid(20),
        0.5,
    );
    tab.set_tree(new_tree);

    assert!(tab.zoomed_pane().is_none());
    assert_eq!(tab.tree().panes().len(), 2);
    assert!(tab.tree().contains(pid(10)));
    assert!(tab.tree().contains(pid(20)));
}
