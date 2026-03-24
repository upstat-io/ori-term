//! Tests for `CachedTextMeasurer` and `TextShapeCache`.

use oriterm_ui::text::{
    FontWeight, ShapedText, TextMetrics, TextOverflow, TextStyle, TextTransform,
};
use oriterm_ui::widgets::TextMeasurer;

use super::{CachedTextMeasurer, TextCacheKey, TextShapeCache, float_to_hundredths};

/// Dummy measurer for testing `CachedTextMeasurer` without font infrastructure.
struct DummyMeasurer;

impl TextMeasurer for DummyMeasurer {
    fn measure(&self, text: &str, _style: &TextStyle, _max_width: f32) -> TextMetrics {
        TextMetrics {
            width: text.len() as f32 * 8.0,
            height: 16.0,
            line_count: 1,
        }
    }

    fn shape(&self, text: &str, _style: &TextStyle, _max_width: f32) -> ShapedText {
        ShapedText::new(vec![], text.len() as f32 * 8.0, 16.0, 12.0, 0, 400)
    }
}

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
    let s2 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE).with_weight(FontWeight::BOLD);
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

// -- CacheStats tests --

#[test]
fn stats_start_at_zero() {
    let cache = TextShapeCache::new();
    let s = cache.stats();
    assert_eq!(s.measure_hits, 0);
    assert_eq!(s.measure_misses, 0);
    assert_eq!(s.shape_hits, 0);
    assert_eq!(s.shape_misses, 0);
}

#[test]
fn stats_hit_rate_vacuously_true_when_empty() {
    let cache = TextShapeCache::new();
    assert_eq!(cache.stats().hit_rate(), 1.0);
}

#[test]
fn stats_reset_clears_all_counters() {
    let cache = TextShapeCache::new();
    let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
    let style = TextStyle::default();
    // Generate some misses.
    measurer.measure("a", &style, f32::INFINITY);
    measurer.shape("b", &style, f32::INFINITY);
    assert!(cache.stats().total_misses() > 0);

    cache.reset_stats();
    let s = cache.stats();
    assert_eq!(s.measure_hits, 0);
    assert_eq!(s.measure_misses, 0);
    assert_eq!(s.shape_hits, 0);
    assert_eq!(s.shape_misses, 0);
}

#[test]
fn measure_miss_then_hit() {
    let cache = TextShapeCache::new();
    let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
    let style = TextStyle::default();

    // First call: miss.
    measurer.measure("hello", &style, f32::INFINITY);
    assert_eq!(cache.stats().measure_misses, 1);
    assert_eq!(cache.stats().measure_hits, 0);

    // Second call with same args: hit.
    measurer.measure("hello", &style, f32::INFINITY);
    assert_eq!(cache.stats().measure_misses, 1);
    assert_eq!(cache.stats().measure_hits, 1);
}

#[test]
fn shape_miss_then_hit() {
    let cache = TextShapeCache::new();
    let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
    let style = TextStyle::default();

    // First call: miss.
    measurer.shape("hello", &style, 200.0);
    assert_eq!(cache.stats().shape_misses, 1);
    assert_eq!(cache.stats().shape_hits, 0);

    // Second call with same args: hit.
    measurer.shape("hello", &style, 200.0);
    assert_eq!(cache.stats().shape_misses, 1);
    assert_eq!(cache.stats().shape_hits, 1);
}

#[test]
fn different_text_produces_separate_misses() {
    let cache = TextShapeCache::new();
    let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
    let style = TextStyle::default();

    measurer.measure("aaa", &style, f32::INFINITY);
    measurer.measure("bbb", &style, f32::INFINITY);
    assert_eq!(cache.stats().measure_misses, 2);
    assert_eq!(cache.stats().measure_hits, 0);
}

#[test]
fn hit_rate_computes_correctly() {
    let cache = TextShapeCache::new();
    let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
    let style = TextStyle::default();

    // 2 unique measure calls → 2 misses.
    measurer.measure("a", &style, f32::INFINITY);
    measurer.measure("b", &style, f32::INFINITY);
    // 2 repeat calls → 2 hits.
    measurer.measure("a", &style, f32::INFINITY);
    measurer.measure("b", &style, f32::INFINITY);
    // Total: 2 hits + 2 misses = 50% hit rate.
    assert_eq!(cache.stats().hit_rate(), 0.5);
}

#[test]
fn warmup_then_steady_state_100_percent_hits() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();
    let strings = ["General", "Appearance", "Font Size: 14", "Apply", "Cancel"];

    // Frame 1 (warmup): all misses.
    {
        let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        for s in &strings {
            measurer.measure(s, &style, f32::INFINITY);
            measurer.shape(s, &style, 400.0);
        }
    }
    let warmup = cache.stats();
    assert_eq!(warmup.measure_misses, 5);
    assert_eq!(warmup.shape_misses, 5);
    assert_eq!(warmup.measure_hits, 0);
    assert_eq!(warmup.shape_hits, 0);

    // Reset stats for steady-state measurement.
    cache.reset_stats();

    // Frame 2 (steady state): same strings, all hits.
    {
        let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        for s in &strings {
            measurer.measure(s, &style, f32::INFINITY);
            measurer.shape(s, &style, 400.0);
        }
    }
    let steady = cache.stats();
    assert_eq!(steady.measure_hits, 5);
    assert_eq!(steady.shape_hits, 5);
    assert_eq!(steady.measure_misses, 0);
    assert_eq!(steady.shape_misses, 0);
    assert_eq!(steady.hit_rate(), 1.0);
}

#[test]
fn new_measurer_per_frame_preserves_cache() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();

    // Frame 1: populate via first measurer.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        m.measure("persist", &style, f32::INFINITY);
    }
    // Measurer dropped, but TextShapeCache persists.

    cache.reset_stats();

    // Frame 2: new measurer, same cache → hit.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        m.measure("persist", &style, f32::INFINITY);
    }
    assert_eq!(cache.stats().measure_hits, 1);
    assert_eq!(cache.stats().measure_misses, 0);
}

#[test]
fn measure_returns_cached_value_not_recomputed() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();
    let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);

    let first = measurer.measure("test", &style, f32::INFINITY);
    let second = measurer.measure("test", &style, f32::INFINITY);
    assert_eq!(first, second, "cached value must match original");
}

#[test]
fn shape_returns_cached_value_not_recomputed() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();
    let measurer = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);

    let first = measurer.shape("test", &style, 200.0);
    let second = measurer.shape("test", &style, 200.0);
    assert_eq!(first.width, second.width, "cached width must match");
    assert_eq!(first.height, second.height, "cached height must match");
    assert_eq!(
        first.glyphs.len(),
        second.glyphs.len(),
        "cached glyphs must match",
    );
}

// -- Memory: bounded cache size --

#[test]
fn text_cache_stabilizes_after_warmup() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();
    let strings: Vec<String> = (0..20).map(|i| format!("Label {i}")).collect();

    // Frame 1: warmup — all misses populate cache.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        for s in &strings {
            m.measure(s, &style, f32::INFINITY);
            m.shape(s, &style, 400.0);
        }
    }
    let size_after_warmup = cache.metrics.borrow().len() + cache.shapes.borrow().len();

    // Frames 2–100: same strings, all hits. Size must not grow.
    for _ in 2..=100 {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        for s in &strings {
            m.measure(s, &style, f32::INFINITY);
            m.shape(s, &style, 400.0);
        }
    }
    let size_after_100 = cache.metrics.borrow().len() + cache.shapes.borrow().len();
    assert_eq!(
        size_after_warmup, size_after_100,
        "cache size must stabilize after warmup",
    );
}

// -- Cache eviction at capacity --

#[test]
fn eviction_at_capacity_no_panic() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();

    // Fill metrics cache to capacity.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        for i in 0..super::MAX_CACHE_ENTRIES {
            m.measure(&format!("entry-{i}"), &style, f32::INFINITY);
        }
    }
    assert_eq!(cache.metrics.borrow().len(), super::MAX_CACHE_ENTRIES);

    // One more entry triggers eviction (clear-all + insert new).
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        m.measure("overflow", &style, f32::INFINITY);
    }
    // After eviction: only the new entry remains.
    assert_eq!(cache.metrics.borrow().len(), 1);
}

#[test]
fn eviction_returns_correct_value_not_stale() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();

    // Fill the shapes cache to capacity with unique entries.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        for i in 0..super::MAX_CACHE_ENTRIES {
            m.shape(&format!("entry-{i}"), &style, 200.0);
        }
    }

    // Insert one more — triggers eviction.
    let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
    let result = m.shape("new-entry", &style, 200.0);
    assert_eq!(
        result.width,
        DummyMeasurer.shape("new-entry", &style, 200.0).width,
        "post-eviction value must match fresh computation",
    );

    // Re-query the new entry — should be a cache hit.
    cache.reset_stats();
    let _ = m.shape("new-entry", &style, 200.0);
    assert_eq!(cache.stats().shape_hits, 1);
    assert_eq!(cache.stats().shape_misses, 0);
}

// -- Invalidation triggers --

#[test]
fn font_generation_change_clears_and_reprobes() {
    let mut cache = TextShapeCache::new();
    let style = TextStyle::default();

    // Populate cache.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        m.measure("cached", &style, f32::INFINITY);
        m.shape("cached", &style, 200.0);
    }
    assert!(!cache.metrics.borrow().is_empty());
    assert!(!cache.shapes.borrow().is_empty());

    // Simulate font reload: generation changes.
    cache.invalidate_if_stale(1);
    assert!(
        cache.metrics.borrow().is_empty(),
        "metrics cleared on font reload"
    );
    assert!(
        cache.shapes.borrow().is_empty(),
        "shapes cleared on font reload"
    );

    // Re-render: all misses, cache repopulates correctly.
    cache.reset_stats();
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        let result = m.measure("cached", &style, f32::INFINITY);
        assert_eq!(result.width, 48.0, "correct value after font reload");
    }
    assert_eq!(cache.stats().measure_misses, 1);
    assert_eq!(cache.stats().measure_hits, 0);
}

#[test]
fn different_text_transform_different_key() {
    let s1 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE);
    let s2 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE)
        .with_text_transform(TextTransform::Uppercase);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_ne!(
        k1, k2,
        "different text_transform must produce different keys"
    );
}

#[test]
fn same_text_transform_same_key() {
    let s1 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE)
        .with_text_transform(TextTransform::Uppercase);
    let s2 = TextStyle::new(12.0, oriterm_ui::color::Color::WHITE)
        .with_text_transform(TextTransform::Uppercase);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_eq!(k1, k2);
}

#[test]
fn dpi_change_produces_different_cache_keys() {
    let cache = TextShapeCache::new();
    let style = TextStyle::default();

    // Scale 1.0.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        m.measure("text", &style, f32::INFINITY);
    }
    cache.reset_stats();

    // Scale 2.0 — different cache key, should miss.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 2.0);
        m.measure("text", &style, f32::INFINITY);
    }
    assert_eq!(
        cache.stats().measure_misses,
        1,
        "different scale should produce a cache miss",
    );
    assert_eq!(cache.stats().measure_hits, 0);
}

// Line height cache key tests

#[test]
fn cache_key_changes_when_line_height_differs() {
    let s1 = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);
    let s2 = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_ne!(k1, k2, "different line_height must produce different keys");
}

#[test]
fn cache_key_same_when_line_height_same() {
    let s1 = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);
    let s2 = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);
    let k1 = TextCacheKey::new("hello", &s1, f32::INFINITY, 1.0);
    let k2 = TextCacheKey::new("hello", &s2, f32::INFINITY, 1.0);
    assert_eq!(k1, k2);
}

#[test]
fn cache_key_invalid_line_height_same_as_none() {
    let base = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let k_none = TextCacheKey::new("hello", &base, f32::INFINITY, 1.0);

    for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut s = base.clone();
        s.line_height = Some(invalid);
        let k = TextCacheKey::new("hello", &s, f32::INFINITY, 1.0);
        assert_eq!(
            k, k_none,
            "invalid line_height {invalid} should produce same key as None"
        );
    }
}

#[test]
fn cache_miss_when_line_height_changes() {
    let cache = TextShapeCache::new();
    let s1 = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE);
    let s2 = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);

    let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
    m.measure("text", &s1, f32::INFINITY);
    cache.reset_stats();
    m.measure("text", &s2, f32::INFINITY);
    assert_eq!(
        cache.stats().measure_misses,
        1,
        "different line_height must miss"
    );
}

#[test]
fn cache_hit_same_line_height_across_frames() {
    let cache = TextShapeCache::new();
    let style = TextStyle::new(13.0, oriterm_ui::color::Color::WHITE).with_line_height(1.5);

    // Frame 1: populate.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        m.measure("text", &style, f32::INFINITY);
    }
    cache.reset_stats();

    // Frame 2: new measurer, same cache.
    {
        let m = CachedTextMeasurer::new(DummyMeasurer, &cache, 1.0);
        m.measure("text", &style, f32::INFINITY);
    }
    assert_eq!(cache.stats().measure_hits, 1);
    assert_eq!(cache.stats().measure_misses, 0);
}
