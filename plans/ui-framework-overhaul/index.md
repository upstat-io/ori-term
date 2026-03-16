---
reroute: true
name: "UI Framework"
full_name: "UI Framework Overhaul"
status: active
order: 1
---

# UI Framework Overhaul Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **References:** `plans/gui-framework-research.md`, `mockups/settings.html`

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

### Section 01: Interaction State System
**File:** `section-01-interaction-state.md` | **Status:** In Progress

```
hot, active, focus, hover, pressed, trifecta, is_hot, is_active, is_focused
hot_direct, focus_within, is_disabled, contains_pointer, is_pointer
InteractionState, InteractionManager, LifecycleEvent
HotChanged, FocusChanged, ActiveChanged, WidgetAdded, WidgetRemoved, WidgetDisabled
layout_hit_test_path, build_parent_map, parent_map, set_parent_map
register_widget, deregister_widget, update_hot_path, drain_events
DrawCtx, EventCtx, widget_id, for_child, FocusManager, set_active, request_focus
Druid, lifecycle, state tracking, automatic, framework-managed
oriterm_ui/src/interaction/, oriterm_ui/src/input/hit_test.rs
```

---

### Section 02: Sense & Hit Testing
**File:** `section-02-sense-hit-testing.md` | **Status:** Complete

```
sense, hit_test, interact_radius, click, drag, hover, none, focusable
Sense, HitTestBehavior, opaque, translucent, defer_to_child
WidgetHitTestResult, HitEntry, widget_ids
layout_hit_test, layout_hit_test_path, layout_hit_test_clipped
LayoutNode sense/hit_test_behavior/clip fields
LayoutBox sense/hit_test_behavior/clip fields, solver propagation
egui, Flutter, hit testing pipeline
oriterm_ui/src/sense.rs, oriterm_ui/src/hit_test_behavior.rs
oriterm_ui/src/input/hit_test.rs, oriterm_ui/src/layout/layout_node.rs
oriterm_ui/src/layout/layout_box.rs, oriterm_ui/src/layout/solver.rs
```

**Note:** `HitTestResult` is NOT a type introduced by this section — it is an existing
type in `oriterm_ui/src/hit_test/mod.rs` that represents window chrome regions
(Client/Caption/ResizeBorder). The widget hit test result type is `WidgetHitTestResult`
(distinct name). `layout_hit_test_path` returns `WidgetHitTestResult` in **root-to-leaf**
order, matching `update_hot_path`'s expectation.

---

### Section 03: Event Propagation
**File:** `section-03-event-propagation.md` | **Status:** In Progress

```
capture, bubble, tunnel, preview, propagation, phase, routing, dispatch
InputEvent, EventPhase, Capture, Bubble, Target, set_handled, stop_propagation
DeliveryAction, plan_propagation, DispatchResult, delivery_loop
focus_ancestor_path, keyboard_routing, active_widget, capture_bypass
routed_event, event_dispatch, event_pipeline, event_flow
CaptureRequest, EventResponse, WidgetResponse, transition_bridge
WPF, GTK4, DOM, preview_mouse_down
oriterm_ui/src/input/dispatch/mod.rs, oriterm_ui/src/input/event.rs
oriterm_ui/src/input/routing.rs (removed)
```

---

### Section 04: Event Controllers
**File:** `section-04-event-controllers.md` | **Status:** In Progress

```
controller, hover_controller, click_controller, drag_controller
scroll_controller, focus_controller, composable, reusable
EventController, handle_event, handle_lifecycle, phase, reset
ControllerCtx, ControllerCtxArgs, ControllerRequests, PropagationState
DispatchOutput, dispatch_to_controllers, dispatch_lifecycle_to_controllers
emit_action, set_handled, is_handled, bitmask, requests
WidgetAction relocation, action.rs, module dependency cycle
HoverController, on_enter, on_leave, on_move, HotChanged
ClickController, click_count, press_pos, click_threshold, multi_click_timeout
DoubleClicked, TripleClicked, click_drag_handoff
DragController, DragState, Idle, Pending, Dragging, drag_threshold
DragStart, DragUpdate, DragEnd, total_delta, WidgetDisabled reset
ScrollController, ScrollBy, ScrollDelta, line_height, Lines, Pixels
FocusController, tab_index, FOCUS_NEXT, FOCUS_PREV, REQUEST_FOCUS
KeyDown(Tab), KeyUp(Tab), focus_on_click, tab_navigation
GtkEventControllerMotion, GtkGestureClick, GtkGestureDrag
oriterm_ui/src/controllers/, oriterm_ui/src/action.rs
```

---

### Section 05: Animation Engine
**File:** `section-05-animation-engine.md` | **Status:** Not Started

```
animation, anim_frame, request_anim_frame, request_paint, delta_time
behavior, property_behavior, implicit_animation, explicit_animation
transaction, animation_metadata, spring, spring_physics, damping
easing, cubic_bezier, ease_in, ease_out, ease_in_out
AnimFrame, AnimationBehavior, Transaction, SpringAnimation
interpolation, lerp, animatable, animatable_property
oriterm_ui/src/animation/
```

---

### Section 06: Visual State Manager
**File:** `section-06-visual-state-manager.md` | **Status:** Not Started

```
visual_state, state_group, state_transition, common_states
normal, hovered, pressed, disabled, focused, unfocused
VisualStateManager, VisualStateGroup, VisualState, StateTransition
property_per_state, animated_transition, go_to_state
WPF, VisualStateManager, ControlTemplate
oriterm_ui/src/visual_state/
```

---

### Section 07: Layout Extensions & Theme
**File:** `section-07-layout-theme.md` | **Status:** Not Started

```
grid_layout, auto_fill, grid_columns, grid_gap, rich_text, styled_span
number_input, range_slider, page_container, sidebar_layout
UiTheme, bg_input, bg_card, text_faint, accent_bg
theme_tokens, style_resolution, widget_style
oriterm_ui/src/layout/, oriterm_ui/src/theme/
```

---

### Section 08: Widget Trait Overhaul
**File:** `section-08-widget-trait.md` | **Status:** Not Started

```
widget_trait, migration, sense, controllers, visual_states, paint
handle_mouse, handle_hover, handle_key, deprecated, new_trait
DrawCtx, EventCtx, LayoutCtx, is_hot, is_active, is_focused
Button, Toggle, Checkbox, Dropdown, Slider, TextInput, Scroll
Container, Panel, FormLayout, FormRow, FormSection
TerminalGridWidget, TerminalPreviewWidget (oriterm crate)
oriterm_ui/src/widgets/, oriterm/src/widgets/
```

---

### Section 09: New Widget Library
**File:** `section-09-new-widgets.md` | **Status:** Not Started

```
sidebar_nav, page_container, setting_row, scheme_card
color_swatch_grid, special_color_swatch, code_preview
cursor_picker, keybind_row, kbd_badge, number_input
range_slider, status_badge, nav_item, active_indicator
icon, svg, vector_icon
oriterm_ui/src/widgets/
```

---

### Section 10: Settings Panel Rebuild
**File:** `section-10-settings-rebuild.md` | **Status:** Not Started

```
settings, settings_panel, settings_dialog, form_builder
sidebar, page_router, appearance, colors, font, terminal
keybindings, window, bell, rendering, config
scheme_grid, palette_editor, font_preview, cursor_selector
save, cancel, reset_defaults, settings_ids
oriterm/src/app/settings_overlay/, dialog_management.rs, dialog_context/content_actions.rs
form_builder split: mod.rs + per-page submodules (appearance.rs, colors.rs, etc.)
```

---

### Section 11: Verification
**File:** `section-11-verification.md` | **Status:** Not Started

```
test, verification, visual_regression, performance, idle_cpu
animation_test, hover_test, controller_test, state_test
test_measurer, test_backend, snapshot, frame_time
build_all, clippy_all, test_all
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Interaction State System | `section-01-interaction-state.md` |
| 02 | Sense & Hit Testing | `section-02-sense-hit-testing.md` |
| 03 | Event Propagation | `section-03-event-propagation.md` |
| 04 | Event Controllers | `section-04-event-controllers.md` |
| 05 | Animation Engine | `section-05-animation-engine.md` |
| 06 | Visual State Manager | `section-06-visual-state-manager.md` |
| 07 | Layout Extensions & Theme | `section-07-layout-theme.md` |
| 08 | Widget Trait Overhaul | `section-08-widget-trait.md` |
| 09 | New Widget Library | `section-09-new-widgets.md` |
| 10 | Settings Panel Rebuild | `section-10-settings-rebuild.md` |
| 11 | Verification | `section-11-verification.md` |
