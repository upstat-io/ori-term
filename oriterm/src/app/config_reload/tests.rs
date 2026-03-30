//! Tests for zoom and reset font size computation.

use super::{MAX_FONT_SIZE, MIN_FONT_SIZE, compute_reset_size, compute_zoomed_size};

#[test]
fn zoom_in_from_default() {
    let result = compute_zoomed_size(11.0, 1.0);
    assert_eq!(result, Some(12.0));
}

#[test]
fn zoom_out_from_default() {
    let result = compute_zoomed_size(11.0, -1.0);
    assert_eq!(result, Some(10.0));
}

#[test]
fn zoom_clamps_at_minimum() {
    let result = compute_zoomed_size(MIN_FONT_SIZE, -1.0);
    assert_eq!(result, None);
}

#[test]
fn zoom_clamps_at_maximum() {
    let result = compute_zoomed_size(MAX_FONT_SIZE, 1.0);
    assert_eq!(result, None);
}

#[test]
fn zoom_near_minimum_clamps() {
    let result = compute_zoomed_size(5.0, -2.0);
    assert_eq!(result, Some(MIN_FONT_SIZE));
}

#[test]
fn zoom_near_maximum_clamps() {
    let result = compute_zoomed_size(71.0, 3.0);
    assert_eq!(result, Some(MAX_FONT_SIZE));
}

#[test]
fn zoom_noop_when_already_at_min() {
    let result = compute_zoomed_size(MIN_FONT_SIZE, 0.0);
    assert_eq!(result, None);
}

#[test]
fn zoom_noop_when_delta_zero() {
    let result = compute_zoomed_size(20.0, 0.0);
    assert_eq!(result, None);
}

#[test]
fn zoom_large_negative_delta_clamps() {
    let result = compute_zoomed_size(11.0, -100.0);
    assert_eq!(result, Some(MIN_FONT_SIZE));
}

#[test]
fn zoom_large_positive_delta_clamps() {
    let result = compute_zoomed_size(11.0, 100.0);
    assert_eq!(result, Some(MAX_FONT_SIZE));
}

#[test]
fn zoom_fractional_delta() {
    let result = compute_zoomed_size(11.0, 0.5);
    assert_eq!(result, Some(11.5));
}

// ── Reset size computation ──────────────────────────────────────────

#[test]
fn reset_noop_when_already_at_configured() {
    let result = compute_reset_size(11.0, 11.0);
    assert_eq!(result, None);
}

#[test]
fn reset_returns_configured_when_different() {
    let result = compute_reset_size(14.0, 11.0);
    assert_eq!(result, Some(11.0));
}

#[test]
fn reset_after_zoom_in() {
    // Simulates: user zoomed to 15.0, config says 11.0 → should reset.
    let result = compute_reset_size(15.0, 11.0);
    assert_eq!(result, Some(11.0));
}

#[test]
fn reset_after_zoom_out() {
    // Simulates: user zoomed to 8.0, config says 11.0 → should reset.
    let result = compute_reset_size(8.0, 11.0);
    assert_eq!(result, Some(11.0));
}
