---
reroute: true
name: "Incremental Rendering"
full_name: "Incremental Rendering Pipeline"
status: queued
order: 2
---

# Incremental Rendering Pipeline Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **References:** `plans/ui-framework-overhaul/`, GPU scene pipeline research

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

### Section 01: Viewport Culling Verification
**File:** `section-01-viewport-culling.md` | **Status:** Not Started

```
cull, culling, viewport, off_screen, visible_bounds, clip_rect, scroll_content
paint_skip, intersects, clip_in_content_space, child_rect, form_field
ScrollWidget, ContainerWidget, FormLayout, FormSection, draw_impl, paint, content_mask
visibility_check, spatial_skip, frustum_cull, early_out
current_clip_in_content_space, push_clip, push_offset, coordinate_space
oriterm_ui/src/widgets/scroll/rendering.rs, oriterm_ui/src/widgets/container/mod.rs
oriterm_ui/src/widgets/form_layout/mod.rs, oriterm_ui/src/widgets/form_section/mod.rs
```

---

### Section 02: Selective Tree Walks
**File:** `section-02-selective-tree-walks.md` | **Status:** Not Started

```
prepaint, prepare, tree_walk, skip_clean, dirty_subtree, per_widget_dirty
prepare_widget_tree, prepaint_widget_tree, for_each_child_mut
lifecycle, anim_frame, visual_state, VisualStateAnimator
InvalidationTracker, DirtyKind, Clean, Paint, Prepaint, Layout
dirty_map, dirty_set, skip_clean_subtrees, O(dirty), ancestors_dirty
full_invalidation, needs_full_rebuild, should_recurse, propagate_dirty
borrow_split, WindowRoot, prepare_overlay_widgets, prepaint_overlay_widgets
module_split, pipeline/prepare.rs, pipeline/prepaint.rs
oriterm_ui/src/pipeline/, oriterm_ui/src/invalidation/
oriterm_ui/src/window_root/pipeline.rs, oriterm_ui/src/window_root/borrow_split.rs
oriterm/src/app/dialog_rendering.rs, oriterm/src/app/redraw/mod.rs
oriterm/src/app/redraw/multi_pane/mod.rs
```

---

### Section 03: Layout Cache Unification
**File:** `section-03-layout-cache-unification.md` | **Status:** Not Started

```
cached_layout, layout_cache, RefCell, compute_layout, get_or_compute_layout
content_offset, scroll_offset, stale_cache, invalidation_trigger
SettingsPanel, ContainerWidget, ScrollWidget, DialogWindowContext
needs_layout, cache_key, bounds_key, structural_change
layout_tree, LayoutNode, LayoutBox, compute_layout, layout_generation
prepaint_bounds, collect_layout_bounds, empty_bounds_bug
DrawCtx, PrepaintCtx, generation_counter
oriterm_ui/src/widgets/container/layout_build.rs, oriterm_ui/src/widgets/settings_panel/mod.rs
oriterm_ui/src/widgets/contexts.rs
oriterm/src/app/dialog_context/mod.rs, oriterm/src/app/dialog_rendering.rs
oriterm/src/app/redraw/mod.rs, oriterm/src/app/redraw/multi_pane/mod.rs
```

---

### Section 04: Retained Scene & Dirty Regions
**File:** `section-04-retained-scene.md` | **Status:** Not Started

```
scene, retained, immediate_mode, clear, rebuild, dirty_region
Scene, push_quad, push_text, push_clip, push_offset
per_widget_scene, fragment_cache, scene_patch, partial_rebuild
DamageTracker, damage_rect, dirty_rect, hash_primitives
content_mask, ContentMask, clip_stack, offset_stack
SceneFragment, begin_fragment, end_fragment, CachedFragment
paint_child, DrawCtx, for_child, absolute_position, position_dependent
build_scene, scene_clear, fragment_hierarchy, leaf_only_caching
oriterm_ui/src/draw/scene/, oriterm_ui/src/draw/scene/fragment.rs
oriterm_ui/src/draw/fragment_cache.rs, oriterm_ui/src/draw/mod.rs
oriterm_ui/src/widgets/contexts.rs
oriterm/src/app/dialog_rendering.rs
```

---

### Section 05: GPU-Side Scroll
**File:** `section-05-gpu-scroll.md` | **Status:** Not Started

```
offscreen_texture, render_target, texture_blit, scroll_texture
wgpu, TextureView, RenderPass, copy_texture_to_texture
scroll_strip, reveal_strip, incremental_blit, content_texture
frame_buffer, FBO, offscreen_render, texture_cache
gpu_scroll, blit_offset, texture_viewport, content_surface
cross_platform, texture_format, DPI_scale, physical_pixels
overlay_compositing, ring_buffer, page_switch_invalidation
scroll_composite, crate_boundary, headless_testing
oriterm/src/gpu/, oriterm/src/gpu/window_renderer/, oriterm/src/gpu/pipeline/
oriterm/src/gpu/scroll_composite/
```

---

### Section 06: Dialog Rendering Integration
**File:** `section-06-dialog-integration.md` | **Status:** Not Started

```
compose_dialog_widgets, render_dialog, build_scene, paint
dialog_rendering, frame_budget, urgent_redraw, request_redraw
cursor_move, mouse_move, hit_test, hot_path, interaction
prepare_widget_tree, prepaint_widget_tree, render_to_surface
event_coalescing, frame_coalescing, scroll_batch
render_dialog_overlays, overlay_rendering, scratch_scene
chrome_content_separation, WindowChromeWidget, SettingsPanel
oriterm/src/app/dialog_rendering.rs, oriterm/src/app/dialog_context/
oriterm/src/app/dialog_context/content_actions.rs
oriterm/src/app/render_dispatch.rs, oriterm/src/app/event_loop.rs
```

---

### Section 07: Verification & Benchmarks
**File:** `section-07-verification.md` | **Status:** Not Started

```
performance, benchmark, frame_time, scroll_fps, idle_cpu
allocation, hot_path, regression, visual_regression
test_matrix, harness_test, scroll_test, damage_test
profiling, perf, flamegraph, tracing, timing
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Viewport Culling Verification | `section-01-viewport-culling.md` |
| 02 | Selective Tree Walks | `section-02-selective-tree-walks.md` |
| 03 | Layout Cache Unification | `section-03-layout-cache-unification.md` |
| 04 | Retained Scene & Dirty Regions | `section-04-retained-scene.md` |
| 05 | GPU-Side Scroll | `section-05-gpu-scroll.md` |
| 06 | Dialog Rendering Integration | `section-06-dialog-integration.md` |
| 07 | Verification & Benchmarks | `section-07-verification.md` |
