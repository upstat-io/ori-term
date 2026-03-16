use std::time::{Duration, Instant};

use crate::animation::behavior::AnimBehavior;
use crate::color::Color;
use crate::interaction::InteractionState;

use super::{StateTransition, VisualStateAnimator, find_transition};
use crate::visual_state::{common_states, focus_states};

// -- Helpers --

const NORMAL_BG: Color = Color::hex(0x1E1E2E);
const HOVER_BG: Color = Color::hex(0x313244);
const PRESSED_BG: Color = Color::hex(0x45475A);
const DISABLED_BG: Color = Color::hex(0x585B70);

const UNFOCUSED_BORDER: Color = Color::hex(0x6C7086);
const FOCUSED_BORDER: Color = Color::hex(0x89B4FA);

fn make_animator() -> VisualStateAnimator {
    VisualStateAnimator::new(vec![common_states(
        NORMAL_BG,
        HOVER_BG,
        PRESSED_BG,
        DISABLED_BG,
    )])
}

fn hovered_state() -> InteractionState {
    let mut s = InteractionState::new();
    s.set_hot(true);
    s
}

fn pressed_state() -> InteractionState {
    let mut s = InteractionState::new();
    s.set_active(true);
    s
}

fn disabled_state() -> InteractionState {
    InteractionState::disabled()
}

fn focused_state() -> InteractionState {
    let mut s = InteractionState::new();
    s.set_focused(true);
    s
}

// -- Tests --

#[test]
fn normal_to_hovered_interpolates_bg_color_over_100ms() {
    let now = Instant::now();
    let mut animator = make_animator();

    // Transition to Hovered.
    animator.update(&hovered_state(), now);
    assert!(animator.is_animating(now));

    // At 50ms, should be partway between normal and hover.
    let mid = now + Duration::from_millis(50);
    let bg = animator.get_bg_color(mid);
    assert_ne!(bg, NORMAL_BG, "Should not be at start color at midpoint");
    assert_ne!(bg, HOVER_BG, "Should not be at end color at midpoint");

    // At 100ms, should be at hover.
    let end = now + Duration::from_millis(100);
    let bg = animator.get_bg_color(end);
    assert_eq!(bg, HOVER_BG);
}

#[test]
fn rapid_state_change_interrupts_and_restarts_from_current() {
    let now = Instant::now();
    let mut animator = make_animator();

    // Start Normal -> Hovered at t=0.
    animator.update(&hovered_state(), now);

    // At t=25ms, switch to Pressed (mid-hover).
    let t25 = now + Duration::from_millis(25);
    let mid_color = animator.get_bg_color(t25);
    assert_ne!(mid_color, NORMAL_BG);
    assert_ne!(mid_color, HOVER_BG);

    animator.update(&pressed_state(), t25);

    // The new transition should start from the mid-hover color.
    let just_after = t25;
    let bg = animator.get_bg_color(just_after);
    // At t=0 of the new transition, should be at the interrupted value.
    assert_ne!(bg, NORMAL_BG);
    assert_ne!(bg, PRESSED_BG);

    // After 100ms from interruption, should be at pressed color.
    let end = t25 + Duration::from_millis(100);
    let bg = animator.get_bg_color(end);
    assert_eq!(bg, PRESSED_BG);
}

#[test]
fn disabled_transition_with_instant_override() {
    let now = Instant::now();
    let mut animator = VisualStateAnimator::new(vec![common_states(
        NORMAL_BG,
        HOVER_BG,
        PRESSED_BG,
        DISABLED_BG,
    )])
    .with_transition(StateTransition {
        from: "*",
        to: "Disabled",
        behavior: AnimBehavior::ease_out(0),
    });

    // Transition to Disabled should be instant (0ms duration).
    animator.update(&disabled_state(), now);
    let bg = animator.get_bg_color(now);
    assert_eq!(bg, DISABLED_BG, "Disabled transition should be instant");
    assert!(!animator.is_animating(now));
}

#[test]
fn focus_states_resolves_correctly() {
    let now = Instant::now();
    let mut animator =
        VisualStateAnimator::new(vec![focus_states(UNFOCUSED_BORDER, FOCUSED_BORDER)]);

    // Initially unfocused — border should be unfocused color.
    let border = animator.get_border_color(now);
    assert_eq!(border, UNFOCUSED_BORDER);

    // Transition to focused.
    animator.update(&focused_state(), now);

    // After 100ms (default transition), should be at focused border.
    let end = now + Duration::from_millis(100);
    let border = animator.get_border_color(end);
    assert_eq!(border, FOCUSED_BORDER);
}

#[test]
fn two_groups_compose_independently() {
    let now = Instant::now();
    let mut animator = VisualStateAnimator::new(vec![
        common_states(NORMAL_BG, HOVER_BG, PRESSED_BG, DISABLED_BG),
        focus_states(UNFOCUSED_BORDER, FOCUSED_BORDER),
    ]);

    // Both hovered and focused.
    let mut state = InteractionState::new();
    state.set_hot(true);
    state.set_focused(true);

    animator.update(&state, now);

    // After 100ms both should have transitioned.
    let end = now + Duration::from_millis(100);
    assert_eq!(animator.get_bg_color(end), HOVER_BG);
    assert_eq!(animator.get_border_color(end), FOCUSED_BORDER);
}

#[test]
fn newly_created_animator_returns_initial_values_without_update() {
    let now = Instant::now();
    let animator = make_animator();

    // Should return Normal state values immediately.
    let bg = animator.get_bg_color(now);
    assert_eq!(bg, NORMAL_BG, "Initial bg should be Normal state color");
    assert!(!animator.is_animating(now));
}

#[test]
fn find_transition_returns_exact_match() {
    let transitions = vec![
        StateTransition {
            from: "Normal",
            to: "Hovered",
            behavior: AnimBehavior::linear(200),
        },
        StateTransition {
            from: "*",
            to: "Hovered",
            behavior: AnimBehavior::ease_out(50),
        },
    ];
    let default = AnimBehavior::ease_out(100);

    let result = find_transition(&transitions, &default, "Normal", "Hovered");
    // Should match the exact (Normal, Hovered) entry, not the wildcard.
    assert!(matches!(
        result.curve,
        crate::animation::behavior::AnimCurve::Easing { duration, .. }
        if duration == Duration::from_millis(200)
    ));
}

#[test]
fn find_transition_falls_back_to_wildcard_then_default() {
    let transitions = vec![StateTransition {
        from: "*",
        to: "Disabled",
        behavior: AnimBehavior::ease_out(0),
    }];
    let default = AnimBehavior::ease_out(100);

    // Should match wildcard.
    let result = find_transition(&transitions, &default, "Normal", "Disabled");
    assert!(matches!(
        result.curve,
        crate::animation::behavior::AnimCurve::Easing { duration, .. }
        if duration == Duration::from_millis(0)
    ));

    // No matching transition — should fall back to default.
    let result = find_transition(&transitions, &default, "Normal", "Hovered");
    assert!(matches!(
        result.curve,
        crate::animation::behavior::AnimCurve::Easing { duration, .. }
        if duration == Duration::from_millis(100)
    ));
}

#[test]
fn spring_based_transition_converges() {
    let now = Instant::now();
    let mut animator = make_animator().with_default_transition(AnimBehavior::spring());

    animator.update(&hovered_state(), now);

    // Tick at 60fps for 5 seconds.
    let mut t = now;
    for _ in 0..300 {
        t += Duration::from_millis(16);
        animator.tick(t);
    }

    let bg = animator.get_bg_color(t);
    // Check each channel is close to hover bg.
    assert!(
        (bg.r - HOVER_BG.r).abs() < 0.01
            && (bg.g - HOVER_BG.g).abs() < 0.01
            && (bg.b - HOVER_BG.b).abs() < 0.01,
        "Spring should converge to target, got {bg:?} expected {HOVER_BG:?}"
    );
}

#[test]
fn get_fg_color_returns_transparent_when_unset() {
    let now = Instant::now();
    let animator = make_animator();

    // CommonStates only sets BgColor, not FgColor.
    assert_eq!(animator.get_fg_color(now), Color::TRANSPARENT);
}

#[test]
fn no_state_change_does_not_start_animation() {
    let now = Instant::now();
    let mut animator = make_animator();

    // Normal state is already active — update should be a no-op.
    animator.update(&InteractionState::new(), now);
    assert!(!animator.is_animating(now));
}

#[test]
fn with_default_transition_overrides_default() {
    let now = Instant::now();
    let mut animator = make_animator().with_default_transition(AnimBehavior::linear(500));

    animator.update(&hovered_state(), now);

    // At 250ms (midpoint of 500ms linear), should be partway.
    let mid = now + Duration::from_millis(250);
    let bg = animator.get_bg_color(mid);
    assert_ne!(bg, NORMAL_BG);
    assert_ne!(bg, HOVER_BG);

    // At 500ms, should be at target.
    let end = now + Duration::from_millis(500);
    assert_eq!(animator.get_bg_color(end), HOVER_BG);
}
