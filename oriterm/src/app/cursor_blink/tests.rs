use std::time::Duration;

use super::{CursorBlink, DEFAULT_BLINK_INTERVAL};

#[test]
fn initial_state_is_visible() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    assert!(blink.is_visible());
}

#[test]
fn update_before_interval_reports_no_change() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    assert!(!blink.update());
    assert!(blink.is_visible());
}

#[test]
fn update_after_interval_reports_change() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    // Backdate the epoch so the interval has elapsed.
    blink.epoch -= Duration::from_millis(600);
    assert!(blink.update());
    assert!(!blink.is_visible());
}

#[test]
fn double_interval_restores_visibility() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);

    // Backdate by 2 intervals — phase 2 is visible again.
    blink.epoch -= Duration::from_millis(1100);
    assert!(blink.is_visible());
}

#[test]
fn reset_makes_visible() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);

    // Backdate into hidden phase.
    blink.epoch -= Duration::from_millis(600);
    assert!(!blink.is_visible());

    // Reset restores visibility.
    blink.reset();
    assert!(blink.is_visible());

    // And the timer is fresh — update reports no change.
    assert!(!blink.update());
}

#[test]
fn next_toggle_is_in_the_future() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let next = blink.next_toggle();
    assert!(next > std::time::Instant::now() - Duration::from_millis(10));
}

#[test]
fn custom_interval_respected() {
    let interval = Duration::from_millis(200);
    let mut blink = CursorBlink::new(interval);

    // 150ms is less than the 200ms interval — still visible.
    blink.epoch -= Duration::from_millis(150);
    assert!(blink.is_visible());
    assert!(!blink.update());

    // 250ms exceeds the 200ms interval — hidden.
    blink.epoch -= Duration::from_millis(100);
    assert!(!blink.is_visible());
    assert!(blink.update());
}

#[test]
fn set_interval_changes_timing() {
    let mut blink = CursorBlink::new(Duration::from_millis(1000));

    // 600ms < 1000ms — still visible.
    blink.epoch -= Duration::from_millis(600);
    assert!(blink.is_visible());

    // Shorten interval to 500ms — now 600ms puts us in phase 1 (hidden).
    blink.set_interval(Duration::from_millis(500));
    assert!(!blink.is_visible());
}

#[test]
fn skipped_update_does_not_lose_phases() {
    let mut blink = CursorBlink::new(Duration::from_millis(100));

    // Skip 3 intervals — should be in phase 3 (hidden).
    blink.epoch -= Duration::from_millis(350);
    assert!(!blink.is_visible());
    // Update detects the transition from the cached last_visible=true.
    assert!(blink.update());

    // Skip 2 more intervals (total 5) — phase 5 is hidden.
    blink.epoch -= Duration::from_millis(200);
    assert!(!blink.is_visible());
    // No change from last cached state (still hidden).
    assert!(!blink.update());
}
