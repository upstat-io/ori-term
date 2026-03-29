//! Tests for `MockMeasurer`.

use crate::text::{TextStyle, TextTransform};
use crate::widgets::text_measurer::TextMeasurer;

use super::MockMeasurer;

#[test]
fn basic_measure() {
    let m = MockMeasurer::new();
    let style = TextStyle::default();
    let metrics = m.measure("hello", &style, f32::INFINITY);
    assert_eq!(metrics.width, 40.0); // 5 chars * 8px
    assert_eq!(metrics.height, 16.0);
    assert_eq!(metrics.line_count, 1);
}

#[test]
fn wrapping() {
    let m = MockMeasurer::new();
    let style = TextStyle::default();
    // "hello world" = 11 chars * 8px = 88px, max_width = 50px -> 2 lines.
    let metrics = m.measure("hello world", &style, 50.0);
    assert_eq!(metrics.width, 50.0);
    assert_eq!(metrics.line_count, 2);
    assert_eq!(metrics.height, 32.0);
}

#[test]
fn shape_produces_one_glyph_per_char() {
    let m = MockMeasurer::new();
    let style = TextStyle::default();
    let shaped = m.shape("abc", &style, f32::INFINITY);
    assert_eq!(shaped.glyph_count(), 3);
    assert_eq!(shaped.width, 24.0);
    assert_eq!(shaped.height, 16.0);
}

#[test]
fn empty_text() {
    let m = MockMeasurer::new();
    let style = TextStyle::default();
    let metrics = m.measure("", &style, f32::INFINITY);
    assert_eq!(metrics.width, 0.0);
    assert_eq!(metrics.height, 16.0);
    assert_eq!(metrics.line_count, 1);
}

// Non-ASCII regression (TPR-03-008)

#[test]
fn non_ascii_width_matches_glyph_count() {
    let m = MockMeasurer::new();
    let style = TextStyle::default();

    // "café" = 4 chars but 5 UTF-8 bytes (é = 2 bytes).
    let metrics = m.measure("café", &style, f32::INFINITY);
    let shaped = m.shape("café", &style, f32::INFINITY);

    assert_eq!(
        metrics.width, 32.0,
        "4 chars * 8px = 32, not 5 bytes * 8px = 40"
    );
    assert_eq!(shaped.glyph_count(), 4, "one glyph per character");
    assert_eq!(shaped.width, 32.0, "shaped width must match measured width");
}

#[test]
fn transform_multibyte_consistent() {
    let m = MockMeasurer::new();
    let style = TextStyle::new(13.0, crate::color::Color::WHITE)
        .with_text_transform(TextTransform::Uppercase);

    // "straße" uppercases to "STRASSE" (6 chars -> 7 chars).
    let metrics = m.measure("straße", &style, f32::INFINITY);
    let shaped = m.shape("straße", &style, f32::INFINITY);

    assert_eq!(shaped.glyph_count(), 7, "STRASSE has 7 chars");
    assert_eq!(
        metrics.width, shaped.width,
        "measure and shape must agree on width"
    );
}

// Line height (Section 04)

#[test]
fn measure_line_height_overrides_height() {
    let m = MockMeasurer::new();
    let style = TextStyle::new(12.0, crate::color::Color::WHITE).with_line_height(1.5);
    let metrics = m.measure("Hello", &style, f32::INFINITY);
    assert_eq!(
        metrics.height, 18.0,
        "12.0 * 1.5 = 18.0, not mock's natural 16.0"
    );
}

#[test]
fn measure_none_line_height_uses_natural() {
    let m = MockMeasurer::new();
    let style = TextStyle::new(12.0, crate::color::Color::WHITE);
    let metrics = m.measure("Hello", &style, f32::INFINITY);
    assert_eq!(metrics.height, 16.0);
}

#[test]
fn measure_invalid_line_height_uses_natural() {
    let m = MockMeasurer::new();
    for invalid in [0.0_f32, -1.0, f32::NAN] {
        let mut style = TextStyle::new(12.0, crate::color::Color::WHITE);
        style.line_height = Some(invalid);
        let metrics = m.measure("Hello", &style, f32::INFINITY);
        assert_eq!(
            metrics.height, 16.0,
            "invalid {invalid} falls back to natural"
        );
    }
}

#[test]
fn shape_baseline_shifts_with_line_height() {
    let m = MockMeasurer::new();
    let style = TextStyle::new(12.0, crate::color::Color::WHITE).with_line_height(1.5);
    let shaped = m.shape("Hello", &style, f32::INFINITY);

    // target = 18.0, natural = 16.0, half_leading = 1.0.
    // baseline = 16.0 * 0.8 + 1.0 = 13.8.
    assert!(
        (shaped.baseline - 13.8).abs() < 0.001,
        "baseline should be 13.8, got {}",
        shaped.baseline,
    );
}

#[test]
fn shape_height_matches_measure_height() {
    let m = MockMeasurer::new();
    let style = TextStyle::new(12.0, crate::color::Color::WHITE).with_line_height(1.5);
    let metrics = m.measure("Hello", &style, f32::INFINITY);
    let shaped = m.shape("Hello", &style, f32::INFINITY);
    assert_eq!(
        shaped.height, metrics.height,
        "shape and measure must agree"
    );
}

#[test]
fn measure_line_height_multiline() {
    let m = MockMeasurer::new();
    let style = TextStyle::new(10.0, crate::color::Color::WHITE).with_line_height(2.0);
    // "hello world" = 11 chars * 8px = 88px. With max_width=50, wraps to 2 lines.
    let metrics = m.measure("hello world", &style, 50.0);
    assert_eq!(metrics.line_count, 2);
    // Each line = 10.0 * 2.0 = 20.0, total = 40.0.
    assert_eq!(metrics.height, 40.0, "2 lines * 20px each = 40px");
}

#[test]
fn shape_negative_half_leading() {
    let m = MockMeasurer::new();
    let style = TextStyle::new(12.0, crate::color::Color::WHITE).with_line_height(0.5);
    let shaped = m.shape("Hello", &style, f32::INFINITY);

    // target = 6.0, natural = 16.0, half_leading = (6 - 16) / 2 = -5.0.
    // baseline = 16.0 * 0.8 + (-5.0) = 7.8.
    assert!(
        (shaped.baseline - 7.8).abs() < 0.001,
        "negative half-leading: baseline should be 7.8, got {}",
        shaped.baseline,
    );
    assert_eq!(shaped.height, 6.0);
}
