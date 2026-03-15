---
section: "04"
title: "Event Controllers"
status: not-started
goal: "Composable controller objects replace monolithic event() methods on widgets"
inspired_by:
  - "GTK4 EventController architecture (gtk/gtkeventcontroller.c)"
  - "GTK4 GtkGestureClick, GtkGestureDrag, GtkEventControllerMotion"
depends_on: ["01", "02", "03"]
reviewed: false
sections:
  - id: "04.1"
    title: "Controller Trait"
    status: not-started
  - id: "04.2"
    title: "HoverController"
    status: not-started
  - id: "04.3"
    title: "ClickController"
    status: not-started
  - id: "04.4"
    title: "DragController"
    status: not-started
  - id: "04.5"
    title: "ScrollController"
    status: not-started
  - id: "04.6"
    title: "FocusController"
    status: not-started
  - id: "04.7"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Event Controllers

**Status:** Not Started
**Goal:** Widgets compose behavior by attaching controller objects instead of implementing
monolithic `handle_mouse()` / `handle_hover()` / `handle_key()` methods. Each controller
is independently testable and reusable across widget types.

**Context:** Currently, `ButtonWidget` splits its interaction handling across three methods:
`handle_mouse()` (~20 lines), `handle_hover()` (~18 lines), and `handle_key()` (~10 lines),
plus a `hovered: bool` field and `hover_progress: AnimatedValue` that it manages manually.
`SliderWidget` duplicates much of the same hover/press logic for drag. `DropdownWidget`
duplicates it again for click + keyboard. When a bug is fixed in one widget's hover logic,
it's not automatically fixed in others. GTK4 solved this by extracting input handling into
composable controller objects.

**Reference implementations:**
- **GTK4** `gtk/gtkeventcontroller.c`: Base controller trait with phase declaration
- **GTK4** `gtk/gtkgestureclick.c`: Click recognition (press, release, n-press)
- **GTK4** `gtk/gtkgesturedrag.c`: Drag with threshold, start/update/end lifecycle
- **GTK4** `gtk/gtkeventcontrollermotion.c`: Enter/leave with contains-pointer semantics

**Depends on:** Sections 01-03 (Interaction State, Sense, Event Propagation).

---

## 04.1 Controller Trait

**File(s):** `oriterm_ui/src/controllers/mod.rs` (new module)

- [ ] Define `EventController` trait:
  ```rust
  pub trait EventController {
      /// Which propagation phase this controller handles.
      fn phase(&self) -> EventPhase { EventPhase::Bubble }

      /// Handle an input event. Returns whether the event was consumed.
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

      /// Reset controller state (e.g., when widget is removed from tree).
      fn reset(&mut self) {}
  }
  ```
- [ ] Define `ControllerCtx`. Two design options:
  **(a) Closure-based** (original plan — maximum flexibility, complex lifetimes):
  ```rust
  pub struct ControllerCtx<'a> {
      pub widget_id: WidgetId,
      pub bounds: Rect,
      pub interaction: &'a InteractionState,
      pub set_active: &'a mut dyn FnMut(bool),
      pub request_focus: &'a mut dyn FnMut(),
      pub request_paint: &'a mut dyn FnMut(),
      pub request_anim_frame: &'a mut dyn FnMut(),
  }
  ```
  **(b) Bitflag-based** (simpler, avoids lifetime/borrow issues):
  ```rust
  pub struct ControllerCtx<'a> {
      pub widget_id: WidgetId,
      pub bounds: Rect,
      pub interaction: &'a InteractionState,
      pub actions: &'a mut Vec<WidgetAction>,
      /// Accumulated side-effect requests (set by controller, read by framework).
      pub requests: ControllerRequests,
  }

  bitflags! {
      pub struct ControllerRequests: u8 {
          const PAINT = 0b0001;
          const ANIM_FRAME = 0b0010;
          const SET_ACTIVE = 0b0100;
          const CLEAR_ACTIVE = 0b1000;
          const REQUEST_FOCUS = 0b10000;
      }
  }
  ```
  **Recommended**: option (b). Closures create complex lifetime relationships that
  make controller testing difficult. Bitflags are simple to construct in tests.
  The framework reads the flags after controller dispatch and applies side effects.
- [ ] Widgets attach controllers via `fn controllers(&self) -> &[Box<dyn EventController>]`
  (see Section 08 for Widget trait integration)
- [ ] Note: `WidgetAction` enum (in `oriterm_ui/src/widgets/mod.rs`) is retained as-is --
  controllers emit the same action variants. What changes is the emission mechanism:
  controllers emit via `ControllerCtx::emit_action()` instead of returning
  `WidgetResponse { action: Some(..) }`. The `actions` field is already part of the
  recommended option (b) struct above.
- [ ] Implement `emit_action(&mut self, action: WidgetAction)` convenience method on
  `ControllerCtx` (pushes to the `actions` vec). The event pipeline collects emitted
  actions after controller dispatch and routes them to the application layer.

---

## 04.2 HoverController

**File(s):** `oriterm_ui/src/controllers/hover.rs`

Tracks enter/leave state and provides callbacks. Replaces all manual hover tracking.

- [ ] Define `HoverController`:
  ```rust
  pub struct HoverController {
      on_enter: Option<WidgetAction>,
      on_leave: Option<WidgetAction>,
      /// Fires continuously while pointer is over the widget.
      on_move: bool,
  }
  ```
- [ ] Responds to `LifecycleEvent::HotChanged`:
  - `is_hot: true` → emit `on_enter` action, `request_paint()`
  - `is_hot: false` → emit `on_leave` action, `request_paint()`
- [ ] Does NOT set active or capture events — it's purely observational
- [ ] Unit tests: HoverController emits enter/leave actions at correct times

---

## 04.3 ClickController

**File(s):** `oriterm_ui/src/controllers/click.rs`

Click recognition with single/double/triple click detection.

- [ ] Define `ClickController`:
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
- [ ] On `MouseDown`: record position and time, `set_active(true)`, `request_paint()`
- [ ] On `MouseUp`:
  - If distance from press < threshold: increment `click_count`, emit `Clicked`
  - If within multi-click timeout of last press: emit `DoubleClicked` / `TripleClicked`
  - `set_active(false)`, `request_paint()`
- [ ] On `MouseMove` while active: if distance > threshold, cancel click (still active for
  potential drag handoff)
- [ ] Emit actions: `Clicked(id)`, `DoubleClicked(id)`, `TripleClicked(id)`
- [ ] **Click/Drag handoff**: When a widget has both ClickController and DragController,
  they must cooperate. On `MouseDown`, ClickController records the press. On `MouseMove`,
  if the drag threshold is exceeded, ClickController cancels its click and DragController
  transitions to `Dragging`. Implementation: controllers on the same widget are dispatched
  in order. DragController should have a higher-priority phase or check whether
  ClickController already consumed the event. Recommended: both controllers operate
  independently; DragController's `Pending → Dragging` transition implicitly invalidates
  ClickController's press state because ClickController checks distance on `MouseUp`.
- [ ] Unit tests: click, double-click, click-then-drag-cancels

---

## 04.4 DragController

**File(s):** `oriterm_ui/src/controllers/drag.rs`

Drag recognition with threshold.

- [ ] Define `DragController`:
  ```rust
  pub struct DragController {
      state: DragState,
      start_pos: Point,
      /// Minimum distance before drag begins (prevents accidental drags).
      threshold: f32,
  }

  enum DragState {
      Idle,
      Pending { pos: Point },  // Mouse down, waiting for threshold
      Dragging,
  }
  ```
- [ ] On `MouseDown`: transition to `Pending`, record position, `set_active(true)`
- [ ] On `MouseMove` while `Pending`:
  - If distance > threshold: transition to `Dragging`, emit `DragStart`
  - Otherwise: stay pending
- [ ] On `MouseMove` while `Dragging`: emit `DragUpdate { delta, total }`
- [ ] On `MouseUp`:
  - If `Dragging`: emit `DragEnd`
  - If `Pending`: emit nothing (it was a click, not a drag — ClickController handles it)
  - `set_active(false)`, transition to `Idle`
- [ ] Actions: `DragStart { pos }`, `DragUpdate { delta, total_delta }`, `DragEnd { pos }`
- [ ] Unit tests: drag threshold, drag delta accumulation, release without threshold = no drag

---

## 04.5 ScrollController

**File(s):** `oriterm_ui/src/controllers/scroll.rs`

Scroll event handling (wheel and trackpad).

- [ ] Define `ScrollController`:
  ```rust
  pub struct ScrollController {
      line_height: f32,
  }
  ```
- [ ] On `Scroll`: convert delta (Lines → pixels via `line_height`), emit
  `ScrollBy { delta_x, delta_y }`
- [ ] Does NOT set active (scroll is instantaneous, no capture needed)
- [ ] Phase: `Bubble` (children get scroll first; if unhandled, parent scrolls)
- [ ] Unit tests: line-to-pixel conversion, pixel passthrough

---

## 04.6 FocusController

**File(s):** `oriterm_ui/src/controllers/focus.rs`

Keyboard focus management with tab navigation.

- [ ] Define `FocusController`:
  ```rust
  pub struct FocusController {
      /// Tab index for focus ordering (lower = earlier).
      tab_index: Option<i32>,
  }
  ```
- [ ] On `KeyDown(Tab)`: call `FocusManager::focus_next()` (or `focus_prev()` with
  Shift). This is a framework-level operation, not a `WidgetAction` -- the controller
  calls it directly via `ControllerCtx`.
- [ ] On `LifecycleEvent::FocusChanged`: `request_paint()` so the widget redraws
  with its focused/unfocused visual state. No action emitted -- the visual state
  change is handled by `VisualStateAnimator` reading `InteractionState::is_focused()`.
- [ ] On `MouseDown` (if widget is focusable): call `request_focus()` via `ControllerCtx`
- [ ] Unit tests: tab navigation order, shift-tab reverse, focus on click

---

## 04.7 Completion Checklist

- [ ] `EventController` trait with `handle_event()`, `handle_lifecycle()`, `phase()`, `reset()`
- [ ] `ControllerCtx` provides `set_active()`, `request_focus()`, `request_paint()`,
  `request_anim_frame()`
- [ ] `HoverController` replaces all manual hover tracking
- [ ] `ClickController` handles single/double/triple click with threshold
- [ ] `DragController` handles drag with threshold and start/update/end lifecycle
- [ ] `ScrollController` handles wheel events with line-to-pixel conversion
- [ ] `FocusController` handles tab navigation and focus-on-click
- [ ] All controllers are independently unit-testable without a real window
- [ ] Controllers compose: a widget can have HoverController + ClickController + FocusController
- [ ] Test files: If controllers stay as flat files (`hover.rs`, `click.rs`, etc.), use a
  single `oriterm_ui/src/controllers/tests.rs`. If any controller is complex enough to need
  its own submodule, convert it to `hover/mod.rs` + `hover/tests.rs` per test-organization.md.
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** A test widget with three controllers (Hover + Click + Focus) correctly
receives hover enter/leave, click, and focus events. Each controller tested independently
and in combination.
