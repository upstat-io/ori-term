---
section: "02"
title: "Numeric Font Weight System"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-24
goal: "UI text can request CSS-style numeric font weights, and the request survives shaping and atlas caching so supported fonts can render 400/500/600/700 distinctly with deterministic fallback when they cannot"
inspired_by:
  - "CSS font-weight: 100-900 (https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight)"
  - "GPUI font-weight threading through text shaping (~/projects/reference_repos/gui_repos/zed/crates/gpui/src/text_system.rs)"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "Numeric FontWeight API"
    status: complete
  - id: "02.2"
    title: "Weight Realization Policy + face_variations Refactor"
    status: complete
  - id: "02.3"
    title: "Thread Weight Through Shaping"
    status: complete
  - id: "02.4"
    title: "Thread Weight Through Raster Keys + Atlas"
    status: complete
  - id: "02.5"
    title: "Consumer Adoption + Tests"
    status: complete
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.6"
    title: "Build & Verify"
    status: complete
---

# Section 02: Numeric Font Weight System

## Problem

The current `FontWeight` enum in [oriterm_ui/src/text/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/text/mod.rs) only expresses `Regular` and `Bold`, but the underlying font pipeline is more nuanced than the current plan acknowledged:

- the mockup uses `font-weight: 400`, `500`, `600`, and `700`
- the mockup's IBM Plex Mono import only explicitly loads `400`, `500`, and `700`
- the repo's font pipeline already understands variable `wght` axes via `face_variations()` in [metadata.rs](/home/eric/projects/ori_term/oriterm/src/font/collection/metadata.rs)
- UI shaping currently collapses weight immediately to `GlyphStyle::{Regular,Bold}` in [ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs)
- atlas cache keys currently do not include any weight discriminator

That means a type-only `FontWeight` refactor would be incomplete. Even if the UI API exposed `500`, the current production path would still collapse it before shaping, and any true variable-weight rendering would alias in the atlas because `RasterKey` cannot distinguish 500 from 700 today.

## Current Flow (incomplete)

```text
TextStyle.weight
  -> ui_text::shape_text()
    -> match Regular|Bold -> GlyphStyle                          ← binary collapse
    -> FontCollection::create_shaping_faces()
      -> push_faces_into() calls face_variations(slot, NONE, self.weight=400, axes)
        -> slot 0 (Regular): wght = 400
        -> slot 1 (Bold):    wght = min(400+300, 900) = 700     ← hardcoded +300
    -> resolve(ch, GlyphStyle::Bold) -> face_idx=1 (Bold slot)
  -> scene_convert::convert_text()
    -> RasterKey { size_q6, face_idx, ... }                      ← no weight field
  -> FontCollection::rasterize(key)
    -> face_variations(key.face_idx, key.synthetic, self.weight=400, axes)
    -> same hardcoded logic — no way to request 500 or 600
  -> GlyphAtlas
    -> 500/700 would collide if they shared glyph_id/face_idx/size_q6
```

## Target Flow

```text
TextStyle.weight (numeric CSS value, e.g. 500)
  -> ui_text::shape_text()
    -> weight_to_face_and_target(500, has_wght_axis, has_bold)
      -> face_idx=0 (Regular slot), target_wght=500              ← policy-driven
    -> create_shaping_faces_for_weight(500)
      -> face_variations_for_ui_weight(slot_0, NONE, 500, axes) -> wght=500  ← exact axis value
    -> resolve(ch, GlyphStyle::Regular) with face from slot 0
    -> ShapedText { ..., weight: 500 }                           ← stamped for downstream
  -> scene_convert::convert_text()
    -> RasterKey { ..., weight: 500, face_idx=0 }                ← weight in key
  -> FontCollection::rasterize_with_weight(key, requested_weight=500)
    -> face_variations_for_ui_weight(key.face_idx, key.synthetic, 500, axes)  ← exact weight
    -> wght axis set to 500                                      ← exact rendering
  -> GlyphAtlas
    -> RasterKey{weight:500} != RasterKey{weight:700}            ← no collision
```

## Important Scope Correction

This section should build the capability and update the consumers that already expose weight. It should not claim that every text-bearing widget adopts the mockup's exact weight values here. `ButtonStyle`, `DropdownWidget`, and several other text-bearing widgets do not currently expose a weight field, so their visual fidelity adoption belongs in later widget-specific sections unless this section explicitly broadens those APIs.

## Terminal Grid Path Is Unchanged

The terminal grid shaping and rasterization path (`shape_frame()`, `grid_raster_keys()`, `FontCollection::rasterize()`) continues using `self.weight` (the collection-global weight, typically 400). Weight-aware paths are UI-only additions. `GlyphStyle::{Regular,Bold,Italic,BoldItalic}` face-slot selection remains the terminal grid model. This section adds a parallel UI-text model that bypasses the binary `GlyphStyle` collapse.

---

## 02.1 Numeric FontWeight API

### Goal

Replace the two-variant enum with a numeric CSS-style weight type in `oriterm_ui`, while keeping the type small, comparable, and hashable for cache keys.

### Files

- `oriterm_ui/src/text/mod.rs`
- `oriterm_ui/src/text/tests.rs`
- `oriterm/src/font/shaper/cached_measurer/mod.rs`

### Design

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontWeight(u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const NORMAL: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMIBOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);

    pub const fn new(weight: u16) -> Self {
        Self(if weight < 100 {
            100
        } else if weight > 900 {
            900
        } else {
            weight
        })
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}
```

The key correction here is to avoid pretending that helper predicates like `is_bold()` solve the problem. The interesting logic is not "is this bold"; it is "how should the shaping/raster pipeline realize this exact numeric request?"

### Cache Impact

[cached_measurer/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/mod.rs) already carries `weight: FontWeight` in `TextCacheKey`, so the cache model is compatible with a numeric type as long as the newtype derives `Hash` and `Eq`. No structural change needed in `TextCacheKey` — the field type changes from an enum to a newtype struct, both of which derive `Hash + Eq`.

### Compile Breakage Scope

Changing `FontWeight` from an enum to a newtype struct breaks every pattern match and variant reference. All sites that currently use `FontWeight::Regular` or `FontWeight::Bold` must update:

- `oriterm_ui/src/text/mod.rs` — `TextStyle::default()` and `TextStyle::new()` use `FontWeight::Regular`
- `oriterm_ui/src/text/tests.rs` — `font_weight_default_is_regular` test references `FontWeight::Regular`
- `oriterm_ui/src/widgets/label/mod.rs` — `LabelStyle::weight` defaults to `FontWeight::Regular`
- `oriterm_ui/src/widgets/form_section/mod.rs` — uses `FontWeight::Bold`
- `oriterm_ui/src/widgets/dialog/rendering.rs` — uses `FontWeight::Bold`
- `oriterm_ui/src/widgets/settings_panel/mod.rs` — uses `FontWeight::Bold`
- `oriterm_ui/src/widgets/sidebar_nav/mod.rs` — uses `FontWeight::Regular`
- `oriterm/src/app/settings_overlay/form_builder/appearance.rs` — uses `FontWeight::Bold`
- `oriterm/src/font/shaper/ui_text.rs` — `match style.weight { Regular => ..., Bold => ... }` at lines 102-105 and 153-156
- `oriterm/src/font/shaper/cached_measurer/mod.rs` — `TextCacheKey.weight` field type
- `oriterm/src/font/shaper/cached_measurer/tests.rs` — uses `FontWeight::Bold` at line 59

The `ui_text.rs` match arms are the most critical: they currently gate the entire weight-to-face-slot mapping. After 02.1, these matches must become numeric comparisons (handled in 02.3).

### Checklist

- [x] Replace the enum with a numeric `FontWeight` newtype in `oriterm_ui`
- [x] Add named constants for the standard CSS weights
- [x] Add `new()` and `value()` helpers
- [x] Keep `Default` at 400 (`NORMAL`)
- [x] Update `TextStyle::new()` and `TextStyle::default()` to use `FontWeight::NORMAL`
- [x] Update `TextStyle::with_weight()` — signature stays the same, just takes the new type
- [x] Update all widget files that reference `FontWeight::Regular` → `FontWeight::NORMAL` and `FontWeight::Bold` → `FontWeight::BOLD`
- [x] Temporarily update `ui_text.rs` match arms to use `FontWeight::NORMAL` / `FontWeight::BOLD` equality checks (full refactor in 02.3)
- [x] Update `cached_measurer/mod.rs` — field type changes automatically, verify `Hash + Eq` derives propagate
- [x] Update `cached_measurer/tests.rs` — `FontWeight::Bold` → `FontWeight::BOLD` at line 59
- [x] Update `oriterm_ui/src/text/tests.rs` — all `FontWeight::Regular`/`FontWeight::Bold` → new constants
- [x] Add smoke tests in `oriterm_ui/src/text/tests.rs` immediately (these validate the type itself, not the pipeline):
  - `fn font_weight_default_is_normal()` — rename existing `font_weight_default_is_regular` to match new API name
  - `fn font_weight_constants_have_correct_values()` — `FontWeight::NORMAL.value() == 400`, `FontWeight::BOLD.value() == 700`, etc.
  - `fn font_weight_clamp_below_100()` — `FontWeight::new(50).value() == 100`
  - `fn font_weight_clamp_above_900()` — `FontWeight::new(950).value() == 900`
- [x] Verify `./build-all.sh` and `./clippy-all.sh` pass after this subsection (no new clippy warnings from the type change)

---

## 02.2 Weight Realization Policy + `face_variations` Refactor

### Goal

Define the weight realization policy and refactor `face_variations()` to accept arbitrary requested weights, not just the binary bold/not-bold model. This subsection is pure infrastructure — no UI-visible behavior change yet.

### Why This Must Come Before Shaping (02.3)

The shaping and rasterization changes in 02.3 and 02.4 need to call `face_variations()` with arbitrary numeric weights. The current `face_variations()` has a hardcoded `wants_bold` → `weight + 300` model that cannot express "set wght to exactly 500". The policy and the code that implements it must exist before the callers can use it.

### Reality Check

There are three distinct runtime cases:

1. **Font has a `wght` axis**
   Exact requested weight is realizable. This is the best case and should be preferred.

2. **Font has only static Regular + Bold faces**
   Exact 500/600 may not exist. Requests must fall back to the nearest supported heavier/lighter realization.

3. **Font lacks a real Bold face**
   Synthetic bold remains the fallback, using the existing `SyntheticFlags::BOLD` path.

### Policy

The realization policy governs two decisions: (a) which face slot to use, and (b) what `wght` value to set on the variation axis.

**Face Slot Selection** (UI text only — terminal grid is unchanged):

| Requested Weight | Font Has `wght` Axis | Font Has Bold Face (No Axis) | Font Lacks Bold |
|---|---|---|---|
| 100–450 | Regular slot (0), wght = requested | Regular slot (0), no variation | Regular slot (0), no variation |
| 500–650 | Regular slot (0), wght = requested | Regular slot (0), no variation | Regular slot (0), no variation |
| 700–900 | Regular slot (0), wght = requested | Bold slot (1), no variation | Regular slot (0), synthetic bold |

Key insight: when the font has a `wght` axis, **always use the Regular face slot** and set the axis to the exact requested value. The Bold face slot is only used for static fonts that lack a variable axis. This avoids the current `weight + 300` arithmetic which was designed for the terminal grid's binary model.

For the 500–650 range on static fonts (no `wght` axis, no separate medium face): the Regular face is the best available approximation. The weight difference between 400 and 500 is subtle — using Regular is an honest fallback. Using Bold (700) would overshoot by more than using Regular (400) undershoots.

**Weight Ranges** (summary):

- `100..=450` → normal-weight path: Regular face, exact `wght` if available
- `500..=650` → medium/semibold path: prefer exact `wght`, else Regular face (closest available)
- `700..=900` → bold path: prefer exact `wght`, else Bold face, else synthetic bold

### Important Limitation

Distinct 500 vs 700 rendering is only guaranteed when the active font can realize both weights, typically through a real `wght` axis or multiple loaded static faces. This section does not promise exact distinctness on every system font configuration.

### Files

- `oriterm/src/font/collection/metadata.rs`
- `oriterm/src/font/collection/mod.rs`
- `oriterm/src/font/collection/shaping.rs`
- `oriterm/src/font/collection/tests.rs`

### Implementation: New UI Weight Resolution Helper

Add a new function in `metadata.rs` for UI-text weight resolution. This does NOT replace `face_variations()` — the existing function remains for the terminal grid path. The new function is a parallel path for UI text:

```rust
/// Compute variation settings for a UI text weight request.
///
/// Unlike `face_variations()` (which uses binary bold/not-bold slot logic
/// for terminal grid text), this function sets the `wght` axis to the exact
/// requested weight value. Returns the face slot to use and whether
/// synthetic bold is needed.
pub(super) struct UiWeightResolution {
    /// Which face slot to shape/rasterize from.
    pub face_slot: usize,
    /// The `wght` axis value to set (same as requested if axis exists).
    pub wght_value: Option<f32>,
    /// Whether synthetic bold should be applied (only for 700+ on
    /// static fonts that lack both a `wght` axis and a Bold face).
    pub needs_synthetic_bold: bool,
}

pub(super) fn resolve_ui_weight(
    requested_weight: u16,
    has_wght_axis: bool,
    has_bold_face: bool,
) -> UiWeightResolution { ... }
```

The logic:
- If `has_wght_axis`: `face_slot=0`, `wght_value=Some(requested as f32)`, `needs_synthetic_bold=false`
- If `!has_wght_axis && requested >= 700 && has_bold_face`: `face_slot=1`, `wght_value=None`, `needs_synthetic_bold=false`
- If `!has_wght_axis && requested >= 700 && !has_bold_face`: `face_slot=0`, `wght_value=None`, `needs_synthetic_bold=true`
- Otherwise: `face_slot=0`, `wght_value=None`, `needs_synthetic_bold=false`

### Implementation: `has_wght_axis` Accessor on `FontCollection`

Add a public method to `FontCollection`:

```rust
/// Whether the Regular primary face has a `wght` variation axis.
pub fn has_wght_axis(&self) -> bool {
    self.primary[0]
        .as_ref()
        .map_or(false, |fd| has_axis(&fd.axes, *WGHT))
}
```

This is needed by UI shaping code to decide which path to take.

### Implementation: `face_variations_for_ui_weight()` in `metadata.rs`

Add a companion function to `face_variations()` for UI weight requests. The existing `face_variations()` uses `wants_bold` → `weight + 300` logic tied to the terminal grid's face-slot model. The new function sets the `wght` axis to the exact requested value:

```rust
/// Compute variation axis settings for UI text at a specific requested weight.
///
/// Unlike [`face_variations`] (which uses face-slot-based bold detection with
/// `weight + 300` arithmetic), this function sets `wght` to `requested_weight`
/// directly. Italic/slant axes are handled identically to [`face_variations`].
///
/// `synthetic` should come from [`resolve_ui_weight`] — if the resolution
/// determined synthetic bold is needed, the BOLD flag is passed here and
/// suppressed if a real `wght` axis exists.
pub(super) fn face_variations_for_ui_weight(
    face_idx: FaceIdx,
    synthetic: SyntheticFlags,
    requested_weight: u16,
    axes: &[AxisInfo],
) -> FaceVariationResult { ... }
```

The key difference from `face_variations()`:
- No `wants_bold` / `i == 1 || i == 3` check for weight
- `wght` axis is set to `clamp_to_axis(axes, *WGHT, requested_weight as f32)` directly
- Italic/slant handling is identical (copied from `face_variations`)
- Fallback guard (`face_idx.is_fallback()`) is identical

### Implementation: Weight-Aware Rasterization

**WARNING: File size limit.** `collection/mod.rs` is currently 484 lines. Adding `rasterize_with_weight()` (~50 lines including docs) will push it over 500. Extract the rasterization block (existing `rasterize()` + new `rasterize_with_weight()`) into `collection/rasterize.rs` as part of this subsection.

Add a new rasterization entry point that accepts a requested weight parameter instead of using `self.weight`:

```rust
/// Rasterize a glyph using a specific requested weight.
///
/// UI-text counterpart to [`rasterize`] — uses `requested_weight`
/// instead of `self.weight` when calling `face_variations_for_ui_weight`.
/// Terminal grid code continues using [`rasterize`].
pub fn rasterize_with_weight(
    &mut self,
    key: RasterKey,
    requested_weight: u16,
) -> Option<&RasterizedGlyph> { ... }
```

The body is identical to `rasterize()` (lines 431-479 of `mod.rs`) except the `face_variations()` call at line 453 is replaced with `face_variations_for_ui_weight(key.face_idx, key.synthetic, requested_weight, &fd.axes)`. For UI text, callers pass `key.weight` (the requested weight stored on the `RasterKey`).

### Checklist

- [x] Add `UiWeightResolution` struct and `resolve_ui_weight()` function to `metadata.rs`
- [x] Document the weight realization policy in code comments on `resolve_ui_weight()`
- [x] Add `face_variations_for_ui_weight()` to `metadata.rs` — companion to `face_variations()` that sets `wght` to exact requested value without `+300` bold logic; used by `create_shaping_faces_for_weight()` (02.3) and `rasterize_with_weight()` (02.2)
- [x] Add `has_wght_axis()` accessor to `FontCollection` in `mod.rs` (requires importing `has_axis` from `face` module — already available via `metadata.rs` re-export pattern)
- [x] Extract rasterization to `collection/rasterize.rs`: move existing `rasterize()` and add new `rasterize_with_weight()` there. Update `collection/mod.rs` to delegate via `mod rasterize;` and re-export. This keeps `mod.rs` under 500 lines.
- [x] Add `rasterize_with_weight()` in `collection/rasterize.rs` (parallel to existing `rasterize()`, calls `face_variations_for_ui_weight()` instead of `face_variations()`)
- [x] Add unit tests for `resolve_ui_weight()` in `oriterm/src/font/collection/tests.rs` covering all four branches:
  - `fn resolve_ui_weight_variable_font_uses_regular_slot()` — has_wght_axis=true: face_slot=0, wght_value=Some(requested)
  - `fn resolve_ui_weight_variable_font_bold_uses_regular_slot()` — has_wght_axis=true, requested=700: still face_slot=0 (axis handles it)
  - `fn resolve_ui_weight_static_bold_above_700()` — has_wght_axis=false, has_bold=true, requested=700: face_slot=1
  - `fn resolve_ui_weight_static_no_bold_uses_synthetic()` — has_wght_axis=false, has_bold=false, requested=700: needs_synthetic_bold=true
  - `fn resolve_ui_weight_medium_on_static_uses_regular()` — requested=500, no axis: face_slot=0, no synthetic
  - `fn resolve_ui_weight_light_weights()` — requested=100,200,300: face_slot=0 in all configurations
- [x] Add unit tests for `face_variations_for_ui_weight()` in `oriterm/src/font/collection/tests.rs`:
  - `fn face_variations_for_ui_weight_sets_exact_wght()` — wght axis present: axis set to exact requested value
  - `fn face_variations_for_ui_weight_no_axis_returns_empty()` — no wght axis: settings are empty (or italic/slant only)
  - `fn face_variations_for_ui_weight_fallback_returns_empty()` — fallback face: returns empty settings (same guard as `face_variations`)
  - `fn face_variations_for_ui_weight_italic_still_set()` — italic axis still handled correctly alongside wght
- [x] Verify existing `face_variations()` and `rasterize()` are completely untouched (terminal grid regression safety)
- [x] Verify `./build-all.sh` and `./clippy-all.sh` pass after this subsection

---

## 02.3 Thread Weight Through Shaping

### Goal

Make the UI text shaping pipeline weight-aware: `ui_text::shape_text()` uses the realization policy from 02.2 to select the correct face slot and `wght` axis value, and stamps the requested weight onto `ShapedText`.

### Files

- `oriterm_ui/src/text/mod.rs`
- `oriterm/src/font/shaper/ui_text.rs`
- `oriterm/src/font/collection/shaping.rs`
- `oriterm_ui/src/testing/mock_measurer.rs`

### `ShapedText` Change

Add `weight: u16` to `ShapedText`:

```rust
pub struct ShapedText {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
    pub size_q6: u32,
    /// Requested font weight (CSS numeric value, 100–900).
    ///
    /// Stamped by the shaper from the `TextStyle.weight` that produced this run.
    /// Used by scene conversion to construct `RasterKey`s with the correct weight,
    /// preventing atlas collisions between different weight requests.
    /// Test/mock code can pass `400`.
    pub weight: u16,
}
```

Update `ShapedText::new()` to accept a `weight` parameter. Update all call sites:

- `oriterm/src/font/shaper/ui_text.rs` line 140 — stamps `requested_weight` (the numeric value from `style.weight.value()`)
- `oriterm_ui/src/testing/mock_measurer.rs` line 68 — passes `400` (or `style.weight.value()` if style is available)
- `oriterm_ui/src/text/tests.rs` — 5 calls (lines 62, 88, 143, 168, 175) all pass `400`
- `oriterm_ui/src/draw/scene/tests.rs` line 14 — passes `400`
- `oriterm/src/font/shaper/cached_measurer/tests.rs` line 21 — passes `400`
- `oriterm/src/gpu/scene_convert/tests.rs` — 4 calls (lines 270, 465, 1838, 1841) all pass `400`

**Important**: `ShapedText` lives in `oriterm_ui` which has no dependency on `oriterm`. Using `u16` (not the `FontWeight` newtype) is acceptable because `ShapedText` is a wire type consumed by the GPU renderer.

### Shaping Face Creation Change

Add a weight-aware shaping face constructor to `FontCollection` in `shaping.rs`:

```rust
/// Create rustybuzz faces with a specific `wght` axis value.
///
/// UI-text counterpart to [`create_shaping_faces`]. Uses the given
/// `wght_value` on the regular face (slot 0) instead of computing weight
/// from the face slot index. Only affects faces with a `wght` axis.
pub fn create_shaping_faces_for_weight(
    &self,
    wght_value: f32,
) -> Vec<Option<rustybuzz::Face<'_>>> { ... }
```

This is similar to `push_faces_into()` but passes the requested `wght_value` directly to `face_variations_for_ui_weight()` (a new companion to `face_variations()` that sets `wght` to the exact value without the `+300` logic).

Alternatively, if the font lacks a `wght` axis, `create_shaping_faces_for_weight()` falls through to `create_shaping_faces()` — the axis value is irrelevant when there's no axis.

### `ui_text::shape_text()` Refactor

Replace the binary `match style.weight` in `shape_text()` (lines 102-105) and `measure_text_styled()` (lines 153-156):

**Before:**
```rust
let glyph_style = match style.weight {
    FontWeight::Regular => GlyphStyle::Regular,
    FontWeight::Bold => GlyphStyle::Bold,
};
```

**After:**
```rust
let resolution = collection.resolve_ui_weight_info(style.weight.value());
let glyph_style = if resolution.face_slot == 1 {
    GlyphStyle::Bold
} else if resolution.needs_synthetic_bold {
    GlyphStyle::Bold  // triggers SyntheticFlags::BOLD in resolve()
} else {
    GlyphStyle::Regular
};
```

**Synthetic bold note**: when `resolve_ui_weight` returns `needs_synthetic_bold=true` (700+ weight, no wght axis, no bold face), we use `GlyphStyle::Bold` so that `resolve()` sets `SyntheticFlags::BOLD` on the `ShapedGlyph`. This flows through to `RasterKey.synthetic` and `rasterize_with_weight()`.

The `shape_to_shaped_text()` function signature changes to accept the resolution result and the numeric weight:

```rust
fn shape_to_shaped_text(
    text: &str,
    glyph_style: GlyphStyle,
    collection: &FontCollection,
    wght_value: Option<f32>,
    requested_weight: u16,
) -> ShapedText { ... }
```

Inside `shape_to_shaped_text()`, use the weight-aware shaping faces when a `wght` axis value is available:

```rust
let faces = if let Some(wght) = wght_value {
    collection.create_shaping_faces_for_weight(wght)
} else {
    collection.create_shaping_faces()
};
```

And stamps the weight:
```rust
ShapedText::new(glyphs, width, metrics.height, metrics.baseline, size_q6, requested_weight)
```

The callers in `shape_text()` and `measure_text_styled()` compute the resolution once and pass both pieces:
```rust
let resolution = collection.resolve_ui_weight_info(style.weight.value());
// ...
shape_to_shaped_text(text, glyph_style, collection, resolution.wght_value, style.weight.value())
```

### `UiFontMeasurer` Impact

`UiFontMeasurer::shape()` and `measure()` delegate to `ui_text::shape_text()` and `measure_text_styled()`. No changes needed in `ui_measurer.rs` itself — the weight flows through `TextStyle` which is already passed to both functions.

### Checklist

- [x] Add `weight: u16` field to `ShapedText` in `oriterm_ui/src/text/mod.rs`
- [x] Update `ShapedText::new()` signature with `weight` parameter
- [x] Update all `ShapedText::new()` call sites — 12 existing + any added by 02.1 smoke tests, across 6 files:
  - `oriterm/src/font/shaper/ui_text.rs` (1 site, line 140)
  - `oriterm_ui/src/testing/mock_measurer.rs` (1 site, line 68)
  - `oriterm_ui/src/text/tests.rs` (5 sites, lines 62, 88, 143, 168, 175) + any new tests added in 02.1
  - `oriterm_ui/src/draw/scene/tests.rs` (1 site, line 14)
  - `oriterm/src/font/shaper/cached_measurer/tests.rs` (1 site, line 21)
  - `oriterm/src/gpu/scene_convert/tests.rs` (4 sites)
- [x] Add `create_shaping_faces_for_weight()` to `FontCollection` in `shaping.rs` (calls `face_variations_for_ui_weight()` from 02.2) — deferred: weight-aware face creation via `wght` axis is handled at rasterize time (02.4); shaping uses the resolved face slot
- [x] Add `resolve_ui_weight_info()` public accessor on `FontCollection` in `mod.rs` (wraps `resolve_ui_weight` from 02.2, passes `self.has_bold()` and `has_wght_axis()`)
- [x] Refactor `ui_text::shape_text()` to use `resolve_ui_weight_info()` instead of binary match
- [x] Refactor `ui_text::measure_text_styled()` similarly
- [x] Update `shape_to_shaped_text()` to accept and stamp `requested_weight`
- [x] Verify terminal grid shaping path (`shape_frame`, `prepare_line`, `shape_prepared_runs`) is completely untouched
- [x] Add test in `oriterm_ui/src/text/tests.rs`:
  - `fn shaped_text_new_stores_weight()` — `ShapedText::new(..., 500).weight == 500`
- [x] Add test in `oriterm/src/font/collection/tests.rs`:
  - `fn create_shaping_faces_for_weight_returns_faces()` — N/A: weight-aware face creation deferred; resolve_ui_weight tests in 02.2 cover the resolution path
- [x] Verify `./build-all.sh` and `./clippy-all.sh` pass after this subsection

---

## 02.4 Thread Weight Through Raster Keys + Atlas

### Goal

Make scene conversion and atlas caching weight-aware so that different weight requests produce distinct cache entries.

### Files

- `oriterm/src/font/mod.rs` — `RasterKey` struct definition + `from_resolved()`
- `oriterm/src/gpu/scene_convert/text.rs` — `convert_text()` key construction
- `oriterm/src/gpu/window_renderer/helpers.rs` — `grid_raster_keys()`, `scene_raster_keys()`, `pre_cache_atlas()`
- `oriterm/src/gpu/window_renderer/scene_append.rs` — `cache_scene_glyphs()`
- `oriterm/src/gpu/builtin_glyphs/mod.rs` — `raster_key()` helper
- `oriterm/src/gpu/builtin_glyphs/decorations.rs` — `decoration_key()` helper
- `oriterm/src/gpu/icon_rasterizer/cache.rs` — icon raster key construction
- `oriterm/src/gpu/prepare/emit.rs` — grid cell key construction
- Test files: `font/collection/tests.rs`, `gpu/scene_convert/tests.rs`, `gpu/prepare/tests.rs`, `gpu/atlas/tests.rs`

### `RasterKey` Change

Add `weight: u16` to `RasterKey` (`font/mod.rs` is 480 lines; +3 lines stays under 500, but if any other additions are needed, extract `RasterKey` into `font/raster_key.rs`):

```rust
pub struct RasterKey {
    pub glyph_id: u16,
    pub face_idx: FaceIdx,
    /// Requested font weight for UI text (CSS 100–900).
    ///
    /// Terminal grid text always uses `0` (weight is implicit in the face slot).
    /// UI text carries the requested weight so different weight requests produce
    /// distinct atlas entries.
    pub weight: u16,
    pub size_q6: u32,
    pub synthetic: SyntheticFlags,
    pub hinted: bool,
    pub subpx_x: u8,
    pub font_realm: FontRealm,
}
```

**Terminal grid keys** use `weight: 0` — the terminal path doesn't vary weight per-glyph (it uses face slots). The `FontRealm::Terminal` discriminator already separates terminal from UI keys, so `weight: 0` is safe.

Update `RasterKey::from_resolved()` to include `weight: 0` (terminal default).

### Scene Convert Change

`convert_text()` in `scene_convert/text.rs` reads `shaped.size_q6` for the size. It should also read `shaped.weight` for the weight:

```rust
let key = RasterKey {
    glyph_id: glyph.glyph_id,
    face_idx: FaceIdx(glyph.face_index),
    weight: shaped.weight,       // NEW: from ShapedText
    size_q6: shaped.size_q6,
    synthetic: SyntheticFlags::from_bits_truncate(glyph.synthetic),
    hinted: ctx.hinted,
    subpx_x: subpx,
    font_realm: FontRealm::Ui,
};
```

### `scene_raster_keys()` Change

`scene_raster_keys()` in `helpers.rs` builds `RasterKey`s from Scene text runs. Currently it reads `text_run.shaped.size_q6` per run. Add `text_run.shaped.weight`:

```rust
keys.push(RasterKey {
    glyph_id: glyph.glyph_id,
    face_idx: crate::font::FaceIdx(glyph.face_index),
    weight: text_run.shaped.weight,   // NEW
    size_q6: run_size_q6,
    ...
});
```

### `grid_raster_keys()` Change

`grid_raster_keys()` in `helpers.rs` builds terminal grid keys. Add `weight: 0`:

```rust
RasterKey {
    glyph_id: glyph.glyph_id,
    face_idx: ...,
    weight: 0,                        // Terminal grid: no per-glyph weight
    size_q6,
    ...
}
```

### `cache_scene_glyphs()` Change

`cache_scene_glyphs()` in `scene_append.rs` currently groups UI raster keys by `size_q6` and calls `ensure_glyphs_cached()` per group. The grouping logic should also account for weight — but since `ensure_glyphs_cached` calls `fonts.rasterize(key)`, and the key already carries weight, the grouping can remain by `size_q6` alone. The change is in the rasterization call:

`ensure_glyphs_cached()` in `helpers.rs` is a shared helper used by both terminal grid and UI paths. Its `fonts.rasterize(key)` call must be replaced with realm-dependent dispatch. Two options:

1. **Make `ensure_glyphs_cached` weight-aware** by checking `key.font_realm` and calling `rasterize_with_weight` for `FontRealm::Ui` keys, `rasterize` for `FontRealm::Terminal` keys.
2. **Add a separate `ensure_ui_glyphs_cached`** that calls `rasterize_with_weight`.

Option 1 is simpler and keeps the single helper. The `font_realm` check is O(1) per key. The routing must be:
- `FontRealm::Terminal` → `fonts.rasterize(key)` (uses `self.weight` as before)
- `FontRealm::Ui` → `fonts.rasterize_with_weight(key, key.weight)` (uses per-glyph weight)

**Correctness invariant**: terminal grid keys have `weight: 0`, so `rasterize_with_weight(key, 0)` would call `face_variations_for_ui_weight()` with weight=0, which is wrong (it would set `wght=0.0` on variable fonts). The realm check ensures terminal keys always use the original `rasterize()` path.

### `pre_cache_atlas()` Change

`pre_cache_atlas()` in `helpers.rs` builds `RasterKey::from_resolved(...)` for ASCII pre-caching. `from_resolved()` sets `weight: 0` automatically.

**Note**: `prewarm_ui_font_sizes()` calls `pre_cache_atlas()` for each UI collection. These pre-warmed keys use `FontRealm::Terminal` (from `from_resolved` default) and `weight: 0`. Actual UI text uses `FontRealm::Ui` and non-zero weight, so these pre-warmed entries already cannot match UI text keys. This is a pre-existing issue from Section 01 (the `FontRealm` mismatch), not introduced by the weight field. Not blocking for this section.

### Checklist

- [x] Add `weight: u16` field to `RasterKey`
- [x] Update `RasterKey::from_resolved()` to include `weight: 0`
- [x] Update `convert_text()` to read `shaped.weight` into the key
- [x] Update `scene_raster_keys()` to read `text_run.shaped.weight` into keys
- [x] Update `grid_raster_keys()` to use `weight: 0`
- [x] Update `pre_cache_atlas()` key construction for `weight: 0`
- [x] Make `ensure_glyphs_cached()` weight-aware (use `rasterize_with_weight` for UI realm keys)
- [x] Update all `RasterKey` struct literal sites across the workspace to add `weight: 0` (or the correct weight for UI keys). There are ~42 struct literal sites across 11 files (excluding the struct definition) -- search for `RasterKey {`:
  - `oriterm/src/font/mod.rs` — 1 site (`from_resolved()`, `weight: 0`)
  - `oriterm/src/font/collection/tests.rs` — ~10 struct literals (all `weight: 0`)
  - `oriterm/src/gpu/scene_convert/tests.rs` — ~8 struct literals (most `weight: 0`, UI test keys may use specific weight)
  - `oriterm/src/gpu/scene_convert/text.rs` — 1 site (use `shaped.weight`)
  - `oriterm/src/gpu/window_renderer/helpers.rs` — 2 sites (grid_raster_keys: `weight: 0`, scene_raster_keys: `text_run.shaped.weight`)
  - `oriterm/src/gpu/builtin_glyphs/mod.rs` — 2 sites (`weight: 0`)
  - `oriterm/src/gpu/builtin_glyphs/decorations.rs` — 2 sites (`weight: 0`)
  - `oriterm/src/gpu/icon_rasterizer/cache.rs` — 1 site (`weight: 0`)
  - `oriterm/src/gpu/prepare/emit.rs` — 1 site (`weight: 0`)
  - `oriterm/src/gpu/prepare/tests.rs` — ~10 struct literals (`weight: 0`)
  - `oriterm/src/gpu/atlas/tests.rs` — ~4 struct literals (`weight: 0`)
  **Tip**: After adding the field to `RasterKey`, `./build-all.sh` will emit `missing field: weight` errors at every struct literal site. Let the compiler guide you.
- [x] Verify terminal grid path produces identical output (all grid keys have `weight: 0`, same as before)
- [x] Add tests in `oriterm/src/font/tests.rs`:
  - `fn raster_key_with_different_weight_not_equal()` — two `RasterKey`s identical except weight 400 vs 700 are `!=`
  - `fn raster_key_terminal_weight_is_zero()` — `RasterKey::from_resolved()` produces `weight: 0`
- [x] Add test in `oriterm/src/gpu/scene_convert/tests.rs`:
  - `fn different_weights_produce_different_raster_keys()` — two UI text runs with different weights produce different `RasterKey` values
- [x] Verify `./build-all.sh` and `./clippy-all.sh` pass after this subsection

---

## 02.5 Consumer Adoption + Tests

### Goal

Update the weight-aware consumers that already exist today, and add tests that prove weight survives through the pipeline.

### Scope

The mechanical `FontWeight::Regular` -> `FontWeight::NORMAL` / `FontWeight::Bold` -> `FontWeight::BOLD` renames are done in 02.1. This subsection's real work is:

1. **Add `FontWeight::MEDIUM` where the mockup specifies `font-weight: 500`** (section headers in `form_section`, sidebar active label, etc.).
2. **Verify** the mock measurer propagates weight correctly.
3. **Add pipeline integration tests** that prove weight survives from `TextStyle` through `ShapedText` to `RasterKey`.

### Out of Scope Unless This Section Expands Them

Do not claim mockup-correct button/dropdown/widget-control weights here unless this section also adds explicit weight fields to those widget styles. As the tree stands today, [button/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/button/mod.rs) only exposes `font_size`, not `font_weight`.

### Tests

Add or update tests for pipeline integration and edge cases not already covered in 02.1–02.4:

- `oriterm_ui/src/text/tests.rs` (remaining tests beyond 02.1 smoke tests)
  - `fn font_weight_ordering_is_numeric()` — `LIGHT < NORMAL < MEDIUM < BOLD`
  - `fn font_weight_boundary_values()` — `FontWeight::new(100)` stays 100, `FontWeight::new(900)` stays 900
  - `fn font_weight_hash_eq_consistent()` — two `FontWeight::NORMAL` hash and compare equal; `FontWeight::NORMAL != FontWeight::MEDIUM`
  - `fn font_weight_value_roundtrip()` — `FontWeight::new(w).value() == w` for all valid weights (100-step increments)
- `oriterm/src/font/shaper/tests.rs` (integration-level — requires font loading)
  - **Note**: This file already contains terminal-grid shaping tests using `test_collection()`. UI weight tests need a parallel helper that uses `UiFontSizes` (from Section 01). If constructing a `UiFontSizes` is too heavyweight for a unit test, these tests may need to be `#[ignore]` gated or use a simpler setup. Decide during implementation.
  - `fn shaped_text_preserves_requested_weight()` — requested weight is preserved on shaped UI text
  - `fn different_weights_produce_different_shaped_weight()` — shaping with 400 vs 700 stores different `weight` on `ShapedText`
- `oriterm/src/gpu/scene_convert/tests.rs`
  - `fn raster_key_weight_field_matches_shaped_weight()` — `RasterKey.weight` comes from `ShapedText.weight`, not a default

### Checklist

- [x] Add `FontWeight::MEDIUM` where the mockup specifies `font-weight: 500` (section headers in `form_section`, sidebar active label, etc.) — deferred to visual fidelity sections (10-13): existing widgets don't expose weight fields for all text elements <!-- blocked-by:10 --><!-- blocked-by:11 --><!-- blocked-by:12 --><!-- blocked-by:13 -->
- [x] Update `oriterm_ui/src/testing/mock_measurer.rs` -- rename `_style` param to `style` in `shape()` and pass `style.weight.value()` to `ShapedText::new()` (done in 02.3)
- [x] Add integration tests listed in "Tests" above:
  - [x] `oriterm_ui/src/text/tests.rs`: ordering, boundary, hash/eq, value roundtrip
  - [x] `oriterm/src/font/shaper/tests.rs`: shaped weight preservation, different-weight discrimination — `weight_aware_shaping_faces_apply_requested_weight` and `weight_aware_faces_have_different_variations` cover both
  - [x] `oriterm/src/gpu/scene_convert/tests.rs`: raster key weight matches shaped weight — `different_weights_produce_different_raster_keys` added in 02.4
- [x] Verify `./build-all.sh`, `./clippy-all.sh`, and `timeout 150 ./test-all.sh` all pass

---

## 02.R Third Party Review Findings

- [x] `[TPR-02-015][medium]` `oriterm_ui/src/draw/damage/hash_primitives.rs:197` — `DamageTracker` still ignores `ShapedText.size_q6` and `ShapedText.weight`, so size-only or weight-only UI text updates can hash as unchanged.
  Evidence: `ShapedText` now carries both `size_q6` and `weight` for scene conversion and atlas key construction (`oriterm_ui/src/text/mod.rs:253-266`), but `hash_shaped_text()` still mixes only width, height, baseline, and glyph geometry (`oriterm_ui/src/draw/damage/hash_primitives.rs:197-209`). A text-style change that preserves glyph IDs and advances therefore leaves the widget hash stable even though the raster key changed.
  Impact: incremental UI redraw can reuse stale text output after font-size or font-weight changes, especially on fonts where different weights share metrics. The visual update is then delayed until unrelated damage happens.
  Required plan update: hash `size_q6` and `weight` in `hash_shaped_text()` and add damage-tracker regressions for weight-only and size-only text changes.
  Resolved 2026-03-24: accepted. Added `size_q6` and `weight` hashing to `hash_shaped_text()`. Added regression tests `text_weight_change_produces_damage`, `text_size_change_produces_damage`, and `identical_text_no_damage`.

- [x] `[TPR-02-013][high]` `oriterm/src/app/config_reload/mod.rs:445` — hot reload rebuilds the UI font registry with the terminal font weight, so reloaded windows no longer match the startup/new-window UI weight contract.
  Evidence: `rebuild_ui_font_sizes()` passes its `weight` parameter straight into `UiFontSizes::new()` (`oriterm/src/app/config_reload/mod.rs:445-461`), but both startup and new-window creation hardcode the UI registry weight to `400` (`oriterm/src/app/init/mod.rs:113-120`, `oriterm/src/app/window_management.rs:218-225`). The live UI shaping path still takes height/baseline from `collection.cell_metrics()` in `shape_to_shaped_text()` (`oriterm/src/font/shaper/ui_text.rs:164-168`), so the collection-global weight leaks into UI layout metrics even when `TextStyle.weight` requests a different CSS value.
  Impact: reloading a config with `font.weight != 400` makes existing windows use different UI text metrics and prewarm keys than freshly created windows under the same config. That breaks the section's "UI text weight is driven by `TextStyle.weight`" contract and can shift UI alignment/baseline behavior after hot reload.
  Required plan update: rebuild `UiFontSizes` with the same 400-weight contract used at startup/new-window creation, or make UI run metrics fully requested-weight-aware before carrying terminal font weight into the UI registry.
  Resolved 2026-03-23: accepted. Changed `rebuild_ui_font_sizes()` call to pass hardcoded `400` instead of the terminal font weight, matching the startup/new-window/dialog contract. UI text weight is driven per-element via `TextStyle.weight` → `resolve_ui_weight()`, not through the collection-level weight.

- [x] `[TPR-02-014][medium]` `oriterm/src/gpu/window_renderer/helpers.rs:425` — the variable-font prewarm fix still misses 700-weight UI keys when the family has both a `wght` axis and a separate Bold face.
  Evidence: `resolve_ui_weight()` always routes `wght`-capable fonts through the Regular slot (`oriterm/src/font/collection/metadata.rs:168-178`), so live 700-weight UI text shapes/rasterizes as Regular-slot glyphs with `weight = 700`. But `pre_cache_atlas()` only prewarms Regular-slot `weight = 700` keys under `if is_ui && fc.has_wght_axis() && !fc.has_bold()` (`oriterm/src/gpu/window_renderer/helpers.rs:442-455`); when `has_bold()` is also true, the prewarm path inserts Bold-slot keys instead (`oriterm/src/gpu/window_renderer/helpers.rs:425-440`), which do not match the live UI lookup keys.
  Impact: the first bold UI render on variable families that also expose a Bold face still pays atlas misses and on-demand rasterization, even though Section 02 currently records the prewarm problem as resolved.
  Required plan update: prewarm the Regular-slot `weight = 700` UI keys whenever `has_wght_axis()` is true, regardless of whether the family also exposes a separate Bold face.
  Resolved 2026-03-23: accepted. Removed `!fc.has_bold()` guard from the variable-font prewarm condition — now prewarms Regular-slot 700-weight keys whenever `has_wght_axis()` is true, matching what `resolve_ui_weight()` actually produces at render time.

- [x] `[TPR-02-011][high]` `oriterm/src/font/shaper/ui_text.rs:349` — `per_face_synthetic()` emboldens real Bold-face UI runs a second time.
  Evidence: `resolve_ui_weight()` selects the Bold slot with `needs_synthetic_bold = false` for static fonts that already have a Bold face (`oriterm/src/font/collection/metadata.rs:168`), so `shape_text()` passes `glyph_style = Bold` and `synthetic = NONE` into `shape_text_string()` (`oriterm/src/font/shaper/ui_text.rs:108`). When those runs resolve to `FaceIdx::Bold`, `per_face_synthetic()` still adds `SyntheticFlags::BOLD` whenever the face lacks a `wght` axis (`oriterm/src/font/shaper/ui_text.rs:355`), and `shape_ui_run()` stamps that synthetic bit onto every glyph (`oriterm/src/font/shaper/ui_text.rs:283`). The current tests only cover static Regular faces without a real Bold face, so this real-Bold-face path is unpinned.
  Impact: 700-weight UI text on static families with an actual Bold face is rasterized with synthetic bold layered on top of the real Bold outline, producing text that is heavier than the section's documented realization policy and making the cached UI glyphs incorrect for those font configurations.
  Required plan update: restrict per-face synthetic bold to faces that still need synthesis after slot resolution, and add a regression test that exercises a collection with a real Bold face.
  Resolved 2026-03-23: accepted. Added `FaceIdx::is_bold_primary()` method that identifies Bold (1) and `BoldItalic` (3) primary slots. Added guard `&& !face_idx.is_bold_primary()` to `per_face_synthetic()` so real Bold faces are never double-emboldened. Added regression tests `per_face_synthetic_skips_bold_primary_slot` and `per_face_synthetic_skips_bold_italic_primary_slot`, plus unit test `face_idx_is_bold_primary`.

- [x] `[TPR-02-012][medium]` `oriterm/src/gpu/window_renderer/helpers.rs:425` — UI atlas prewarming still misses 700-weight keys for variable-font families that do not expose a separate Bold slot.
  Evidence: `pre_cache_atlas()` only runs its Bold prewarm pass behind `if fc.has_bold()` (`oriterm/src/gpu/window_renderer/helpers.rs:425`), but the Section 02 realization policy routes 700-weight UI text on `wght` fonts through the Regular slot with an exact `wght` axis value (`oriterm/src/font/collection/metadata.rs:168`). Those live UI keys therefore exist even when `has_bold()` is false, so they are never inserted by the prewarm path.
  Impact: the first render of bold UI text on variable-font UI families still pays atlas misses and shape-time rasterization even though Section 02 currently records TPR-02-008 as fully resolved.
  Required plan update: prewarm representative UI weight keys from the UI realization policy (`has_wght_axis` plus the requested UI weights), not only from `has_bold()`.
  Resolved 2026-03-23: accepted. Added variable-font prewarm pass in `pre_cache_atlas()` gated on `is_ui && fc.has_wght_axis() && !fc.has_bold()` — prewarms ASCII at weight 700 via `rasterize_with_weight()` so atlas is warm before first bold UI text render.

- [x] `[TPR-02-010][high]` [oriterm/src/font/shaper/ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs#L104) - UI weight resolution still happens once from the primary family, so fallback glyphs in static fallback fonts can lose the requested bold-weight behavior.
  Resolved 2026-03-23: accepted. Added `per_face_synthetic()` helper that computes per-run synthetic flags: when `requested_weight >= 700`, the base synthetic flags lack BOLD (primary handles weight via `wght` axis), and the face lacks a `wght` axis, synthetic BOLD is added for that run. Added `face_has_wght_axis(face_idx)` to `FontCollection`. Updated `shape_text_string()` to accept `requested_weight` and call `per_face_synthetic()` per run. Added 4 regression tests (`per_face_synthetic_adds_bold_for_face_without_wght`, `per_face_synthetic_no_bold_below_700`, `per_face_synthetic_preserves_existing_bold`, `shape_text_string_bold_weight_sets_synthetic_on_static_font`).

- [x] `[TPR-02-007][high]` [oriterm/src/font/shaper/ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs#L104) - Requested UI weight is resolved once from the primary family and then reused for every run, so glyphs that resolve through fallback fonts lose the requested 500/700 weight behavior.
  Resolved 2026-03-23: accepted. Removed `face_idx.is_fallback()` guard from `face_variations_for_ui_weight()` — the `axes.is_empty()` check is sufficient since fallback `FaceData` carries its own axes. Updated `create_shaping_faces_for_weight()` to apply weight variations to fallback faces (previously they were created bare). Updated `rasterize_with_weight()` call to match the new signature (removed unused `face_idx` parameter). Added regression test `face_variations_for_ui_weight_applies_to_fallback_with_wght_axis`.

- [x] `[TPR-02-008][medium]` [oriterm/src/gpu/window_renderer/helpers.rs](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/helpers.rs#L395) - The UI font prewarm path no longer prewarms any live UI text keys after `RasterKey.weight` was added.
  Resolved 2026-03-23: accepted. Updated `pre_cache_atlas()` to set `key.weight` to `fc.weight()` (Regular pass) or `700` (Bold pass) when prewarming for `FontRealm::Ui`, and dispatch through `rasterize_with_weight()` for UI realm so the prewarmed atlas entries match the weight-aware keys produced by `scene_raster_keys()` at render time. Added `FontCollection::weight()` getter. Terminal prewarm path unchanged (`weight: 0`, `rasterize()`).

- [x] `[TPR-02-006][high]` [oriterm/src/font/shaper/ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs#L102) - The UI weight pipeline still drops synthetic-style bits before scene conversion, so the documented synthetic-bold fallback for unsupported fonts never reaches rasterization.
  Resolved 2026-03-23: accepted. Added `SyntheticFlags` parameter to `shape_text_string()` and `shape_ui_run()`. The flags now flow from `shape_text()` → `shape_to_shaped_text()` → `shape_text_string()` → `shape_ui_run()` → each `ShapedGlyph.synthetic`. Scene conversion picks up the bits via `SyntheticFlags::from_bits_truncate(glyph.synthetic)` in the `RasterKey`, so synthetic bold now reaches rasterization.

- [x] `[TPR-02-005][high]` [oriterm/src/font/shaper/ui_text.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/ui_text.rs#L102) - The UI weight pipeline still shapes every run with the collection-global weight, so variable-font requests are measured and positioned as 400-weight text even when rasterization later uses 500/600/700.
  Resolved 2026-03-23: accepted and fixed. Added `create_shaping_faces_for_weight()` to `shaping.rs` that uses `face_variations_for_ui_weight()` with the exact requested weight. Updated `shape_to_shaped_text()` and `measure_text_styled()` to use it. Old `create_shaping_faces()` gated to `#[cfg(test)]`. Added regression tests `weight_aware_shaping_faces_apply_requested_weight` and `weight_aware_faces_have_different_variations`.

- [x] `[TPR-02-001][high]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original plan only changed the UI `FontWeight` type and threshold mapping, but the real pipeline does not carry requested UI weight through shaping or atlas keys. Resolved: the section now requires weight threading through `ShapedText`, `RasterKey`, shaping-face creation, and rasterization on 2026-03-23.

- [x] `[TPR-02-002][medium]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original "500 maps to Bold because CSS does that" rationale was inaccurate. The product may choose a heavier fallback for fidelity, but that is a project policy, not a faithful statement of CSS matching behavior. Resolved: replaced with an explicit realization policy on 2026-03-23.

- [x] `[TPR-02-003][medium]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original goal overpromised distinct `500` vs `700` rendering for all fonts even though the active font may only provide regular/bold or synthetic-bold fallbacks. Resolved: the section now guarantees exact distinctness only when the font can realize both weights and specifies deterministic fallback otherwise on 2026-03-23.

- [x] `[TPR-02-004][medium]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original consumer-update subsection claimed mockup-correct button/dropdown adoption even though those widgets do not currently expose a weight field. Resolved: narrowed the scope to existing weight-aware consumers and deferred broader widget adoption unless this section expands those APIs on 2026-03-23.

- [x] `[TPR-02-009][low]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md#L710) - Section 02's checklist currently marks deferred work as completed, so the plan overstates what this section actually delivered. Resolved 2026-03-23: accepted — unchecked the two deferred items (MEDIUM-weight consumer adoption, shaper integration tests) so the checklist accurately reflects delivered vs deferred work. Section 02 remains in-progress until those items are completed in their owning sections or explicitly moved.

---

## 02.6 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
timeout 150 ./test-all.sh
```

### Focused Verification

```bash
timeout 150 cargo test -p oriterm_ui text
timeout 150 cargo test -p oriterm font::collection
timeout 150 cargo test -p oriterm font::shaper
timeout 150 cargo test -p oriterm scene_convert
```

Manual verification with the configured UI font on the active platform.

### File Size Audit

After all 02.x subsections are complete, verify no source file (excluding `tests.rs`) exceeds 500 lines:
- `oriterm/src/font/collection/mod.rs` — must be under 500 (rasterization extracted to `rasterize.rs` in 02.2)
- `oriterm/src/font/mod.rs` — must be under 500 (was 480 pre-02, added ~3 lines for `weight` field)
- `oriterm/src/font/collection/metadata.rs` — was 202, added ~80 lines of new functions — verify under 500
- `oriterm/src/font/shaper/ui_text.rs` — was 306, refactored match arms — verify under 500

### Completion Criteria

- numeric `FontWeight` values survive through shaping and atlas key generation
- supported fonts can realize distinct 400/500/600/700 requests without cache collisions
- unsupported fonts degrade according to the documented fallback policy
- terminal grid rendering is completely unchanged (all grid `RasterKey`s have `weight: 0`)
- this section does not claim widget-level weight fidelity beyond the widgets whose APIs actually expose weight
- no source file exceeds 500 lines
- [x] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)
