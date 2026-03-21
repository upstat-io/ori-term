---
section: "04"
title: "Main-Window Rollout"
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
goal: "The single-pane and multi-pane main-window render paths benefit from the same correctness fixes and optimizations proven in the dialog path (Sections 01-03)"
inspired_by:
  - "Dialog rendering fixes from Sections 01-03"
depends_on: ["01", "02", "03"]
sections:
  - id: "04.1"
    title: "Single-Pane Path Rollout"
    status: not-started
  - id: "04.2"
    title: "Multi-Pane Path Rollout"
    status: not-started
  - id: "04.3"
    title: "Overlay Prepaint Alignment"
    status: not-started
  - id: "04.4"
    title: "Tests"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.5"
    title: "Build & Verify"
    status: not-started
---

# Section 04: Main-Window Rollout

**Status:** Not Started
**Goal:** The single-pane and multi-pane main-window render paths use the same correct prepaint bounds, viewport culling, and selective tree walks that were proven in the dialog path. This is a rollout, not a redesign — apply the identical patterns from Sections 01-03.

**Production code path:** `App::handle_redraw()` in `redraw/mod.rs` (single-pane) and `App::handle_redraw_multi_pane()` in `redraw/multi_pane/mod.rs` (multi-pane). Both paths handle tab bar widgets, overlay popups, and the chrome scene.

**Observable change:** Tab bar hover interactions skip clean subtrees in prepare/prepaint. Overlay popups receive correct prepaint bounds. No behavioral regressions in tab switching, tab bar animation, or overlay popup rendering.

**Context:** Section 01 already fixes the empty prepaint bounds in all three paths. This section applies the selective walk optimizations from Section 03 to the main-window paths. The tab bar widget tree is smaller than the dialog content tree, so the improvement is less dramatic, but correctness and consistency matter. The main-window paths also handle overlays (dropdown lists, context menus) which need the same treatment.

> **WARNING — SIGNATURE SYNC.** If Section 03 changed the signatures of `prepare_widget_tree()` or `prepaint_widget_tree()` (e.g., adding `Option<&InvalidationTracker>`), those changes already propagated to ALL callers in Section 03's sync point list. This section should NOT need to change any signatures — it only needs to verify the already-changed call sites in `handle_redraw()` and `handle_redraw_multi_pane()` are passing real `InvalidationTracker` references instead of `None`. If Section 03 used `None` as a temporary placeholder in these paths, this section replaces `None` with the real tracker reference.

**Reference implementations:**
- Dialog path fixes from Sections 01-03 — the proven pattern to replicate

**Depends on:** Section 01 (bounds correctness already applied), Section 02 (culling approach established), Section 03 (selective walks proven in dialog).

---

## 04.1 Single-Pane Path Rollout

**File(s):** `oriterm/src/app/redraw/mod.rs`

Apply the selective walk pattern from Section 03 to the single-pane `handle_redraw()`.

- [ ] Verify Section 01's prepaint bounds fix is already in place for `handle_redraw()`
- [ ] Apply selective `prepare_widget_tree()` — pass `InvalidationTracker` reference so clean tab bar subtrees are skipped
- [ ] Apply selective `prepaint_widget_tree()` — same approach
- [ ] Verify tab bar hover still works: hover over a tab → only that tab's subtree is visited
- [ ] Verify tab bar animation still works: close button fade-in triggers prepaint for the animating widget only
- [ ] No changes to the grid rendering path (that's GPU-side, not UI framework)

---

## 04.2 Multi-Pane Path Rollout

**File(s):** `oriterm/src/app/redraw/multi_pane/mod.rs`

> **WARNING — FILE SIZE.** `multi_pane/mod.rs` is currently 490 lines (10 lines from the 500-line hard limit). Any additions here must be offset by extractions. If passing `InvalidationTracker` references adds lines, extract helper functions into `multi_pane/helpers.rs` or similar before making the change.

Same as 04.1 but for the multi-pane path.

- [ ] Verify Section 01's prepaint bounds fix is already in place for `handle_redraw_multi_pane()`
- [ ] Apply selective `prepare_widget_tree()` for the tab bar in multi-pane mode
- [ ] Apply selective `prepaint_widget_tree()` for the tab bar in multi-pane mode
- [ ] Verify the multi-pane path's tab bar behavior is identical to single-pane after changes
- [ ] Verify pane divider rendering is unaffected (dividers are drawn separately, not through the widget tree)
- [ ] Verify `register_widget_tree()` still registers all widgets correctly — Section 02 changed `register_widget_tree()` to use `for_each_child_mut_all()` (which visits all pages), so registration should still work. Confirm this is the case in multi-pane mode specifically, where registration timing may differ from dialog mode

---

## 04.3 Overlay Prepaint Alignment

**File(s):** `oriterm_ui/src/window_root/pipeline.rs` (where `prepaint_overlay_widgets()` is defined, line 360)

Both main-window paths call `ctx.root.prepaint_overlay_widgets(&prepaint_bounds, ...)`. Ensure overlays also benefit from the correct bounds and selective walks.

- [ ] Verify `WindowRoot::prepaint_overlay_widgets()` receives the populated bounds map (from Section 01's fix)
- [ ] **Overlay layout timing problem and resolution:** Overlays are positioned by `OverlayManager::layout_overlays()` which is called during the draw phase (`draw_helpers.rs:159`), not before prepaint. This means overlay layout bounds are NOT available for `collect_layout_bounds()` at prepaint time. **Resolution strategy (pick one):**
  - **(a) Move `layout_overlays()` before prepaint.** Call it right after `prepare_overlay_widgets()` in both redraw paths. This requires a measurer to be available before the draw phase — the same measurer created for tab bar prepaint bounds (from Section 01) can be reused. Then `collect_layout_bounds()` from the overlay layout can be added to `prepaint_bounds`. **Risk:** Layout depends on the trigger widget's rendered position, which may not be finalized until paint. Check if overlay anchor bounds come from layout (available early) or paint (available late).
  - **(b) Accept that overlay prepaint bounds are approximate.** Overlays are small (dropdown lists, context menus) and their prepaint needs are minimal. If no overlay widget uses `PrepaintCtx::bounds` for critical logic, the empty bounds are harmless. **Verify:** Audit overlay widget `prepaint()` implementations to confirm they don't depend on bounds.
  - **(c) Compute overlay bounds separately.** After `prepare_overlay_widgets()`, compute overlay layout into a separate bounds map and merge it with the main bounds map before calling `prepaint_overlay_widgets()`.

  **Recommended path:** Start with option (b) — audit overlay `prepaint()` impls first. If no overlay widget depends on `PrepaintCtx::bounds`, the problem is harmless and no code change is needed. Only pursue (a) or (c) if a concrete overlay widget requires accurate bounds during prepaint.
**Note:** Overlays do NOT use `WindowRoot::run_prepaint()` — the main-window paths call `WindowRoot::prepaint_overlay_widgets()` which directly passes through the caller's `bounds_map`.

- [ ] Verify that selective walk changes in `prepaint_widget_tree()` also apply when called by `prepaint_overlay_widgets()` — both code paths should use the same `Option<&InvalidationTracker>` parameter

---

## 04.4 Tests

**File(s):** Existing test files in `oriterm/src/app/redraw/` and `oriterm_ui`

- [ ] Verify all existing tab bar tests pass with the selective walk changes
- [ ] Add a test (or extend an existing one) that verifies tab bar hover visits only the hovered tab's subtree
- [ ] Add a test that verifies overlay prepaint receives non-zero bounds
- [ ] Run the full test suite and confirm 0 regressions

---

## 04.R Third Party Review Findings

- None.

---

## 04.5 Build & Verify

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] New tests exist proving this section's changes work
- [ ] No `#[allow(dead_code)]` on new items — everything has a production caller
- [ ] Tab bar hover in both single-pane and multi-pane paths uses selective tree walks
- [ ] Overlay prepaint receives populated bounds maps in both paths
- [ ] No behavioral regressions in tab switching, tab close, tab drag, or overlay popups

**Exit Criteria:** All three main-window render paths (single-pane, multi-pane, overlays) use the same correct patterns as the dialog path. `cargo test -p oriterm` and `cargo test -p oriterm_ui` pass with 0 failures. Tab bar and overlay interactions work identically to before, with reduced tree-walk counts logged in debug output.
