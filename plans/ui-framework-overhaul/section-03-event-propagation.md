---
section: "03"
title: "Event Propagation"
status: not-started
goal: "Capture + Bubble two-phase event propagation replaces the current single-pass dispatch"
inspired_by:
  - "WPF Preview/Bubble paired propagation (PresentationCore)"
  - "GTK4 Capture/Target/Bubble three-phase (gtk_event_controller.c)"
depends_on: ["01", "02"]
reviewed: false
sections:
  - id: "03.1"
    title: "Event Types & Routing"
    status: not-started
  - id: "03.2"
    title: "Propagation Pipeline"
    status: not-started
  - id: "03.3"
    title: "Active Widget Capture"
    status: not-started
  - id: "03.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Event Propagation

**Status:** Not Started
**Goal:** Input events flow through a two-phase pipeline: Capture (root → target) then Bubble
(target → root). Parents can intercept events before children see them. Active widgets capture
all mouse events until released. `set_handled()` stops propagation at any point.

**Context:** The current event system is single-pass: `InputState` (in `routing.rs`) routes
events to the deepest widget in the hit path, and the widget returns an `EventResponse`
(`Handled`, `Ignored`, `RequestPaint`, `RequestLayout`, or `RequestFocus`). Containers manually
forward events to children via `ContainerInputState`. There is no way for a parent to intercept
an event before its child sees it (e.g., a modal overlay suppressing input to background
content). The WPF Preview/Bubble pattern solves this elegantly.

**Reference implementations:**
- **WPF** `PresentationCore`: Every input event is a Preview+Bubble pair. PreviewMouseDown
  tunnels from root to target, then MouseDown bubbles back up.
- **GTK4** `gtk_event_controller.c`: Controllers declare their phase (CAPTURE, TARGET, BUBBLE).
  Multiple controllers per widget. Consumed events stop propagation.

**Depends on:** Section 01 (InteractionState), Section 02 (Hit Testing).

**WARNING — OverlayManager integration**: `oriterm_ui/src/overlay/manager/event_routing.rs`
(333 lines) is a SEPARATE event routing implementation used for popup overlays (dropdowns,
menus, dialogs). It has its own `process_mouse_event()`, `process_hover_event()`, and
`process_key_event()` methods that dispatch events to widgets inside overlays. This event
routing must also be updated to use the new propagation pipeline, or at minimum be made
compatible with it. The overlay manager is called from `oriterm/src/app/mouse_input.rs`
and `oriterm/src/app/dialog_context/event_handling/`. Failing to update it will leave
overlay widgets (dropdown menus, dialogs) on the old event system while main widgets use
the new one.

---

## 03.1 Event Types & Routing

**File(s):** `oriterm_ui/src/input/event.rs`

Unify all input events into a single routable event type.

- [ ] Define unified `InputEvent` enum:
  ```rust
  pub enum InputEvent {
      MouseDown { pos: Point, button: MouseButton, modifiers: Modifiers },
      MouseUp { pos: Point, button: MouseButton, modifiers: Modifiers },
      MouseMove { pos: Point, modifiers: Modifiers },
      Scroll { pos: Point, delta: ScrollDelta, modifiers: Modifiers },
      KeyDown { key: Key, modifiers: Modifiers },
      KeyUp { key: Key, modifiers: Modifiers },
  }
  ```
- [ ] Define `EventPhase` enum:
  ```rust
  pub enum EventPhase {
      /// Root → target. Parents see the event first. Can intercept.
      Capture,
      /// Event reaches the target widget.
      Target,
      /// Target → root. Standard handling. Children handle first.
      Bubble,
  }
  ```
- [ ] Define `PropagationState`:
  ```rust
  pub struct PropagationState {
      handled: bool,
      phase: EventPhase,
      source: WidgetId,    // original target
      current: WidgetId,   // widget currently handling
  }

  impl PropagationState {
      pub fn set_handled(&mut self) { self.handled = true; }
      pub fn is_handled(&self) -> bool { self.handled }
      pub fn phase(&self) -> EventPhase { self.phase }
  }
  ```

---

## 03.2 Propagation Pipeline

**File(s):** `oriterm_ui/src/input/routing.rs`

**Migration note**: The existing `InputState` struct in `routing.rs` must be replaced
by the new propagation pipeline. `InputState` currently handles hover tracking (hot
state), capture (active state), and event routing. These responsibilities move to:
- Hot tracking → `InteractionManager` (Section 01)
- Capture → `InteractionManager.active_widget` (Section 01)
- Event routing → `dispatch_event()` (this section)

The `InputState` struct and its `process_mouse_event()` method should be removed
once the new pipeline is functional. The existing `RouteAction` enum is also removed
(replaced by direct controller dispatch).

**Existing `EventResponse` reconciliation**: The current `EventResponse` enum
(`Handled`, `Ignored`, `RequestPaint`, `RequestLayout`, `RequestFocus`) serves
double-duty as both propagation control and side-effect request. In the new model:
- **Propagation**: controlled by `PropagationState::set_handled()`
- **Side effects**: requested via `ControllerCtx` (`request_paint()`, `request_focus()`,
  `request_anim_frame()`)
- `EventResponse` is removed in Section 08 (with the old Widget trait methods)

Implement the two-phase dispatch.

- [ ] Implement `dispatch_event(root, event, hit_path, interaction_mgr)`:
  <!-- reviewed: completeness fix — hit_path is a WidgetHitTestResult from Section 02,
  not a Vec<WidgetId>. Use result.widget_ids() for update_hot_path, and the full HitEntry
  path for propagation (provides bounds and sense per widget). -->
  ```
  1. Hit test to get path: WidgetHitTestResult [root, ..., parent, target]

  2. CAPTURE phase (root → target):
     for widget in path {
         deliver event with phase = Capture
         if handled: STOP
     }

  3. TARGET phase:
     deliver event to target with phase = Target
     if handled: STOP

  4. BUBBLE phase (target → root):
     for widget in path.reverse() {
         deliver event with phase = Bubble
         if handled: STOP
     }
  ```
- [ ] Lifecycle integration: Before dispatching MouseMove, deliver any pending
  `HotChanged` lifecycle events (Druid pattern: hot state updates precede mouse events)
- [ ] For keyboard events: route to focused widget (no hit testing).
  Capture: ancestors of focused widget. Target: focused widget. Bubble: back up.

---

## 03.3 Active Widget Capture

**File(s):** `oriterm_ui/src/input/routing.rs`

When a widget is active (mouse captured), bypass normal hit testing.

- [ ] On `MouseDown`: normal propagation. Widget can call `ctx.set_active(true)` to capture.
- [ ] While a widget is active:
  - All `MouseMove` events route directly to the active widget (no hit testing)
  - All `MouseUp` events route directly to the active widget
  - `Scroll` events still use normal hit testing (scroll containers should work
    even during a drag in a child widget)
- [ ] On `MouseUp` delivered to active widget: widget should call `ctx.set_active(false)`
  to release capture. If it doesn't, capture persists (widget's responsibility).
- [ ] Hot tracking continues during capture (other widgets' hot state updates normally,
  but they don't receive mouse events). This enables drag-and-drop visual feedback.

---

## 03.4 Completion Checklist

- [ ] `InputEvent` enum covers all mouse and keyboard events
- [ ] Two-phase dispatch: Capture (root → target) then Bubble (target → root)
- [ ] `set_handled()` stops propagation at any phase
- [ ] `HotChanged` lifecycle events fire before `MouseMove` dispatch
- [ ] Active widget captures all mouse events (bypass hit testing)
- [ ] Keyboard events route to focused widget with capture/bubble through ancestors
- [ ] Scroll events always hit-test normally (even during active capture)
- [ ] Unit tests: parent with capture-phase handler can intercept child click
- [ ] Unit tests: active widget receives mouse events when pointer is outside bounds
- [ ] Unit tests: keyboard events route through focus ancestors (capture then bubble)
- [ ] `InputState` struct removed (replaced by new pipeline)
- [ ] `RouteAction` enum removed (replaced by controller dispatch)
- [ ] All callers of `InputState::process_mouse_event()` updated to use new pipeline.
  **Callers are internal to `oriterm_ui` only:**
  - `oriterm_ui/src/input/routing.rs` — definition site
  - `oriterm_ui/src/input/tests.rs` — ~30 test call sites (rewrite tests for new pipeline)
  - Note: `OverlayManager::process_mouse_event()` (in `overlay/manager/event_routing.rs`)
    is a SEPARATE method on a different type. It may also need updating to use the new
    event pipeline, but it is not part of `InputState`.
- [ ] Test file: `oriterm_ui/src/input/tests.rs` (expand existing routing tests)
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** A test demonstrating: parent intercepts MouseDown in capture phase,
prevents child from seeing it. A separate test: child handles MouseDown, parent never
sees it in bubble phase because child calls `set_handled()`.
