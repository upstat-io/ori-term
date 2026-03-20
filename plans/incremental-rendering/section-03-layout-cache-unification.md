---
section: "03"
title: "Layout Cache Unification"
status: not-started
reviewed: true
goal: "Eliminate redundant layout recomputation by coordinating the 4 independent layout caches (DialogWindowContext, SettingsPanel, ContainerWidget, ScrollWidget) into a single canonical layout with explicit invalidation"
depends_on: []
sections:
  - id: "03.1"
    title: "Cache Inventory & Audit"
    status: not-started
  - id: "03.2"
    title: "Scroll-Stable Layout Cache"
    status: not-started
  - id: "03.3"
    title: "Structural Invalidation Protocol"
    status: not-started
  - id: "03.4"
    title: "Tests"
    status: not-started
  - id: "03.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Layout Cache Unification

**Status:** Not Started
**Goal:** Layout is computed once per structural change (window resize, page switch, widget add/remove) and reused across all consumers (hit testing, painting, prepaint bounds). Scroll offset changes do NOT trigger layout recomputation.

**Context:** Four independent layout caches exist, each with its own invalidation logic:

1. `DialogWindowContext.cached_layout` — `Option<(Rect, Rc<LayoutNode>)>`, invalidated on resize (`= None`), page switch
2. `SettingsPanel.cached_layout` — `RefCell<Option<(Rect, Rc<LayoutNode>)>>`, keyed on bounds, invalidated via `invalidate_cache()` which sets it to `None`
3. `ContainerWidget.cached_layout` — `RefCell<Option<(Rect, Rc<LayoutNode>)>>`, keyed on bounds + `needs_layout` flag (skips cache when `needs_layout == true`)
4. `ScrollWidget.cached_child_layout` — `RefCell<Option<(Rect, Rc<LayoutNode>)>>`, keyed on viewport bounds

When any cache misses, it recomputes layout from scratch — walking the widget tree, measuring text, running the flex/grid solver. On scroll, `DialogWindowContext.cached_layout` is now correctly NOT invalidated (the layout structure and sizes are stable). The SettingsPanel's cache also hits correctly (keyed on bounds, not scroll offset). The primary waste is **redundant caching at multiple layers** — `DialogWindowContext.cached_layout` and `SettingsPanel.cached_layout` both cache the full layout tree independently, and `ContainerWidget` caches its subtree on top of that.

**Reference implementations:**
- **Flutter** `rendering/object.dart`: Single `RenderObject.layout()` call per frame, cached via `_needsLayout` flag. Relayout boundary prevents propagation.
- **GPUI (Zed)** `crates/gpui/src/window.rs`: Layout is computed once in `layout()`, cached in `LayoutEngine`, reused by paint and hit test.

**Depends on:** None.

---

## 03.1 Cache Inventory & Audit

Audit all 4 caches to understand their invalidation triggers and recomputation cost.

- [ ] Document each cache's key, invalidation triggers, and recomputation cost:
  | Cache | Key | Invalidated By | Cost |
  |-------|-----|---------------|------|
  | DialogWindowContext.cached_layout | Rect (viewport) | resize (`= None`), page switch (`= None`), DPI (`resize_surface`) | Full tree layout |
  | SettingsPanel.cached_layout | Rect (bounds) | explicit `invalidate_cache()` (sets to `None`), bounds change | Full panel layout |
  | ContainerWidget.cached_layout | Rect (bounds) + `needs_layout` flag | `needs_layout` flag set, bounds change | Container subtree |
  | ScrollWidget.cached_child_layout | Rect (viewport) | viewport bounds change (NOT scroll offset) | Child natural size |

- [ ] Identify which caches are redundant (same data recomputed at different layers)
- [ ] Identify which invalidation triggers are over-aggressive (invalidating when not needed)

---

## 03.2 Scroll-Stable Layout Cache

**File(s):** `oriterm_ui/src/widgets/scroll/mod.rs`, `oriterm_ui/src/widgets/scroll/rendering.rs`

The scroll widget's `layout()` method includes `content_offset` in the LayoutBox. Since `content_offset` changes on every scroll event, any cache keyed on the LayoutBox value would miss. But `content_offset` doesn't affect layout SIZE — it only affects rendering position.

- [ ] Separate `content_offset` from the layout computation:
  - Layout produces structure + sizes (stable across scroll)
  - Paint reads `self.scroll_offset` directly for offset (already does this)
  - Hit testing reads `content_offset` from the LayoutNode (needs update on scroll)

- [ ] Make ScrollWidget's `cached_child_layout` truly stable across scroll:
  - Key on viewport bounds (already done)
  - Don't invalidate on scroll_offset change (already the case)
  - Verify `child_natural_size()` is called only when bounds change, not per-frame

- [ ] Remove `content_offset` from the layout cache key if it's causing misses. The hit test function `layout_hit_test_path` uses `content_offset` from the LayoutNode, so the cached layout tree needs the correct offset — but this can be updated in-place rather than recomputing the entire tree.

---

## 03.3 Structural Invalidation Protocol

**File(s):** `oriterm_ui/src/widgets/container/layout_build.rs`, `oriterm_ui/src/widgets/settings_panel/mod.rs`

Define when layout MUST be recomputed (structural changes) vs. when it can be reused (scroll, hover, visual state changes).

- [ ] Define a `LayoutGeneration` counter on `WindowRoot`:
  ```rust
  /// Monotonically increasing generation counter. Bumped on structural
  /// changes (page switch, widget add/remove, resize). Layout caches
  /// check their generation against this — stale caches recompute.
  layout_generation: u64,
  ```

- [ ] Each layout cache stores the generation it was computed at:
  ```rust
  cached_layout: RefCell<Option<(u64, Rect, Rc<LayoutNode>)>>,
  //                       ^^^ generation
  ```

- [ ] Cache hit = same generation + same bounds → return cached
- [ ] Cache miss = different generation OR different bounds → recompute + update generation

- [ ] Bump generation on:
  - Window/dialog resize
  - Page switch (PageContainerWidget.accept_action)
  - Widget add/remove (structural tree change)
  - DPI change

- [ ] Do NOT bump generation on:
  - Scroll offset change
  - Hover state change
  - Visual state animation tick
  - Focus change

- [ ] Add `layout_generation: u64` field to `PrepaintCtx`. Set it from `WindowRoot.layout_generation` at the `prepaint_widget_tree()` call sites. Widgets that cache layout (SettingsPanel, ContainerWidget, ScrollWidget) read `ctx.layout_generation` in their `prepaint()` and store it on `self.layout_generation: u64`.

- [ ] **DialogWindowContext.cached_layout is a SEPARATE cache** from the 4 widget-level caches. It lives at `oriterm/src/app/dialog_context/mod.rs:82` and is `Option<(Rect, Rc<LayoutNode>)>`. It must also adopt the generation counter. Since DialogWindowContext is in the `oriterm` crate (not `oriterm_ui`), the generation counter must be accessible via `WindowRoot` (which `DialogWindowContext` already owns as `ctx.root`).

- [ ] **Generation counter accessibility during paint:** `SettingsPanel::paint()` and `ContainerWidget::paint()` call `get_or_compute_layout()` with only `&self` + `DrawCtx`. Options:
  (a) Add `layout_generation: u64` field to `DrawCtx` (propagated from WindowRoot). Requires updating all 8+ `DrawCtx` construction sites across `window_root/pipeline.rs`, `dialog_rendering.rs`, `testing/harness.rs`, and any widget that builds child contexts.
  (b) Set generation on each widget during prepaint (via `PrepaintCtx`). No `DrawCtx` API change — widgets store generation on `self` during prepaint, check it during paint.
  (c) Store generation on `InvalidationTracker` and pass it through `DrawCtx`.
  **Recommended: Option (b)** — avoids `DrawCtx` API churn. Widgets that cache layout (SettingsPanel, ContainerWidget, ScrollWidget) already have a `prepaint()` hook where the generation can be injected. Add a `layout_generation: u64` field to each caching widget, set it from `PrepaintCtx` during prepaint.

---

## 03.3b Pre-existing Bug: Empty prepaint_bounds

**Priority: Fix first.** This is a correctness bug (widgets get `Rect::default()` during prepaint), not just a performance issue. Per the Broken Window Policy, fix this before or at the start of Section 03 implementation.

**Affected files:**
- `oriterm/src/app/dialog_rendering.rs`, line 147
- `oriterm/src/app/redraw/mod.rs`, line 297
- `oriterm/src/app/redraw/multi_pane/mod.rs`, line 375

All three sites create an **empty** `HashMap` for prepaint bounds:
```rust
let prepaint_bounds = std::collections::HashMap::new();
```
This means `prepaint_widget_tree()` receives an empty bounds map, so every widget gets `Rect::default()` for its bounds during prepaint. This defeats bounds-dependent visual state resolution (e.g., a widget that adapts its visual state based on its size).

- [ ] Fix `dialog_rendering.rs`: populate `prepaint_bounds` from the layout tree by calling `collect_layout_bounds(&layout, &mut prepaint_bounds)` using `DialogWindowContext.cached_layout`
- [ ] Fix `redraw/mod.rs`: same pattern for tab bar prepaint bounds
- [ ] Fix `redraw/multi_pane/mod.rs`: same pattern for multi-pane tab bar prepaint bounds
- [ ] Consider routing chrome and content through `WindowRoot.compute_layout()` + `WindowRoot.run_prepaint()` instead of doing it manually. This would eliminate the duplication and ensure bounds are always populated.

---

## 03.4 Tests

- [ ] Test: `layout_cache_stable_across_scroll` — compute layout, change scroll_offset, verify layout cache still valid (no recomputation)
- [ ] Test: `layout_cache_invalidated_on_page_switch` — compute layout, switch page, verify cache miss
- [ ] Test: `layout_cache_invalidated_on_resize` — compute layout, change viewport, verify cache miss
- [ ] Test: `generation_counter_increments_on_structural_change` — verify generation bumps correctly
- [ ] Test: `generation_counter_stable_on_scroll` — verify generation stays same on scroll
- [ ] Test: `prepaint_bounds_populated_from_layout` — verify that prepaint receives correct per-widget bounds (not empty/default) in all three call sites (dialog, main window, multi-pane)

---

## 03.5 Completion Checklist

- [ ] All 5 layout caches use generation-based invalidation (4 widget-level + DialogWindowContext)
- [ ] Scroll offset changes do NOT trigger layout recomputation at any layer
- [ ] Structural changes (page switch, resize, DPI) trigger layout recomputation
- [ ] `layout_generation` counter exists on WindowRoot and is bumped correctly
- [ ] Generation counter accessible during paint via widget field set in `prepaint()`
- [ ] `prepaint_bounds` populated from layout tree in all 3 affected files (empty HashMap bug fixed)
- [ ] No regressions -- `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** Scrolling 100 times in the Colors page triggers 0 layout recomputations (verified by a counter or log). Switching pages triggers exactly 1 layout recomputation.
