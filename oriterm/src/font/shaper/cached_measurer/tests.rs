//! Tests for `CachedTextMeasurer` and `TextShapeCache`.

use oriterm_ui::text::{FontWeight, TextMetrics, TextOverflow, TextStyle};

use super::{TextCacheKey, TextShapeCache, float_to_hundredths};

#[test]
fn float_to_hundredths_exact_integers() {
    assert_eq!(float_to_hundredths(11.0), 1100);
    assert_eq!(float_to_hundredths(0.0), 0);
    assert_eq!(float_to_hundredths(1.5), 150);
}

#[test]
fn float_to_hundredths_infinity_maps_to_max() {
    assert_eq!(float_to_hundredths(f32::INFINITY), u32::MAX);
    assert_eq!(float_to_hundredths(f32::NEG_INFINITY), u32::MAX);
    assert_eq!(float_to_hundredths(f32::NAN), u32::MAX);
}

#[test]
fn same_text_same_style_same_key() {
    let style = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let k1 = TextCacheKey::new("hello", &style, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &style, f32::INFINITY, 1.0);
    assert_eq!(k1, k2);
}

#[test]
fn different_size_different_key() {
    let s1 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let s2 = TextStyle::new(14.0, oriterm_ui::color::Color::WHITE);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_ne!(k1, k2);
}

#[test]
fn different_weight_different_key() {
    let s1 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let s2 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE).with_weight(FontWeight::Bold);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_ne!(k1, k2);
}

#[test]
fn different_color_same_key() {
    let s1 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let s2 = TextStyle::new(12.0, oriterm_ui::color::Color::BLACK);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_eq!(k1, k2);
}

#[test]
fn different_max_width_different_shape_key() {
    let style = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let k1 = TextCacheKey::new("hello", &style, 200.0, 1.0);
    let k2 = TextCacheKey::new("hello", &style, 300.0, 1.0);
    assert_ne!(k1, k2);
}

#[test]
fn measure_key_ignores_max_width() {
    let style = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let k1 = TextCacheKey::for_measure("hello", &style, 1.0);
    let k2 = TextCacheKey::for_measure("hello", &style, 1.0);
    assert_eq!(k1, k2);
    // Measure key uses u32::MAX sentinel.
    assert_eq!(k1.max_width_hundredths, u32::MAX);
}

#[test]
fn different_overflow_different_key() {
    let s1 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let s2 =
        TextStyle::new(12.0, oriterm_ui::color::Color::WHITE).with_overflow(TextOverflow::Ellipsis);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_ne!(k1, k2);
}

#[test]
fn cache_clear_empties_both_maps() {
    let mut cache = TextShapeCache::new();
    let key = TextCacheKey::for_measure("test", &TextStyle::default(), 1.0);
    cache.metrics.borrow_mut().insert(
        key,
        TextMetrics {
            width: 10.0,
            height: 12.0,
            line_count: 1,
        },
    );
    assert_eq!(cache.metrics.borrow().len(), 1);

    cache.clear();
    assert!(cache.metrics.borrow().is_empty());
    assert!(cache.shapes.borrow().is_empty());
}

#[test]
fn invalidate_if_stale_clears_on_generation_change() {
    let mut cache = TextShapeCache::new();
    let key = TextCacheKey::for_measure("test", &TextStyle::default(), 1.0);
    cache.metrics.borrow_mut().insert(
        key,
        TextMetrics {
            width: 10.0,
            height: 12.0,
            line_count: 1,
        },
    );
    assert_eq!(cache.metrics.borrow().len(), 1);

    // Same generation — no clear.
    cache.invalidate_if_stale(0);
    assert_eq!(cache.metrics.borrow().len(), 1);

    // New generation — clear.
    cache.invalidate_if_stale(1);
    assert!(cache.metrics.borrow().is_empty());
    assert_eq!(cache.generation, 1);
}

#[test]
fn empty_string_produces_valid_key() {
    let style = TextStyle::default();
    let key = TextCacheKey::new("", &style, f32::INFINITY, 1.0);
    assert!(key.text.is_empty());
}
