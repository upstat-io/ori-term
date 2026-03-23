---
plan: "ui-css-framework"
title: "CSS-Equivalent UI Framework Features"
status: not-started
references:
  - "mockups/settings-brutal.html"
  - "plans/brutal-design-pass-2/"
---

# CSS-Equivalent UI Framework Features

## Mission

Implement proper CSS-equivalent support for every feature the mockup (`mockups/settings-brutal.html`) uses. The mockup is the spec. When oriterm's rendered settings dialog and the mockup disagree, the mockup wins — and the gap is a feature missing from the UI framework, not a styling choice.

The mockup uses 14 distinct CSS features that oriterm_ui currently lacks or handles incorrectly. This plan adds each one as a proper framework capability, not a widget-level hack. After this plan, any widget can use multi-size text, numeric font weights, text transforms, line height control, per-side borders, opacity, scrollbar styling, and icons — the same way CSS makes these available to any HTML element.

## Architecture

### Text Rendering Pipeline

```
TextStyle (oriterm_ui)                     ← Widget declares desired appearance
  ↓
TextMeasurer::shape() / measure()          ← UiFontMeasurer selects collection by size
  ↓
ui_text::shape_text() / measure_text()     ← Shapes via rustybuzz, selects face by weight
  ↓
FontCollection (selected by size pool)     ← Font bytes + face data + cell metrics
  ↓
ShapedText { glyphs, width, height }       ← Physical-pixel advances and metrics
  ↓
Scene::push_text()                         ← Widget paints shaped text to draw list
  ↓
scene_convert::convert_text()              ← Builds RasterKey per glyph (size_q6, face_idx)
  ↓
GlyphAtlas::lookup() / rasterize()         ← Cache hit or rasterize at target size
  ↓
GPU instance buffer → wgpu render pass     ← Final screen pixels
```

Key invariant: `RasterKey.size_q6` must match the `FontCollection` that shaped the glyph. Mismatches produce blurry or wrong-sized glyphs. Section 01 establishes this invariant by threading size through the entire pipeline.

### Layout Pipeline

```
Widget::layout(constraints, measurer)      ← Widget returns LayoutBox descriptor
  ↓
LayoutBox { size: SizeSpec, children }     ← Declarative flex layout tree
  ↓
solver::compute_layout(root, viewport)     ← Two-pass flex solver
  ↓
LayoutNode { rect, children }              ← Concrete pixel positions and sizes
  ↓
Widget::paint(scene, layout_node)          ← Widget emits draw commands at solved positions
```

## Implementation Note: Insets Parameter Order

`Insets::tlbr(top, left, bottom, right)` uses TLBR (top-left-bottom-right) order, NOT CSS's TRBL (top-right-bottom-left) shorthand. When translating CSS `padding: 6px 30px 6px 10px` (T=6, R=30, B=6, L=10), write `Insets::tlbr(6.0, 10.0, 6.0, 30.0)`. The values are the same but the parameter positions differ. Double-check every CSS-to-Rust padding translation.

## Design Principles

1. **Per-element styling.** Every CSS property in the mockup must be translatable to a field on `TextStyle`, `RectStyle`, or a widget-level style struct. No property should require knowing about the rendering pipeline to use — widgets describe what they want, the framework makes it happen.

2. **Proper rasterization.** No GPU scaling of glyph bitmaps. When a widget requests 18px text, the font collection at 18px rasterizes the glyphs at 18px. The atlas stores 18px bitmaps. The GPU renders them 1:1. This is how every serious text renderer works (GPUI, Chromium, FreeType).

3. **Mockup is the spec.** Every CSS value in `settings-brutal.html` has a corresponding Rust field and the pipeline to honor it. If the mockup says `font-weight: 500`, there must be a `FontWeight::MEDIUM` that maps to the correct face or synthetic weight. If it says `text-transform: uppercase`, there must be a `TextTransform::Uppercase` that the label applies before shaping.

## Section Dependency Graph

```
01 Multi-Size Fonts ─────┐
                         ├──→ 02 Font Weight (needs size pools for face selection)
                         ├──→ 04 Line Height (needs size-aware metrics)
                         │
03 Text Transform ───────┤     (independent — string transforms + spacing fix)
                         │
05 Per-Side Borders ─────┤     (independent — draw/layout change)
06 Opacity + Display ────┤     (independent — paint-time alpha)
07 Scrollbar Styling ────┤     (independent — scroll widget)
08 Icon Verification ────┤     (independent — icon paths)
                         │
                         ↓
09 Settings Content ─────┤     (needs 01-08 for complete widget styling)
                         │
10 Sidebar Fidelity ─────┤
11 Content Typography ───┤     (need 01-09 for correct rendering)
12 Footer + Buttons ─────┤
13 Widget Controls ──────┤
                         │
                         ↓
14 Verification ─────────┘     (needs all above — visual regression)
```

## Implementation Phases

**Phase 1 — Typography Foundation (Sections 01-04)**
Multi-size font rendering, numeric font weights, text transforms, line height. These are the most impactful: every text element in the mockup uses at least one of these features. After Phase 1, page titles are visibly larger than body text, section headers use Medium weight, and uppercase transforms work.

**Phase 2 — Layout + Visual Features (Sections 05-08)**
Per-side borders, opacity/display control, scrollbar styling, icon verification. These are independent of each other and of Phase 1 (except icons may need multi-size for labels). Can be parallelized.

**Phase 3 — Visual Fidelity (Sections 09-13)**
Settings content completeness, then pixel-level matching of sidebar, content area, footer, and widget controls. Each section targets one visual region and applies the framework features from Phases 1-2 to match the mockup exactly.

**Phase 4 — Verification (Section 14)**
Side-by-side comparison, visual regression tests, DPI scaling verification. Build gate: all prior sections must pass `build-all.sh`, `clippy-all.sh`, `test-all.sh`.

## Metrics — Key Module Sizes

| File | Lines | Role |
|------|-------|------|
| `oriterm_ui/src/text/mod.rs` | ~200 | TextStyle, ShapedGlyph, ShapedText, TextMetrics |
| `oriterm/src/font/shaper/ui_measurer.rs` | 76 | UiFontMeasurer (TextMeasurer impl) |
| `oriterm/src/font/shaper/ui_text.rs` | ~308 | shape_text, measure_text_styled, shape_text_string |
| `oriterm/src/font/collection/mod.rs` | ~484 | FontCollection (faces, metrics, rasterization) |
| `oriterm/src/font/mod.rs` | ~478 | RasterKey, FaceIdx, CellMetrics, GlyphStyle |
| `oriterm/src/gpu/scene_convert/text.rs` | ~190 | convert_text (RasterKey construction) |
| `oriterm/src/gpu/scene_convert/mod.rs` | ~342 | TextContext, convert_scene, convert_rect_clipped |
| `oriterm/src/gpu/window_renderer/scene_append.rs` | 148 | cache_scene_glyphs, ui_size_q6, ui_hinted |
| `oriterm/src/gpu/window_renderer/mod.rs` | ~554 | WindowRenderer -- OVER 500-line limit, file-size split required in Section 01 |
| `oriterm_ui/src/layout/solver.rs` | 469 | Layout flex solver |
| `oriterm_ui/src/layout/size_spec.rs` | 34 | SizeSpec enum (Fixed, Fill, FillPortion, Hug) |
| `oriterm_ui/src/draw/border.rs` | 16 | Border struct (uniform only — width + color) |
| `oriterm_ui/src/draw/scene/content_mask.rs` | 30 | ContentMask (clip rect only, no opacity yet) |
| `oriterm_ui/src/layout/layout_box.rs` | ~112 | LayoutBox (no `visible` field yet) |

## Known Bugs

1. **TextStyle.size ignored.** `UiFontMeasurer` always uses the single `FontCollection`'s `size_px()` (~13.3px at init, ~14.7px after font-size change). A widget setting `TextStyle { size: 18.0, .. }` gets the same glyph size as `size: 13.0`. Fixed in Section 01.

2. **Font weight not rendering.** `FontWeight::Bold` in `TextStyle` maps to `GlyphStyle::Bold` which selects the Bold face slot — but the UI font collection is built from the monospace font set, which may not have a Bold variant loaded for the UI size. Fixed in Section 02.

3. **Letter spacing scale bug.** `UiFontMeasurer::shape()` applies `letter_spacing * scale` to physical advances, but `UiFontMeasurer::measure()` applies `letter_spacing` directly to logical width. The counting basis also differs: `measure()` counts `text.chars()` while `shape()` counts `shaped.glyphs`. These can differ (ligatures reduce glyph count). Additionally, the mockup uses em-based spacing (`0.05em`, `0.15em`) which has no API. Fixed in Section 03.

## Quick Reference

| ID | Title | File | Depends On |
|----|-------|------|------------|
| 01 | Multi-Size Font Rendering | `section-01-multi-size-fonts.md` | — |
| 02 | Numeric Font Weight System | `section-02-font-weight.md` | 01 |
| 03 | Text Transform + Letter Spacing | `section-03-text-transform.md` | — |
| 04 | Line Height Control | `section-04-line-height.md` | 01 |
| 05 | Per-Side Borders | `section-05-per-side-borders.md` | — |
| 06 | Opacity + Display Control | `section-06-opacity-display.md` | — |
| 07 | Scrollbar Styling | `section-07-scrollbar-styling.md` | — |
| 08 | Icon Path Verification | `section-08-icon-verification.md` | — |
| 09 | Settings Content Completeness | `section-09-settings-content.md` | 01-03, 05-08 |
| 10 | Visual Fidelity: Sidebar + Nav | `section-10-sidebar-fidelity.md` | 01-03, 05, 08 |
| 11 | Visual Fidelity: Content + Typography | `section-11-content-typography.md` | 01-04 |
| 12 | Visual Fidelity: Footer + Buttons | `section-12-footer-buttons.md` | 02, 03, 05 |
| 13 | Visual Fidelity: Widget Controls | `section-13-widget-controls.md` | 01, 02 |
| 14 | Verification + Visual Regression | `section-14-verification.md` | 01-13 |
