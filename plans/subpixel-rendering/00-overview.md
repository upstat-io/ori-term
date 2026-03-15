---
plan: "subpixel-rendering"
title: "Proper LCD Subpixel Text Rendering: Exhaustive Implementation Plan"
status: complete
references:
  - "oriterm/src/gpu/shaders/subpixel_fg.wgsl"
  - "oriterm/src/gpu/prepare/emit.rs"
  - "oriterm/src/gpu/prepare/dirty_skip/mod.rs"
  - "oriterm/src/gpu/instance_writer/mod.rs"
  - "oriterm/src/gpu/draw_list_convert/text.rs"
  - "oriterm/src/gpu/pipeline_tests.rs"
  - "oriterm/src/font/collection/face.rs"
  - "oriterm/src/font/collection/mod.rs"
  - "oriterm/src/font/collection/colr_v1/rasterize.rs"
  - "oriterm/src/font/mod.rs"
  - "oriterm/src/app/config_reload.rs"
---

# Proper LCD Subpixel Text Rendering: Exhaustive Implementation Plan

## Mission

Fix the font rendering bug where text at weight 400 appears significantly bolder
than it should on Windows at 1x DPI. The root cause is the subpixel shader's
unknown-background fallback using `max(r, g, b)` to collapse per-channel coverage
to grayscale, which overestimates stroke thickness. The fix is to pass the real
cell background color to the subpixel shader, enabling proper per-channel LCD
compositing that produces correct weight rendering.

## Architecture

```
Cell iteration (fill_frame_shaped)
  |
  v
resolve_cell_colors() --> (fg: Rgb, bg: Rgb)
  |
  +--> Background rect: push_rect(bg)           [draw call #1]
  |
  +--> GlyphEmitter::emit(fg, bg)               [NEW: bg passed through]
         |
         +--> AtlasKind::Mono    --> push_glyph(fg)          [unchanged]
         +--> AtlasKind::Subpixel --> push_glyph_with_bg(fg, bg) [CHANGED]
         +--> AtlasKind::Color   --> push_glyph(fg)          [unchanged]
  |
  v
GPU render pass:
  1. Clear color (palette background, carries opacity)
  2. bg_pipeline: background rects
  3. fg_pipeline: mono glyphs (R8 alpha, tinted by fg)
  4. subpixel_fg_pipeline: subpixel glyphs (RGBA per-channel)
     --> shader reads bg_color, does mix(bg, fg, mask) per-channel
     --> outputs vec4(r, g, b, 1.0) for opaque pixels
     --> outputs vec4(0, 0, 0, 0) for zero-coverage pixels
  5. color_fg_pipeline: color glyphs (emoji)
```

## Design Principles

**1. Correct compositing over correct fallback.** The `max(r, g, b)` formula
produces visually incorrect coverage that makes normal-weight text look bold.
This is not a cosmetic issue; it misrepresents the font's intended weight. True
per-channel LCD compositing with a known background produces correct results.

**2. Zero cross-cell contamination.** Glyph quads can extend beyond cell
boundaries due to bearing offsets. The shader must output transparent pixels
(pass-through) where coverage is zero, preventing background color from one cell
from leaking into a neighboring cell with a different background.

**3. Opacity-aware mode selection.** Subpixel rendering assumes an opaque
background. When the terminal has transparent backgrounds (opacity < 1.0), the
per-channel compositing produces visible color fringing. The system must
automatically disable subpixel rendering when opacity is not 1.0.

## Section Dependency Graph

```
Section 01 (Shader Fix) ----+
                             +---> Section 02 (Data Flow) ---> Section 05 (Verification)
Section 03 (Opacity Disable) +                                       ^
                                                                      |
Section 04 (Var Font Metrics)  (independent) -------------------------+
```

- Section 01 (shader) and Section 03 (opacity disable) are prerequisites for
  Section 02 (data flow), because enabling bg-hint compositing without the
  zero-coverage guard or opacity gating would produce visual artifacts.
- Section 04 (variable font metrics) is independent and can be done in any order.
- Section 05 (verification) requires all other sections (01–04).

**Cross-section interactions (must be co-implemented):**
- **Section 01 + Section 02**: The shader fix alone does nothing if no bg color
  is passed. The data flow alone would hit the old shader path if the shader
  isn't updated. Both must land together for the fix to work.
- **Section 03 + Section 02**: If bg-hint compositing is enabled without
  opacity-aware disable, transparent windows would show color fringing.

**Signature change ripple effects (must update callers + tests together):**
- **Section 03**: `resolve_subpixel_mode()` gains an `opacity` parameter —
  all 5 call sites and all 5 existing tests must be updated atomically.
- **Section 04**: `compute_metrics()` gains a `variations` parameter —
  all 4 non-test call sites and all 5 existing tests must be updated atomically.
- **Section 02**: `GlyphEmitter::emit()` gains a `bg` parameter — both
  `fill_frame_shaped` and `fill_frame_incremental` call sites must be updated.

## Implementation Sequence

```
Phase 0 - Prerequisites
  +-- Section 01: Fix subpixel shader (zero-coverage guard + fg.a handling)
  +-- Section 03: Wire opacity-aware subpixel disable

Phase 1 - Core Fix
  +-- Section 02: Pass bg color through GlyphEmitter to push_glyph_with_bg
  Gate: Weight 400 text on Windows renders at correct visual weight

Phase 2 - Independent Improvement (parallel with Phase 1)
  +-- Section 04: Fix variable font metrics to pass variations

Phase 3 - Verification
  +-- Section 05: Full test matrix, visual regression, performance validation
  Gate: ./test-all.sh, ./clippy-all.sh, ./build-all.sh all green
```

**Why this order:**
- Phase 0 is pure preparation: shader changes and config wiring that don't
  change behavior until Phase 1 activates them (the shader's known-bg path is
  only reached when bg.a > 0, which doesn't happen until Phase 1).
- Phase 1 is the critical change that switches terminal glyphs from
  `push_glyph` to `push_glyph_with_bg`, activating the known-bg shader path.
- Phase 2 is orthogonal: variable font metrics affect cell sizing regardless of
  subpixel rendering.

## File Size Warnings

Several files this plan touches are at or near the 500-line hard limit:

| File | Current Lines | Plan Section | Risk |
|------|--------------|--------------|------|
| `prepare/dirty_skip/mod.rs` | 495 | 02 | **At limit** — adding `bg` plumbing will exceed 500. Must split first. |
| `config_reload.rs` | 493 | 03 | **At limit** — adding opacity param will exceed 500. Must split first. |
| `prepare/mod.rs` | 485 | 02 | Near limit — monitor after changes. |
| `font/mod.rs` | 482 | 03 | Near limit — only removing `dead_code` attr, low risk. |
| `font/collection/mod.rs` | 457 | 04 | Safe — changes are small. |

Implementers: split files **before** adding new code, not after.

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| Text at weight 400 appears too bold on Windows | `max(r,g,b)` coverage collapse in subpixel shader | Section 01 + 02 | Not Started |
| `fg.a` (dim factor) ignored in known-bg shader path | Known-bg branch uses `fg.rgb` without applying `fg.a` | Section 01 | Not Started |
| `SubpixelMode::for_display()` is dead code | Opacity-aware disable not wired into config resolution | Section 03 | Not Started |
| `glyph_metrics(&[])` ignores variable font variations | Advance width computed without axis settings | Section 04 | Not Started |
| `metrics(&[])` ignores variable font variations | Cell height/width computed without axis settings | Section 04 | Not Started |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Shader Compositing Fix | `section-01-shader-compositing.md` | Not Started |
| 02 | Background Hint Data Flow | `section-02-bg-hint-data-flow.md` | Not Started |
| 03 | Opacity-Aware Subpixel Disable | `section-03-opacity-aware-disable.md` | Not Started |
| 04 | Variable Font Metrics | `section-04-variable-font-metrics.md` | Not Started |
| 05 | Verification | `section-05-verification.md` | Not Started |
