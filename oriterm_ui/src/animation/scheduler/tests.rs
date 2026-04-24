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

/// `set_animation_deadline(key, Some(t))` surfaces `t` as `next_wake_time()`
/// when no other wake source is pending. Semantic pin for the per-key
/// animation deadline API used by kitty graphics animation ticks.
/// Regression: §13.3 timer wiring.
#[test]
fn set_animation_deadline_some_surfaces_as_next_wake_time() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let deadline = now + Duration::from_millis(100);

    s.set_animation_deadline(42, Some(deadline));
    assert_eq!(
        s.next_wake_time(),
        Some(deadline),
        "set_animation_deadline MUST surface the Some(t) instant via next_wake_time"
    );
}

/// `set_animation_deadline(key, None)` clears the prior deadline for that
/// key — when an animation stops, the queued wake MUST NOT fire spuriously.
/// Regression: §13.3 TPR round 1 codex F2 — append-only push(Reverse(t))
/// left stop notifications unable to cancel a queued wake.
#[test]
fn set_animation_deadline_none_clears_prior_deadline_for_that_key() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let deadline = now + Duration::from_millis(100);

    s.set_animation_deadline(42, Some(deadline));
    assert_eq!(s.next_wake_time(), Some(deadline));

    s.set_animation_deadline(42, None);
    assert_eq!(
        s.next_wake_time(),
        None,
        "None MUST remove the prior deadline so a stopped animation does \
         not fire a spurious wake"
    );
}

/// REPLACEMENT semantics: calling set_animation_deadline twice with the
/// same key overwrites — no heap growth under animation start/stop churn.
#[test]
fn set_animation_deadline_same_key_replaces_instead_of_accumulating() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let first = now + Duration::from_millis(50);
    let second = now + Duration::from_millis(200);

    s.set_animation_deadline(7, Some(first));
    s.set_animation_deadline(7, Some(second));
    assert_eq!(
        s.next_wake_time(),
        Some(second),
        "second Some(t) for the same key MUST replace, not accumulate"
    );
}

/// Distinct keys are kept independently — two panes with overlapping
/// animations each contribute their own deadline; `next_wake_time` returns
/// the min.
#[test]
fn set_animation_deadline_distinct_keys_min_across_panes() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let pane_a_deadline = now + Duration::from_millis(200);
    let pane_b_deadline = now + Duration::from_millis(50);

    s.set_animation_deadline(1, Some(pane_a_deadline));
    s.set_animation_deadline(2, Some(pane_b_deadline));
    assert_eq!(
        s.next_wake_time(),
        Some(pane_b_deadline),
        "min MUST span multiple pane keys"
    );

    // Clearing pane B leaves pane A's deadline.
    s.set_animation_deadline(2, None);
    assert_eq!(s.next_wake_time(), Some(pane_a_deadline));
}

/// When both the widget-bound deferred-repaint heap and the per-key
/// animation deadline map have entries, `next_wake_time()` returns the
/// minimum — proves the two wake sources are composed correctly.
/// Regression: ensures cursor blink wakes and kitty animation wakes both
/// participate in the event-loop `WaitUntil` computation.
#[test]
fn next_wake_time_returns_min_of_widget_deferred_and_animation_deadline() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let earlier = now + Duration::from_millis(50);
    let id = WidgetId::next();

    // Widget deferred is later; animation deadline is earlier.
    s.request_repaint_after(id, Duration::from_millis(200), now);
    s.set_animation_deadline(99, Some(earlier));
    assert_eq!(s.next_wake_time(), Some(earlier));

    // Now push a widget deferred that beats the animation deadline.
    let even_earlier = now + Duration::from_millis(10);
    let id2 = WidgetId::next();
    s.request_repaint_after(id2, Duration::from_millis(10), now);
    assert_eq!(
        s.next_wake_time(),
        Some(even_earlier),
        "min MUST span both wake sources"
    );
}

/// `has_pending_work` returns true when a matured animation deadline is
/// ready — the event loop consults this to decide whether to start a frame.
#[test]
fn has_pending_work_true_when_animation_deadline_matured() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let past = now - Duration::from_millis(1);

    s.set_animation_deadline(1, Some(past));
    assert!(
        s.has_pending_work(now),
        "matured animation deadline MUST mark work pending"
    );
}

/// `has_pending_work` returns false when the only entry is a future
/// animation deadline — the negative pin that proves the readiness check
/// is time-gated.
#[test]
fn has_pending_work_false_when_animation_deadline_in_future() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    let future = now + Duration::from_millis(500);

    s.set_animation_deadline(1, Some(future));
    assert!(
        !s.has_pending_work(now),
        "future-only animation deadline MUST NOT mark work pending"
    );
}

/// `promote_deferred` drains matured animation deadlines so an animation
/// that stopped exactly at its deadline does not leave a stale entry in
/// the map. Idempotent: a second call at a later instant is a no-op once
/// nothing is pending.
#[test]
fn promote_deferred_drains_matured_animation_deadlines() {
    let mut s = RenderScheduler::new();
    let now = Instant::now();
    s.set_animation_deadline(1, Some(now - Duration::from_millis(1)));
    s.set_animation_deadline(2, Some(now - Duration::from_millis(2)));

    s.promote_deferred(now);
    assert_eq!(
        s.next_wake_time(),
        None,
        "both matured animation deadlines MUST be drained"
    );

    // Idempotency: re-running does not resurrect anything.
    s.promote_deferred(now + Duration::from_millis(10));
    assert_eq!(s.next_wake_time(), None);
}
