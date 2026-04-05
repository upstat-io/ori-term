use std::time::{Duration, Instant};

use super::{
    HIDDEN_PUSH_INTERVAL, SNAPSHOT_PUSH_INTERVAL, VISIBLE_PUSH_INTERVAL, interval_for_priority,
    should_push,
};

#[test]
fn throttled_within_interval() {
    let now = Instant::now();
    let last = now - Duration::from_millis(2);
    assert!(!should_push(now, Some(last), SNAPSHOT_PUSH_INTERVAL));
}

#[test]
fn unthrottled_past_interval() {
    let now = Instant::now();
    let last = now - Duration::from_millis(20);
    assert!(should_push(now, Some(last), SNAPSHOT_PUSH_INTERVAL));
}

#[test]
fn first_push_always_allowed() {
    let now = Instant::now();
    assert!(should_push(now, None, SNAPSHOT_PUSH_INTERVAL));
}

#[test]
fn should_push_respects_custom_interval() {
    let now = Instant::now();

    // 10ms ago with a 16ms visible interval: should NOT push.
    let last = now - Duration::from_millis(10);
    assert!(!should_push(now, Some(last), VISIBLE_PUSH_INTERVAL));

    // 20ms ago with a 16ms visible interval: should push.
    let last = now - Duration::from_millis(20);
    assert!(should_push(now, Some(last), VISIBLE_PUSH_INTERVAL));

    // 50ms ago with a 100ms hidden interval: should NOT push.
    let last = now - Duration::from_millis(50);
    assert!(!should_push(now, Some(last), HIDDEN_PUSH_INTERVAL));

    // 110ms ago with a 100ms hidden interval: should push.
    let last = now - Duration::from_millis(110);
    assert!(should_push(now, Some(last), HIDDEN_PUSH_INTERVAL));
}

#[test]
fn default_priority_is_focused() {
    // Priority 0 (focused) maps to the 4ms interval.
    assert_eq!(interval_for_priority(0), SNAPSHOT_PUSH_INTERVAL);
}

#[test]
fn interval_for_priority_tiers() {
    assert_eq!(interval_for_priority(0), SNAPSHOT_PUSH_INTERVAL);
    assert_eq!(interval_for_priority(1), VISIBLE_PUSH_INTERVAL);
    assert_eq!(interval_for_priority(2), HIDDEN_PUSH_INTERVAL);
    // Values above 2 also map to hidden.
    assert_eq!(interval_for_priority(255), HIDDEN_PUSH_INTERVAL);
}
