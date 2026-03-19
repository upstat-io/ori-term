//! Tests for `DirtyKind` and `InvalidationTracker`.

use crate::controllers::ControllerRequests;
use crate::widget_id::WidgetId;

use super::{DirtyKind, InvalidationTracker};

// DirtyKind::merge tests.

#[test]
fn merge_clean_clean() {
    assert_eq!(DirtyKind::Clean.merge(DirtyKind::Clean), DirtyKind::Clean);
}

#[test]
fn merge_clean_layout() {
    assert_eq!(DirtyKind::Clean.merge(DirtyKind::Layout), DirtyKind::Layout);
}

#[test]
fn merge_layout_clean() {
    assert_eq!(DirtyKind::Layout.merge(DirtyKind::Clean), DirtyKind::Layout);
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
fn layout_is_dirty() {
    assert!(DirtyKind::Layout.is_dirty());
}

// ControllerRequests -> DirtyKind conversion tests.

#[test]
fn controller_requests_none_is_clean() {
    assert_eq!(DirtyKind::from(ControllerRequests::NONE), DirtyKind::Clean);
}

#[test]
fn controller_requests_paint_maps_to_clean() {
    // Paint-only invalidation is handled by full-scene rebuild + damage
    // diffing, so PAINT flag maps to Clean (no per-widget tracking needed).
    assert_eq!(DirtyKind::from(ControllerRequests::PAINT), DirtyKind::Clean);
}

#[test]
fn controller_requests_other_flags_are_clean() {
    // Non-paint flags (e.g. ANIM_FRAME, SET_ACTIVE) don't imply layout dirty.
    assert_eq!(
        DirtyKind::from(ControllerRequests::ANIM_FRAME),
        DirtyKind::Clean
    );
}

#[test]
fn controller_requests_paint_combined_is_clean() {
    // PAINT combined with other flags still yields Clean (no per-widget paint tracking).
    let combined = ControllerRequests::PAINT.union(ControllerRequests::ANIM_FRAME);
    assert_eq!(DirtyKind::from(combined), DirtyKind::Clean);
}

// InvalidationTracker tests.

#[test]
fn new_tracker_is_clean() {
    let tracker = InvalidationTracker::new();
    assert!(!tracker.is_any_dirty());
    assert!(!tracker.needs_full_rebuild());
}

#[test]
fn mark_layout_sets_layout_dirty() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Layout);

    assert!(tracker.is_layout_dirty(id));
    assert!(tracker.is_any_dirty());
}

#[test]
fn mark_clean_is_noop() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Clean);

    assert!(!tracker.is_layout_dirty(id));
    assert!(!tracker.is_any_dirty());
}

#[test]
fn unmarked_widget_is_clean() {
    let mut tracker = InvalidationTracker::new();
    let marked = WidgetId::next();
    let unmarked = WidgetId::next();
    tracker.mark(marked, DirtyKind::Layout);

    assert!(!tracker.is_layout_dirty(unmarked));
}

#[test]
fn clear_resets_all_state() {
    let mut tracker = InvalidationTracker::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    tracker.mark(a, DirtyKind::Layout);
    tracker.mark(b, DirtyKind::Layout);
    tracker.invalidate_all();

    tracker.clear();

    assert!(!tracker.is_layout_dirty(a));
    assert!(!tracker.is_layout_dirty(b));
    assert!(!tracker.is_any_dirty());
    assert!(!tracker.needs_full_rebuild());
}

#[test]
fn invalidate_all_marks_full_rebuild() {
    let mut tracker = InvalidationTracker::new();
    tracker.invalidate_all();

    assert!(tracker.needs_full_rebuild());
    assert!(tracker.is_any_dirty());

    // Every widget reports dirty under full invalidation.
    let id = WidgetId::next();
    assert!(tracker.is_layout_dirty(id));
}

#[test]
fn multiple_widgets_tracked_independently() {
    let mut tracker = InvalidationTracker::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    let c = WidgetId::next();

    tracker.mark(a, DirtyKind::Layout);

    assert!(tracker.is_layout_dirty(a));
    assert!(!tracker.is_layout_dirty(b));
    assert!(!tracker.is_layout_dirty(c));
}
