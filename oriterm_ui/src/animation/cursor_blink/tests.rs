use std::time::Duration;

use super::{CursorBlink, DEFAULT_BLINK_INTERVAL, FADE_FRACTION};
use crate::animation::Easing;

/// Phase boundary offsets for a symmetric blink interval.
struct PhaseBounds {
    /// Start of fade-out transition (end of visible plateau).
    fade_out_start: Duration,
    /// End of fade-out (= `in_duration`, start of hidden plateau).
    fade_out_end: Duration,
    /// End of hidden plateau (start of fade-in).
    hidden_end: Duration,
    /// Full cycle duration (`in_duration + out_duration`).
    cycle: Duration,
}

impl PhaseBounds {
    fn new(interval: Duration) -> Self {
        let secs = interval.as_secs_f64();
        let ff = f64::from(FADE_FRACTION);
        Self {
            fade_out_start: Duration::from_secs_f64(secs * (1.0 - ff)),
            fade_out_end: interval,
            hidden_end: Duration::from_secs_f64(secs + secs * (1.0 - ff)),
            cycle: interval * 2,
        }
    }
}

// --- Intensity: plateaus ---

#[test]
fn initial_intensity_is_full() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let i = blink.intensity_at(Duration::ZERO);
    assert!((i - 1.0).abs() < f32::EPSILON, "expected 1.0, got {i}");
}

#[test]
fn intensity_plateau_stable() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let pb = PhaseBounds::new(DEFAULT_BLINK_INTERVAL);

    // Visible plateau: 5 evenly-spaced samples in [0, fade_out_start).
    let vis_end = pb.fade_out_start.as_secs_f64();
    for i in 0..5 {
        let t = vis_end * (i as f64 / 5.0);
        let intensity = blink.intensity_at(Duration::from_secs_f64(t));
        assert!(
            (intensity - 1.0).abs() < f32::EPSILON,
            "visible plateau at {t:.4}s: expected 1.0, got {intensity}",
        );
    }

    // Hidden plateau: 5 evenly-spaced samples in [fade_out_end, hidden_end).
    let hid_start = pb.fade_out_end.as_secs_f64();
    let hid_end = pb.hidden_end.as_secs_f64();
    for i in 0..5 {
        let t = hid_start + (hid_end - hid_start) * (i as f64 / 5.0);
        let intensity = blink.intensity_at(Duration::from_secs_f64(t));
        assert!(
            intensity.abs() < f32::EPSILON,
            "hidden plateau at {t:.4}s: expected 0.0, got {intensity}",
        );
    }
}

// --- Intensity: fade transitions ---

#[test]
fn opacity_ramp_over_one_cycle() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let pb = PhaseBounds::new(DEFAULT_BLINK_INTERVAL);

    // Start: full opacity.
    assert!((blink.intensity_at(Duration::ZERO) - 1.0).abs() < f32::EPSILON);

    // Just before in_duration boundary: near zero.
    let near_end = Duration::from_secs_f64(pb.fade_out_end.as_secs_f64() - 0.001);
    assert!(
        blink.intensity_at(near_end) < 0.05,
        "should be near zero at end of fade-out",
    );

    // Middle of hidden plateau: zero.
    let mid_hidden = Duration::from_secs_f64(
        (pb.fade_out_end.as_secs_f64() + pb.hidden_end.as_secs_f64()) / 2.0,
    );
    assert!(blink.intensity_at(mid_hidden).abs() < f32::EPSILON);

    // Just before cycle end: near one.
    let near_cycle = Duration::from_secs_f64(pb.cycle.as_secs_f64() - 0.001);
    assert!(
        blink.intensity_at(near_cycle) > 0.95,
        "should be near one at end of fade-in",
    );

    // Cycle boundary wraps back to visible plateau.
    assert!((blink.intensity_at(pb.cycle) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn intensity_monotonic_during_fade_out() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let pb = PhaseBounds::new(DEFAULT_BLINK_INTERVAL);

    let start = pb.fade_out_start.as_secs_f64();
    let end = pb.fade_out_end.as_secs_f64();

    let mut prev = 1.0_f32;
    for i in 0..=10 {
        let frac = i as f64 / 10.0;
        let t = start + frac * (end - start);
        let intensity = blink.intensity_at(Duration::from_secs_f64(t));
        assert!(
            intensity <= prev + f32::EPSILON,
            "fade-out not monotonic at sample {i}/10: {intensity} > {prev}",
        );
        prev = intensity;
    }
    assert!(prev < 0.01, "fade-out should reach near zero: {prev}");
}

#[test]
fn intensity_monotonic_during_fade_in() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let pb = PhaseBounds::new(DEFAULT_BLINK_INTERVAL);

    let start = pb.hidden_end.as_secs_f64();
    let end = pb.cycle.as_secs_f64();

    let mut prev = 0.0_f32;
    for i in 0..=10 {
        let frac = i as f64 / 10.0;
        let t = start + frac * (end - start);
        let intensity = blink.intensity_at(Duration::from_secs_f64(t));
        assert!(
            intensity >= prev - f32::EPSILON,
            "fade-in not monotonic at sample {i}/10: {intensity} < {prev}",
        );
        prev = intensity;
    }
    assert!(prev > 0.99, "fade-in should reach near one: {prev}");
}

// --- Reset ---

#[test]
fn reset_returns_to_full_opacity() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);

    // Backdate into hidden plateau.
    blink.epoch -= Duration::from_millis(630);
    assert!(blink.intensity() < 0.01);

    blink.reset();
    assert!((blink.intensity() - 1.0).abs() < 0.02);
    // update reports no change (both at 1.0).
    assert!(!blink.update());
}

// --- next_change scheduling ---

#[test]
fn next_change_during_plateau_returns_phase_boundary() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let now = std::time::Instant::now();
    let next = blink.next_change();

    // Fresh blink is at visible plateau. next_change points to fade_out_start.
    let expected_secs = DEFAULT_BLINK_INTERVAL.as_secs_f64() * (1.0 - f64::from(FADE_FRACTION));
    let expected = Duration::from_secs_f64(expected_secs);
    let delta = next.duration_since(now);

    assert!(
        delta <= expected + Duration::from_millis(50),
        "next_change too far: {delta:?} (expected ~{expected:?})",
    );
    assert!(
        delta >= Duration::from_millis(1),
        "next_change should be in the future: {delta:?}",
    );
}

#[test]
fn next_change_during_fade_returns_animation_frame() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);

    // Backdate into the fade-out region.
    let fade_out_start_secs =
        DEFAULT_BLINK_INTERVAL.as_secs_f64() * (1.0 - f64::from(FADE_FRACTION));
    blink.epoch -= Duration::from_secs_f64(fade_out_start_secs + 0.010);

    let now = std::time::Instant::now();
    let next = blink.next_change();
    let delta = next.duration_since(now);

    assert!(
        delta <= Duration::from_millis(30),
        "fade wakeup should be ~16ms, got {delta:?}",
    );
}

#[test]
fn next_change_during_hidden_plateau() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);

    // Backdate into the hidden plateau (e.g. 600ms into a 530+530 cycle).
    blink.epoch -= Duration::from_millis(600);

    let now = std::time::Instant::now();
    let next = blink.next_change();
    let delta = next.duration_since(now);

    // Hidden plateau ends at in + out*(1-FF) = 530 + 328.6 = 858.6ms.
    // Elapsed is 600ms. Remaining = 858.6 - 600 = 258.6ms.
    assert!(
        delta >= Duration::from_millis(200),
        "hidden plateau wait should be >200ms, got {delta:?}",
    );
    assert!(
        delta <= Duration::from_millis(320),
        "hidden plateau wait should be <320ms, got {delta:?}",
    );
}

// --- EaseInOut key values ---

#[test]
fn ease_in_out_key_values() {
    let ease = Easing::EaseInOut;
    assert!(ease.apply(0.0).abs() < f32::EPSILON);
    assert!((ease.apply(0.5) - 0.5).abs() < 0.01);
    assert!((ease.apply(1.0) - 1.0).abs() < f32::EPSILON);
}

// --- set_interval ---

#[test]
fn set_interval_updates_both_durations() {
    let mut blink = CursorBlink::new(Duration::from_millis(1000));

    // 600ms into a 1000ms visible phase: visible plateau ends at 620ms.
    blink.epoch -= Duration::from_millis(600);
    assert!(blink.intensity() > 0.99);

    // Shorten to 500ms. Now 600ms is past in_duration=500ms → hidden plateau.
    blink.set_interval(Duration::from_millis(500));
    assert!(blink.intensity() < 0.01);
}

// --- update ---

#[test]
fn update_during_plateau_reports_no_change() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    assert!(!blink.update());
}

#[test]
fn update_across_phase_boundary_reports_change() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    // Backdate into hidden plateau (large opacity delta).
    blink.epoch -= Duration::from_millis(630);
    assert!(blink.update());
}

// --- Backward-compat methods ---

#[test]
fn is_visible_during_visible_plateau() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    assert!(blink.is_visible());
}

#[test]
fn is_visible_false_during_hidden_plateau() {
    let mut blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    blink.epoch -= Duration::from_millis(630);
    assert!(!blink.is_visible());
}

#[test]
fn next_toggle_delegates_to_next_change() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let toggle = blink.next_toggle();
    let change = blink.next_change();
    let diff = if toggle > change {
        toggle.duration_since(change)
    } else {
        change.duration_since(toggle)
    };
    assert!(
        diff < Duration::from_millis(5),
        "next_toggle and next_change diverged by {diff:?}",
    );
}

// --- Cycle wrapping ---

#[test]
fn intensity_wraps_correctly_across_cycles() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    let cycle = DEFAULT_BLINK_INTERVAL * 2;

    let offset = Duration::from_millis(100);
    let i0 = blink.intensity_at(offset);
    let i3 = blink.intensity_at(cycle * 3 + offset);

    assert!(
        (i0 - i3).abs() < f32::EPSILON,
        "cycle 0 ({i0}) and cycle 3 ({i3}) differ at same offset",
    );
}

// --- Edge cases ---

#[test]
fn zero_interval_returns_full_opacity() {
    let blink = CursorBlink::new(Duration::ZERO);
    assert!((blink.intensity() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn skipped_updates_do_not_accumulate_drift() {
    let mut blink = CursorBlink::new(Duration::from_millis(100));

    // Skip 3 intervals (300ms). Cycle = 200ms. 300 % 200 = 100ms.
    // At 100ms = in_duration: start of hidden plateau. intensity = 0.0.
    blink.epoch -= Duration::from_millis(300);
    assert!(blink.intensity() < 0.01);
    assert!(blink.update()); // Changed from 1.0 to ~0.0.

    // Skip 2 more (500ms total). 500 % 200 = 100ms. Same position.
    blink.epoch -= Duration::from_millis(200);
    assert!(blink.intensity() < 0.01);
    assert!(!blink.update()); // No change.
}

#[test]
fn double_cycle_restores_visibility() {
    let blink = CursorBlink::new(DEFAULT_BLINK_INTERVAL);
    // Two full cycles: back to visible plateau.
    let two_cycles = DEFAULT_BLINK_INTERVAL * 4;
    let i = blink.intensity_at(two_cycles);
    assert!(
        (i - 1.0).abs() < f32::EPSILON,
        "expected 1.0 after full cycles, got {i}",
    );
}
