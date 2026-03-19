---
plan: "ui-framework-refinement"
title: "UI Framework Refinement: Best-in-Class Patterns"
status: in-progress
references:
  - "plans/ui-framework-distillation/summary.md"
  - "plans/ui-framework-overhaul/"
---

# UI Framework Refinement: Best-in-Class Patterns

## Mission

Incorporate the highest-impact patterns distilled from 6 reference GUI frameworks (egui, iced, GPUI, druid, masonry, makepad) into oriterm_ui's existing architecture. The framework is already well-designed — animation engine, controller decomposition, Sense declarations, and InteractionManager are confirmed best-in-class. This plan targets the **gaps**: headless testing, safety rails, scene abstraction, rendering pipeline split, action/keymap dispatch, **window-level composition**, and **architectural boundary enforcement**.

The test harness is the highest priority: it unblocks fast iteration on everything else. WindowRoot extraction (Section 07) is the architectural centerpiece: it makes every layer — widgets, interaction, focus, overlays, and windows — headless-testable.

## Architecture

```
                    ┌──────────────────────────────────┐
                    │     Test Harness (Section 01)     │
                    │  WidgetTestHarness wraps          │
                    │  WindowRoot (Section 07)          │
                    │  Input sim, state inspect, clock  │
                    └──────────┬───────────────────────┘
                               │ validates
          ┌────────────────────┼────────────────────┐
          │                    │                    │
┌─────────▼────────┐ ┌────────▼────────┐ ┌────────▼────────┐
│  Safety Rails    │ │ Scene Abstract  │ │ Action/Keymap   │
│  (Section 02)    │ │ (Section 03)    │ │ (Section 05)    │
│                  │ │                 │ │                 │
│ Debug assertions │ │ Type-separated  │ │ Actions as data │
│ on child visit,  │ │ Scene + damage  │ │ Keymap routing  │
│ double-visit,    │ │ tracking        │ │ Context-scoped  │
│ lifecycle order  │ │                 │ │ dispatch        │
└──────────────────┘ └────────┬────────┘ └─────────────────┘
                              │
                     ┌────────▼────────┐
                     │ Prepaint Phase  │
                     │ (Section 04)    │
                     │                 │
                     │ 3-pass render:  │
                     │ layout →        │
                     │ prepaint →      │
                     │ paint           │
                     └─────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          │                                       │
┌─────────▼────────────────┐  ┌───────────────────▼────┐
│  WindowRoot (Section 07) │  │ Pure Logic Migration   │
│                          │  │ (Section 08)           │
│  Per-window composition: │  │                        │
│  widget tree + interact  │  │ CursorBlink, resize,   │
│  + focus + overlay +     │  │ cursor_hide, mark mode │
│  compositor + pipeline   │  │ motion → oriterm_ui    │
│                          │  │                        │
│  WidgetTestHarness wraps │  │ mux-coupled logic      │
│  WindowContext wraps     │  │ stays in oriterm       │
│  DialogContext wraps     │  │                        │
└──────────────────────────┘  └────────────────────────┘
                              │
                     ┌────────▼────────┐
                     │ Boundary Rules  │
                     │ (Section 09)    │
                     │                 │
                     │ Crate rules,    │
                     │ arch tests,     │
                     │ drift prevent   │
                     └─────────────────┘
```

## Design Principles

**1. Test-first infrastructure.**
Every framework except makepad has headless widget testing. We have none. This is the single biggest gap blocking fast iteration. The harness must let us create widgets, simulate input, inspect interaction state, control time, and verify rendering — all without a GPU or window. This unblocks everything else.

**2. Defensive correctness via safety rails.**
Masonry proved that debug assertions on tree traversal (all children visited, no double-visits, lifecycle ordering validated) catch the most common container bugs. These are cheap to add and high value: they turn silent logic errors into immediate panics with clear messages.

**3. Correct rendering architecture via type-separated Scene.**
GPUI collects paint primitives into typed arrays (quads, text, paths) with per-primitive resolved state (ContentMask). The type-separated Scene (completed in Section 03) replaced DrawList: primitives carry resolved clip rects, GPU consumes typed arrays directly, and per-widget damage tracking via `DamageTracker` identifies changed regions between frames. Opacity is applied at GPU conversion time via `base_opacity`, not per-primitive.

## Section Dependency Graph

```
  01.2a Pipeline Move ──────────────────────────────────────┐
       (PREREQUISITE — move shared pipeline to oriterm_ui)  │
                                                            │
  01 Test Harness ──────────────────────────────────────────┤
       (depends on 01.2a for shared pipeline functions)     │
                                                            │
  02 Safety Rails ──────────────────────────────────────────┤
       (depends on 01.2a for assertion placement)           │
                                                            │
  03 Scene Abstraction ─ ─ → 04 Prepaint Phase ─────────────┤
       (recommended, not required)                          │
                                                            │
  04 Prepaint Phase ────────────────────────────────────────┤
       (depends on 01.2a for pipeline extension)            │
                                                            │
  05 Action/Keymap System ──────────────────────────────────┤
       (independent of rendering changes)                   │
                                                            │
  07 WindowRoot Extraction ─────────────────────────────────┤
       (depends on 01 — extracts from test harness)         │
                                                            │
  08 Pure Logic Migration ──────────────────────────────────┤
       (depends on 07 — WindowRoot is the target)           │
                                                            │
  09 Boundary Enforcement ──────────────────────────────────┤
       (depends on 07, 08 — enforces what they establish)   │
                                                            │
                                                            ▼
                                                     06 Verification
                                                       (requires all)
```

- **01.2a (pipeline move)** is a prerequisite for Sections 01, 02, and 04.
- Sections 01, 02, 03, 04, 05 are **parallelizable** after 01.2a (with 03 before 04 recommended).
- Section 05 is **fully independent** — no dependency on 01.2a or any other section.
- **Section 07 depends on 01** — WindowRoot extracts from the test harness pattern established in 01.
- **Section 08 depends on 07** — migrated logic integrates with WindowRoot.
- **Section 09 depends on 07 + 08** — enforces the boundaries they establish.
- Section 06 requires all prior sections (updated to include 07-09).

**Cross-section interactions:**
- **Section 01 + 02**: Test harness validates that safety rail assertions fire correctly. Tests for safety rails ARE harness tests.
- **Section 03 + 04**: Scene abstraction and prepaint phase are complementary but not blocking. Scene benefits from `DirtyKind::Prepaint` (Section 04) for finer damage tracking. Prepaint works without Scene.
- **Section 01.2a prerequisite:** Moving shared pipeline functions (`prepare_widget_tree`, `register_widget_tree`, `collect_focusable_ids`, `apply_dispatch_requests`, `DispatchResult`, `dispatch_step`) from `oriterm/src/app/widget_pipeline/` to `oriterm_ui/src/pipeline.rs` is a prerequisite for the test harness, safety rails, and prepaint phase. This is a mechanical refactor (~290 lines moved, visibility changed from `pub(crate)`/`pub(super)` to `pub`).
- **Section 05 + existing controllers:** The keymap system coexists with existing EventControllers during migration. Keymap lookup runs first; unmatched keys fall through to controllers. Controllers are removed one at a time after keymap migration is verified.
- **Section 07 + 01**: WindowRoot is the formalization of what `WidgetTestHarness` already does — wrapping widget tree + framework state. The harness is refactored to wrap WindowRoot.
- **Section 07 + app layer**: `WindowContext` and `DialogWindowContext` are decomposed to wrap WindowRoot, eliminating ~30 lines of duplicated framework wiring per window type.
- **Section 08 + 09**: Pure logic migration (08) establishes correct boundaries; enforcement (09) prevents drift back.

## Implementation Sequence

```
Phase 0a — Pipeline Foundation (must come first within Phase 0)
  └── 01.2a: Move shared pipeline functions to oriterm_ui/src/pipeline.rs

Phase 0b — Foundation (can parallelize after 0a)
  ├── 01: Headless Test Harness (uses pipeline.rs)
  ├── 02: Safety Rails (assertions go in pipeline.rs)
  └── 05: Action/Keymap System

Phase 1 — Rendering Pipeline
  ├── 03: Scene Abstraction & Damage Tracking
  └── 04: Prepaint Phase (extends pipeline.rs)

Phase 2 — Architecture (sequential)
  ├── 07: WindowRoot Extraction (formalizes harness → reusable composition unit)
  ├── 08: Pure Logic Migration (moves UI logic to oriterm_ui)
  └── 09: Boundary Enforcement (codifies rules, adds arch tests)

Phase 3 — Verification
  └── 06: Verification & Polish (updated to cover 07-09)
```

**Why this order:**
- Phase 0a (pipeline move) is a prerequisite for Sections 01, 02, and 04 — all of which
  need the shared pipeline functions in `oriterm_ui`. It is a low-risk mechanical refactor.
- Phase 0b items are independent additions to existing code. No behavioral changes.
- Section 01 (test harness) enables fast iteration on all subsequent work.
- Phase 1 must be sequential (Scene before Prepaint).
- Phase 2 is the architectural backbone: WindowRoot must exist before logic migrates to it,
  and boundaries must be correct before enforcement rules are written.
- Phase 3 validates everything works together (including the new architecture).

## Estimated Effort

| Section | Est. Lines | Files | Max File | Complexity | Depends On |
|---------|-----------|-------|----------|------------|------------|
| 01 Test Harness | ~1400 | 7 | ~250 | High | pipeline move (01.2a) |
| 02 Safety Rails | ~350 | 4 | ~200 | Medium | pipeline move (01.2a) |
| 03 Scene Architecture | ~2000 new, ~2000 modified, ~500 removed | 8 new + ~80 modified + ~5 removed | ~250 | High | -- |
| 04 Prepaint Phase | ~700 | 9 | ~200 | High | 03 (recommended), pipeline (01.2a) |
| 05 Action/Keymap System | ~900 | 5 | ~150 | Medium | -- |
| 07 WindowRoot Extraction | ~500 | 4 | ~300 | High | 01 |
| 08 Pure Logic Migration | ~475 moved, ~395 restructured | 12 | ~200 | Medium | 07 |
| 09 Boundary Enforcement | ~200 | 4 | ~150 | Low | 07, 08 |
| 06 Verification | ~600 | -- | -- | Medium | 01-09 |
| **Total new** | **~5020** | | | | |
| **Total deleted/moved** | **~875** | | | | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Headless Test Harness | `section-01-test-harness.md` | Complete |
| 02 | Safety Rails | `section-02-safety-rails.md` | Complete |
| 03 | Scene Architecture | `section-03-scene-abstraction.md` | Complete |
| 04 | Prepaint Phase | `section-04-prepaint-phase.md` | Complete |
| 05 | Action/Keymap System | `section-05-action-keymap.md` | Not Started |
| 06 | Verification | `section-06-verification.md` | Not Started |
| 07 | WindowRoot Extraction | `section-07-window-root.md` | Not Started |
| 08 | Pure Logic Migration | `section-08-pure-logic-migration.md` | Not Started |
| 09 | Boundary Enforcement | `section-09-boundary-enforcement.md` | Not Started |
