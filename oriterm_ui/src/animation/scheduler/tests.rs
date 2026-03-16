use std::time::{Duration, Instant};

use super::RenderScheduler;
use crate::widget_id::WidgetId;

#[test]
fn scheduler_empty_has_no_pending_work() {
    let s = RenderScheduler::new();
    let now = Instant::now();
    assert!(!s.has_pending_work(now));
}

#[test]
fn scheduler_anim_frame_request_has_pending_work() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id = WidgetId::next();
    s.request_anim_frame(id);
    assert!(s.has_pending_work(now));
}

#[test]
fn scheduler_paint_request_has_pending_work() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id = WidgetId::next();
    s.request_paint(id);
    assert!(s.has_pending_work(now));
}

#[test]
fn scheduler_take_anim_frames_clears_set() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id1 = WidgetId::next();
    let id2 = WidgetId::next();
    s.request_anim_frame(id1);
    s.request_anim_frame(id2);

    let frames = s.take_anim_frames();
    assert_eq!(frames.len(), 2);
    assert!(frames.contains(&id1));
    assert!(frames.contains(&id2));
    assert!(!s.has_pending_work(now), "Should be empty after take");
}

#[test]
fn scheduler_deferred_repaint_before_wake_time_not_pending() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id = WidgetId::next();
    s.request_repaint_after(id, Duration::from_millis(500), now);

    assert!(!s.has_pending_work(now + Duration::from_millis(100)));
}

#[test]
fn scheduler_deferred_repaint_after_wake_time_is_pending() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id = WidgetId::next();
    s.request_repaint_after(id, Duration::from_millis(500), now);

    assert!(s.has_pending_work(now + Duration::from_millis(500)));
}

#[test]
fn scheduler_next_wake_time_returns_earliest() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id1 = WidgetId::next();
    let id2 = WidgetId::next();
    s.request_repaint_after(id1, Duration::from_millis(500), now);
    s.request_repaint_after(id2, Duration::from_millis(200), now);

    let wake = s.next_wake_time().unwrap();
    assert_eq!(wake, now + Duration::from_millis(200));
}

#[test]
fn scheduler_promote_deferred_moves_to_paint() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id1 = WidgetId::next();
    let id2 = WidgetId::next();
    s.request_repaint_after(id1, Duration::from_millis(100), now);
    s.request_repaint_after(id2, Duration::from_millis(500), now);

    // Promote at 200ms: only id1 should be promoted.
    s.promote_deferred(now + Duration::from_millis(200));

    let paints = s.take_paint_requests();
    assert!(paints.contains(&id1), "id1 should be promoted");
    assert!(!paints.contains(&id2), "id2 should not yet be promoted");

    // id2 is still deferred.
    assert_eq!(s.next_wake_time(), Some(now + Duration::from_millis(500)));
}

#[test]
fn scheduler_remove_widget_clears_all_requests() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id = WidgetId::next();
    s.request_anim_frame(id);
    s.request_paint(id);
    s.request_repaint_after(id, Duration::from_millis(100), now);

    s.remove_widget(id);

    assert!(!s.has_pending_work(now));

    // Deferred entry is lazily removed during promote.
    s.promote_deferred(now + Duration::from_millis(200));
    let paints = s.take_paint_requests();
    assert!(paints.is_empty(), "Removed widget should not be promoted");
}

#[test]
fn scheduler_multiple_deferred_ordered_correctly() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let id1 = WidgetId::next();
    let id2 = WidgetId::next();
    let id3 = WidgetId::next();

    // Add in non-chronological order.
    s.request_repaint_after(id3, Duration::from_millis(300), now);
    s.request_repaint_after(id1, Duration::from_millis(100), now);
    s.request_repaint_after(id2, Duration::from_millis(200), now);

    // Promote all at once.
    s.promote_deferred(now + Duration::from_millis(400));

    let paints = s.take_paint_requests();
    assert_eq!(paints.len(), 3, "All three should be promoted");
    assert!(paints.contains(&id1));
    assert!(paints.contains(&id2));
    assert!(paints.contains(&id3));
}
