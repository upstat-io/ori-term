---
reroute: true
name: "macOS Platform Fixes"
full_name: "macOS Platform Fixes: Chrome, Tear-Off, and Snapshot Latency"
status: active
order: 1
---

# macOS Platform Fixes Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Window Chrome Platform Gate
**File:** `section-01-chrome-platform-gate.md` | **Status:** Not Started

```
chrome, window controls, traffic lights, minimize, maximize, close
draw_window_controls, controls_draw.rs, control_state.rs, WindowControlButton
platform gate, cfg, target_os, macos, windows, linux
tab bar, CONTROLS_ZONE_WIDTH, control buttons, NSFullSizeContentView
downstream call sites, chrome/mod.rs, tab_bar_input.rs, route_control_mouse
update_control_hover_animation, clear_control_hover, set_maximized
interactive_rects, control_rect, hovered_control, handle_control_mouse
ControlButtonColors, ControlKind, create_controls, control_colors_from_theme
```

---

### Section 02: Non-Blocking Snapshot Refresh
**File:** `section-02-nonblocking-snapshot.md` | **Status:** Not Started

```
tab switch, hang, freeze, latency, blocking, synchronous
refresh_pane_snapshot, rpc, RPC_TIMEOUT, recv_timeout
snapshot, pane_snapshot, dirty_panes, pushed snapshot
daemon mode, MuxClient, transport, mux pump
redraw, handle_redraw, content_changed, pane_changed
MarkAllDirty, fire_and_forget, stale snapshot, dirty flag lifecycle
pending_refresh, clear_pane_snapshot_dirty, invalidate_pushed_snapshot
```

---

### Section 03: macOS Tab Tear-Off
**File:** `section-03-macos-tear-off.md` | **Status:** Not Started

```
tear-off, tab drag, torn off, new window, detach tab
tear_off.rs, merge.rs, begin_os_tab_drag, TornOffPending
platform_windows, cursor_screen_pos, begin_os_drag, OsDragConfig
drag_window, WM_MOVING, modal drag loop, event_loop.rs
macOS, Cocoa, NSWindow, NSEvent, mouseLocation, objc2
OsDragResult, drag_types.rs, constructors.rs, Wayland, X11
begin_single_tab_os_drag, check_torn_off_merge, update_drag_in_bar
```

---

### Section 04: Verification
**File:** `section-04-verification.md` | **Status:** Not Started

```
test, verify, visual regression, platform matrix
build-all, clippy-all, test-all, cross-platform
macOS, windows, linux, CI, daemon mode, embedded mode
pending_refresh cleanup, multi-display, Wayland
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Window Chrome Platform Gate | `section-01-chrome-platform-gate.md` |
| 02 | Non-Blocking Snapshot Refresh | `section-02-nonblocking-snapshot.md` |
| 03 | macOS Tab Tear-Off | `section-03-macos-tear-off.md` |
| 04 | Verification | `section-04-verification.md` |
