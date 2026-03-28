---
section: "03"
title: "UI Framework Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in the UI framework (layout, interaction, focus, animation)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Active Bugs"
    status: in-progress
---

# Section 03: UI Framework Bugs

**Status:** In Progress
**Goal:** Track and fix all discovered bugs in the UI framework.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 03.1 Active Bugs

- [x] **BUG-03.1**: Clicking outside focused input doesn't unfocus — only clicking nav works
  - **File(s)**: `oriterm_ui/src/window_root/pipeline.rs`
  - **Root cause**: The event dispatch pipeline only changed focus when a `FocusController` explicitly requested it (i.e., clicking another focusable widget). Clicking non-focusable areas (empty space, labels) produced no focus change request, leaving the previous input focused.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Added Step 5.5 in `process_event()`: after dispatch, if the event is `MouseDown`, no overlay consumed it, no `REQUEST_FOCUS` was issued, and focus is active, clear focus via `interaction.clear_focus()`.
