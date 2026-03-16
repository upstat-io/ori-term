---
section: "04"
title: "Event Controllers"
status: in-progress
goal: "Composable controller objects replace monolithic event() methods on widgets"
inspired_by:
  - "GTK4 EventController architecture (gtk/gtkeventcontroller.c)"
  - "GTK4 GtkGestureClick, GtkGestureDrag, GtkEventControllerMotion"
depends_on: ["01", "02", "03"]
reviewed: true
sections:
  - id: "04.1"
    title: "Controller Trait"
    status: complete
  - id: "04.2"
    title: "HoverController"
    status: complete
  - id: "04.3"
    title: "ClickController"
    status: complete
  - id: "04.4"
    title: "DragController"
    status: complete
  - id: "04.5"
    title: "ScrollController"
    status: complete
  - id: "04.6"
    title: "FocusController"
    status: complete
  - id: "04.7"
    title: "Completion Checklist"
    status: in-progress
---

# Section 04: Event Controllers

**Status:** Not Started
**Goal:** Widgets compose behavior by attaching controller objects instead of implementing
monolithic `handle_mouse()` / `handle_hover()` / `handle_key()` methods. Each controller
is independently testable and reusable across widget types.

**Context:** Currently, `ButtonWidget` splits its interaction handling across three methods:
`handle_mouse()` (~21 lines), `handle_hover()` (~19 lines), and `handle_key()` (~11 lines),
plus a `hovered: bool` field and `hover_progress: AnimatedValue` that it manages manually.
`SliderWidget` duplicates hover/press tracking with its own `hovered: bool` and
`dragging: bool` fields (no `AnimatedValue`). `DropdownWidget` duplicates it again with
`hovered: bool` and `pressed: bool` for click + keyboard (also no `AnimatedValue`).
When a bug is fixed in one widget's hover logic, it's not automatically fixed in others.
GTK4 solved this by extracting input handling into composable controller objects.

**Reference implementations:**
- **GTK4** `gtk/gtkeventcontroller.c`: Base controller trait with phase declaration
- **GTK4** `gtk/gtkgestureclick.c`: Click recognition (press, release, n-press)
- **GTK4** `gtk/gtkgesturedrag.c`: Drag with threshold, start/update/end lifecycle
- **GTK4** `gtk/gtkeventcontrollermotion.c`: Enter/leave with contains-pointer semantics

**Depends on:** Sections 01-03 (Interaction State, Sense, Event Propagation).

**File size projections:**
- `action.rs`: ~50 lines (`WidgetAction` enum + new variants, relocated from `widgets/mod.rs`).
- `controllers/mod.rs`: ~150 lines (trait, `ControllerCtx`, `ControllerCtxArgs`,
  `DispatchOutput`, `ControllerRequests`, `PropagationState`, dispatch functions,
  `emit_action()`, re-exports, mod declarations).
- `controllers/hover/mod.rs`: ~50 lines. `controllers/click/mod.rs`: ~90 lines.
  `controllers/drag/mod.rs`: ~80 lines. `controllers/scroll/mod.rs`: ~40 lines.
  `controllers/focus/mod.rs`: ~50 lines.
- Each `controllers/*/tests.rs`: exempt from 500-line limit.
- `controllers/tests.rs` (dispatch/composition tests): exempt from 500-line limit.
- All source files well under the 500-line limit.

---

## 04.1 Controller Trait

**File(s):** `oriterm_ui/src/controllers/mod.rs` (new module)

- [x] **Prerequisite: Relocate `WidgetAction` to break module dependency cycle.**
  Move `WidgetAction` from `oriterm_ui/src/widgets/mod.rs` to a new standalone module
  `oriterm_ui/src/action.rs`. Add `pub mod action;` to `oriterm_ui/src/lib.rs`.
  Re-export from `widgets/mod.rs` (`pub use crate::action::WidgetAction;`) for backward
  compatibility so existing callers continue to compile. Update direct imports in
  `oriterm_ui` and `oriterm` crates to use `crate::action::WidgetAction` (or rely on
  the re-export). This ensures `controllers/` imports from `action/`, not from `widgets/`,
  preserving one-way data flow (widgets import controllers, controllers import action,
  widgets import action — no cycle).
- [x] Add `pub mod controllers;` to `oriterm_ui/src/lib.rs`. Without this declaration
  the module is invisible to the rest of the crate. (Same pattern as Section 01's
  `pub mod interaction;` declaration.)
- [x] Define the module structure in `controllers/mod.rs`:
  ```rust
  mod hover;
  mod click;
  mod drag;
  mod scroll;
  mod focus;

  pub use hover::HoverController;
  pub use click::ClickController;
  pub use drag::DragController;
  pub use scroll::ScrollController;
  pub use focus::FocusController;

  #[cfg(test)]
  mod tests;
  ```
  Each controller is a **directory module** (`hover/mod.rs` + `hover/tests.rs`), not a
  flat file, per test-organization.md. Re-export all public types from `controllers/mod.rs`.
- [x] Define `EventController` trait:
  ```rust
  pub trait EventController {
      /// Which propagation phase this controller handles.
      fn phase(&self) -> EventPhase { EventPhase::Bubble }

      /// Handle an input event.
      ///
      /// Two ways to signal "handled" (either is sufficient):
      /// 1. Return `true` (convenience shorthand).
      /// 2. Call `ctx.propagation.set_handled()` (explicit API).
      ///
      /// The dispatch function treats both identically: if either
      /// is set, the event is marked handled for propagation
      /// purposes. Remaining controllers on the SAME widget still
      /// run (GTK4 semantics); the handled flag stops propagation
      /// to the NEXT widget in the capture/bubble chain.
      fn handle_event(
          &mut self,
          event: &InputEvent,
          ctx: &mut ControllerCtx,
      ) -> bool;  // true = consumed, false = pass through

      /// Handle a lifecycle event (hot/active/focus changes).
      fn handle_lifecycle(
          &mut self,
          event: &LifecycleEvent,
          ctx: &mut ControllerCtx,
      ) {
          let _ = (event, ctx);
      }

      /// Reset controller state (e.g., when widget is removed from tree
      /// or disabled). Called on `WidgetRemoved` and `WidgetDisabled(true)`.
      fn reset(&mut self) {}
  }
  ```
- [x] Define `PropagationState` (deferred from Section 03.1 which noted: "PropagationState
  is deferred to Section 04 (ControllerCtx)"). Simple boolean wrapper:
  ```rust
  /// Tracks whether the current event has been handled during dispatch.
  ///
  /// Controllers signal handling via `ctx.propagation.set_handled()` or
  /// by returning `true` from `handle_event()`. The dispatch function
  /// merges both signals: `handled = return_value || propagation.is_handled()`.
  #[derive(Debug, Default)]
  pub struct PropagationState {
      handled: bool,
  }

  impl PropagationState {
      pub fn set_handled(&mut self) { self.handled = true; }
      pub fn is_handled(&self) -> bool { self.handled }
  }
  ```
  `PropagationState` is stored as a field on `ControllerCtx`. The dispatch function
  checks `ctx.propagation.is_handled() || returned_true` after each controller.
- [x] Define `ControllerCtx` (bitmask-based design — simple lifetimes, easy to test):
  ```rust
  pub struct ControllerCtx<'a> {
      pub widget_id: WidgetId,
      pub bounds: Rect,
      pub interaction: &'a InteractionState,
      pub actions: &'a mut Vec<WidgetAction>,
      /// Accumulated side-effect requests (set by controller, read by framework).
      pub requests: ControllerRequests,
      /// Current frame timestamp for multi-click timeout checks.
      pub now: Instant,
      /// Propagation control — call `set_handled()` to stop propagation.
      pub propagation: &'a mut PropagationState,
  }

  /// Manual bitmask (same pattern as `Sense` in `sense/mod.rs`).
  /// The `bitflags` crate is NOT a current dependency. Use a manual
  /// bitmask unless Section 04 also adds `bitflags = "2"` to Cargo.toml.
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
  pub struct ControllerRequests(u8);

  impl ControllerRequests {
      pub const NONE: Self = Self(0);
      pub const PAINT: Self = Self(0b0001);
      pub const ANIM_FRAME: Self = Self(0b0010);
      pub const SET_ACTIVE: Self = Self(0b0100);
      pub const CLEAR_ACTIVE: Self = Self(0b1000);
      pub const REQUEST_FOCUS: Self = Self(0b0001_0000);
      pub const FOCUS_NEXT: Self = Self(0b0010_0000);
      pub const FOCUS_PREV: Self = Self(0b0100_0000);

      pub fn contains(self, other: Self) -> bool { self.0 & other.0 == other.0 }
      pub fn insert(&mut self, other: Self) { self.0 |= other.0; }
      /// Combines two request sets (bitwise OR). Mirrors `Sense::union()`.
      #[must_use]
      pub const fn union(self, other: Self) -> Self { Self(self.0 | other.0) }
  }
  ```
  The framework reads the flags after controller dispatch and applies side effects.
  **Note**: `FOCUS_NEXT` and `FOCUS_PREV` flags are needed for `FocusController`
  (Section 04.6) since `ControllerCtx` does not hold a `&mut FocusManager` reference.
  The framework interprets these flags and calls `FocusManager::focus_next()` /
  `focus_prev()` after controller dispatch.
  **WARNING: This is a cross-cutting refactor.** Moving `WidgetAction` out of
  `widgets/mod.rs` touches every file that imports it — approximately 12-15 files
  across `oriterm_ui` and `oriterm`. Plan for this as a mechanical but high-touch
  change. Run `./clippy-all.sh` and `./build-all.sh` immediately after to catch
  any missed import sites.
- [ ] Widgets expose controllers via `fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>]`
  for event dispatch (requires `&mut self` since `handle_event` takes `&mut self`).
  A read-only accessor `fn controllers(&self) -> &[Box<dyn EventController>]` is also
  provided for introspection. See Section 08 for Widget trait integration.
- [x] Note: `WidgetAction` enum (relocated to `oriterm_ui/src/action.rs` in the
  prerequisite step; re-exported from `widgets/mod.rs`) gains new variants in this section:
  `DoubleClicked`, `TripleClicked` (04.3), `DragStart`, `DragUpdate`,
  `DragEnd` (04.4), `ScrollBy` (04.5). Existing variants (`Clicked`, `Toggled`,
  `ValueChanged`, `TextChanged`, `Selected`, `OpenDropdown`, `DismissOverlay`,
  `MoveOverlay`, `SaveSettings`, `CancelSettings`, `WindowMinimize`, `WindowMaximize`,
  `WindowClose`) are retained. What changes is the emission mechanism: controllers emit
  via `ControllerCtx::emit_action()` instead of returning
  `WidgetResponse { action: Some(..) }`. The `actions` field on `ControllerCtx`
  serves as the collection point.
- [x] Implement `emit_action(&mut self, action: WidgetAction)` convenience method on
  `ControllerCtx` (pushes to the `actions` vec). The event pipeline collects emitted
  actions after controller dispatch and routes them to the application layer.
- [x] Implement `dispatch_to_controllers()` in `controllers/mod.rs` — the function
  that iterates a widget's controllers and delivers events. This is the critical
  orchestration function that wires controllers into the propagation pipeline.
  ```rust
  /// Dispatches an input event to all controllers on a widget.
  ///
  /// Iterates controllers in declaration order. If any controller returns
  /// `true` (consumed) or calls `ctx.set_handled()`, remaining controllers
  /// on the same widget still run (GTK4 behavior: all controllers see the
  /// event), but the event is marked handled for propagation purposes.
  /// Returns the accumulated `ControllerRequests` and collected actions.
  pub fn dispatch_to_controllers(
      controllers: &mut [Box<dyn EventController>],
      event: &InputEvent,
      phase: EventPhase,
      ctx_template: ControllerCtxArgs,  // fields needed to construct ControllerCtx
  ) -> DispatchOutput {
      // ...
  }
  ```
  `ControllerCtxArgs` is a plain struct with the fields needed to construct
  `ControllerCtx` (avoids passing 7 parameters). `DispatchOutput` bundles
  `requests: ControllerRequests`, `actions: Vec<WidgetAction>`, and
  `handled: bool`.

  **Phase filtering**: Only controllers whose `phase()` matches the current
  `DeliveryAction.phase` are invoked. Controllers declaring `EventPhase::Bubble`
  (the default) are skipped during the Capture phase and vice versa. Controllers
  declaring `EventPhase::Target` are only invoked during the Target phase. This
  is the phase gate that makes Capture-phase interception work.

  **Lifecycle dispatch**: A separate `dispatch_lifecycle_to_controllers()` function
  delivers `LifecycleEvent`s to all controllers (no phase filtering — lifecycle
  events are not part of the capture/bubble pipeline).

  **Framework integration point**: The delivery loop (Section 03.2, currently
  deferred) calls `dispatch_to_controllers()` for each `DeliveryAction`. After
  the call, the framework reads `DispatchOutput.requests` and applies side
  effects: `SET_ACTIVE` → `InteractionManager::set_active()`, `CLEAR_ACTIVE` →
  `clear_active()`, `REQUEST_FOCUS` → `request_focus()`, `FOCUS_NEXT` →
  `FocusManager::focus_next()` then `InteractionManager::request_focus()`,
  `PAINT` → mark widget dirty, `ANIM_FRAME` → add to animation scheduler.

- [x] **Controller dispatch ordering**: Controllers on the same widget are dispatched
  in the order returned by `Widget::controllers_mut()`. This is declaration order
  (the order the widget pushes controllers into its `Vec<Box<dyn EventController>>`).
  All controllers see the event even if an earlier one marks it handled (GTK4
  semantics). The `handled` flag only affects propagation to the NEXT widget in the
  capture/bubble chain, not to sibling controllers on the same widget.

- [x] **Bounds during active capture**: When the active widget receives events via
  capture bypass (Section 03.3), `DeliveryAction.bounds` is `Rect::default()`
  because the widget is outside the hit path. `ControllerCtx.bounds` will be
  `Rect::default()` in this case. Controllers that compare positions against bounds
  (e.g., `ClickController` checking press-to-release distance) must use the
  recorded `press_pos` distance, NOT a bounds containment check. Document this
  constraint in `ControllerCtx` rustdoc: "During active capture, `bounds` may be
  `Rect::default()` — do not rely on it for containment checks."

---

## 04.2 HoverController

**File(s):** `oriterm_ui/src/controllers/hover/mod.rs` (+ `hover/tests.rs`)

Tracks enter/leave state and provides callbacks. Replaces all manual hover tracking.

- [x] Define `HoverController`:
  ```rust
  pub struct HoverController {
      on_enter: Option<WidgetAction>,
      on_leave: Option<WidgetAction>,
      /// Fires continuously while pointer is over the widget.
      on_move: bool,
  }
  ```
  **Note:** Emitting stored actions requires `.clone()` since `emit_action()` takes
  ownership. `WidgetAction` derives `Clone`, but some variants contain `Vec<String>`
  (`OpenDropdown`). For hover enter/leave, the stored action should be a cheap variant
  (e.g., `Clicked`). If this becomes a concern, consider `Cow` or taking `&WidgetAction`
  in `emit_action()`. Defer unless profiling shows an issue.
- [x] Responds to `LifecycleEvent::HotChanged` via `handle_lifecycle()`:
  - `is_hot: true` → emit `on_enter` action (cloned), `ctx.requests.insert(PAINT)`
  - `is_hot: false` → emit `on_leave` action (cloned), `ctx.requests.insert(PAINT)`
- [x] Responds to `LifecycleEvent::WidgetDisabled { disabled: true }`: call `reset()`
  to clear any internal state. This prevents a disabled widget from appearing in its
  hovered visual state if it was hovered when disabled.
- [x] `handle_event()` with `InputEvent::MouseMove` when `on_move` is `true`:
  `ctx.requests.insert(PAINT)`. This allows widgets to track continuous pointer
  position (e.g., for tooltip placement). No action emitted on move.
- [x] Does NOT set active or capture events — it's purely observational
- [x] Unit tests: HoverController emits enter/leave actions at correct times,
  WidgetDisabled resets state

---

## 04.3 ClickController

**File(s):** `oriterm_ui/src/controllers/click/mod.rs` (+ `click/tests.rs`)

Click recognition with single/double/triple click detection.

- [x] Define `ClickController`. Requires `use std::time::{Duration, Instant}` and
  `use crate::geometry::Point`. The `Instant` for double-click timeout comparison
  comes from `ctx.now` (the frame timestamp on `ControllerCtx`).
  ```rust
  pub struct ClickController {
      /// Accumulated click count (resets after timeout or movement).
      click_count: u32,
      /// Position of the initial mouse-down.
      press_pos: Option<Point>,
      /// Time of last mouse-down (for double-click detection).
      last_press: Option<Instant>,
      /// Max distance from press to release for a valid click (px).
      click_threshold: f32,
      /// Max time between clicks for multi-click (ms).
      multi_click_timeout: Duration,
  }
  ```
- [x] Implement `ClickController::new()` with sensible defaults:
  `click_threshold: 4.0` (px), `multi_click_timeout: Duration::from_millis(500)`.
  Provide `with_threshold(f32)` and `with_multi_click_timeout(Duration)` builders.
- [x] On `MouseDown`: record position and time (from `ctx.now`),
  `ctx.requests.insert(SET_ACTIVE)`, `ctx.requests.insert(PAINT)`
- [x] On `MouseUp`:
  - If distance from press < threshold: increment `click_count`, emit `Clicked`
  - If within multi-click timeout of last press: emit `DoubleClicked` / `TripleClicked`
  - `ctx.requests.insert(CLEAR_ACTIVE)`, `ctx.requests.insert(PAINT)`
- [x] On `MouseMove` while active: if distance > threshold, cancel the pending click
  by clearing `press_pos`. The widget remains active (captured) so that a co-located
  `DragController` can take over. `ClickController` will not emit `Clicked` on the
  subsequent `MouseUp` because `press_pos` is `None`.
- [x] Add `DoubleClicked(WidgetId)` and `TripleClicked(WidgetId)` variants to `WidgetAction`
  in `oriterm_ui/src/action.rs`. Currently `WidgetAction` only has `Clicked(WidgetId)`.
  **Match arm impact**: All 7 match sites on `WidgetAction` in the `oriterm` binary crate
  use wildcard `_ => {}` arms, so new variants do NOT break compilation. However, call sites
  that should handle double/triple clicks (e.g., `content_actions.rs` for text selection,
  `tab_bar_input.rs` for tab close on double-click) must be updated with explicit arms.
  File list: `chrome/mod.rs`, `keyboard_input/overlay_dispatch.rs`,
  `dialog_context/content_actions.rs`, `dialog_context/event_handling/mouse.rs`,
  `settings_overlay/action_handler/mod.rs`, `mouse_input.rs`, `tab_bar_input.rs`.
- [x] Emit actions: `Clicked(id)`, `DoubleClicked(id)`, `TripleClicked(id)`
- [x] **Click/Drag handoff**: When a widget has both ClickController and DragController,
  they must cooperate. On `MouseDown`, ClickController records the press. On `MouseMove`,
  if the drag threshold is exceeded, ClickController cancels its click and DragController
  transitions to `Dragging`. Implementation: controllers on the same widget are dispatched
  in order. DragController should have a higher-priority phase or check whether
  ClickController already consumed the event. Recommended: both controllers operate
  independently; DragController's `Pending -> Dragging` transition implicitly invalidates
  ClickController's press state because ClickController checks distance on `MouseUp`.
  **WARNING: This is subtle.** Both controllers call `SET_ACTIVE` on `MouseDown`. Since
  `ControllerRequests` is a bitmask union, this works (both set the same flag). But on
  `MouseUp`, both call `CLEAR_ACTIVE`. If DragController is still `Dragging` when
  ClickController sees the `MouseUp`, the drag should take priority. Test this interaction
  carefully with the composition test (scenario: MouseDown -> large move -> MouseUp should
  produce DragEnd, NOT Clicked).
- [x] Unit tests: click, double-click, click-then-drag-cancels

---

## 04.4 DragController

**File(s):** `oriterm_ui/src/controllers/drag/mod.rs` (+ `drag/tests.rs`)

Drag recognition with threshold.

- [x] Define `DragController`. Requires `use crate::geometry::Point`.
  ```rust
  pub struct DragController {
      state: DragState,
      /// Minimum distance before drag begins (prevents accidental drags).
      threshold: f32,
  }

  enum DragState {
      Idle,
      /// Mouse down, waiting for threshold to be exceeded.
      Pending { press_pos: Point },
      /// Drag in progress. `start_pos` is the position where the threshold
      /// was exceeded, `last_pos` is the most recent `MouseMove` position.
      Dragging { start_pos: Point, last_pos: Point },
  }
  ```
- [x] Implement `DragController::new()` with default `threshold: 4.0` (px).
  Provide `with_threshold(f32)` builder.
- [x] On `MouseDown`: transition to `Pending`, record position,
  `ctx.requests.insert(SET_ACTIVE)`
- [x] On `MouseMove` while `Pending`:
  - If distance > threshold: transition to `Dragging`, emit `DragStart { id, pos }`
  - Otherwise: stay pending
- [x] On `MouseMove` while `Dragging`: emit `DragUpdate { id, delta, total_delta }`
  where `delta` is the movement since the last `MouseMove` and `total_delta` is the
  cumulative movement since `DragStart`
- [x] On `MouseUp`:
  - If `Dragging`: emit `DragEnd { id, pos }`
  - If `Pending`: emit nothing (it was a click, not a drag — ClickController handles it)
  - `ctx.requests.insert(CLEAR_ACTIVE)`, transition to `Idle`
- [x] Add `DragStart { id: WidgetId, pos: Point }`, `DragUpdate { id: WidgetId, delta: Point, total_delta: Point }`,
  and `DragEnd { id: WidgetId, pos: Point }` variants to `WidgetAction` in
  `oriterm_ui/src/action.rs`. Requires `use crate::geometry::Point;` in `action.rs`.
  `WidgetAction` derives `PartialEq` and `Debug` — `Point` implements both, so this is safe.
  **Match arm impact**: Same as 04.3 — wildcard arms prevent compilation failure, but
  callers that should handle drag events (e.g., `tab_bar_input.rs` for tab reorder,
  `dialog_context/event_handling/mouse.rs` for dialog drag) need explicit arms.
  Note: `MoveOverlay { delta_x, delta_y }` already exists for drag-to-reposition but is
  overlay-specific. The new variants are generic drag actions.
- [x] Emit actions: `DragStart { id, pos }`, `DragUpdate { id, delta, total_delta }`, `DragEnd { id, pos }`
- [x] Responds to `LifecycleEvent::WidgetDisabled { disabled: true }`: call `reset()` to
  transition to `Idle`. Without this, a drag in progress when the widget is disabled will
  leave the widget in the `Dragging` state with active capture. The framework clears
  active capture separately (see `reset()` item below).
- [x] `reset()` implementation: set `state = DragState::Idle`. The framework is
  responsible for calling `InteractionManager::clear_active()` when delivering
  `WidgetDisabled` to the active widget — `reset()` does not need to request this
  because the framework already knows the widget is active and can clear it directly.
- [x] Unit tests: drag threshold, drag delta accumulation, release without threshold = no drag,
  disable mid-drag resets state

---

## 04.5 ScrollController

**File(s):** `oriterm_ui/src/controllers/scroll/mod.rs` (+ `scroll/tests.rs`)

Scroll event handling (wheel and trackpad).

- [x] Define `ScrollController`:
  ```rust
  pub struct ScrollController {
      line_height: f32,
  }
  ```
- [x] Add `ScrollBy { id: WidgetId, delta_x: f32, delta_y: f32 }` variant to `WidgetAction`
  in `oriterm_ui/src/action.rs`. **Match arm impact**: Same as 04.3 — wildcard arms
  prevent compilation failure, but `scroll/mod.rs` consumers should add explicit handling.
- [x] On `InputEvent::Scroll { delta, .. }`: match on `ScrollDelta`:
  - `ScrollDelta::Lines { x, y }` → convert to pixels: `(x * line_height, y * line_height)`.
  - `ScrollDelta::Pixels { x, y }` → pass through as-is.
  Emit `ScrollBy { id, delta_x, delta_y }` with the converted values.
- [x] Does NOT set active (scroll is instantaneous, no capture needed)
- [x] Phase: `Bubble` (children get scroll first; if unhandled, parent scrolls)
- [x] Unit tests: line-to-pixel conversion, pixel passthrough

---

## 04.6 FocusController

**File(s):** `oriterm_ui/src/controllers/focus/mod.rs` (+ `focus/tests.rs`)

Keyboard focus management with tab navigation.

- [x] Define `FocusController`:
  ```rust
  pub struct FocusController {
      /// Tab index for focus ordering (lower = earlier).
      tab_index: Option<i32>,
  }
  ```
- [x] On `KeyDown(Tab)`: set `ControllerRequests::FOCUS_NEXT` (or `FOCUS_PREV` with
  Shift). This is a framework-level operation, not a `WidgetAction`. The framework
  reads the flag after controller dispatch and calls `FocusManager::focus_next()` /
  `focus_prev()` followed by `InteractionManager::request_focus()` to keep both
  managers in sync. (`FocusManager` is in `oriterm_ui/src/focus/mod.rs`; both
  `focus_next` and `focus_prev` are `pub fn(&mut self)` methods.)
- [x] On `LifecycleEvent::FocusChanged`: `ctx.requests.insert(PAINT)` so the widget
  redraws with its focused/unfocused visual state. No action emitted -- the visual
  state change is handled by `VisualStateAnimator` (Section 06) reading
  `InteractionState::is_focused()`.
- [x] On `MouseDown` (if widget is focusable): set `ControllerRequests::REQUEST_FOCUS`
- [x] On `KeyUp(Tab)`: return `true` (consumed) to prevent the Tab key-up from
  bubbling to parent widgets. Without this, the key-down is consumed but the key-up
  leaks, which can cause parent containers to see an unmatched key-up.
- [x] **`tab_index` field**: The `tab_index: Option<i32>` field is not used by
  `FocusController` directly — it is metadata that the framework reads when building
  the focus order in `FocusManager::set_focus_order()`. If `tab_index` is `None`, the
  widget uses natural tree order. If `Some(n)`, it sorts by `n` (lower = earlier).
  Currently `FocusManager::set_focus_order()` accepts a flat `Vec<WidgetId>` — the
  sorting by `tab_index` must happen at the call site (Section 08 wiring), not inside
  `FocusController`.
- [x] Unit tests: tab navigation order, shift-tab reverse, focus on click,
  KeyUp(Tab) consumed

---

## 04.7 Completion Checklist

### Module structure
- [x] `pub mod action;` declared in `oriterm_ui/src/lib.rs` (`WidgetAction` relocation)
- [x] `pub mod controllers;` declared in `oriterm_ui/src/lib.rs`
- [x] `controllers/mod.rs` declares `mod hover;`, `mod click;`, `mod drag;`,
  `mod scroll;`, `mod focus;` with public re-exports
- [x] Each controller is a directory module (`hover/mod.rs` + `hover/tests.rs`, etc.)
  per test-organization.md

### Types and traits
- [x] `EventController` trait with `handle_event()`, `handle_lifecycle()`, `phase()`, `reset()`
- [x] `PropagationState` struct with `set_handled()` / `is_handled()` (deferred from Section 03)
- [x] `ControllerCtx` provides: `widget_id`, `bounds`, `interaction`, `actions`, `requests`,
  `now: Instant`, `propagation: &mut PropagationState`
- [x] `ControllerRequests` manual bitmask with flags: `PAINT`, `ANIM_FRAME`, `SET_ACTIVE`,
  `CLEAR_ACTIVE`, `REQUEST_FOCUS`, `FOCUS_NEXT`, `FOCUS_PREV` plus `contains()`,
  `insert()`, `union()` (same pattern as `Sense`; no `bitflags` crate dependency
  unless explicitly added)
- [x] `ControllerCtx::emit_action()` pushes to the `actions` vec
- [x] `dispatch_to_controllers()` function dispatches events with phase filtering
- [x] `dispatch_lifecycle_to_controllers()` function dispatches lifecycle events (no phase
  filtering)

### WidgetAction variants
- [x] `WidgetAction` in `oriterm_ui/src/action.rs` extended with:
  `DoubleClicked(WidgetId)`, `TripleClicked(WidgetId)`,
  `DragStart`, `DragUpdate`, `DragEnd`, `ScrollBy`
- [x] `use crate::geometry::Point;` added to `action.rs` imports (needed by
  `DragStart`, `DragUpdate`, `DragEnd` variants)
- [ ] Callers in `oriterm` binary that should handle new variants updated with explicit
  arms (wildcard arms prevent compilation failure but mean events are silently dropped)

### LifecycleEvent prerequisite
- [x] `LifecycleEvent` variant fields are already accessible — `pub enum` variant fields
  in Rust have no separate visibility; they are always accessible via pattern matching.
  No changes to `interaction/lifecycle.rs` needed.

### Controllers
- [x] `HoverController` replaces all manual hover tracking; handles `WidgetDisabled` reset
- [x] `ClickController` handles single/double/triple click with threshold;
  `new()` with defaults (4px threshold, 500ms multi-click timeout); uses `ctx.now`
  for timestamp comparisons
- [x] `DragController` handles drag with threshold and start/update/end lifecycle;
  handles `WidgetDisabled` reset (clears active capture mid-drag)
- [x] `ScrollController` handles wheel events; converts both `ScrollDelta::Lines` and
  `ScrollDelta::Pixels`
- [x] `FocusController` handles tab navigation, focus-on-click, and `KeyUp(Tab)` consumption
- [x] All controllers are independently unit-testable without a real window
- [x] Controllers compose: a widget can have HoverController + ClickController + FocusController
- [x] `ControllerCtx` rustdoc documents that `bounds` may be `Rect::default()` during
  active capture — controllers must not rely on bounds for containment checks

### Sync points (types that span crate boundaries)
- [x] `WidgetAction` relocated to `oriterm_ui/src/action.rs`; re-exported from
  `widgets/mod.rs` for backward compatibility; new variants added, `Point` import added
- [x] `LifecycleEvent` enum (`interaction/lifecycle.rs`): no changes needed (enum variant
  fields are always accessible in Rust)
- [x] `ControllerRequests` type: used by `ControllerCtx` (Section 04), `LifecycleCtx`
  (Section 08), and `AnimCtx` (Section 08) — all three contexts use the same bitflag
  type for side-effect requests
- [x] `dispatch_to_controllers()` output feeds into `InteractionManager` methods
  (`set_active`, `clear_active`, `request_focus`) and `FocusManager` methods
  (`focus_next`, `focus_prev`) — the caller must hold mutable references to both managers

### Tests
- [x] Each controller is a directory module with its own `tests.rs`:
  - `controllers/hover/mod.rs` + `controllers/hover/tests.rs`
  - `controllers/click/mod.rs` + `controllers/click/tests.rs`
  - `controllers/drag/mod.rs` + `controllers/drag/tests.rs`
  - `controllers/scroll/mod.rs` + `controllers/scroll/tests.rs`
  - `controllers/focus/mod.rs` + `controllers/focus/tests.rs`
  - `controllers/mod.rs` ends with `#[cfg(test)] mod tests;` for dispatch function tests.
  - `controllers/tests.rs` contains dispatch/composition/phase-filtering tests.
  Per test-organization.md Rule 2: one `tests.rs` per source file that has tests.

### Required test scenarios
- [x] HoverController emits enter/leave actions on HotChanged lifecycle events
- [x] HoverController reset on WidgetDisabled
- [x] ClickController single click: MouseDown → MouseUp within threshold
- [x] ClickController double click: two presses within multi_click_timeout
- [x] ClickController triple click: three presses within timeout
- [x] ClickController click cancelled by drag (distance > threshold on MouseUp)
- [x] ClickController timeout resets click count
- [x] DragController threshold: MouseDown → small move → no DragStart
- [x] DragController threshold exceeded: MouseDown → large move → DragStart emitted
- [x] DragController delta accumulation: DragUpdate.total_delta correct across moves
- [x] DragController reset on WidgetDisabled mid-drag
- [x] ScrollController Lines-to-pixels conversion
- [x] ScrollController Pixels passthrough
- [x] FocusController Tab → FOCUS_NEXT request flag
- [x] FocusController Shift+Tab → FOCUS_PREV request flag
- [x] FocusController KeyUp(Tab) consumed (returns true)
- [x] FocusController MouseDown → REQUEST_FOCUS flag
- [x] Composition test: widget with Hover + Click + Focus, full event sequence
- [x] Click/Drag composition: MouseDown -> large move -> MouseUp produces DragEnd, NOT Clicked
- [x] Phase filtering: Capture-phase controller invoked only during Capture phase
- [x] Controller dispatch ordering: declaration order preserved

### Build gate
- [x] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** A test widget with three controllers (Hover + Click + Focus) correctly
receives hover enter/leave, click, and focus events. Each controller tested independently
and in combination. `dispatch_to_controllers()` correctly filters by phase and accumulates
requests/actions from multiple controllers on the same widget.
