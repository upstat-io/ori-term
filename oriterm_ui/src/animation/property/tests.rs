use super::AnimProperty;
use crate::animation::behavior::AnimBehavior;
use crate::animation::spring::Spring;

#[test]
fn set_behavior_enables_animation_on_previously_instant_property() {
    let mut prop = AnimProperty::new(0.0_f32);

    // No behavior — set is instant.
    prop.set(1.0);
    assert_eq!(prop.get(), 1.0);
    assert!(!prop.is_animating());

    // Attach a behavior, then set a new target.
    // 200ms at 60fps = 12 frames.
    prop.set_behavior(Some(AnimBehavior::linear(200)));
    prop.set(0.0);
    assert!(prop.is_animating(), "Should animate after set_behavior");

    // At 6 frames (midpoint of 12), linear should give ~0.5.
    for _ in 0..6 {
        prop.tick();
    }
    let val = prop.get();
    assert!(
        (val - 0.5).abs() < 0.05,
        "Expected ~0.5 at midpoint, got {val}"
    );
}

#[test]
fn anim_property_new_has_no_behavior() {
    let prop = AnimProperty::new(0.5_f32);
    assert_eq!(prop.get(), 0.5);
    assert!(!prop.is_animating());
}

#[test]
fn anim_property_with_behavior_animates_on_set() {
    // 200ms at 60fps = 12 frames.
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0);
    assert!(prop.is_animating());

    // At the midpoint (6 frames), the value should be between 0 and 1 (eased).
    for _ in 0..6 {
        prop.tick();
    }
    let val = prop.get();
    assert!(
        val > 0.0 && val < 1.0,
        "Mid-animation value should be between 0 and 1, got {val}"
    );
}

#[test]
fn anim_property_new_set_is_instant() {
    let mut prop = AnimProperty::new(0.0_f32);

    prop.set(1.0);
    assert_eq!(prop.get(), 1.0, "No behavior means instant set");
    assert!(!prop.is_animating());
}

#[test]
fn anim_property_set_immediate_bypasses_behavior() {
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set_immediate(1.0);
    assert_eq!(prop.get(), 1.0, "set_immediate should bypass animation");
    assert!(!prop.is_animating());
}

#[test]
fn anim_property_get_returns_interpolated_value() {
    // 100ms at 60fps = 6 frames.
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::linear(100));

    prop.set(1.0);

    // At 3 frames (50%), linear easing should give ~0.5.
    for _ in 0..3 {
        prop.tick();
    }
    let val = prop.get();
    assert!(
        (val - 0.5).abs() < 0.05,
        "Expected ~0.5 at 50% of linear animation, got {val}"
    );

    // At 6 frames (100%), should be at target.
    for _ in 0..3 {
        prop.tick();
    }
    let val = prop.get();
    assert_eq!(val, 1.0, "Should be at target after duration");
}

#[test]
fn anim_property_is_animating_during_transition() {
    // 200ms at 60fps = 12 frames.
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0);
    assert!(prop.is_animating());
    for _ in 0..6 {
        prop.tick();
    }
    assert!(prop.is_animating());
}

#[test]
fn anim_property_not_animating_after_completion() {
    // 200ms at 60fps = 12 frames.
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0);
    for _ in 0..12 {
        prop.tick();
    }
    assert!(!prop.is_animating());
}

#[test]
fn anim_property_smooth_interruption() {
    // 200ms at 60fps = 12 frames.
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::linear(200));

    prop.set(1.0);

    // At 6 frames (halfway through linear), interrupt with a new target.
    for _ in 0..6 {
        prop.tick();
    }
    let mid_val = prop.get();
    assert!(
        (mid_val - 0.5).abs() < 0.05,
        "Should be ~0.5 at interruption point, got {mid_val}"
    );

    prop.set(0.0);

    // The new animation should start from the interrupted position (~0.5).
    let val = prop.get();
    assert!(
        (val - 0.5).abs() < 0.05,
        "Should start new animation from interrupted position, got {val}"
    );

    // After the new animation completes (12 more frames), should be at 0.0.
    for _ in 0..12 {
        prop.tick();
    }
    assert_eq!(prop.get(), 0.0, "Should reach new target");
}

#[test]
fn anim_property_target_returns_final_value() {
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0);
    assert_eq!(prop.target(), 1.0, "target() should return the set value");

    // Even during animation, target is the final value.
    assert_eq!(prop.target(), 1.0);
}

#[test]
fn anim_property_tick_advances_spring() {
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::spring());

    prop.set(1.0);
    assert!(prop.is_animating());

    // Tick repeatedly and verify convergence.
    for _ in 0..300 {
        prop.tick();
    }

    let val = prop.get();
    assert!(
        (val - 1.0).abs() < 0.01,
        "Spring should converge to target after ~5 seconds, got {val}"
    );
}

#[test]
fn anim_property_tick_noop_for_easing() {
    // 200ms at 60fps = 12 frames.
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::linear(200));

    prop.set(1.0);

    // Advance 3 frames.
    for _ in 0..3 {
        prop.tick();
    }

    // tick() for easing simply increments the frame counter; get() is deterministic.
    let before = prop.get();
    // No tick here — just read again.
    let after = prop.get();
    assert_eq!(
        before, after,
        "get() should return consistent value without additional tick()"
    );
}

#[test]
fn anim_property_spring_with_custom_params() {
    let spring = Spring {
        response: 0.3,
        damping: 1.0, // Critically damped.
        epsilon: 0.001,
    };
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::spring_with(spring));

    prop.set(1.0);

    for _ in 0..300 {
        prop.tick();
    }

    let val = prop.get();
    assert!(
        (val - 1.0).abs() < 0.01,
        "Critically damped spring should converge, got {val}"
    );
}
