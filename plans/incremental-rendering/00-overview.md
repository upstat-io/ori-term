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
Correctness bugs (empty prepaint-bounds maps) were fixed first (Section 01). Existing viewport
culling and layout caching were validated before assuming a retained-scene rewrite is necessary.

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
| 03 | Dialog selective walks | `tree_walk.rs` + `WindowRoot` pipeline methods | Selective walk infrastructure built and tested; WindowRoot callers use it; app-layer callers pass `None` (wired in 04) |
| 04 | App-layer wiring + main-window rollout | `dialog_rendering.rs`, `handle_redraw()`, `handle_redraw_multi_pane()` | All app-layer render paths pass real tracker references; dialog + tab bar + overlays use selective walks |
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
- `04` wires the proven library infrastructure into all app-layer render paths (dialog, single-pane, multi-pane). Requires a new borrow-split method on `WindowRoot`.
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

Phase 4 - Wire Selective Walks Into All App-Layer Render Paths
  +-- Add borrow-split method for (InteractionManager, InvalidationTracker, FrameRequestFlags)
  +-- Wire tracker into dialog path (compose_dialog_widgets) — closes Section 03's app-layer gap
  +-- Wire tracker into single-pane path (handle_redraw)
  +-- Wire tracker into multi-pane path (handle_redraw_multi_pane)
  +-- Verify overlay methods already pass Some (they do — no changes needed)
  Gate: no app-layer render path passes None for the tracker parameter

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
- The fourth section wires the proven library infrastructure into all production render paths (dialog + main windows) and verifies overlay methods are already correct.
- The fifth section is a deliberate checkpoint against overengineering. If simpler changes solve the
  problem, advanced rendering work should not exist.

## Success Criteria

This plan is successful if the real app gets cheaper step by step without needing a massive
integration finale:

- After the first section, prepaint correctness bugs are gone.
- After the second section, dialog scrolling and hover avoid obviously wasted work.
- After the third section, selective walk infrastructure is built, tested, and active in WindowRoot pipeline methods.
- After the fourth section, all app-layer render paths (dialog, single-pane, multi-pane) pass real tracker references. Small interactions cost roughly the size of the changed subtree.
- After all of that, retained-scene patching or GPU scroll should exist only if profiles still say
  they are necessary.

## Known Problems

**Open (blocking next sections):**

| Problem | Location | Why it matters | Addressed in |
|---------|----------|----------------|--------------|
| Per-widget dirty tracking built but not wired at app layer | `dialog_rendering.rs`, `redraw/mod.rs`, `redraw/multi_pane/mod.rs` | `mark()` is wired into the interaction pipeline (Section 03). `WindowRoot::prepare()`/`run_prepaint()` pass `Some(tracker)`. But all app-layer render paths still pass `None`, bypassing selective walks | Section 04.0 |
| Damage tracked after full rebuild | `DamageTracker` integration | Good primitive, but not yet reducing most CPU work | Section 03 (explored, not feasible without retained scene) |
| Dialog overlays skip prepare/prepaint | `dialog_rendering.rs:226-273` | `render_dialog_overlays()` only calls `layout_overlays()` + `draw_overlay_at()` -- overlay widgets may not receive lifecycle events or animator ticks. Pre-existing gap unrelated to selective walks | Out of scope (separate bug) |

**Resolved (by earlier sections):**

| Problem | Fixed in |
|---------|----------|
| Empty prepaint-bounds maps in all 3 render paths | Section 01 |
| Full-tree prepare/prepaint on every interaction | Sections 02-03 (library-level) |
| `PageContainerWidget::for_each_child_mut()` visited ALL pages | Section 02.2b/02.2c |
| `ScrollWidget` layout cache stale on page switch | Section 02.2 |
| Windows modal loop cleared invalidation before render | Section 03.1 |

## Scope Boundary

This overview intentionally stays narrow:

- It is a production-first rendering plan, not a new framework plan.
- It does not assume retained-scene patching is required.
- It does not commit to GPU-side scroll unless earlier sections fail to reach the target.

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Current-Path Correctness | `section-01-current-path-correctness.md` | Complete |
| 02 | Dialog Quick Wins | `section-02-dialog-quick-wins.md` | Complete |
| 03 | Dialog Selective Walks | `section-03-dialog-selective-walks.md` | Complete |
| 04 | App-Layer Wiring + Main-Window Rollout | `section-04-main-window-rollout.md` | In Progress |
| 05 | Verification & Measurement | `section-05-verification.md` | In Progress |
