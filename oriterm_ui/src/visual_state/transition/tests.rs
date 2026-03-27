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
fn normal_to_hovered_interpolates_bg_color() {
    let mut animator = make_animator();

    // Transition to Hovered.
    // Default transition is 100ms = 6 frames.
    animator.update(&hovered_state());
    assert!(animator.is_animating());

    // At 3 frames (midpoint), should be partway between normal and hover.
    for _ in 0..3 {
        animator.tick();
    }
    let bg = animator.get_bg_color();
    assert_ne!(bg, NORMAL_BG, "Should not be at start color at midpoint");
    assert_ne!(bg, HOVER_BG, "Should not be at end color at midpoint");

    // At 6 frames (complete), should be at hover.
    for _ in 0..3 {
        animator.tick();
    }
    let bg = animator.get_bg_color();
    assert_eq!(bg, HOVER_BG);
}

#[test]
fn rapid_state_change_interrupts_and_restarts_from_current() {
    let mut animator = make_animator();

    // Start Normal -> Hovered.
    animator.update(&hovered_state());

    // At 2 frames, switch to Pressed (mid-hover).
    for _ in 0..2 {
        animator.tick();
    }
    let mid_color = animator.get_bg_color();
    assert_ne!(mid_color, NORMAL_BG);
    assert_ne!(mid_color, HOVER_BG);

    animator.update(&pressed_state());

    // The new transition should start from the mid-hover color.
    let bg = animator.get_bg_color();
    // At frame 0 of the new transition, should be at the interrupted value.
    assert_ne!(bg, NORMAL_BG);
    assert_ne!(bg, PRESSED_BG);

    // After 6 frames from interruption, should be at pressed color.
    for _ in 0..6 {
        animator.tick();
    }
    let bg = animator.get_bg_color();
    assert_eq!(bg, PRESSED_BG);
}

#[test]
fn disabled_transition_with_instant_override() {
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

    // Transition to Disabled should be instant (0 frames).
    animator.update(&disabled_state());
    let bg = animator.get_bg_color();
    assert_eq!(bg, DISABLED_BG, "Disabled transition should be instant");
    assert!(!animator.is_animating());
}

#[test]
fn focus_states_resolves_correctly() {
    let mut animator =
        VisualStateAnimator::new(vec![focus_states(UNFOCUSED_BORDER, FOCUSED_BORDER)]);

    // Initially unfocused — border should be unfocused color.
    let border = animator.get_border_color();
    assert_eq!(border, UNFOCUSED_BORDER);

    // Transition to focused.
    animator.update(&focused_state());

    // After 6 frames (default 100ms transition), should be at focused border.
    for _ in 0..6 {
        animator.tick();
    }
    let border = animator.get_border_color();
    assert_eq!(border, FOCUSED_BORDER);
}

#[test]
fn two_groups_compose_independently() {
    let mut animator = VisualStateAnimator::new(vec![
        common_states(NORMAL_BG, HOVER_BG, PRESSED_BG, DISABLED_BG),
        focus_states(UNFOCUSED_BORDER, FOCUSED_BORDER),
    ]);

    // Both hovered and focused.
    let mut state = InteractionState::new();
    state.set_hot(true);
    state.set_focused(true);

    animator.update(&state);

    // After 6 frames both should have transitioned.
    for _ in 0..6 {
        animator.tick();
    }
    assert_eq!(animator.get_bg_color(), HOVER_BG);
    assert_eq!(animator.get_border_color(), FOCUSED_BORDER);
}

#[test]
fn newly_created_animator_returns_initial_values_without_update() {
    let animator = make_animator();

    // Should return Normal state values immediately.
    let bg = animator.get_bg_color();
    assert_eq!(bg, NORMAL_BG, "Initial bg should be Normal state color");
    assert!(!animator.is_animating());
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
    // 200ms at 60fps = 12 frames.
    assert!(matches!(
        result.curve,
        crate::animation::behavior::AnimCurve::Easing { total_frames, .. }
        if total_frames == 12
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

    // Should match wildcard. 0ms = 0 frames.
    let result = find_transition(&transitions, &default, "Normal", "Disabled");
    assert!(matches!(
        result.curve,
        crate::animation::behavior::AnimCurve::Easing { total_frames, .. }
        if total_frames == 0
    ));

    // No matching transition — should fall back to default. 100ms = 6 frames.
    let result = find_transition(&transitions, &default, "Normal", "Hovered");
    assert!(matches!(
        result.curve,
        crate::animation::behavior::AnimCurve::Easing { total_frames, .. }
        if total_frames == 6
    ));
}

#[test]
fn spring_based_transition_converges() {
    let mut animator = make_animator().with_default_transition(AnimBehavior::spring());

    animator.update(&hovered_state());

    // Tick 300 frames (about 5 seconds at 60fps).
    for _ in 0..300 {
        animator.tick();
    }

    let bg = animator.get_bg_color();
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
    let animator = make_animator();

    // CommonStates only sets BgColor, not FgColor.
    assert_eq!(animator.get_fg_color(), Color::TRANSPARENT);
}

#[test]
fn no_state_change_does_not_start_animation() {
    let mut animator = make_animator();

    // Normal state is already active — update should be a no-op.
    animator.update(&InteractionState::new());
    assert!(!animator.is_animating());
}

#[test]
fn with_default_transition_overrides_default() {
    // 500ms at 60fps = 30 frames.
    let mut animator = make_animator().with_default_transition(AnimBehavior::linear(500));

    animator.update(&hovered_state());

    // At 15 frames (midpoint of 30 linear), should be partway.
    for _ in 0..15 {
        animator.tick();
    }
    let bg = animator.get_bg_color();
    assert_ne!(bg, NORMAL_BG);
    assert_ne!(bg, HOVER_BG);

    // At 30 frames, should be at target.
    for _ in 0..15 {
        animator.tick();
    }
    assert_eq!(animator.get_bg_color(), HOVER_BG);
}
