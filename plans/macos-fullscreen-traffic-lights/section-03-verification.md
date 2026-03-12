---
section: "03"
title: "Verification"
status: complete
goal: "Verify that fullscreen transitions produce no traffic light repositioning artifacts"
reviewed: true
depends_on: ["02"]
sections:
  - id: "03.1"
    title: "Manual Test Matrix"
    status: complete
  - id: "03.2"
    title: "Edge Cases"
    status: complete
  - id: "03.3"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Verification

**Status:** Complete
**Goal:** Confirm that the hide/show fix eliminates traffic light jump artifacts across all fullscreen transition scenarios, with no regressions in other traffic light behavior.

**Context:** This fix cannot be automatically tested — it requires visual verification of animation smoothness. The test matrix covers all fullscreen entry/exit paths and edge cases that could reveal regressions.

**Depends on:** Section 02 (Implementation complete).

---

## 03.1 Manual Test Matrix

Each test should be performed and the result noted.

- [x] **Basic fullscreen exit (green button click):**
  - Enter fullscreen via green zoom button
  - Wait for animation to complete
  - Exit fullscreen via green zoom button
  - **Verify:** Traffic lights appear at correct vertically-centered position after exit animation. No jump, no flash, no pop.

- [x] **Basic fullscreen exit (keyboard shortcut):**
  - Enter fullscreen via Ctrl+Cmd+F (or menu)
  - Exit fullscreen via Ctrl+Cmd+F
  - **Verify:** Same as above.

- [x] **Rapid enter/exit cycle:**
  - Enter fullscreen
  - Immediately exit before animation completes
  - **Verify:** Traffic lights end up at correct position. No crash, no stuck hidden state.

- [x] **Multiple enter/exit cycles:**
  - Enter → exit → enter → exit (4+ cycles)
  - **Verify:** Traffic lights correct every time. No progressive degradation.

- [x] **Fullscreen on secondary display:**
  - Move window to a secondary monitor
  - Enter and exit fullscreen
  - **Verify:** Traffic lights correct. DPI differences (if any) handled.

- [x] **Traffic light hover state after exit:**
  - Exit fullscreen
  - Hover over traffic lights
  - **Verify:** Hover highlight (orange/green/red glow) works correctly. Buttons are interactive.

- [x] **Traffic light click after exit:**
  - Exit fullscreen
  - Click close button (traffic light)
  - **Verify:** Window closes (or close confirmation appears). Buttons are functional.

- [x] **Window resize after exit:**
  - Exit fullscreen
  - Resize window by dragging edges
  - **Verify:** Traffic lights remain vertically centered in tab bar. No shift during resize.

- [x] **Tab bar interactions after exit:**
  - Exit fullscreen
  - Click tabs, create new tab, close tabs
  - **Verify:** Traffic lights remain stable. Tab bar inset is correct (76px left padding).

---

## 03.2 Edge Cases

- [x] **Focus change during fullscreen exit:**
  - Start fullscreen exit
  - Click another window during animation
  - Return to ori_term window
  - **Verify:** Traffic lights at correct position. No hidden-stuck state.

- [x] **Mission Control during fullscreen:**
  - Enter fullscreen
  - Open Mission Control (swipe up with 3 fingers)
  - Exit fullscreen from Mission Control
  - **Verify:** Traffic lights correct after transition.

- [x] **Split View exit:**
  - Enter Split View with another app
  - Exit Split View
  - **Verify:** Traffic lights at correct position.

- [x] **Fullscreen enter (regression check):**
  - Enter fullscreen
  - **Verify:** Traffic lights hidden by macOS during fullscreen (no visible buttons in fullscreen space). Tab bar left inset removed (no 76px gap).

- [x] **App launch in fullscreen (if supported):**
  - Set macOS to reopen windows in fullscreen (System Settings -> Desktop & Dock)
  - Launch app
  - Exit fullscreen
  - **Verify:** Traffic lights correct on first exit after launch.

- [x] **Multi-window fullscreen exit:**
  - Open two windows
  - Enter fullscreen on one window
  - Exit fullscreen on that window
  - **Verify:** Traffic lights correct on the exiting window. Other window's traffic lights unaffected.

- [x] **Window close during fullscreen exit:**
  - Enter fullscreen
  - Begin exiting fullscreen (click green button)
  - Immediately Cmd+W to close the window during the exit animation
  - **Verify:** No crash, no error. The window closes cleanly. (The hidden container is destroyed with the window — no dangling state.)

- [x] **Tab operations during fullscreen exit:**
  - Enter fullscreen with multiple tabs
  - Begin exiting fullscreen
  - Close a tab during the exit animation
  - **Verify:** No crash. Traffic lights appear correctly after animation completes.

- [x] **macOS 26 (Tahoe) regression (if available):**
  - If running macOS 26 or later, perform the basic fullscreen exit test
  - **Verify:** Traffic lights at correct position after exit. The post-show re-center in `handle_did_exit_fs` compensates for Tahoe's `setHidden:` re-layout behavior.

---

## 03.3 Completion Checklist

- [x] All test matrix items verified (03.1)
- [x] All edge cases verified (03.2)
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Every item in the test matrix is checked and passes. No visual artifacts observed during any fullscreen transition scenario. Build and test suites green on all platforms.
