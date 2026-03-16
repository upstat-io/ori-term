use super::Spring;

/// Run the spring until it converges or hits a step limit.
fn run_spring(spring: &Spring, start: f32, target: f32, dt: f32) -> Vec<f32> {
    let mut current = start;
    let mut velocity = 0.0;
    let mut values = vec![current];

    for _ in 0..10_000 {
        let (new_current, new_velocity, done) = spring.step(current, target, velocity, dt);
        current = new_current;
        velocity = new_velocity;
        values.push(current);
        if done {
            break;
        }
    }

    values
}

#[test]
fn spring_converges_to_target() {
    let spring = Spring::default();
    let values = run_spring(&spring, 0.0, 1.0, 1.0 / 60.0);

    let last = *values.last().unwrap();
    assert!(
        (last - 1.0).abs() < spring.epsilon,
        "Spring should converge to target 1.0, got {last}"
    );
}

#[test]
fn spring_critically_damped_no_overshoot() {
    let spring = Spring {
        damping: 1.0,
        ..Spring::default()
    };
    let values = run_spring(&spring, 0.0, 1.0, 1.0 / 60.0);

    for (i, &v) in values.iter().enumerate() {
        assert!(
            v <= 1.0 + spring.epsilon,
            "Critically damped spring overshot at step {i}: {v}"
        );
    }
}

#[test]
fn spring_underdamped_overshoots() {
    let spring = Spring {
        damping: 0.5,
        ..Spring::default()
    };
    let values = run_spring(&spring, 0.0, 1.0, 1.0 / 60.0);

    let max = values.iter().copied().reduce(f32::max).unwrap();
    assert!(
        max > 1.0,
        "Underdamped spring should overshoot, max was {max}"
    );
}

#[test]
fn spring_overdamped_no_overshoot() {
    let spring = Spring {
        damping: 2.0,
        ..Spring::default()
    };
    let values = run_spring(&spring, 0.0, 1.0, 1.0 / 60.0);

    for (i, &v) in values.iter().enumerate() {
        assert!(
            v <= 1.0 + spring.epsilon,
            "Overdamped spring overshot at step {i}: {v}"
        );
    }
}

#[test]
fn spring_zero_dt_no_change() {
    let spring = Spring::default();
    let (val, vel, _) = spring.step(0.5, 1.0, 0.3, 0.0);
    assert_eq!(val, 0.5, "Zero dt should not change value");
    assert_eq!(vel, 0.3, "Zero dt should not change velocity");
}

#[test]
fn spring_large_dt_clamped() {
    let spring = Spring::default();
    // With dt=1.0, the spring should clamp to MAX_DT (~33ms) and not diverge.
    let (val, _, _) = spring.step(0.0, 1.0, 0.0, 1.0);
    assert!(
        val.is_finite() && val.abs() < 100.0,
        "Large dt should be clamped, got {val}"
    );
}

#[test]
fn spring_at_rest_is_done() {
    let spring = Spring::default();
    let (_, _, done) = spring.step(1.0, 1.0, 0.0, 1.0 / 60.0);
    assert!(done, "Spring at target with zero velocity should be done");
}

#[test]
fn spring_default_parameters_reasonable() {
    let spring = Spring::default();
    let values = run_spring(&spring, 0.0, 1.0, 1.0 / 60.0);

    // Should converge within a reasonable number of frames (~300 at 60fps = 5 seconds).
    assert!(
        values.len() < 300,
        "Default spring took {} steps to converge",
        values.len()
    );

    // Should reach 90% of target within 30 frames (~0.5 seconds).
    assert!(
        values.len() > 30 || values[values.len().min(30) - 1] > 0.9,
        "Default spring should be responsive"
    );
}

#[test]
fn spring_negative_velocity_direction() {
    let spring = Spring::default();
    // Start above target and converge downward.
    let values = run_spring(&spring, 2.0, 1.0, 1.0 / 60.0);

    let last = *values.last().unwrap();
    assert!(
        (last - 1.0).abs() < spring.epsilon,
        "Spring should converge to target 1.0 from above, got {last}"
    );
}
