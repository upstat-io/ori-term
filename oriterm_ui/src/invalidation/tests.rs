//! Tests for `DirtyKind` and `InvalidationTracker`.

use std::collections::HashMap;

use crate::controllers::ControllerRequests;
use crate::widget_id::WidgetId;

use super::{DirtyKind, InvalidationTracker};

/// Empty parent map for tests that don't need ancestor tracking.
fn no_parents() -> HashMap<WidgetId, WidgetId> {
    HashMap::new()
}

// DirtyKind ordering tests.

#[test]
fn dirty_kind_ordering() {
    assert!(DirtyKind::Clean < DirtyKind::Paint);
    assert!(DirtyKind::Paint < DirtyKind::Prepaint);
    assert!(DirtyKind::Prepaint < DirtyKind::Layout);
}

// DirtyKind::merge tests — all 16 combinations.

#[test]
fn merge_clean_clean() {
    assert_eq!(DirtyKind::Clean.merge(DirtyKind::Clean), DirtyKind::Clean);
}

#[test]
fn merge_clean_paint() {
    assert_eq!(DirtyKind::Clean.merge(DirtyKind::Paint), DirtyKind::Paint);
}

#[test]
fn merge_clean_prepaint() {
    assert_eq!(
        DirtyKind::Clean.merge(DirtyKind::Prepaint),
        DirtyKind::Prepaint
    );
}

#[test]
fn merge_clean_layout() {
    assert_eq!(DirtyKind::Clean.merge(DirtyKind::Layout), DirtyKind::Layout);
}

#[test]
fn merge_paint_clean() {
    assert_eq!(DirtyKind::Paint.merge(DirtyKind::Clean), DirtyKind::Paint);
}

#[test]
fn merge_paint_paint() {
    assert_eq!(DirtyKind::Paint.merge(DirtyKind::Paint), DirtyKind::Paint);
}

#[test]
fn merge_paint_prepaint() {
    assert_eq!(
        DirtyKind::Paint.merge(DirtyKind::Prepaint),
        DirtyKind::Prepaint
    );
}

#[test]
fn merge_paint_layout() {
    assert_eq!(DirtyKind::Paint.merge(DirtyKind::Layout), DirtyKind::Layout);
}

#[test]
fn merge_prepaint_clean() {
    assert_eq!(
        DirtyKind::Prepaint.merge(DirtyKind::Clean),
        DirtyKind::Prepaint
    );
}

#[test]
fn merge_prepaint_paint() {
    assert_eq!(
        DirtyKind::Prepaint.merge(DirtyKind::Paint),
        DirtyKind::Prepaint
    );
}

#[test]
fn merge_prepaint_prepaint() {
    assert_eq!(
        DirtyKind::Prepaint.merge(DirtyKind::Prepaint),
        DirtyKind::Prepaint
    );
}

#[test]
fn merge_prepaint_layout() {
    assert_eq!(
        DirtyKind::Prepaint.merge(DirtyKind::Layout),
        DirtyKind::Layout
    );
}

#[test]
fn merge_layout_clean() {
    assert_eq!(DirtyKind::Layout.merge(DirtyKind::Clean), DirtyKind::Layout);
}

#[test]
fn merge_layout_paint() {
    assert_eq!(DirtyKind::Layout.merge(DirtyKind::Paint), DirtyKind::Layout);
}

#[test]
fn merge_layout_prepaint() {
    assert_eq!(
        DirtyKind::Layout.merge(DirtyKind::Prepaint),
        DirtyKind::Layout
    );
}

#[test]
fn merge_layout_layout() {
    assert_eq!(
        DirtyKind::Layout.merge(DirtyKind::Layout),
        DirtyKind::Layout
    );
}

// DirtyKind::is_dirty tests.

#[test]
fn clean_is_not_dirty() {
    assert!(!DirtyKind::Clean.is_dirty());
}

#[test]
fn paint_is_dirty() {
    assert!(DirtyKind::Paint.is_dirty());
}

#[test]
fn prepaint_is_dirty() {
    assert!(DirtyKind::Prepaint.is_dirty());
}

#[test]
fn layout_is_dirty() {
    assert!(DirtyKind::Layout.is_dirty());
}

// ControllerRequests -> DirtyKind conversion tests.

#[test]
fn controller_requests_none_is_clean() {
    assert_eq!(DirtyKind::from(ControllerRequests::NONE), DirtyKind::Clean);
}

#[test]
fn controller_requests_paint_maps_to_paint() {
    assert_eq!(DirtyKind::from(ControllerRequests::PAINT), DirtyKind::Paint);
}

#[test]
fn controller_requests_other_flags_are_clean() {
    assert_eq!(
        DirtyKind::from(ControllerRequests::ANIM_FRAME),
        DirtyKind::Clean
    );
}

#[test]
fn controller_requests_paint_combined_is_paint() {
    let combined = ControllerRequests::PAINT.union(ControllerRequests::ANIM_FRAME);
    assert_eq!(DirtyKind::from(combined), DirtyKind::Paint);
}

// InvalidationTracker tests.

#[test]
fn new_tracker_is_clean() {
    let tracker = InvalidationTracker::new();
    assert!(!tracker.is_any_dirty());
    assert!(!tracker.needs_full_rebuild());
    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Clean);
}

#[test]
fn mark_layout_sets_layout_dirty() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Layout, &no_parents());

    assert!(tracker.is_layout_dirty(id));
    assert!(tracker.is_prepaint_dirty(id));
    assert!(tracker.is_paint_dirty(id));
    assert!(tracker.is_any_dirty());
    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Layout);
}

#[test]
fn mark_prepaint_sets_prepaint_dirty() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Prepaint, &no_parents());

    assert!(!tracker.is_layout_dirty(id));
    assert!(tracker.is_prepaint_dirty(id));
    assert!(tracker.is_paint_dirty(id));
    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Prepaint);
}

#[test]
fn mark_paint_sets_paint_dirty() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Paint, &no_parents());

    assert!(!tracker.is_layout_dirty(id));
    assert!(!tracker.is_prepaint_dirty(id));
    assert!(tracker.is_paint_dirty(id));
    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Paint);
}

#[test]
fn mark_clean_is_noop() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Clean, &no_parents());

    assert!(!tracker.is_layout_dirty(id));
    assert!(!tracker.is_any_dirty());
}

#[test]
fn mark_promotes_severity() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Paint, &no_parents());
    tracker.mark(id, DirtyKind::Prepaint, &no_parents());

    assert!(tracker.is_prepaint_dirty(id));
    assert!(!tracker.is_layout_dirty(id));
    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Prepaint);
}

#[test]
fn unmarked_widget_is_clean() {
    let mut tracker = InvalidationTracker::new();
    let marked = WidgetId::next();
    let unmarked = WidgetId::next();
    tracker.mark(marked, DirtyKind::Layout, &no_parents());

    assert!(!tracker.is_layout_dirty(unmarked));
}

#[test]
fn clear_resets_all_state() {
    let mut tracker = InvalidationTracker::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    tracker.mark(a, DirtyKind::Layout, &no_parents());
    tracker.mark(b, DirtyKind::Prepaint, &no_parents());
    tracker.invalidate_all();

    tracker.clear();

    assert!(!tracker.is_layout_dirty(a));
    assert!(!tracker.is_prepaint_dirty(b));
    assert!(!tracker.is_any_dirty());
    assert!(!tracker.needs_full_rebuild());
    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Clean);
}

#[test]
fn invalidate_all_marks_full_rebuild() {
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();

    assert!(tracker.needs_full_rebuild());
    assert!(tracker.is_any_dirty());
    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Layout);

    // Every widget reports dirty under full invalidation.
    let id = WidgetId::next();
    assert!(tracker.is_layout_dirty(id));
    assert!(tracker.is_prepaint_dirty(id));
    assert!(tracker.is_paint_dirty(id));
}

#[test]
fn max_dirty_kind_across_multiple_widgets() {
    let mut tracker = InvalidationTracker::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    let c = WidgetId::next();

    tracker.mark(a, DirtyKind::Paint, &no_parents());
    tracker.mark(b, DirtyKind::Prepaint, &no_parents());
    tracker.mark(c, DirtyKind::Paint, &no_parents());

    assert_eq!(tracker.max_dirty_kind(), DirtyKind::Prepaint);
}

#[test]
fn multiple_widgets_tracked_independently() {
    let mut tracker = InvalidationTracker::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    let c = WidgetId::next();

    tracker.mark(a, DirtyKind::Layout, &no_parents());

    assert!(tracker.is_layout_dirty(a));
    assert!(!tracker.is_layout_dirty(b));
    assert!(!tracker.is_layout_dirty(c));
}

// Ancestor tracking tests.

#[test]
fn mark_with_parents_propagates_ancestors() {
    let mut tracker = InvalidationTracker::new();
    let root = WidgetId::next();
    let mid = WidgetId::next();
    let leaf = WidgetId::next();

    // Build parent map: leaf -> mid -> root.
    let mut parents = HashMap::new();
    parents.insert(leaf, mid);
    parents.insert(mid, root);

    tracker.mark(leaf, DirtyKind::Prepaint, &parents);

    // Leaf is directly dirty.
    assert!(tracker.is_prepaint_dirty(leaf));
    // Ancestors report dirty descendants.
    assert!(tracker.has_dirty_descendant(mid));
    assert!(tracker.has_dirty_descendant(root));
    // Unrelated widget is clean.
    let unrelated = WidgetId::next();
    assert!(!tracker.has_dirty_descendant(unrelated));
}

#[test]
fn has_dirty_descendant_returns_true_for_dirty_widget_itself() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Paint, &no_parents());

    // The widget itself is dirty, so has_dirty_descendant includes it.
    assert!(tracker.has_dirty_descendant(id));
}

#[test]
fn has_dirty_descendant_false_when_clean() {
    let tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    assert!(!tracker.has_dirty_descendant(id));
}

#[test]
fn clear_resets_dirty_ancestors() {
    let mut tracker = InvalidationTracker::new();
    let root = WidgetId::next();
    let leaf = WidgetId::next();

    let mut parents = HashMap::new();
    parents.insert(leaf, root);

    tracker.mark(leaf, DirtyKind::Prepaint, &parents);
    assert!(tracker.has_dirty_descendant(root));

    tracker.clear();

    assert!(!tracker.has_dirty_descendant(root));
    assert!(!tracker.has_dirty_descendant(leaf));
}

#[test]
fn ancestor_propagation_stops_at_already_marked_ancestor() {
    let mut tracker = InvalidationTracker::new();
    let root = WidgetId::next();
    let mid = WidgetId::next();
    let leaf_a = WidgetId::next();
    let leaf_b = WidgetId::next();

    // Tree: leaf_a -> mid -> root, leaf_b -> mid -> root.
    let mut parents = HashMap::new();
    parents.insert(leaf_a, mid);
    parents.insert(leaf_b, mid);
    parents.insert(mid, root);

    // First mark propagates all the way up.
    tracker.mark(leaf_a, DirtyKind::Prepaint, &parents);
    // Second mark stops early at mid (already in dirty_ancestors).
    tracker.mark(leaf_b, DirtyKind::Paint, &parents);

    assert!(tracker.has_dirty_descendant(root));
    assert!(tracker.has_dirty_descendant(mid));
    assert!(tracker.is_prepaint_dirty(leaf_a));
    assert!(tracker.is_paint_dirty(leaf_b));
}
