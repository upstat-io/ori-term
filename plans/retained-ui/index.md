---
reroute: true
name: "Retained UI"
full_name: "Retained UI Framework"
status: resolved
order: 1
---

# Retained UI Framework Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Text/Layout Caching
**File:** `section-01-text-cache.md` | **Status:** Complete

```
text cache, shape cache, text shaping, font cache, measure cache
CachedTextMeasurer, TextMeasurer, UiFontMeasurer, TextShapeCache
TextCacheKey, TextMetrics, ShapedText, TextStyle
ui_text::shape_text, ui_text::measure_text_styled
oriterm/src/font/shaper/ui_measurer.rs, cached_measurer.rs (new), mod.rs (register)
cache key, generation, invalidation, hit rate, font reload
```

---

### Section 02: Subtree Invalidation
**File:** `section-02-subtree-invalidation.md` | **Status:** Complete

```
dirty tracking, invalidation, subtree dirty, per-widget dirty
DirtyKind, InvalidationTracker, RequestPaint, RequestLayout
EventResponse, WidgetResponse, WidgetResponse::source
ContainerWidget::update_dirty, DirtyKind::merge
paint dirty, layout dirty, full invalidation, dirty propagation
oriterm_ui/src/invalidation/mod.rs (new), invalidation/tests.rs (new), lib.rs (register)
container/event_dispatch.rs, overlay/manager/event_routing.rs
OverlayManager::process_mouse_event, widgets/mod.rs
```

---

### Section 03: Scene Retention
**File:** `section-03-scene-retention.md` | **Status:** Complete

```
scene node, scene cache, retained scene, draw list cache
SceneNode, compose_scene, per-widget cache, scene composition
DrawCommand, DrawList, selective rebuild, cached draw commands
ContainerWidget::draw, render_dialog, draw_tab_bar
draw/scene_node.rs (new), draw/scene_compose.rs (new), draw/mod.rs (register), draw_list.rs
clip stack, layer stack, cached subtree, replay commands
```

---

### Section 04: Scroll Transform
**File:** `section-04-scroll-transform.md` | **Status:** Complete

```
scroll transform, viewport offset, translate transform
PushTranslate, PopTranslate, ScrollWidget, scroll_offset
scroll without redraw, viewport transform, content stable
scroll/mod.rs, draw_list.rs, gpu/draw_list_convert/mod.rs
GPU converter, transform stack, scroll composition
```

---

### Section 05: Surface Strategy
**File:** `section-05-surface-strategy.md` | **Status:** Complete

```
surface strategy, render strategy, damage kind, surface host
RenderStrategy, TerminalCached, UiRetained, Transient
DamageKind, DamageSet, SurfaceHost, damage tracking
WindowContext, DialogWindowContext, dirty flag, ui_stale, chrome_draw_list
surface/mod.rs (new), window_context.rs, dialog_context/mod.rs
oriterm_ui/src/lib.rs (add pub mod surface)
```

---

### Section 06: Window Lifecycle
**File:** `section-06-window-lifecycle.md` | **Status:** Complete

```
window lifecycle, surface lifecycle, visibility, show/hide
SurfaceLifecycle, CreatedHidden, Primed, Visible, Closing, Destroyed
dialog open, dialog close, flash suppression, modal cleanup
finalize_dialog, close_dialog, set_visible, DWM transitions
dialog_management.rs, event_loop.rs
```

---

### Section 07: Verification
**File:** `section-07-verification.md` | **Status:** Complete (runtime items deferred)

```
verification, test matrix, behavioral equivalence, performance
frame time, draw call count, cache hit rate, memory bounded
widget test, scroll test, overlay test, chrome test
visual regression, equivalence test, benchmark
```

---

## Performance Validation

Use profiling after modifying hot paths.

**When to benchmark:** Sections 01, 02, 03, 04, 07
**Skip benchmarks for:** Sections 05, 06 (pure abstractions, no render changes)

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Text/Layout Caching | `section-01-text-cache.md` |
| 02 | Subtree Invalidation | `section-02-subtree-invalidation.md` |
| 03 | Scene Retention | `section-03-scene-retention.md` |
| 04 | Scroll Transform | `section-04-scroll-transform.md` |
| 05 | Surface Strategy | `section-05-surface-strategy.md` |
| 06 | Window Lifecycle | `section-06-window-lifecycle.md` |
| 07 | Verification | `section-07-verification.md` |
