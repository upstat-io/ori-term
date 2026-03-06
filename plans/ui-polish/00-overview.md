---
plan: "ui-polish"
title: "2D Framework Polish: Z-Index, Clipping, Animation Quality"
status: not-started
references:
  - "plans/roadmap/section-43-compositor-layers.md"
  - "plans/roadmap/section-16-tab-bar.md"
  - "plans/roadmap/section-07-ui-framework.md"
---

# 2D Framework Polish: Z-Index, Clipping, Animation Quality

## Mission

Bring the UI framework from functional to Chrome-class: fix tab show-through via proper GPU clipping, connect the existing compositor infrastructure to the tab bar for correct z-ordering, and add smooth animated transitions for hover, close-button visibility, tab open/close, and drag elevation. The compositor layer tree, layer animator, and GPU compositor were built in Section 43 — this plan wires them into production use and fills the remaining gaps.

## Architecture

```
Current (broken):                       Target:

TabBarWidget                            TabBarWidget
  │                                       │
  ├─ draw() → flat DrawList               ├─ draw() → per-tab clip rects
  │   ├─ all inactive tabs                │   ├─ PushClip(tab_rect)
  │   ├─ active tab                       │   ├─ draw tab content
  │   └─ dragged tab (backing rect)       │   └─ PopClip
  │                                       │
  └─ no clipping, no layers               ├─ AnimatedValue<f32> for hover progress
                                          ├─ width animation for open/close
DrawList → convert_draw_list              └─ shadow on dragged tab
  │
  └─ PushClip/PopClip → log::trace!      DrawList → convert_draw_list
     (no-op — NEVER reaches GPU)            │
                                            └─ PushClip/PopClip → set_scissor_rect
                                               (GPU scissor rects active)
```

## Design Principles

1. **Fix the infrastructure gap first**: The `DrawList` has clip commands, the GPU converter ignores them. This is the root cause of tab show-through — not a tab bar bug. Fix the converter, and clipping works everywhere (tabs, overlays, scroll widgets, future features).

2. **Use what's already built**: Section 43 built `LayerTree`, `LayerAnimator` (in `oriterm_ui/src/compositor/`), `GpuCompositor` (in `oriterm/src/gpu/compositor/`), `AnimatedValue<T>`, `Easing` with cubic bezier, `AnimationGroup`, `AnimationSequence`. `Lerp` for `Color` and `Color::lerp()` already exist in `oriterm_ui/src/color/mod.rs`. This plan connects existing infrastructure — it does NOT rebuild it.

3. **Chrome-grade animation quality**: Every visual state change should animate. Instant swaps feel broken. Chrome uses `Tween::EASE_OUT` (~`CubicBezier(0.0, 0.0, 0.2, 1.0)`) with duration proportional to visual distance.

## Section Dependency Graph

```
01 GPU Scissor ──────→ 02 Tab Clipping ──→ 06 Verification
                                     │
03 Color Animation ──→ 04 Tab Lifecycle ──┤
   (structural split)   (depends on        │
                         03's split)       │
05 Drag Elevation ────────────────────────┘
```

- Section 01 (GPU scissor) is the prerequisite — clipping must work before tabs can use it.
- Sections 02, 03, and 05 are independent of each other once 01 lands.
- **Section 04 has a soft dependency on Section 03**: Both add `Vec` fields and methods to `widget/mod.rs` (468 lines). Section 03 must extract animation state into `widget/animation.rs` first, or Section 04 will push the file past 500 lines.
- Section 06 verifies everything together.

**Cross-section interactions:**
- **Section 01 + Section 02**: Clipping infrastructure + first consumer. Section 02 is trivial once 01 works, but useless without it.
- **Section 03 + Section 04**: Both add `Vec` fields to `TabBarWidget`. Section 03 adds `hover_progress` and `close_btn_opacity` Vecs; Section 04 adds `width_multipliers` and `closing_tabs` Vecs. All must be resized in `set_tabs()`. No direct conflict, but `set_tabs()` becomes a sync point for ALL parallel Vecs — implement carefully to avoid index drift.
- **Section 01 side effect**: The scroll widget (`oriterm_ui/src/widgets/scroll/`) already emits `PushClip`/`PopClip` in its draw list. Section 01 activates these clips for real — verify no visual regression in scroll content.
- **Section 04 scope warning**: Section 04 requires changing `TabBarLayout` from uniform-width to variable-width layout. This affects `tab_x()`, `tabs_end()`, `new_tab_x()`, `dropdown_x()`, `tab_index_at()`, and all consumers. Adding `Vec` fields also removes `Copy` from `TabBarLayout` — a breaking API change. This is the highest-risk change in the plan — it touches the layout contract used by hit testing, interactive rects, and drawing.
- **Section 03 → 04 ordering**: Both sections add `Vec` fields and methods to `widget/mod.rs` (468 lines). Section 03 must extract animation state into `widget/animation.rs` first, or Section 04 will push the file past the 500-line limit. This creates a soft ordering constraint despite the sections being logically independent.

## Implementation Sequence

```
Phase 0 — Foundation
  └── 01: GPU scissor rect support in convert_draw_list
          (extract clip.rs from draw_list_convert/mod.rs first — 425 lines)

Phase 1a — Independent Tab Bar Fixes
  ├── 02: Per-tab clip rects (fixes show-through)
  ├── 03: Animated hover + close-button transitions
  │        (extract widget/animation.rs — mod.rs is 468 lines)
  └── 05: Dragged tab shadow + remove backing rect hack

Phase 1b — Depends on 03's structural split
  └── 04: Tab open/close width + opacity animations
          (adds to widget/animation.rs; changes TabBarLayout from Copy → non-Copy)

Phase 2 — Verification
  └── 06: Visual regression, test matrix, performance
  Gate: ./test-all.sh + ./clippy-all.sh + ./build-all.sh all green
```

**Why this order:**
- Phase 0 is pure infrastructure — no behavioral change to existing rendering. Includes proactive file split.
- Phase 1a items are independent widget-level changes that each improve a specific visual gap.
- Phase 1b (Section 04) depends on Section 03's `widget/animation.rs` extraction to stay under 500 lines.
- Phase 2 validates the whole stack.

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 GPU Scissor | ~200 | High | — |
| 02 Tab Clipping | ~20 | Low | 01 |
| 03 Color Animation | ~120 | Medium | — |
| 04 Tab Lifecycle Anim | ~350 | High | 03 (file split) |
| 05 Drag Elevation | ~40 | Low | — |
| 06 Verification | ~100 | Low | All |
| **Total new** | **~830** | | |

**Estimate notes:**
- **Section 01** (~200): Multi-writer clip segments (`TierClips`), `record_draw_clipped` helper, `PreparedFrame` changes, 8 tests. Includes extracting `clip.rs` from `draw_list_convert/mod.rs` (currently 425 lines).
- **Section 03** (~120): Includes extracting `widget/animation.rs` from `widget/mod.rs` (currently 468 lines). This extraction is a prerequisite for Section 04.
- **Section 04** (~350): `TabBarLayout` API change (per-tab positions via cumulative sums, `Copy` removal), `tab_index_at()` binary search, `closing_tabs` Vec, `closing_complete()` method, `set_tabs()` Vec sync, 10 tests.

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| Tabs show through each other | `PushClip`/`PopClip` are no-ops in GPU converter | Section 01 | Not Started |
| Dragged tab uses opaque backing rect | No proper z-ordering for drag overlay | Section 05 | Not Started |
| Hover color is instant swap | No `AnimatedValue<f32>` hover progress in `TabBarWidget` — color resolves instantly | Section 03 | Not Started |
| Close button appears/disappears instantly | No opacity animation on show/hide | Section 03 | Not Started |
| Tab open/close has no width animation | Layout recomputes instantly | Section 04 | Not Started |
| Slide duration is fixed 150ms | Not proportional to distance | Section 04 | Not Started |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | GPU Scissor Rect Support | `section-01-gpu-scissor.md` | Not Started |
| 02 | Tab Bar Clipping | `section-02-tab-clipping.md` | Not Started |
| 03 | Color Lerp & Animated Hover | `section-03-color-animation.md` | Not Started |
| 04 | Tab Open/Close Animations | `section-04-tab-lifecycle-anim.md` | Not Started |
| 05 | Dragged Tab Elevation | `section-05-drag-elevation.md` | Not Started |
| 06 | Verification | `section-06-verification.md` | Not Started |
