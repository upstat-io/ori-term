---
section: "02"
title: "Numeric Font Weight System"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "UI text can request CSS-style numeric font weights, and the request survives shaping and atlas caching so supported fonts can render 400/500/600/700 distinctly with deterministic fallback when they cannot"
inspired_by:
  - "CSS font-weight: 100-900 (https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight)"
  - "GPUI font-weight threading through text shaping (~/projects/reference_repos/gui_repos/zed/crates/gpui/src/text_system.rs)"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "Numeric FontWeight API"
    status: not-started
  - id: "02.2"
    title: "Thread Weight Through Shaping and Raster Keys"
    status: not-started
  - id: "02.3"
    title: "Weight Realization Policy"
    status: not-started
  - id: "02.4"
    title: "Consumer Adoption + Tests"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.5"
    title: "Build & Verify"
    status: not-started
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
    -> match Regular|Bold -> GlyphStyle
    -> FontCollection::create_shaping_faces() uses collection-global weight only
  -> scene_convert::convert_text()
    -> RasterKey has no weight field
  -> GlyphAtlas
    -> 500/700 would collide if they shared glyph_id/face_idx/size_q6
```

## Target Flow

```text
TextStyle.weight (numeric CSS value)
  -> ui_text::shape_text()
    -> requested weight preserved as u16
    -> shaping faces and face resolution receive requested weight
    -> ShapedText stores the requested weight for downstream use
  -> scene_convert::convert_text()
    -> RasterKey includes weight
  -> GlyphAtlas / FontCollection::rasterize()
    -> variable-weight and fallback-weight glyphs cache separately
```

## Important Scope Correction

This section should build the capability and update the consumers that already expose weight. It should not claim that every text-bearing widget adopts the mockup's exact weight values here. `ButtonStyle`, `DropdownWidget`, and several other text-bearing widgets do not currently expose a weight field, so their visual fidelity adoption belongs in later widget-specific sections unless this section explicitly broadens those APIs.

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

[cached_measurer/mod.rs](/home/eric/projects/ori_term/oriterm/src/font/shaper/cached_measurer/mod.rs) already carries `weight: FontWeight` in `TextCacheKey`, so the cache model is compatible with a numeric type as long as the newtype derives `Hash` and `Eq`.

### Checklist

- [ ] Replace the enum with a numeric `FontWeight` newtype in `oriterm_ui`
- [ ] Add named constants for the standard CSS weights
- [ ] Add `new()` and `value()` helpers
- [ ] Keep `Default` at 400 (`NORMAL`)
- [ ] Update `TextStyle::new()` and `TextStyle::default()`
- [ ] Update text tests and cache-key callers to the new constants

---

## 02.2 Thread Weight Through Shaping and Raster Keys

### Goal

Make requested UI weight part of the actual rendering pipeline, not just the public API.

### Files

- `oriterm_ui/src/text/mod.rs`
- `oriterm/src/font/shaper/ui_text.rs`
- `oriterm/src/font/collection/shaping.rs`
- `oriterm/src/font/collection/mod.rs`
- `oriterm/src/gpu/scene_convert/text.rs`
- `oriterm/src/gpu/window_renderer/helpers.rs`
- `oriterm/src/font/mod.rs`

### Required Data-Flow Changes

Section 01 already threads `size_q6` per shaped run. Section 02 should extend the same boundary with weight:

```rust
pub struct ShapedText {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
    pub size_q6: u32,
    pub weight: u16,
}
```

`ShapedText` is the correct ownership point because the whole run shares one requested weight, just as it shares one requested size.

### RasterKey Change

If requested weight affects rasterization, `RasterKey` must include it:

```rust
pub struct RasterKey {
    pub glyph_id: u16,
    pub face_idx: FaceIdx,
    pub weight: u16,
    pub size_q6: u32,
    pub synthetic: SyntheticFlags,
    pub hinted: bool,
    pub subpx_x: u8,
    pub font_realm: FontRealm,
}
```

Without this field, 500- and 700-weight glyphs from the same face and size would alias in the atlas cache.

### Shaping API Change

The current `FontCollection::create_shaping_faces()` reads the collection-global `self.weight`. That is not sufficient for UI text, which can use multiple weights in one frame.

Add a requested-weight-aware shaping entry point for UI text, for example:

```rust
pub fn create_shaping_faces_for_weight(
    &self,
    weight: u16,
) -> Vec<Option<rustybuzz::Face<'_>>> { ... }
```

or the reusable-buffer equivalent.

Then update `ui_text::shape_text()` / `measure_text_styled()` to pass `style.weight.value()` through rather than collapsing immediately to a binary threshold.

### Rasterization Change

`FontCollection::rasterize()` currently consults `self.weight` when calling `face_variations()`. For UI text, rasterization must instead use the requested weight carried by the key or a companion parameter. Otherwise the atlas entry cannot match the shaped request.

### Checklist

- [ ] Extend `ShapedText` with requested weight
- [ ] Extend `RasterKey` with weight
- [ ] Update `RasterKey` constructors/helpers/tests across the workspace
- [ ] Add weight-aware shaping-face creation for UI text
- [ ] Make UI shaping pass `style.weight.value()` through to shaping and rasterization
- [ ] Make scene conversion and `scene_raster_keys()` use the shaped weight when building keys

---

## 02.3 Weight Realization Policy

### Goal

Define honest, deterministic behavior for numeric weight requests across the fonts the app may actually load.

### Reality Check

The original plan promised that `MEDIUM` (500) and `BOLD` (700) would become visually distinct just by changing the UI `FontWeight` type and mapping 500 to `GlyphStyle::Bold`. That is not correct.

There are three distinct runtime cases:

1. **Font has a `wght` axis**  
   Exact requested weight is realizable. This is the best case and should be preferred.

2. **Font has only static Regular + Bold faces**  
   Exact 500/600 may not exist. Requests must fall back to the nearest supported heavier/lighter realization.

3. **Font lacks a real Bold face**  
   Synthetic bold remains the fallback, using the existing `SyntheticFlags::BOLD` path.

### Policy

The plan should state the fallback policy explicitly as product behavior, not as "CSS does this":

- `100..=450` → normal-weight path
- `500..=650` → medium/semibold path: prefer exact `wght` if available, otherwise nearest heavier supported rendering
- `700..=900` → bold path: prefer exact `wght` or bold face, otherwise synthetic bold

That makes the intent clear without misquoting CSS matching rules.

### Important Limitation

Distinct 500 vs 700 rendering is only guaranteed when the active font can realize both weights, typically through:

- a real `wght` axis
- multiple loaded static faces for those weights

This section should not promise exact distinctness on every system font configuration. If exact medium support for static families is required later, that likely needs additional font-discovery/loading work outside this section.

### Checklist

- [ ] Document the requested-weight realization policy in the section and in code comments where the helper lands
- [ ] Prefer exact `wght` axis rendering when available
- [ ] Fall back to nearest supported heavier/lighter rendering when exact weight is unavailable
- [ ] Keep synthetic bold as the last fallback, not the primary model

---

## 02.4 Consumer Adoption + Tests

### Goal

Update the weight-aware consumers that already exist today, and add tests that prove weight survives through the pipeline.

### Existing Consumers In Scope

These already set or expose weight and should migrate in this section:

- [oriterm_ui/src/widgets/label/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/label/mod.rs)
- [oriterm_ui/src/widgets/form_section/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/form_section/mod.rs)
- [oriterm_ui/src/widgets/dialog/rendering.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/dialog/rendering.rs)
- [oriterm_ui/src/widgets/settings_panel/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/settings_panel/mod.rs)
- [oriterm_ui/src/widgets/sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs)
- [oriterm/src/app/settings_overlay/form_builder/appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)

### Out of Scope Unless This Section Expands Them

Do not claim mockup-correct button/dropdown/widget-control weights here unless this section also adds explicit weight fields to those widget styles. As the tree stands today, [button/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/button/mod.rs) only exposes `font_size`, not `font_weight`.

### Tests

Add or update tests for:

- `oriterm_ui/src/text/tests.rs`
  - ordering, clamping, default weight
- `oriterm/src/font/shaper/tests.rs`
  - requested weight is preserved on shaped UI text
- `oriterm/src/gpu/scene_convert/tests.rs`
  - two otherwise-identical UI text runs with different weights produce different `RasterKey` values
- `oriterm/src/font/collection/tests.rs`
  - weight-aware rasterization uses the requested weight when `wght` is available

### Checklist

- [ ] Migrate existing in-scope `FontWeight` consumers to numeric constants
- [ ] Avoid overclaiming widget adoption for styles that do not yet expose weight
- [ ] Add tests for numeric weight API behavior
- [ ] Add tests for weight-aware atlas key separation
- [ ] Add tests for requested weight reaching shaping/rasterization

---

## 02.R Third Party Review Findings

- [x] `[TPR-02-001][high]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original plan only changed the UI `FontWeight` type and threshold mapping, but the real pipeline does not carry requested UI weight through shaping or atlas keys. Resolved: the section now requires weight threading through `ShapedText`, `RasterKey`, shaping-face creation, and rasterization on 2026-03-23.

- [x] `[TPR-02-002][medium]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original "500 maps to Bold because CSS does that" rationale was inaccurate. The product may choose a heavier fallback for fidelity, but that is a project policy, not a faithful statement of CSS matching behavior. Resolved: replaced with an explicit realization policy on 2026-03-23.

- [x] `[TPR-02-003][medium]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original goal overpromised distinct `500` vs `700` rendering for all fonts even though the active font may only provide regular/bold or synthetic-bold fallbacks. Resolved: the section now guarantees exact distinctness only when the font can realize both weights and specifies deterministic fallback otherwise on 2026-03-23.

- [x] `[TPR-02-004][medium]` [plans/ui-css-framework/section-02-font-weight.md](/home/eric/projects/ori_term/plans/ui-css-framework/section-02-font-weight.md) - The original consumer-update subsection claimed mockup-correct button/dropdown adoption even though those widgets do not currently expose a weight field. Resolved: narrowed the scope to existing weight-aware consumers and deferred broader widget adoption unless this section expands those APIs on 2026-03-23.

---

## 02.5 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

1. `cargo test -p oriterm_ui text`
2. `cargo test -p oriterm font::shaper`
3. `cargo test -p oriterm scene_convert`
4. Manual verification with the configured UI font on the active platform

### Completion Criteria

- numeric `FontWeight` values survive through shaping and atlas key generation
- supported fonts can realize distinct 400/500/600/700 requests without cache collisions
- unsupported fonts degrade according to the documented fallback policy
- this section does not claim widget-level weight fidelity beyond the widgets whose APIs actually expose weight
