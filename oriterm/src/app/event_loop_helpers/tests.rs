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

/// A kitty-graphics animation deadline reaches `compute_control_flow` via
/// `ControlFlowInput.scheduler_wake` and produces `WaitUntil(deadline)`
/// when it is sooner than the text-blink wake. Pins the end-to-end wiring
/// invariant: `IO thread → MuxEvent::AnimationDeadlineChanged →
/// RenderScheduler::request_frame_at → scheduler.next_wake_time() →
/// ControlFlowInput.scheduler_wake → ControlFlow::WaitUntil`.
/// Regression: §13.3 TPR checkpoint anchor.
#[test]
fn scheduler_wake_populates_wait_until_when_sooner_than_text_blink() {
    let mut input = idle_input();
    let animation_deadline = input.now + Duration::from_millis(50);
    input.scheduler_wake = Some(animation_deadline);
    // Text blink is 1s out per idle_input() — animation deadline is sooner.

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(animation_deadline),
        "animation deadline 50ms < text blink 1s, so WaitUntil MUST honor \
         the scheduler_wake value"
    );
}

/// Text blink wins when it is sooner than the animation deadline — the
/// `min` discipline in `compute_control_flow` must not drop the blink
/// just because an animation is queued.
#[test]
fn text_blink_wins_when_sooner_than_scheduler_wake() {
    let mut input = idle_input();
    let text_blink = input.now + Duration::from_millis(20);
    input.next_text_blink_change = text_blink;
    input.scheduler_wake = Some(input.now + Duration::from_millis(500));

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(text_blink),
        "text blink 20ms < scheduler_wake 500ms — min MUST pick blink"
    );
}

/// When `still_dirty && !budget_elapsed` coincides with a queued
/// `scheduler_wake` (e.g. a kitty animation deadline that fires inside
/// the 16ms render-budget window), the decision MUST promote to
/// `WaitUntil(scheduler_wake)` instead of the naked `Wait` — otherwise
/// the deadline is masked by the render gate and the animation stalls
/// until the next unrelated MuxWakeup arrives.
/// Regression: §13.3 review round 1 F1.
#[test]
fn still_dirty_budget_not_elapsed_honors_scheduler_wake_when_sooner() {
    let mut input = idle_input();
    input.still_dirty = true;
    input.budget_elapsed = false;
    let animation_deadline = input.now + Duration::from_millis(10);
    input.scheduler_wake = Some(animation_deadline);

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(animation_deadline),
        "queued animation deadline inside the render budget MUST escape \
         the still_dirty Wait gate — otherwise the frame stalls until an \
         unrelated MuxWakeup"
    );
}

/// Past-due scheduler_wake values are NOT promoted to WaitUntil in the
/// still_dirty branch — those would trigger the Windows/WSL2 "WaitUntil
/// returns immediately" failure mode that the `Wait` gate is designed to
/// avoid. Past-due entries should already be drained by `promote_deferred`
/// before reaching `compute_control_flow`; this guard is belt-and-suspenders.
#[test]
fn still_dirty_budget_not_elapsed_ignores_past_due_scheduler_wake() {
    let mut input = idle_input();
    input.still_dirty = true;
    input.budget_elapsed = false;
    input.scheduler_wake = Some(input.now - Duration::from_millis(1));

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::Wait,
        "past-due scheduler_wake MUST NOT promote to WaitUntil — would \
         trigger the Windows/WSL2 immediate-return failure mode"
    );
}

/// Past-due `scheduler_wake` in the idle branch (!still_dirty && !has_animations)
/// MUST fall through to the text-blink wake rather than producing
/// `WaitUntil(past)` which fires immediately and spins the event loop. The
/// canonical drain site is `about_to_wait`'s `promote_deferred` sweep; this
/// guard inside `compute_control_flow` is belt-and-suspenders.
/// Regression: §13.3 review round 2 F1 / F3.
#[test]
fn idle_branch_ignores_past_due_scheduler_wake() {
    let mut input = idle_input();
    input.scheduler_wake = Some(input.now - Duration::from_millis(1));

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(input.next_text_blink_change),
        "past-due scheduler_wake in idle branch MUST fall through to \
         text-blink wake — otherwise WaitUntil(past) spins the event loop"
    );
}

/// `scheduler_wake == input.now` (exactly the moment) is NOT in the future
/// — treat as past-due per the same guard. Pin the `>` boundary (not `>=`).
#[test]
fn idle_branch_ignores_scheduler_wake_equal_to_now() {
    let mut input = idle_input();
    input.scheduler_wake = Some(input.now);

    let result = compute_control_flow(&input);
    assert_eq!(
        result,
        ControlFlowDecision::WaitUntil(input.next_text_blink_change),
        "scheduler_wake == now is NOT strictly in the future; MUST fall \
         through to text-blink wake"
    );
}
