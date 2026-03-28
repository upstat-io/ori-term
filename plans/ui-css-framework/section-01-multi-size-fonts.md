---
section: "01"
title: "Multi-Size Font Rendering"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-24
goal: "TextStyle.size drives actual glyph rasterization size so 18px titles, 13px body text, and 10-11px sidebar/footer text render at distinct physical sizes"
inspired_by:
  - "GPUI FontIdWithSize pattern (~/projects/reference_repos/gui_repos/zed/crates/gpui/src/text_system.rs)"
depends_on: []
sections:
  - id: "01.1"
    title: "Exact-Size UI Font Registry"
    status: complete
  - id: "01.2"
    title: "UiFontMeasurer Size-Aware Shaping"
    status: complete
  - id: "01.3"
    title: "Scene Conversion Size Threading"
    status: complete
  - id: "01.4"
    title: "Glyph Cache Pre-warming"
    status: complete
  - id: "01.5"
    title: "Integration + Tests"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.6"
    title: "Build & Verify"
    status: complete
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

`FontCollection::new()` takes `size_pt` and `dpi`, then internally computes `size_px = (size_pt * dpi / 72.0).clamp(...)`. The UI style contract uses logical pixels. The conversion for a requested logical size is:

```rust
let logical_px = style.size;
let physical_px = logical_px * scale;
let size_q6 = size_key(physical_px);
let size_pt = logical_px * 72.0 / 96.0;
```

That keeps `collection.size_px()` aligned with `logical_px * scale` at runtime.

**Important**: the `size_pt = logical_px * 72.0 / 96.0` formula is correct for all scale factors. Pass the actual window DPI (which already encodes scale: e.g. `192` at 2x) to `FontCollection::new()`. Then internally: `size_px = size_pt * dpi / 72.0 = (logical_px * 72/96) * dpi / 72 = logical_px * dpi / 96 = logical_px * scale`. The `size_pt` value stays constant regardless of scale — the DPI parameter handles physical scaling. Do NOT use `logical_px * 72.0 / dpi` as `size_pt`; that would cancel the scale factor and produce logical-sized (unscaled) glyphs.

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

- [x] Create `oriterm/src/font/ui_font_sizes/mod.rs` with exact-size, lazily populated storage
- [x] Create `oriterm/src/font/ui_font_sizes/tests.rs` with `#[cfg(test)] mod tests;` in `mod.rs`
- [x] Re-export the module from `oriterm/src/font/mod.rs`
- [x] Update startup in `app/init/mod.rs` to construct `UiFontSizes` instead of one 10pt collection
- [x] Update `create_window_renderer()` and `create_dialog_renderer()` to construct the same registry
- [x] Replace `ui_font_collection` with `ui_font_sizes` on `WindowRenderer`
- [x] Split [window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs) before adding more code there
- [x] Update `set_font_size()`, `set_hinting_and_format()`, and related font-config code to keep the registry in sync
- [x] Remove the hardcoded `ui_fc.set_size(11.0, dpi)` in `font_config.rs` and replace it with registry-wide DPI update

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

- [x] Change `UiFontMeasurer` to borrow `Option<&UiFontSizes>` plus a fallback `&FontCollection`
- [x] Select the correct collection in both `measure()` and `shape()`
- [x] Update all constructor call sites that currently pass `renderer.active_ui_collection()`
- [x] Fix `TextStyle.size` docs to say logical pixels
- [x] Verify `measure()` and `shape()` agree for the same size/style

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

`convert_text()` in `scene_convert/text.rs` should read `shaped.size_q6` instead of `TextContext.size_q6`. Currently at line 66 it reads `size_q6: ctx.size_q6`. After this change:

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

`scene_raster_keys()` in `helpers.rs` (currently line 199) takes a single `size_q6` parameter and stamps every key with it. After this change, it should read `text_run.shaped.size_q6` per text run instead.

`cache_scene_glyphs()` in `scene_append.rs` (currently line 109) currently calls `scene_raster_keys(scene, size_q6, hinted, scale, &mut self.ui_raster_keys)` with one size. After this change, it should group or bucket the resulting keys by `size_q6`, select the matching `FontCollection` from `UiFontSizes` via `select_by_q6_mut()`, and call `ensure_glyphs_cached()` once per size bucket.

That is cleaner than teaching `ensure_glyphs_cached()` to switch collections per individual key.

### TextContext Simplification

Once size lives on `ShapedText`, `TextContext.size_q6` is no longer needed. Remove it. Icon conversion does not use it.

### Checklist

- [x] Add `size_q6` to `ShapedText`, not `ShapedGlyph`
- [x] Update `ShapedText::new()` and all UI-only construction sites/tests
- [x] Stamp `size_q6` in `ui_text::shape_to_shaped_text()`
- [x] Remove `size_q6` from `scene_convert::TextContext`
- [x] Use `text_run.shaped.size_q6` in `scene_raster_keys()` and `convert_text()`
- [x] Group UI raster keys by `size_q6` in `cache_scene_glyphs()`

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

- [x] Extend renderer init to pre-cache common UI sizes after atlas creation
- [x] Apply the same prewarm logic in `new_ui_only()`
- [x] Keep uncommon sizes lazy to avoid unnecessary startup cost
- [x] Log the UI prewarm timing separately from terminal font pre-cache

---

## 01.5 Integration + Tests

### Goal

Prove end-to-end correctness for mixed-size scenes, not just isolated shaping.

### Tests

Add or update tests in these areas:

- `oriterm/src/font/ui_font_sizes/tests.rs`
  - `fn select_returns_exact_size_collection()` — exact-size lookup returns the requested size bucket, not a nearest bucket
  - `fn select_returns_none_for_missing_size()` — unseen size with immutable `select()` returns `None`
  - `fn select_mut_creates_collection_for_unseen_size()` — lazy insertion creates a new collection for an unseen size
  - `fn select_by_q6_finds_preloaded_size()` — q6 key lookup matches a preloaded collection
  - `fn select_by_q6_returns_none_for_unknown()` — q6 key lookup for never-created size returns `None`
  - `fn set_dpi_rebuilds_all_collections()` — DPI change rebuilds every existing collection at the new physical size
  - `fn preloaded_sizes_match_expected_set()` — the `new()` constructor with the standard preload list creates exactly the expected entries
- `oriterm/src/font/shaper/tests.rs`
  - `fn larger_size_measures_wider_and_taller()` — 18px text measures wider/taller than 13px text
  - `fn shaped_output_stamps_expected_size_q6()` — shaped output stamps the expected `size_q6`
  - `fn zero_length_text_returns_zero_width_with_valid_size_q6()` — empty string shaping produces zero width but valid `size_q6`
- `oriterm/src/gpu/scene_convert/tests.rs`
  - `fn mixed_size_text_runs_produce_different_raster_key_size_q6()` — two text runs with different sizes produce different `RasterKey.size_q6` values
  - `fn scene_mixed_sizes_atlas_lookups_correct()` — a scene containing mixed sizes resolves atlas lookups correctly
- `oriterm_ui/src/text/tests.rs`
  - `fn shaped_text_new_stores_size_q6()` — `ShapedText::new()` stores `size_q6`
  - `fn shaped_text_default_has_zero_size_q6()` — default or empty `ShapedText` has a sensible `size_q6` value
- `oriterm_ui/src/testing/mock_measurer.rs`
  - update the helper to construct `ShapedText` with an explicit `size_q6`

### Visual Verification

After implementation:

1. Open the settings dialog.
2. Confirm the page title is visibly larger than setting labels.
3. Confirm sidebar section titles and footer text are visibly smaller than body text.
4. Confirm mixed-size content in one frame does not show missing-glyph flashes after the first frame.

### Checklist

- [x] Unit tests for exact-size registry behavior
- [x] Unit tests for size-aware measurement/shaping
- [x] Scene conversion tests covering mixed-size text runs
- [x] Update UI text tests and mock measurer for the new `ShapedText` shape
- [x] Manual verification in the settings dialog

---

## 01.R Third Party Review Findings

- [x] `[TPR-01-020][high]` `oriterm/src/gpu/window_renderer/scene_append.rs:131` — unsupported UI text sizes now take the documented warning-and-fallback path in `UiFontSizes::select()`, but `cache_scene_glyphs()` still unconditionally expects every shaped `size_q6` to exist in the registry and panics otherwise.
  Resolved 2026-03-24: accepted and fixed — replaced `.expect()` with a two-phase lookup: shared `select_by_q6()` check, then mutable borrow of either the registry entry or `self.font_collection` as fallback. This mirrors `UiFontMeasurer::collection_for_style()`'s graceful fallback. Logs a warning when the fallback path triggers so unregistered sizes are visible. Also removed `#[allow(dead_code)]` from `select_by_q6()` since it's now used in production.

- [x] `[TPR-01-017][medium]` `oriterm/src/app/config_reload/mod.rs:345` — `apply_font_config()` assigns per-fallback metadata by config index instead of loaded fallback index, so a skipped user fallback shifts `size_offset` and feature overrides onto the wrong loaded font.
  Resolved 2026-03-24: accepted and fixed — changed `prepend_user_fallbacks()` to return `Vec<usize>` mapping loaded index → config index. Replaced `user_fb_count: usize` with `fallback_map: &[usize]` throughout `apply_font_config`, `apply_font_config_to_ui_sizes`, `rebuild_ui_font_sizes`, and all callers. Per-fallback metadata loop now uses `fallback_map[loaded_idx]` to look up the correct config entry. Added regression test `apply_font_config_skipped_fallback_metadata_uses_correct_config_entry`.

- [x] `[TPR-01-018][high]` `oriterm/src/app/config_reload/mod.rs:369` — codepoint-map face resolution still uses the raw config position rather than the loaded fallback position, so mappings can target the wrong fallback or a nonexistent face whenever any earlier user fallback failed to load.
  Resolved 2026-03-24: accepted and fixed — codepoint-map resolution now finds the config index by family name, then looks up the loaded index via `fallback_map.iter().position(|&mi| mi == ci)`. Families not in the loaded map are logged and skipped. Added regression tests `apply_font_config_codepoint_map_skipped_fallback_resolves_correct_loaded_index` and `apply_font_config_codepoint_map_unloaded_family_skipped`.

- [x] `[TPR-01-019][medium]` `oriterm/src/app/dialog_management.rs:343` — dialog UI renderers still derive subpixel-vs-alpha glyph format from the main-window opacity even though dialog windows are always created opaque.
  Resolved 2026-03-24: accepted and fixed — `create_dialog_renderer()` now passes `1.0` to `resolve_subpixel_mode()` instead of `self.config.window.effective_opacity()`, matching the dialog surface's opaque contract. Comment documents the rationale.

- [x] `[TPR-01-016][high]` `oriterm/src/font/ui_font_sizes/mod.rs:269` — DPI-triggered UI font-registry rebuilds still drop the user font config that Section 01 already fixed for startup and hot reload.
  Resolved 2026-03-24: accepted and fixed — added `PostRebuildHook` (a `Box<dyn Fn(&mut FontCollection)>`) field to `UiFontSizes`. The hook captures font config and is invoked in `rebuild_all()`, `ensure_size()`, and `create_default_collection()`. All 4 creation sites (init, new-window, dialog, hot-reload) now call `apply_font_config_to_ui_sizes()` which both applies config and installs the hook. Three regression tests verify DPI rebuild, ensure_size, and create_default_collection all preserve features.

- [x] `[TPR-01-015][high]` `oriterm/src/app/init/mod.rs:113` — startup, new-window, and UI-only dialog creation all build `UiFontSizes` without applying the user font config that the live terminal `FontCollection` receives.
  Resolved 2026-03-24: accepted and fixed — all three creation paths (`init/mod.rs`, `window_management.rs`, `dialog_management.rs`) now call `apply_font_config()` on every collection in the `UiFontSizes` registry immediately after construction, matching the hot-reload path's behavior.

- [x] `[TPR-01-013][medium]` [oriterm/src/font/ui_font_sizes/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/ui_font_sizes/mod.rs#L151) - Section 01 still cannot create exact-size UI collections on first request, so non-preloaded widget sizes silently render from the fallback collection.
  Resolved 2026-03-23: accepted. Chose option 2: narrowed the contract to preloaded sizes only. Audited all `font_size` / `TextStyle::new()` entry points across `oriterm_ui` — every production size (9.0, 9.5, 10.0, 11.0, 11.5, 12.0, 13.0, 16.0, 18.0) is in `PRELOAD_SIZES`. Updated doc comments on `PRELOAD_SIZES`, `UiFontSizes`, and `ensure_size()` to document that lazy creation does not happen at render time and that new sizes must be registered via `ensure_size()` in `&mut` contexts before creating the measurer.

- [x] `[TPR-01-010][high]` [oriterm/src/font/ui_font_sizes/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/ui_font_sizes/mod.rs#L145) - The exact-size registry still falls back to the default collection for live widget sizes that are not preloaded, so Section 01's "no nearest-size substitution remains" completion claim is false in the current tree.
  Resolved 2026-03-23: accepted. Added 9.5 to `PRELOAD_SIZES` so color swatch index labels get an exact-size collection instead of falling back to the 13px default.

- [x] `[TPR-01-011][medium]` [oriterm/src/gpu/window_renderer/helpers.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/helpers.rs#L372) - Section 01's UI font prewarm path seeds atlas entries under terminal-style keys, so the prewarmed glyphs cannot satisfy later UI text lookups.
  Resolved 2026-03-23: accepted. Added `FontRealm` parameter to `pre_cache_atlas()`. Terminal callers pass `FontRealm::Terminal`, `prewarm_ui_font_sizes()` passes `FontRealm::Ui`. Keys now match between prewarm and live UI lookups.

- [x] `[TPR-01-008][medium]` [oriterm/src/font/shaper/ui_measurer.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_measurer.rs#L56) - The live UI measurer never exercises the registry's lazy-creation path, so Section 01 still falls back to the default 13px collection for any non-preloaded size instead of creating an exact-size collection.
  Resolved 2026-03-23: accepted. Replaced `select_mut()` with `ensure_size()` for `&mut` contexts. Made `select()` log a warning on missing sizes. The `UiFontMeasurer` is `&self` (via `TextMeasurer` trait through `CachedTextMeasurer`) so interior mutability would require `RefCell` that can't return `&FontCollection` through a `Ref` guard. All current sizes are in `PRELOAD_SIZES`. The warning makes fallback explicit.

- [x] `[TPR-01-009][low]` [oriterm/src/gpu/window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs#L1) - `WindowRenderer` still violates the repository's 500-line source-file limit after Section 01's claimed split.
  Resolved 2026-03-23: accepted and fixed. Extracted `SurfaceError` into `window_renderer/error.rs` (re-exported). File is now 483 lines. Also cleaned decorative banners.

- [x] `[TPR-01-001][high]` [plans/ui-css-framework/section-01-multi-size-fonts.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-01-multi-size-fonts.md) - The original three-pool nearest-match design could not satisfy the section goal because it intentionally rendered several requested sizes from the wrong collection. Resolved: replaced with an exact-size registry plus lazy creation on 2026-03-23.

- [x] `[TPR-01-002][medium]` [plans/ui-css-framework/section-01-multi-size-fonts.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-01-multi-size-fonts.md) - Storing `size_q6` on every `ShapedGlyph` was the wrong boundary and would have created unnecessary churn in terminal-shaping code that shares the glyph type. Resolved: store `size_q6` once on `ShapedText` and consume it per text run on 2026-03-23.

- [x] `[TPR-01-003][medium]` [plans/ui-css-framework/section-01-multi-size-fonts.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-01-multi-size-fonts.md) - The original prewarm subsection claimed atlas work happened on the font thread, but current atlas creation occurs in renderer/GPU init paths only. Resolved: updated the section to prewarm during renderer initialization on 2026-03-23.

- [x] `[TPR-01-004][medium]` [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs) - The plan inherited the codebase's misleading "size in points" wording even though widgets and the mockup use logical pixel sizes. Resolved: made the documentation correction an explicit task in Section 01 on 2026-03-23.

- [x] `[TPR-01-005][high]` [oriterm/src/gpu/window_renderer/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/mod.rs#L262) - The new `UiFontSizes` registry is still dead state in the live UI pipeline, so `TextStyle.size` remains ignored at runtime. Resolved 2026-03-23: accepted — this is exactly what subsections 01.2, 01.3, and 01.5 implement. No new tasks needed; the existing plan items wire the registry into the pipeline end-to-end.

- [x] `[TPR-01-006][medium]` [plans/brutal-design-pass-2/section-01-appearance-tab.md](/home/eric/projects/ori_term/plans/brutal-design-pass-2/section-01-appearance-tab.md#L1) - The brutal-design pass was marked complete even though the typography dependency it relies on is still incomplete. Resolved 2026-03-23: accepted — added re-verification of settings dialog visual fidelity to 01.5 manual verification checklist. The brutal-design pass metadata stays as-is until 01.5 visual verification confirms or reopens specific claims.

- [x] `[TPR-01-007][low]` [oriterm/src/font/ui_font_sizes/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/ui_font_sizes/mod.rs#L98) - The new registry module introduces decorative banner comments that violate the repository hygiene rules. Resolved 2026-03-23: accepted and fixed — replaced `// ── X ──` decorative banners with plain `// X` section labels.

- [x] `[TPR-01-012][low]` [oriterm_ui/src/theme/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/theme/mod.rs#L69) - Section 01 corrected `TextStyle.size` to logical pixels, but several public wrapper/theme APIs still document their font-size fields as points. Resolved 2026-03-23: accepted and fixed — updated doc comments on `UiTheme::{font_size,font_size_small,font_size_large}`, `LabelStyle.font_size`, `ButtonStyle.font_size`, `DropdownStyle.font_size`, `StatusBadgeStyle.font_size`, `SeparatorStyle.label_font_size`, and `TextInputStyle.font_size` from "points" to "logical pixels".

- [x] `[TPR-01-014][high]` `oriterm/src/app/config_reload/mod.rs:140` — font config reload rebuilds only the terminal `FontCollection`, leaving the live `UiFontSizes` registry stale. Resolved 2026-03-23: accepted and fixed — `apply_font_changes()` now calls `rebuild_ui_font_sizes()` to construct a fresh `UiFontSizes` from the same `FontSet`/DPI/format/hinting/weight, applies user font config (features, fallback meta, codepoint mappings) to every collection, and calls `renderer.replace_ui_font_sizes()` before `replace_font_collection()`. Also enhanced `clear_and_recache()` to prewarm UI font atlases alongside terminal atlases on every atlas clear.

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
- [x] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)
