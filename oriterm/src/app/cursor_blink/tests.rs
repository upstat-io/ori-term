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
fn next_toggle_is_approximately_one_interval_away() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let now = std::time::Instant::now();
    let next = blink.next_toggle();

    // next_toggle should be within one interval of now (freshly created blink
    // has epoch ≈ now, so next toggle ≈ now + interval).
    let delta = next.duration_since(now);
    assert!(
        delta <= DEFAULT_BLINK_INTERVAL + Duration::from_millis(50),
        "next_toggle too far in the future: {delta:?}",
    );
    assert!(
        delta >= Duration::from_millis(1),
        "next_toggle should be in the future, got delta: {delta:?}",
    );
}

#[test]
fn consecutive_toggles_spaced_by_one_interval() {
    let blink = CursorBlink::new(Duration::from_millis(500));

    // next_toggle = epoch + (floor(elapsed/interval) + 1) * interval.
    // At phase 0 (elapsed ≈ 0): next = epoch + 500ms.
    // At phase 1 (elapsed ≈ 500ms): next = epoch + 1000ms.
    // Gap between consecutive phase toggles is always exactly one interval.
    //
    // Verify by computing next_toggle at two different elapsed offsets
    // within consecutive phases. Since we can't control Instant::now(),
    // use the epoch-backdating trick and compare the absolute instants.
    let epoch = blink.epoch;

    // Phase 0: next_toggle = epoch + 500ms.
    let t0 = blink.next_toggle();
    assert_eq!(t0, epoch + Duration::from_millis(500));

    // Now simulate phase 1 by creating a new blink with epoch 600ms ago.
    let mut blink2 = CursorBlink::new(Duration::from_millis(500));
    blink2.epoch = epoch - Duration::from_millis(600);
    // elapsed ≈ 600ms → phase 1 → next_toggle = (epoch - 600) + 2*500 = epoch + 400ms.
    let t1 = blink2.next_toggle();
    assert_eq!(t1, epoch + Duration::from_millis(400));
    // From phase 1, next toggle is 400ms into the future (remainder of phase 1 + full phase 2 start).
    // The key property: phase toggles are on exact interval boundaries.
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
