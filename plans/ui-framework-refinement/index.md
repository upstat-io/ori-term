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
**File:** `section-01-test-harness.md` | **Status:** Complete

```
test, harness, headless, WidgetTestHarness, MockMeasurer
integration test, widget test, unit test, test infrastructure
input simulation, mouse_move, click, key_press, tab, drag
state inspection, is_hot, is_active, is_focused, WidgetRef
time control, advance_time, deterministic, animation test
paint capture, Scene, render, snapshot, visual regression
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

### Section 03: Type-Separated Scene Architecture
**File:** `section-03-scene-abstraction.md` | **Status:** Complete

```
Scene, type-separated, typed arrays, primitive types
Quad, TextRun, LinePrimitive, IconPrimitive, ImagePrimitive
ContentMask, resolved clip, shader-side clip, base_opacity at GPU convert time
push_quad, push_text, push_line, push_icon, push_image
push_clip, pop_clip, push_offset, pop_offset, push_layer_bg, pop_layer_bg
build_scene, full repaint, SceneCache removal, compose_scene removal
DamageTracker, per-widget hash, dirty region, damage tracking
DrawList removal, DrawCommand removal, ClipContext removal, ClipSegment removal
convert_scene, GPU typed array consumption, instance layout 96-byte
widget paint migration, DrawCtx scene field, state stack resolution
GPUI Scene, ContentMask pattern, instanced rendering
```

---

### Section 04: Prepaint Phase (3-Pass Rendering)
**File:** `section-04-prepaint-phase.md` | **Status:** Complete

```
prepaint, 3-pass, three-pass, rendering pipeline
layout caching, paint-only dirty, phase separation, phase gating
PrepaintCtx, visual state resolution, interaction state queries
DirtyKind::Prepaint, DirtyKind::Paint, FrameRequestFlags, prepaint_widget_tree
VisualStateAnimator, resolved fields, resolved_bg, widget migration
DrawCtx.interaction removal, gradual migration, atomic migration
overlay prepaint, for_each_widget_mut, overlay interaction state
test harness prepaint, harness render, deliver_lifecycle_events
control_state bypass, tab_bar control_state, WindowControlButton
InvalidationTracker HashMap, max_dirty_kind, DirtyKind Ord
app layer call sites, widget_pipeline re-export, multi_pane hygiene
container delegation, flat map, parallel tree walk, bounds resolution
GPUI Element request_layout prepaint paint
```

---

### Section 05: Action & Keymap System
**File:** `section-05-action-keymap.md` | **Status:** Complete

```
action, keymap, keybinding, keyboard shortcut, rebind
KeymapAction, Keystroke, KeyBinding, KeyContext, boxed_clone
context scoping, focus path, build_context_stack, key_context
Widget::key_context, Widget::handle_keymap_action, find_widget_mut
runtime rebinding, macro recording, accessibility
controller migration, KeyActivationController, DropdownKeyController
MenuKeyController, SliderKeyController, TextEditController, FocusController
coexistence, fallback, gradual migration, KeyUp suppression
TOML config deferred, hardcoded defaults, action module restructure
action/mod.rs, action/keymap/mod.rs, action/keymap_action/mod.rs, action/context.rs
TreeDispatchResult, FocusNext, FocusPrev, Activate, Dismiss, Confirm
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
LayerTree, LayerAnimator, InvalidationTracker, DamageTracker
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
| 03 | Type-Separated Scene Architecture | `section-03-scene-abstraction.md` |
| 04 | Prepaint Phase (3-Pass Rendering) | `section-04-prepaint-phase.md` |
| 05 | Action & Keymap System | `section-05-action-keymap.md` |
| 06 | Verification & Polish | `section-06-verification.md` |
| 07 | WindowRoot Extraction | `section-07-window-root.md` |
| 08 | Pure Logic Migration | `section-08-pure-logic-migration.md` |
| 09 | Architectural Boundary Enforcement | `section-09-boundary-enforcement.md` |
