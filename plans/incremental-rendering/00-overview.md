---
plan: "incremental-rendering"
title: "Incremental Rendering Pipeline: Production-First Overview"
status: in-progress
references:
  - "plans/ui-framework-overhaul/"
  - "plans/roadmap/section-23-performance.md"
---

# Incremental Rendering Pipeline: Production-First Overview

## Mission

Reduce wasted UI work in the current production render paths without repeating the
"design the whole retained pipeline first, wire it in later" failure mode. Every section
in this plan must change the real app immediately: fix a correctness bug, remove a
redundant tree walk, skip work for a concrete interaction, or measurably reduce scroll
cost in the running dialog.

This plan does **not** start by building a new abstraction layer. It starts in
`oriterm/src/app/dialog_rendering.rs`, `oriterm/src/app/redraw/mod.rs`, and
`oriterm/src/app/redraw/multi_pane/mod.rs`, changes those production paths directly, and
only pulls in additional machinery when a step cannot be completed cleanly without it.

## Current System

```
Event
  -> Interaction / invalidation flags update
  -> about_to_wait
  -> App::render_dialog() or App::handle_redraw()
     -> choose coarse phase gate from DirtyKind
     -> prepare_widget_tree(whole tree for that render path)
     -> prepaint_widget_tree(whole tree for that render path)
     -> scene.clear()
     -> paint visible content into a fresh Scene
     -> convert full Scene to GPU instances
     -> submit full UI frame

Existing foundations already present:
  - WindowRoot / InteractionManager / InvalidationTracker
  - type-separated Scene
  - DamageTracker
  - some viewport culling in widget paint code
```

The problem is not that the framework lacks retained-mode primitives. The problem is that the
app-layer render paths still collapse back to full-tree, full-scene work too often. That
makes the system expensive for hover, page switching, and scroll even when only a small region
actually changed.

## Design Principles

**1. Every step must touch a production code path.**
No section is allowed to introduce types, fragment caches, scroll compositors, or patch APIs in
isolation. If a section cannot name the exact function it changes and the observable behavior it
improves, it is too abstract and must be rewritten.

**2. Fix correctness and cheap wins before architecture.**
The current renderers still pass empty prepaint-bounds maps in three production paths. That is a
real behavior bug today. Existing viewport culling and layout caching also need validation before
we assume a retained-scene rewrite is necessary.

**3. Escalate only when measurement justifies it.**
Selective tree walks and cache coordination are the first line of attack. Retained-scene patching
and GPU-side scroll are optional later stages, not mandatory upfront commitments. If the simpler
steps make dialog scrolling and hover cheap enough, stop there.

## Proposed Sections

These are the intended sections, ordered by production impact, not by architectural neatness.

| ID | Section | Production code path | Observable change |
|----|------|----------------------|-------------------|
| 01 | Current-path correctness | `render_dialog()`, `handle_redraw()`, `handle_redraw_multi_pane()` | Widgets receive correct prepaint bounds; no hidden geometry bugs during hover/focus |
| 02 | Dialog quick wins | `render_dialog()` + dialog event/layout helpers | Off-screen dialog content stops repainting; redundant dialog layout work is removed |
| 03 | Dialog selective walks | dialog prepare/prepaint/paint path | Hover and page-local changes stop walking the entire dialog tree |
| 04 | Main-window rollout | `handle_redraw()` + multi-pane redraw | Tab bar / chrome / pane UI get the same selective behavior |
| 05 | Verification & Measurement | all render paths (measurement only) | Measure cumulative impact of 01-04; data-driven go/no-go on retained scene / GPU scroll |

## Dependency Graph

```
01 Current-path correctness
  -> 02 Dialog quick wins
     -> 03 Dialog selective walks
        -> 04 Main-window rollout
           -> 05 Verification & Measurement
```

- `01` must land first because later optimization work is not trustworthy while prepaint is using
  incorrect bounds.
- `02` and `03` intentionally start in the dialog path only. The dialog is the smallest,
  easiest-to-measure production surface.
- `04` is a rollout step, not a new design step.
- `05` is a measurement and decision checkpoint. The verification test matrix (05.2) should always run. The advanced rendering decision (05.3) is data-driven — if Sections 02-04 already meet all performance targets, the decision is simply "no further work needed."

## Implementation Sequence

```text
Phase 1 - Fix What Is Wrong Today
  +-- Populate real prepaint bounds in dialog + single-pane + multi-pane renderers
  +-- Add focused tests that prove prepaint sees actual widget bounds
  Gate: hover/focus-dependent widgets read correct bounds in the running app

Phase 2 - Take The Cheap Wins In The Dialog Path
  +-- Verify and harden existing viewport culling in the dialog content tree
  +-- Remove redundant dialog layout recomputation where the current path can safely reuse work
  +-- Add metrics/logging/tests around dialog scroll and hover cost
  Gate: off-screen dialog content no longer emits primitives; dialog interactions show fewer tree walks

Phase 3 - Make Dialog Work Proportional To What Changed
  +-- Teach dialog prepare/prepaint to skip clean subtrees
  +-- If needed, teach dialog paint to stop rebuilding clearly clean subtrees
  +-- Keep all changes inside the live dialog rendering path
  Gate: a single-row hover or page-local control change does not traverse the entire dialog tree

Phase 4 - Roll The Same Strategy Into Main Windows
  +-- Apply the proven dialog strategy to `handle_redraw()`
  +-- Apply it again to multi-pane redraw
  +-- Keep behavior identical; this is a rollout, not a redesign
  Gate: terminal-adjacent UI chrome benefits from the same selective work rules

Phase 5 - Only Then Decide If Advanced Work Is Still Worth It
  +-- Re-profile scroll and hover after Phases 1-4
  +-- If still needed, prototype retained-scene patching for position-stable updates
  +-- If scroll is still the dominant cost, prototype a GPU-side scroll fast path
  Gate: measured improvement beats the simpler path enough to justify permanent complexity
```

## Why This Order

- The first section fixes an existing production bug instead of designing for a hypothetical future.
- The second and third sections operate on the smallest real surface that already hurts: the settings
  dialog.
- The fourth section reuses a proven production change instead of inventing a second system.
- The fifth section is a deliberate checkpoint against overengineering. If simpler changes solve the
  problem, advanced rendering work should not exist.

## Success Criteria

This plan is successful if the real app gets cheaper step by step without needing a massive
integration finale:

- After the first section, prepaint correctness bugs are gone.
- After the second section, dialog scrolling and hover avoid obviously wasted work.
- After the third section, small dialog interactions cost roughly the size of the changed subtree,
  not the whole dialog.
- After the fourth section, the same strategy covers the main UI renderers.
- After all of that, retained-scene patching or GPU scroll should exist only if profiles still say
  they are necessary.

## Known Problems To Start From

| Problem | Current location | Why it matters | Fixed in |
|--------|------------------|----------------|----------|
| Empty prepaint-bounds maps | `dialog_rendering.rs`, `redraw/mod.rs`, `redraw/multi_pane/mod.rs` | Current behavior bug; invalidates later optimization work | Section 01 |
| Full-tree prepare/prepaint on every interaction | `App::compose_dialog_widgets()` | Expensive hover/page-switch/scroll behavior (paint already has viewport culling in `FormLayout`/`FormSection`/`Container`, but prepare/prepaint walk the entire tree) | Sections 02-03 |
| Coarse dirty gating only | app-layer render paths + `InvalidationTracker` usage | `max_dirty_kind()` gates entire phases, but per-widget `mark()` is never called from production — `dirty_map` is always empty; only `full_invalidation` is used | Section 03 |
| `PageContainerWidget::for_each_child_mut()` visits ALL pages | `page_container/mod.rs:115-119` | `prepare_widget_tree()` and `prepaint_widget_tree()` walk hidden pages' entire widget trees every frame — single biggest source of wasted work in the dialog path | Section 02.2b/02.2c |
| `ScrollWidget` layout cache stale on page switch | `scroll/mod.rs:117,190-216` | **Confirmed bug:** Cache keyed on viewport `Rect`, not child identity. `reset_scroll()` (called on page switch) does NOT clear `cached_child_layout`. Page switch with same viewport bounds returns stale layout from previous page | Section 02.2 |
| Damage tracked after full rebuild | `DamageTracker` integration | Good primitive, but not yet reducing most CPU work | Section 03 (explored) |

## Scope Boundary

This overview intentionally stays narrow:

- It is a production-first rendering plan, not a new framework plan.
- It does not assume retained-scene patching is required.
- It does not commit to GPU-side scroll unless earlier sections fail to reach the target.

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Current-Path Correctness | `section-01-current-path-correctness.md` | Complete |
| 02 | Dialog Quick Wins | `section-02-dialog-quick-wins.md` | Not Started |
| 03 | Dialog Selective Walks | `section-03-dialog-selective-walks.md` | Not Started |
| 04 | Main-Window Rollout | `section-04-main-window-rollout.md` | Not Started |
| 05 | Verification & Measurement | `section-05-verification.md` | Not Started |
