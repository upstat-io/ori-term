---
reroute: true
name: "Hygiene Fixes"
full_name: "Hygiene: Last Commit (Blink Timer + Event Loop)"
status: active
order: 1
---

# Hygiene Last-Commit Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **Disposable plan** — delete directory after all fixes land.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Redraw Pipeline Consolidation
**File:** `section-01-redraw-pipeline.md` | **Status:** Complete

```
chrome rendering, draw_tab_bar, draw_overlays, draw_search_bar, draw_status_bar
single-pane, multi-pane, handle_redraw, handle_redraw_multi_pane
extract phase, snapshot refresh, swap_renderable_content, frame initialization
opacity resolution, surface_has_alpha, effective_opacity, effective_unfocused_opacity
redraw/mod.rs, redraw/multi_pane/mod.rs, redraw/draw_helpers.rs
LEAK:algorithmic-duplication, chrome duplication, extract duplication
```

---

### Section 02: Blink Timer Fixes
**File:** `section-02-blink-timers.md` | **Status:** Complete

```
cursor_blink, text_blink, CursorBlink, intensity, is_animating, next_change
blink opacity, BLINK_OPACITY_THRESHOLD, compute_blink_opacity
phase boundaries, fade_out_start, hidden_plateau_end, FADE_FRACTION
schedule_blink_wakeup, thread spawn, MuxWakeup, about_to_wait
text_blink_active, ControlFlowInput, compute_control_flow
next_toggle, dead API, event_loop_helpers/mod.rs
cursor_blink/mod.rs, redraw/mod.rs, multi_pane/mod.rs
```

---

### Section 03: Render Dispatch Consolidation
**File:** `section-03-render-dispatch.md` | **Status:** Not Started

```
render_dirty_windows, modal_loop_render, scratch_dirty_windows
windows loop, dialogs loop, save focus, restore focus
any_dirty, still_dirty, is_any_dirty
maybe_shrink, chrome_scene, scene, dialog scene shrink
render_dispatch.rs, event_loop_helpers/mod.rs, event_loop.rs
```

---

### Section 04: Scattered Knowledge and Bloat
**File:** `section-04-scattered-and-bloat.md` | **Status:** Not Started

```
apply_theme, tab_bar.apply_theme, status_bar.apply_theme
blinking_active, config.terminal.cursor_blink, CURSOR_BLINKING
config_reload/mod.rs, window_management.rs, tab_bar_input/mod.rs
BLOAT, 500-line limit, file splitting
dead_code, move_tab_to_new_window, render_strategy, damage
```

---

### Section 05: Cleanup
**File:** `section-05-cleanup.md` | **Status:** Not Started

```
test-all, clippy-all, build-all, plan deletion
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Redraw Pipeline Consolidation | `section-01-redraw-pipeline.md` |
| 02 | Blink Timer Fixes | `section-02-blink-timers.md` |
| 03 | Render Dispatch Consolidation | `section-03-render-dispatch.md` |
| 04 | Scattered Knowledge and Bloat | `section-04-scattered-and-bloat.md` |
| 05 | Cleanup | `section-05-cleanup.md` |
