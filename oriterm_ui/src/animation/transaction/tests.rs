use std::time::Instant;

use super::{Transaction, current_transaction, with_transaction};
use crate::animation::behavior::AnimBehavior;
use crate::animation::property::AnimProperty;

#[test]
fn transaction_instant_overrides_behavior() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    with_transaction(Transaction::instant(), || {
        prop.set(1.0, now);
    });

    // Should be instant — no animation.
    assert_eq!(
        prop.get(now),
        1.0,
        "Instant transaction should skip animation"
    );
    assert!(
        !prop.is_animating(now),
        "Should not be animating after instant set"
    );
}

#[test]
fn transaction_animated_overrides_behavior() {
    let now = Instant::now();
    // Property has no behavior (instant by default).
    let mut prop = AnimProperty::new(0.0_f32);

    with_transaction(Transaction::animated(AnimBehavior::ease_out(200)), || {
        prop.set(1.0, now);
    });

    // Should be animating despite property having no behavior.
    assert!(
        prop.is_animating(now),
        "Animated transaction should start animation"
    );
    assert!(
        prop.get(now) < 0.5,
        "Should be near start immediately after set"
    );
}

#[test]
fn transaction_nesting_inner_overrides_outer() {
    let now = Instant::now();
    let mut prop = AnimProperty::new(0.0_f32);

    with_transaction(Transaction::animated(AnimBehavior::ease_out(200)), || {
        with_transaction(Transaction::instant(), || {
            prop.set(1.0, now);
        });
    });

    // Inner instant should win.
    assert_eq!(
        prop.get(now),
        1.0,
        "Inner instant transaction should override outer"
    );
}

#[test]
fn transaction_no_transaction_uses_property_behavior() {
    let now = Instant::now();
    let mut prop = AnimProperty::with_behavior(0.0_f32, AnimBehavior::ease_out(200));

    // No transaction active.
    prop.set(1.0, now);

    assert!(
        prop.is_animating(now),
        "Should animate using property behavior"
    );
}

#[test]
fn transaction_panic_restores_previous() {
    // Verify that a panic inside with_transaction restores the previous state.
    let result = std::panic::catch_unwind(|| {
        with_transaction(Transaction::instant(), || {
            panic!("test panic");
        });
    });

    assert!(result.is_err(), "Should have panicked");

    // After the panic, the transaction should be restored to None.
    let tx = current_transaction();
    assert!(tx.is_none(), "Transaction should be restored after panic");
}
