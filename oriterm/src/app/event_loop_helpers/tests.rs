use std::time::{Duration, Instant};

use super::{ControlFlowDecision, ControlFlowInput, compute_control_flow};

/// Helper to build a default input (all false / idle).
fn idle_input() -> ControlFlowInput {
    let now = Instant::now();
    ControlFlowInput {
        still_dirty: false,
        budget_elapsed: false,
        has_animations: false,
        blinking_active: false,
        next_blink_change: now + Duration::from_secs(1),
        next_text_blink_change: now + Duration::from_secs(1),
        budget_remaining: Duration::from_millis(16),
        now,
        scheduler_wake: None,
    }
}

#[test]
fn idle_returns_text_blink_wait() {
    // Text blink timer always contributes — true idle is WaitUntil, not Wait.
    let input = idle_input();
    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(input.next_text_blink_change),
    );
}

#[test]
fn still_dirty_waits_for_budget() {
    // BUG-11-1 fix: dirty windows always respect frame budget to prevent
    // event loop starvation during sustained PTY output flooding.
    let mut input = idle_input();
    input.still_dirty = true;
    input.budget_elapsed = false;
    input.budget_remaining = Duration::from_millis(8);

    let result = compute_control_flow(&input);
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(8));
    assert_eq!(result, expected);
}

#[test]
fn still_dirty_budget_elapsed_wakes_immediately() {
    let mut input = idle_input();
    input.still_dirty = true;
    input.budget_elapsed = true;

    let result = compute_control_flow(&input);
    let expected = ControlFlowDecision::WaitUntil(input.now);
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
fn blinking_returns_next_blink_change() {
    let mut input = idle_input();
    input.blinking_active = true;
    let toggle = input.now + Duration::from_millis(530);
    input.next_blink_change = toggle;

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::WaitUntil(toggle));
}

#[test]
fn animations_active_uses_animation_cadence() {
    // Animations drive 16ms wakeup regardless of dirty state.
    let mut input = idle_input();
    input.has_animations = true;

    let result = compute_control_flow(&input);
    let expected = ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(16));
    assert_eq!(result, expected);
}

#[test]
fn animations_take_priority_over_blinking() {
    let mut input = idle_input();
    input.has_animations = true;
    input.blinking_active = true;
    input.next_blink_change = input.now + Duration::from_millis(530);

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
    input.next_blink_change = input.now + Duration::from_millis(530);
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
    input.next_blink_change = toggle;
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

// Budget gate tests — now applies to ALL present modes (BUG-11-1).

// Fade blink wakeup tests

#[test]
fn compute_control_flow_fade_blink_wakeup() {
    // During a fade transition, next_blink_change is ~16ms away.
    let mut input = idle_input();
    input.blinking_active = true;
    input.next_blink_change = input.now + Duration::from_millis(16);

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(16)),
    );
}

#[test]
fn compute_control_flow_plateau_blink_wakeup() {
    // During a plateau, next_blink_change is ~530ms away.
    let mut input = idle_input();
    input.blinking_active = true;
    input.next_blink_change = input.now + Duration::from_millis(530);

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(input.now + Duration::from_millis(530)),
    );
}
