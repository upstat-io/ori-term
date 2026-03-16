use std::time::{Duration, Instant};

use super::AnimProperty;
use crate::animation::behavior::AnimBehavior;
use crate::animation::spring::Spring;

#[test]
fn set_behavior_enables_animation_on_previously_instant_property() {
    let now = Instant::now();
    let mut prop = AnimProperty::new(0.0_f32);

    // No behavior — set is instant.
    prop.set(1.0, now);
    assert_eq!(prop.get(now), 1.0);
    assert!(!prop.is_animating(now));

    // Attach a behavior, then set a new target.
    prop.set_behavior(Some(AnimBehavior::linear(200)));
    prop.set(0.0, now);
    assert!(prop.is_animating(now), "Should animate after set_behavior");

    // At 100ms, linear should give ~0.5.
    let mid = now + Duration::from_millis(100);
    let val = prop.get(mid);
    assert!(
        (val - 0.5).abs() < 0.05,
        "Expected ~0.5 at midpoint, got {val}"
    );
}

#[test]
fn anim_property_new_has_no_behavior() {
    let prop = AnimProperty::new(0.5_f32);
    let now = Instant::now();
    assert_eq!(prop.get(now), 0.5);
    assert!(!prop.is_animating(now));
}

#[test]
fn anim_property_with_behavior_animates_on_set() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0, now);
    assert!(prop.is_animating(now));

    // At the midpoint, the value should be between 0 and 1 (eased).
    let mid = now + Duration::from_millis(100);
    let val = prop.get(mid);
    assert!(
        val > 0.0 && val < 1.0,
        "Mid-animation value should be between 0 and 1, got {val}"
    );
}

#[test]
fn anim_property_new_set_is_instant() {
    let now = Instant::now();
    let mut prop = AnimProperty::new(0.0_f32);

    prop.set(1.0, now);
    assert_eq!(prop.get(now), 1.0, "No behavior means instant set");
    assert!(!prop.is_animating(now));
}

#[test]
fn anim_property_set_immediate_bypasses_behavior() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set_immediate(1.0);
    assert_eq!(prop.get(now), 1.0, "set_immediate should bypass animation");
    assert!(!prop.is_animating(now));
}

#[test]
fn anim_property_get_returns_interpolated_value() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::linear(100));

    prop.set(1.0, now);

    // At 50ms, linear easing should give ~0.5.
    let val = prop.get(now + Duration::from_millis(50));
    assert!(
        (val - 0.5).abs() < 0.05,
        "Expected ~0.5 at 50% of linear animation, got {val}"
    );

    // At 100ms, should be at target.
    let val = prop.get(now + Duration::from_millis(100));
    assert_eq!(val, 1.0, "Should be at target after duration");
}

#[test]
fn anim_property_is_animating_during_transition() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0, now);
    assert!(prop.is_animating(now));
    assert!(prop.is_animating(now + Duration::from_millis(100)));
}

#[test]
fn anim_property_not_animating_after_completion() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0, now);
    assert!(!prop.is_animating(now + Duration::from_millis(200)));
}

#[test]
fn anim_property_smooth_interruption() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::linear(200));

    prop.set(1.0, now);

    // At 100ms (halfway through linear), interrupt with a new target.
    let mid = now + Duration::from_millis(100);
    let mid_val = prop.get(mid);
    assert!(
        (mid_val - 0.5).abs() < 0.05,
        "Should be ~0.5 at interruption point, got {mid_val}"
    );

    prop.set(0.0, mid);

    // The new animation should start from the interrupted position (~0.5).
    let val = prop.get(mid);
    assert!(
        (val - 0.5).abs() < 0.05,
        "Should start new animation from interrupted position, got {val}"
    );

    // After the new animation completes, should be at 0.0.
    let end = mid + Duration::from_millis(200);
    assert_eq!(prop.get(end), 0.0, "Should reach new target");
}

#[test]
fn anim_property_target_returns_final_value() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    prop.set(1.0, now);
    assert_eq!(prop.target(), 1.0, "target() should return the set value");

    // Even during animation, target is the final value.
    assert_eq!(prop.target(), 1.0);
}

#[test]
fn anim_property_tick_advances_spring() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::spring());

    prop.set(1.0, now);
    assert!(prop.is_animating(now));

    // Tick repeatedly at 60fps and verify convergence.
    let mut t = now;
    for _ in 0..300 {
        t += Duration::from_millis(16);
        prop.tick(t);
    }

    let val = prop.get(t);
    assert!(
        (val - 1.0).abs() < 0.01,
        "Spring should converge to target after ~5 seconds, got {val}"
    );
}

#[test]
fn anim_property_tick_noop_for_easing() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::linear(200));

    prop.set(1.0, now);

    // tick() should be a no-op for easing — the value is computed lazily.
    let before = prop.get(now + Duration::from_millis(50));
    prop.tick(now + Duration::from_millis(50));
    let after = prop.get(now + Duration::from_millis(50));
    assert_eq!(
        before, after,
        "tick() should not affect easing-based properties"
    );
}

#[test]
fn anim_property_spring_with_custom_params() {
    let now = Instant::now();
    let spring = Spring {
        response: 0.3,
        damping: 1.0, // Critically damped.
        epsilon: 0.001,
    };
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::spring_with(spring));

    prop.set(1.0, now);

    let mut t = now;
    for _ in 0..300 {
        t += Duration::from_millis(16);
        prop.tick(t);
    }

    let val = prop.get(t);
    assert!(
        (val - 1.0).abs() < 0.01,
        "Critically damped spring should converge, got {val}"
    );
}
