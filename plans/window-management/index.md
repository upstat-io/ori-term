---
reroute: true
name: "Window Management"
full_name: "Window Management System"
status: in-progress
order: 1
---

# Window Management System Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Window Manager Core
**File:** `section-01-core.md` | **Status:** Complete

```
WindowManager, ManagedWindow, WindowKind, window registry
window hierarchy, parent-child, transient window, owned window
window lifecycle, create window, close window, destroy window
WindowId, WinitId, SessionWindowId, window lookup
focused_id, z-order, window stacking, should_exit_on_close
```

---

### Section 02: Platform Native Window Layer
**File:** `section-02-platform.md` | **Status:** Complete

```
NativeWindowOps, platform abstraction, raw window handle
HWND, SetWindowLongPtr, GWL_HWNDPARENT, WS_EX_TOOLWINDOW
DWM, DwmExtendFrameIntoClientArea, shadow, frameless shadow
NSWindow, addChildWindow, orderFront, hasShadow, NSWindowLevel
NSPanel, NSFloatingWindowLevel, NSModalPanelWindowLevel
X11, XSetTransientForHint, _NET_WM_WINDOW_TYPE, _NET_WM_WINDOW_TYPE_DIALOG
Wayland, xdg_toplevel, set_parent, xdg_popup, layer_shell
window type hints, taskbar grouping, minimize behavior
```

---

### Section 03: Main Window Migration
**File:** `section-03-main-window.md` | **Status:** Complete

```
TermWindow, WindowContext, window creation, App.windows
create_window, create_window_bare, window initialization
SessionRegistry, session window, active_window, focused_window_id
event loop integration, about_to_wait, handle_redraw
window swap pattern, temp focus swap, multi-window render
dual-map design, window_manager field, parallel HashMap
```

---

### Section 04: Dialog Window System
**File:** `section-04-dialogs.md` | **Status:** In Progress

```
DialogWindow, settings dialog, settings window, preferences
confirmation dialog, about dialog, DialogKind
OverlayManager removal, modal overlay replacement
dialog content, form rendering, settings panel
DialogWindowContext, dialog_context, dialog_management.rs
dialog positioning, center on parent, dialog chrome
real OS window, floating dialog, moveable dialog
```

---

### Section 05: Tear-Off Window Unification
**File:** `section-05-tear-off.md` | **Status:** In Progress (Phase 3b done)
**Note:** Split into Phase 3b (05.1+05.4 refactor, Windows-only verify) and Phase 3c (05.2+05.3 cross-platform, higher risk). See overview dependency graph.

```
tear-off, tab drag, tab detach, DragPhase, TornOffPending
tear_off_tab, begin_os_tab_drag, merge detection
cross-platform tear-off, macOS drag, Linux drag
window merge, tab reattach, parking lot
create_window_bare, window_management.rs
cfg gate removal, platform abstraction, OsDragConfig, OsDragResult
cursor_screen_pos, visible_frame_bounds, supports_merge_detection
Wayland merge limitation
```

---

### Section 06: Event Routing & Focus Management
**File:** `section-06-event-routing.md` | **Status:** In Progress (06.1-06.3 done, 06.4 in progress)

```
event routing, event dispatch, WindowEvent, UserEvent
focus management, focus hierarchy, active window
keyboard routing, mouse routing, input dispatch
modal behavior, modal block, dialog focus trap
IME routing, text input, composition
window activation, window deactivation, focus change
dialog_events.rs, handle_dialog_window_event
blinking_active, focus escape sequence, cursor blink
global vs terminal keybindings, Action::is_global
```

---

### Section 07: GPU Multi-Window Rendering
**File:** `section-07-gpu.md` | **Status:** Complete

```
GPU sharing, shared device, shared queue, GpuState
per-window renderer, WindowRenderer, surface, atlases
dialog rendering, UI-only pipeline, no grid, UiOnly
RendererMode, new_ui_only, render_ui_only, ui_only.rs
frame scheduling, multi-window redraw, dirty tracking
instance buffers, bind groups, atlas sharing
PreparedFrame, glyph atlas per-window
```

---

### Section 08: Verification & Cross-Platform Testing
**File:** `section-08-verification.md` | **Status:** Not Started

```
test matrix, cross-platform, Windows test, macOS test, Linux test
visual regression, screenshot comparison, shadow rendering
focus behavior, modal behavior, z-order verification
tear-off test, dialog test, window lifecycle test
performance validation, frame time, multi-window overhead
regression testing, terminal rendering, copy paste, tab management
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Window Manager Core | `section-01-core.md` |
| 02 | Platform Native Window Layer | `section-02-platform.md` |
| 03 | Main Window Migration | `section-03-main-window.md` |
| 04 | Dialog Window System | `section-04-dialogs.md` |
| 05 | Tear-Off Window Unification | `section-05-tear-off.md` |
| 06 | Event Routing & Focus Management | `section-06-event-routing.md` |
| 07 | GPU Multi-Window Rendering | `section-07-gpu.md` |
| 08 | Verification & Cross-Platform Testing | `section-08-verification.md` |
