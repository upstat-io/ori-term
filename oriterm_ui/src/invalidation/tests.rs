//! Tests for `DirtyKind` and `InvalidationTracker`.

use crate::input::EventResponse;
use crate::widget_id::WidgetId;

use super::{DirtyKind, InvalidationTracker};

// DirtyKind::merge tests.

#[test]
fn merge_clean_clean() {
    assert_eq!(DirtyKind::Clean.merge(DirtyKind::Clean), DirtyKind::Clean);
}

#[test]
fn merge_clean_paint() {
    assert_eq!(DirtyKind::Clean.merge(DirtyKind::Paint), DirtyKind::Paint);
}

#[test]
fn merge_paint_clean() {
    assert_eq!(DirtyKind::Paint.merge(DirtyKind::Clean), DirtyKind::Paint);
}

#[test]
fn merge_paint_layout() {
    assert_eq!(DirtyKind::Paint.merge(DirtyKind::Layout), DirtyKind::Layout);
}

#[test]
fn merge_layout_paint() {
    assert_eq!(DirtyKind::Layout.merge(DirtyKind::Paint), DirtyKind::Layout);
}

#[test]
fn merge_layout_layout() {
    assert_eq!(
        DirtyKind::Layout.merge(DirtyKind::Layout),
        DirtyKind::Layout
    );
}

#[test]
fn merge_paint_paint() {
    assert_eq!(DirtyKind::Paint.merge(DirtyKind::Paint), DirtyKind::Paint);
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
fn layout_is_dirty() {
    assert!(DirtyKind::Layout.is_dirty());
}

// EventResponse → DirtyKind conversion tests.

#[test]
fn event_response_handled_is_clean() {
    assert_eq!(DirtyKind::from(EventResponse::Handled), DirtyKind::Clean);
}

#[test]
fn event_response_ignored_is_clean() {
    assert_eq!(DirtyKind::from(EventResponse::Ignored), DirtyKind::Clean);
}

#[test]
fn event_response_request_paint_is_paint() {
    assert_eq!(
        DirtyKind::from(EventResponse::RequestPaint),
        DirtyKind::Paint
    );
}

#[test]
fn event_response_request_layout_is_layout() {
    assert_eq!(
        DirtyKind::from(EventResponse::RequestLayout),
        DirtyKind::Layout
    );
}

#[test]
fn event_response_request_focus_is_paint() {
    assert_eq!(
        DirtyKind::from(EventResponse::RequestFocus),
        DirtyKind::Paint
    );
}

// InvalidationTracker tests.

#[test]
fn new_tracker_is_clean() {
    let tracker = InvalidationTracker::new();
    assert!(!tracker.is_any_dirty());
    assert!(!tracker.needs_full_rebuild());
}

#[test]
fn mark_paint_sets_paint_dirty() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Paint);

    assert!(tracker.is_paint_dirty(id));
    assert!(!tracker.is_layout_dirty(id));
    assert!(tracker.is_any_dirty());
}

#[test]
fn mark_layout_sets_layout_dirty() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Layout);

    assert!(tracker.is_layout_dirty(id));
    // Layout-dirty implies paint-dirty.
    assert!(tracker.is_paint_dirty(id));
    assert!(tracker.is_any_dirty());
}

#[test]
fn mark_clean_is_noop() {
    let mut tracker = InvalidationTracker::new();
    let id = WidgetId::next();
    tracker.mark(id, DirtyKind::Clean);

    assert!(!tracker.is_paint_dirty(id));
    assert!(!tracker.is_layout_dirty(id));
    assert!(!tracker.is_any_dirty());
}

#[test]
fn unmarked_widget_is_clean() {
    let mut tracker = InvalidationTracker::new();
    let marked = WidgetId::next();
    let unmarked = WidgetId::next();
    tracker.mark(marked, DirtyKind::Paint);

    assert!(!tracker.is_paint_dirty(unmarked));
    assert!(!tracker.is_layout_dirty(unmarked));
}

#[test]
fn clear_resets_all_state() {
    let mut tracker = InvalidationTracker::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    tracker.mark(a, DirtyKind::Paint);
    tracker.mark(b, DirtyKind::Layout);
    tracker.invalidate_all();

    tracker.clear();

    assert!(!tracker.is_paint_dirty(a));
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
    assert!(tracker.is_paint_dirty(id));
    assert!(tracker.is_layout_dirty(id));
}

#[test]
fn multiple_widgets_tracked_independently() {
    let mut tracker = InvalidationTracker::new();
    let a = WidgetId::next();
    let b = WidgetId::next();
    let c = WidgetId::next();

    tracker.mark(a, DirtyKind::Paint);
    tracker.mark(b, DirtyKind::Layout);

    assert!(tracker.is_paint_dirty(a));
    assert!(!tracker.is_layout_dirty(a));
    assert!(tracker.is_paint_dirty(b));
    assert!(tracker.is_layout_dirty(b));
    assert!(!tracker.is_paint_dirty(c));
    assert!(!tracker.is_layout_dirty(c));
}
