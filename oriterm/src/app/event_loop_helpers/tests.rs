use std::time::{Duration, Instant};

use super::{ControlFlowDecision, ControlFlowInput, compute_control_flow};

/// Helper to build a default input (all false / idle).
fn idle_input() -> ControlFlowInput {
    let now = Instant::now();
    ControlFlowInput {
        any_dirty: false,
        budget_elapsed: false,
        urgent_redraw: false,
        still_dirty: false,
        has_animations: false,
        blinking_active: false,
        next_toggle: now + Duration::from_secs(1),
        budget_remaining: Duration::from_millis(16),
        now,
        scheduler_wake: None,
    }
}

#[test]
fn idle_returns_wait() {
    let input = idle_input();
    assert_eq!(compute_control_flow(&input), ControlFlowDecision::Wait);
}

#[test]
fn dirty_before_budget_returns_wait_until_remaining() {
    let mut input = idle_input();
    input.any_dirty = true;
    input.budget_elapsed = false;
    input.budget_remaining = Duration::from_millis(10);

    let result = compute_control_flow(&input);
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(10));
    assert_eq!(result, expected);
}

#[test]
fn still_dirty_after_render_returns_wait_until() {
    let mut input = idle_input();
    input.still_dirty = true;
    input.budget_remaining = Duration::from_millis(5);

    let result = compute_control_flow(&input);
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(5));
    assert_eq!(result, expected);
}

#[test]
fn animations_return_16ms_wait() {
    let mut input = idle_input();
    input.has_animations = true;

    let result = compute_control_flow(&input);
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(16));
    assert_eq!(result, expected);
}

#[test]
fn blinking_returns_next_toggle() {
    let mut input = idle_input();
    input.blinking_active = true;
    let toggle = input.now + Duration::from_millis(530);
    input.next_toggle = toggle;

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::WaitUntil(toggle));
}

#[test]
fn dirty_takes_priority_over_animations() {
    let mut input = idle_input();
    input.any_dirty = true;
    input.budget_elapsed = false;
    input.has_animations = true;
    input.budget_remaining = Duration::from_millis(8);

    let result = compute_control_flow(&input);
    // Dirty+budget-not-elapsed takes priority over animations.
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(8));
    assert_eq!(result, expected);
}

#[test]
fn urgent_dirty_bypasses_budget_wait() {
    let mut input = idle_input();
    input.any_dirty = true;
    input.budget_elapsed = false;
    input.urgent_redraw = true;

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::Wait);
}

#[test]
fn animations_take_priority_over_blinking() {
    let mut input = idle_input();
    input.has_animations = true;
    input.blinking_active = true;
    input.next_toggle = input.now + Duration::from_millis(530);

    let result = compute_control_flow(&input);
    // Animations (16ms) take priority over blink (530ms).
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(16));
    assert_eq!(result, expected);
}

// Scheduler wake tests

#[test]
fn scheduler_wake_returns_wait_until_when_idle() {
    let mut input = idle_input();
    let wake = input.now + Duration::from_millis(200);
    input.scheduler_wake = Some(wake);

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::WaitUntil(wake));
}

#[test]
fn scheduler_wake_picks_earlier_of_blink_and_wake() {
    let mut input = idle_input();
    input.blinking_active = true;
    input.next_toggle = input.now + Duration::from_millis(530);
    // Scheduler wake is earlier than blink toggle.
    input.scheduler_wake = Some(input.now + Duration::from_millis(100));

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(100))
    );
}

#[test]
fn scheduler_wake_blink_wins_when_earlier() {
    let mut input = idle_input();
    input.blinking_active = true;
    let toggle = input.now + Duration::from_millis(100);
    input.next_toggle = toggle;
    // Scheduler wake is later than blink toggle.
    input.scheduler_wake = Some(input.now + Duration::from_millis(500));

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::WaitUntil(toggle));
}

#[test]
fn animations_take_priority_over_scheduler_wake() {
    let mut input = idle_input();
    input.has_animations = true;
    input.scheduler_wake = Some(input.now + Duration::from_millis(200));

    let result = compute_control_flow(&input);
    // Animations (16ms) take priority.
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(16));
    assert_eq!(result, expected);
}
