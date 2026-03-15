---
plan: "ui-framework-overhaul"
title: "UI Framework Overhaul: Exhaustive Implementation Plan"
status: queued
references:
  - "plans/gui-framework-research.md"
  - "mockups/settings.html"
---

# UI Framework Overhaul: Exhaustive Implementation Plan

## Mission

Transform `oriterm_ui` from an ad-hoc widget collection into a production-grade retained-mode
GUI framework with framework-managed interaction state, composable event controllers, implicit
animations, and visual state management — borrowing the best architectural patterns from Druid,
GTK4, WPF, SwiftUI, QML, Flutter, and egui. The settings panel (matching `mockups/settings.html`)
is the first consumer, but the framework must be general-purpose.

## Architecture

```
                          ┌─────────────────────────────────────┐
                          │           Application Layer          │
                          │   (App owns state, emits actions)    │
                          └──────────────┬──────────────────────┘
                                         │
                          ┌──────────────▼──────────────────────┐
                          │         Widget Tree                  │
                          │  ┌─────────────────────────────┐    │
                          │  │  Widget Trait (new shape)    │    │
                          │  │  - sense()                   │    │
                          │  │  - controllers()             │    │
                          │  │  - visual_states()           │    │
                          │  │  - layout()                  │    │
                          │  │  - paint()                   │    │
                          │  └─────────────────────────────┘    │
                          └──────────┬────────┬─────────────────┘
                                     │        │
                     ┌───────────────▼─┐  ┌───▼───────────────┐
                     │ Interaction     │  │ Visual State      │
                     │ State Manager   │  │ Manager           │
                     │                 │  │                   │
                     │ Hot/Active/     │  │ State groups      │
                     │ Focus trifecta  │  │ with animated     │
                     │ (per-widget)    │  │ transitions       │
                     └───────┬─────────┘  └────────┬──────────┘
                             │                     │
                     ┌───────▼─────────────────────▼──────────┐
                     │         Event Pipeline                  │
                     │                                         │
                     │  Hit Test (Sense filter + interact_r)   │
                     │       │                                 │
                     │  Capture Phase (root → target)          │
                     │       │                                 │
                     │  Target Phase                           │
                     │       │                                 │
                     │  Bubble Phase (target → root)           │
                     │       │                                 │
                     │  Controllers: Hover, Click, Drag,       │
                     │               Scroll, Focus             │
                     └─────────────────┬───────────────────────┘
                                       │
                     ┌─────────────────▼───────────────────────┐
                     │         Animation Engine                 │
                     │                                         │
                     │  AnimFrame(delta) timing pulses          │
                     │  Property Behaviors (implicit anim)      │
                     │  Transactions (animation metadata)       │
                     │  Spring physics                          │
                     │  request_anim_frame() / request_paint()  │
                     └─────────────────┬───────────────────────┘
                                       │
                     ┌─────────────────▼───────────────────────┐
                     │         DrawList → GPU                   │
                     │  (unchanged — already GPU-agnostic)      │
                     └─────────────────────────────────────────┘
```

## Design Principles

**1. Framework-managed state, not widget-managed state.**
Every widget currently hand-rolls its own `hovered: bool`, `hover_progress: AnimatedValue`,
enter/leave detection. This leads to duplicated logic, inconsistent hover behavior, and bugs
(e.g., fast mouse movement missing leave events). The framework should own Hot/Active/Focus
state and compute it automatically from layout geometry. Widgets query `ctx.is_hot()` — they
never track mouse position themselves.

**2. Composition over inheritance.**
GTK4 proved that decomposing event handling into composable controller objects (HoverController,
DragController, ClickController) is strictly better than monolithic `fn event()` methods.
Widgets compose behavior by attaching controllers. Controllers are independently testable
and reusable across widget types.

**3. Animation is metadata, not mechanism.**
SwiftUI and QML showed that widgets shouldn't manage animation controllers. A widget declares
animatable properties. The framework handles interpolation. State changes carry animation
metadata (Transactions). Properties have optional Behaviors that auto-animate changes. Visual
State Manager resolves which state is active and transitions between them.

## Section Dependency Graph

```
  01 Interaction State ──┐
                         ├──→ 03 Event Propagation ──→ 04 Event Controllers ──┐
  02 Sense & Hit Test ───┘                                                     │
                                                                               ├──→ 08 Widget Trait
  05 Animation Engine ──→ 06 Visual State Manager ────────────────────────────┘       Migration
                                                                               │
  07 Layout Extensions ────────────────────────────────────────────────────────┤
                                                                               │
                                                                               ├──→ 09 New Widgets
  (07b Theme Extension ─ folded into Section 07) ─────────────────────────────┤
                                                                               │
                                                                               └──→ 10 Settings
                                                                                      Panel
                                                                                        │
                                                                                        ▼
                                                                                   11 Verification
```

- Sections 01, 02 are foundation — everything depends on them.
- Sections 05, 07 are independent of the event system and can be built in parallel.
- Section 08 (Widget Trait Migration) is the convergence point — requires Sections 01-06.
- Sections 09, 10 are consumers — they use the new framework.
- Section 11 verifies everything works together.

**Cross-section interactions (must be co-implemented):**
- **Section 01 + 03**: Hot state computation requires the event propagation pipeline to know
  which widgets the pointer is over. If only one lands, hover tracking breaks.
- **Section 05 + 06**: Visual State Manager depends on the animation engine for transitions.
  Implementing VSM without the animation engine produces instant state changes (no interpolation).
- **Section 08 + 09**: Widget trait migration and new widgets must use the same trait shape.
  Can't build new widgets on the old trait while migrating the old ones to the new trait.

**Additional sync points (types/enums that span multiple sections):**
- **`WidgetAction` enum** (widgets/mod.rs): Section 04 may add `DoubleClicked`/`TripleClicked`.
  Section 10 adds `ResetDefaults`. All callers in `oriterm` crate must handle new variants.
- **`IconId` enum** (icons/mod.rs): Section 09 adds 8 new variants. The enum's impl block
  has exactly 1 method (`path()`) with a single match arm. `Debug` is derived (no manual impl).
  The test file (`icons/tests.rs`) must be updated alongside the match arm and any new
  `static` icon path constants.
- **`BoxContent` enum** (layout/layout_box.rs): Section 07 adds `Grid` variant. The solver
  match in `solver.rs` must handle it.
- **`AnimCurve` enum** (animation/behavior.rs): Section 05 introduces `AnimCurve` wrapping
  `Easing` and `Spring` as separate variants. `AnimBehavior` uses `AnimCurve` instead of
  separate `duration` + `easing` fields. The existing `Easing` enum is unchanged.
- **`Widget` trait** (widgets/mod.rs): Section 08 adds 7+ new methods and removes 3. This is
  the single largest breaking change — must use the additive-then-remove strategy.
- **`InputState` struct** (input/routing.rs): Removed in Section 03. Callers are internal
  to `oriterm_ui` only (routing.rs definition + tests.rs). `OverlayManager::process_mouse_event()`
  is a separate method on a different type and is updated independently.
- **`EventResponse` enum** (input/event.rs): Removed in Section 08. All callers across
  both `oriterm_ui` and `oriterm` crates must be updated.
- **`Widget` trait** consumers in `oriterm` crate: `TerminalGridWidget`
  (`oriterm/src/widgets/terminal_grid/mod.rs`) and `TerminalPreviewWidget`
  (`oriterm/src/widgets/terminal_preview/mod.rs`) also implement `Widget`. These are NOT
  in `oriterm_ui` — they live in the binary crate. Section 08 must migrate these too.
- **`FocusManager`** (focus/mod.rs): Section 01 `InteractionManager` delegates to it.
  `FocusManager` retains tab-order cycling (`focus_next`/`focus_prev`).
  `InteractionManager` calls `FocusManager::set_focus()`/`clear_focus()` internally
  and generates `FocusChanged` lifecycle events from the state change.
- **`OverlayManager` event routing** (overlay/manager/event_routing.rs, 333 lines):
  Has its own `process_mouse_event()`, `process_hover_event()`, `process_key_event()`
  that dispatch events to overlay widgets. Must be updated alongside Section 03's event
  pipeline — overlay widgets cannot remain on the old event system while main widgets
  use the new one. `process_mouse_event()` is called from `oriterm/src/app/mouse_input.rs`
  (3 call sites: button, cursor move, scroll) and
  `oriterm/src/app/dialog_context/event_handling/mouse.rs` (1 call site).
  `process_key_event()` is called from `oriterm/src/app/keyboard_input/mod.rs`.
  `process_hover_event()` is only exercised in `oriterm_ui` tests — it has no call site
  in the binary crate as of this writing.

## Implementation Sequence

```
Phase 0 — Foundation
  ├── 01: Interaction State System (Hot/Active/Focus)
  └── 02: Sense Declaration & Hit Testing

Phase 1 — Event Pipeline
  ├── 03: Event Propagation (Capture + Bubble)
  └── 04: Event Controllers (Hover, Click, Drag, Scroll, Focus)

Phase 2 — Animation (parallel with Phase 1)
  ├── 05: Animation Engine (AnimFrame, Behaviors, Springs)
  └── 06: Visual State Manager

Phase 3 — Layout & Theme (parallel with Phases 1-2)
  └── 07: Layout Extensions & Theme

Phase 4 — Convergence  [CRITICAL PATH]
  └── 08: Widget Trait Overhaul & Migration

Phase 5 — Consumers
  ├── 09: New Widget Library
  └── 10: Settings Panel Rebuild

Phase 6 — Verification
  └── 11: Verification & Polish
```

**Why this order:**
- Phases 0-1 establish the event infrastructure that all widgets will use.
- Phase 2 can run in parallel because animations are orthogonal to event propagation.
- Phase 3 can run in parallel — layout and theme changes don't depend on event/animation work.
- Phase 4 is the critical path: it unifies all prior work into the new Widget trait and migrates
  every existing widget. Nothing after this can start until it's done.
- Phases 5-6 are consumers of the framework.

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 Interaction State | ~400 | Medium | — |
| 02 Sense & Hit Testing | ~250 | Medium | 01 |
| 03 Event Propagation | ~500 | High | 01, 02 |
| 04 Event Controllers | ~600 | Medium | 01, 02, 03 |
| 05 Animation Engine | ~500 | High | — |
| 06 Visual State Manager | ~400 | High | 05 |
| 07 Layout & Theme | ~500 | Medium | — |
| 08 Widget Trait Migration (25 widgets) | ~900 | High | 01-07 |
| 09 New Widget Library | ~1500 | Medium | 08 |
| 10 Settings Panel Rebuild | ~800 | Medium | 09 |
| 11 Verification | ~600 | Medium | 10 |
| **Total new** | **~6950** | | |
| **Total deleted** | **~1500** | | |

**Dependency changes** (oriterm_ui/Cargo.toml):
- Section 02: Either add `bitflags = "2"` or implement Sense manually (4 flags).
  Same decision applies to `ControllerRequests` in Section 04 (5 flags).
- No other new crate dependencies expected. `smallvec`, `log` already present.

**Module declarations** (`oriterm_ui/src/lib.rs`): The following new modules must be
declared as they are created:
- Section 01: `pub mod interaction;`
- Section 04: `pub mod controllers;`
- Section 06: `pub mod visual_state;`

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Interaction State System | `section-01-interaction-state.md` | Not Started |
| 02 | Sense & Hit Testing | `section-02-sense-hit-testing.md` | Not Started |
| 03 | Event Propagation | `section-03-event-propagation.md` | Not Started |
| 04 | Event Controllers | `section-04-event-controllers.md` | Not Started |
| 05 | Animation Engine | `section-05-animation-engine.md` | Not Started |
| 06 | Visual State Manager | `section-06-visual-state-manager.md` | Not Started |
| 07 | Layout Extensions & Theme | `section-07-layout-theme.md` | Not Started |
| 08 | Widget Trait Overhaul | `section-08-widget-trait.md` | Not Started |
| 09 | New Widget Library | `section-09-new-widgets.md` | Not Started |
| 10 | Settings Panel Rebuild | `section-10-settings-rebuild.md` | Not Started |
| 11 | Verification | `section-11-verification.md` | Not Started |
