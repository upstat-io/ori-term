---
reroute: true
name: "Hygiene 40-Commits"
full_name: "Implementation Hygiene Fixes from 40-Commit Review"
status: resolved
order: 10
---

# Hygiene 40-Commits Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: oriterm_core — Term/Grid/Image Boundaries
**File:** `section-01-oriterm-core.md` | **Status:** Complete

```
term, grid, image, handler, snapshot, renderable, cell, scrollback
expect, panic, unwrap, alt_screen, alt_grid, leak
DisplayEraseMode, scrollback clearing, erase_scrollback, stub
pub, pub(crate), exposure, visibility, doc(hidden), seen_image_ids
ImageData, ImageCache, placed_id_set, eviction
zerowidth, combining marks, Vec::new, clone, allocation, hot path
handler/mod.rs, term/mod.rs, editing/mod.rs, snapshot.rs
wide_char, fix_wide_boundaries, clear_wide_char_at
named_private_mode_flag, apply_decset, decset, drift, sync
kitty, path traversal, d=y, StableRowIndex
cwd_short_path, shell_state, osc, dynamic_color, prefix
```

---

### Section 02: oriterm_mux — Server/Protocol/PTY Boundaries
**File:** `section-02-oriterm-mux.md` | **Status:** Complete

```
mux, server, protocol, pty, pane, domain, client, backend
inject, shell_integration, set_common_env, spawn, duplicate
snapshot, icon_name, cwd, allocation, per-cycle
DomainId, default_domain, hardcoded, off-by-one
close_pane, pane map, two-phase removal
trailing_edge_flush, Vec<PaneId>, scratch buffer
disconnect_client, sync_subscriptions, HashSet, O(n*m)
FrameWriter, FrameReader, ProtocolCodec, buffer, shrink
CommandComplete, ClipboardStore, ClipboardLoad, IPC, daemon
WireColor, dead code, SAFETY comment, non-unsafe
fcntl, macOS, return value, unchecked
PaneBell, bell flag, event_pump
pane_cwd, clone, String, borrow
```

---

### Section 03: oriterm/app — Event Loop/Input/Redraw
**File:** `section-03-oriterm-app.md` | **Status:** Complete

```
app, event_loop, input, redraw, mouse, keyboard, tab_bar
expect, panic, handle_mouse_press, handle_mouse_drag, mark_mode
EventLoopProxy, TermEvent, Arc, dyn Fn, callback, exposure
render_dirty_windows, Vec<WindowId>, scratch, allocation, frame
modal_loop_render, multi_pane, HashMap, SmallVec
update_tab_bar_hover, clone, layout, cursor move
SelectAll, overlay_dispatch, action_dispatch, duplicate, drift
tab_drag, merge, compute_drop_index, find_merge_target, platform
try_overlay_mouse, boilerplate, helper, extract
cfg, function parameters, inline, platform helpers
constructors, new, new_daemon, struct literal, shared init
WIDE_CHAR_SPACER_BIT, WRAP_BIT, CellFlags, magic constant
delimiter_class, snapshot_grid, duplicate
event_loop.rs, bloat, render_dispatch
```

---

### Section 04: oriterm/gpu — Render Pipeline
**File:** `section-04-oriterm-gpu.md` | **Status:** Complete

```
gpu, render, pipeline, atlas, prepare, extract, window_renderer
clear_surface, sRGB, srgb_to_linear, color, wrong
reserve, total_cells, incorrect, logic
overlay_scratch_clips, clone, mem::take, allocation
evict_over_limit, O(n^2), sort, evict_unused, HashMap::retain
UI_RECT_ATTRS, INSTANCE_ATTRS, duplicate, concatenation
window_focused, stale, reset, extract_frame
FrameSearch, from_snapshot, from_snapshot_into, allocation
ShapedFrame, maybe_shrink, buffer discipline
maybe_shrink_vec, duplicate, shared utility
new_ui_only, atlas creation, duplicate, helper
build_dirty_set, Vec<bool>, per-frame, accept &mut
zerowidth, clone, snapshot conversion, per-cell
PreparedFrame, pub, pub(crate), writer fields
SavedTerminalTier, BufferLengths, pub, pub(super)
pipeline/mod.rs, bloat, create_atlas_pipeline
upload_instance_buffers, repetition, array iteration
emit_cell, too_many_lines, suppression
```

---

### Section 05: oriterm_ui — Widget Tree
**File:** `section-05-oriterm-ui.md` | **Status:** Complete

```
ui, widget, overlay, toggle, button, slider, dropdown, menu
EventResponse, RequestRedraw, dead variant, remove
WidgetResponse, redraw, backward-compat, alias, replace
Instant::now, EventCtx, now parameter, toggle, button, controls
tab_bar, cfg, struct fields, platform-conditional
hit_test, to_winit, coupling, platform-independent
Box<dyn Widget>, module doc, inaccurate
draw_overlay_at, LayerTree, opacity, pre-resolve
DrawCtx, for_child, child-context, construction, repeated
format_value, String, allocation, cache
OpenDropdown, items, Vec<String>, clone, widget ID
panel, draw, invalidates, layout cache
has_checks, O(N), O(N^2), cache, bool field
dialog, preview, line height, measure, cache
focusable_children, vec!, SmallVec, allocation
menu, layout, measure, label, natural width, cache
hovered, pub, pub(super), visibility
CompositorCtx, too_many_arguments, bundle
```

---

### Section 06: oriterm misc — Config/Keybindings/WindowManager/VTE
**File:** `section-06-misc-vte.md` | **Status:** Complete

```
config, keybindings, window_manager, vte, platform, font
ansi.rs, bloat, 2686 lines, split, handler_trait, osc, csi, esc
lib.rs, inline tests, sibling tests.rs, extract
config, pub, pub(crate), binary crate, visibility
FontCollection, RasterizedGlyph, font, pub(crate)
ManagedWindow, pub fields, pub(crate)
platform, pub mod, pub(crate) mod
MoveTabToNewWindow, usize, TabId, type safety
memory.rs, fallback, non-Windows, non-macOS, non-Linux
intern_atom, double unwrap, error masking
key_to_binding_key, String, allocation, keystroke
main.rs, cfg, inline, platform::startup
memory limits, SI, binary, MiB, document
image_config, image_gpu_memory_limit, omitted
traffic light, centering, duplicate, helper
children clone, boundary, unnecessary
```

---

### Section 07: Cleanup
**File:** `section-07-cleanup.md` | **Status:** Complete

```
cleanup, verification, test-all, clippy-all, build-all
fmt-all, final check, delete plan, green
regression, no warnings, no errors
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | oriterm_core — Term/Grid/Image Boundaries | `section-01-oriterm-core.md` |
| 02 | oriterm_mux — Server/Protocol/PTY Boundaries | `section-02-oriterm-mux.md` |
| 03 | oriterm/app — Event Loop/Input/Redraw | `section-03-oriterm-app.md` |
| 04 | oriterm/gpu — Render Pipeline | `section-04-oriterm-gpu.md` |
| 05 | oriterm_ui — Widget Tree | `section-05-oriterm-ui.md` |
| 06 | oriterm misc — Config/Keybindings/WindowManager/VTE | `section-06-misc-vte.md` |
| 07 | Cleanup | `section-07-cleanup.md` |
