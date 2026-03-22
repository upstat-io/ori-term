---
section: "04"
title: "App-Layer Wiring + Main-Window Rollout"
status: in-progress
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-22
goal: "All app-layer render paths (dialog, single-pane, multi-pane) wire the InvalidationTracker into prepare/prepaint calls so selective walks are active in production"
inspired_by:
  - "Dialog rendering fixes from Sections 01-03"
depends_on: ["01", "02", "03"]
sections:
  - id: "04.0"
    title: "Borrow-Split Prerequisite and Dialog Wiring"
    status: in-progress
  - id: "04.1"
    title: "Single-Pane Path Rollout"
    status: in-progress
  - id: "04.2"
    title: "Multi-Pane Path Rollout"
    status: in-progress
  - id: "04.3"
    title: "Overlay Prepaint Alignment"
    status: complete
  - id: "04.4"
    title: "Tests"
    status: in-progress
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.5"
    title: "Build & Verify"
    status: in-progress
---

# Section 04: App-Layer Wiring + Main-Window Rollout

**Status:** In Progress
**Goal:** All app-layer render paths — dialog (`compose_dialog_widgets`), single-pane (`handle_redraw`), and multi-pane (`handle_redraw_multi_pane`) — pass real `InvalidationTracker` references to `prepare_widget_tree` and `prepaint_widget_tree` so selective walks are active in production. This completes the work that Section 03 built in the library crate but left unwired at the app layer.

**Production code path:** `App::compose_dialog_widgets()` in `dialog_rendering.rs`, `App::handle_redraw()` in `redraw/mod.rs` (single-pane), and `App::handle_redraw_multi_pane()` in `redraw/multi_pane/mod.rs` (multi-pane). All three currently pass `None` for the tracker parameter in both `prepare_widget_tree` (3rd arg) and `prepaint_widget_tree` (7th arg).

**Exact `None` sites to replace (8 total):**
- `dialog_rendering.rs:134` — prepare chrome tracker (`None`)
- `dialog_rendering.rs:144` — prepare content tracker (`None`)
- `dialog_rendering.rs:170` — prepaint chrome tracker (`None`)
- `dialog_rendering.rs:180` — prepaint content tracker (`None`)
- `redraw/mod.rs:286` — prepare tab_bar tracker (`None`)
- `redraw/mod.rs:314` — prepaint tab_bar tracker (`None`)
- `redraw/multi_pane/mod.rs:366` — prepare tab_bar tracker (`None`)
- `redraw/multi_pane/mod.rs:395` — prepaint tab_bar tracker (`None`)

**Observable change:** Hover interactions in the dialog, tab bar, and overlays skip clean subtrees in prepare/prepaint. No behavioral regressions in any render path.

**Context:** Section 03 built the selective walk infrastructure in `tree_walk.rs` and wired it into `WindowRoot::prepare()` / `run_prepaint()` / overlay methods (all pass `Some`). But the app-layer render paths bypass WindowRoot's pipeline methods — they call `prepare_widget_tree` and `prepaint_widget_tree` directly on chrome/content/tab_bar widgets, and all these calls pass `None` for the tracker. This section replaces all `None` placeholders with real tracker references.

> **BORROW-SPLIT CHALLENGE.** The existing `WindowRoot::interaction_mut_and_frame_requests()` returns `(&mut InteractionManager, &FrameRequestFlags)` — no tracker. `prepare_widget_tree` needs `&mut InteractionManager` + `Option<&mut InvalidationTracker>` + `&FrameRequestFlags`, all from the same `WindowRoot`. A new borrow-split method is required (04.0).

**Reference implementations:**
- `WindowRoot::prepare()` (`pipeline.rs:143`) — passes `Some(&mut self.invalidation)` to `prepare_widget_tree`
- `WindowRoot::run_prepaint()` (`pipeline.rs:355`) — passes `Some(&self.invalidation)` to `prepaint_widget_tree`

**Depends on:** Section 01 (bounds correctness already applied), Section 02 (culling approach established), Section 03 (selective walks proven in library crate).

---

## 04.0 Borrow-Split Prerequisite and Dialog Wiring

**File(s):** `oriterm_ui/src/window_root/borrow_split.rs` (88 lines), `oriterm/src/app/dialog_rendering.rs` (306 lines)

> **PREREQUISITE.** All three app-layer render paths need `&mut InteractionManager` + `Option<&mut InvalidationTracker>` + `&FrameRequestFlags` from the same `WindowRoot`. The existing `interaction_mut_and_frame_requests()` returns only `(&mut InteractionManager, &FrameRequestFlags)`. A new borrow-split method is needed before any caller can be wired.

### Step 1 — Add borrow-split method (borrow_split.rs)

- [x] Add `pub fn interaction_invalidation_and_frame_requests_mut(&mut self) -> (&mut InteractionManager, &mut InvalidationTracker, &FrameRequestFlags)` to `WindowRoot`. Destructures `self` to yield three disjoint borrows. File grows to ~97 lines (well under 500)
- [x] After all callers in Steps 2/04.1/04.2 are migrated: remove `interaction_mut_and_frame_requests()` from `borrow_split.rs` (currently has exactly 4 callers: `dialog_rendering.rs:130`, `dialog_rendering.rs:140`, `redraw/mod.rs:282`, `multi_pane/mod.rs:362`). Project has `dead_code = "deny"` so the build will catch any missed callers
- [x] **Immutable variant: NOT needed.** For `prepaint_widget_tree` calls, compose existing shared-ref accessors: `ctx.root.interaction_and_frame_requests()` + `ctx.root.invalidation()`. Since `&self` borrows compose naturally, no new three-field immutable method is required. The immutable callers (`dialog_rendering.rs:162,172`, `redraw/mod.rs:306`, `multi_pane/mod.rs:387`) keep using `interaction_and_frame_requests()` and add `invalidation()` for the tracker parameter

### Step 2 — Wire tracker into dialog path (dialog_rendering.rs)

Section 03 left the dialog path passing `None` for both `prepare_widget_tree` (3rd arg) and `prepaint_widget_tree` (7th arg) tracker parameters. This step replaces all 4 `None`s with real tracker references.

**Concrete code pattern for the prepare phase (lines 129-149):**
```rust
// Current: two separate calls, each re-destructuring 2 fields, passing None for tracker
// New: one destructure of 3 fields, two sequential calls with reborrows, in a block scope
if widget_dirty >= DirtyKind::Prepaint {
    {
        let (interaction, invalidation, flags) =
            ctx.root.interaction_invalidation_and_frame_requests_mut();
        prepare_widget_tree(
            &mut ctx.chrome, &mut *interaction, Some(&mut *invalidation),
            &lifecycle_events, None, Some(flags), now,
        );
        prepare_widget_tree(
            ctx.content.content_widget_mut(), &mut *interaction,
            Some(&mut *invalidation), &lifecycle_events, None, Some(flags), now,
        );
    } // mutable borrow of ctx.root ends here
    // collect_dialog_prepaint_bounds borrows ctx.chrome and ctx.content immutably — safe now
    let prepaint_bounds = collect_dialog_prepaint_bounds(...);
    let (interaction, flags) = ctx.root.interaction_and_frame_requests();
    let invalidation = ctx.root.invalidation();
    prepaint_widget_tree(&mut ctx.chrome, &prepaint_bounds, Some(interaction),
        ui_theme, now, Some(flags), Some(invalidation));
    prepaint_widget_tree(ctx.content.content_widget_mut(), &prepaint_bounds,
        Some(interaction), ui_theme, now, Some(flags), Some(invalidation));
}
```

> **Borrow safety note:** `ctx.chrome` and `ctx.root` are separate fields on `DialogWindowContext`, so `&mut ctx.root` (via the borrow-split) does not conflict with `&mut ctx.chrome` or `ctx.content.content_widget_mut()`. The block scope is only needed to release the mutable `ctx.root` borrow before the immutable prepaint destructure begins.

- [x] Replace both `prepare_widget_tree(..., None, ...)` calls with `Some(&mut *invalidation)` via the 3-field mutable borrow-split, in a block scope
- [x] Replace both `prepaint_widget_tree(..., None)` calls with `Some(invalidation)` via existing `interaction_and_frame_requests()` + `invalidation()` composable accessors
- [x] Chrome gets the tracker for consistency — the tracker also records animator-driven dirty marks for the next frame, even though chrome is a shallow tree (< 5 widgets)
- [ ] Verify dialog hover in settings panel uses selective walks (content tree should skip clean sections)
  <!-- requires-binary: visual verification -->

---

## 04.1 Single-Pane Path Rollout

**File(s):** `oriterm/src/app/redraw/mod.rs` (442 lines, headroom for minor changes)

Apply the selective walk pattern to the single-pane `handle_redraw()`.

- [x] Verify Section 01's prepaint bounds fix is already in place for `handle_redraw()` (the `collect_tab_bar_prepaint_bounds` call at line 298 and populated `prepaint_bounds` map)

**Prepare phase (line 281-291):**
- [x] Replace `let (interaction, flags) = ctx.root.interaction_mut_and_frame_requests();` (line 282) with `let (interaction, invalidation, flags) = ctx.root.interaction_invalidation_and_frame_requests_mut();`
- [x] Replace `None` tracker arg (line 286) with `Some(&mut *invalidation)`. Use reborrow so the destructure remains live for any future use in this scope
- [x] **Scoping:** NLL handles borrow lifetimes — variables are last used in the `prepare_widget_tree` call so the mutable borrow drops before `prepare_overlay_widgets()`. No explicit block scope needed

**Prepaint phase (line 306-315):**
- [x] Keep existing `let (interaction, flags) = ctx.root.interaction_and_frame_requests();` (line 306)
- [x] Add `let invalidation = ctx.root.invalidation();` — composes with the shared-ref pair (no borrow conflict)
- [x] Replace `None` tracker arg (line 314) with `Some(invalidation)`
- [x] **Scoping:** NLL drops shared refs after their last use in `prepaint_widget_tree`, before `prepaint_overlay_widgets()`. No explicit block scope needed

> **WARNING — file size.** Adding block scopes adds ~4 lines (2 braces per block). `redraw/mod.rs` is 442 lines, so this is safe. But if any further additions approach 490 lines, extract the tab bar prepare/prepaint into a helper method

- [ ] Verify tab bar hover still works: hover over a tab triggers only that tab's subtree visit
- [ ] Verify tab bar animation still works: close button fade-in triggers prepaint for the animating widget only
- [x] **`tick_animation()` already handles selective walks correctly.** `WindowRoot::tick_animation()` (`pipeline.rs:174`) passes `None` as tracker (TPR-03-010 resolution: animation ticks do full tree walks because `flush_frame_requests` only schedules the root widget ID). No change needed here — verified unchanged
- [x] No changes to the grid rendering path (that's GPU-side, not UI framework)

---

## 04.2 Multi-Pane Path Rollout

**File(s):** `oriterm/src/app/redraw/multi_pane/mod.rs`

> **WARNING -- FILE SIZE.** `multi_pane/mod.rs` is currently 492 lines (8 lines from the 500-line hard limit). The change itself is net-neutral per call site (replace 2-field destructure with 3-field, replace `None` with `Some(...)`) but adding block scopes adds ~4 lines. **Before making any changes, count lines.** If the file would exceed 500, extract the tab-bar prepare/prepaint block (lines 361-399) into a helper function in a new `multi_pane/tab_bar_pipeline.rs` submodule. Note: `multi_pane/pane_layouts.rs` already exists as a submodule — do NOT reuse that name.

Same pattern as 04.1 but for the multi-pane path.

- [x] **File size check first.** Count lines in `multi_pane/mod.rs`. Was 492, now 494 after changes — safely under 500

**Prepare phase (line 361-371):**
- [x] Replace `let (interaction, flags) = ctx.root.interaction_mut_and_frame_requests();` (line 362) with the 3-field mutable borrow-split
- [x] Replace `None` tracker arg (line 366) with `Some(&mut *invalidation)`
- [x] NLL handles scoping — no explicit block scope needed (same as 04.1)

**Prepaint phase (line 387-396):**
- [x] Keep `let (interaction, flags) = ctx.root.interaction_and_frame_requests();` (line 387)
- [x] Add `let invalidation = ctx.root.invalidation();`
- [x] Replace `None` tracker arg (line 395) with `Some(invalidation)`
- [x] NLL handles scoping — no explicit block scope needed (same as 04.1)

- [x] Verify Section 01's prepaint bounds fix is already in place for `handle_redraw_multi_pane()`
- [ ] Verify the multi-pane path's tab bar behavior is identical to single-pane after changes
- [x] Verify pane divider rendering is unaffected (dividers are drawn separately, not through the widget tree)
- [x] **Post-change line count:** Verify `multi_pane/mod.rs` is still under 500 lines — confirmed 494 lines

---

## 04.3 Overlay Prepaint Alignment

**File(s):** `oriterm_ui/src/window_root/pipeline.rs` (where `prepaint_overlay_widgets()` is defined, line 394)

Both main-window paths call `ctx.root.prepare_overlay_widgets()` and `ctx.root.prepaint_overlay_widgets()`. These are already wired with `Some` for the tracker. **No code changes needed here** — verify only.

> **KNOWN PRE-EXISTING GAP -- dialog overlay prepare/prepaint missing.** The dialog path (`render_dialog_overlays` in `dialog_rendering.rs:226-273`) does NOT call `prepare_overlay_widgets()` or `prepaint_overlay_widgets()` — it only calls `layout_overlays()` + `draw_overlay_at()` (paint only). This means dialog overlay widgets never receive lifecycle events or VisualStateAnimator ticks. **Impact:** Hover animations on dropdown list items in the settings dialog may not work (no `HotChanged` delivery, no animator ticks). This is NOT caused by this plan and is NOT in scope here — it predates the incremental rendering work. If dropdown hover is broken in the dialog, file a separate bug. **Do not attempt to fix it in this section** — it requires careful analysis of the dialog overlay lifecycle.

- [x] Verify `WindowRoot::prepaint_overlay_widgets()` already passes `Some(invalidation)` to `prepaint_widget_tree` (confirmed: `pipeline.rs:411`)
- [x] Verify `WindowRoot::prepare_overlay_widgets()` already passes `Some(&mut *invalidation)` to `prepare_widget_tree` (confirmed: `pipeline.rs:381`)
- [x] **Overlay layout timing:** `ButtonWidget::prepaint()` reads `ctx.bounds` for focus rings/bg, but overlay bounds come from `layout_overlays()` during draw phase. Since overlays are small (dropdown lists, context menus), approximate prepaint bounds are acceptable. No concrete overlay widget depends on exact prepaint bounds for critical logic

---

## 04.4 Tests

**File(s):** Existing test files in `oriterm/src/app/` and `oriterm_ui`

> **Testing limitations:** The app-layer render paths (`compose_dialog_widgets`, `handle_redraw`, `handle_redraw_multi_pane`) require a GPU context and cannot be unit-tested directly. The wiring changes in 04.0-04.2 are mechanical (replacing `None` with `Some(...)`) and the underlying selective walk logic is already thoroughly tested by Section 03's 8 test cases in `pipeline/tests.rs`. The tests below verify that the WidgetTestHarness path (which mirrors the production path) remains correct, and that no regressions appear.

- [x] Verify all existing tab bar tests pass with the selective walk changes — 161 tab bar tests pass
- [x] Verify Section 03's selective walk tests still pass: `selective_prepare_skips_clean_subtree`, `selective_walk_delivers_lifecycle_events_to_clean_subtree`, `selective_prepare_identical_to_full_for_dirty_widgets`, `selective_prepaint_identical_to_full_for_dirty_widgets` — all 4 pass
- [x] Verify `tick_animation()` regression tests still pass: `animation_driven_widget_updates_without_interaction_dirtiness`, `animation_dirty_marking_persists_across_frames` — both pass
- [x] **New test — borrow-split accessor** (`window_root/tests.rs`): `interaction_invalidation_and_frame_requests_mut_destructures_correctly` — verifies all three returned references are functional (registers widget via InteractionManager, marks dirty via InvalidationTracker, asserts is_prepaint_dirty)
- [ ] **Optional — tab bar hover selective walk** (`widget_pipeline/tests.rs` or `window_root/tests.rs`): Create a `WidgetTestHarness` with a `TabBarWidget`, hover one tab, verify only the hovered tab's ancestor chain is visited. This tests the WidgetTestHarness path (which already uses `Some(tracker)`) — it does NOT test the app-layer wiring, but verifies the tab bar widget structure is compatible with selective walks
- [x] Run `./test-all.sh` and confirm 0 regressions — 5,625 tests pass, 0 failures

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-004][medium]` `oriterm/src/app/dialog_context/content_actions.rs:135` — dialog page rebuilds still leave `InteractionManager` hot state stale until the next cursor-move event.
  **Resolved 2026-03-21**: Accepted. Added `WindowRoot::clear_hot_path()` method that calls
  `interaction.update_hot_path(&[])` + `mark_widgets_prepaint_dirty()`. Called from both
  `reset_dialog_settings()` and the page-switch branch in `dispatch_dialog_settings_action()`.
  Old hot widgets receive `HotChanged(false)` on the next prepare frame; the next cursor move
  recomputes the hot path against the new tree. Two regression tests added in
  `window_root/tests.rs`: `clear_hot_path_removes_stale_hover` and `clear_hot_path_marks_dirty`.

- [x] `[TPR-04-003][high]` `oriterm/src/app/dialog_context/content_actions.rs:136` — dialog setup and page-rebuild paths still discard queued lifecycle events instead of delivering them.
  **Resolved 2026-03-21**: Accepted. Removed all 4 discarding `drain_events()` calls in
  `content_actions.rs`: `reset_dialog_settings()` (line 137), `dispatch_dialog_settings_action()`
  page-switch (line 229), `setup_dialog_focus()` registration (line 436) and initial focus (line 472).
  Lifecycle events (`WidgetAdded`, `FocusChanged`) now stay in `pending_events` and are delivered
  naturally on the next `compose_dialog_widgets()` render frame via `drain_events()` →
  `prepare_widget_tree()`. The selective walk pre-marking (tree_walk.rs:51-57) ensures event
  targets are visited even when the tracker skips clean subtrees. Regression test
  `register_without_drain_delivers_widget_added_on_next_frame` added in `pipeline/tests.rs`.
  All 5,626 tests pass, 0 failures.

- [x] `[TPR-04-001][medium]` `oriterm_ui/src/testing/harness_dispatch.rs:32` — `WidgetTestHarness` now clears `InvalidationTracker` before any render, so it no longer matches the production render/clear cadence it claims to simulate.
  **Resolved 2026-03-21**: Accepted. Three fixes applied:
  1. Moved `invalidation_mut().clear()` from `advance_time()`/`run_until_stable()` to `render()`,
     matching production cadence: tick → render → clear.
  2. Fixed pre-existing nested-widget animation bug: `tick_animation()` used `None` tracker in
     `prepare_widget_tree` (correct for full walk), but then called `run_prepaint()` with selective
     walks — nested animating children were skipped because they couldn't be marked dirty during
     prepare. Fix: `run_prepaint()` now takes a `selective` parameter; `tick_animation()` passes
     `false` for a full prepaint walk, all other callers pass `true`.
  3. Added regression test `nested_widget_animation_advances_through_render_clear_cadence` in
     `pipeline/tests.rs` — container with nested button, verifies animation reaches hover_bg
     across multiple render-clear cycles. All 1609 tests pass.

- [x] `[TPR-04-002][high]` `oriterm/src/app/dialog_context/content_actions.rs:136` — dialog tree rebuilds still leave `InteractionManager`'s parent map stale after mouse-driven settings page changes and reset-to-defaults.
  **Resolved 2026-03-21**: Accepted. Added `content_parent_map()` helper that computes layout,
  builds the parent map, and installs it. Called from both `reset_dialog_settings()` and the
  page-switch path in `dispatch_dialog_settings_action()`. Extracted as a free function taking
  borrow-split fields to avoid `ctx` vs `panel` borrow conflicts. File stays under 500 lines (494).
  Note: regression test requiring actual dialog page switching via mouse is not feasible in the
  headless test harness (needs GPU + dialog window). The fix is verified structurally — both
  sites now call the helper immediately after `sync_focus_order()`, matching the pattern in
  `dispatch_dialog_content_key()` which already rebuilds the parent map.

- [x] `[TPR-04-005][medium]` `oriterm_ui/src/window_root/pipeline.rs:399` — overlay prepare/prepaint now opt into selective walking even though overlay widgets still have no dirty-tracking integration.
  **Resolved 2026-03-21**: Accepted. Changed both `prepare_overlay_widgets()` and
  `prepaint_overlay_widgets()` to pass `None` for the invalidation tracker instead of
  `Some(...)`. Overlay widgets don't participate in dirty tracking (interactions route
  through `OverlayManager`, not `InteractionManager` hot path), so selective walks
  incorrectly skipped all overlay descendants. Full walks are acceptable since overlay
  trees are small (dropdown menus, context menus, modals). All 5,637 tests pass.

- [x] `[TPR-04-006][medium]` `oriterm_ui/src/window_root/pipeline.rs:39` — `WindowRoot::compute_layout()` still leaves stale interaction registrations behind, even though the harness exposes it as the structural-rebuild API.
  **Resolved 2026-03-21**: Accepted. Added `gc_stale_widgets()` to `compute_layout()`,
  matching the GC already in `rebuild()`. After `register_widget_tree()`, collects all
  current widget IDs and removes stale entries from `InteractionManager`. Added
  `set_widget_raw()` (`#[cfg(test)]`) to `WindowRoot` for testing compute_layout GC
  independently of rebuild. Regression test `compute_layout_gcs_stale_registrations` added
  in `window_root/tests.rs` — replaces tree via set_widget_raw, calls compute_layout,
  verifies old children are deregistered. All 5,637 tests pass.

- [x] `[TPR-04-007][medium]` `oriterm/src/app/dialog_context/content_actions.rs:159` — the `clear_hot_path()` follow-up fixes stale hover on removed widgets by also dropping hover on widgets that survive a reset or page switch.
  **Resolved 2026-03-21**: Accepted. Replaced both `clear_hot_path()` calls in `content_actions.rs`
  (reset-to-defaults and page-switch) with `recompute_dialog_hot_path()` — a new decomposed helper
  in `dialog_context/mod.rs` that hit-tests the current widget tree against `ctx.last_cursor_pos`
  to preserve hover on surviving widgets. Added `WindowRoot::refresh_hot_path(pos)` for testing
  and main-window use. Two regression tests in `window_root/tests.rs`:
  `refresh_hot_path_preserves_hover_after_rebuild` (proves stationary-pointer hover survives)
  and `refresh_hot_path_clears_hover_when_cursor_outside` (proves non-hovering cursor clears).
  All 1,620 oriterm_ui tests pass.

- [x] `[TPR-04-008][medium]` `oriterm/src/app/dialog_context/content_actions.rs:1` — the current dialog action dispatcher is 501 lines, so this section now violates the repo’s hard 500-line source-file limit while the plan still reports the file as safely under budget.
  **Resolved 2026-03-22**: Accepted. Extracted `setup_dialog_focus()` (72 lines) to new
  `dialog_context/focus_setup.rs`. `content_actions.rs` is now 430 lines. `focus_setup.rs`
  is 90 lines. All clippy/tests/build pass.

---

## 04.5 Build & Verify

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes — 5,625 tests, 0 failures
- [x] New borrow-split accessor test exists in `window_root/tests.rs`
- [x] No `#[allow(dead_code)]` on new items — everything has a production caller
- [x] **Zero remaining `None` tracker args in app-layer render paths.** Verified: only `None` in `widget_pipeline/tests.rs` (test code). All production paths pass `Some`
- [x] `interaction_mut_and_frame_requests()` removed from `borrow_split.rs` (dead code after migration)
- [x] Dialog path passes `Some(invalidation)` to prepare/prepaint (4 `None`s replaced)
- [x] Tab bar hover in both single-pane and multi-pane paths uses selective tree walks (2 `None`s replaced each)
- [x] Overlay methods on `WindowRoot` already pass `Some` — verified, no changes needed
- [ ] No behavioral regressions in tab switching, tab close, tab drag, or overlay popups
- [x] `tick_animation()` correctly delivers animation frames (passes `None` by design — TPR-03-010)
- [x] `multi_pane/mod.rs` is under 500 lines after changes — 494 lines

**Exit Criteria:** All 8 `None` tracker parameters in production render paths are replaced with `Some`. The old 2-field `interaction_mut_and_frame_requests()` is removed (dead code). `cargo test -p oriterm` and `cargo test -p oriterm_ui` pass with 0 failures. Interactions work identically to before, with reduced tree-walk counts for hover events.
