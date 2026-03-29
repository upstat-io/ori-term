//! Unit tests for text types.

use crate::color::Color;

use super::{
    FontWeight, ShapedGlyph, ShapedText, TextAlign, TextMetrics, TextOverflow, TextStyle,
    TextTransform,
};

// TextStyle

#[test]
fn text_style_default() {
    let s = TextStyle::default();
    assert!(s.font_family.is_none());
    assert_eq!(s.size, 12.0);
    assert_eq!(s.weight, FontWeight::NORMAL);
    assert_eq!(s.color, Color::WHITE);
    assert_eq!(s.align, TextAlign::Left);
    assert_eq!(s.overflow, TextOverflow::Clip);
    assert_eq!(s.text_transform, TextTransform::None);
    assert_eq!(s.line_height, None);
}

#[test]
fn text_style_new() {
    let s = TextStyle::new(16.0, Color::BLACK);
    assert_eq!(s.size, 16.0);
    assert_eq!(s.color, Color::BLACK);
    assert_eq!(s.weight, FontWeight::NORMAL);
    assert_eq!(s.line_height, None);
}

#[test]
fn text_style_builder_chain() {
    let s = TextStyle::new(14.0, Color::WHITE)
        .with_weight(FontWeight::BOLD)
        .with_align(TextAlign::Center)
        .with_overflow(TextOverflow::Ellipsis);

    assert_eq!(s.size, 14.0);
    assert_eq!(s.weight, FontWeight::BOLD);
    assert_eq!(s.align, TextAlign::Center);
    assert_eq!(s.overflow, TextOverflow::Ellipsis);
}

// ShapedGlyph

#[test]
fn shaped_glyph_construction() {
    let g = ShapedGlyph {
        glyph_id: 42,
        face_index: 0,
        synthetic: 0,
        x_advance: 7.5,
        x_offset: 0.0,
        y_offset: 0.0,
    };
    assert_eq!(g.glyph_id, 42);
    assert_eq!(g.face_index, 0);
    assert_eq!(g.x_advance, 7.5);
}

// ShapedText

#[test]
fn shaped_text_empty() {
    let t = ShapedText::new(Vec::new(), 0.0, 0.0, 0.0, 0, 400);
    assert!(t.is_empty());
    assert_eq!(t.glyph_count(), 0);
    assert_eq!(t.width, 0.0);
}

#[test]
fn shaped_text_with_glyphs() {
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 10,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 20,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.0,
            y_offset: 0.0,
        },
    ];
    let t = ShapedText::new(glyphs, 16.0, 20.0, 14.0, 0, 400);
    assert!(!t.is_empty());
    assert_eq!(t.glyph_count(), 2);
    assert_eq!(t.width, 16.0);
    assert_eq!(t.height, 20.0);
    assert_eq!(t.baseline, 14.0);
}

// TextMetrics

#[test]
fn text_metrics_single_line() {
    let m = TextMetrics {
        width: 50.0,
        height: 16.0,
        line_count: 1,
    };
    assert_eq!(m.width, 50.0);
    assert_eq!(m.height, 16.0);
    assert_eq!(m.line_count, 1);
}

#[test]
fn text_metrics_multi_line() {
    let m = TextMetrics {
        width: 100.0,
        height: 48.0,
        line_count: 3,
    };
    assert_eq!(m.line_count, 3);
    assert_eq!(m.height, 48.0);
}

// FontWeight

#[test]
fn font_weight_default_is_normal() {
    assert_eq!(FontWeight::default(), FontWeight::NORMAL);
}

#[test]
fn font_weight_constants_have_correct_values() {
    assert_eq!(FontWeight::THIN.value(), 100);
    assert_eq!(FontWeight::EXTRA_LIGHT.value(), 200);
    assert_eq!(FontWeight::LIGHT.value(), 300);
    assert_eq!(FontWeight::NORMAL.value(), 400);
    assert_eq!(FontWeight::MEDIUM.value(), 500);
    assert_eq!(FontWeight::SEMIBOLD.value(), 600);
    assert_eq!(FontWeight::BOLD.value(), 700);
    assert_eq!(FontWeight::EXTRA_BOLD.value(), 800);
    assert_eq!(FontWeight::BLACK.value(), 900);
}

#[test]
fn font_weight_clamp_below_100() {
    assert_eq!(FontWeight::new(50).value(), 100);
    assert_eq!(FontWeight::new(0).value(), 100);
}

#[test]
fn font_weight_clamp_above_900() {
    assert_eq!(FontWeight::new(950).value(), 900);
    assert_eq!(FontWeight::new(u16::MAX).value(), 900);
}

#[test]
fn font_weight_ordering_is_numeric() {
    assert!(FontWeight::LIGHT < FontWeight::NORMAL);
    assert!(FontWeight::NORMAL < FontWeight::MEDIUM);
    assert!(FontWeight::MEDIUM < FontWeight::BOLD);
}

#[test]
fn font_weight_boundary_values() {
    assert_eq!(FontWeight::new(100).value(), 100);
    assert_eq!(FontWeight::new(900).value(), 900);
}

#[test]
fn font_weight_hash_eq_consistent() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let a = FontWeight::NORMAL;
    let b = FontWeight::NORMAL;
    assert_eq!(a, b);

    let mut ha = DefaultHasher::new();
    let mut hb = DefaultHasher::new();
    a.hash(&mut ha);
    b.hash(&mut hb);
    assert_eq!(ha.finish(), hb.finish());

    assert_ne!(FontWeight::NORMAL, FontWeight::MEDIUM);
}

#[test]
fn font_weight_value_roundtrip() {
    for w in (100..=900).step_by(100) {
        assert_eq!(FontWeight::new(w).value(), w);
    }
}

#[test]
fn text_align_default_is_left() {
    assert_eq!(TextAlign::default(), TextAlign::Left);
}

#[test]
fn text_overflow_default_is_clip() {
    assert_eq!(TextOverflow::default(), TextOverflow::Clip);
}

// Boundary value tests

#[test]
fn shaped_text_negative_baseline() {
    // Negative baseline should be stored as-is (no clamping).
    let t = ShapedText::new(Vec::new(), 0.0, 14.0, -5.0, 0, 400);
    assert_eq!(t.baseline, -5.0);
    assert!(t.is_empty());
}

#[test]
fn shaped_glyph_zero_advance() {
    // Zero-advance glyphs (e.g., combining marks) are valid.
    let g = ShapedGlyph {
        glyph_id: 100,
        face_index: 1,
        synthetic: 0,
        x_advance: 0.0,
        x_offset: 2.0,
        y_offset: -3.0,
    };
    assert_eq!(g.x_advance, 0.0);
    assert_eq!(g.x_offset, 2.0);
    assert_eq!(g.y_offset, -3.0);
}

// ShapedText size_q6

#[test]
fn shaped_text_new_stores_size_q6() {
    let t = ShapedText::new(Vec::new(), 0.0, 14.0, 12.0, 850, 400);
    assert_eq!(t.size_q6, 850);
}

#[test]
fn shaped_text_zero_size_q6_is_valid() {
    // Test/mock code passes 0; must not panic or assert.
    let t = ShapedText::new(Vec::new(), 0.0, 14.0, 12.0, 0, 400);
    assert_eq!(t.size_q6, 0);
}

#[test]
fn shaped_text_new_stores_weight() {
    let t = ShapedText::new(Vec::new(), 0.0, 14.0, 12.0, 0, 500);
    assert_eq!(t.weight, 500);
}

// TextTransform

#[test]
fn text_transform_default_is_none() {
    assert_eq!(TextTransform::default(), TextTransform::None);
}

#[test]
fn text_transform_none_borrows() {
    let result = TextTransform::None.apply("Hello");
    assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
    assert_eq!(&*result, "Hello");
}

#[test]
fn text_transform_uppercase_matches_explicit() {
    assert_eq!(&*TextTransform::Uppercase.apply("hello"), "HELLO");
}

#[test]
fn text_transform_lowercase_matches_explicit() {
    assert_eq!(&*TextTransform::Lowercase.apply("HELLO"), "hello");
}

#[test]
fn text_transform_uppercase_multibyte_expansion() {
    // German sharp s expands from 1 char to 2 chars.
    assert_eq!(&*TextTransform::Uppercase.apply("straße"), "STRASSE");
}

#[test]
fn text_transform_empty_string() {
    assert_eq!(&*TextTransform::None.apply(""), "");
    assert_eq!(&*TextTransform::Uppercase.apply(""), "");
    assert_eq!(&*TextTransform::Lowercase.apply(""), "");
}

#[test]
fn text_style_with_text_transform() {
    let s = TextStyle::new(14.0, Color::WHITE).with_text_transform(TextTransform::Uppercase);
    assert_eq!(s.text_transform, TextTransform::Uppercase);
}

// Line height

#[test]
fn text_style_with_line_height_sets_override() {
    let s = TextStyle::new(13.0, Color::WHITE).with_line_height(1.5);
    assert_eq!(s.line_height, Some(1.5));
}

#[test]
fn text_style_with_line_height_builder_chain() {
    let s = TextStyle::new(13.0, Color::WHITE)
        .with_weight(FontWeight::BOLD)
        .with_line_height(1.5)
        .with_letter_spacing(2.0);
    assert_eq!(s.weight, FontWeight::BOLD);
    assert_eq!(s.line_height, Some(1.5));
    assert_eq!(s.letter_spacing, 2.0);
}

#[test]
fn normalized_line_height_valid_values() {
    for m in [0.5, 1.0, 1.5, 2.0] {
        let s = TextStyle::new(13.0, Color::WHITE).with_line_height(m);
        assert_eq!(s.normalized_line_height(), Some(m), "multiplier {m}");
    }
}

#[test]
fn normalized_line_height_filters_zero() {
    let mut s = TextStyle::new(13.0, Color::WHITE);
    s.line_height = Some(0.0);
    assert_eq!(s.normalized_line_height(), None);
}

#[test]
fn normalized_line_height_filters_negative() {
    let mut s = TextStyle::new(13.0, Color::WHITE);
    s.line_height = Some(-1.0);
    assert_eq!(s.normalized_line_height(), None);
    s.line_height = Some(-0.001);
    assert_eq!(s.normalized_line_height(), None);
}

#[test]
fn normalized_line_height_filters_nan() {
    let mut s = TextStyle::new(13.0, Color::WHITE);
    s.line_height = Some(f32::NAN);
    assert_eq!(s.normalized_line_height(), None);
}

#[test]
fn normalized_line_height_filters_infinity() {
    let mut s = TextStyle::new(13.0, Color::WHITE);
    s.line_height = Some(f32::INFINITY);
    assert_eq!(s.normalized_line_height(), None);
    s.line_height = Some(f32::NEG_INFINITY);
    assert_eq!(s.normalized_line_height(), None);
}

#[test]
fn normalized_line_height_none_returns_none() {
    let s = TextStyle::new(13.0, Color::WHITE);
    assert_eq!(s.normalized_line_height(), None);
}

#[test]
fn text_style_debug_includes_line_height() {
    let s = TextStyle::new(13.0, Color::WHITE).with_line_height(1.5);
    let dbg = format!("{s:?}");
    assert!(dbg.contains("line_height"), "Debug output: {dbg}");
}

#[test]
fn text_style_partial_eq_distinguishes_line_height() {
    let a = TextStyle::new(13.0, Color::WHITE).with_line_height(1.5);
    let b = TextStyle::new(13.0, Color::WHITE);
    assert_ne!(a, b);
}
