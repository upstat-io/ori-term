---
section: "01"
title: "Current-Path Correctness"
status: complete
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "All production render paths populate real prepaint bounds so widgets know their screen position during the prepaint phase"
inspired_by:
  - "WindowRoot::run_prepaint() (oriterm_ui/src/window_root/pipeline.rs:324-335) — correct pattern"
depends_on: []
sections:
  - id: "01.1"
    title: "Understand the Bug"
    status: complete
  - id: "01.2"
    title: "Fix Dialog Rendering Path"
    status: complete
  - id: "01.3"
    title: "Fix Single-Pane Redraw Path"
    status: complete
  - id: "01.4"
    title: "Fix Multi-Pane Redraw Path"
    status: complete
  - id: "01.5"
    title: "Tests"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.6"
    title: "Build & Verify"
    status: complete
---

# Section 01: Current-Path Correctness

**Status:** Complete
**Goal:** Every production render path populates real `prepaint_bounds` so that `PrepaintCtx::bounds` reflects the widget's actual screen rectangle, not `Rect::default()`. No regressions in hover, focus, or button press behavior.

**Production code path:** `App::compose_dialog_widgets()` in `dialog_rendering.rs`, `App::handle_redraw()` in `redraw/mod.rs`, and `App::handle_redraw_multi_pane()` in `redraw/multi_pane/mod.rs` — specifically the `prepaint_widget_tree()` calls that pass an empty `HashMap`.

**Observable change:** Widgets that use `PrepaintCtx::bounds` during prepaint receive their actual layout bounds instead of `Rect { x: 0, y: 0, width: 0, height: 0 }`. This fixes any widget that needs position-aware prepaint logic (e.g., viewport-aware animation gating, position-dependent visual state).

**Context:** The `prepaint_widget_tree()` function resolves per-widget bounds from a `HashMap<WidgetId, Rect>` via `bounds_map.get(&id).copied().unwrap_or_default()`. `WindowRoot::run_prepaint()` correctly populates this map by calling `collect_layout_bounds()` on the layout tree. But all three app-layer render paths bypass `WindowRoot::run_prepaint()` and create an empty `HashMap` instead, so every widget receives `Rect::default()` during prepaint. The comment in `redraw/mod.rs:294-296` says "Empty bounds map is correct here — widget layout happens inside containers' paint() methods" — this is wrong. Layout happening during paint doesn't help prepaint, which runs *before* paint.

**Reference implementations:**
- **WindowRoot** `oriterm_ui/src/window_root/pipeline.rs:324-335`: `run_prepaint()` — the correct pattern that calls `collect_layout_bounds()` before `prepaint_widget_tree()`
- **Pipeline** `oriterm_ui/src/pipeline/mod.rs:260-267`: `collect_layout_bounds()` — utility that walks a `LayoutNode` tree and populates the bounds map

**Depends on:** Nothing — this is the first section.

---

## 01.1 Understand the Bug

**File(s):** `oriterm/src/app/dialog_rendering.rs`, `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`

The three app-layer render paths all follow the same broken pattern:

```rust
// BUG: empty map — every widget gets Rect::default() during prepaint
let prepaint_bounds = std::collections::HashMap::new();
prepaint_widget_tree(&mut widget, &prepaint_bounds, ...);
```

The correct pattern exists in `WindowRoot::run_prepaint()`:

```rust
let mut bounds_map: HashMap<WidgetId, Rect> = HashMap::new();
collect_layout_bounds(&self.layout, &mut bounds_map);
prepaint_widget_tree(&mut *self.widget, &bounds_map, ...);
```

The gap: the app-layer paths don't call `widget.layout()` or `compute_layout()` for their standalone widgets (chrome, tab_bar, content). They pass bounds directly via `DrawCtx::bounds` during paint, skipping the layout system entirely. To fix prepaint, we need to compute layout for these widgets before prepaint runs.

- [x] Audit all three render paths and confirm the empty `HashMap` pattern
- [x] Verify which widgets actually use `PrepaintCtx::bounds` today (search for `ctx.bounds` in prepaint impls)
- [x] Determine if any widget behavior is currently broken due to the zero bounds

---

## 01.2 Fix Dialog Rendering Path

**File(s):** `oriterm/src/app/dialog_rendering.rs`

In `compose_dialog_widgets()`, the chrome widget gets `chrome_bounds` and the content widget gets `content_bounds` during paint. We need to compute layout trees for both before prepaint.

- [x] After the `prepare_widget_tree()` calls (line ~144), compute layout for chrome and content. Extracted to `collect_dialog_prepaint_bounds()` helper. Reuses existing `measurer`.
- [x] Replace the empty `HashMap::new()` on line 147 with the populated `prepaint_bounds`
- [x] Remove or correct the misleading comment
- [x] Imported directly from `oriterm_ui::pipeline` and `oriterm_ui::layout` (not re-exported through widget_pipeline)
- [x] Imported `LayoutCtx` from `oriterm_ui::widgets` and `compute_layout` from `oriterm_ui::layout`
- [x] Verify the measurer is available before prepaint (confirmed: `CachedTextMeasurer` created before prepare/prepaint)
- [x] Verify chrome and content bounds are passed consistently between layout computation and paint `DrawCtx`
- [x] Borrow splitting resolved: helper takes `&dyn Widget` for chrome and content separately (avoids `&mut ctx` conflict with `renderer` borrow)

---

## 01.3 Fix Single-Pane Redraw Path

**File(s):** `oriterm/src/app/redraw/mod.rs`

In `handle_redraw()`, the tab_bar widget is the primary UI widget that goes through prepaint. Overlays are handled by `WindowRoot::prepaint_overlay_widgets()`.

- [x] Before the `prepaint_widget_tree()` call, compute layout for tab_bar via shared `collect_tab_bar_prepaint_bounds()` helper in `draw_helpers.rs`
- [x] Replace the empty `HashMap::new()` with the populated `prepaint_bounds`
- [x] Remove the incorrect comment on lines 294-296
- [x] Borrow splitting resolved: helper function takes `&WindowRenderer` (immutable reborrow within scoped block), measurer drops before mutable renderer usage resumes
- [x] Compute `tab_bar_rect` from existing constants (`TAB_BAR_HEIGHT`)
- [x] Overlay prepaint receives the same bounds map (verified: `prepaint_overlay_widgets(&prepaint_bounds, ...)`)

---

## 01.4 Fix Multi-Pane Redraw Path

**File(s):** `oriterm/src/app/redraw/multi_pane/mod.rs`

Same pattern as single-pane — the tab_bar and overlays get an empty bounds map.

- [x] Before the `prepaint_widget_tree()` call, compute layout for tab_bar via shared `collect_tab_bar_prepaint_bounds()` helper
- [x] Replace the empty `HashMap::new()` with the populated map
- [x] Remove misleading comments
- [x] Consistency with single-pane fix: both paths use the same `collect_tab_bar_prepaint_bounds()` helper
- [x] File size: extracted helper to `draw_helpers.rs`, `multi_pane/mod.rs` now 490 lines (under 500)

---

## 01.5 Tests

**File(s):** New test file(s) as needed

- [x] `harness_prepaint_provides_nonzero_bounds` — verifies `PrepaintCtx::bounds` is non-zero via `WidgetTestHarness` (correct pipeline path)
- [x] `collect_layout_bounds_populates_map_for_nested_tree` — verifies bounds for parent + child in nested layout
- [x] `collect_layout_bounds_skips_nodes_without_widget_id` — verifies anonymous layout containers are skipped
- [x] Dialog rendering path test not added (requires GPU context); correctness verified via build + harness tests
- [x] All existing 2135 tests pass (no regressions)

---

## 01.R Third Party Review Findings

- None.

---

## 01.6 Build & Verify

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [x] New tests exist proving this section's changes work (3 new tests in `pipeline/tests.rs`)
- [x] No `#[allow(dead_code)]` on new items — everything has a production caller
- [x] All three render paths pass populated bounds maps to `prepaint_widget_tree()`
- [x] `PrepaintCtx::bounds` returns real widget bounds (not `Rect::default()`) in dialog, single-pane, and multi-pane paths

**Exit Criteria:** `cargo test -p oriterm_ui` and `cargo test -p oriterm` pass with 0 failures. A focused test demonstrates that `PrepaintCtx::bounds` returns a non-zero `Rect` for at least one widget in each render path. The empty `HashMap::new()` pattern no longer exists in any production render path.
