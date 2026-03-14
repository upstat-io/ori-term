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

use std::cell::RefCell;
use std::collections::HashMap;

use oriterm_ui::text::{FontWeight, ShapedText, TextMetrics, TextOverflow, TextStyle};
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
}

impl TextShapeCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            metrics: RefCell::new(HashMap::new()),
            shapes: RefCell::new(HashMap::new()),
            generation: 0,
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
}

impl Default for TextShapeCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-frame text measurer that wraps [`UiFontMeasurer`] and interposes a cache.
///
/// The cache maps live on the context struct ([`TextShapeCache`]) and are
/// borrowed here. The measurer is constructed per-frame; the cache persists.
pub struct CachedTextMeasurer<'a> {
    inner: UiFontMeasurer<'a>,
    cache: &'a TextShapeCache,
    scale: f32,
}

impl<'a> CachedTextMeasurer<'a> {
    /// Wrap a font measurer with caching.
    ///
    /// `scale` must match the scale passed to `UiFontMeasurer::new()` — it's
    /// included in cache keys so different DPI windows don't share entries.
    pub fn new(inner: UiFontMeasurer<'a>, cache: &'a TextShapeCache, scale: f32) -> Self {
        Self {
            inner,
            cache,
            scale,
        }
    }
}

impl TextMeasurer for CachedTextMeasurer<'_> {
    fn measure(&self, text: &str, style: &TextStyle, max_width: f32) -> TextMetrics {
        let key = TextCacheKey::for_measure(text, style, self.scale);

        // Check cache.
        if let Some(cached) = self.cache.metrics.borrow().get(&key) {
            return *cached;
        }

        // Cache miss — delegate to inner measurer.
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
            return cached.clone();
        }

        // Cache miss — delegate to inner measurer.
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
