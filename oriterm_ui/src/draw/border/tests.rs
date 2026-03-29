//! Unit tests for border types.

use crate::color::Color;

use super::{Border, BorderSides};

// BorderSides::default / is_empty

#[test]
fn border_sides_default_is_empty() {
    assert!(BorderSides::default().is_empty());
}

// BorderSides::uniform

#[test]
fn border_sides_uniform_all_sides_equal() {
    let sides = BorderSides::uniform(2.0, Color::WHITE);
    let expected = Some(Border {
        width: 2.0,
        color: Color::WHITE,
    });
    assert_eq!(sides.top, expected);
    assert_eq!(sides.right, expected);
    assert_eq!(sides.bottom, expected);
    assert_eq!(sides.left, expected);
}

// BorderSides::as_uniform

#[test]
fn border_sides_as_uniform_returns_some_when_identical() {
    let sides = BorderSides::uniform(3.0, Color::BLACK);
    let u = sides.as_uniform().expect("should be uniform");
    assert_eq!(u.width, 3.0);
    assert_eq!(u.color, Color::BLACK);
}

#[test]
fn border_sides_as_uniform_returns_none_when_different() {
    let mut sides = BorderSides::uniform(2.0, Color::WHITE);
    sides.left = Some(Border {
        width: 4.0,
        color: Color::WHITE,
    });
    assert!(sides.as_uniform().is_none());
}

#[test]
fn border_sides_as_uniform_returns_none_when_colors_differ() {
    let mut sides = BorderSides::uniform(2.0, Color::WHITE);
    sides.bottom = Some(Border {
        width: 2.0,
        color: Color::BLACK,
    });
    assert!(sides.as_uniform().is_none());
}

// BorderSides::only_* constructors

#[test]
fn border_sides_only_top_leaves_others_none() {
    let sides = BorderSides::only_top(1.0, Color::WHITE);
    assert!(sides.top.is_some());
    assert!(sides.right.is_none());
    assert!(sides.bottom.is_none());
    assert!(sides.left.is_none());
}

#[test]
fn border_sides_only_right_leaves_others_none() {
    let sides = BorderSides::only_right(1.0, Color::WHITE);
    assert!(sides.top.is_none());
    assert!(sides.right.is_some());
    assert!(sides.bottom.is_none());
    assert!(sides.left.is_none());
}

#[test]
fn border_sides_only_bottom_leaves_others_none() {
    let sides = BorderSides::only_bottom(1.0, Color::WHITE);
    assert!(sides.top.is_none());
    assert!(sides.right.is_none());
    assert!(sides.bottom.is_some());
    assert!(sides.left.is_none());
}

#[test]
fn border_sides_only_left_leaves_others_none() {
    let sides = BorderSides::only_left(1.0, Color::WHITE);
    assert!(sides.top.is_none());
    assert!(sides.right.is_none());
    assert!(sides.bottom.is_none());
    assert!(sides.left.is_some());
}

// BorderSides::widths / colors

#[test]
fn border_sides_widths_returns_correct_array() {
    let sides = BorderSides {
        top: Some(Border {
            width: 1.0,
            color: Color::WHITE,
        }),
        right: None,
        bottom: Some(Border {
            width: 3.0,
            color: Color::WHITE,
        }),
        left: Some(Border {
            width: 4.0,
            color: Color::WHITE,
        }),
    };
    assert_eq!(sides.widths(), [1.0, 0.0, 3.0, 4.0]);
}

#[test]
fn border_sides_colors_uses_transparent_for_absent() {
    let sides = BorderSides::only_top(2.0, Color::BLACK);
    let colors = sides.colors();
    assert_eq!(colors[0], Color::BLACK);
    assert_eq!(colors[1], Color::TRANSPARENT);
    assert_eq!(colors[2], Color::TRANSPARENT);
    assert_eq!(colors[3], Color::TRANSPARENT);
}

// Width normalization

#[test]
fn border_sides_normalizes_invalid_width() {
    let cases = [0.0, -1.0, f32::NAN];
    for w in cases {
        let sides = BorderSides::uniform(w, Color::WHITE);
        assert!(sides.is_empty(), "width {w} should be treated as no border");
        assert_eq!(sides.widths(), [0.0; 4]);
    }
}

#[test]
fn border_sides_normalizes_infinity_width() {
    let sides = BorderSides::uniform(f32::INFINITY, Color::WHITE);
    assert!(sides.is_empty());
    assert_eq!(sides.widths(), [0.0; 4]);

    let sides = BorderSides::uniform(f32::NEG_INFINITY, Color::WHITE);
    assert!(sides.is_empty());
    assert_eq!(sides.widths(), [0.0; 4]);
}

// Colors normalized for invisible sides

#[test]
fn border_sides_colors_transparent_for_zero_width() {
    let sides = BorderSides {
        top: Some(Border {
            width: 2.0,
            color: Color::BLACK,
        }),
        right: Some(Border {
            width: 0.0,
            color: Color::WHITE,
        }),
        bottom: Some(Border {
            width: -1.0,
            color: Color::WHITE,
        }),
        left: Some(Border {
            width: f32::NAN,
            color: Color::WHITE,
        }),
    };
    let colors = sides.colors();
    assert_eq!(colors[0], Color::BLACK, "visible side keeps its color");
    assert_eq!(
        colors[1],
        Color::TRANSPARENT,
        "zero-width side is transparent"
    );
    assert_eq!(
        colors[2],
        Color::TRANSPARENT,
        "negative-width side is transparent"
    );
    assert_eq!(
        colors[3],
        Color::TRANSPARENT,
        "NaN-width side is transparent"
    );
}

// PartialEq

#[test]
fn border_sides_partial_eq_distinguishes_sides() {
    let a = BorderSides::only_top(2.0, Color::WHITE);
    let b = BorderSides::only_left(2.0, Color::WHITE);
    assert_ne!(a, b);
}
