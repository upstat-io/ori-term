//! Tests for the interaction state system.

use std::collections::HashMap;

use crate::focus::FocusManager;
use crate::geometry::{Point, Rect};
use crate::input::layout_hit_test_path;
use crate::layout::LayoutNode;
use crate::widget_id::WidgetId;

use super::build_parent_map;
use super::lifecycle::LifecycleEvent;
use super::manager::{InteractionManager, InteractionState};

// --- InteractionState ---

#[test]
fn interaction_state_new_all_false() {
    let state = InteractionState::new();
    assert!(!state.is_hot());
    assert!(!state.is_hot_direct());
    assert!(!state.is_active());
    assert!(!state.is_focused());
    assert!(!state.has_focus_within());
    assert!(!state.is_disabled());
}

#[test]
fn interaction_state_disabled_constructor() {
    let state = InteractionState::disabled();
    assert!(state.is_disabled());
    assert!(!state.is_hot());
    assert!(!state.is_active());
    assert!(!state.is_focused());
}

// --- InteractionManager: registration ---

#[test]
fn register_widget_adds_state() {
    let mut mgr = InteractionManager::new();
    let id = WidgetId::next();
    mgr.register_widget(id);

    let state = mgr.get_state(id);
    assert!(!state.is_hot());
    assert!(!state.is_active());

    let events = mgr.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], LifecycleEvent::WidgetAdded { widget_id: id });
}

#[test]
fn register_widget_is_idempotent() {
    let mut mgr = InteractionManager::new();
    let id = WidgetId::next();
    mgr.register_widget(id);
    mgr.drain_events();

    mgr.register_widget(id);
    let events = mgr.drain_events();
    // Still emits WidgetAdded (idempotent on state, not on events).
    assert_eq!(events.len(), 1);
}

#[test]
fn get_state_unregistered_returns_default() {
    let mgr = InteractionManager::new();
    let id = WidgetId::next();
    let state = mgr.get_state(id);
    assert!(!state.is_hot());
    assert!(!state.is_active());
    assert!(!state.is_focused());
}

// --- InteractionManager: hot path ---

#[test]
fn update_hot_path_three_level_nesting() {
    let mut mgr = InteractionManager::new();
    let root = WidgetId::next();
    let mid = WidgetId::next();
    let leaf = WidgetId::next();

    mgr.register_widget(root);
    mgr.register_widget(mid);
    mgr.register_widget(leaf);
    mgr.drain_events();

    // Move pointer over the nested hierarchy.
    mgr.update_hot_path(&[root, mid, leaf]);

    assert!(mgr.get_state(root).is_hot());
    assert!(!mgr.get_state(root).is_hot_direct());
    assert!(mgr.get_state(mid).is_hot());
    assert!(!mgr.get_state(mid).is_hot_direct());
    assert!(mgr.get_state(leaf).is_hot());
    assert!(mgr.get_state(leaf).is_hot_direct());

    let events = mgr.drain_events();
    // Three HotChanged(true) events — one per widget.
    let hot_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, LifecycleEvent::HotChanged { is_hot: true, .. }))
        .collect();
    assert_eq!(hot_events.len(), 3);
}

#[test]
fn update_hot_path_leaves_generate_hot_changed_false() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    let b = WidgetId::next();

    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.drain_events();

    // Pointer over A.
    mgr.update_hot_path(&[a]);
    mgr.drain_events();

    // Pointer moves to B (leaves A).
    mgr.update_hot_path(&[b]);

    assert!(!mgr.get_state(a).is_hot());
    assert!(mgr.get_state(b).is_hot());

    let events = mgr.drain_events();
    assert!(events.contains(&LifecycleEvent::HotChanged {
        widget_id: a,
        is_hot: false,
    }));
    assert!(events.contains(&LifecycleEvent::HotChanged {
        widget_id: b,
        is_hot: true,
    }));
}

#[test]
fn update_hot_path_empty_clears_all() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.update_hot_path(&[a]);
    mgr.drain_events();

    // Pointer leaves.
    mgr.update_hot_path(&[]);

    assert!(!mgr.get_state(a).is_hot());
    let events = mgr.drain_events();
    assert!(events.contains(&LifecycleEvent::HotChanged {
        widget_id: a,
        is_hot: false,
    }));
}

// --- InteractionManager: active ---

#[test]
fn set_active_displaces_previous() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.drain_events();

    mgr.set_active(a);
    assert!(mgr.get_state(a).is_active());
    mgr.drain_events();

    // B becomes active — A should be deactivated.
    mgr.set_active(b);
    assert!(!mgr.get_state(a).is_active());
    assert!(mgr.get_state(b).is_active());

    let events = mgr.drain_events();
    assert_eq!(
        events[0],
        LifecycleEvent::ActiveChanged {
            widget_id: a,
            is_active: false,
        }
    );
    assert_eq!(
        events[1],
        LifecycleEvent::ActiveChanged {
            widget_id: b,
            is_active: true,
        }
    );
}

#[test]
fn set_active_same_widget_is_noop() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.set_active(a);
    mgr.drain_events();

    mgr.set_active(a);
    assert!(mgr.drain_events().is_empty());
}

#[test]
fn clear_active() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.set_active(a);
    mgr.drain_events();

    mgr.clear_active();
    assert!(!mgr.get_state(a).is_active());
    assert_eq!(mgr.active_widget(), None);

    let events = mgr.drain_events();
    assert_eq!(
        events[0],
        LifecycleEvent::ActiveChanged {
            widget_id: a,
            is_active: false,
        }
    );
}

// --- InteractionManager: focus ---

#[test]
fn request_focus_syncs_with_focus_manager() {
    let mut mgr = InteractionManager::new();
    let mut fm = FocusManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.request_focus(a, &mut fm);

    assert!(mgr.get_state(a).is_focused());
    assert_eq!(mgr.focused_widget(), Some(a));
    assert_eq!(fm.focused(), Some(a));

    let events = mgr.drain_events();
    assert!(events.contains(&LifecycleEvent::FocusChanged {
        widget_id: a,
        is_focused: true,
    }));
}

#[test]
fn request_focus_transfers_from_old() {
    let mut mgr = InteractionManager::new();
    let mut fm = FocusManager::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.drain_events();

    mgr.request_focus(a, &mut fm);
    mgr.drain_events();

    mgr.request_focus(b, &mut fm);
    assert!(!mgr.get_state(a).is_focused());
    assert!(mgr.get_state(b).is_focused());
    assert_eq!(fm.focused(), Some(b));

    let events = mgr.drain_events();
    assert_eq!(
        events[0],
        LifecycleEvent::FocusChanged {
            widget_id: a,
            is_focused: false,
        }
    );
    assert_eq!(
        events[1],
        LifecycleEvent::FocusChanged {
            widget_id: b,
            is_focused: true,
        }
    );
}

#[test]
fn request_focus_same_widget_is_noop() {
    let mut mgr = InteractionManager::new();
    let mut fm = FocusManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.request_focus(a, &mut fm);
    mgr.drain_events();

    mgr.request_focus(a, &mut fm);
    assert!(mgr.drain_events().is_empty());
}

#[test]
fn clear_focus_syncs_with_focus_manager() {
    let mut mgr = InteractionManager::new();
    let mut fm = FocusManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.request_focus(a, &mut fm);
    mgr.drain_events();

    mgr.clear_focus(&mut fm);
    assert!(!mgr.get_state(a).is_focused());
    assert_eq!(mgr.focused_widget(), None);
    assert_eq!(fm.focused(), None);
}

// --- InteractionManager: focus_within ---

#[test]
fn focus_within_propagates_to_ancestors() {
    let mut mgr = InteractionManager::new();
    let mut fm = FocusManager::new();
    let root = WidgetId::next();
    let mid = WidgetId::next();
    let leaf = WidgetId::next();
    mgr.register_widget(root);
    mgr.register_widget(mid);
    mgr.register_widget(leaf);
    mgr.drain_events();

    // Set up parent map: leaf → mid → root.
    let mut parent_map = HashMap::new();
    parent_map.insert(leaf, mid);
    parent_map.insert(mid, root);
    mgr.set_parent_map(parent_map);

    // Focus the leaf.
    mgr.request_focus(leaf, &mut fm);

    assert!(mgr.get_state(leaf).is_focused());
    assert!(mgr.get_state(mid).has_focus_within());
    assert!(mgr.get_state(root).has_focus_within());
    // Leaf itself does NOT have focus_within (only ancestors).
    assert!(!mgr.get_state(leaf).has_focus_within());
}

#[test]
fn focus_within_clears_on_focus_transfer() {
    let mut mgr = InteractionManager::new();
    let mut fm = FocusManager::new();
    let root = WidgetId::next();
    let branch_a = WidgetId::next();
    let leaf_a = WidgetId::next();
    let branch_b = WidgetId::next();
    let leaf_b = WidgetId::next();

    mgr.register_widget(root);
    mgr.register_widget(branch_a);
    mgr.register_widget(leaf_a);
    mgr.register_widget(branch_b);
    mgr.register_widget(leaf_b);
    mgr.drain_events();

    // Tree: root → branch_a → leaf_a, root → branch_b → leaf_b.
    let mut parent_map = HashMap::new();
    parent_map.insert(leaf_a, branch_a);
    parent_map.insert(branch_a, root);
    parent_map.insert(leaf_b, branch_b);
    parent_map.insert(branch_b, root);
    mgr.set_parent_map(parent_map);

    // Focus leaf_a.
    mgr.request_focus(leaf_a, &mut fm);
    assert!(mgr.get_state(branch_a).has_focus_within());
    assert!(mgr.get_state(root).has_focus_within());

    // Transfer focus to leaf_b.
    mgr.request_focus(leaf_b, &mut fm);
    assert!(!mgr.get_state(branch_a).has_focus_within());
    assert!(mgr.get_state(branch_b).has_focus_within());
    // Root still has focus_within (descendant is focused).
    assert!(mgr.get_state(root).has_focus_within());
}

// --- InteractionManager: disabled ---

#[test]
fn set_disabled_emits_lifecycle_event() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.set_disabled(a, true);
    assert!(mgr.get_state(a).is_disabled());

    let events = mgr.drain_events();
    assert_eq!(
        events[0],
        LifecycleEvent::WidgetDisabled {
            widget_id: a,
            disabled: true,
        }
    );
}

#[test]
fn set_disabled_same_value_is_noop() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.set_disabled(a, false);
    assert!(mgr.drain_events().is_empty());
}

// --- InteractionManager: deregister ---

#[test]
fn deregister_widget_clears_hot_path() {
    let mut mgr = InteractionManager::new();
    let root = WidgetId::next();
    let leaf = WidgetId::next();
    mgr.register_widget(root);
    mgr.register_widget(leaf);
    mgr.drain_events();

    mgr.update_hot_path(&[root, leaf]);
    mgr.drain_events();

    // Deregister the leaf — should clear it from hot path.
    mgr.deregister_widget(leaf);

    let events = mgr.drain_events();
    assert!(events.contains(&LifecycleEvent::HotChanged {
        widget_id: leaf,
        is_hot: false,
    }));
    assert!(events.contains(&LifecycleEvent::WidgetRemoved { widget_id: leaf }));

    // Root should still be hot (not deregistered).
    assert!(mgr.get_state(root).is_hot());
}

#[test]
fn deregister_active_widget_clears_active() {
    let mut mgr = InteractionManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.set_active(a);
    mgr.drain_events();

    mgr.deregister_widget(a);
    assert_eq!(mgr.active_widget(), None);

    let events = mgr.drain_events();
    assert!(events.contains(&LifecycleEvent::ActiveChanged {
        widget_id: a,
        is_active: false,
    }));
}

#[test]
fn deregister_focused_widget_clears_focus() {
    let mut mgr = InteractionManager::new();
    let mut fm = FocusManager::new();
    let a = WidgetId::next();
    mgr.register_widget(a);
    mgr.drain_events();

    mgr.request_focus(a, &mut fm);
    mgr.drain_events();

    mgr.deregister_widget(a);
    assert_eq!(mgr.focused_widget(), None);

    let events = mgr.drain_events();
    assert!(events.contains(&LifecycleEvent::FocusChanged {
        widget_id: a,
        is_focused: false,
    }));
}

// --- parent_map ---

#[test]
fn build_parent_map_three_level_tree() {
    let root_id = WidgetId::next();
    let mid_id = WidgetId::next();
    let leaf_id = WidgetId::next();

    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    let leaf = LayoutNode::new(r, r).with_widget_id(leaf_id);
    let mid = LayoutNode::new(r, r)
        .with_children(vec![leaf])
        .with_widget_id(mid_id);
    let root = LayoutNode::new(r, r)
        .with_children(vec![mid])
        .with_widget_id(root_id);

    let map = build_parent_map(&root);
    assert_eq!(map.get(&leaf_id), Some(&mid_id));
    assert_eq!(map.get(&mid_id), Some(&root_id));
    assert_eq!(map.get(&root_id), None); // Root has no parent.
}

#[test]
fn build_parent_map_skips_nodes_without_widget_id() {
    let root_id = WidgetId::next();
    let leaf_id = WidgetId::next();

    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    // Middle node has no widget_id.
    let leaf = LayoutNode::new(r, r).with_widget_id(leaf_id);
    let mid_no_id = LayoutNode::new(r, r).with_children(vec![leaf]);
    let root = LayoutNode::new(r, r)
        .with_children(vec![mid_no_id])
        .with_widget_id(root_id);

    let map = build_parent_map(&root);
    // Leaf's parent should be root (skipping the id-less intermediate node).
    assert_eq!(map.get(&leaf_id), Some(&root_id));
}

// --- layout_hit_test_path ---

#[test]
fn hit_test_path_three_level_nesting() {
    let root_id = WidgetId::next();
    let mid_id = WidgetId::next();
    let leaf_id = WidgetId::next();

    let root_rect = Rect::new(0.0, 0.0, 200.0, 200.0);
    let mid_rect = Rect::new(10.0, 10.0, 180.0, 180.0);
    let leaf_rect = Rect::new(20.0, 20.0, 50.0, 50.0);

    let leaf = LayoutNode::new(leaf_rect, leaf_rect).with_widget_id(leaf_id);
    let mid = LayoutNode::new(mid_rect, mid_rect)
        .with_children(vec![leaf])
        .with_widget_id(mid_id);
    let root = LayoutNode::new(root_rect, root_rect)
        .with_children(vec![mid])
        .with_widget_id(root_id);

    // Point inside leaf.
    let path = layout_hit_test_path(&root, Point::new(30.0, 30.0));
    assert_eq!(path, vec![root_id, mid_id, leaf_id]);
}

#[test]
fn hit_test_path_miss_returns_empty() {
    let root_id = WidgetId::next();
    let root_rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let root = LayoutNode::new(root_rect, root_rect).with_widget_id(root_id);

    let path = layout_hit_test_path(&root, Point::new(200.0, 200.0));
    assert!(path.is_empty());
}

#[test]
fn hit_test_path_leaf_only() {
    let root_id = WidgetId::next();
    let root_rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let root = LayoutNode::new(root_rect, root_rect).with_widget_id(root_id);

    let path = layout_hit_test_path(&root, Point::new(50.0, 50.0));
    assert_eq!(path, vec![root_id]);
}

#[test]
fn hit_test_path_skips_nodes_without_widget_id() {
    let root_id = WidgetId::next();
    let leaf_id = WidgetId::next();

    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    let leaf = LayoutNode::new(r, r).with_widget_id(leaf_id);
    let mid_no_id = LayoutNode::new(r, r).with_children(vec![leaf]);
    let root = LayoutNode::new(r, r)
        .with_children(vec![mid_no_id])
        .with_widget_id(root_id);

    let path = layout_hit_test_path(&root, Point::new(50.0, 50.0));
    assert_eq!(path, vec![root_id, leaf_id]);
}

#[test]
fn hit_test_path_frontmost_child_wins() {
    let root_id = WidgetId::next();
    let back_id = WidgetId::next();
    let front_id = WidgetId::next();

    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    // Both children overlap at the same position, front is last in children vec.
    let back = LayoutNode::new(r, r).with_widget_id(back_id);
    let front = LayoutNode::new(r, r).with_widget_id(front_id);
    let root = LayoutNode::new(r, r)
        .with_children(vec![back, front])
        .with_widget_id(root_id);

    let path = layout_hit_test_path(&root, Point::new(50.0, 50.0));
    assert_eq!(path, vec![root_id, front_id]);
}
