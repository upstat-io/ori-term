---
section: "02"
title: "Selective Tree Walks"
status: not-started
reviewed: true
goal: "prepare_widget_tree and prepaint_widget_tree skip clean subtrees, reducing per-frame tree walk cost from O(n) to O(dirty)"
inspired_by:
  - "Druid invalidation model (per-widget dirty flags with subtree skip)"
  - "GPUI (Zed) dirty tracking (only relayout/repaint changed elements)"
depends_on: []
sections:
  - id: "02.1"
    title: "Per-Widget Dirty Set"
    status: not-started
  - id: "02.2"
    title: "Dirty Propagation"
    status: not-started
  - id: "02.2b"
    title: "Module Split: pipeline/mod.rs"
    status: not-started
  - id: "02.3"
    title: "Selective Prepare"
    status: not-started
  - id: "02.4"
    title: "Selective Prepaint"
    status: not-started
  - id: "02.5"
    title: "Tests"
    status: not-started
  - id: "02.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Selective Tree Walks

**Status:** Not Started
**Goal:** `prepare_widget_tree` and `prepaint_widget_tree` skip widgets that haven't changed since the last frame. A hover on a single SettingRow should trigger lifecycle/prepaint on only that widget (and its ancestors), not all 37+ widgets.

**Context:** Currently, `prepare_widget_tree()` and `prepaint_widget_tree()` unconditionally recurse into every widget via `for_each_child_mut()`. For a hover event that changes one widget's visual state, both functions walk the entire 37+ widget tree — calling `lifecycle()`, `anim_frame()`, `prepaint()` on every node. Most of these calls are no-ops, but the traversal itself has overhead (virtual dispatch, cache misses, function call overhead).

**Reference implementations:**
- **Druid** `druid/src/core.rs`: `lifecycle()` checks `ctx.is_handled()` and skips children. `update()` uses `ctx.children_changed()` to decide whether to recurse.
- **GPUI (Zed)** `crates/gpui/src/window.rs` + `crates/gpui/src/view.rs`: Elements track `PrepaintStateIndex` ranges and call `reuse_prepaint()` to skip unchanged subtrees.
- **Masonry** `masonry/src/widget/widget.rs`: `StatusChange` enum gates which lifecycle callbacks fire via `on_status_change()`.

**Depends on:** None (independent from Section 01).

**Cross-section note:** If Section 03 is implemented alongside Section 02, coordinate `PrepaintCtx` changes. Section 02 adds `dirty_set` to `prepaint_widget_tree`. Section 03 adds `layout_generation` to `PrepaintCtx`. Both modify the same function signature. Complete Section 02's parameter additions first, then Section 03 adds `layout_generation` on top.

---

## 02.1 Per-Widget Dirty Set

**File(s):** `oriterm_ui/src/invalidation/mod.rs`

The `InvalidationTracker` already has per-widget tracking via `dirty_map: HashMap<WidgetId, DirtyKind>` with `mark(id, kind)`, `is_paint_dirty(id)`, `is_prepaint_dirty(id)`, `is_layout_dirty(id)`, and `full_invalidation: bool` for global rebuild. What is missing is **ancestor propagation** -- the tree walk functions need to know which widgets are on the path to a dirty widget so they can recurse selectively.

- [ ] Add a `should_recurse(id)` method (or equivalent ancestor tracking) to `InvalidationTracker`. The existing `dirty_map` tells us which widgets are dirty but not which ancestors need visiting. Options:
  (a) Build an `ancestors_dirty: HashSet<WidgetId>` lazily from `dirty_map` + parent map (see 02.2)
  (b) Propagate dirty flags upward at `mark()` time (requires parent map reference)

- [ ] Wire `InteractionManager` lifecycle events to mark specific widgets dirty:
  - `HotChanged` → mark the old and new hot widget dirty (Prepaint level)
  - `FocusChanged` → mark the old and new focus widget dirty (Prepaint level)
  - `ActiveChanged` → mark the old and new active widget dirty (Prepaint level)

- [ ] Wire `RenderScheduler` animation frame requests to mark specific widgets dirty:
  - When a widget requests an animation frame → mark it dirty (Prepaint level)

---

## 02.2 Dirty Propagation

**File(s):** `oriterm_ui/src/invalidation/mod.rs`, `oriterm_ui/src/interaction/manager.rs`

When a leaf widget is marked dirty, its ancestors must also be visited during tree walks (to reach the dirty leaf). Use the existing `parent_map` on `InteractionManager` (populated by `set_parent_map()` after each layout pass) to propagate dirty flags up.

- [ ] Add an `ancestors_dirty: HashSet<WidgetId>` field to `InvalidationTracker` to track which widgets are on the path to a dirty widget:
  ```rust
  pub fn propagate_dirty_to_ancestors(&mut self, parent_map: &HashMap<WidgetId, WidgetId>) {
      let dirty_ids: Vec<WidgetId> = self.dirty_map.keys().copied().collect();
      for id in dirty_ids {
          let mut current = id;
          while let Some(&parent) = parent_map.get(&current) {
              if !self.ancestors_dirty.insert(parent) {
                  break; // already marked — ancestors above are also marked
              }
              current = parent;
          }
      }
  }
  ```

- [ ] Add `should_recurse(id)` returning true if `id` is dirty OR is an ancestor of a dirty widget:
  ```rust
  pub fn should_recurse(&self, id: WidgetId) -> bool {
      self.dirty_map.contains_key(&id) || self.ancestors_dirty.contains(&id)
  }
  ```

- [ ] Update `clear()` to also clear `ancestors_dirty`:
  ```rust
  pub fn clear(&mut self) {
      self.dirty_map.clear();
      self.ancestors_dirty.clear();
      self.full_invalidation = false;
  }
  ```

---

## 02.2b Module Split: pipeline/mod.rs

**File(s):** `oriterm_ui/src/pipeline/mod.rs` (437 lines) -> split into submodules

This step MUST be completed before 02.3. `pipeline/mod.rs` is at 437 lines. Adding dirty_set parameters and selective logic will exceed the 500-line hard limit.

- [ ] Create `oriterm_ui/src/pipeline/prepare.rs`:
  - Move `prepare_widget_tree()` and `prepare_widget_frame()` (~130 lines)
  - Re-export from `mod.rs`
- [ ] Create `oriterm_ui/src/pipeline/prepaint.rs`:
  - Move `prepaint_widget_tree()` and `collect_layout_bounds()` (~50 lines)
  - Re-export from `mod.rs`
- [ ] Keep in `mod.rs`: `DispatchResult`, `dispatch_step()`, `register_widget_tree()`, `dispatch_keymap_action()`, `collect_focusable_ids()`, `apply_dispatch_requests()`, debug helpers
- [ ] Update `#[cfg(test)] mod tests;` — tests may need updating for moved functions
- [ ] Verify: `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh`

---

## 02.3 Selective Prepare

**File(s):** `oriterm_ui/src/pipeline/prepare.rs` (after 02.2b split)

Modify `prepare_widget_tree()` to skip clean subtrees. This step requires the 02.2b module split to be complete first.

- [ ] Pass the `InvalidationTracker` (or the dirty set) to `prepare_widget_tree()`. Current signature:
  ```rust
  pub fn prepare_widget_tree(
      widget: &mut dyn Widget,
      interaction: &mut InteractionManager,
      lifecycle_events: &[LifecycleEvent],
      anim_event: Option<&AnimFrameEvent>,
      frame_requests: Option<&FrameRequestFlags>,
      now: Instant,
  ) {
  ```
  Add a new parameter:
  ```rust
      dirty_set: Option<&InvalidationTracker>,  // NEW
  ```

- [ ] Before recursing into children, check if the subtree has any dirty widgets:
  ```rust
  // Skip clean subtrees unless lifecycle events need broadcasting
  if lifecycle_events.is_empty()
      && anim_event.is_none()
      && dirty_set.map_or(false, |ds| !ds.should_recurse(id))
  {
      return; // nothing to do in this subtree
  }
  ```

- [ ] When `full_invalidation` is true, bypass all selective skipping -- walk every widget unconditionally. The dirty set is meaningless during a full rebuild (resize, theme change, DPI change):
  ```rust
  // Full invalidation overrides selective walk — visit everything
  if dirty_set.map_or(false, |ds| ds.needs_full_rebuild()) {
      // Fall through to unconditional walk
  }
  ```

- [ ] Lifecycle events (HotChanged, FocusChanged) target specific widget IDs — only deliver them to the target, not broadcast to all.

---

## 02.4 Selective Prepaint

**File(s):** `oriterm_ui/src/pipeline/prepaint.rs` (after 02.2b split)

Modify `prepaint_widget_tree()` to skip clean widgets.

- [ ] Pass dirty set to `prepaint_widget_tree()`. Current signature:
  ```rust
  pub fn prepaint_widget_tree(
      widget: &mut dyn Widget,
      bounds_map: &HashMap<WidgetId, Rect>,
      interaction: Option<&InteractionManager>,
      theme: &UiTheme,
      now: Instant,
      frame_requests: Option<&FrameRequestFlags>,
  ) {
  ```
  Add a new parameter:
  ```rust
      dirty_set: Option<&InvalidationTracker>,  // NEW
  ```

- [ ] Skip prepaint for clean widgets:
  ```rust
  let id = widget.id();
  if dirty_set.map_or(false, |ds| !ds.is_widget_dirty(id)) {
      // Widget is clean — skip prepaint but still recurse if ancestors are dirty
      if dirty_set.map_or(true, |ds| ds.should_recurse(id)) {
          widget.for_each_child_mut(&mut |child| {
              prepaint_widget_tree(child, bounds_map, interaction, theme, now, frame_requests, dirty_set);
          });
      }
      return;
  }
  // Widget is dirty — call prepaint
  widget.prepaint(&mut ctx);
  // Recurse into children
  widget.for_each_child_mut(&mut |child| {
      prepaint_widget_tree(child, ...);
  });
  ```

---

## 02.5 Tests

**File(s):** `oriterm_ui/src/invalidation/tests.rs`, `oriterm_ui/src/pipeline/tests.rs`

- [ ] Test: `mark_widget_dirty_tracks_id` — mark one widget, verify only it is in the dirty set
- [ ] Test: `propagate_dirty_marks_ancestors` — mark a leaf, verify parent chain is in ancestors_dirty
- [ ] Test: `should_recurse_true_for_dirty_and_ancestors` — verify predicate
- [ ] Test: `should_recurse_false_for_clean_subtree` — clean widget with no dirty descendants → false
- [ ] Test: `selective_prepare_skips_clean_widgets` — mock widget tree, mark one dirty, verify only it receives lifecycle
- [ ] Test: `selective_prepaint_skips_clean_widgets` — same for prepaint

---

## 02.5b Caller Inventory — All Sites That Must Be Updated

**All call sites for `prepare_widget_tree`** (must add `dirty_set` parameter):

1. `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::prepare()` (line ~144)
2. `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::tick_animation()` (line ~174)
3. `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::deliver_lifecycle_events()` (line ~311)
4. `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::prepare_overlay_widgets()` (line ~345)
5. `oriterm/src/app/dialog_rendering.rs` — `App::compose_dialog_widgets()` chrome call (line ~128)
6. `oriterm/src/app/dialog_rendering.rs` — `App::compose_dialog_widgets()` content call (line ~137)
7. `oriterm/src/app/redraw/mod.rs` — tab bar prepare (line ~283)
8. `oriterm/src/app/redraw/multi_pane/mod.rs` — tab bar prepare (line ~363)

**All call sites for `prepaint_widget_tree`** (must add `dirty_set` parameter):

1. `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::run_prepaint()` (line ~327)
2. `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::prepaint_overlay_widgets()` (line ~369)
3. `oriterm/src/app/dialog_rendering.rs` — `App::compose_dialog_widgets()` chrome call (line ~149)
4. `oriterm/src/app/dialog_rendering.rs` — `App::compose_dialog_widgets()` content call (line ~158)
5. `oriterm/src/app/redraw/mod.rs` — tab bar prepaint (line ~299)
6. `oriterm/src/app/redraw/multi_pane/mod.rs` — tab bar prepaint (line ~377)

**Borrow-splitting concern:** `WindowRoot::run_prepaint()` borrows `self.interaction`, `self.frame_requests`, `self.layout`, and `self.widget` -- adding `self.invalidation` to the borrow set requires careful destructuring or a new borrow-splitting helper in `borrow_split.rs`.

- [ ] Add borrow-splitting method for `(&InvalidationTracker, &InteractionManager, &FrameRequestFlags)` to `WindowRoot`
- [ ] Update all 8 `prepare_widget_tree` call sites
- [ ] Update all 6 `prepaint_widget_tree` call sites
- [ ] Overlay widgets: decide whether overlays use the same dirty set or get their own (overlays have independent widget trees -- probably need separate tracking or always full-walk)

---

## 02.6 Completion Checklist

- [ ] `pipeline/mod.rs` split into `prepare.rs` + `prepaint.rs` submodules (each < 500 lines)
- [ ] `InvalidationTracker` has `ancestors_dirty: HashSet<WidgetId>` with `propagate_dirty_to_ancestors()`
- [ ] `ancestors_dirty` is cleared in `InvalidationTracker::clear()`
- [ ] `full_invalidation` bypasses selective walks (unconditional walk)
- [ ] `prepare_widget_tree` skips clean subtrees (verified by test)
- [ ] `prepaint_widget_tree` skips clean widgets but recurses for ancestors (verified by test)
- [ ] Lifecycle events are delivered only to target widgets, not broadcast
- [ ] All 8 `prepare_widget_tree` call sites updated to pass dirty set
- [ ] All 6 `prepaint_widget_tree` call sites updated to pass dirty set
- [ ] Overlay prepare/prepaint paths handled (separate dirty set or full-walk)
- [ ] Borrow-splitting helper added to `WindowRoot` for invalidation access
- [ ] No regressions -- `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** A harness test marks one widget dirty in a 20-widget tree. `prepare_widget_tree` calls `lifecycle()` on only 1 widget (plus ancestors). `prepaint_widget_tree` calls `prepaint()` on only 1 widget. Verified via callback counter.
