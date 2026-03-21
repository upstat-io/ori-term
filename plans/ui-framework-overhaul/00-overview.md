---
plan: "ui-framework-overhaul"
title: "UI Framework Overhaul: Exhaustive Implementation Plan"
status: active
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
                     │  AnimFrameEvent timing pulses             │
                     │  AnimBehavior / AnimProperty (implicit)   │
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
