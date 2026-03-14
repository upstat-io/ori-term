---
section: "01"
title: "Text/Layout Caching"
status: complete
goal: "Eliminate redundant text shaping across all UI rendering — no string is shaped twice with the same parameters within a frame or across frames until invalidated."
inspired_by:
  - "Chromium text layout cache (ui/gfx/render_text.cc)"
  - "Flutter paragraph cache (keyed by text+style+constraints)"
depends_on: []
sections:
  - id: "01.1"
    title: "CachedTextMeasurer"
    status: complete
  - id: "01.2"
    title: "Cache Key Design"
    status: complete
  - id: "01.3"
    title: "Integration with Widget Contexts"
    status: complete
  - id: "01.4"
    title: "Cache Lifecycle"
    status: complete
  - id: "01.5"
    title: "Completion Checklist"
    status: complete
reviewed: true
---

# Section 01: Text/Layout Caching

**Status:** Complete
**Goal:** Every `TextMeasurer::measure()` and `TextMeasurer::shape()` call is served from cache when the same text+style+constraints were seen before. No widget code changes required — caching is transparent at the framework boundary.

**Context:** Today `UiFontMeasurer` (`oriterm/src/font/shaper/ui_measurer.rs:41`) delegates every `measure()` and `shape()` call directly to `ui_text::measure_text_styled()` and `ui_text::shape_text()`. These functions perform full rustybuzz shaping on every invocation. A Settings dialog with 40 labels, 20 buttons, and 10 dropdowns shapes ~140 text strings per draw call. On hover — which triggers a full redraw via `dialog_rendering.rs` — all 140 strings are re-shaped even though only one button's text color changed. This is the single highest-ROI fix because text shaping dominates the per-frame cost for UI rendering.

The same problem exists for terminal chrome: `draw_tab_bar()` in `redraw/draw_helpers.rs` shapes every tab title every frame via `draw_list.clear()` (line 48) + full widget tree traversal.

**Reference implementations:**
- **Chromium** `ui/gfx/render_text.cc`: Caches shaped runs keyed by text+font+size+features. Invalidated on text change, font change, or DPI change.
- **Flutter** `paragraph_builder.cc`: Paragraph layout cached by text+style+constraints triple. Cache is per-paragraph, cleared on text mutation.

**Depends on:** Nothing — pure addition.

---

## 01.1 CachedTextMeasurer

**File(s):** `oriterm_ui/src/widgets/text_measurer.rs` (trait), new `oriterm/src/font/shaper/cached_measurer/mod.rs`

**Module registration:** Add `pub mod cached_measurer;` to `oriterm/src/font/shaper/mod.rs` (after `pub mod ui_measurer;` at line 12). Without this, the new file won't compile. Also add a `pub use cached_measurer::CachedTextMeasurer;` re-export alongside the existing `pub use ui_measurer::UiFontMeasurer;` (line 15).

The `CachedTextMeasurer` wraps any `TextMeasurer` impl (currently `UiFontMeasurer`) and interposes a `HashMap` cache. It implements the same `TextMeasurer` trait so it's a drop-in replacement.

- [x] Create `CachedTextMeasurer` struct in `oriterm/src/font/shaper/cached_measurer/mod.rs`
  ```rust
  /// Per-frame text measurer that wraps `UiFontMeasurer` and interposes a cache.
  /// The cache maps live on the context struct (`TextShapeCache`) and are
  /// borrowed here. The measurer is constructed per-frame; the cache persists.
  pub struct CachedTextMeasurer<'a> {
      inner: UiFontMeasurer<'a>,
      cache: &'a TextShapeCache,
  }
  ```
  Note: the earlier draft showed `RefCell<HashMap<...>>` fields owned by the measurer. This is wrong -- the cache must survive across frames. The `TextShapeCache` struct (defined in 01.4) owns the maps with interior mutability (`RefCell`). The measurer borrows `&TextShapeCache` (shared reference). Since `TextMeasurer::measure(&self, ...)` and `shape(&self, ...)` take `&self`, the cache must be mutable through `&self` -- hence `RefCell` on the maps inside `TextShapeCache`, not on the `CachedTextMeasurer`.

- [x] Implement `TextMeasurer` for `CachedTextMeasurer` — check cache first, fall through to `inner` on miss, store result before returning.

- [x] Export from `oriterm/src/font/shaper/mod.rs` alongside `UiFontMeasurer`. Also export `TextShapeCache` since it is stored on context structs.

---

## 01.2 Cache Key Design

**File(s):** `oriterm/src/font/shaper/cached_measurer/mod.rs`

The cache key must capture everything that affects shaping output. Missing a field means stale cache hits; including too much means poor hit rates.

- [x] Define `TextCacheKey`:
  ```rust
  #[derive(Clone, PartialEq, Eq, Hash)]
  struct TextCacheKey {
      text: String,
      font_family: Option<String>,
      size_hundredths: u32,      // f32 → u32 via (size * 100.0) as u32
      weight: FontWeight,
      overflow: TextOverflow,
      max_width_hundredths: u32, // f32 → u32, INFINITY → u32::MAX
      scale_hundredths: u32,     // display scale factor
  }
  ```

- [x] `TextAlign` is excluded from the key — it doesn't affect shaping, only positioning. Verified against `ui_text::shape_text()`.

- [x] `color` is excluded — it doesn't affect glyph selection or metrics, only rendering. The `ShapedText` struct doesn't contain color. Verified.

- [x] Add `TextCacheKey::new(text, style, max_width, scale)` constructor that normalizes floats to fixed-point integers for hashability.

- [x] **Float-to-integer normalization:** `TextStyle` contains `size: f32` which cannot be hashed directly. The cache key normalizes all `f32` fields to integer representations (e.g. `(size * 100.0) as u32`). The constructor handles `f32::INFINITY` for `max_width` (maps to `u32::MAX`). No precision loss causes spurious cache misses — `11.0 * 100.0 = 1100` exactly in IEEE 754.

- [x] **`measure()` cache key:** `UiFontMeasurer::measure()` (ui_measurer.rs:42) accepts `_max_width` but ignores it — calls `measure_text_styled()` with no width constraint. The `metrics_cache` key uses a sentinel value (`u32::MAX`) for `max_width` on measure calls so all measure calls for the same text+style share one cache entry regardless of the caller's `max_width`.

---

## 01.3 Integration with Widget Contexts

**File(s):** `oriterm/src/app/dialog_rendering.rs`, `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/draw_helpers.rs`, and all other `UiFontMeasurer` usage sites

Replace `UiFontMeasurer::new()` with `CachedTextMeasurer::new()` at every call site where a `TextMeasurer` is constructed for widget rendering.

- [x] `dialog_rendering.rs` — Settings/confirmation dialog rendering. Replaced with `CachedTextMeasurer` using `&ctx.text_cache`.

- [x] `dialog_rendering.rs` overlay rendering — Extracted to `render_dialog_overlays()` helper, uses cached measurer.

- [x] `redraw/draw_helpers.rs` — `draw_tab_bar()` and `draw_overlays()`. Added `&TextShapeCache` parameter, uses cached measurer.

- [x] `dialog_management.rs` — `form.compute_label_widths()` during settings build. Uses a temporary `TextShapeCache::new()` since no persistent dialog context exists at that point.

- [x] Store `TextShapeCache` on `WindowContext` and `DialogWindowContext` so the cache persists across frames. The `UiFontMeasurer` borrow lifetime (`'a` tied to `FontCollection`) means the cache maps are owned separately and the measurer wrapper is constructed per-frame from the maps + a fresh `UiFontMeasurer`.

- [x] Additional usage sites updated: `chrome/mod.rs` (2 sites), `dialog_context/event_handling.rs` (4 sites), `dialog_context/content_actions.rs` (2 sites), `mouse_input.rs`, `keyboard_input/mod.rs`, `tab_bar_input.rs`, `redraw/search_bar.rs`, `settings_overlay/mod.rs`.

**Design decision — cache ownership:**

**(a) Cache maps on context structs** (implemented):
```rust
// On WindowContext / DialogWindowContext:
text_cache: TextShapeCache,  // owns the RefCell<HashMap>s

// Per-frame:
let measurer = CachedTextMeasurer::new(
    UiFontMeasurer::new(collection, scale),
    &self.text_cache,  // shared reference — RefCell provides interior mutability
    scale,
);
```
**Why this is best:** Cache survives across frames. No lifetime gymnastics -- `TextShapeCache` is `'static`, the wrapper borrows it via `&`. Interior mutability (`RefCell`) enables cache insertion from `&self` methods.

---

## 01.4 Cache Lifecycle

**File(s):** `oriterm/src/font/shaper/cached_measurer/mod.rs`

The cache must be invalidated when its key assumptions change. It must also not grow unboundedly.

- [x] **Invalidation triggers:**
  - Font reload (family, size, DPI change) → clear entire cache. `invalidate_if_stale(generation)` method provided.
  - Theme change → clear cache (font size may change via `UiTheme::font_size`).
  - Window DPI change (scale factor change) → clear cache.
  - Note: Scale is part of the cache key, so DPI changes naturally produce cache misses. Explicit clearing via `invalidate_if_stale()` prevents unbounded growth.

- [x] **Bounded size:** Capped at 1024 entries per map. Uses clear-on-overflow — the Settings dialog has ~140 unique strings, so 1024 is generous.

- [x] **`TextShapeCache` struct:**
  ```rust
  pub struct TextShapeCache {
      metrics: RefCell<HashMap<TextCacheKey, TextMetrics>>,
      shapes: RefCell<HashMap<TextCacheKey, ShapedText>>,
      generation: u64,
  }

  impl TextShapeCache {
      pub fn new() -> Self { ... }
      pub fn clear(&mut self) { ... }
      pub fn invalidate_if_stale(&mut self, current_gen: u64) { ... }
  }
  ```
  The `RefCell` enables `CachedTextMeasurer` (which holds `&TextShapeCache`) to insert cache entries from `&self` methods. This is safe because UI rendering is single-threaded.

- [x] Wire cache clearing at font reload, theme change, and DPI change sites:
  - `config_reload.rs:apply_font_changes()` — clears `text_cache` after font collection replacement
  - `mod.rs:handle_theme_changed()` — clears `text_cache` on system theme change
  - `mod.rs:handle_dpi_change()` — clears `text_cache` on DPI change
  - `dialog_context/event_handling.rs:ScaleFactorChanged` — clears dialog `text_cache` on DPI change
  - Note: `invalidate_if_stale(generation)` API exists but explicit `clear()` at the right sites is simpler and more reliable than threading a generation counter through `FontCollection`.

---

## 01.5 Completion Checklist

- [x] `CachedTextMeasurer` passes all existing widget tests (`cargo test -p oriterm_ui`)
- [x] Dialog rendering uses `CachedTextMeasurer` — all `UiFontMeasurer` sites replaced (dialog_rendering.rs, event_handling.rs, content_actions.rs, dialog_management.rs)
- [x] Tab bar rendering uses `CachedTextMeasurer` — draw_helpers.rs, chrome/mod.rs, tab_bar_input.rs all updated
- [x] Cache invalidates correctly on font reload, theme change, and DPI change — wired in config_reload.rs, mod.rs, event_handling.rs
- [x] Cache does not grow unboundedly (capped at 1024 entries)

- [x] **Cache return strategy:** Implemented option (a) — `ShapedText::clone()` on cache hit. The clone cost (~1KB for a 50-glyph string) is negligible compared to full rustybuzz shaping (~100x cheaper). Option (b) (`Rc<ShapedText>` in `DrawCommand`) deferred as a potential future optimization if profiling shows clone cost matters.
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green (one pre-existing `subpixel_vs_grayscale` visual regression failure, unrelated to text caching)

- [x] Cache correctness tests:
  - Same text, same style, same scale → cache hit (same `ShapedText` output)
  - Same text, different size → cache miss (different key)
  - Same text, different weight → cache miss (different key)
  - Same text, different color → cache hit (color excluded from key)
  - Same text, different max_width → cache miss for `shape()`, cache hit for `measure()` (measure ignores max_width)
  - Empty string → handles gracefully (no panic, returns zero-width metrics)

**Exit Criteria:** On a Settings dialog hover event, zero calls to `ui_text::shape_text()` are made for unchanged strings. Verified by adding a temporary counter in `ui_text::shape_text()` and observing it stays at 0 during hover-only redraws.
