---
section: "03"
title: "Dialog Selective Walks"
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
goal: "Dialog prepare/prepaint/paint phases skip clean subtrees so that hover and page-local interactions cost proportional to the changed subtree, not the whole dialog tree"
inspired_by:
  - "InvalidationTracker per-widget dirty state (oriterm_ui/src/invalidation/mod.rs)"
  - "DamageTracker per-widget hash diff (oriterm_ui/src/draw/damage/mod.rs:56-104)"
depends_on: ["01", "02"]
sections:
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
    status: not-started
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
- `oriterm/src/app/widget_pipeline/mod.rs` — re-export list must include any new dependencies
- `oriterm/src/app/widget_pipeline/tests.rs` — test calls
- `oriterm_ui/src/window_root/pipeline.rs` — `WindowRoot::tick_animation()` (line 174) also calls `prepare_widget_tree` — must be included in signature sync

---

## 03.1 Per-Widget Dirty Tracking in Prepare

**File(s):** `oriterm_ui/src/pipeline/mod.rs` (orchestration), `oriterm_ui/src/pipeline/tree_walk.rs` (new — extracted from `mod.rs` as mandatory pre-step, contains `prepare_widget_tree` and other tree-walk functions), `oriterm_ui/src/invalidation/mod.rs`, `oriterm_ui/src/interaction/manager.rs`

> **WARNING — NO DEAD CODE.** This subsection must wire `mark()` into production AND modify `prepare_widget_tree()` to consume the dirty state, both in the same implementation pass. Do NOT land the `mark()` wiring without the selective walk consumer, and do NOT land the selective walk without the `mark()` wiring. Both together or neither.

**Critical prerequisite:** `InvalidationTracker::mark()` exists but is **never called from any production code path**. The `dirty_map` is always empty in production — only `full_invalidation` (via `invalidate_all()`) is used. Before any selective walk optimization can work, per-widget dirty marking must be wired into the interaction/lifecycle pipeline. This is the **central task** of this subsection.

The `prepare_widget_tree()` function walks the entire tree via `for_each_child_mut`. To make it selective, it needs to skip subtrees where no widget is dirty.

- [ ] **Wire `mark()` into `InteractionManager` state transitions.** When `InteractionManager` changes a widget's hot/active/focused state, it must call `InvalidationTracker::mark(widget_id, DirtyKind::Prepaint)` for the affected widget. **Specific call sites to audit:** `update_hot_path()` (`manager.rs:122` — the actual hot-tracking entry point; there is no `set_hot()`/`clear_hot()` on `InteractionManager` — those are on `InteractionState`), `set_active()` (`manager.rs:168`), `clear_active()` (`manager.rs:197`), any focus change method. The `InteractionManager` does not currently hold a reference to `InvalidationTracker` — either pass it as a parameter to the state-change methods, or collect the changed widget IDs and mark them in the caller (the event dispatch pipeline in `WindowRoot`). **File size constraint:** `manager.rs` is currently 406 lines (94 lines from the 500-line hard limit). Adding `InvalidationTracker` coupling directly WILL risk breaching the limit. **Recommended approach:** collect changed widget IDs as return values from `update_hot_path()`, `set_active()`, `clear_active()` and mark them in the caller (`WindowRoot`). This keeps `InteractionManager` lean and avoids a new dependency from `interaction/` to `invalidation/`
- [ ] **Also wire `mark()` into `VisualStateAnimator` ticks.** When an animator is actively interpolating (returning `is_animating() == true`), its widget needs `DirtyKind::Prepaint` on subsequent frames until the animation completes. Currently, `prepare_widget_frame()` (`pipeline/mod.rs:173-249`) handles this — when `animator.is_animating(now)` is true (line 243), it calls `flags.request_anim_frame()` (line 245). At that same point, also mark the widget dirty
- [ ] Add a method to `InvalidationTracker` that answers "is any widget in subtree rooted at X dirty?" — this requires knowing the parent-child relationship. **Two options:** (a) use the parent map (already built during layout in `WindowRoot`), or (b) use a simpler approach: track a `Set<WidgetId>` of dirty widget ancestors, populated when `mark()` is called by walking the parent map upward. Option (b) makes subtree queries O(1) instead of O(tree size)
- [ ] Modify `prepare_widget_tree()` to accept `Option<&InvalidationTracker>` and skip children whose subtrees are all `Clean`. **File size warning:** `pipeline/mod.rs` is currently 437 lines. Adding subtree-skip logic to both `prepare_widget_tree` and `prepaint_widget_tree` WILL push it over the 500-line limit. **Mandatory pre-step:** Extract the tree-walk functions (`prepare_widget_tree`, `prepare_widget_frame`, `prepaint_widget_tree`, `register_widget_tree`, `collect_focusable_ids`, `dispatch_keymap_action`) into a submodule `pipeline/tree_walk.rs` BEFORE adding any new logic. This keeps `pipeline/mod.rs` as the orchestration module and `tree_walk.rs` as the traversal module. Both files must stay under 500 lines
- [ ] Ensure lifecycle events still reach all widgets that need them — lifecycle delivery may require visiting widgets that aren't dirty themselves but have lifecycle events pending
- [ ] Gate: selective prepare must produce identical results to full prepare for the same set of dirty widgets

**Design consideration:** Lifecycle events are delivered to specific widgets (not broadcast). `prepare_widget_tree()` delivers them by matching `widget.id()` against the event's target. If we skip clean subtrees, we must ensure lifecycle events for widgets in clean subtrees still get delivered. **Concrete strategy:** If `lifecycle_events` is non-empty, also mark each event's target widget as dirty before the selective walk. This way, the walk visits the targets naturally. If `lifecycle_events` is empty AND `full_invalidation` is false AND no animating widgets exist, the selective walk uses only the `dirty_map`.

---

## 03.2 Per-Widget Dirty Tracking in Prepaint

**File(s):** `oriterm_ui/src/pipeline/tree_walk.rs` (`prepaint_widget_tree` — moved here in 03.1's mandatory extraction), `oriterm_ui/src/invalidation/mod.rs`

> **NOTE:** This subsection depends on 03.1's `mark()` wiring being complete. The `InvalidationTracker` must already be populated by the interaction pipeline before selective prepaint can skip anything. Do not implement 03.2 before 03.1 is fully wired.

Same approach for `prepaint_widget_tree()` — skip subtrees where no widget needs prepaint.

- [ ] Modify `prepaint_widget_tree()` to accept `Option<&InvalidationTracker>` and skip children whose subtrees are all `Clean` or `Paint`-only
- [ ] A widget needs prepaint if its `DirtyKind >= Prepaint` — skip if all descendants are `< Prepaint`
- [ ] Ensure `VisualStateAnimator` updates still happen for animating widgets even if they aren't dirty from interaction — this is handled by 03.1's requirement to mark animating widgets as `Prepaint`-dirty
- [ ] **Interaction with Section 02's `PageContainerWidget` fix:** If `for_each_child_mut()` already only visits the active page (from Section 02), selective walks automatically skip inactive pages. Verify that the two optimizations compose correctly — selective walks should further reduce work within the active page's subtree
- [ ] Add a widget visit counter (debug-only) that logs how many widgets were visited during prepaint — use this to verify the selective walk actually reduces work. **Implementation:** Add `#[cfg(debug_assertions)] static PREPAINT_VISIT_COUNT: std::sync::atomic::AtomicU32` or pass a mutable counter through the recursive calls
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

**File(s):** `oriterm_ui/src/pipeline/tests.rs` (tree-walk behavior tests), `oriterm_ui/src/invalidation/tests.rs` (dirty marking tests — already exists)

- [ ] **Test dirty marking integration:** Use `WidgetTestHarness` to simulate a hover event (mouse_move_to), then verify that `InvalidationTracker::dirty_map` contains the hovered widget's ID with `DirtyKind::Prepaint`. This proves the wiring from 03.1 works end-to-end
- [ ] Add a test using `WidgetTestHarness` that marks one widget dirty, runs the pipeline, and verifies that only the dirty widget and its ancestors were visited during prepare/prepaint (via a visit counter or mock)
- [ ] Add a test that verifies lifecycle events are still delivered correctly when selective walks are enabled — specifically, deliver a `WidgetAdded` event to a widget in a clean subtree and verify it arrives
- [ ] Add a test that verifies animation-driven widgets continue to update even when no interaction-driven dirtiness exists
- [ ] Before/after measurement: log widget visit counts for a hover event on a single button in a dialog with 50+ widgets, comparing full walk vs selective walk

---

## 03.R Third Party Review Findings

- None.

---

## 03.5 Build & Verify

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] New tests exist proving this section's changes work
- [ ] No `#[allow(dead_code)]` on new items — everything has a production caller
- [ ] A single-widget hover in the dialog visits only the hovered widget's ancestor chain during prepare/prepaint, not the entire tree
- [ ] Widget visit count for a hover interaction drops from O(total widgets) to O(depth of hovered widget)

**Exit Criteria:** A `WidgetTestHarness` test demonstrates that a dirty-only hover on one widget in a 50+ widget tree visits fewer than 15 widgets during prepare/prepaint (ancestors of hovered widget + the widget itself, roughly O(tree depth)). `log::debug!` output in the dialog render path shows reduced visit counts for hover interactions. All existing tests pass with 0 regressions.
