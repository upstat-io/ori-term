use std::time::Duration;

use super::{AnimBehavior, AnimCurve};
use crate::animation::Easing;
use crate::animation::spring::Spring;

#[test]
fn anim_behavior_ease_out_creates_easing_curve() {
    let b = AnimBehavior::ease_out(200);
    match b.curve {
        AnimCurve::Easing { easing, duration } => {
            assert_eq!(easing, Easing::EaseOut);
            assert_eq!(duration, Duration::from_millis(200));
        }
        AnimCurve::Spring(_) => panic!("Expected Easing curve"),
    }
}

#[test]
fn anim_behavior_spring_creates_spring_curve() {
    let b = AnimBehavior::spring();
    match b.curve {
        AnimCurve::Spring(s) => {
            assert_eq!(s, Spring::default());
        }
        AnimCurve::Easing { .. } => panic!("Expected Spring curve"),
    }
}

#[test]
fn anim_curve_easing_debug_format() {
    let c = AnimCurve::Easing {
        easing: Easing::Linear,
        duration: Duration::from_millis(100),
    };
    let debug = format!("{c:?}");
    assert!(
        debug.contains("Easing"),
        "Debug should contain 'Easing': {debug}"
    );
    assert!(
        debug.contains("Linear"),
        "Debug should contain 'Linear': {debug}"
    );
}
