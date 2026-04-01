---
section: 9
title: "Session & Tab/Window Management"
domain: "oriterm/src/app/tab_management/, oriterm/src/session/"
status: in-progress
---

# Section 09: Session & Tab/Window Management

Bugs in tab lifecycle, window management, tab movement, split trees, floating panes, and navigation.

## Open Bugs

- [ ] `[BUG-09-1][high]` **"Move to New Window" context menu action creates blank window** — found by manual.
  Repro: Right-click a tab > "Move to New Window" > new window appears blank. Dragging the same tab off (tear-off) works correctly.
  Subsystem: `oriterm/src/app/tab_management/move_ops.rs` (`move_tab_to_new_window_embedded`)
  Root cause (likely): The embedded path uses `create_window()` (which spawns a fresh pane/tab), moves the requested tab, then tries to close the initial tab. Compare with the working `tear_off_tab()` in `oriterm/src/app/tab_drag/tear_off.rs` which uses `create_window_bare()` (no initial tab), directly inserts the moved tab, pre-renders both windows, and explicitly shows the new window. The context menu path likely fails to properly activate the moved tab's content, wire up the pane rendering, or pre-render the new window.
  Found: 2026-03-31 | Source: manual
  Note: Active work in roadmap section 32 (tab-window-mux) and section 44 (multi-process-windows) touches this area.

## Resolved Bugs

(none yet)
