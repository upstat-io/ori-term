---
reroute: true
name: "Fullscreen Traffic Lights"
full_name: "macOS Fullscreen Exit Traffic Light Repositioning Fix"
status: resolved
order: 10
---

# macOS Fullscreen Traffic Lights Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Research Findings
**File:** `section-01-research-findings.md` | **Status:** Complete

```
traffic lights, fullscreen, exit, reposition, jump, pop, bump, flash
NSTitlebarContainerView, standardWindowButton, setFrameOrigin
windowWillExitFullScreen, windowDidExitFullScreen, NSNotification
CATransaction, setDisableActions, setHidden, setVisible
Electron, WindowButtonsProxy, buttons_proxy_, RedrawTrafficLights
Ghostty, Alacritty, WezTerm, reference implementations
fullscreen.rs, macos/mod.rs, center_traffic_lights
NSViewFrameDidChangeNotification, CENTERING_GUARD, animation snapshot
```

---

### Section 02: Implementation
**File:** `section-02-implementation.md` | **Status:** Complete

```
set_titlebar_container_hidden, helper extraction, mod.rs
hide container, show container, setHidden, setVisible
handle_will_exit_fs, handle_did_exit_fs, center_and_disable_drag_raw
NSTitlebarContainerView, titlebar container, superview
windowWillExitFullScreen, windowDidExitFullScreen
fullscreen.rs, macos/mod.rs, event_loop_helpers.rs
Electron pattern, hide/show, redraw, reposition
CATransaction, setDisableActions, file size warning
doc comments, multi-window, process_fullscreen_events
handle_will_enter_fs, safety net, interrupted transition
frame-change observer, handle_frame_change, stale comment, macOS 26 Tahoe
```

---

### Section 03: Verification
**File:** `section-03-verification.md` | **Status:** Complete

```
visual regression, fullscreen exit, fullscreen enter
traffic light position, animation, smooth transition
no jump, no pop, no flash, no bump
manual testing, macOS fullscreen, green button
multi-window, window close during exit, tab operations
Mission Control, Split View, secondary display
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Research Findings | `section-01-research-findings.md` |
| 02 | Implementation (02.1 Helper, 02.2 willExit, 02.3 didExit, 02.4 willEnter, 02.5 Docs, 02.6 Multi-Window, 02.7 Checklist) | `section-02-implementation.md` |
| 03 | Verification | `section-03-verification.md` |
