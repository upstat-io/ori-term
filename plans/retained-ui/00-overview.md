---
plan: "retained-ui"
title: "Retained UI Framework: Exhaustive Implementation Plan"
status: not-started
references:
  - "plans/roadmap/"
---

# Retained UI Framework: Exhaustive Implementation Plan

## Mission

Transform the `oriterm_ui` rendering pipeline from immediate-mode full-scene rebuilds into a retained, cached, invalidation-aware framework. Every UI surface — terminal chrome (tab bar, search bar), dialogs (settings, confirmation), overlays (dropdowns, context menus, tooltips), and future UI windows — benefits from text caching, subtree invalidation, and retained scene composition. The target is native-feel interaction latency: hover on one button never reshapes every label in the window.

## Architecture

```
                    ┌──────────────────────────────────────────────┐
                    │              Event Loop (App)                │
                    │                                              │
                    │   mouse/key ──► Widget Tree                  │
                    │                    │                          │
                    │              WidgetResponse                   │
                    │   { response: RequestPaint, source: id }     │
                    │   { response: RequestLayout, source: id }    │
                    │                    │                          │
                    │         InvalidationTracker                   │
                    │     mark(widget_id, DirtyKind::Paint)         │
                    │            ┌────┴─────┐                      │
                    │        clean?      dirty?                    │
                    │          │            │                       │
                    │     reuse scene   rebuild subtree             │
                    │          │            │                       │
                    │          └────┬───────┘                       │
                    │               ▼                               │
                    │         Scene Composer                        │
                    │    (merge retained + rebuilt subtrees)        │
                    │               │                               │
                    │          DrawList                             │
                    │               │                               │
                    │     GPU Converter (existing)                  │
                    │               │                               │
                    │     Instance Buffers → Render                 │
                    └──────────────────────────────────────────────┘
```

## Design Principles

**1. Cache at the boundary, not inside the widget.**
Today each widget calls `ctx.measurer.shape()` on every draw, re-shaping the same "Cancel" string thousands of times. The fix is a caching `TextMeasurer` wrapper — `CachedTextMeasurer` — that sits at the framework boundary, invisible to widgets. Widgets keep their current simple `measure()` / `shape()` API. Root cause: `UiFontMeasurer` in `oriterm/src/font/shaper/ui_measurer.rs` has no cache; it delegates directly to `ui_text::shape_text()` every call.

**2. Invalidation is typed and scoped, not boolean and global.**

Today `dirty: bool` and `ui_stale: bool` on `WindowContext` mean "rebuild everything". `EventResponse` already distinguishes `RequestPaint` vs `RequestLayout` (in `oriterm_ui/src/input/event.rs`), but the consumer ignores the distinction — `render_dialog` always does a full rebuild. The fix is carrying the dirty scope (which widget, what kind) through to the render path so unchanged subtrees are skipped.

**3. Retained scene nodes replace per-frame draw list regeneration.**

Today `DrawList::clear()` is called at the top of every render (dialog_rendering.rs:58, draw_helpers.rs:48 for chrome). Every widget re-emits its entire draw command sequence. The fix is per-subtree `DrawList` caching: unchanged widgets reuse their cached draw commands, and a composition pass merges cached + rebuilt subtrees into the final draw list.

## Section Dependency Graph

```
Phase 0 (independent, parallel):
  Section 01 (Text Cache)
  Section 05 (Surface Strategy) ◄─── Section 06 (Window Lifecycle)
                                       (06 defines SurfaceLifecycle
                                        used by 05's SurfaceHost trait)

Phase 1 (sequential):
  Section 01 ──► Section 02 (Subtree Invalidation)
                      │
                      ▼
                 Section 03 (Scene Retention)
                      │      [02 + 03 must land together]
                      ▼
Phase 2:         Section 04 (Scroll Transform)
                      │
                      ▼
Phase 3:         Section 07 (Verification)
```

- **Section 01** is fully independent -- pure addition, no behavioral changes.
- **Section 05** is independent. **Section 06** is independent but provides the `SurfaceLifecycle` type that Section 05's `SurfaceHost` trait references. Implement Section 06 before or alongside Section 05.
- **Section 02** requires Section 01 (cached text makes subtree reuse meaningful).
- **Section 03** requires Section 02 (subtree invalidation drives selective rebuild).
- **Section 04** requires Section 03 (retained child scenes make scroll transforms useful).
- **Section 07** requires all others.

**Cross-section interactions (must be co-implemented):**
- **Section 02 + Section 03**: Subtree invalidation without scene retention means you track dirty state but still rebuild everything. Scene retention without invalidation means you cache but never know what to invalidate. They must land together to be useful. **Implementation strategy:** Implement Section 02 first (the tracker is testable in isolation with unit tests), then implement Section 03 and wire it to the tracker. Both sections commit together — do not merge Section 02 without Section 03.

- **Section 01 + Section 03**: Scene retention caches `DrawCommand` sequences that include `DrawCommand::Text` with `ShapedText`. If the text cache (Section 01) returns a different `ShapedText` instance (e.g. after cache eviction + re-shaping), the scene cache's `DrawCommand::Text` entries will have stale `ShapedText` data. This is correct only if the re-shaped text produces identical output — which it will if the font and parameters are unchanged. Font/DPI changes clear both caches simultaneously, so staleness cannot occur. Document this invariant.
- **Section 04 + Section 02**: Scroll events produce `DirtyKind::Paint` for the scroll widget but NOT for children. The invalidation tracker must support this selective propagation — `mark(scroll_widget_id, Paint)` without marking child IDs.

## Implementation Sequence

```
Phase 0 - Prerequisites
  +-- 01: Text/layout caching (CachedTextMeasurer)
  +-- 05: Surface strategy abstraction (RenderStrategy enum)
  +-- 06: Window lifecycle state machine (SurfaceLifecycle)

Phase 1 - Core Retention
  +-- 02: Subtree invalidation (DirtyKind, InvalidationTracker)
  +-- 03: Scene retention (SceneNode, per-widget DrawList caching)
  Gate: hover on a Settings button does not call shape() for any other widget

Phase 2 - Optimization
  +-- 04: Scroll as viewport transform
  Gate: scrolling Settings does not call draw() on offscreen widgets

Phase 3 - Verification
  +-- 07: Test matrix, performance validation, visual regression
  Gate: ./test-all.sh green, frame time ≤16.6ms for Settings interactions
```

**Why this order:**
- Phase 0 items are pure additions -- no behavioral changes, no risk.
- Phase 1 must follow Phase 0 because text caching makes scene retention meaningful (without it, even "retained" scenes re-shape text on reuse, saving nothing).
- Phase 2 depends on Phase 1 because scroll transform reuse requires retained child scene content.
- Phase 3 validates the complete pipeline.

**File size prerequisites (must be done before the sections that modify these files):**
- **Before Section 04**: `oriterm/src/gpu/draw_list_convert/mod.rs` is 488 lines. Extract `convert_text()` + `emit_text_glyph()` + `convert_icon()` into `draw_list_convert/text.rs` (~120 lines) to make room for `PushTranslate`/`PopTranslate` handling.
- **Monitor during Section 03**: `oriterm_ui/src/widgets/container/mod.rs` is 413 lines. Cache-check additions to `draw()` must not push it past 500.

## Metrics (Current State)

| Module | Key Files | Concern |
|--------|-----------|---------|
| `oriterm_ui/src/widgets/` | 45 widget source files | Widget draw() calls shape() every frame |
| `oriterm/src/font/shaper/ui_measurer.rs` | 61 lines | No cache, delegates to shaper per call |
| `oriterm/src/app/dialog_rendering.rs` | 143 lines | Full rebuild every render |
| `oriterm/src/app/redraw/draw_helpers.rs` | 163 lines | Chrome draw list cleared per frame (`draw_list.clear()` at line 48) |
| `oriterm_ui/src/draw/draw_list.rs` | 267 lines | No per-subtree caching |

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 Text Cache | ~200 new | Medium | — |
| 02 Subtree Invalidation | ~350 new | High (reduced with container-side injection) | 01 |
| 03 Scene Retention | ~400 new | High | 02 |
| 04 Scroll Transform | ~150 modified | Medium | 03 |
| 05 Surface Strategy | ~200 new | Medium | — |
| 06 Window Lifecycle | ~150 new | Low | — |
| 07 Verification | ~300 test | Medium | All |
| **Total new** | **~1750** | | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Text/Layout Caching | `section-01-text-cache.md` | Complete |
| 02 | Subtree Invalidation | `section-02-subtree-invalidation.md` | Not Started |
| 03 | Scene Retention | `section-03-scene-retention.md` | Not Started |
| 04 | Scroll Transform | `section-04-scroll-transform.md` | Not Started |
| 05 | Surface Strategy | `section-05-surface-strategy.md` | Not Started |
| 06 | Window Lifecycle | `section-06-window-lifecycle.md` | Not Started |
| 07 | Verification | `section-07-verification.md` | Not Started |
