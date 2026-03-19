---
section: "04"
title: "Prepaint Phase (3-Pass Rendering)"
status: complete
reviewed: true
goal: "Split the current 2-pass rendering (layout + paint) into 3 passes (layout -> prepaint -> paint) to separate visual state resolution and interaction state queries from actual painting, enabling layout caching independent of paint."
inspired_by:
  - "GPUI Element trait (src/element.rs) — request_layout(), prepaint(), paint() three-pass model"
depends_on:
  - "01.2a (pipeline move — prepaint_widget_tree lives in oriterm_ui/src/pipeline/)"
  - "03 (recommended — Scene's DamageTracker benefits from DirtyKind::Prepaint)"
sections:
  - id: "04.1"
    title: "Widget Trait Prepaint Method"
    status: complete
  - id: "04.2"
    title: "Prepaint Pipeline Integration (library crate)"
    status: complete
  - id: "04.3"
    title: "Prepaint Pipeline Integration (app layer + test harness)"
    status: complete
  - id: "04.4"
    title: "Layout Caching via DirtyKind"
    status: complete
    # Phase gating implemented: lifecycle events + ui_stale merge Prepaint level,
    # cursor-blink frames skip prepare + prepaint entirely.
  - id: "04.5"
    title: "Widget Migration (ButtonWidget)"
    status: complete
  - id: "04.6"
    title: "Tests"
    status: complete
  - id: "04.7"
    title: "Completion Checklist"
    status: complete
---

# Section 04: Prepaint Phase (3-Pass Rendering)

**Status:** Not Started

**Goal:** Add a `prepaint()` phase between layout and paint. Currently, interaction state queries (`is_hot()`, `is_active()`, `is_interaction_focused()`) and visual state reads (`animator.get_bg_color(now)`, etc.) happen during `paint()`. Separating these into prepaint enables: layout results to be cached across frames when only visual state changes (hover color), and visual state resolution to be separated from draw command emission.

**Context:** GPUI's Element trait has three phases with associated type state that flows between them. Our simpler retained-mode architecture does not need the full associated-type machinery, but the phase separation is valuable. When a button changes hover color, we currently relayout + repaint. With prepaint, we skip layout (unchanged) and only run prepaint (resolve visual state) + paint (emit draw commands with resolved state).

**Depends on:** Section 03 (complete) introduced Scene, which replaced DrawList. Scene's DamageTracker benefits from prepaint's `DirtyKind::Prepaint` for finer-grained damage tracking. Section 03 completion is helpful context but not blocking -- prepaint is a pipeline change, not a rendering primitive change. Section 01.2a (pipeline move) is a prerequisite because `prepaint_widget_tree()` lives in `oriterm_ui/src/pipeline/`.

---

## 04.1 Widget Trait Prepaint Method

**File(s):** `oriterm_ui/src/widgets/mod.rs` (currently 261 lines; +5 lines stays under 500), `oriterm_ui/src/widgets/contexts.rs` (currently 266 lines; +~60 lines for PrepaintCtx struct + impl block stays under 500 at ~326)

- [x] Add `prepaint()` method to Widget trait with default no-op body:
  ```rust
  /// Resolves visual states and caches interaction state queries.
  /// Called after layout, before paint.
  fn prepaint(&mut self, _ctx: &mut PrepaintCtx<'_>) {}
  ```
  **Impact:** All 35 existing widget types get the default no-op `prepaint()`.
  No existing code changes required for compilation. Widgets are migrated
  incrementally: move interaction state queries from `paint()` to `prepaint()`
  one widget at a time, validating each migration with the test harness.

- [x] Define `PrepaintCtx` in `contexts.rs`:
  ```rust
  pub struct PrepaintCtx<'a> {
      pub widget_id: WidgetId,
      pub bounds: Rect,
      pub interaction: Option<&'a InteractionManager>,
      pub theme: &'a UiTheme,
      pub now: Instant,
      /// Shared frame request flags for scheduling.
      pub frame_requests: Option<&'a FrameRequestFlags>,
  }
  ```

- [x] Add `PrepaintCtx` convenience methods mirroring `DrawCtx`:
  `is_hot()`, `is_hot_direct()`, `is_active()`, `is_interaction_focused()` -- same
  implementation as `DrawCtx` (delegate to `InteractionManager::get_state`). These
  are the methods widgets call during prepaint to resolve interaction state into
  their `resolved_*` fields. Also add `request_anim_frame()` and `request_paint()`
  delegating to `frame_requests`.

- [x] Add `PrepaintCtx` to re-export in `widgets/mod.rs`:
  Add `PrepaintCtx` to `pub use contexts::{..., PrepaintCtx};` so widgets can
  import it alongside `DrawCtx`, `LayoutCtx`, etc.

---

## 04.2 Prepaint Pipeline Integration (library crate)

**File(s):**
- `oriterm_ui/src/pipeline/mod.rs` (currently 352 lines; +~30 lines for `prepaint_widget_tree()` stays under 500 at ~382)

The current frame pipeline is:
```
event dispatch -> apply requests -> prepare_widget_tree() -> layout -> build_scene() (paint)
```
Where `prepare_widget_tree()` handles lifecycle delivery, animation frame ticks, and
visual state animator updates (state transitions, e.g. Normal -> Hovered). `build_scene()`
calls `widget.paint()` which currently reads animated values (`animator.get_bg_color(now)`)
and interaction state (`ctx.is_hot()`) during painting.

Prepaint inserts between layout and build_scene:
```
event dispatch -> apply requests -> prepare_widget_tree() -> layout -> prepaint_widget_tree() -> build_scene()
```

- [x] Add `prepaint_widget_tree()` function in `oriterm_ui/src/pipeline/mod.rs`. This function walks the tree depth-first and calls `widget.prepaint(&mut PrepaintCtx { ... })` for each widget. It is distinct from `prepare_widget_tree()` which handles lifecycle/animation/state transitions -- `prepaint_widget_tree()` resolves the *output values* of those transitions into concrete fields on each widget. Parameters: `widget: &mut dyn Widget`, `interaction: &InteractionManager` (immutable -- prepaint only reads state, never mutates it), `theme: &UiTheme`, `now: Instant`, `frame_requests: Option<&FrameRequestFlags>`, `layout: &LayoutNode` (to resolve per-widget bounds). The `layout` tree is needed to look up each widget's computed bounds for the `PrepaintCtx::bounds` field.

  > **Complexity warning:** This function must walk the widget tree (via `for_each_child_mut`) and the `LayoutNode` tree in parallel, matching nodes by `widget_id`. This is more complex than `prepare_widget_tree()` (which only walks the widget tree) because the layout tree's child order may not match the widget tree's `for_each_child_mut` order for containers that conditionally include children (e.g., scroll containers with `content_offset`). The implementer must handle: (1) `LayoutNode` children without `widget_id` (anonymous padding/spacing nodes), (2) `widget_id` mismatches between trees, (3) fallback to `Rect::default()` when a widget has no corresponding layout node. Study how `build_scene` (via `DrawCtx::for_child`) solves this -- it receives child bounds explicitly from the container's `paint()` method, NOT from a parallel tree walk. Consider adopting the same approach: have `prepaint_widget_tree` pass `Rect::default()` initially, and have containers override `prepaint()` to call `child.prepaint()` with correct child bounds. This would mirror `build_scene`'s delegation pattern and avoid the parallel walk entirely.

- [x] Resolve per-widget bounds for `PrepaintCtx::bounds`. Implemented approach (c) — flat `HashMap<WidgetId, Rect>` via `collect_layout_bounds()`. Three approaches evaluated:

  **(a) Parallel tree walk** -- walk the layout tree alongside the widget tree in `prepaint_widget_tree`, matching by `widget_id`. Complex (see warning above) and fragile.

  **(b) Container delegation** -- `prepaint_widget_tree` calls `widget.prepaint(ctx)` with the root bounds, and containers override `prepaint()` to call children's `prepaint()` with child-specific bounds (mirroring how `paint()` delegates via `DrawCtx::for_child`). This is the `build_scene` pattern and avoids parallel tree walking.

  **(c) Flat map** -- pre-compute a `HashMap<WidgetId, Rect>` from the `LayoutNode` tree, pass it to `prepaint_widget_tree`. One extra allocation per frame, but simple and correct.

  **Recommendation:** Approach (b) is cleanest (matches existing `build_scene` pattern, no allocation). Approach (c) is acceptable as a simpler fallback. Approach (a) is NOT recommended -- it couples the widget tree's traversal order to the layout tree's structure, which is fragile. If approach (b) is chosen, `prepaint_widget_tree` only needs to call `widget.prepaint(ctx)` at the root, and containers handle recursion internally. This simplifies `prepaint_widget_tree` to ~10 lines.

---

## 04.3 Prepaint Pipeline Integration (app layer + test harness)

**File(s):**
- `oriterm/src/app/widget_pipeline/mod.rs` (27 lines) -- re-export `prepaint_widget_tree` alongside existing re-exports
- `oriterm/src/app/redraw/mod.rs` (404 lines) -- add `prepaint_widget_tree` call after `prepare_widget_tree` for tab bar + overlay widgets
- `oriterm/src/app/redraw/multi_pane.rs` (542 lines -- **already over 500-line limit**)
- `oriterm/src/app/dialog_rendering.rs` (227 lines) -- add `prepaint_widget_tree` calls for chrome + content + overlay widgets
- `oriterm_ui/src/testing/harness_dispatch.rs` (177 lines) -- call `prepaint_widget_tree` in `deliver_lifecycle_events()`, `tick_animation_frame()`, and `run_until_stable()`
- `oriterm_ui/src/testing/harness_inspect.rs` (175 lines) -- call `prepaint_widget_tree` before `widget.paint()` in `render()`

> **Pre-existing hygiene violation in `multi_pane.rs`:** This file is already 542 lines (over the 500-line hard limit) and has inline test bodies (lines 524-542) violating test-organization.md. Before adding `prepaint_widget_tree` calls here, the following must be done first:
> 1. Extract the inline `mod tests { ... }` block to a sibling `tests.rs` file. Multi-pane.rs is a file module (`multi_pane.rs`), so it must become a directory module (`multi_pane/mod.rs` + `multi_pane/tests.rs`) per test-organization.md rules.
> 2. Extract per-pane content extraction into a helper to bring the file under 500 lines.
> Only then add the `prepaint_widget_tree` calls. Do NOT add code to a file that already exceeds the limit.

**This section wires `prepaint_widget_tree` into all call sites. Actual widget migration (moving animator reads from paint to prepaint) is deferred to 04.5.**

- [x] **App layer re-export:** Add `prepaint_widget_tree` to the re-export in
  `oriterm/src/app/widget_pipeline/mod.rs`:
  ```rust
  pub(crate) use oriterm_ui::pipeline::{
      DispatchResult, apply_dispatch_requests, collect_focusable_ids,
      prepare_widget_tree, prepaint_widget_tree, register_widget_tree,
  };
  ```

- [x] **App layer call sites** -- add `prepaint_widget_tree()` calls at every
  location that currently calls `prepare_widget_tree()` followed by `build_scene()`:
  1. `oriterm/src/app/redraw/mod.rs` -- main window: after `prepare_widget_tree(&mut ctx.tab_bar, ...)`,
     add `prepaint_widget_tree(&mut ctx.tab_bar, ...)`. After overlay `prepare_widget_tree` loop,
     add matching `prepaint_widget_tree` loop via `ctx.overlays.for_each_widget_mut(...)`.
  2. `oriterm/src/app/redraw/multi_pane.rs` -- multi-pane path: same pattern.
     **Prerequisite:** multi_pane.rs must be under 500 lines before adding code (see warning above).
  3. `oriterm/src/app/dialog_rendering.rs` -- dialog windows: after `prepare_widget_tree(&mut ctx.chrome, ...)`
     and `prepare_widget_tree(ctx.content.content_widget_mut(), ...)`, add corresponding
     `prepaint_widget_tree` calls for both chrome and content widgets. Overlay prepaint
     in `render_dialog_overlays` follows the same pattern.

  > **Implementation note:** The `prepaint_widget_tree` call requires layout bounds. At the app layer call sites, the tab bar layout is computed by `draw_tab_bar` internally (it creates `bounds` from `logical_width` and `TAB_BAR_HEIGHT`). For `prepaint_widget_tree` to have the correct bounds, it must be called with the same bounds value. The `LayoutNode` tree is available in `self.layout` (harness) or computed per-frame in the app. If approach (b) from 04.2 is chosen (container delegation), the app layer only passes root bounds and the containers recurse, simplifying this.

- [x] **Overlay prepaint integration:** `OverlayManager::for_each_widget_mut()` is
  currently used by the app layer to call `prepare_widget_tree` on each overlay's root
  widget (see `oriterm/src/app/redraw/mod.rs:275`). The same pattern applies for
  `prepaint_widget_tree`: after calling `prepaint_widget_tree` on the main widget tree,
  the app layer must iterate overlay widgets via `for_each_widget_mut` and call
  `prepaint_widget_tree` on each. Note: overlay widgets currently paint with
  `interaction: None` in `OverlayManager::draw_overlay_at()` -- prepaint should pass
  the real `InteractionManager` so overlay widgets can resolve hover/focus state properly.
  This fixes a latent correctness issue: overlay widgets already receive lifecycle events
  from `prepare_widget_tree`, so they should also get prepaint with the real interaction state.

- [x] **Verify Scene is unchanged:** `build_scene()` and `Scene` itself do NOT change.
  `build_scene()` calls `widget.paint()`, which after migration reads pre-resolved
  fields (`self.resolved_bg`, `self.resolved_focused`, etc.) instead of calling
  `ctx.is_hot()` or `animator.get_bg_color(now)`. The Scene collects the same
  typed primitives as before -- only the source of color/state values changes
  (widget fields vs. real-time queries). `DamageTracker` benefits indirectly:
  when only prepaint-level state changes (hover), the damage diff shows
  changed widget hashes only for widgets whose visual output actually changed,
  which is already the current behavior.

- [x] **Test harness integration (`WidgetTestHarness`):**
  The harness must call `prepaint_widget_tree()` so that widget `prepaint()` methods
  run during tests, matching the production pipeline. Four integration points:

  1. **`harness_dispatch.rs::deliver_lifecycle_events()`** -- after `prepare_widget_tree()`,
     add `prepaint_widget_tree()`. This ensures that after any lifecycle event delivery
     (HotChanged, ActiveChanged, FocusChanged), widgets resolve their visual state into
     fields before the test inspects them or the harness renders.

  2. **`harness_dispatch.rs::tick_animation_frame()`** -- after `prepare_widget_tree()`,
     add `prepaint_widget_tree()`. Same reasoning: animation frame ticks update
     `VisualStateAnimator` state, and prepaint resolves the output values.

  3. **`harness_dispatch.rs::run_until_stable()`** -- after `prepare_widget_tree()` in the loop,
     add `prepaint_widget_tree()`.

  4. **`harness_inspect.rs::render()`** -- before `self.widget.paint(&mut ctx)`, add
     `prepaint_widget_tree(&mut *self.widget, ...)`. This ensures that `render()` always
     returns a Scene with correctly-resolved visual state, even if the test has not
     explicitly run lifecycle delivery since the last state change. Without this,
     `render()` would produce stale colors for widgets that migrated to prepaint.

  > **Implementation note:** `harness_inspect.rs::render()` already has access to `self.layout` (the `LayoutNode` tree). Pass `&self.layout` to `prepaint_widget_tree`. The harness re-lays out on demand, so layout is always fresh.

---

## 04.4 Layout Caching via DirtyKind

- [x] Extend `DirtyKind` enum in `oriterm_ui/src/invalidation/mod.rs` (currently 130 lines; changes add ~30 lines, stays well under 500). Currently has only `Clean` and `Layout`. Add `Paint` and `Prepaint` variants in ascending severity order (required for `#[derive(PartialOrd, Ord)]` to produce correct ordering):
  ```rust
  #[derive(PartialEq, Eq, PartialOrd, Ord)]
  pub enum DirtyKind {
      Clean,     // No changes
      Paint,     // Visual change only (cursor blink) -- skip layout + prepaint
      Prepaint,  // Interaction state change (hover) -- skip layout, run prepaint + paint
      Layout,    // Structural change -- run all three phases
  }
  ```
  The existing `merge()` method and `InvalidationTracker` must be updated to handle the new variants. `merge()` returns the higher-severity kind (`Layout > Prepaint > Paint > Clean`).

- [x] Update `DirtyKind::merge()` to handle 4-way merge:
  ```rust
  pub fn merge(self, other: Self) -> Self {
      match (self, other) {
          (Self::Layout, _) | (_, Self::Layout) => Self::Layout,
          (Self::Prepaint, _) | (_, Self::Prepaint) => Self::Prepaint,
          (Self::Paint, _) | (_, Self::Paint) => Self::Paint,
          _ => Self::Clean,
      }
  }
  ```

- [x] Verify `DirtyKind::is_dirty()` still correct -- unchanged (`!matches!(self, Self::Clean)`).

- [x] Update `From<ControllerRequests> for DirtyKind` -- currently returns `Clean` for all
  requests. After this change: `PAINT -> Paint`, everything else -> `Clean`.

  > **Test impact:** The existing test `controller_requests_paint_maps_to_clean` in
  > `oriterm_ui/src/invalidation/tests.rs` (line 53) asserts `PAINT -> Clean`.
  > Update to assert `PAINT -> Paint`. Similarly, `controller_requests_paint_combined_is_clean`
  > (line 69) must be updated.

- [x] Replace `InvalidationTracker` internal storage: change `HashSet<WidgetId>` (`layout_dirty`)
  to `HashMap<WidgetId, DirtyKind>` storing the highest dirty level per widget.
  Add `is_prepaint_dirty(id)` and `is_paint_dirty(id)` query methods.
  Update `is_any_dirty()` to check the new map. Update `clear()` to clear the map.

  > **API note:** `is_layout_dirty(id)` is called by existing code. It must continue to work
  > (return `true` when `dirty_map[id] == Layout`). The new query methods are additive.

- [x] Add `max_dirty_kind()` method to `InvalidationTracker` that returns the highest
  `DirtyKind` across all tracked widgets (used by the app layer to decide which phases to run).

- [x] **Phase gating in the app layer:** The call sites in `oriterm/src/app/redraw/mod.rs`
  and `oriterm/src/app/dialog_rendering.rs` must check `InvalidationTracker` to decide
  which phases to run:
  ```
  let dirty = invalidation.max_dirty_kind();
  if dirty >= DirtyKind::Layout   { run_layout(); }
  if dirty >= DirtyKind::Prepaint { run_prepaint(); }
  if dirty >= DirtyKind::Paint    { run_build_scene(); }
  ```
  This uses the `Ord` impl on `DirtyKind`. When `dirty == Prepaint`, layout is skipped
  but prepaint and paint both run. When `dirty == Paint`, only `build_scene` runs
  (cursor blink, no interaction state change).

  > **Correctness invariant:** Phase gating is only safe if prepaint never changes inputs
  > to the layout solver (text content, child count, min/max sizes). If a widget's
  > `prepaint()` modifies state that affects `layout()`, the cached layout will be stale.
  > Add a debug assertion: when `dirty < Layout`, assert that the layout tree hash is
  > unchanged after prepaint. Remove the assertion once phase gating is validated.

---

## 04.5 Widget Migration (ButtonWidget)

**File(s):**
- `oriterm_ui/src/widgets/button/mod.rs` -- add `resolved_bg`, `resolved_border_color`, `resolved_focused` fields and `prepaint()` impl
- `oriterm_ui/src/widgets/button/tests.rs` -- add prepaint migration test

> **Dead code strategy:** The project uses `dead_code = "deny"`. Adding `resolved_bg: Color`
> to `ButtonWidget` before `paint()` reads it will cause a compilation error. The migration
> must be atomic per widget: in a single commit, add the `resolved_*` field, write to it
> in `prepaint()`, AND read from it in `paint()` (removing the old `animator.get_bg_color(now)`
> call). Do NOT add `resolved_*` fields to multiple widgets in advance -- each widget is
> migrated individually in one atomic step.

- [x] Add `resolved_bg`, `resolved_focused` fields to `ButtonWidget` and implement `prepaint()` to populate them (border_color is static from style, not animated — no resolved field needed):
  Currently, `ButtonWidget::paint()` calls `animator.get_bg_color(now)`, `animator.get_border_color(now)`,
  and checks `ctx.is_interaction_focused()`.
  After this change, `ButtonWidget::prepaint()` resolves all animated properties and interaction
  state, storing results in `self.resolved_bg`, `self.resolved_border_color`, `self.resolved_focused`.
  `ButtonWidget::paint()` then reads these pre-resolved values without calling the animator
  or `ctx.is_interaction_focused()`. This must be done atomically (field + prepaint write + paint read)
  in a single commit to avoid dead code errors.

- [x] Move interaction state queries (`is_interaction_focused()`) from `paint()` into `prepaint()` for `ButtonWidget`:
  `PrepaintCtx` queries interaction state; results are stored on the widget struct.

  **Migration strategy:** Keep `DrawCtx.interaction` during migration. Remove it only after
  ALL 7 widgets that call `ctx.is_hot()` etc. during `paint()` are converted to read from
  `prepaint()` results. The 7 widgets: button, checkbox, dropdown, number_input, slider,
  text_input, and toggle. (The Widget trait doc comment in `mod.rs` mentions `ctx.is_hot()`
  but that is documentation, not a call site.) Only `ButtonWidget` is migrated in this
  section as proof of concept. Remaining 6 widgets are migrated in follow-up work.

- [ ] **Remaining widget migration scope (follow-up, not this section):**
  14 widgets implement `visual_states()` and use VisualStateAnimator
  (button, checkbox, color_swatch, cursor_picker, dropdown, keybind, number_input,
  scheme_card, setting_row, slider, text_input, toggle, window_chrome/controls,
  settings_panel/id_override_button). Plus `tab_bar/widget/control_state.rs` uses
  VisualStateAnimator directly without the trait method. Each migration is ~20 lines
  (add `resolved_*` fields, implement `prepaint()`, update `paint()`).
  Total: ~300 lines across ~15 files. Widgets are migrated one at a time, each in
  an atomic commit.

- [ ] **Tab bar control_state bypass (follow-up, not this section):** `tab_bar/widget/control_state.rs`
  drives `VisualStateAnimator` directly via `update_control_hover_state()` and
  `clear_control_hover_state()`, bypassing the normal pipeline. When `WindowControlButton`
  is eventually migrated to prepaint, the control_state bypass must either:
  (a) call `prepaint()` on each control button after driving the animator, or
  (b) be refactored to use the normal `InteractionManager` hot path so that
  `prepaint_widget_tree` handles them naturally. Option (b) is preferred (cleaner,
  no special case). This is only needed when `WindowControlButton` is migrated,
  which is NOT in this section.

---

## 04.6 Tests

**File(s):**
- `oriterm_ui/src/pipeline/tests.rs` -- unit tests for `prepaint_widget_tree()`
- `oriterm_ui/src/invalidation/tests.rs` -- unit tests for new `DirtyKind` variants and `InvalidationTracker` changes
- `oriterm_ui/src/widgets/button/tests.rs` -- verify ButtonWidget prepaint migration
- New integration test using `WidgetTestHarness` (in an appropriate test file)

- [x] **Update existing `DirtyKind` tests** in `oriterm_ui/src/invalidation/tests.rs`:
  - Rename `controller_requests_paint_maps_to_clean` to `controller_requests_paint_maps_to_paint` and assert `DirtyKind::Paint`.
  - Update `controller_requests_paint_combined_is_clean` assertion to `DirtyKind::Paint` (PAINT is the highest flag in the combined set).
  - Verify all existing `merge()` tests still pass (Clean+Layout, Layout+Clean combinations are unchanged).

- [x] **`DirtyKind` merge and ordering tests:** Verify `merge()` with all 16 combinations of 4 variants.
  Verify `Ord` impl: `Clean < Paint < Prepaint < Layout`.

- [x] **`InvalidationTracker` tests:** Verify `mark(id, Prepaint)` records prepaint-level dirty.
  Verify `is_prepaint_dirty(id)` and `is_paint_dirty(id)` query methods. Verify merge
  promotion: marking `Paint` then `Prepaint` on same widget results in `Prepaint`.

- [x] **`From<ControllerRequests>` test:** Verify `PAINT` flag maps to `DirtyKind::Paint`.

- [x] **`prepaint_widget_tree` unit test:** Create a test widget that sets a boolean flag
  in `prepaint()`. Run `prepaint_widget_tree` and assert the flag is set. Verify that
  `PrepaintCtx` has correct `widget_id` and `bounds`.

- [x] **`prepaint_widget_tree` child traversal test:** Create a container with 2+ children.
  Run `prepaint_widget_tree`. Assert all children received `prepaint()` calls with
  correct per-child bounds.

- [x] **Phase count test (exit criteria):** Use `WidgetTestHarness` with a ButtonWidget.
  Instrument or wrap to count layout/prepaint/paint invocations. Simulate hover
  (mouse_move into button bounds). Assert: layout NOT called, prepaint called, paint called.
  This is the core exit criteria test.

  > **Complexity note:** This test requires the harness to use `InvalidationTracker` for
  > phase gating. If the harness does not yet integrate invalidation tracking (it currently
  > calls all phases unconditionally), this test must either: (a) add basic invalidation
  > tracking to the harness, or (b) be implemented as an integration test that runs the
  > full app pipeline. Option (a) is preferred but adds scope.

- [x] **Harness `render()` correctness test:** Create a ButtonWidget via `WidgetTestHarness`.
  Simulate hover. Call `render()`. Verify that the Scene contains a quad with the
  hover background color (proving prepaint resolved the color before paint used it).

- [x] **ButtonWidget `resolved_bg` test:** After `prepaint()`, verify that
  `ButtonWidget.resolved_bg` matches the expected animated color value for the
  current interaction state (e.g., hovered color when hot, default color when not).

---

## 04.7 Completion Checklist

**04.1 -- Widget Trait:**
- [x] `prepaint()` method added to Widget trait with default no-op body
- [x] `PrepaintCtx` struct defined in `contexts.rs` with fields: `widget_id`, `bounds`, `interaction`, `theme`, `now`, `frame_requests`
- [x] `PrepaintCtx` convenience methods implemented: `is_hot()`, `is_hot_direct()`, `is_active()`, `is_interaction_focused()`, `request_anim_frame()`, `request_paint()`
- [x] `PrepaintCtx` re-exported from `widgets/mod.rs`

**04.2 -- Library crate pipeline:**
- [x] `prepaint_widget_tree()` function added to `pipeline/mod.rs`, walks tree depth-first calling `widget.prepaint()`
- [x] `prepaint_widget_tree()` resolves per-widget bounds (via approach (c) flat map — `collect_layout_bounds`)

**04.3 -- App layer + test harness:**
- [x] `multi_pane.rs` hygiene resolved: converted to directory module, tests in sibling `tests.rs`, under 500 lines (494)
- [x] App layer re-export added in `widget_pipeline/mod.rs`
- [x] App layer call sites updated: `redraw/mod.rs`, `redraw/multi_pane/mod.rs`, `dialog_rendering.rs`
- [x] Overlay widgets receive `prepaint_widget_tree` calls via `for_each_widget_mut` with real `InteractionManager`
- [x] `build_scene()` and Scene unchanged -- only the source of color/state values changes in migrated widgets
- [x] Test harness updated at all 4 integration points: `deliver_lifecycle_events()`, `tick_animation_frame()`, `run_until_stable()`, `render()`

**04.4 -- Layout caching:**
- [x] `DirtyKind::Paint` and `DirtyKind::Prepaint` variants added (enum declared in ascending severity order for `derive(Ord)`)
- [x] `DirtyKind::merge()` updated for 4-way merge
- [x] `From<ControllerRequests>` updated: `PAINT` maps to `DirtyKind::Paint`
- [x] Existing invalidation tests updated for new `From<ControllerRequests>` behavior
- [x] `InvalidationTracker` storage changed from `HashSet` to `HashMap<WidgetId, DirtyKind>`
- [x] `max_dirty_kind()` method added to `InvalidationTracker`
- [x] Phase gating implemented: hover skips layout (`DirtyKind::Prepaint`), cursor blink skips layout + prepaint (`DirtyKind::Paint`)

**04.5 -- ButtonWidget migration:**
- [x] `ButtonWidget` migrated atomically: `resolved_bg`, `resolved_focused` fields + `prepaint()` + updated `paint()` (border_color is static from style — no resolved field needed)
- [x] `DrawCtx.interaction` retained during migration period (removed only after all 7 widgets converted)

**04.6 -- Tests:**
- [x] Existing `DirtyKind` tests updated for new variants
- [x] `DirtyKind` merge tests cover all 16 combinations; `Ord` tests verify `Clean < Paint < Prepaint < Layout`
- [x] `InvalidationTracker` tests cover `mark()`, `is_prepaint_dirty()`, `is_paint_dirty()`, merge promotion
- [x] `prepaint_widget_tree` unit tests: single widget + container with children
- [x] Phase count test passes: hover triggers prepaint+paint but NOT layout
- [x] Harness `render()` correctness test: hover produces correct background color in Scene
- [x] `ButtonWidget.resolved_bg` test: prepaint populates correct animated color

**Build gates:**
- [x] No regressions in `./test-all.sh`
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` clean

**Exit Criteria:** A hover state change (mouse enters button) triggers only prepaint + paint, not layout. Measured via a test that counts phase invocations. At least ButtonWidget demonstrates the full prepaint pattern (resolved fields populated in prepaint, read in paint).
