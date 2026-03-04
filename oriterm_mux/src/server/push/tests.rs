use std::time::{Duration, Instant};

use super::{SNAPSHOT_PUSH_INTERVAL, should_push};

#[test]
fn throttled_within_interval() {
    let now = Instant::now();
    let last = now - Duration::from_millis(5);
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
