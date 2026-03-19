//! Unit tests for draw primitives.

use crate::color::Color;

use super::{Border, RectStyle, Shadow};

// RectStyle

#[test]
fn rect_style_default_is_invisible() {
    let s = RectStyle::default();
    assert!(s.fill.is_none());
    assert!(s.border.is_none());
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
    assert_eq!(
        s.border,
        Some(Border {
            width: 2.0,
            color: Color::WHITE,
        })
    );
    assert_eq!(s.corner_radius, [8.0; 4]);
    assert!(s.shadow.is_some());
}

#[test]
fn rect_style_per_corner_radius() {
    let s = RectStyle::filled(Color::BLACK).with_per_corner_radius(1.0, 2.0, 3.0, 4.0);
    assert_eq!(s.corner_radius, [1.0, 2.0, 3.0, 4.0]);
}
