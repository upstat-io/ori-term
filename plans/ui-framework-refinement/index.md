---
reroute: true
name: "UI Refinement"
full_name: "UI Framework Refinement: Best-in-Class Patterns"
status: active
order: 1
---

# UI Framework Refinement Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **References:** `plans/ui-framework-distillation/summary.md` (research basis)

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Headless Test Harness
**File:** `section-01-test-harness.md` | **Status:** Not Started

```
test, harness, headless, WidgetTestHarness, MockMeasurer
integration test, widget test, unit test, test infrastructure
input simulation, mouse_move, click, key_press, tab, drag
state inspection, is_hot, is_active, is_focused, WidgetRef
time control, advance_time, deterministic, animation test
paint capture, DrawList, render, snapshot, visual regression
widget query, find_by_name, widget_at, widgets_with_sense
RenderScheduler, animation scheduling, deferred repaint
overlay testing, OverlayManager, dialog, dropdown
pipeline functions, prepare_widget_tree, register_widget_tree
pipeline move, oriterm_ui/src/pipeline.rs, 01.2a prerequisite
harness split, harness_dispatch, harness_input, harness_inspect
masonry TestHarness, GPUI TestAppContext, egui kittest, iced Simulator
```

---

### Section 02: Safety Rails
**File:** `section-02-safety-rails.md` | **Status:** Complete

```
safety rails, debug assertion, child visitation, double-visit
cross-phase consistency, layout vs dispatch child mismatch
container validation, for_each_child_mut, tree traversal
explicit parameter passing, HashSet tracking, cfg(debug_assertions)
lifecycle validation, WidgetAdded ordering, is_registered
register_widget fix, idempotent registration
prepare_widget_tree signature change, &mut InteractionManager
collect_layout_widget_ids, check_cross_phase_consistency
masonry safety_rails.rs, debug_assert, panic message
pipeline directory module conversion, pipeline/tests.rs
```

---

### Section 03: Scene Abstraction & Damage Tracking
**File:** `section-03-scene-abstraction.md` | **Status:** Not Started

```
PaintScene, scene abstraction, damage tracking, dirty region
z-order, paint primitive, DrawCommand, DrawList, DamageTracker
incremental rendering, partial repaint, changed regions
SceneCache, compose_scene, InvalidationTracker, compositor
LayerTree, LayerAnimator, per-widget hash
GPUI Scene, BoundsTree, makepad DrawList, instanced rendering
```

---

### Section 04: Prepaint Phase (3-Pass Rendering)
**File:** `section-04-prepaint-phase.md` | **Status:** Not Started

```
prepaint, 3-pass, three-pass, rendering pipeline
layout caching, paint-only dirty, phase separation
PrepaintCtx, visual state resolution, interaction state queries
DirtyKind::Prepaint, FrameRequestFlags, prepaint_widget_tree
VisualStateAnimator, resolved fields, widget migration
DrawCtx.interaction removal, gradual migration
GPUI Element request_layout prepaint paint
```

---

### Section 05: Action & Keymap System
**File:** `section-05-action-keymap.md` | **Status:** Not Started

```
action, keymap, keybinding, keyboard shortcut, rebind
KeymapAction, Keystroke, KeyBinding, KeyContext
context scoping, focus path, dispatch tree
runtime rebinding, macro recording, accessibility
controller migration, KeyActivationController, DropdownKeyController
MenuKeyController, SliderKeyController, TextEditController, FocusController
coexistence, fallback, gradual migration
TOML config deferred, hardcoded defaults, action module restructure
action/mod.rs, action/keymap.rs, action/keymap_action.rs, action/context.rs
GPUI key_dispatch, DispatchTree, actions! macro
```

---

### Section 06: Verification & Polish
**File:** `section-06-verification.md` | **Status:** Not Started

```
verification, test matrix, performance validation
context capability audit, LayoutCtx, DrawCtx, PrepaintCtx
documentation, CLAUDE.md update
frame time, idle CPU, damage tracking benefit
```

---

### Section 07: WindowRoot Extraction
**File:** `section-07-window-root.md` | **Status:** Not Started

```
WindowRoot, per-window, composition unit
headless window, testable window, window testing
WidgetTestHarness unification, harness refactor
WindowContext decomposition, DialogWindowContext decomposition
widget tree ownership, framework state consolidation
InteractionManager ownership, FocusManager ownership, OverlayManager ownership
LayerTree, LayerAnimator, SceneCache, InvalidationTracker
TextMeasurer trait, MockMeasurer, compute_layout
dispatch_event, prepare, rebuild, pipeline methods
GPUI Window, masonry WindowRoot, druid Window<T>
thin shell, app layer, platform wiring
RenderScheduler production, FocusManager behavior change
borrow splitting, field destructuring, overlay priority routing
```

---

### Section 08: Pure Logic Migration
**File:** `section-08-pure-logic-migration.md` | **Status:** Not Started

```
pure logic, migration, move to oriterm_ui
CursorBlink, cursor blink, animation timer
cursor_hide, should_hide_cursor, HideContext
ResizeEdge, resize edge, hit test zone, resize cursor
consolidation, extraction, reusable geometry
FloatingDragState, HitZone, DividerDragState
drag state machine, floating drag, divider drag
ContextMenuState, context menu, menu builder
MarkAction, Motion, MarkModeResult, mark mode
handle_mark_mode_key, SelectionUpdate
pure function extraction, headless testable
oriterm_mux dependency blocker, PaneId, TabId, MarkCursor
```

---

### Section 09: Architectural Boundary Enforcement
**File:** `section-09-boundary-enforcement.md` | **Status:** Not Started

```
boundary enforcement, crate boundaries, architectural test
crate responsibility, ownership rules, litmus test
oriterm thin shell, oriterm_ui framework
GPU-free test, headless test, platform-free
architecture.rs, boundary validation
CLAUDE.md update, impl-hygiene update
crate-boundaries.md, rules file
drift prevention, regression guardrail
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Headless Test Harness | `section-01-test-harness.md` |
| 02 | Safety Rails | `section-02-safety-rails.md` |
| 03 | Scene Abstraction & Damage Tracking | `section-03-scene-abstraction.md` |
| 04 | Prepaint Phase (3-Pass Rendering) | `section-04-prepaint-phase.md` |
| 05 | Action & Keymap System | `section-05-action-keymap.md` |
| 06 | Verification & Polish | `section-06-verification.md` |
| 07 | WindowRoot Extraction | `section-07-window-root.md` |
| 08 | Pure Logic Migration | `section-08-pure-logic-migration.md` |
| 09 | Architectural Boundary Enforcement | `section-09-boundary-enforcement.md` |
