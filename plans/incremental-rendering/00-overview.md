---
plan: "incremental-rendering"
title: "Incremental Rendering Pipeline: Exhaustive Implementation Plan"
status: not-started
references:
  - "plans/ui-framework-overhaul/"
  - "plans/roadmap/section-23-performance.md"
---

# Incremental Rendering Pipeline: Exhaustive Implementation Plan

## Mission

Eliminate full-scene rebuilds from the UI rendering pipeline. Every frame currently clears the entire Scene, walks the full widget tree (prepare + prepaint + paint), converts all primitives to GPU instances, and submits — regardless of what actually changed. This plan transforms the pipeline from immediate-mode full-repaint to retained-mode incremental-update, so that a scroll event repaints only the scrolled region, a hover highlights only the hovered widget, and an idle dialog produces zero GPU work.

## Architecture

```
Current (immediate, full-repaint):

  Event → mark_dirty → about_to_wait → App::render_dialog()
    → App::compose_dialog_widgets():
      → scene.clear()
      → prepare_widget_tree(ALL)      ← O(n) tree walk
      → prepaint_widget_tree(ALL)     ← O(n) tree walk
      → chrome.paint() + content.paint()  ← O(n) tree walk, ALL primitives
      → damage.compute_damage()       ← per-widget hash comparison
      → append_ui_scene_with_text()   ← O(p) GPU conversion
    → renderer.render_to_surface()    ← full GPU submit

Target (incremental, partial-repaint):

  Event → invalidate(widget_id, region)
    → about_to_wait
    → prepare_widget_tree(DIRTY only)    ← O(dirty) walk
    → prepaint_widget_tree(DIRTY only)   ← O(dirty) walk
    → paint(DIRTY widgets only)          ← O(dirty) paint
    → patch_scene(changed primitives)    ← O(delta) GPU update
    → render_to_surface(damage_rects)    ← partial GPU submit

  Scroll-specific fast path:
    → update scroll_offset
    → blit existing texture at new offset
    → paint newly-revealed strip only
    → composite scroll texture + chrome + footer
```

## Design Principles

**1. Skip what hasn't changed.** The current pipeline already gates entire phases (prepare/prepaint are skipped when `widget_dirty < DirtyKind::Prepaint`), but WITHIN each phase it walks ALL ~37 widgets unconditionally. A single hover state change walks 37+ widgets, measures all their text, resolves all their visual states, and paints all their primitives. The fix is to track per-widget dirty state and skip clean subtrees at every phase: prepare, prepaint, and paint.

**2. Verify and harden viewport culling.** ContainerWidget, FormLayout, and FormSection already call `current_clip_in_content_space()` and skip children whose `child_node.rect` does not intersect `visible_bounds`. ScrollWidget pushes clip + offset before painting its child. The existing culling likely already works correctly for most cases. Section 01 verifies this with concrete tests and fixes any edge cases (e.g., nested scroll containers, fallback when no clip is active).

**3. Retain what was painted.** The Scene is cleared and rebuilt from scratch every frame. This prevents any form of caching. The fix is to retain the Scene across frames and patch only the regions that changed, using per-widget dirty tracking to identify what needs repainting.

## Section Dependency Graph

```
01: Viewport Culling Verification ────────────────┐
                                                   │
02: Selective Tree Walks ─────────────────────────┤
                                                   │
03: Layout Cache Unification ─────────────────────┤
                                                   ├──→ 06: Dialog Integration ──→ 07: Verification
04: Retained Scene & Dirty Regions ───────────────┤
                                                   │
05: GPU-Side Scroll ──────────────────────────────┘
```

- **Sections 01-03** are independent and address CPU-side bottlenecks. Each provides measurable improvement alone.
- **Section 04** builds on 02 (needs per-widget dirty tracking to know which scene fragments to patch).
- **Section 05** builds on 01 (viewport culling) and 04 (retained scene) for the GPU scroll texture approach.
- **Section 06** integrates all previous sections into the dialog rendering pipeline.
- **Section 07** verifies everything.

**Cross-section interactions:**
- **Section 02 + Section 04**: Selective tree walks produce a "dirty widget set" that Section 04's scene patching consumes. The dirty set format must be designed together.
- **Section 03 + Section 01**: Layout cache unification ensures viewport culling uses fresh, correct layout bounds without recomputing the entire tree.
- **Section 04 vs Section 05 for scroll**: Fragment caching (Section 04) does NOT help with scroll performance, because Scene primitives contain absolute positions baked in at paint time. Scrolling changes every visible widget's position, invalidating all fragments. Section 04's retained scene helps hover/focus/animation (position-stable visual changes). Section 05's GPU texture approach is required for scroll performance.

## Implementation Sequence

```
Phase 0 - Bug Fix (immediate, before optimization work)
  +-- 03.3b: Fix empty prepaint_bounds in 3 files (dialog_rendering.rs, redraw/mod.rs, redraw/multi_pane/mod.rs)
  Gate: All widgets receive correct bounds during prepaint (not Rect::default())

Phase 1 - Quick Wins (independent, each provides immediate improvement)
  +-- 01: Verify and harden existing viewport culling (ContainerWidget, FormLayout, FormSection)
  +-- 03.1-03.2: Layout cache coordination (stop redundant recomputation)
  Gate: Test proves off-screen scheme cards produce zero Scene primitives during scroll

Phase 2 - Dirty Tracking Infrastructure
  +-- 02.1-02.2: Per-widget dirty set in InvalidationTracker
  +-- 02.2b: Module split (pipeline/mod.rs -> prepare.rs + prepaint.rs)
  +-- 02.3-02.4: Selective prepare/prepaint that skip clean subtrees
  Gate: Hover on a single setting row triggers O(1) prepaint, not O(n)

Phase 3 - Scene Retention
  +-- 04.1-04.2: Per-widget scene fragment storage
  +-- 04.3-04.4: Scene patching (repaint dirty fragments, keep clean ones)
  Gate: Scroll doesn't rebuild entire Scene — only changed widgets repaint

Phase 4 - GPU-Side Scroll  [ADVANCED]
  +-- 05.1-05.3: Offscreen texture for scroll content
  +-- 05.4: Texture blit on scroll, strip-paint for new content
  Gate: Scroll produces <2ms frame time (texture blit + strip paint)

Phase 5 - Integration & Verification
  +-- 06: Wire incremental pipeline into dialog rendering
  +-- 07: Benchmarks, regression tests, visual verification
  Gate: All tests pass, frame time <8ms, zero visual regressions
```

**Why this order:**
- Phase 0 fixes a correctness bug (empty prepaint bounds) that affects current behavior.
- Phase 1 provides immediate relief with no architectural changes.
- Phase 2 builds the dirty tracking infrastructure that Phases 3-4 depend on.
- Phase 3 is the core architectural change — retained scene with partial updates.
- Phase 4 is advanced GPU optimization that makes scroll near-free.
- Phase 5 validates everything.

## Metrics (Current State)

**Per-scroll-frame breakdown (estimated from code analysis):**

| Phase | Work | Cost |
|-------|------|------|
| Layout (cache miss) | Full tree layout computation | ~2-5ms |
| Prepare | Walk all ~37 widgets, deliver lifecycle | ~0.5ms |
| Prepaint | Walk all ~37 widgets, resolve visual state | ~0.5ms |
| Paint | Walk all widgets, build primitives (culling exists but all visible widgets repaint) | ~3-8ms |
| Scene convert | Convert ~200-500 primitives to GPU instances | ~1-2ms |
| GPU submit + present | Render pass + present | ~1-3ms |
| **Total** | | **~8-19ms** |

**Target:** <4ms per scroll frame (viewport cull + selective walk + retained scene).

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On | File Split Required? |
|---------|-----------|------------|------------|---------------------|
| 01 Viewport Culling Verification | ~50 | Low | — | No |
| 02 Selective Tree Walks | ~350 | Medium-High | — | Yes: `pipeline/mod.rs` -> `prepare.rs` + `prepaint.rs` |
| 03 Layout Cache Unification | ~250 | Medium | — | No (but touches 5+ files) |
| 04 Retained Scene | ~600 | High | 02 | Yes: new `scene/fragment.rs` + `draw/fragment_cache.rs` |
| 05 GPU-Side Scroll | ~700 | Very High | 01, 04 | Yes: new `gpu/scroll_composite/` module |
| 06 Dialog Integration | ~250 | Medium | 01-05 | Yes: `dialog_rendering.rs` -> `dialog_rendering/` dir |
| 07 Verification | ~200 | Low | 06 | No |
| **Total new** | **~2400** | | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| Scroll stutter in settings dialog | Full scene rebuild per frame | Sections 01-04 | Not Started |
| `build_scene()` clears chrome before content paint | Double `scene.clear()` call | Fixed in current session | Fixed |
| Text/icons not GPU-clipped | `CLIP_UNCLIPPED` hardcoded | Fixed in current session | Fixed |
| Footer content overflow | Missing body clip | Fixed in current session | Fixed |
| Empty prepaint_bounds in compose_dialog_widgets | `HashMap::new()` instead of populated bounds map | Section 03.3b | Not Started |
| Empty prepaint_bounds in redraw/mod.rs | Same bug as above, also in main window tab bar rendering | Section 03.3b | Not Started |
| Empty prepaint_bounds in redraw/multi_pane/mod.rs | Same bug as above, also in multi-pane tab bar rendering | Section 03.3b | Not Started |
| MouseMove handler drops controller actions | `dispatch_dialog_content_move` dispatched events but dropped `result.actions` (DragUpdate/ValueChanged) and didn't call `apply_dispatch_requests` | Fixed in current session | Fixed |
| Slider/toggle hit areas too small | No `interact_radius` on toggle (40x22) or slider layouts | Fixed in current session | Fixed |
| Stale layout cache after page switch | `cached_layout` not invalidated after `accept_action(Selected)` | Fixed in current session | Fixed |
| ContainerWidget::accept_action short-circuits | `Iterator::any()` stopped at first handler, preventing PageContainer from seeing Selected | Fixed in current session | Fixed |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Viewport Culling Verification | `section-01-viewport-culling.md` | Not Started |
| 02 | Selective Tree Walks | `section-02-selective-tree-walks.md` | Not Started |
| 03 | Layout Cache Unification | `section-03-layout-cache-unification.md` | Not Started |
| 04 | Retained Scene & Dirty Regions | `section-04-retained-scene.md` | Not Started |
| 05 | GPU-Side Scroll | `section-05-gpu-scroll.md` | Not Started |
| 06 | Dialog Rendering Integration | `section-06-dialog-integration.md` | Not Started |
| 07 | Verification & Benchmarks | `section-07-verification.md` | Not Started |
