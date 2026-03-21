---
reroute: true
name: "Incremental Rendering"
full_name: "Incremental Rendering"
status: active
order: 2
---

# Incremental Rendering Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Current-Path Correctness
**File:** `section-01-current-path-correctness.md` | **Status:** Complete

```
prepaint, bounds, PrepaintCtx, collect_layout_bounds, layout bounds
empty HashMap, Rect::default, prepaint_bounds, bounds_map
dialog_rendering.rs, redraw/mod.rs, redraw/multi_pane/mod.rs
compose_dialog_widgets, handle_redraw, handle_redraw_multi_pane
LayoutNode, LayoutBox, compute_layout, widget.layout()
WindowRoot::run_prepaint, pipeline/mod.rs
```

---

### Section 02: Dialog Quick Wins
**File:** `section-02-dialog-quick-wins.md` | **Status:** Complete

```
viewport culling, off-screen, clip rect, intersection
FormLayout, FormSection, Container, ScrollWidget, current_clip_in_content_space
layout cache, cached_child_layout, redundant layout, stale cache, page switch
dialog rendering, settings panel, page switching, PageContainerWidget
scene.clear, full rebuild, content_bounds
DirtyKind, phase gating, Paint vs Prepaint
prepare_widget_tree full walk, inactive pages traversed
for_each_child_mut, for_each_child_mut_all, active page only, hidden page waste
Widget trait method, register_widget_tree, collect_key_contexts (for_each_child_mut_all)
dispatch_keymap_action, dispatch_to_widget_tree, collect_focusable_ids (for_each_child_mut — active only)
reset_scroll, cached_child_layout invalidation
WindowRoot::compute_layout, WindowRoot::rebuild, WidgetTestHarness
02.2 scroll cache, 02.2b trait method, 02.2c pipeline callers, 02.2d prepare skip
```

---

### Section 03: Dialog Selective Walks
**File:** `section-03-dialog-selective-walks.md` | **Status:** Not Started

```
selective tree walk, prepare_widget_tree, prepaint_widget_tree
subtree skip, dirty subtree, clean subtree
InvalidationTracker, max_dirty_kind, per-widget dirty, mark() unused in production
hover, focus, page-local, proportional work
for_each_child_mut, depth-first, tree traversal
InteractionManager, lifecycle pipeline, dirty marking
set_hot, clear_hot, set_active, clear_active, focus change
VisualStateAnimator, is_animating, animation-driven dirty
parent map, subtree dirty query, ancestor tracking
signature change sync points, all callers must update
pipeline/tree_walk.rs, mandatory extraction, file size 500-line limit
```

---

### Section 04: Main-Window Rollout
**File:** `section-04-main-window-rollout.md` | **Status:** Not Started

```
tab_bar, chrome, overlay, multi-pane, single-pane
handle_redraw, handle_redraw_multi_pane, draw_tab_bar
prepare_overlay_widgets, prepaint_overlay_widgets
rollout, same strategy, proven pattern
WindowContext, chrome_scene, tab bar animation
overlay layout timing, layout_overlays, anchor bounds
register_widget_tree, widget registration, page switch
```

---

### Section 05: Verification & Measurement
**File:** `section-05-verification.md` | **Status:** Not Started

```
profile, measurement, frame time, CPU cost
retained scene, GPU scroll, advanced rendering
before/after, regression, performance validation
test matrix, behavioral equivalence, visual regression
dialog scroll, hover cost, tree walk count
Scene::len(), primitive count, widget visit count
page switch cost, idle CPU, ControlFlow::Wait
go/no-go, decision criteria, follow-up plan
log::debug!, measurement logging, feature flag
```

---

## Performance Validation

Use profiling after modifying hot paths.

**When to benchmark:** Sections 02, 03, 04 (each modifies render-loop work)
**Skip benchmarks for:** Section 01 (correctness fix, not perf)
**Cumulative measurement:** Section 05 (measures combined impact of all prior sections)

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Current-Path Correctness | `section-01-current-path-correctness.md` |
| 02 | Dialog Quick Wins | `section-02-dialog-quick-wins.md` |
| 03 | Dialog Selective Walks | `section-03-dialog-selective-walks.md` |
| 04 | Main-Window Rollout | `section-04-main-window-rollout.md` |
| 05 | Verification & Measurement | `section-05-verification.md` |
