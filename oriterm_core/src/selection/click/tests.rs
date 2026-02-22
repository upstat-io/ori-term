//! Tests for ClickDetector multi-click detection.

use std::time::{Duration, Instant};

use super::ClickDetector;

#[test]
fn first_click_returns_one() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(5, 10), 1);
}

#[test]
fn rapid_same_position_cycles_1_2_3_1() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(5, 10), 1);
    assert_eq!(d.click(5, 10), 2);
    assert_eq!(d.click(5, 10), 3);
    assert_eq!(d.click(5, 10), 1);
}

#[test]
fn different_position_resets_to_one() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(5, 10), 1);
    assert_eq!(d.click(5, 10), 2);
    // Move to different column.
    assert_eq!(d.click(6, 10), 1);
}

#[test]
fn different_row_resets_to_one() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(5, 10), 1);
    assert_eq!(d.click(5, 10), 2);
    // Move to different row.
    assert_eq!(d.click(5, 11), 1);
}

#[test]
fn expired_time_window_resets_to_one() {
    let mut d = ClickDetector::new();
    let t0 = Instant::now();
    assert_eq!(d.click_at(5, 10, t0), 1);
    // 550ms > 500ms threshold — resets.
    assert_eq!(d.click_at(5, 10, t0 + Duration::from_millis(550)), 1);
}

#[test]
fn reset_clears_all_state() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(5, 10), 1);
    assert_eq!(d.click(5, 10), 2);
    d.reset();
    assert_eq!(d.click(5, 10), 1);
}

#[test]
fn default_is_same_as_new() {
    let d = ClickDetector::default();
    assert!(d.last_time.is_none());
    assert!(d.last_pos.is_none());
    assert_eq!(d.count, 0);
}

#[test]
fn triple_click_then_different_position() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(0, 0), 1);
    assert_eq!(d.click(0, 0), 2);
    assert_eq!(d.click(0, 0), 3);
    // Move to a different cell.
    assert_eq!(d.click(1, 0), 1);
    assert_eq!(d.click(1, 0), 2);
}

#[test]
fn click_just_within_threshold_counts_as_multi() {
    let mut d = ClickDetector::new();
    let t0 = Instant::now();
    assert_eq!(d.click_at(3, 7, t0), 1);
    // 450ms < 500ms threshold — should still count.
    assert_eq!(d.click_at(3, 7, t0 + Duration::from_millis(450)), 2);
}

#[test]
fn return_to_original_position_starts_new_sequence() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(5, 10), 1);
    assert_eq!(d.click(5, 10), 2);
    // Different position breaks the chain.
    assert_eq!(d.click(6, 10), 1);
    // Return to original position — fresh sequence, not continuing the old one.
    assert_eq!(d.click(5, 10), 1);
}

#[test]
fn two_full_cycles() {
    let mut d = ClickDetector::new();
    // First cycle.
    assert_eq!(d.click(2, 3), 1);
    assert_eq!(d.click(2, 3), 2);
    assert_eq!(d.click(2, 3), 3);
    assert_eq!(d.click(2, 3), 1);
    // Second cycle.
    assert_eq!(d.click(2, 3), 2);
    assert_eq!(d.click(2, 3), 3);
    assert_eq!(d.click(2, 3), 1);
}

#[test]
fn large_coordinates() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(9999, 9999), 1);
    assert_eq!(d.click(9999, 9999), 2);
    assert_eq!(d.click(9999, 9999), 3);
}

#[test]
fn zero_coordinates() {
    let mut d = ClickDetector::new();
    assert_eq!(d.click(0, 0), 1);
    assert_eq!(d.click(0, 0), 2);
    assert_eq!(d.click(0, 0), 3);
    assert_eq!(d.click(0, 0), 1);
}
