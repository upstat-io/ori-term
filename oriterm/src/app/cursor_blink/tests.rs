use std::time::Duration;

use super::CursorBlink;

#[test]
fn initial_state_is_visible() {
    let blink = CursorBlink::new();
    assert!(blink.is_visible());
}

#[test]
fn update_before_interval_is_noop() {
    let mut blink = CursorBlink::new();
    assert!(!blink.update());
    assert!(blink.is_visible());
}

#[test]
fn update_after_interval_toggles() {
    let mut blink = CursorBlink::new();
    // Backdate the phase start so the interval has elapsed.
    blink.phase_start -= Duration::from_millis(600);
    assert!(blink.update());
    assert!(!blink.is_visible());
}

#[test]
fn double_toggle_restores_visibility() {
    let mut blink = CursorBlink::new();

    // First toggle: visible → hidden.
    blink.phase_start -= Duration::from_millis(600);
    blink.update();
    assert!(!blink.is_visible());

    // Second toggle: hidden → visible.
    blink.phase_start -= Duration::from_millis(600);
    blink.update();
    assert!(blink.is_visible());
}

#[test]
fn reset_makes_visible() {
    let mut blink = CursorBlink::new();

    // Toggle to hidden.
    blink.phase_start -= Duration::from_millis(600);
    blink.update();
    assert!(!blink.is_visible());

    // Reset restores visibility.
    blink.reset();
    assert!(blink.is_visible());

    // And the timer is fresh — update is a no-op.
    assert!(!blink.update());
}

#[test]
fn next_toggle_is_in_the_future() {
    let blink = CursorBlink::new();
    let next = blink.next_toggle();
    assert!(next > std::time::Instant::now() - Duration::from_millis(10));
}
