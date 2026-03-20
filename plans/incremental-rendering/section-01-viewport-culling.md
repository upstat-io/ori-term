---
section: "01"
title: "Viewport Culling Verification"
status: not-started
reviewed: true
goal: "Verify that the existing viewport culling in ContainerWidget, FormLayout, and FormSection correctly skips off-screen children during scroll, and harden with tests"
inspired_by:
  - "Ratatui Buffer-based rendering (only renders visible cells)"
  - "egui clip_rect culling (skip painting outside clip)"
depends_on: []
sections:
  - id: "01.1"
    title: "Verify Existing Culling Pipeline"
    status: not-started
  - id: "01.2"
    title: "Edge Case Hardening"
    status: not-started
  - id: "01.3"
    title: "Culling Tests"
    status: not-started
  - id: "01.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Viewport Culling Verification

**Status:** Not Started
**Goal:** Verify that the existing viewport culling infrastructure works correctly end-to-end and add test coverage. ContainerWidget (line 326), FormLayout (line 161), and FormSection (line 198) already call `current_clip_in_content_space()` and skip children whose `child_node.rect` does not intersect `visible_bounds`. ScrollWidget (rendering.rs) pushes clip + offset before painting its child. This section proves the culling works with concrete tests and fixes any edge cases.

**Context:** Viewport culling is already implemented across the paint pipeline. ScrollWidget's `draw_impl()` calls `push_clip(ctx.bounds)` followed by `push_offset(-scroll_x, -scroll_y)`, which means `current_clip_in_content_space()` returns the viewport clip in content coordinates. ContainerWidget, FormLayout, and FormSection all check `child_node.rect.intersects(visible_bounds)` and skip children outside. The missing piece is verification: no test confirms this actually reduces Scene primitive counts during scroll.

**Reference implementations:**
- **egui** `epaint/src/tessellator.rs`: `coarse_tessellation_culling` checks `clip_rect.intersects(mesh.calc_bounds())` before tessellating shapes.
- **Ratatui** `src/buffer.rs`: Only processes cells within the visible terminal area.

**Depends on:** None (independent, lowest effort).

---

## 01.1 Verify Existing Culling Pipeline

**File(s):** `oriterm_ui/src/widgets/scroll/rendering.rs`, `oriterm_ui/src/widgets/container/mod.rs`, `oriterm_ui/src/widgets/form_layout/mod.rs`, `oriterm_ui/src/widgets/form_section/mod.rs`

The culling pipeline already exists end-to-end:
1. `ScrollWidget::draw_impl()` calls `push_clip(ctx.bounds)` then `push_offset(-scroll_x, -scroll_y)`.
2. `ContainerWidget::paint()` (line 326) calls `current_clip_in_content_space()`, intersects with `ctx.bounds`, and skips children where `!child_node.rect.intersects(visible_bounds)`.
3. `FormLayout::paint()` (line 161) does the same check per form section.
4. `FormSection::paint()` (line 198) does the same check per row.

The primary task is to verify this works correctly with a concrete test.

- [ ] Trace the coordinate transform chain manually for a concrete case: ScrollWidget with bounds `(0, 56, 400, 500)`, scroll_offset `200`. After `push_clip((0, 56, 400, 500))` + `push_offset(0, -200)`, verify `current_clip_in_content_space()` returns `(0, 256, 400, 500)` (the viewport shifted into content space). A child at content position `(0, 0, 400, 50)` should NOT intersect. A child at `(0, 300, 400, 50)` SHOULD intersect.

- [ ] Verify the `visible_bounds` fallback: when `current_clip_in_content_space()` returns `None` (no clip active), ContainerWidget falls back to `ctx.bounds`. Verify this is correct when painting without a ScrollWidget ancestor (the container paints everything, which is expected).

- [ ] Write a test that creates a ScrollWidget with 20 label children (each 50px tall, total 1000px) in a 200px viewport, renders at scroll_offset=0, and counts Scene text runs. Expect ~4-5 text runs, not 20.

---

## 01.2 Edge Case Hardening

**File(s):** `oriterm_ui/src/draw/scene/stacks.rs`, `oriterm_ui/src/widgets/container/mod.rs`

Verify correctness of `current_clip_in_content_space()` for edge cases and consider consolidating the duplicated culling pattern.

- [ ] Verify `current_clip_in_content_space()` correctness for nested scroll containers: if a ScrollWidget is nested inside another ScrollWidget, `cumulative_offset` includes both scroll offsets. The clip rect is the intersection of both viewports. Verify the resulting content-space clip correctly represents the visible area at the innermost content level.

- [ ] Verify boundary behavior: a child whose rect exactly touches (shares an edge with) `visible_bounds` should still paint (half-open intervals allow edge-adjacent rendering). Confirm `Rect::intersects()` returns `true` for touching rects.

- [ ] Evaluate extracting a `should_paint_child(child_rect, visible_bounds) -> bool` helper to consolidate the identical culling check that appears in ContainerWidget (line 335), FormLayout (line 166), and FormSection (line 213). If the logic is identical, a shared function reduces the chance of future divergence.

---

## 01.3 Culling Tests

**File(s):** `oriterm_ui/src/widgets/scroll/tests.rs`, `oriterm_ui/src/widgets/container/tests.rs`

- [ ] Test: `scroll_culling_paints_only_visible_children` — create a ScrollWidget with 10 label children, each 50px tall (total 500px), in a 200px viewport. Render via WidgetTestHarness and count Scene text runs:
  - At scroll offset 0: expect ~4 text runs (children 0-3 visible), not 10
  - At scroll offset 200: expect ~4 text runs (children 4-7 visible)
  - At scroll offset 300: expect ~4 text runs (children 6-9 visible)

- [ ] Test: `container_skips_offscreen_children` — create a column container taller than the viewport, push a clip rect smaller than the container, paint, verify children outside clip produce zero Scene primitives.

- [ ] Test: `culling_does_not_break_scrollbar` — render a scrollable container, verify the scrollbar thumb quad is present in the Scene regardless of scroll position.

- [ ] Test: `stacks_test_clip_in_content_space` — unit test for `current_clip_in_content_space()` in `scene/stacks.rs`: push_clip with viewport rect, push_offset with scroll offset, verify returned rect equals the viewport shifted into content coordinates.

---

## 01.4 Completion Checklist

- [ ] Manual coordinate transform trace confirms `current_clip_in_content_space()` correctness
- [ ] Existing culling in ContainerWidget, FormLayout, FormSection confirmed working via test
- [ ] `current_clip_in_content_space()` has a direct unit test in `scene/tests.rs`
- [ ] `scroll_culling_paints_only_visible_children` test passes with correct primitive counts
- [ ] Scrollbar always renders regardless of scroll position (not culled)
- [ ] Edge case: boundary children (partially visible) still paint
- [ ] No visual regressions -- `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** A 20-child scroll container at scroll_offset=0 produces ~5 text runs in the Scene (not 20). Verified via WidgetTestHarness render + Scene primitive count inspection.
