---
section: "03"
title: "Dialog Selective Walks"
status: in-progress
reviewed: true
third_party_review:
  status: findings
  updated: 2026-03-21
goal: "Dialog prepare/prepaint/paint phases skip clean subtrees so that hover and page-local interactions cost proportional to the changed subtree, not the whole dialog tree"
inspired_by:
  - "InvalidationTracker per-widget dirty state (oriterm_ui/src/invalidation/mod.rs)"
  - "DamageTracker per-widget hash diff (oriterm_ui/src/draw/damage/mod.rs:56-104)"
depends_on: ["01", "02"]
sections:
  - id: "03.0"
    title: "Extract Tree-Walk Functions to Submodule"
    status: complete
  - id: "03.1"
    title: "Per-Widget Dirty Tracking in Prepare"
    status: not-started
  - id: "03.2"
    title: "Per-Widget Dirty Tracking in Prepaint"
    status: not-started
  - id: "03.3"
    title: "Selective Paint via Damage Regions"
    status: not-started
  - id: "03.4"
    title: "Tests"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: in-progress
  - id: "03.5"
    title: "Build & Verify"
    status: not-started
---

# Section 03: Dialog Selective Walks

**Status:** Not Started
**Goal:** Dialog prepare/prepaint/paint phases skip clean subtrees. A single-widget hover in the dialog stops traversing the entire content tree. Work scales with the size of the changed subtree, not the whole dialog.

**Production code path:** `compose_dialog_widgets()` in `dialog_rendering.rs` — the `prepare_widget_tree()`, `prepaint_widget_tree()`, and paint calls that currently walk the entire chrome and content widget trees unconditionally.

**Observable change:** When hovering over a single button in the settings dialog, the prepare/prepaint phases visit only the ancestors of the hovered widget plus the widget itself, not every widget in every section of every page. Measurable via a tree-walk counter or `log::debug!` widget visit counts.

**Context:** Currently, `InvalidationTracker::max_dirty_kind()` returns the highest `DirtyKind` across all widgets. If one widget is `Prepaint`-dirty (hover), the entire tree gets `prepare_widget_tree()` + `prepaint_widget_tree()`. The `InvalidationTracker` has per-widget query methods (`is_prepaint_dirty(id)`, `is_paint_dirty(id)`, etc.) and a `mark(id, DirtyKind)` method, but **`mark()` is never called from production code** — only from tests. The `dirty_map` is always empty in production; only `full_invalidation` (set by `invalidate_all()`) and the coarse `max_dirty_kind()` are exercised. Before selective walks can work, something must populate per-widget dirty state — likely the interaction/lifecycle pipeline (hover state changes, focus transitions).

**Reference implementations:**
- **InvalidationTracker** `oriterm_ui/src/invalidation/mod.rs`: Has per-widget `DirtyKind` infrastructure (`mark()`, `is_prepaint_dirty()`, etc.) but `mark()` is never called from production code — the `dirty_map` is always empty in production. Per-widget dirty state must be populated before selective walks can use it
- **DamageTracker** `oriterm_ui/src/draw/damage/mod.rs:56-104`: Per-widget hash comparison identifies which widgets' paint output changed — could gate selective paint

**Depends on:** Section 01 (correct bounds), Section 02 (viewport culling hardened — selective walks build on culling).

**Sync points — signature changes propagate to ALL callers:**
If `prepare_widget_tree()` or `prepaint_widget_tree()` signatures change (e.g., adding `Option<&InvalidationTracker>`), ALL of these call sites must be updated simultaneously:
- `oriterm/src/app/dialog_rendering.rs` — `compose_dialog_widgets()` (2 calls each for prepare + prepaint)
- `oriterm/src/app/redraw/mod.rs` — `handle_redraw()` (1 call each)
- `oriterm/src/app/redraw/multi_pane/mod.rs` — `handle_redraw_multi_pane()` (1 call each)
- `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::prepare()` (line 141) and `run_prepaint()` (line 324) (used by `WidgetTestHarness`)
- `oriterm_ui/src/window_root/pipeline.rs` — `prepare_overlay_widgets()` (line 341, calls `prepare_widget_tree` per overlay) and `prepaint_overlay_widgets()` (line 360, calls `prepaint_widget_tree` per overlay)
- `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::deliver_lifecycle_events()` (line 301, private) calls `prepare_widget_tree` — must be included in signature sync
- `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::run_prepaint()` (line 324, private) wraps `prepaint_widget_tree` — if `prepaint_widget_tree`'s signature changes, `run_prepaint` must be updated to forward the new parameter
- `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::tick_animation()` (line 164) also calls `prepare_widget_tree` — must be included in signature sync
- `oriterm/src/app/widget_pipeline/mod.rs` — re-export list must include any new dependencies
- `oriterm/src/app/widget_pipeline/tests.rs` — test calls

**Sync point — `apply_dispatch_requests` in `pipeline/mod.rs`:** This function calls `interaction.set_active()`, `interaction.clear_active()`, and `interaction.request_focus()`. If those methods change to return changed widget IDs (see 03.1), `apply_dispatch_requests` must forward those IDs to the `InvalidationTracker`. Either pass `&mut InvalidationTracker` as an additional parameter, or return a `Vec<WidgetId>` from `apply_dispatch_requests` for the caller to mark dirty. This is called from `WindowRoot::dispatch_event()` (step 5) which already has access to `self.invalidation`.

> **BUG — Windows modal loop `clear()` timing.** `event_loop_helpers/mod.rs:85` calls `ctx.root.invalidation_mut().clear()` BEFORE `self.handle_redraw()` during the Win32 modal move/resize loop. The normal render path in `render_dispatch.rs:41,64` correctly calls `clear()` AFTER `render_dialog()`/`handle_redraw()`. Once selective walks consume dirty state during render, the modal loop's early `clear()` will wipe the dirty map before the selective walk can use it. **Fix required in 03.1:** Move the `clear()` call in `modal_loop_render()` to AFTER `self.handle_redraw()`, matching the pattern in `render_dirty_windows()`.

---

## 03.0 Extract Tree-Walk Functions to Submodule

**File(s):** `oriterm_ui/src/pipeline/mod.rs` (441 lines) -> `oriterm_ui/src/pipeline/mod.rs` (orchestration, ~120 lines) + `oriterm_ui/src/pipeline/tree_walk.rs` (traversal, ~320 lines)

> **MANDATORY PRE-STEP.** `pipeline/mod.rs` is currently 441 lines. Adding subtree-skip logic to `prepare_widget_tree` and `prepaint_widget_tree` in subsequent subsections WILL push it over the 500-line hard limit. This extraction MUST happen before any 03.1 work begins. It is a pure refactor with zero behavioral change.

**What moves to `pipeline/tree_walk.rs`:**
- `prepare_widget_tree()` (lines 117-157)
- `prepare_widget_frame()` (lines 173-249)
- `prepaint_widget_tree()` (lines 283-306)
- `register_widget_tree()` (lines 317-322)
- `dispatch_keymap_action()` (lines 331-347)
- `collect_focusable_ids()` (lines 354-361)

**What stays in `pipeline/mod.rs`:**
- `DispatchResult` struct + impl (lines 37-69)
- `dispatch_step()` (lines 89-106)
- `collect_layout_bounds()` (lines 260-267)
- `apply_dispatch_requests()` (lines 372-403)
- `collect_layout_widget_ids()` (debug-only, lines 411-418)
- `check_cross_phase_consistency()` (debug-only, lines 426-438)
- Re-exports from `tree_walk` via `pub use tree_walk::*;` or selective re-exports

**Test impact:** Existing `pipeline/tests.rs` tests import from `super::`. Since `tree_walk` is a submodule of `pipeline`, the `super::` imports in `pipeline/tests.rs` still resolve. However, if `tree_walk` needs its own dedicated tests later, those go in `pipeline/tree_walk/tests.rs` (converting `tree_walk.rs` to `tree_walk/mod.rs` + `tree_walk/tests.rs`). For now, keep all pipeline tests in `pipeline/tests.rs` since the re-exports make everything accessible.

- [x] Move the 6 functions listed above into `pipeline/tree_walk.rs`
- [x] Add `mod tree_walk;` to `pipeline/mod.rs` and re-export all moved functions
- [x] Verify `pipeline/mod.rs` is under 300 lines (207) and `tree_walk.rs` is under 400 lines (261)
- [x] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` pass — zero behavioral change
- [x] All callers (sync points listed above) continue to compile without import changes (re-exports preserve the public API)
- [x] Gate: `git diff` shows only structural moves, no logic changes

---

## 03.1 Per-Widget Dirty Tracking in Prepare

**File(s):** `oriterm_ui/src/pipeline/tree_walk.rs` (extracted in 03.0), `oriterm_ui/src/invalidation/mod.rs`, `oriterm_ui/src/interaction/manager.rs`, `oriterm_ui/src/window_root/pipeline.rs`, `oriterm/src/app/event_loop_helpers/mod.rs`

> **WARNING — NO DEAD CODE.** This subsection must wire `mark()` into production AND modify `prepare_widget_tree()` to consume the dirty state, both in the same implementation pass. Do NOT land the `mark()` wiring without the selective walk consumer, and do NOT land the selective walk without the `mark()` wiring. Both together or neither.

**Critical prerequisite:** `InvalidationTracker::mark()` exists but is **never called from any production code path**. The `dirty_map` is always empty in production — only `full_invalidation` (via `invalidate_all()`) is used. Before any selective walk optimization can work, per-widget dirty marking must be wired into the interaction/lifecycle pipeline. This is the **central task** of this subsection.

The `prepare_widget_tree()` function walks the entire tree via `for_each_child_mut`. To make it selective, it needs to skip subtrees where no widget is dirty.

### Step 1: Extend `InvalidationTracker` with subtree-dirty tracking (library crate first)

- [x] Add `dirty_ancestors: HashSet<WidgetId>` field to `InvalidationTracker`
- [x] Change `mark()` signature to `mark(&mut self, id: WidgetId, kind: DirtyKind, parent_map: &HashMap<WidgetId, WidgetId>)`. Walk `parent_map` upward from `id`, insert all ancestors into `dirty_ancestors`. 12 existing tests updated to pass `&no_parents()` helper
- [x] Add `pub fn has_dirty_descendant(&self, id: WidgetId) -> bool` — checks `self.dirty_ancestors.contains(&id) || self.dirty_map.contains_key(&id)`
- [x] Update `clear()` to also clear `dirty_ancestors`
- [x] Verify `invalidation/mod.rs` stays under 500 lines (217 lines). 5 new tests added: ancestor propagation, self-dirty, clean state, clear resets ancestors, early-stop on duplicate

### Step 2: Wire `mark()` into interaction state transitions

- [ ] **Modify `InteractionManager` state-change methods to return changed widget IDs.** When `InteractionManager` changes a widget's hot/active/focused state, the caller needs to know which widgets changed so it can call `InvalidationTracker::mark()`. **Concrete return type changes:**
  - `update_hot_path()` (`manager.rs:122`): currently returns `bool`. Change to return `Vec<WidgetId>` containing all widgets whose hot state changed (both newly-hot and newly-not-hot). The hot path typically has 3-5 widgets. (`smallvec` is in `Cargo.toml` but unused — either `SmallVec<[WidgetId; 8]>` or plain `Vec` works; prefer `Vec` for simplicity unless profiling shows allocation pressure)
  - `set_active()` (`manager.rs:168`): currently returns nothing. Change to return `Vec<WidgetId>` (previous active + new active)
  - `clear_active()` (`manager.rs:197`): currently returns nothing. Change to return `Option<WidgetId>` (the previously-active widget, if any)
  - `request_focus()` (`manager.rs:215`): currently returns nothing. Change to return `Vec<WidgetId>` (old focused, new focused, plus ancestors whose `focus_within` changed)
  - `clear_focus()` (`manager.rs:253`): currently returns nothing. Change to return `Vec<WidgetId>` (old focused + ancestors)
  - `set_disabled()` (`manager.rs:274`): currently returns nothing. Change to return `Option<WidgetId>` (the widget if its disabled state actually changed)
  - `deregister_widget()` (`manager.rs:73`): currently returns nothing. Change to return `Vec<WidgetId>` (widget itself + any cleared hot/active/focus state)
- [ ] **File size constraint:** `manager.rs` is currently 406 lines. Return-type changes add ~2-3 lines per method (collect into SmallVec). The total increase is ~15-20 lines, staying well under 500. No extraction needed

- [ ] **Module boundary discipline:** `InteractionManager` (in `interaction/`) must NOT depend on `InvalidationTracker` (in `invalidation/`). The recommended approach — returning changed IDs — keeps the two modules independent. The caller (`WindowRoot`) bridges them

### Step 3: Wire dirty marking into `WindowRoot` and callers

- [ ] **Update `WindowRoot::dispatch_event()`** (`window_root/pipeline.rs`). At step 1 (hot path update, line 88), take the returned changed IDs from `update_hot_path()` and call `self.invalidation.mark(id, DirtyKind::Prepaint, &self.interaction.parent_map_ref())` for each. Note: `InteractionManager` currently stores `parent_map` as private. Add a `pub fn parent_map_ref(&self) -> &HashMap<WidgetId, WidgetId>` accessor (1 line, no file size concern)

- [ ] **Update `apply_dispatch_requests()`** (`pipeline/mod.rs` or `pipeline/tree_walk.rs`). This function calls `interaction.set_active()`, `clear_active()`, `request_focus()`. It must either (a) accept `&mut InvalidationTracker` + parent map and mark dirty inline, or (b) return the changed widget IDs so the caller can mark. Option (b) is cleaner — change `apply_dispatch_requests` to return a `Vec<WidgetId>` of changed widgets. `WindowRoot::dispatch_event()` step 5 then marks them dirty
- [ ] **Update `WindowRoot::deliver_lifecycle_events()`** (`window_root/pipeline.rs:301`). This drains events and calls `prepare_widget_tree`. After the hot path update in `dispatch_event()` (which generates lifecycle events), the changed IDs should already be marked. No additional marking needed here, but verify this is the case
- [ ] **Wire into dialog path.** `compose_dialog_widgets()` (`dialog_rendering.rs`) drains lifecycle events at line 116. The dialog path's interaction state changes happen in `event_handling/mod.rs:290` (`update_hot_path`) — which runs BEFORE `compose_dialog_widgets`. Verify that the dialog event handling path marks dirty IDs before render begins

- [ ] **Fix Windows modal loop `clear()` timing.** In `event_loop_helpers/mod.rs:85`, move `ctx.root.invalidation_mut().clear()` to AFTER `self.handle_redraw()`, matching the correct pattern in `render_dispatch.rs:41,64`. Current code clears dirty state before the render, which would wipe the `dirty_map` before selective walks can use it. This is a correctness bug exposed by selective walks

### Step 4: Wire `mark()` into animator ticks

- [ ] **Wire `mark()` into `VisualStateAnimator` ticks.** When an animator is actively interpolating (returning `is_animating() == true`), its widget needs `DirtyKind::Prepaint` on subsequent frames until the animation completes. Currently, `prepare_widget_frame()` (`tree_walk.rs`, moved from `pipeline/mod.rs:173-249`) handles this — when `animator.is_animating(now)` is true (line 243), it calls `flags.request_anim_frame()` (line 245). At that same point, also mark the widget dirty. **Important clarification:** This mark is for the NEXT frame's dirty state, not the current frame. The `InvalidationTracker` must NOT be cleared until after ALL walks (prepare + prepaint) complete in the current frame. The mark persists into the next frame's `dirty_map`. Since `render_dispatch.rs` calls `clear()` after `render_dialog()` / `handle_redraw()`, and `compose_dialog_widgets` runs inside those functions, the timing is correct: marks made during prepare survive through prepaint within the same frame, then `clear()` resets for the next frame

- [ ] **Threading `InvalidationTracker` into `prepare_widget_frame`:** The function currently takes `&mut InteractionManager` but not `&mut InvalidationTracker`. To mark animating widgets, it needs the tracker. Two approaches: (a) pass `&mut InvalidationTracker` as an additional parameter to `prepare_widget_tree` and `prepare_widget_frame`, or (b) collect animating widget IDs into a return value and mark in the caller. Option (a) is simpler since `prepare_widget_tree` already needs the tracker for selective walk decisions. Use `Option<&mut InvalidationTracker>` to maintain backward compatibility with callers that don't need selective walks

### Step 5: Modify `prepare_widget_tree` for selective walks

- [ ] Modify `prepare_widget_tree()` to accept `Option<&mut InvalidationTracker>` and skip children whose subtrees are all `Clean` (via `has_dirty_descendant()`). When `tracker` is `Some` and `!tracker.needs_full_rebuild()` and `!tracker.has_dirty_descendant(child.id())` and `!tracker.is_prepaint_dirty(child.id())`, skip that child's subtree. **Borrow pattern:** The `&mut` reference is reborrowed at each recursive call. The query (`has_dirty_descendant`) and mutation (`mark` from animator ticks) never overlap within a single stack frame — the query runs before recursion, the mark runs inside the recursive `prepare_widget_frame` call. This pattern compiles cleanly in Rust
- [ ] **Handle `full_invalidation` short-circuit.** When `InvalidationTracker::needs_full_rebuild()` returns `true`, the selective walk MUST fall back to a full tree walk (identical to current behavior). This is the correct response to resize, theme change, font change, and scale factor change. Do not attempt subtree queries when `full_invalidation` is set — it would be wasted work since every widget reports dirty. Add an explicit early check: `if tracker.needs_full_rebuild() { /* full walk */ }`
- [ ] Ensure lifecycle events still reach all widgets that need them — lifecycle delivery may require visiting widgets that aren't dirty themselves but have lifecycle events pending
- [ ] **Verify `InvalidationTracker::clear()` timing.** The dialog path currently reads `max_dirty_kind()` for phase gating, then runs prepare/prepaint, then `clear()` is called by `render_dispatch.rs:64` AFTER `render_dialog()` returns. With selective walks, the tracker must NOT be cleared between phase gating and selective walk execution — both reads happen in the same frame. Verify that `clear()` is only called once per frame, AFTER all phases complete. **Current production flow (verified):** `render_dirty_windows()` in `render_dispatch.rs` calls `render_dialog(wid)` then `ctx.root.invalidation_mut().clear()` — correct order. The Windows modal loop `modal_loop_render()` currently calls `clear()` BEFORE `handle_redraw()` — this is the bug fixed in Step 3 above
- [ ] Gate: selective prepare must produce identical results to full prepare for the same set of dirty widgets

**Design consideration:** Lifecycle events are delivered to specific widgets (not broadcast). `prepare_widget_tree()` delivers them by matching `widget.id()` against the event's target. If we skip clean subtrees, we must ensure lifecycle events for widgets in clean subtrees still get delivered. **Concrete strategy:** If `lifecycle_events` is non-empty, also mark each event's target widget as dirty (with ancestor propagation) before the selective walk begins. This way, the walk visits the targets naturally. If `lifecycle_events` is empty AND `full_invalidation` is false AND no animating widgets exist, the selective walk uses only the `dirty_map`.

**Design consideration — parent map staleness in dialog path:** The dialog path builds its parent map in `dispatch_dialog_content_key()` (`content_actions.rs:313`) and `setup_dialog_focus()` (`content_actions.rs:431`), both of which run on input events, NOT during `compose_dialog_widgets()`. If dirty-ancestor tracking relies on the parent map to walk ancestors when `mark()` is called, the parent map may be stale during render if no input event triggered a layout recomputation between the last structural change and the current render frame. **Mitigation:** `mark()` is called from the interaction pipeline (hover, active, focus), which always runs after layout/parent-map rebuild. The parent map is current at interaction dispatch time. Verify this invariant holds for all call sites. **Edge case — `invalidate_all()` callers:** `invalidate_all()` does not need the parent map (it sets `full_invalidation = true`, bypassing per-widget tracking). The `invalidate_all()` callers in `dialog_context/mod.rs:208`, `event_handling/mod.rs:94`, and `chrome/resize.rs:209` are all correct as-is

---

## 03.2 Per-Widget Dirty Tracking in Prepaint

**File(s):** `oriterm_ui/src/pipeline/tree_walk.rs` (`prepaint_widget_tree` — moved here in 03.0's extraction), `oriterm_ui/src/invalidation/mod.rs`

> **NOTE:** This subsection depends on 03.1's `mark()` wiring being complete. The `InvalidationTracker` must already be populated by the interaction pipeline before selective prepaint can skip anything. Do not implement 03.2 before 03.1 is fully wired.

Same approach for `prepaint_widget_tree()` — skip subtrees where no widget needs prepaint.

- [ ] Modify `prepaint_widget_tree()` to accept `Option<&InvalidationTracker>` and skip children whose subtrees are all `Clean` or `Paint`-only. Use `has_dirty_descendant()` for the subtree query (same API as 03.1). **Note:** `prepaint_widget_tree` currently takes `Option<&InteractionManager>`. Adding `Option<&InvalidationTracker>` as a second optional param brings the total to 8 params — the existing `#[expect(clippy::too_many_arguments)]` covers this
- [ ] A widget needs prepaint if its `DirtyKind >= Prepaint` — skip if all descendants are `< Prepaint`
- [ ] Ensure `VisualStateAnimator` updates still happen for animating widgets even if they aren't dirty from interaction — this is handled by 03.1's requirement to mark animating widgets as `Prepaint`-dirty
- [ ] **Interaction with Section 02's `PageContainerWidget` fix:** If `for_each_child_mut()` already only visits the active page (from Section 02), selective walks automatically skip inactive pages. Verify that the two optimizations compose correctly — selective walks should further reduce work within the active page's subtree

- [ ] Add a widget visit counter (debug-only) that logs how many widgets were visited during prepaint — use this to verify the selective walk actually reduces work. **Implementation:** Pass a `&mut usize` counter through the recursive `prepaint_widget_tree` calls (guarded by `#[cfg(debug_assertions)]`). Avoid `static AtomicU32` — statics are not reset between tests and cause race conditions in parallel test runs. The mutable counter is thread-local by nature of the recursive call stack and matches codebase patterns (no statics for state)
- [ ] Gate: selective prepaint must produce identical `resolved_bg`/`resolved_focused` values as full prepaint

---

## 03.3 Selective Paint via Damage Regions

**File(s):** `oriterm/src/app/dialog_rendering.rs`, `oriterm_ui/src/draw/damage/mod.rs`

> **WARNING — EXPLORATORY SUBSECTION.** This subsection is analysis-only unless feasibility is confirmed. If the analysis concludes "not feasible without retained scene," the correct outcome is documenting that conclusion and moving to Section 04, NOT building retained-scene infrastructure. Do NOT introduce any new types or caching infrastructure without a production consumer in this section.

The `DamageTracker` already computes per-widget damage after paint. Investigate whether it can be used *before* paint to skip widgets whose output hasn't changed.

- [ ] Evaluate feasibility: `DamageTracker` compares current vs previous scene hashes — but the hashes are computed from the scene primitives, which requires painting first. Can we predict "will this widget's paint output change?" without painting?
- [ ] Alternative approach: if a widget's prepaint resolved fields didn't change (same `resolved_bg`, same `resolved_focused`, same bounds), its paint output will be identical. Track this per-widget.
- [ ] **Concrete feasibility analysis:** Paint is an `&self` method on `Widget` — it cannot be easily skipped for individual widgets because the `Scene` is built by a depth-first traversal. Skipping a widget means its subtree produces no primitives, but surrounding widgets may reference the scene state (clip stack, offset stack) from the skipped widget's parent. A truly selective paint requires either (a) a retained scene that can be patched in place, or (b) caching per-widget scene fragments and replaying them. Both are significant architectural changes — this is likely the boundary where "quick wins" end and "retained scene" begins
- [ ] If selective paint is feasible, implement it for the dialog content tree only (smallest surface, easiest to verify)
- [ ] If not feasible without retained scene (likely), document the analysis and ensure Sections 02-03's prepare/prepaint wins are sufficient. Mark retained scene as a potential follow-up from Section 05
- [ ] Gate: selective paint must produce an identical `Scene` (byte-for-byte identical primitive arrays) compared to full paint for unchanged widgets

**Note:** This subsection is exploratory. If measurement from Section 02 shows that viewport culling alone makes dialog rendering cheap enough, this optimization may not be needed. Measure first, optimize second. The combination of viewport culling (Section 02) + active-page-only traversal (Section 02) + selective prepare/prepaint (Sections 03.1/03.2) is likely to handle the vast majority of wasted work without needing selective paint.

---

## 03.4 Tests

**File(s):** `oriterm_ui/src/pipeline/tests.rs` (tree-walk behavior tests), `oriterm_ui/src/invalidation/tests.rs` (dirty marking + ancestor tracking tests — already exists)

- [ ] **Test dirty marking integration:** Use `WidgetTestHarness` to simulate a hover event (`mouse_move_to`), then verify that `InvalidationTracker::is_prepaint_dirty(hovered_widget_id)` returns `true`. Do NOT access `dirty_map` directly — it is private. Use the public query methods (`is_prepaint_dirty()`, `is_paint_dirty()`, `has_dirty_descendant()`). This proves the wiring from 03.1 works end-to-end
- [ ] **Test dirty ancestor propagation:** Mark a leaf widget dirty via `mark()` with a parent map containing a 3-level hierarchy. Verify `has_dirty_descendant()` returns `true` for all ancestors and `false` for unrelated widgets. This goes in `invalidation/tests.rs`
- [ ] Add a test using `WidgetTestHarness` that marks one widget dirty, runs the pipeline, and verifies that only the dirty widget and its ancestors were visited during prepare/prepaint (via the debug visit counter from 03.2, or by checking that `is_prepaint_dirty` is `false` for unvisited widgets)
- [ ] Add a test that verifies lifecycle events are still delivered correctly when selective walks are enabled — specifically, deliver a `WidgetAdded` event to a widget in a clean subtree and verify it arrives. **Implementation:** The lifecycle event target should be pre-marked dirty (per the design consideration in 03.1) so the walk visits it
- [ ] Add a test that verifies animation-driven widgets continue to update even when no interaction-driven dirtiness exists
- [ ] Add a test that verifies animation-driven dirty marking persists across multiple frames: widget starts animating on frame N (`is_animating() == true`), frame N+1 should also mark the widget dirty via the `prepare_widget_frame` animator check, until `is_animating()` returns false
- [ ] Add a test that verifies `full_invalidation` causes the selective walk to fall back to a full tree walk — mark one widget dirty, set `full_invalidation = true`, verify ALL widgets are visited (not just the dirty one's subtree)
- [ ] Add a test that verifies `InvalidationTracker::clear()` resets both `dirty_map` AND `dirty_ancestors` — after `clear()`, `is_prepaint_dirty()` and `has_dirty_descendant()` should both return `false` for all previously-dirty widgets
- [ ] Before/after measurement: log widget visit counts for a hover event on a single button in a dialog with 50+ widgets, comparing full walk vs selective walk

**Test file organization:** All tests follow the sibling `tests.rs` pattern per `test-organization.md`:
- `invalidation/tests.rs` — existing, add ancestor-tracking unit tests here
- `pipeline/tests.rs` — existing, add selective walk integration tests here
- No inline test modules. No new test files unless a new source module is created

---

## 03.R Third Party Review Findings

- [x] `[TPR-03-001][medium]` **Dialog parent map only covers content widgets, not chrome.** `build_parent_map()` is called on the content widget's layout only (`content_actions.rs:313`), not on the chrome widget tree. This means dirty-ancestor tracking (03.1 option b) will only work for content widgets. Chrome hover/focus changes cannot be tracked through the parent map. **Impact on 03.1:** Either build a separate chrome parent map and merge it into the `InteractionManager`'s parent map, or accept that chrome dirty-ancestor tracking is not possible and fall back to full chrome tree walks (chrome is shallow enough that this is acceptable). **Recommended resolution:** Accept the limitation — chrome is a shallow tree (typically < 5 widgets: title bar label, close button, minimize/maximize buttons). Full chrome tree walks cost negligible CPU compared to the dialog content tree (50+ widgets). The selective walk optimization should target the content tree only. Chrome subtree walks remain full. No code change needed, but document this in 03.1 implementation notes.

- [x] `[TPR-03-002][low]` **`InvalidationTracker::dirty_map` visibility for subtree queries.** The `dirty_map` field is private and `InvalidationTracker` has no method to query "is any widget in subtree X dirty?" Section 03.1's Step 1 addresses this directly. **Recommended resolution (adopted in 03.1 Step 1):** Maintain a `HashSet<WidgetId>` of dirty ancestors inside `InvalidationTracker`. When `mark(id, kind, parent_map)` is called, walk `parent_map` upward from `id` and insert all ancestors into `dirty_ancestors`. Add `pub fn has_dirty_descendant(&self, id: WidgetId) -> bool` that checks `self.dirty_ancestors.contains(&id) || self.dirty_map.contains_key(&id)`. The parent map is passed at call time (not stored), keeping `InvalidationTracker` independent of `InteractionManager`. `dirty_ancestors` is cleared alongside `dirty_map` in `clear()`. **Test impact:** 12 existing test calls in `invalidation/tests.rs` must be updated to pass `&HashMap::new()`. No production callers exist yet, so no production code breaks

- [x] `[TPR-03-003][high]` **Diagnostic `log::info!` on every mouse move in production code.** `oriterm_ui/src/input/dispatch/tree.rs:272-280` had a `log::info!("hit_path: ...")` that fired on every `MouseMove` event, allocating a `Vec` and formatting a debug string on every pointer movement. The comment said "diagnostic -- remove after debugging." This was a hot-path allocation directly relevant to section 03's goal of making hover cheap.
  **Resolved 2026-03-21**: Removed the diagnostic `log::info!` block and its associated `Vec` allocation. Build, clippy, and tests all pass.

- [x] `[TPR-03-004][medium]` **`prepare_widget_tree` signature change will affect `deliver_lifecycle_events`.** The sync points list in this section originally missed `WindowRoot::deliver_lifecycle_events()` (private, line 301 in `window_root/pipeline.rs`), which also calls `prepare_widget_tree`. This caller has been added to the sync points list. It is called from `dispatch_event()` (line 92 and line 128) and is a high-frequency path for hover events.

- [x] `[TPR-03-005][high]` **Windows modal loop clears `InvalidationTracker` BEFORE render.** `event_loop_helpers/mod.rs:85` calls `ctx.root.invalidation_mut().clear()` before `self.handle_redraw()` during the Win32 modal move/resize loop. The normal render path in `render_dispatch.rs:41,64` correctly calls `clear()` AFTER render. This pre-render `clear()` wipes the `dirty_map` before selective walks can consume it. **Fix:** Move the `clear()` call to after `self.handle_redraw()` in `modal_loop_render()`. This is a latent correctness bug — it's harmless today because `dirty_map` is always empty in production, but it will break selective walks. Woven into 03.1 Step 3.

- [ ] `[TPR-03-006][medium]` **`apply_dispatch_requests` calls `InteractionManager` state-change methods.** `pipeline/mod.rs` line 372 calls `set_active()`, `clear_active()`, and `request_focus()`. If those methods change return types (03.1 Step 2), `apply_dispatch_requests` must either forward changed IDs or return them to the caller. Woven into 03.1 Step 3.

- [ ] `[TPR-03-007][high]` `oriterm/src/app/event_loop_helpers/mod.rs:83` — `modal_loop_render()` still clears `InvalidationTracker` before rendering, even though `[TPR-03-005]` is marked resolved here.
  Evidence: Current code still executes `ctx.root.invalidation_mut().clear()` immediately before `self.handle_redraw()`. The normal render path in `render_dispatch.rs` clears after rendering; the modal-loop path does not.
  Impact: Once Section 03 starts consuming per-widget dirty state in production, Win32 modal move/resize frames will discard that dirty state before the selective walk can see it. The plan currently hides that prerequisite behind resolved metadata.
  Required plan update: Move the modal-loop `clear()` call to after `self.handle_redraw()`, then mark the original finding resolved with fresh verification.

- [ ] `[TPR-03-008][medium]` `oriterm_ui/src/widgets/sidebar_nav/mod.rs:353` — `SidebarNavWidget` still emits `log::info!` on hover changes and every click in production input handling.
  Evidence: `on_input()` logs pointer coordinates, bounds, hit indices, and miss cases from the live `MouseMove`/`MouseDown` paths with no debug or feature gating.
  Impact: The sidebar now pays formatting/logging cost during ordinary pointer movement and click handling, which distorts the hover-cost work this section is trying to measure and leaves noisy production logs in a hot UI path.
  Required plan update: Remove these `log::info!` calls or downgrade them behind an explicit debug-only diagnostic path before using Section 05 measurements as evidence.

---

## 03.5 Build & Verify

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] 03.0 extraction is a pure refactor — no behavioral change, all existing tests pass
- [ ] New tests exist proving this section's changes work
- [ ] No `#[allow(dead_code)]` on new items — everything has a production caller
- [ ] `pipeline/mod.rs` is under 300 lines after extraction; `pipeline/tree_walk.rs` is under 500 lines
- [ ] `interaction/manager.rs` is under 500 lines after return-type changes
- [ ] `invalidation/mod.rs` is under 500 lines after `dirty_ancestors` addition
- [ ] A single-widget hover in the dialog visits only the hovered widget's ancestor chain during prepare/prepaint, not the entire tree
- [ ] Widget visit count for a hover interaction drops from O(total widgets) to O(depth of hovered widget)
- [ ] Windows modal loop `clear()` timing is corrected (clear AFTER render, not before)
- [ ] `apply_dispatch_requests` correctly forwards changed widget IDs for dirty marking

**Exit Criteria:** A `WidgetTestHarness` test demonstrates that a dirty-only hover on one widget in a 50+ widget tree visits fewer than 15 widgets during prepare/prepaint (ancestors of hovered widget + the widget itself, roughly O(tree depth)). `log::debug!` output in the dialog render path shows reduced visit counts for hover interactions. `InvalidationTracker::clear()` is called exactly once per frame, after all phases complete, verified by test or code audit. `full_invalidation` correctly falls back to full tree walk (verified by test). `dirty_ancestors` is cleared alongside `dirty_map`. All existing tests pass with 0 regressions.
