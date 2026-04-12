use std::time::{Duration, Instant};

use super::{ControlFlowDecision, ControlFlowInput, compute_control_flow};

/// Helper to build a default input (all false / idle, GPU healthy).
fn idle_input() -> ControlFlowInput {
    let now = Instant::now();
    ControlFlowInput {
        still_dirty: false,
        budget_elapsed: false,
        has_animations: false,
        blinking_active: false,
        gpu_healthy: true,
        next_blink_change: now + Duration::from_secs(1),
        next_text_blink_change: now + Duration::from_secs(1),
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
fn still_dirty_budget_not_elapsed_waits() {
    // fix: when dirty but budget hasn't elapsed, use Wait (not
    // WaitUntil) so winit truly sleeps and processes keyboard events on
    // the next MuxWakeup. WaitUntil is broken on Windows/WSL2.
    let mut input = idle_input();
    input.still_dirty = true;
    input.budget_elapsed = false;

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::Wait);
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

// Budget gate tests — now applies to ALL present modes.

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

// 5.16.2 wakeup gate — gpu_healthy = false silences every periodic source.

#[test]
fn control_flow_during_recovering_returns_wait() {
    // Performance Invariant 1: zero idle CPU during recovery.
    // When the GPU is not healthy, every wakeup source is silenced and
    // compute_control_flow returns Wait — the event loop sleeps until the
    // next external event arrives. This is the I3 (bounded cost) pin
    // referenced from 5.16.13's "Idle CPU during Recovering" pin.
    let mut input = idle_input();
    input.gpu_healthy = false;

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::Wait);
}

#[test]
fn control_flow_recovering_overrides_blink_wakeup() {
    // Even with cursor blink and text blink active, the recovery gate
    // takes precedence so the event loop does not spin against a dead
    // device.
    let mut input = idle_input();
    input.gpu_healthy = false;
    input.blinking_active = true;
    input.next_blink_change = input.now + Duration::from_millis(16);
    input.next_text_blink_change = input.now + Duration::from_millis(16);

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::Wait);
}

#[test]
fn control_flow_recovering_overrides_animations() {
    // Animations also yield to the recovery gate.
    let mut input = idle_input();
    input.gpu_healthy = false;
    input.has_animations = true;

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::Wait);
}

#[test]
fn control_flow_recovering_overrides_dirty() {
    // Even when windows are dirty, recovery means we cannot render —
    // sleep until recovery completes.
    let mut input = idle_input();
    input.gpu_healthy = false;
    input.still_dirty = true;
    input.budget_elapsed = true;

    let result = compute_control_flow(&input);
    assert_eq!(result, ControlFlowDecision::Wait);
}

// Section 06.5 Track B kxIN/kxOUT cross-crate xcheck tests
// live in the `focus_events` sibling submodule — see
// `oriterm/src/app/event_loop_helpers/focus_events/tests.rs`.
