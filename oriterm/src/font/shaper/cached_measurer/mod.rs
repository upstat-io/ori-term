//! Caching wrapper for [`UiFontMeasurer`] that eliminates redundant text shaping.
//!
//! Widgets call `measure()` and `shape()` on every layout and draw pass. Without
//! caching, each call performs full rustybuzz shaping — expensive for a Settings
//! dialog with 140+ text strings that don't change between frames. This module
//! provides [`CachedTextMeasurer`] as a drop-in replacement: same `TextMeasurer`
//! trait, but interposes a `HashMap` cache keyed on text + style + constraints.
//!
//! The cache maps live in [`TextShapeCache`], which is stored on context structs
//! (`WindowContext`, `DialogWindowContext`) so entries persist across frames.
//! The measurer borrows `&TextShapeCache` per-frame; `RefCell` provides interior
//! mutability for cache insertion from `&self` trait methods.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use oriterm_ui::text::{
    FontWeight, ShapedText, TextMetrics, TextOverflow, TextStyle, TextTransform,
};
use oriterm_ui::widgets::TextMeasurer;

use super::ui_measurer::UiFontMeasurer;

/// Maximum entries per cache map before eviction.
const MAX_CACHE_ENTRIES: usize = 1024;

/// Cache key capturing all parameters that affect shaping output.
///
/// `color` and `align` are excluded — they don't affect glyph selection or
/// metrics, only rendering and positioning.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextCacheKey {
    text: String,
    font_family: Option<String>,
    /// `f32` size normalized to fixed-point: `(size * 100.0) as u32`.
    size_hundredths: u32,
    weight: FontWeight,
    overflow: TextOverflow,
    /// `f32` `max_width` normalized to fixed-point. `f32::INFINITY` → `u32::MAX`.
    max_width_hundredths: u32,
    /// Display scale factor normalized to fixed-point.
    scale_hundredths: u32,
    /// Letter spacing normalized to fixed-point.
    letter_spacing_hundredths: u32,
    /// Case transformation — affects shaped output.
    text_transform: TextTransform,
    /// Line-height multiplier (normalized: invalid values map to `None`).
    line_height_hundredths: Option<u32>,
}

impl TextCacheKey {
    /// Build a cache key, normalizing floats to fixed-point integers.
    fn new(text: &str, style: &TextStyle, max_width: f32, scale: f32) -> Self {
        Self {
            text: text.to_owned(),
            font_family: style.font_family.clone(),
            size_hundredths: float_to_hundredths(style.size),
            weight: style.weight,
            overflow: style.overflow,
            max_width_hundredths: float_to_hundredths(max_width),
            scale_hundredths: float_to_hundredths(scale),
            letter_spacing_hundredths: float_to_hundredths(style.letter_spacing),
            text_transform: style.text_transform,
            line_height_hundredths: style.normalized_line_height().map(float_to_hundredths),
        }
    }

    /// Build a key for `measure()` calls, which ignore `max_width`.
    ///
    /// Uses `u32::MAX` as a sentinel so all `measure()` calls for the same
    /// text+style share one cache entry regardless of the caller's `max_width`.
    fn for_measure(text: &str, style: &TextStyle, scale: f32) -> Self {
        Self {
            text: text.to_owned(),
            font_family: style.font_family.clone(),
            size_hundredths: float_to_hundredths(style.size),
            weight: style.weight,
            overflow: style.overflow,
            max_width_hundredths: u32::MAX,
            scale_hundredths: float_to_hundredths(scale),
            letter_spacing_hundredths: float_to_hundredths(style.letter_spacing),
            text_transform: style.text_transform,
            line_height_hundredths: style.normalized_line_height().map(float_to_hundredths),
        }
    }
}

/// Convert `f32` to fixed-point hundredths. `INFINITY` maps to `u32::MAX`.
fn float_to_hundredths(v: f32) -> u32 {
    if v.is_infinite() || v.is_nan() {
        u32::MAX
    } else {
        (v * 100.0) as u32
    }
}

/// Snapshot of cache hit/miss counters.
///
/// Counters accumulate across frames until [`TextShapeCache::reset_stats`] is
/// called. Use [`CacheStats::hit_rate`] to compute the overall hit percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "instrumentation API for performance validation tests"
)]
pub struct CacheStats {
    /// Number of `measure()` calls that returned a cached result.
    pub measure_hits: usize,
    /// Number of `measure()` calls that performed full computation.
    pub measure_misses: usize,
    /// Number of `shape()` calls that returned a cached result.
    pub shape_hits: usize,
    /// Number of `shape()` calls that performed full computation.
    pub shape_misses: usize,
}

#[allow(
    dead_code,
    reason = "instrumentation API for performance validation tests"
)]
impl CacheStats {
    /// Total hits across both measure and shape caches.
    pub fn total_hits(&self) -> usize {
        self.measure_hits + self.shape_hits
    }

    /// Total misses across both measure and shape caches.
    pub fn total_misses(&self) -> usize {
        self.measure_misses + self.shape_misses
    }

    /// Overall hit rate as a fraction in `[0.0, 1.0]`.
    ///
    /// Returns `1.0` if no calls have been made (vacuously true).
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits() + self.total_misses();
        if total == 0 {
            return 1.0;
        }
        self.total_hits() as f64 / total as f64
    }
}

/// Persistent cache maps for text shaping results.
///
/// Stored on `WindowContext` and `DialogWindowContext` so entries survive across
/// frames. Interior mutability via `RefCell` allows `CachedTextMeasurer` (which
/// holds `&TextShapeCache`) to insert entries from `&self` trait methods.
///
/// Thread safety: UI rendering is single-threaded, so `RefCell` is appropriate.
pub struct TextShapeCache {
    metrics: RefCell<HashMap<TextCacheKey, TextMetrics>>,
    shapes: RefCell<HashMap<TextCacheKey, ShapedText>>,
    /// Font generation counter — clear cache when fonts change.
    #[allow(
        dead_code,
        reason = "used by tests; provisioned for font-reload invalidation"
    )]
    generation: u64,
    // Hit/miss counters — `Cell` for interior mutability from `&self`.
    measure_hits: Cell<usize>,
    measure_misses: Cell<usize>,
    shape_hits: Cell<usize>,
    shape_misses: Cell<usize>,
}

impl TextShapeCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            metrics: RefCell::new(HashMap::new()),
            shapes: RefCell::new(HashMap::new()),
            generation: 0,
            measure_hits: Cell::new(0),
            measure_misses: Cell::new(0),
            shape_hits: Cell::new(0),
            shape_misses: Cell::new(0),
        }
    }

    /// Clear all cached entries. Called on font reload, theme change, or DPI change.
    #[allow(
        dead_code,
        reason = "used by tests; provisioned for font-reload invalidation"
    )]
    pub fn clear(&mut self) {
        self.metrics.get_mut().clear();
        self.shapes.get_mut().clear();
    }

    /// Clear cache if the font generation has changed.
    ///
    /// Call this before constructing a `CachedTextMeasurer` each frame.
    #[allow(
        dead_code,
        reason = "used by tests; provisioned for font-reload invalidation"
    )]
    pub fn invalidate_if_stale(&mut self, current_gen: u64) {
        if self.generation != current_gen {
            self.clear();
            self.generation = current_gen;
        }
    }

    /// Snapshot of accumulated hit/miss counters.
    #[allow(
        dead_code,
        reason = "instrumentation API for performance validation tests"
    )]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            measure_hits: self.measure_hits.get(),
            measure_misses: self.measure_misses.get(),
            shape_hits: self.shape_hits.get(),
            shape_misses: self.shape_misses.get(),
        }
    }

    /// Reset all hit/miss counters to zero.
    #[allow(
        dead_code,
        reason = "instrumentation API for performance validation tests"
    )]
    pub fn reset_stats(&self) {
        self.measure_hits.set(0);
        self.measure_misses.set(0);
        self.shape_hits.set(0);
        self.shape_misses.set(0);
    }
}

impl Default for TextShapeCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-frame text measurer that wraps a [`TextMeasurer`] and interposes a cache.
///
/// The cache maps live on the context struct ([`TextShapeCache`]) and are
/// borrowed here. The measurer is constructed per-frame; the cache persists.
///
/// Generic over the inner measurer `M` to support testing with mocks.
/// Production code uses the default `M = UiFontMeasurer<'a>`.
pub struct CachedTextMeasurer<'a, M: TextMeasurer = UiFontMeasurer<'a>> {
    inner: M,
    cache: &'a TextShapeCache,
    scale: f32,
}

impl<'a, M: TextMeasurer> CachedTextMeasurer<'a, M> {
    /// Wrap a font measurer with caching.
    ///
    /// `scale` must match the scale passed to the inner measurer — it's
    /// included in cache keys so different DPI windows don't share entries.
    pub fn new(inner: M, cache: &'a TextShapeCache, scale: f32) -> Self {
        Self {
            inner,
            cache,
            scale,
        }
    }
}

impl<M: TextMeasurer> TextMeasurer for CachedTextMeasurer<'_, M> {
    fn measure(&self, text: &str, style: &TextStyle, max_width: f32) -> TextMetrics {
        let key = TextCacheKey::for_measure(text, style, self.scale);

        // Check cache.
        if let Some(cached) = self.cache.metrics.borrow().get(&key) {
            self.cache
                .measure_hits
                .set(self.cache.measure_hits.get() + 1);
            return *cached;
        }

        // Cache miss — delegate to inner measurer.
        self.cache
            .measure_misses
            .set(self.cache.measure_misses.get() + 1);
        let result = self.inner.measure(text, style, max_width);

        // Store in cache (evict all if full).
        let mut map = self.cache.metrics.borrow_mut();
        if map.len() >= MAX_CACHE_ENTRIES {
            map.clear();
        }
        map.insert(key, result);

        result
    }

    fn shape(&self, text: &str, style: &TextStyle, max_width: f32) -> ShapedText {
        let key = TextCacheKey::new(text, style, max_width, self.scale);

        // Check cache.
        if let Some(cached) = self.cache.shapes.borrow().get(&key) {
            self.cache.shape_hits.set(self.cache.shape_hits.get() + 1);
            return cached.clone();
        }

        // Cache miss — delegate to inner measurer.
        self.cache
            .shape_misses
            .set(self.cache.shape_misses.get() + 1);
        let result = self.inner.shape(text, style, max_width);

        // Store in cache (evict all if full).
        let mut map = self.cache.shapes.borrow_mut();
        if map.len() >= MAX_CACHE_ENTRIES {
            map.clear();
        }
        map.insert(key, result.clone());

        result
    }
}

#[cfg(test)]
mod tests;
