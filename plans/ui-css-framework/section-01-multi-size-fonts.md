---
section: "01"
title: "Multi-Size Font Rendering"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "TextStyle.size drives actual glyph rasterization size so 18px titles, 13px body text, and 10-11px sidebar/footer text render at distinct physical sizes"
inspired_by:
  - "GPUI FontIdWithSize pattern (~/projects/reference_repos/gui_repos/zed/crates/gpui/src/text_system.rs)"
depends_on: []
sections:
  - id: "01.1"
    title: "Exact-Size UI Font Registry"
    status: not-started
  - id: "01.2"
    title: "UiFontMeasurer Size-Aware Shaping"
    status: not-started
  - id: "01.3"
    title: "Scene Conversion Size Threading"
    status: not-started
  - id: "01.4"
    title: "Glyph Cache Pre-warming"
    status: not-started
  - id: "01.5"
    title: "Integration + Tests"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.6"
    title: "Build & Verify"
    status: not-started
---

# Section 01: Multi-Size Font Rendering

## Problem

The current bug is real in the checked-in code:

- `UiFontMeasurer` always shapes against one `&FontCollection` in [ui_measurer.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_measurer.rs).
- `WindowRenderer::cache_scene_glyphs()` and `ui_size_q6()` also read one UI collection in [scene_append.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/scene_append.rs).
- `convert_text()` constructs every UI `RasterKey` with one `TextContext.size_q6` in [text.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/text.rs).

As a result, `TextStyle.size` is effectively ignored for real rasterization. Widgets already request multiple sizes in the current tree, including 10px and 11px in [sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs), 11.5px and 13px in [setting_row/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/setting_row/mod.rs), and larger title sizes in the settings/dialog code. Those requests all collapse onto one UI font collection today.

There is also a documentation bug to fix as part of this section: [TextStyle](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs) currently documents `size` as points, but the UI code and mockup use CSS-like logical pixels. Section 01 should make the behavior and docs match the actual contract: `TextStyle.size` is a logical pixel size, and the renderer converts it to physical pixels with the window scale factor.

## Current Flow (broken)

```text
Widget: TextStyle { size: 18.0, .. }
  -> UiFontMeasurer::shape(text, style, max_width)
    -> ui_text::shape_text(text, style, max_width * scale, self.collection)
      -> collection.size_px()             // one UI collection only
  -> WindowRenderer::cache_scene_glyphs()
    -> size_key(ui_fc.size_px())          // one size_q6 only
  -> scene_convert::convert_text()
    -> RasterKey { size_q6: ctx.size_q6 } // one size_q6 only
```

## Target Flow (fixed)

```text
Widget: TextStyle { size: 18.0, .. }      // logical px
  -> UiFontMeasurer::shape(text, style, max_width)
    -> UiFontSizes::select(18.0, scale)   // exact logical size -> exact collection
    -> ui_text::shape_text(..., collection_for_18px)
      -> ShapedText { size_q6: size_key(collection.size_px()), .. }
  -> WindowRenderer::cache_scene_glyphs()
    -> group scene text keys by shaped.size_q6
    -> UiFontSizes::select_by_q6(size_q6) for rasterization
  -> scene_convert::convert_text()
    -> RasterKey { size_q6: shaped.size_q6 }
```

## Non-Negotiable Invariants

1. `TextStyle.size` means logical pixels, not points.
2. Requested UI text size must round-trip exactly through shaping, atlas lookup, and rasterization.
3. Two text runs in the same `Scene` may use different sizes in the same frame.
4. No nearest-pool approximation is allowed for the final design. If a widget asks for 10px, it must not silently render from a 12px collection.

---

## 01.1 Exact-Size UI Font Registry

### Goal

Replace the single optional `ui_font_collection: Option<FontCollection>` on `WindowRenderer` with an exact-size registry that can return a `FontCollection` for the requested logical text size.

### Files

- `oriterm/src/font/ui_font_sizes/mod.rs` (new directory module)
- `oriterm/src/font/mod.rs`
- `oriterm/src/app/init/mod.rs`
- `oriterm/src/app/window_management.rs`
- `oriterm/src/app/dialog_management.rs`
- `oriterm/src/gpu/window_renderer/mod.rs`
- `oriterm/src/gpu/window_renderer/ui_only.rs`
- `oriterm/src/gpu/window_renderer/font_config.rs`

### Design

The reviewed plan should not use three nearest-match size pools. That design would knowingly rasterize some text at the wrong size and conflicts with the section goal.

Instead, add an exact-size registry:

```rust
pub(crate) struct UiFontSizes {
    font_set: FontSet,
    dpi: f32,
    format: GlyphFormat,
    hinting: HintingMode,
    weight: u16,
    collections: BTreeMap<u32, FontCollection>, // key = physical size_q6
}

impl UiFontSizes {
    pub(crate) fn new(
        font_set: FontSet,
        dpi: f32,
        format: GlyphFormat,
        hinting: HintingMode,
        weight: u16,
        preload_logical_sizes: &[f32],
    ) -> Result<Self, FontError> { ... }

    pub(crate) fn select(&self, logical_size: f32, scale: f32) -> Option<&FontCollection> { ... }

    pub(crate) fn select_mut(
        &mut self,
        logical_size: f32,
        scale: f32,
    ) -> Result<&mut FontCollection, FontError> { ... }

    pub(crate) fn select_by_q6(&self, size_q6: u32) -> Option<&FontCollection> { ... }

    pub(crate) fn select_by_q6_mut(&mut self, size_q6: u32) -> Option<&mut FontCollection> { ... }

    pub(crate) fn set_dpi(&mut self, dpi: f32) -> Result<(), FontError> { ... }

    pub(crate) fn set_hinting(&mut self, hinting: HintingMode) { ... }

    pub(crate) fn set_format(&mut self, format: GlyphFormat) { ... }
}
```

### Exact Size Construction

`FontCollection::new()` takes points, but the UI style contract is logical pixels. The conversion for a requested logical size is:

```rust
let logical_px = style.size;
let physical_px = logical_px * scale;
let size_q6 = size_key(physical_px);
let size_pt = logical_px * 72.0 / 96.0;
```

That keeps `collection.size_px()` aligned with `logical_px * scale` at runtime.

### Preloaded Sizes

Preload only the sizes the current UI actually uses frequently:

- `9.0`
- `10.0`
- `11.0`
- `11.5`
- `12.0`
- `13.0`
- `16.0`
- `18.0`

Anything else should be created lazily on first request, then retained for reuse.

### WindowRenderer Integration

Change `WindowRenderer` from:

```rust
ui_font_collection: Option<FontCollection>
```

to:

```rust
ui_font_sizes: Option<UiFontSizes>
```

Keep `font_collection` as the terminal/grid collection and as the fallback when UI font creation fails.

Because [window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs) is already 554 lines, any implementation in this section that touches it must first split out font-related state/accessors so the touched production file no longer violates the repo's 500-line limit.

### UiOnly Renderer Path

The current `UiOnly` constructor relies on "use `font_collection` as the UI font and let `active_ui_collection()` fall back". That fallback-based design no longer works once size selection becomes dynamic. `new_ui_only()` should own a real `UiFontSizes` registry as well.

### Checklist

- [ ] Create `oriterm/src/font/ui_font_sizes/mod.rs` with exact-size, lazily populated storage
- [ ] Re-export the module from `oriterm/src/font/mod.rs`
- [ ] Update startup in `app/init/mod.rs` to construct `UiFontSizes` instead of one 10pt collection
- [ ] Update `create_window_renderer()` and `create_dialog_renderer()` to construct the same registry
- [ ] Replace `ui_font_collection` with `ui_font_sizes` on `WindowRenderer`
- [ ] Split [window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs) before adding more code there
- [ ] Update `set_font_size()`, `set_hinting_and_format()`, and related font-config code to keep the registry in sync

---

## 01.2 UiFontMeasurer Size-Aware Shaping

### Goal

Make `UiFontMeasurer` select the exact `FontCollection` for each `TextStyle.size`.

### Files

- `oriterm/src/font/shaper/ui_measurer.rs`
- `oriterm/src/app/redraw/draw_helpers.rs`
- `oriterm/src/app/redraw/search_bar.rs`
- `oriterm/src/app/dialog_rendering.rs`
- `oriterm/src/app/dialog_context/`
- `oriterm/src/app/keyboard_input/mod.rs`
- `oriterm/src/app/mouse_input.rs`
- `oriterm_ui/src/text/mod.rs`

### Design

```rust
pub struct UiFontMeasurer<'a> {
    sizes: Option<&'a UiFontSizes>,
    fallback: &'a FontCollection,
    scale: f32,
}

impl UiFontMeasurer<'_> {
    fn collection_for_style(&self, style: &TextStyle) -> &FontCollection {
        self.sizes
            .and_then(|sizes| sizes.select(style.size, self.scale))
            .unwrap_or(self.fallback)
    }
}
```

This keeps the existing graceful fallback behavior when UI font creation fails, but the normal path uses exact size-aware collections.

### Required Doc Fix

Update [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs) so `TextStyle.size` is documented as logical pixels. The current "points" wording is incorrect relative to the actual UI API and this section's design.

### CachedTextMeasurer Impact

[cached_measurer/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/mod.rs) already keys entries by `size_hundredths`, so the cache shape is compatible with multi-size rendering. Only the wrapped measurer construction changes.

### Checklist

- [ ] Change `UiFontMeasurer` to borrow `Option<&UiFontSizes>` plus a fallback `&FontCollection`
- [ ] Select the correct collection in both `measure()` and `shape()`
- [ ] Update all constructor call sites that currently pass `renderer.active_ui_collection()`
- [ ] Fix `TextStyle.size` docs to say logical pixels
- [ ] Verify `measure()` and `shape()` agree for the same size/style

---

## 01.3 Scene Conversion Size Threading

### Goal

Thread the shaped text size through scene conversion and atlas caching without widening every `ShapedGlyph`.

### Files

- `oriterm_ui/src/text/mod.rs`
- `oriterm/src/font/shaper/ui_text.rs`
- `oriterm/src/gpu/scene_convert/mod.rs`
- `oriterm/src/gpu/scene_convert/text.rs`
- `oriterm/src/gpu/window_renderer/helpers.rs`
- `oriterm/src/gpu/window_renderer/scene_append.rs`

### Correct Data Shape

The earlier plan proposed storing `size_q6` on every `ShapedGlyph`. That is the wrong ownership boundary here:

- `ShapedGlyph` is shared with the terminal shaping path.
- Every glyph inside one UI `ShapedText` run already shares one size.
- The size is needed per run, not per glyph.

Store the size once on `ShapedText` instead:

```rust
pub struct ShapedText {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
    pub size_q6: u32,
}
```

Then stamp it in `ui_text::shape_to_shaped_text()`:

```rust
let size_q6 = crate::font::size_key(collection.size_px());
ShapedText::new(glyphs, width, metrics.height, metrics.baseline, size_q6)
```

### Scene Convert Changes

`convert_text()` should read `shaped.size_q6` instead of `TextContext.size_q6`:

```rust
let key = RasterKey {
    glyph_id: glyph.glyph_id,
    face_idx: FaceIdx(glyph.face_index),
    size_q6: shaped.size_q6,
    synthetic: SyntheticFlags::from_bits_truncate(glyph.synthetic),
    hinted: ctx.hinted,
    subpx_x: subpx,
    font_realm: FontRealm::Ui,
};
```

That lets one scene contain 10px sidebar text and 18px title text in the same frame.

### Atlas Cache Changes

`scene_raster_keys()` should push `RasterKey` values using `text_run.shaped.size_q6`.

`cache_scene_glyphs()` should then group or bucket those keys by `size_q6`, select the matching `FontCollection` from `UiFontSizes`, and call `ensure_glyphs_cached()` once per size bucket.

That is cleaner than teaching `ensure_glyphs_cached()` to switch collections per individual key.

### TextContext Simplification

Once size lives on `ShapedText`, `TextContext.size_q6` is no longer needed. Remove it. Icon conversion does not use it.

### Checklist

- [ ] Add `size_q6` to `ShapedText`, not `ShapedGlyph`
- [ ] Update `ShapedText::new()` and all UI-only construction sites/tests
- [ ] Stamp `size_q6` in `ui_text::shape_to_shaped_text()`
- [ ] Remove `size_q6` from `scene_convert::TextContext`
- [ ] Use `text_run.shaped.size_q6` in `scene_raster_keys()` and `convert_text()`
- [ ] Group UI raster keys by `size_q6` in `cache_scene_glyphs()`

---

## 01.4 Glyph Cache Pre-warming

### Goal

Pre-cache the most common UI sizes so the first settings/dialog frame does not hitch on glyph rasterization.

### Files

- `oriterm/src/gpu/window_renderer/helpers.rs`
- `oriterm/src/gpu/window_renderer/mod.rs`
- `oriterm/src/gpu/window_renderer/ui_only.rs`

### Correct Timing

The earlier plan incorrectly said this work runs on the font thread during init. It cannot: atlas creation and atlas insertion happen during renderer/GPU initialization on the main thread, in the same paths that currently call `create_atlases()`.

### Design

After creating the atlases, pre-cache printable ASCII for the preloaded UI sizes listed in 01.1. Reuse the existing `pre_cache_atlas()` helper shape, but drive it with the exact-size registry.

Keep this bounded:

- prewarm only the common sizes
- prewarm Regular and Bold
- lazily create and lazily rasterize uncommon sizes later

### Checklist

- [ ] Extend renderer init to pre-cache common UI sizes after atlas creation
- [ ] Apply the same prewarm logic in `new_ui_only()`
- [ ] Keep uncommon sizes lazy to avoid unnecessary startup cost
- [ ] Log the UI prewarm timing separately from terminal font pre-cache

---

## 01.5 Integration + Tests

### Goal

Prove end-to-end correctness for mixed-size scenes, not just isolated shaping.

### Tests

Add or update tests in these areas:

- `oriterm/src/font/ui_font_sizes/tests.rs`
  - exact-size lookup returns the requested size bucket, not a nearest bucket
  - lazy insertion creates a new collection for an unseen size
- `oriterm/src/font/shaper/tests.rs`
  - 18px text measures wider/taller than 13px text
  - shaped output stamps the expected `size_q6`
- `oriterm/src/gpu/scene_convert/tests.rs`
  - two text runs with different sizes produce different `RasterKey.size_q6` values
  - a scene containing mixed sizes resolves atlas lookups correctly
- `oriterm_ui/src/text/tests.rs`
  - `ShapedText::new()` stores `size_q6`
- `oriterm_ui/src/testing/mock_measurer.rs`
  - update the helper to construct `ShapedText` with an explicit `size_q6`

### Visual Verification

After implementation:

1. Open the settings dialog.
2. Confirm the page title is visibly larger than setting labels.
3. Confirm sidebar section titles and footer text are visibly smaller than body text.
4. Confirm mixed-size content in one frame does not show missing-glyph flashes after the first frame.

### Checklist

- [ ] Unit tests for exact-size registry behavior
- [ ] Unit tests for size-aware measurement/shaping
- [ ] Scene conversion tests covering mixed-size text runs
- [ ] Update UI text tests and mock measurer for the new `ShapedText` shape
- [ ] Manual verification in the settings dialog

---

## 01.R Third Party Review Findings

- [x] `[TPR-01-001][high]` [plans/ui-css-framework/section-01-multi-size-fonts.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-01-multi-size-fonts.md) - The original three-pool nearest-match design could not satisfy the section goal because it intentionally rendered several requested sizes from the wrong collection. Resolved: replaced with an exact-size registry plus lazy creation on 2026-03-23.

- [x] `[TPR-01-002][medium]` [plans/ui-css-framework/section-01-multi-size-fonts.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-01-multi-size-fonts.md) - Storing `size_q6` on every `ShapedGlyph` was the wrong boundary and would have created unnecessary churn in terminal-shaping code that shares the glyph type. Resolved: store `size_q6` once on `ShapedText` and consume it per text run on 2026-03-23.

- [x] `[TPR-01-003][medium]` [plans/ui-css-framework/section-01-multi-size-fonts.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-01-multi-size-fonts.md) - The original prewarm subsection claimed atlas work happened on the font thread, but current atlas creation occurs in renderer/GPU init paths only. Resolved: updated the section to prewarm during renderer initialization on 2026-03-23.

- [x] `[TPR-01-004][medium]` [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs) - The plan inherited the codebase's misleading "size in points" wording even though widgets and the mockup use logical pixel sizes. Resolved: made the documentation correction an explicit task in Section 01 on 2026-03-23.

---

## 01.6 Build & Verify

### Gate

All of the following must pass before this section is marked complete:

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

1. `cargo test -p oriterm ui_font_sizes`
2. `cargo test -p oriterm scene_convert`
3. `cargo test -p oriterm_ui text`
4. Manual settings-dialog check at the active window scale factor

### Completion Criteria

- `TextStyle.size` changes rendered output, not just layout metadata
- mixed-size text in one `Scene` uses distinct `RasterKey.size_q6` values
- no nearest-size substitution remains in the production path
- normal windows and `UiOnly` dialog windows use the same size-aware pipeline
