---
reroute: true
name: "Subpixel Rendering"
full_name: "Proper LCD Subpixel Text Rendering with Background-Hint Compositing"
status: resolved
order: 1
---

# Subpixel Rendering Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Shader Compositing Fix
**File:** `section-01-shader-compositing.md` | **Status:** Complete

```
subpixel, LCD, per-channel, compositing, blend, mix, max, coverage
subpixel_fg.wgsl, fragment shader, bg_color, bg.a, mask, grayscale
premultiplied alpha, PREMUL_ALPHA_BLEND, BlendState, BlendFactor
bold, weight, thick, font rendering, glyph coverage
fg.a, dim, fg_dim, dimming, alpha, opacity
```

---

### Section 02: Background Hint Data Flow
**File:** `section-02-bg-hint-data-flow.md` | **Status:** Complete

```
push_glyph, push_glyph_with_bg, GlyphEmitter, emit, bg color
instance_writer, InstanceWriter, instance record, 80-byte
fill_frame_shaped, fill_frame_incremental, prepare, PreparedFrame
dirty_skip, incremental, row ranges, cached rows
AtlasKind, Mono, Subpixel, Color, routing
draw_list_convert, TextContext, UI text, bg_hint
unshaped, builtin, geometric, selection, search, resolve_cell_colors
```

---

### Section 03: Opacity-Aware Subpixel Disable
**File:** `section-03-opacity-aware-disable.md` | **Status:** Complete

```
opacity, transparent, transparency, glass, acrylic, background opacity
SubpixelMode, for_display, from_scale_factor, GlyphFormat
config_reload, resolve_subpixel_mode, dead_code, wiring
scale_factor, HiDPI, Retina, 2x, DPI
clear_color, window opacity, palette.opacity
```

---

### Section 04: Variable Font Metrics
**File:** `section-04-variable-font-metrics.md` | **Status:** Complete

```
glyph_metrics, metrics, advance_width, cell_width, cell_height
variable font, wght, variations, fvar, axes, AxisInfo
swash, FontRef, scale, compute_metrics, rasterize_from_face
face_variations, VarSettings, FaceVariationResult
weight, 400, 700, non-default, light, medium, semibold
cmap_glyph, colr_v1, clip_box, estimate_clip_box
```

---

### Section 05: Verification
**File:** `section-05-verification.md` | **Status:** Complete

```
test, pixel readback, headless GPU, visual regression
pipeline_tests, subpixel_blend, blend formula
render_text, render_colored_cell, offscreen
dim, cross-cell, overhang, bearing, neighboring cell
build-all, clippy-all, test-all, regression
selection, search, cursor, block cursor, epsilon, edge case
opacity, transparent, variable font, weight, frame time, performance
```

---

## Hygiene Review Status

Reviewed against `.claude/rules/impl-hygiene.md` and `.claude/rules/code-hygiene.md`.
Cleanup items woven into sections: 3 BLOAT, 3 STYLE.
See individual section files for `[BLOAT]` and `[STYLE]` checklist items.

---

## Performance Validation

This plan modifies GPU rendering hot paths (subpixel shader, glyph emission).

**When to benchmark:** Sections 01, 02 (shader change + data flow)
**Skip benchmarks for:** Section 03 (config wiring), Section 04 (metrics, cold path)

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Shader Compositing Fix | `section-01-shader-compositing.md` |
| 02 | Background Hint Data Flow | `section-02-bg-hint-data-flow.md` |
| 03 | Opacity-Aware Subpixel Disable | `section-03-opacity-aware-disable.md` |
| 04 | Variable Font Metrics | `section-04-variable-font-metrics.md` |
| 05 | Verification | `section-05-verification.md` |
