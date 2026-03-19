---
section: "02"
title: "Safety Rails"
status: complete
reviewed: true
goal: "Add debug assertions that catch the most common container widget bugs: double-visits, cross-phase child mismatch (layout vs dispatch), and lifecycle ordering violations."
inspired_by:
  - "masonry safety_rails.rs (src/widget/tests/safety_rails.rs, 370 lines) — Bloom filter child tracking, debug panic on missed children"
depends_on: []
sections:
  - id: "02.1"
    title: "Child Visitation Tracking"
    status: complete
  - id: "02.2"
    title: "Lifecycle Delivery Validation"
    status: complete
  - id: "02.3"
    title: "Safety Rail Tests"
    status: complete
  - id: "02.4"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Safety Rails

**Status:** Not Started
**Goal:** Debug-mode assertions that validate widget tree traversal correctness. When a container widget forgets to recurse to a child during event dispatch, lifecycle delivery, or pre-paint -- catch it immediately with a clear panic message instead of a silent logic bug.

**Context:** With 35 widgets, many containers (Stack, Container, FormLayout, FormSection, FormRow, SettingRow, PageContainer, Scroll, Dialog, Panel, SettingsPanel, WindowChrome), forgetting to visit a child in `for_each_child_mut()` is a realistic bug. Masonry found this so common they added Bloom filter tracking and debug assertions. We should too.

Our `for_each_child_mut` visitor pattern makes this feasible: we can wrap the visitor to track which children were actually yielded.

**Reference implementations:**
- **masonry** `src/widget/tests/safety_rails.rs` (370 lines): Tests that container widgets panic when they forget to recurse. `Bloom<WidgetId>` for child tracking. Validates `place_child()` called for every child in layout.

**Depends on:** None (pure additions to existing dispatch code). Best placed in `oriterm_ui/src/pipeline.rs` (which already contains the shared pipeline functions moved in Section 01.2a, now complete).

**Prerequisite step:** `oriterm_ui/src/pipeline.rs` is currently a flat file (271 lines).
Adding safety rail code plus tests requires converting it to a directory module:
`pipeline/mod.rs` + `pipeline/tests.rs`. This must happen first, before adding any
assertions. See test-organization.md. The app-layer re-export in
`oriterm/src/app/widget_pipeline/mod.rs` (line 7: `use oriterm_ui::pipeline::...`) will
continue to work unchanged after the conversion (Rust module paths are the same whether
the module is a flat file or directory).

**Implementation order within Section 02:**
1. Convert `pipeline.rs` to `pipeline/mod.rs` + `pipeline/tests.rs` (prerequisite)
2. Fix `register_widget()` in `manager.rs` to only push `WidgetAdded` on first registration (02.2 prerequisite)
3. Add `is_registered()` and debug tracking (`widget_added_delivered` field, `mark_widget_added_delivered`, `was_widget_added_delivered`) to `InteractionManager` (02.2)
4. Change `prepare_widget_tree` to take `&mut InteractionManager` and update all call sites (02.2 prerequisite for marking `WidgetAdded` delivery)
5. Add double-visit assertions to `tree.rs` and `pipeline/mod.rs` (02.1)
6. Add lifecycle assertions to `pipeline/mod.rs` (02.2)
7. Add cross-phase consistency helpers (`collect_layout_widget_ids`, `check_cross_phase_consistency`) to `pipeline/mod.rs` (02.1)
8. Add `Option<&HashSet<WidgetId>>` parameter to `deliver_event_to_tree` and `&mut HashSet<WidgetId>` to `dispatch_to_widget_tree` for layout-vs-dispatch comparison (02.1)
9. Integrate cross-phase checks at app-layer call sites where `compute_layout` + `deliver_event_to_tree` are called together (02.1)
10. Write tests (02.3)

This order ensures: library crate changes before binary crate changes,
prerequisites before dependents, and all production code before tests.

---

## 02.1 Child Visitation Tracking

**File(s):** `oriterm_ui/src/input/dispatch/tree.rs`, `oriterm_ui/src/pipeline/mod.rs` (converted from `pipeline.rs`), `oriterm_ui/src/interaction/manager.rs` (493 lines -- near the 500-line hard limit; the `is_registered` addition is small but monitor closely)

Add debug assertions that validate: (a) no child is visited twice during tree traversal, and (b) children seen during dispatch are a superset of children seen during layout.

### Double-visit detection

- [x] Add `#[cfg(debug_assertions)] use std::collections::HashSet;` to both `tree.rs` and `pipeline/mod.rs`.

- [x] In `dispatch_to_widget_tree` (`tree.rs`, line 138), wrap the existing `for_each_child_mut` call with a `HashSet`-based double-visit check. `widget.id()` is already captured on line 79 before the closure borrows `widget` mutably:
  ```rust
  #[cfg(debug_assertions)]
  let mut visited = HashSet::new();
  widget.for_each_child_mut(&mut |child| {
      #[cfg(debug_assertions)]
      {
          let child_id = child.id();
          debug_assert!(
              visited.insert(child_id),
              "Container widget {:?} visited child {:?} twice during event dispatch",
              id, child_id
          );
      }
      dispatch_to_widget_tree(child, event, actions, now, result);
  });
  ```

- [x] In `prepare_widget_tree` (`pipeline/mod.rs`, line 127), add the same double-visit check. A container that double-visits during the pre-paint phase is equally buggy. Same pattern: capture `widget.id()` before the closure, create a `HashSet`, assert on insert inside the closure.

### Cross-phase consistency (layout children vs dispatch children)

This check validates that every widget that was laid out is also reachable via `for_each_child_mut` during dispatch. The invariant is: **layout children must be a subset of dispatch children.** Dispatch visiting extra children not in layout is valid (e.g., `PageContainerWidget` yields all pages to `for_each_child_mut` but only lays out the active page).

> **This is the most complex sub-step.** The cross-phase check bridges layout
> (which runs during event handling, via `cached_content_layout`) and dispatch
> (which also runs during event handling, via `deliver_event_to_tree`). Both
> happen in the same function scope in the app layer, so explicit parameter
> passing is feasible and preferred over thread-locals.

- [x] Add helper functions to `pipeline/mod.rs`:
  ```rust
  /// Walks a `LayoutNode` tree and collects all `Some(widget_id)` values.
  #[cfg(debug_assertions)]
  pub fn collect_layout_widget_ids(node: &LayoutNode, out: &mut HashSet<WidgetId>) {
      if let Some(id) = node.widget_id {
          out.insert(id);
      }
      for child in &node.children {
          collect_layout_widget_ids(child, out);
      }
  }

  /// Asserts that every laid-out widget was also visited during dispatch.
  #[cfg(debug_assertions)]
  pub fn check_cross_phase_consistency(
      layout_ids: &HashSet<WidgetId>,
      dispatch_ids: &HashSet<WidgetId>,
  ) {
      for id in layout_ids {
          debug_assert!(
              dispatch_ids.contains(id),
              "Cross-phase mismatch: widget {:?} was laid out but never \
               visited by for_each_child_mut during dispatch",
              id
          );
      }
  }
  ```

- [x] Add an `Option<&mut HashSet<WidgetId>>` parameter to `dispatch_to_widget_tree` to collect visited IDs. At the top of each call, insert `widget.id()` if tracking is active. `dispatch_to_widget_tree` is crate-internal (only called from `deliver_event_to_tree` and tests), so this is not a breaking change.

- [x] Add an `Option<&HashSet<WidgetId>>` parameter to `deliver_event_to_tree` for layout IDs. Always present in the signature; callers pass `None` when no cross-phase check is needed. In debug mode, create a local `HashSet` for dispatch IDs, pass it through `dispatch_to_widget_tree`, then call `check_cross_phase_consistency` after dispatch completes:

  > **Practical note on `cfg`-gated parameters:** Rust does not support
  > conditionally adding function parameters via `#[cfg]`. Two clean options:
  > (a) Always include the parameter in the signature (`Option`, so passing
  > `None` in release is zero-cost -- the optimizer eliminates dead code).
  > (b) Use a newtype wrapper that's empty in release:
  > `struct DebugLayoutIds(#[cfg(debug_assertions)] HashSet<WidgetId>)`.
  > Option (a) is simpler and recommended.

  ```rust
  // In deliver_event_to_tree:
  #[cfg(debug_assertions)]
  let mut dispatch_ids = HashSet::new();
  #[cfg(debug_assertions)]
  let dispatch_ids_param = Some(&mut dispatch_ids);
  #[cfg(not(debug_assertions))]
  let dispatch_ids_param: Option<&mut HashSet<WidgetId>> = None;

  dispatch_to_widget_tree(widget, event, &delivery_actions, now, &mut result, dispatch_ids_param);

  #[cfg(debug_assertions)]
  if let Some(layout_ids) = layout_ids {
      crate::pipeline::check_cross_phase_consistency(layout_ids, &dispatch_ids);
  }
  ```

### App-layer integration

- [x] Add `#[cfg(debug_assertions)]` cross-phase checks at each app-layer call site where `compute_layout` and `deliver_event_to_tree` are called together. After layout, call `collect_layout_widget_ids()` into a local `HashSet` and pass `Some(&layout_ids)` to `deliver_event_to_tree`. Specific call sites:
  - `dialog_context/event_handling/mouse.rs` -- `cached_content_layout()` + `deliver_event_to_tree()` in click/scroll handlers (lines 204-210, 296-302).
  - `dialog_context/event_handling/mod.rs` -- `dispatch_dialog_content_move()` calls `cached_content_layout()` at line 333 then `deliver_event_to_tree()` at line 345.
  - `dialog_context/content_actions.rs` -- `dispatch_dialog_content_key()` calls `compute_layout()` at line 338 then `deliver_event_to_tree()` at line 353.
  - **NOT** `dialog_rendering.rs` -- it calls `prepare_widget_tree()` (pre-paint), not `compute_layout()`. No cross-phase check needed there.
  - **NOT** `content_actions.rs::setup_dialog_focus()` (line 441) -- it calls `compute_layout()` for parent map setup but never dispatches events. No cross-phase check needed.

- [x] If false positives appear beyond `PageContainerWidget` (which legitimately dispatches to more children than it lays out), add an opt-out trait method `fn skip_cross_phase_check(&self) -> bool { false }` to `Widget`.

---

## 02.2 Lifecycle Delivery Validation

**File(s):** `oriterm_ui/src/pipeline/mod.rs`, `oriterm_ui/src/interaction/manager.rs`

### Prerequisite: fix `register_widget()` (implement first)

- [x] Fix `InteractionManager::register_widget()` to only push `WidgetAdded` on first registration. Currently (line 176-180 of `manager.rs`), `entry().or_default()` is idempotent for the state map but `WidgetAdded` is pushed unconditionally every frame. Fix: use the `Entry` API return value to detect first insertion:
  ```rust
  pub fn register_widget(&mut self, widget_id: WidgetId) {
      use std::collections::hash_map::Entry;
      if let Entry::Vacant(e) = self.states.entry(widget_id) {
          e.insert(InteractionState::default());
          self.pending_events
              .push(LifecycleEvent::WidgetAdded { widget_id });
      }
  }
  ```
  Without this fix, the `WidgetAdded`-first assertion (below) is trivially satisfied because every widget receives `WidgetAdded` every frame, making the safety rail useless.

### Registered-widget assertion

- [x] Add `pub fn is_registered(&self, id: WidgetId) -> bool` to `InteractionManager` (`self.states.contains_key(&id)`). `InteractionManager` is already re-exported from `interaction/mod.rs`, so no re-export change needed.

- [x] In `prepare_widget_frame` (a free function in `pipeline/mod.rs`), inside the existing `for event in lifecycle_events.iter().filter(...)` loop at line 171, add a debug assertion before `dispatch_lifecycle_to_controllers` that validates the target widget is registered (except for `WidgetAdded` itself, which is the registration event):
  ```rust
  #[cfg(debug_assertions)]
  debug_assert!(
      interaction.is_registered(id)
          || matches!(event, LifecycleEvent::WidgetAdded { .. }),
      "Lifecycle event {:?} delivered to unregistered widget {:?}",
      event, id
  );
  ```

### WidgetAdded-first assertion

- [x] Assert that `WidgetAdded` is the first lifecycle event any widget receives.

  This requires state that persists across frames (`WidgetAdded` is delivered on frame 1, other events like `HotChanged` arrive on later frames). Neither `is_registered()` (trivially true after `register_widget`) nor a per-call `HashSet` (resets each frame) works. The correct approach: add a `#[cfg(debug_assertions)]` `HashSet<WidgetId>` field to `InteractionManager` that tracks which widgets have had `WidgetAdded` delivered.

  **Changes to `InteractionManager`** (`oriterm_ui/src/interaction/manager.rs`):
  ```rust
  pub struct InteractionManager {
      // ... existing fields ...
      /// Tracks which widgets have received WidgetAdded delivery (debug only).
      #[cfg(debug_assertions)]
      widget_added_delivered: HashSet<WidgetId>,
  }

  impl InteractionManager {
      /// Records that WidgetAdded has been delivered for this widget (debug).
      #[cfg(debug_assertions)]
      pub fn mark_widget_added_delivered(&mut self, widget_id: WidgetId) {
          self.widget_added_delivered.insert(widget_id);
      }

      /// Returns whether WidgetAdded has ever been delivered (debug).
      #[cfg(debug_assertions)]
      pub fn was_widget_added_delivered(&self, widget_id: WidgetId) -> bool {
          self.widget_added_delivered.contains(&widget_id)
      }
  }
  ```

  > **WARNING:** Adding a `#[cfg(debug_assertions)]` field requires updating
  > `new()` to initialize it and `deregister_widget()` to remove from the set.
  > This adds ~10 lines to `manager.rs` (currently 493 lines -- very close to
  > the 500-line limit). If this pushes it over, extract the debug tracking
  > into a helper struct in a submodule.

- [x] In `prepare_widget_frame` (`pipeline/mod.rs`), inside the lifecycle loop, add a debug assertion before `dispatch_lifecycle_to_controllers`:
  ```rust
  #[cfg(debug_assertions)]
  debug_assert!(
      matches!(event, LifecycleEvent::WidgetAdded { .. })
          || interaction.was_widget_added_delivered(id),
      "Widget {:?} received {:?} before WidgetAdded",
      id, event
  );
  ```

- [x] Mark `WidgetAdded` delivery in `prepare_widget_tree` after `prepare_widget_frame` returns. This requires changing `prepare_widget_tree` from `interaction: &InteractionManager` to `interaction: &mut InteractionManager`:

  > **Signature change cascade:** This affects ~6 call sites in `oriterm` and
  > ~4 in tests. The `&mut` is justified because `prepare_widget_tree` is the
  > pre-paint mutation phase -- it already mutates widgets, and marking debug
  > state on the interaction manager is consistent with its mutation role.
  > Explicit mutation is preferred over interior mutability per impl-hygiene.md.

---

## 02.3 Safety Rail Tests

**File(s):**
- `oriterm_ui/src/pipeline/tests.rs` (NEW -- created when converting `pipeline.rs` to directory module; tests for lifecycle assertions and cross-phase consistency helpers)
- `oriterm_ui/src/input/dispatch/tests.rs` (already exists with `plan_propagation` tests; add double-visit dispatch tests here since `dispatch_to_widget_tree` lives in this module)
- `oriterm_ui/src/testing/tests.rs` (already exists with harness integration tests; add harness-based safety rail tests here)

Tests can be written as standalone unit tests using direct calls to `dispatch_to_widget_tree` with mock container widgets (no full harness required), or as harness-based integration tests using the completed `WidgetTestHarness` from Section 01.

**Test isolation:** Cross-phase consistency uses explicit `HashSet` parameters, so no global state leaks between tests. `WidgetId` uses a global atomic counter (IDs are unique across tests), so different test runs never collide. For `#[should_panic]` tests that exercise debug assertions, use `std::panic::catch_unwind` if any cleanup is needed after the expected panic.

- [x] Test double-visit detection in dispatch: create a container whose `for_each_child_mut` yields the same child twice, call `dispatch_to_widget_tree`, assert debug panic fires with message identifying the container and child IDs
- [x] Test double-visit detection in pre-paint: create a container whose `for_each_child_mut` yields the same child twice, call `prepare_widget_tree`, assert debug panic fires
- [x] Test unregistered-widget assertion: deliver `HotChanged` to a widget ID that was never passed to `register_widget()`, assert the registered-widget debug assertion fires
- [x] Test cross-phase false-negative (no false positive): create a container that yields extra children in `for_each_child_mut` beyond those in layout (simulating `PageContainerWidget` behavior), assert NO assertion fires (dispatch superset of layout is valid)
- [x] Test cross-phase mismatch: create a container whose layout includes a child but whose `for_each_child_mut` does NOT yield it, assert cross-phase debug panic fires with the missing widget ID
- [x] Test `WidgetAdded`-first ordering: deliver a lifecycle event (e.g., `HotChanged`) to a widget before any `WidgetAdded` has been delivered, assert the ordering assertion fires
- [x] Test `register_widget` idempotency: call `register_widget` twice for the same widget ID, assert only one `WidgetAdded` event is produced (regression test for the `register_widget` fix)

---

## 02.4 Completion Checklist

- [x] `pipeline.rs` converted to `pipeline/mod.rs` + `pipeline/tests.rs` directory module
- [x] `register_widget()` only pushes `WidgetAdded` on first registration (not every frame)
- [x] `prepare_widget_tree` signature updated to `&mut InteractionManager` (for `WidgetAdded` delivery tracking)
- [x] Debug assertion fires when a container double-visits a child during dispatch
- [x] Debug assertion fires when a container double-visits a child during pre-paint (`prepare_widget_tree`)
- [x] Debug assertion fires when layout children are not a subset of dispatch children (a laid-out widget is unreachable via `for_each_child_mut`)
- [x] Debug assertion fires when a lifecycle event targets an unregistered widget (except `WidgetAdded`)
- [x] Debug assertion fires when a widget receives a lifecycle event before `WidgetAdded`
- [x] All assertion messages include the relevant widget IDs for diagnosis
- [x] Legitimate asymmetry (dispatch visiting more children than layout) does not trigger false positives
- [x] Safety rail tests pass under `timeout 150 cargo test -p oriterm_ui`
- [x] Safety rail tests pass under `timeout 150 cargo test -p oriterm` (app-layer call sites exercise full pipeline)
- [x] No debug assertion failures in existing tests (proves current containers are correct)
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` clean

**Exit Criteria:** A deliberately broken container that double-visits a child causes a debug-mode panic (in both `dispatch_to_widget_tree` and `prepare_widget_tree`). A container whose layout children are not a subset of its dispatch children causes a debug-mode panic. A lifecycle event targeting an unregistered widget (except `WidgetAdded`) causes a debug-mode panic. A widget receiving a lifecycle event before `WidgetAdded` causes a debug-mode panic. `register_widget()` only pushes `WidgetAdded` on first registration. All existing container widgets pass all assertions without modification.
