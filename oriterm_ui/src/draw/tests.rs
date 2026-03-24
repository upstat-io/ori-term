//! Unit tests for draw primitives.

use crate::color::Color;

use super::{BorderSides, RectStyle, Shadow};

// RectStyle

#[test]
fn rect_style_default_is_invisible() {
    let s = RectStyle::default();
    assert!(s.fill.is_none());
    assert!(s.border.is_empty());
    assert_eq!(s.corner_radius, [0.0; 4]);
    assert!(s.shadow.is_none());
    assert!(s.gradient.is_none());
}

#[test]
fn rect_style_filled() {
    let s = RectStyle::filled(Color::WHITE);
    assert_eq!(s.fill, Some(Color::WHITE));
}

#[test]
fn rect_style_builder_chain() {
    let s = RectStyle::filled(Color::BLACK)
        .with_border(2.0, Color::WHITE)
        .with_radius(8.0)
        .with_shadow(Shadow {
            offset_x: 0.0,
            offset_y: 4.0,
            blur_radius: 8.0,
            spread: 0.0,
            color: Color::rgba(0.0, 0.0, 0.0, 0.5),
        });

    assert_eq!(s.fill, Some(Color::BLACK));
    assert_eq!(s.border, BorderSides::uniform(2.0, Color::WHITE));
    assert_eq!(s.corner_radius, [8.0; 4]);
    assert!(s.shadow.is_some());
}

#[test]
fn rect_style_per_corner_radius() {
    let s = RectStyle::filled(Color::BLACK).with_per_corner_radius(1.0, 2.0, 3.0, 4.0);
    assert_eq!(s.corner_radius, [1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn rect_style_default_border_is_empty() {
    let s = RectStyle::default();
    assert!(s.border.is_empty());
}

#[test]
fn rect_style_with_border_creates_uniform() {
    let s = RectStyle::default().with_border(2.0, Color::WHITE);
    assert_eq!(s.border, BorderSides::uniform(2.0, Color::WHITE));
}

#[test]
fn rect_style_with_border_top_only_sets_top() {
    let s = RectStyle::default().with_border_top(1.0, Color::BLACK);
    assert_eq!(s.border.widths(), [1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn rect_style_composable_border_sides() {
    let s = RectStyle::default()
        .with_border(2.0, Color::WHITE)
        .with_border_left(3.0, Color::BLACK);
    // Uniform sets all 4, then left override replaces left only.
    assert_eq!(s.border.widths(), [2.0, 2.0, 2.0, 3.0]);
    assert_eq!(s.border.colors()[3], Color::BLACK);
    assert_eq!(s.border.colors()[0], Color::WHITE);
}

#[test]
fn rect_style_with_border_overrides_previous() {
    let s = RectStyle::default()
        .with_border_left(3.0, Color::BLACK)
        .with_border(2.0, Color::WHITE);
    // Uniform replaces all sides, including the previous left override.
    assert_eq!(s.border, BorderSides::uniform(2.0, Color::WHITE));
}
