---
section: "01"
title: "Headless Test Harness"
status: complete
reviewed: true
goal: "Build a complete headless widget testing infrastructure that enables creating widgets, simulating input, inspecting interaction state, controlling time, and verifying rendering output — all without a GPU or window."
inspired_by:
  - "masonry TestHarness (src/testing/harness.rs) — architecture, input simulation, WidgetRef inspection, snapshot testing"
  - "GPUI TestAppContext (src/app/test_context.rs) — deterministic time, entity introspection"
  - "egui kittest (crates/egui_kittest/) — AccessKit-based widget queries, snapshot comparison"
  - "iced Simulator (test/src/simulator.rs) — Selector trait for polymorphic widget finding"
  - "ratatui TestBackend — buffer-based rendering verification"
depends_on: []
sections:
  - id: "01.1"
    title: "MockMeasurer & Test Theme"
    status: complete
  - id: "01.2a"
    title: "Move Shared Pipeline Functions to oriterm_ui"
    status: complete
  - id: "01.2"
    title: "WidgetTestHarness Core"
    status: complete
  - id: "01.3"
    title: "Input Simulation API"
    status: complete
  - id: "01.4"
    title: "State Inspection API"
    status: complete
  - id: "01.5"
    title: "Time Control & Animation Testing"
    status: complete
  - id: "01.6"
    title: "Rendering Verification"
    status: complete
  - id: "01.7"
    title: "Widget Query System"
    status: complete
  - id: "01.7b"
    title: "Overlay & Dialog Testing Support"
    status: complete
  - id: "01.8"
    title: "Integration Tests for Existing Widgets"
    status: complete
  - id: "01.9"
    title: "Completion Checklist"
    status: complete
---

# Section 01: Headless Test Harness

**Status:** Not Started
**Goal:** A `WidgetTestHarness` that wraps any widget tree and provides: input simulation (mouse, keyboard), interaction state inspection (hot, active, focused), deterministic time control (for animation testing), rendering verification (DrawList assertions), and widget queries (find by ID, by type). All without requiring a GPU, window, or real font stack.

**Context:** We have 35 widget types and zero integration tests. Every interaction change requires manual testing on Windows — launch the binary, click around, visually verify. This is the single biggest bottleneck for iteration speed. Every reference framework (masonry, GPUI, egui, iced, druid) has headless testing. We're the only one without it.

Currently, our unit tests cover:
- Controller dispatch in isolation (ClickController, HoverController)
- Propagation planning logic (plan_propagation pure function)
- DrawList command building
- Visual state transition evaluation

What we CAN'T test today:
- Full event propagation through a widget tree
- Interaction state transitions (hover -> press -> release -> click action)
- Focus traversal (tab through widgets)
- Layout-dependent behavior (hit testing requires computed LayoutNode)
- Widget paint output (DrawList commands produced by a real widget)
- Animation progression over time

**Reference implementations:**
- **masonry** `src/testing/harness.rs` (670 lines): TestHarness wraps RenderRoot. `mouse_move_to(id)`, `mouse_button_press()`, `pop_action()`. Recording widgets capture method calls. Best model for our architecture.
- **GPUI** `src/app/test_context.rs`: TestAppContext with deterministic `TestDispatcher`, `ForegroundExecutor`. Entity/view introspection. MockClock pattern.
- **egui** `crates/egui_kittest/`: Harness wraps `egui::Context`. Widget queries via AccessKit tree (by label, role). Snapshot comparison.
- **iced** `test/src/simulator.rs`: Headless `UserInterface` + `Selector` trait for finding widgets (by text, role, custom predicate).

---

## 01.1 MockMeasurer & Test Theme

**File(s):** `oriterm_ui/src/testing/mod.rs`, `oriterm_ui/src/testing/mock_measurer.rs`

**Existing code:** `MockMeasurer` and `TEST_THEME` already exist at `oriterm_ui/src/widgets/tests.rs` (8px/char, 16px line height, `UiTheme::dark()`). They are behind `#[cfg(test)] pub(crate) mod tests;` and need to be promoted to a public module accessible from integration tests in other crates.

**Module structure:**
```
testing/
  mod.rs              (test_theme(), re-exports, #[cfg(test)] mod tests;)
  mock_measurer.rs    (MockMeasurer)
  harness.rs          (WidgetTestHarness struct + constructor + layout)
  harness_dispatch.rs (process_event, lifecycle, animation)
  harness_input.rs    (mouse/keyboard/scroll/drag simulation)
  harness_inspect.rs  (state inspection, paint capture)
  widget_ref.rs       (WidgetRef)
  query.rs            (widget query system)
  render_assert.rs    (DrawList assertion helpers)
  tests.rs            (self-tests for the harness)
```

- [x] Create `oriterm_ui/src/testing/` module directory
- [x] Move `MockMeasurer` from `widgets/tests.rs` to `testing/mock_measurer.rs` (promote from `pub(crate)` to `pub`)
- [x] Move `TEST_THEME` to `testing/mod.rs` as `pub fn test_theme() -> UiTheme`
- [x] Update `widgets/tests.rs` to import from `crate::testing::MockMeasurer` instead of defining it locally
- [x] Add `#[cfg(any(test, feature = "testing"))] pub mod testing;` to `oriterm_ui/src/lib.rs` (avoids shipping test infrastructure in release builds; the `testing` feature flag is needed for integration tests in other crates like `oriterm`)
- [x] Add `testing = []` feature to `oriterm_ui/Cargo.toml`
- [x] Verify: `MockMeasurer` compiles and returns non-zero metrics for `"Hello"` (existing tests already verify this)

---

## 01.2a Move Shared Pipeline Functions to oriterm_ui

**File(s):** `oriterm_ui/src/pipeline.rs` (new, ~180 lines), `oriterm/src/app/widget_pipeline/mod.rs` (updated to re-export)

**Prerequisite for Sections 01.2, 02, and 04.** The following functions currently live in `oriterm/src/app/widget_pipeline/mod.rs` and must be moved to `oriterm_ui/src/pipeline.rs` so both the app layer and the test harness can use the same code:

- `prepare_widget_tree` (~20 lines)
- `prepare_widget_frame` (~45 lines)
- `register_widget_tree` (~8 lines)
- `collect_focusable_ids` (~10 lines)
- `apply_dispatch_requests` (~30 lines)
- `DispatchResult` enum (~30 lines)
- `dispatch_step` (~20 lines)

Total: ~180 lines, well under 500-line limit.

- [x] Create `oriterm_ui/src/pipeline.rs` with the functions listed above
- [x] Change visibility from `pub(crate)`/`pub(super)` to `pub`
- [x] Update `oriterm/src/app/widget_pipeline/mod.rs` to re-export from `oriterm_ui::pipeline`
- [x] Update all call sites in `oriterm/src/app/` to use the new path
- [x] Run `./build-all.sh` and `./test-all.sh` after the move

**Risk:** `DispatchResult` and `dispatch_step` reference `oriterm_ui` types that are already public (`ControllerRequests`, `DispatchOutput`, `InputEvent`, `InteractionManager`, etc.), so the move is straightforward. The `apply_requests` wrapper function (3 lines) can stay in `oriterm` since it just calls `apply_dispatch_requests`.

---

## 01.2 WidgetTestHarness Core

**File(s):** `oriterm_ui/src/testing/harness.rs` (struct + constructor + layout, ~250 lines), `oriterm_ui/src/testing/harness_dispatch.rs` (event pipeline + lifecycle, ~200 lines)

The harness wraps a root widget and provides the full framework pipeline: layout -> event dispatch -> interaction state -> lifecycle -> animation -> paint. All deterministic, no external dependencies.

- [x] Define `WidgetTestHarness`:
  ```rust
  /// Headless test harness for widget integration testing.
  ///
  /// Wraps a root widget and wires up the full framework pipeline:
  /// layout solver, hit testing, event propagation, interaction state,
  /// lifecycle events, and animation scheduling — without requiring
  /// a GPU, window, or real font stack.
  pub struct WidgetTestHarness {
      /// The root widget under test.
      widget: Box<dyn Widget>,
      /// Computed layout tree (from last `rebuild_layout()`).
      layout: LayoutNode,
      /// Interaction state manager.
      interaction: InteractionManager,
      /// Focus manager.
      focus: FocusManager,
      /// Animation/paint request scheduler.
      scheduler: RenderScheduler,
      /// Current simulated time (Instant is cross-platform safe; constructed
      /// via Instant::now() at init, then advanced via Duration addition).
      clock: Instant,
      /// Mock text measurer.
      measurer: MockMeasurer,
      /// Theme for rendering.
      theme: UiTheme,
      /// Viewport size.
      viewport: Rect,
      /// Collected actions from last event dispatch.
      pending_actions: Vec<WidgetAction>,
      /// Frame request flags (shared with widget contexts).
      frame_requests: FrameRequestFlags,
      /// Current mouse position (for mouse_down/mouse_up without explicit pos).
      mouse_pos: Point,
      /// Invalidation tracker for layout dirty tracking (simplified when
      /// Section 03 replaces paint-dirty tracking with DamageTracker).
      invalidation: InvalidationTracker,
  }
  ```

- [x] Implement constructor:
  ```rust
  impl WidgetTestHarness {
      /// Creates a harness wrapping `widget` in a viewport of `size`.
      pub fn new(widget: impl Widget + 'static) -> Self { ... }

      /// Creates a harness with a custom viewport size.
      pub fn with_size(widget: impl Widget + 'static, width: f32, height: f32) -> Self { ... }
  }
  ```

- [x] Implement `rebuild_layout()`:
  ```rust
  /// Recomputes layout from the root widget's `layout()` method.
  ///
  /// Must be called after construction and after any structural change
  /// (widget add/remove, text change that affects size). Called
  /// automatically by input simulation methods.
  pub fn rebuild_layout(&mut self) {
      let ctx = LayoutCtx { measurer: &self.measurer, theme: &self.theme };
      let layout_box = self.widget.layout(&ctx);
      self.layout = compute_layout(&layout_box, self.viewport);
      // Rebuild parent map for focus_within tracking.
      let parent_map = build_parent_map(&self.layout);
      self.interaction.set_parent_map(parent_map);
      // Register all widget IDs with InteractionManager (idempotent).
      register_widget_tree(&mut *self.widget, &mut self.interaction);
      // Rebuild focus order from tree traversal.
      let mut focusable = Vec::new();
      collect_focusable_ids(&mut *self.widget, &mut focusable);
      self.focus.set_focus_order(focusable);
  }
  ```

- [x] Implement lifecycle delivery:
  ```rust
  /// Drains pending lifecycle events and delivers them to the widget tree.
  ///
  /// Mirrors `prepare_widget_tree` from `oriterm_ui/src/pipeline.rs`:
  /// for each widget (depth-first via for_each_child_mut):
  ///   1. Filter lifecycle events by widget.id() and deliver via
  ///      dispatch_lifecycle_to_controllers + widget.lifecycle().
  ///   2. Deliver anim frame event if pending for this widget.
  ///   3. Update visual state animator from interaction state.
  fn deliver_lifecycle_events(&mut self) {
      let events = self.interaction.drain_events();
      prepare_widget_tree(
          &mut *self.widget,
          &self.interaction,
          &events,
          None, // anim_event — separate, see tick_animation_frame()
          Some(&self.frame_requests),
          self.clock,
      );
  }
  ```

- [x] Implement `process_event()` -- the internal entry point that replicates the full dispatch pipeline:
  1. Hit test the layout tree for the event position (via `layout_hit_test_path`).
  2. Plan propagation (via `plan_propagation`).
  3. Dispatch to widget tree (via `dispatch_to_widget_tree`).
  4. Apply controller requests: `SET_ACTIVE` -> `interaction.set_active(source)`,
     `CLEAR_ACTIVE` -> `interaction.clear_active()`, `REQUEST_FOCUS` ->
     `interaction.request_focus(source, &mut focus)`, `FOCUS_NEXT` ->
     `focus.focus_next()` then `interaction.request_focus(new, &mut focus)`,
     `FOCUS_PREV` -> `focus.focus_prev()` then `interaction.request_focus(new, &mut focus)`.
     This logic mirrors `apply_dispatch_requests` in `oriterm_ui/src/pipeline.rs`.
  5. Update hot path from hit test result (via `interaction.update_hot_path()`).
  6. Drain lifecycle events (via `interaction.drain_events()`).
  7. Deliver lifecycle events to widget tree (via `deliver_lifecycle_events`).
  8. Collect emitted actions into `pending_actions`.
  9. Forward `PAINT` / `ANIM_FRAME` request flags to `scheduler`.
- [x] Implement `rebuild_focus_order()` -- calls `collect_focusable_ids` on widget tree, then `focus.set_focus_order()`. Called automatically after `rebuild_layout()`.
- [x] Verify: Can construct a harness with a `ButtonWidget`, rebuild layout, and inspect the LayoutNode

---

## 01.3 Input Simulation API

**File(s):** `oriterm_ui/src/testing/harness_input.rs` (methods on `WidgetTestHarness`, ~200 lines)

High-level input simulation methods that mirror real user interactions. Each method: constructs the appropriate `InputEvent`, dispatches through the full pipeline, updates interaction state, delivers lifecycle events, and returns results.

- [x] Mouse movement:
  ```rust
  /// Moves the mouse to a screen-space position.
  ///
  /// Updates the hot path (which widgets are hovered), delivers
  /// `HotChanged` lifecycle events, and returns the hit path.
  pub fn mouse_move(&mut self, pos: Point) -> &[WidgetId] { ... }

  /// Moves the mouse to the center of a widget by ID.
  ///
  /// Finds the widget's layout bounds, computes center point,
  /// and calls `mouse_move()`. Panics if the widget ID is not found.
  pub fn mouse_move_to(&mut self, widget_id: WidgetId) -> &[WidgetId] { ... }
  ```

- [x] Mouse buttons:
  ```rust
  /// Simulates a mouse button press at the current position.
  pub fn mouse_down(&mut self, button: MouseButton) { ... }

  /// Simulates a mouse button release at the current position.
  pub fn mouse_up(&mut self, button: MouseButton) { ... }

  /// Convenience: mouse_move_to(id) + mouse_down + mouse_up.
  ///
  /// Returns the actions emitted during the click.
  pub fn click(&mut self, widget_id: WidgetId) -> Vec<WidgetAction> { ... }

  /// Double-click: two clicks within the multi-click timeout.
  pub fn double_click(&mut self, widget_id: WidgetId) -> Vec<WidgetAction> { ... }
  ```

- [x] Drag simulation:
  ```rust
  /// Simulates a drag from `start` to `end` with `steps` intermediate moves.
  pub fn drag(&mut self, start: Point, end: Point, steps: usize) -> Vec<WidgetAction> { ... }
  ```

- [x] Keyboard:
  ```rust
  /// Simulates a key press + release.
  pub fn key_press(&mut self, key: Key, modifiers: Modifiers) -> Vec<WidgetAction> { ... }

  /// Simulates typing a string (one key event per character).
  pub fn type_text(&mut self, text: &str) -> Vec<WidgetAction> { ... }

  /// Tab to the next focusable widget.
  pub fn tab(&mut self) -> Vec<WidgetAction> { ... }

  /// Shift+Tab to the previous focusable widget.
  pub fn shift_tab(&mut self) -> Vec<WidgetAction> { ... }
  ```

- [x] Scroll:
  ```rust
  /// Simulates a scroll wheel event at the current mouse position.
  pub fn scroll(&mut self, delta: ScrollDelta) -> Vec<WidgetAction> { ... }
  ```

- [x] Action collection:
  ```rust
  /// Returns and clears all pending actions from the last event.
  pub fn take_actions(&mut self) -> Vec<WidgetAction> { ... }

  /// Returns the next pending action, or None.
  pub fn pop_action(&mut self) -> Option<WidgetAction> { ... }
  ```

- [x] Verify: `click(button_id)` on a ButtonWidget produces `WidgetAction::Clicked(button_id)`

---

## 01.4 State Inspection API

**File(s):** `oriterm_ui/src/testing/harness_inspect.rs` (~150 lines), `oriterm_ui/src/testing/widget_ref.rs`

After simulating input, tests need to inspect widget state: interaction state, layout bounds, visual state animator progress, controller state.

- [x] Interaction state queries:
  ```rust
  /// Returns the interaction state for a widget.
  pub fn interaction_state(&self, widget_id: WidgetId) -> &InteractionState { ... }

  /// Whether the pointer is over this widget.
  pub fn is_hot(&self, widget_id: WidgetId) -> bool { ... }

  /// Whether this widget has captured the mouse.
  pub fn is_active(&self, widget_id: WidgetId) -> bool { ... }

  /// Whether this widget has keyboard focus.
  pub fn is_focused(&self, widget_id: WidgetId) -> bool { ... }

  /// The currently focused widget, if any.
  pub fn focused_widget(&self) -> Option<WidgetId> { ... }

  /// The currently active (capturing) widget, if any.
  pub fn active_widget(&self) -> Option<WidgetId> { ... }
  ```

- [x] Layout queries:
  ```rust
  /// Returns the layout bounds of a widget by ID.
  ///
  /// Panics if the widget ID is not found in the layout tree.
  pub fn widget_bounds(&self, widget_id: WidgetId) -> Rect { ... }

  /// Returns the layout bounds of a widget by ID, or None.
  pub fn try_widget_bounds(&self, widget_id: WidgetId) -> Option<Rect> { ... }
  ```

- [x] Widget tree inspection:
  ```rust
  /// Visits every widget in the tree depth-first.
  pub fn inspect_widgets(&self, visitor: impl FnMut(&dyn Widget)) { ... }

  /// Returns a list of all widget IDs in the tree.
  pub fn all_widget_ids(&self) -> Vec<WidgetId> { ... }

  /// Returns a list of all focusable widget IDs in tab order.
  pub fn focusable_widgets(&self) -> Vec<WidgetId> { ... }
  ```

- [x] WidgetRef for typed access (inspired by masonry):
  ```rust
  /// Read-only reference to a widget in the harness.
  pub struct WidgetRef<'a> {
      widget: &'a dyn Widget,
      interaction: &'a InteractionState,
      bounds: Rect,
  }

  impl<'a> WidgetRef<'a> {
      pub fn is_hot(&self) -> bool { self.interaction.is_hot() }
      pub fn is_active(&self) -> bool { self.interaction.is_active() }
      pub fn is_focused(&self) -> bool { self.interaction.is_focused() }
      pub fn bounds(&self) -> Rect { self.bounds }
      pub fn sense(&self) -> Sense { self.widget.sense() }
  }
  ```

- [x] `get_widget` method on harness:
  ```rust
  /// Returns a WidgetRef for a widget by ID.
  pub fn get_widget(&self, widget_id: WidgetId) -> WidgetRef<'_> { ... }
  ```

- [x] Verify: After `mouse_move_to(button_id)`, `is_hot(button_id)` returns `true`

---

## 01.5 Time Control & Animation Testing

**File(s):** `oriterm_ui/src/testing/harness_dispatch.rs` (time control methods call `tick_animation_frame` which is the dispatch pipeline)

Our animation engine (spring physics, easing curves, AnimProperty, VisualStateAnimator) needs deterministic testing. The harness controls a virtual clock.

- [x] Time advancement:
  ```rust
  /// Advances the simulated clock by `duration`.
  ///
  /// Ticks animation frames for all widgets that requested them.
  /// Multiple calls accumulate: `advance(100ms) + advance(100ms)` = 200ms total.
  pub fn advance_time(&mut self, duration: Duration) { ... }

  /// Returns the current simulated time.
  pub fn now(&self) -> Instant { ... }
  ```

- [x] Animation frame delivery:
  ```rust
  /// Ticks one animation frame at the current time.
  ///
  /// 1. Promote deferred repaints from RenderScheduler (cursor blink timers, etc.).
  /// 2. Take anim frame requests from RenderScheduler.
  /// 3. Build AnimFrameEvent with delta since last frame.
  /// 4. Walk widget tree via prepare_widget_tree, delivering anim frames
  ///    and updating visual state animators.
  /// 5. Forward new frame_requests flags back to RenderScheduler.
  ///
  /// Called automatically by `advance_time()`.
  fn tick_animation_frame(&mut self) {
      self.scheduler.promote_deferred(self.clock);
      let anim_widgets = self.scheduler.take_anim_frames();
      if anim_widgets.is_empty() { return; }
      let anim_event = AnimFrameEvent { /* delta, now */ };
      prepare_widget_tree(
          &mut *self.widget,
          &self.interaction,
          &[], // lifecycle events already drained
          Some(&anim_event),
          Some(&self.frame_requests),
          self.clock,
      );
      // Forward accumulated flags to scheduler.
      self.flush_frame_requests();
  }
  ```

- [x] Run-until-stable:
  ```rust
  /// Advances time in 16ms steps until no widgets request animation frames.
  ///
  /// Panics after 300 steps (5 seconds simulated) to prevent infinite loops
  /// from buggy animations.
  pub fn run_until_stable(&mut self) { ... }
  ```

- [x] Verify: Create a widget with a 200ms hover animation. `mouse_move_to()`, `advance_time(100ms)`, verify animator is at ~50%. `advance_time(100ms)`, verify animator is at 100%.

---

## 01.6 Rendering Verification

**File(s):** `oriterm_ui/src/testing/harness_inspect.rs` (paint/render methods), `oriterm_ui/src/testing/render_assert.rs` (assertion helpers)

Widgets paint to a `DrawList`. The harness captures draw commands and provides assertion helpers.

- [x] Paint capture:
  ```rust
  /// Paints the widget tree and returns the draw commands.
  ///
  /// Uses MockMeasurer and test theme. No GPU required — returns
  /// the raw DrawList that would be sent to the GPU renderer.
  pub fn paint(&mut self) -> &DrawList { ... }

  /// Paints and returns a copy of the draw commands.
  pub fn render(&mut self) -> DrawList { ... }
  ```

- [x] DrawList assertion helpers:
  ```rust
  /// Asserts that the draw list contains a rect with the given fill color.
  pub fn assert_has_rect_with_color(draw_list: &DrawList, color: Color) { ... }

  /// Asserts that the draw list contains text matching the given string.
  pub fn assert_has_text(draw_list: &DrawList, text: &str) { ... }

  /// Returns the number of draw commands.
  pub fn command_count(draw_list: &DrawList) -> usize { ... }

  /// Returns all rect commands in the draw list.
  pub fn rects(draw_list: &DrawList) -> Vec<&DrawCommand> { ... }

  /// Returns all text commands in the draw list.
  pub fn texts(draw_list: &DrawList) -> Vec<&DrawCommand> { ... }
  ```

- [x] Snapshot testing (optional, lower priority):
  ```rust
  /// Serializes draw commands to a deterministic text format for snapshot comparison.
  ///
  /// Example output:
  /// ```text
  /// RECT (10, 20, 100, 30) fill=#333333 radius=4
  /// TEXT (14, 22) "Click me" color=#ffffff
  /// ```
  pub fn snapshot_text(draw_list: &DrawList) -> String { ... }
  ```

- [x] Verify: Paint a ButtonWidget, assert the DrawList contains a rect (background) and text (label)

---

## 01.7 Widget Query System

**File(s):** `oriterm_ui/src/testing/query.rs`

Find widgets in the tree by criteria other than ID (inspired by iced's Selector and egui's AccessKit queries).

- [x] Query by type:
  ```rust
  /// Finds the first widget whose Debug name contains the given substring.
  pub fn find_by_name(&self, name: &str) -> Option<WidgetId> { ... }
  ```

- [x] Query by sense:
  ```rust
  /// Returns all widgets with the given sense flags.
  pub fn widgets_with_sense(&self, sense: Sense) -> Vec<WidgetId> { ... }
  ```

- [x] Query by bounds:
  ```rust
  /// Returns the widget at the given point (hit testing).
  pub fn widget_at(&self, pos: Point) -> Option<WidgetId> { ... }
  ```

- [x] Helper macros:
  ```rust
  /// Generates unique WidgetIds for use in tests.
  ///
  /// Usage: `let [button, label, toggle] = widget_ids();`
  macro_rules! widget_ids {
      () => { ... };
  }
  ```

- [x] Verify: `widgets_with_sense(Sense::click())` returns all clickable widgets in the tree

---

## 01.7b Overlay & Dialog Testing Support

**File(s):** `oriterm_ui/src/testing/harness.rs` (optional extension)

Overlays (dropdowns, modals/dialogs) are managed by `OverlayManager`, which sits alongside the widget tree. Testing overlay interactions requires either:
- **Option A (recommended for v1):** Don't include OverlayManager in the harness.
  Test overlay widgets in isolation by wrapping them as the root widget of a harness.
  This works for most cases: a dropdown menu widget can be tested as a standalone
  widget tree. The OverlayManager's event routing and dismiss logic can be tested
  via unit tests on `OverlayManager` directly.
- **Option B (future):** Add `OverlayTestHarness` that wraps `OverlayManager` +
  `LayerTree` + `LayerAnimator` and provides `push_overlay()`, `push_modal()`,
  `dismiss_topmost()`, etc. This enables end-to-end testing of dropdown open -> select
  -> dismiss flows.

For v1, document that overlay testing uses Option A. Add a TODO for Option B.

- [x] Document overlay testing strategy in harness module docs
- [x] Verify: can test a dropdown menu widget in isolation by wrapping it as root widget
- [x] Add `// TODO: OverlayTestHarness for end-to-end overlay flow testing` comment

---

## 01.8 Integration Tests for Existing Widgets

**File(s):** `oriterm_ui/src/testing/tests.rs` (harness self-tests), plus additions to existing widget test files: `oriterm_ui/src/widgets/button/tests.rs`, `oriterm_ui/src/widgets/toggle/tests.rs`, `oriterm_ui/src/widgets/scroll/tests.rs`

Per test-organization.md, widget integration tests go in each widget's existing `tests.rs` file. Harness self-tests (focus traversal, paint output) go in `testing/tests.rs`.

Write integration tests using the harness to validate the full pipeline for key widget types. This proves the harness works AND catches existing bugs.

- [x] **ButtonWidget integration test** (in `button/tests.rs`):
  - Create button with known ID
  - Verify layout produces non-zero bounds
  - `mouse_move_to(button_id)` -> assert `is_hot`
  - `mouse_down(Left)` -> assert `is_active`
  - `mouse_up(Left)` -> assert `!is_active`, pop `Clicked` action
  - Move mouse away -> assert `!is_hot`

- [x] **ToggleWidget integration test** (in `toggle/tests.rs`):
  - Create toggle with known ID
  - `click(toggle_id)` -> assert `Toggled` action, value flipped
  - `click(toggle_id)` -> assert `Toggled` again, value flipped back

- [x] **Focus traversal test** (in `testing/tests.rs`):
  - Create 3 focusable widgets in a stack
  - `click(first)` -> assert `is_focused(first)`
  - `tab()` -> assert `is_focused(second)`
  - `tab()` -> assert `is_focused(third)`
  - `shift_tab()` -> assert `is_focused(second)`

- [x] **Scroll container test** (in `scroll/tests.rs`):
  - Create scroll container with tall content
  - `mouse_move_to(scroll_id)`
  - `scroll(delta)` -> verify scroll offset changed

- [x] **Keyboard activation test** (in `button/tests.rs`):
  - Focus a button via `click(button_id)`
  - `key_press(Key::Enter)` -> assert `Clicked` action
  - `key_press(Key::Space)` -> assert `Clicked` action

- [x] **Paint output test** (in `testing/tests.rs`):
  - Create a button, paint it
  - Assert DrawList contains background rect and label text
  - `mouse_move_to(button_id)`, paint again
  - Assert background color changed (hover state)

- [x] Verify: All integration tests pass with `timeout 150 cargo test -p oriterm_ui`

---

## 01.9 Completion Checklist

- [x] `MockMeasurer` implements `TextMeasurer` with deterministic fixed-width metrics
- [x] Shared pipeline functions moved to `oriterm_ui/src/pipeline.rs` (prerequisite 01.2a)
- [x] `WidgetTestHarness::new(widget)` constructs harness with full pipeline wiring
- [x] `rebuild_layout()` computes layout + parent map + widget registration + focus order
- [x] `process_event()` replicates full dispatch pipeline (hit test -> propagation -> dispatch -> apply_requests -> hot path -> lifecycle -> actions)
- [x] Mouse simulation: `mouse_move()`, `mouse_move_to()`, `mouse_down()`, `mouse_up()`, `click()`, `double_click()`
- [x] Keyboard simulation: `key_press()`, `type_text()`, `tab()`, `shift_tab()`
- [x] Scroll simulation: `scroll()`
- [x] Drag simulation: `drag()`
- [x] State inspection: `is_hot()`, `is_active()`, `is_focused()`, `widget_bounds()`, `get_widget()`
- [x] Action collection: `take_actions()`, `pop_action()`
- [x] Time control: `advance_time()`, `run_until_stable()`
- [x] `RenderScheduler` integration: anim frame requests, deferred repaints, promote_deferred
- [x] Paint capture: `paint()`, `render()`
- [x] Widget queries: `find_by_name()`, `widgets_with_sense()`, `widget_at()`
- [x] Overlay testing strategy documented (Option A: test widgets in isolation)
- [x] No source file exceeds 500 lines (harness split: harness.rs, harness_dispatch.rs, harness_input.rs, harness_inspect.rs)
- [x] Tests in sibling tests.rs files (testing/tests.rs for harness self-tests, widget tests in widget/tests.rs)
- [x] ButtonWidget integration test passes (in `button/tests.rs`)
- [x] ToggleWidget integration test passes (in `toggle/tests.rs`)
- [x] Focus traversal integration test passes (in `testing/tests.rs`)
- [x] Paint output integration test passes (in `testing/tests.rs`)
- [x] `timeout 150 cargo test -p oriterm_ui` passes with no regressions
- [x] `./clippy-all.sh` clean

**Exit Criteria:** `WidgetTestHarness` can create a ButtonWidget, simulate hover -> click -> release, verify `is_hot` / `is_active` / `Clicked` action, paint the button, and assert the DrawList contains the expected commands -- all in a `#[test]` function with no GPU, window, or font stack.
