---
section: "03"
title: "Event Propagation"
status: in-progress
goal: "Capture + Bubble two-phase event propagation replaces the current single-pass dispatch"
inspired_by:
  - "WPF Preview/Bubble paired propagation (PresentationCore)"
  - "GTK4 Capture/Target/Bubble three-phase (gtk_event_controller.c)"
depends_on: ["01", "02"]
reviewed: true
sections:
  - id: "03.1"
    title: "Event Types & Routing"
    status: complete
  - id: "03.2"
    title: "Propagation Pipeline"
    status: in-progress
  - id: "03.3"
    title: "Active Widget Capture"
    status: complete
  - id: "03.3a"
    title: "Coexistence During Transition"
    status: complete
  - id: "03.4"
    title: "Completion Checklist"
    status: in-progress
---

# Section 03: Event Propagation

**Status:** In Progress
**Goal:** Input events flow through a two-phase pipeline: Capture (root -> target) then Bubble
(target -> root). Parents can intercept events before children see them. Active widgets capture
all mouse events until released. Any widget returning a non-`Ignored` `EventResponse` stops
propagation (the `set_handled()` API is introduced in Section 04 with `ControllerCtx`).

**Context:** The old single-pass `InputState` routing has been replaced. `InputState` and
`RouteAction` have been removed from `routing.rs` (file deleted). The new system uses
`InteractionManager` (hot/active/focus), `layout_hit_test_path()` (hit testing), and
`plan_propagation()` (two-phase routing). Containers and overlays retain their own internal
dispatch during the transition period (migrated in Section 08).

**Reference implementations:**
- **WPF** `PresentationCore`: Every input event is a Preview+Bubble pair. PreviewMouseDown
  tunnels from root to target, then MouseDown bubbles back up.
- **GTK4** `gtk_event_controller.c`: Controllers declare their phase (CAPTURE, TARGET, BUBBLE).
  Multiple controllers per widget. Consumed events stop propagation.

**Depends on:** Section 01 (InteractionState), Section 02 (Hit Testing).

**WARNING -- OverlayManager integration**: `oriterm_ui/src/overlay/manager/event_routing.rs`
(343 lines) is a SEPARATE event routing implementation used for popup overlays (dropdowns,
menus, dialogs). It has its own `process_mouse_event()`, `process_hover_event()`, and
`process_key_event()` methods that dispatch events to widgets inside overlays. This
overlay routing is NOT updated in Section 03 (see 03.3a for the coexistence strategy).
It must be migrated to the new pipeline in Section 08 when overlay widgets adopt the
new Widget trait. The overlay manager callers for reference:
- `oriterm/src/app/mouse_input.rs` -- `process_mouse_event`
- `oriterm/src/app/dialog_context/event_handling/mouse.rs` -- `process_mouse_event`
- `oriterm/src/app/dialog_context/event_handling/mod.rs` -- `process_mouse_event`
  (hover routing via `MouseEventKind::Move`, NOT `process_hover_event`)
- `oriterm/src/app/keyboard_input/mod.rs` -- `process_key_event`

Note: `process_hover_event()` exists on `OverlayManager` but is currently only called
from overlay tests, not from the `oriterm` binary.

---

## 03.1 Event Types & Routing

**File(s):** `oriterm_ui/src/input/event.rs`

Unify all input events into a single routable event type.

- [x] Define `InputEvent` enum in `event.rs` with variants: `MouseDown`, `MouseUp`,
  `MouseMove`, `Scroll`, `KeyDown`, `KeyUp`. Each variant carries `pos` (mouse) or
  `key` (keyboard), `modifiers`, and event-specific data (`button`, `delta`).
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
- [x] Define `EventPhase` enum in `event.rs` with variants `Capture`, `Target`, `Bubble`:
  ```rust
  pub enum EventPhase {
      /// Root -> target. Parents see the event first. Can intercept.
      Capture,
      /// Event reaches the target widget.
      Target,
      /// Target -> root. Standard handling. Children handle first.
      Bubble,
  }
  ```
- [x] `PropagationState` is NOT defined in this section. It is deferred to Section 04
  (ControllerCtx). In Section 03, the caller tracks handled-ness by inspecting
  `WidgetResponse` from each delivery. `EventPhase` is carried on `DeliveryAction`.
  When Section 04 introduces `ControllerCtx`, `PropagationState` becomes the context
  object passed to controllers so they can call `ctx.set_handled()` and `ctx.phase()`.

---

## 03.2 Propagation Pipeline

**File(s):** `oriterm_ui/src/input/dispatch/mod.rs` (new), `oriterm_ui/src/input/routing.rs` (removed)

**Migration note**: The `InputState` struct in `routing.rs` has been replaced and removed.
`InputState` handled three responsibilities now in separate owners:
- Hot tracking -> `InteractionManager` (Section 01)
- Capture -> `InteractionManager.active_widget` (Section 01)
- Event routing -> `plan_propagation()` (this section)

The `InputState` struct, `RouteAction` enum, and `routing.rs` file are removed.

**`EventResponse` during transition**: The current `EventResponse` enum (`Handled`,
`Ignored`, `RequestPaint`, `RequestLayout`, `RequestFocus`) serves double-duty as both
propagation control and side-effect request. During the transition period (Sections 03
through 07), `EventResponse` continues to exist. The delivery loop interprets it for
both purposes: `Ignored` means "continue propagation", anything else means "stop
propagation" and the specific variant indicates a side-effect. In the final model
(Sections 04 and 08):
- **Propagation**: controlled by `PropagationState::set_handled()` (Section 04)
- **Side effects**: requested via `ControllerCtx` methods (Section 04)
- `EventResponse` is removed in Section 08 (with the old Widget trait methods)

### Tasks

- [x] Create directory module `oriterm_ui/src/input/dispatch/mod.rs`. Declare
  `pub mod dispatch;` in `input/mod.rs`. Re-export `plan_propagation` and
  `DeliveryAction` from `input/mod.rs`. Keep `routing.rs` intact during transition
  (old and new coexist until Section 08 removes the old). Tests go in
  `oriterm_ui/src/input/dispatch/tests.rs` (sibling `tests.rs` pattern per
  test-organization.md).
- [x] Define `DeliveryAction` struct in `dispatch/mod.rs`:
  ```rust
  /// A single delivery action in the propagation sequence.
  pub struct DeliveryAction {
      /// Target widget to receive the event.
      pub widget_id: WidgetId,
      /// Propagation phase for this delivery.
      pub phase: EventPhase,
      /// Bounds to use when constructing EventCtx for this widget.
      pub bounds: Rect,
  }
  ```
- [x] Implement `plan_propagation()` in `dispatch/mod.rs` as a **pure routing
  function** that does NOT touch the widget tree and does NOT allocate. It writes
  into a caller-owned `&mut Vec<DeliveryAction>` buffer (cleared and filled; caller
  retains capacity across frames). Signature:
  ```rust
  pub fn plan_propagation(
      event: &InputEvent,
      hit_path: &WidgetHitTestResult,
      active_widget: Option<WidgetId>,
      out: &mut Vec<DeliveryAction>,
  )
  ```
  `hit_path` is a `WidgetHitTestResult` from Section 02 (not a `Vec<WidgetId>`).
  Each `HitEntry` provides `widget_id`, `bounds`, and `sense` per widget.

  Algorithm:
  ```
  1. Receive hit path: [root, ..., parent, target]
  2. CAPTURE phase (root -> target):
     for each widget in path: emit DeliveryAction { phase: Capture }
  3. TARGET phase:
     emit DeliveryAction { phase: Target } for the target widget
  4. BUBBLE phase (target -> root):
     for each widget in path.reverse(): emit DeliveryAction { phase: Bubble }
  ```
  The caller iterates actions, delivers each to the widget, and stops on first
  handled response.

  **Why pure routing**: A naive `dispatch_event()` cannot directly call
  `handle_mouse()` on nested widgets because the caller only has `&mut root_widget`.
  Rust's ownership model prevents extracting multiple `&mut` refs to nested children.
  Instead, the caller (app layer, which owns the widget tree and can borrow children
  one at a time) iterates the actions and delivers each one. This matches the current
  `InputState::process_mouse_event()` pattern, which returns
  `RouteAction::Deliver { target, event }` for the caller to dispatch.

  **Note:** Actual signature includes a `focus_path: &[WidgetId]` parameter for
  keyboard event routing, beyond what the plan originally specified.

- [ ] Implement the caller-side delivery loop (transition bridge). For each
  `DeliveryAction`:
  1. Construct `EventCtx` using `action.bounds`, `interaction: Some(&interaction_mgr)`,
     `widget_id: Some(action.widget_id)`.
  2. Call the widget's `handle_mouse()` / `handle_key()` (old trait methods).
  3. Map `WidgetResponse` to propagation control:
     `EventResponse::Ignored` -> continue to next action.
     Any other `EventResponse` -> stop (event handled).
  4. Map `CaptureRequest::Acquire` -> `InteractionManager::set_active(widget_id)`.
  5. Map `CaptureRequest::Release` (or `None` + `MouseUp`) ->
     `InteractionManager::clear_active()`.
  6. Collect side effects (`RequestPaint`, `RequestLayout`) into a `DispatchResult`.

  This bridge is temporary -- Section 04 (controllers) and Section 08 (trait
  migration) replace the `handle_mouse()` calls with controller dispatch.

  **DEFERRED**: This is app-layer code (`oriterm` binary crate) that would be dead code
  until the app layer is wired to use the new pipeline (Section 08). The delivery loop
  logic is demonstrated in integration tests via `simulate_mouse()` helper.

- [ ] Define `DispatchResult` at the **app/caller layer** (e.g.,
  `oriterm/src/app/event_dispatch.rs`), NOT in `input/dispatch/`. It references
  `WidgetAction` from `widgets/mod.rs`, and `input/` must not import from `widgets/`
  (one-way data flow: `widgets/` -> `input/`, not both directions).
  ```rust
  pub struct DispatchResult {
      /// Whether any widget handled the event.
      pub handled: bool,
      /// Highest-priority side effect requested during propagation
      /// (transitional -- removed with EventResponse in Section 08).
      pub effect: EventResponse,
      /// Semantic action emitted by a widget (at most one).
      pub action: Option<WidgetAction>,
      /// Source widget ID for invalidation tracking.
      pub source: Option<WidgetId>,
  }
  ```
  This replaces the `SmallVec<[RouteAction; 4]>` returned by the old
  `InputState::process_mouse_event()`.

  **DEFERRED**: Would be dead code in binary crate until Section 08 wiring.

- [x] Before dispatching `MouseMove`, deliver any pending `HotChanged` lifecycle
  events via `InteractionManager::drain_events()` (Druid pattern: hot state updates
  precede mouse events). Demonstrated in `simulate_mouse()` test helper.
- [x] Add `pub(crate) fn focus_ancestor_path(&self) -> Vec<WidgetId>` to
  `InteractionManager`. This walks `parent_map` from the focused widget to root,
  reverses to root-to-leaf order, and returns the path. `plan_propagation()` accepts
  this path for keyboard event routing (Capture: root -> focused, Target: focused,
  Bubble: focused -> root). `InteractionManager::ancestors()` is currently private;
  this new method encapsulates the `parent_map` walk with a clean public interface.
  **Note:** Made `pub` (not `pub(crate)`) since the `oriterm` binary needs access.
- [x] Remove `InputState::keyboard_target()` along with `InputState`. It has zero
  callers (defined but unused). The focused widget is obtained directly from
  `InteractionManager::focused_widget()` (already exists from Section 01).
  **Done:** Entire `routing.rs` removed.

---

## 03.3 Active Widget Capture

**File(s):** `oriterm_ui/src/input/dispatch/mod.rs` (capture bypass logic in
`plan_propagation()`)

When a widget is active (mouse captured), `plan_propagation()` bypasses normal hit
testing and routes mouse events directly to the active widget.

Note: `ctx.set_active()` does not yet exist on `EventCtx` (deferred from Section 01.4).
During the transition period, capture is managed by the delivery loop mapping
`CaptureRequest::Acquire` / `Release` from `WidgetResponse` to
`InteractionManager::set_active()` / `clear_active()` (see 03.2). The `ctx.set_active()`
API is added in Section 04 via `ControllerCtx`.

- [x] `MouseDown` with no active widget: normal two-phase propagation. The delivery
  loop maps `CaptureRequest::Acquire` from the widget's response to
  `InteractionManager::set_active(widget_id)`.
- [x] While a widget is active, `plan_propagation()` emits a single `DeliveryAction`
  targeting the active widget (phase: `Target`) for `MouseMove` and `MouseUp` events.
  No hit testing, no capture/bubble phases. `Scroll` events still use normal
  hit testing (scroll containers must work even during a drag in a child widget).
- [x] On `MouseUp` delivered to the active widget: the delivery loop maps
  `CaptureRequest::Release` (or `CaptureRequest::None` on `MouseUp`) to
  `InteractionManager::clear_active()`. If the widget returns
  `CaptureRequest::Acquire` on `MouseUp`, capture persists (widget's responsibility).
- [x] Hot tracking continues during capture: `InteractionManager::update_hot_path()`
  is still called with the real hit path, so other widgets' hot state updates normally
  (but they do not receive mouse events). This enables drag-and-drop visual feedback
  on drop targets. **Behavioral change from existing system**: the old `InputState`
  SUPPRESSED hover transitions during capture. The new system intentionally reverses
  this. Three tests rewritten:
  - `routing_hover_changes_during_capture` — asserts hover DOES change during capture.
  - `routing_captured_move_outside_all_bounds` — asserts hot cleared when pointer leaves.
  - `routing_capture_release_hover_already_current` — hover already correct on release.
- [x] Cursor-left handling: When the cursor leaves the window entirely (`CursorLeft`
  event from winit), call `InteractionManager::update_hot_path(&[])` to clear all hot
  state and emit `HotChanged(false)` events. This replaces the old
  `InputState::process_cursor_left()` method. The app layer must wire this through
  the existing callers: `oriterm` handles `CursorLeft` in the event loop, and dialog
  windows handle it via `clear_dialog_hover` in `content_actions.rs:304`.
  Tested in `routing_cursor_left_clears_hot`.

---

## 03.3a Coexistence During Transition

**This section addresses how the new propagation pipeline coexists with the old
event system during the transition period (Sections 03 through 08).**

### ContainerWidget internal dispatch

`ContainerWidget` has its own internal event routing in `event_dispatch.rs`:
`dispatch_mouse()`, `dispatch_to_captured()`, `deliver_mouse_to_child()`,
`update_hover()`, `dispatch_key()`, and `handle_hover()`. It also owns a
`ContainerInputState` with `hovered_child` and `captured_child` tracking.

During Section 03, these remain functional — the new two-phase pipeline delivers
events to the topmost widget in the hit path, and `ContainerWidget::handle_mouse()`
internally forwards to children using the old single-pass dispatch. This is the
correct layering: Section 03 handles inter-widget routing (across the tree), while
`ContainerWidget` handles intra-container routing (among its direct children).

`ContainerInputState` and the container's internal dispatch are removed in Section 08
when `handle_mouse()` / `handle_hover()` / `handle_key()` are removed from the
Widget trait. At that point, the framework's two-phase propagation handles ALL
dispatch, including into container children.

**No changes to `ContainerWidget` in Section 03.** The coexistence is natural.

### OverlayManager event routing

`OverlayManager` (in `overlay/manager/event_routing.rs`) has its own independent
event routing: `process_mouse_event()`, `process_hover_event()`, `process_key_event()`.
These dispatch events to overlay widgets (popups, modals, dialogs) and are called
from the `oriterm` binary app layer BEFORE the main widget tree receives events.

During Section 03, `OverlayManager`'s event routing is NOT changed. The overlay
system sits above the main widget tree — events hit overlays first, and only pass
through to the main tree if no overlay consumes them. The new two-phase pipeline
applies to the main widget tree only.

The overlay system should adopt the new pipeline in Section 08 when overlay widgets
are migrated to the new Widget trait. At that point, overlays can participate in
the same capture/bubble flow. For now, the two systems coexist at the app layer:
overlay check first, then `plan_propagation()` + delivery loop if overlays don't consume.

### CaptureRequest bridge

The old Widget trait methods return `WidgetResponse` with a `CaptureRequest` field.
The new pipeline uses `InteractionManager::set_active()` / `clear_active()`. During
the transition:

- When the delivery loop calls a widget's `handle_mouse()` and receives
  `CaptureRequest::Acquire`, it calls `InteractionManager::set_active(widget_id)`.
- When it receives `CaptureRequest::Release` (or `CaptureRequest::None` with
  `MouseUp`), it calls `InteractionManager::clear_active()`.
- The `WidgetResponse::capture` field is consumed by the propagation pipeline
  and does NOT propagate to the caller. The caller observes capture state via
  `InteractionManager::active_widget()`.

---

## 03.4 Completion Checklist

### New types and functions

- [x] `InputEvent` enum in `event.rs` with 6 variants (MouseDown, MouseUp, MouseMove,
  Scroll, KeyDown, KeyUp)
- [x] `EventPhase` enum in `event.rs` with 3 variants (Capture, Target, Bubble)
- [x] `DeliveryAction` struct in `input/dispatch/mod.rs` with fields: `widget_id`,
  `phase`, `bounds`
- [x] `plan_propagation()` in `input/dispatch/mod.rs` -- pure function writing into
  `&mut Vec<DeliveryAction>` (no allocation, no widget tree access)
- [ ] `DispatchResult` struct at the app/caller layer (NOT in `input/dispatch/` --
  references `WidgetAction` from `widgets/`, so must avoid bidirectional dependency)
  **DEFERRED: dead code in binary crate until Section 08 wiring.**
- [x] `InteractionManager::focus_ancestor_path()` (`pub`) -- returns root-to-leaf
  `Vec<WidgetId>` path for the focused widget via `parent_map`

### Propagation behavior

- [x] `plan_propagation()` emits Capture actions (root -> target), then Target action,
  then Bubble actions (target -> root) for the full hit path
- [x] Delivery loop stops propagation when any widget returns non-`Ignored`
  `EventResponse` (the `set_handled()` API is deferred to Section 04).
  Logic demonstrated in tests; formal delivery loop deferred to Section 08.
- [x] `HotChanged` lifecycle events are drained and delivered before `MouseMove`
  dispatch. Demonstrated in `simulate_mouse()` test helper.
- [x] Keyboard events route through `focus_ancestor_path()` with capture/bubble
  through the focused widget's ancestors (no hit testing)
- [x] `Scroll` events always use normal hit testing, even during active capture

### Capture behavior

- [x] While a widget is active: `MouseMove` and `MouseUp` route directly to the
  active widget (single `Target`-phase `DeliveryAction`, no hit testing)
- [x] Delivery loop maps `CaptureRequest::Acquire` to
  `InteractionManager::set_active()`; `Release`/`None+MouseUp` to `clear_active()`.
  Logic demonstrated in tests; formal delivery loop deferred to Section 08.
- [x] Hot tracking continues during capture (behavioral change from the old system
  which suppressed hover transitions)
- [x] Cursor-left handling: `InteractionManager::update_hot_path(&[])` clears all
  hot state (replaces `InputState::process_cursor_left()`)

### Coexistence (unchanged in this section)

- [x] `ContainerWidget` internal dispatch (`event_dispatch.rs`,
  `ContainerInputState`) left unchanged (removed in Section 08)
- [x] `OverlayManager` event routing left unchanged (adapted in Section 08)

### Removals

- [x] `InputState` struct removed from `routing.rs`
- [x] `RouteAction` enum removed from `routing.rs`
- [x] `InputState::keyboard_target()` removed (zero callers -- defined but unused)
- [x] All `process_mouse_event` and `process_cursor_left` call sites
  in `oriterm_ui/src/input/tests.rs` rewritten for the new pipeline.
  Note: `OverlayManager::process_mouse_event()` (in `overlay/manager/event_routing.rs`)
  is a SEPARATE method on a different type and is NOT updated in this section.

### Test files

- [x] `oriterm_ui/src/input/dispatch/tests.rs` -- pure function tests for
  `plan_propagation()` with constructed hit paths (14 tests)
- [x] `oriterm_ui/src/input/tests.rs` -- existing routing tests rewritten for the new
  pipeline; 3 hover-during-capture tests rewritten for the new behavior (hover
  continues during capture)

### Required test scenarios

- [x] Parent intercepts `MouseDown` in capture phase, child never receives the event
  (`parent_capture_precedes_child` in dispatch/tests.rs)
- [x] Child handles `MouseDown` in target phase, parent never sees it in bubble phase
  (child returns non-`Ignored` response)
  (`child_handles_in_target_prevents_parent_bubble` in dispatch/tests.rs)
- [x] Active widget receives `MouseMove` when pointer is outside its bounds
  (`active_widget_receives_mouse_move_directly` + `routing_captured_widget_receives_events_outside_bounds`)
- [x] `plan_propagation()` produces `DeliveryAction` entries with correct `bounds`
  from `HitEntry.bounds` for each widget in the path
  (`mouse_bounds_from_hit_entries` in dispatch/tests.rs)
- [x] Cursor leaves window: all hot state cleared, `HotChanged(false)` emitted for
  all previously-hot widgets (`routing_cursor_left_clears_hot`)
- [x] Keyboard event routes through focused widget's ancestors in capture/bubble
  order, with focus ancestor path derived from `parent_map`
  (`keyboard_routes_through_focus_ancestors` in dispatch/tests.rs)

### Build gate

- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
