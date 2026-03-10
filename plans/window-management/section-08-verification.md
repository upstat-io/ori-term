---
section: "08"
title: "Verification & Cross-Platform Testing"
status: not-started
goal: "Comprehensive test coverage for all window management operations across Windows, macOS, and Linux"
inspired_by:
  - "Chromium aura test suite (ui/aura/*_unittest.cc)"
  - "ori_term existing test patterns (sibling tests.rs)"
depends_on: ["01", "02", "03", "04", "05", "06", "07"]
sections:
  - id: "08.1"
    title: "Unit Test Matrix"
    status: not-started
  - id: "08.2"
    title: "Integration Test Scenarios"
    status: not-started
  - id: "08.3"
    title: "Cross-Platform Validation"
    status: not-started
  - id: "08.4"
    title: "Visual Regression"
    status: not-started
  - id: "08.5"
    title: "Performance Validation"
    status: not-started
  - id: "08.6"
    title: "Regression Testing"
    status: not-started
  - id: "08.7"
    title: "Completion Checklist"
    status: not-started
---

# Section 08: Verification & Cross-Platform Testing

**Status:** Not Started
**Goal:** Every window management operation is tested at the unit level (WindowManager logic), integration level (window lifecycle scenarios), and platform level (native behavior verification). Zero regressions in existing terminal functionality.

**Context:** The window management system touches every layer: event loop, GPU rendering, platform FFI, session model. A comprehensive test strategy is essential because many failure modes are platform-specific (e.g., z-order behavior, shadow rendering, modal blocking) and only visible through manual testing or visual regression.

**Reference implementations:**
- **Chromium** `ui/aura/window_unittest.cc`: Extensive hierarchy, focus, event dispatch, and stacking tests using a mock WindowDelegate.
- **ori_term**: Existing test pattern — sibling `tests.rs` files with `super::` imports.

**Depends on:** All sections (this is the final verification pass).

---

## 08.1 Unit Test Matrix

**File(s):** `oriterm/src/window_manager/tests.rs` (sibling to `mod.rs` per test-organization rules)

Test WindowManager logic without OS windows (pure in-memory). Tests use `super::` imports and no `mod tests {}` wrapper.

- [ ] **Registry tests:**
  - Register single window → exists in registry
  - Register multiple windows → all accessible
  - Unregister → removed from registry
  - Unregister nonexistent → no panic
  - `main_window_count()` counts correctly (excludes dialogs)

- [ ] **Hierarchy tests:**
  - Register child with parent → parent's children includes child
  - Unregister parent → children collected (depth-first, children before parent)
  - Deep hierarchy (parent → child → grandchild) → cascading close correct
  - Reparent → old parent loses child, new parent gains child
  - Reparent to None → becomes root window

- [ ] **Lifecycle tests:**
  - `should_exit_on_close` with 1 main window → true
  - `should_exit_on_close` with 2 main windows, closing 1 → false
  - `should_exit_on_close` with 1 main + 1 dialog → true (closing main)
  - `should_exit_on_close` when closing dialog → false
  - `find_dialog_parent` with focused main → returns focused
  - `find_dialog_parent` with focused dialog → returns dialog's parent
  - `find_dialog_parent` with no focus → returns any main window

- [ ] **Focus tests:**
  - Set focus to main window → `focused()` returns it
  - Set focus to dialog → `active_main_window()` returns dialog's parent
  - `is_modal_blocked` with no modal children → false
  - `is_modal_blocked` with modal confirmation dialog child → true
  - `is_modal_blocked` with non-modal settings dialog child → false
  - Unregister focused window → `focused()` returns `None`
  - Two main windows, focus second → `active_main_window()` returns second
  - Dialog with no parent (orphaned) → `active_main_window()` returns `None`

---

## 08.2 Integration Test Scenarios

**File(s):** Manual test scripts / automated where possible

These scenarios require real OS windows and must be tested manually on each platform.

- [ ] **Scenario: Settings dialog lifecycle**
  1. Open main window with terminal
  2. Press settings keybinding → settings dialog appears
  3. Verify: dialog has OS shadow
  4. Verify: dialog can be dragged outside main window bounds
  5. Verify: dialog stays above main window in z-order
  6. Click main window → main window activates but dialog remains visible
  7. Close main window → settings dialog also closes
  8. Re-open settings → verify no duplicate dialog

- [ ] **Scenario: Modal confirmation dialog**
  1. Open main window with running process
  2. Close main window → confirmation dialog appears
  3. Verify: clicking main window brings confirmation to front (modal block)
  4. Verify: keyboard input to main window is blocked
  5. Click "Cancel" → dialog closes, main window stays open
  6. Close main window again → dialog appears
  7. Click "OK" → both dialog and main window close

- [ ] **Scenario: Tab tear-off and merge**
  1. Open main window with 3 tabs
  2. Drag tab out → new window created
  3. Verify: new window has OS shadow and chrome
  4. Verify: new window has 1 tab, source has 2 tabs
  5. Drag tab from new window toward source window's tab bar
  6. Verify: tab merges back into source window
  7. Source now has 3 tabs, new window closes

- [ ] **Scenario: Multiple main windows with dialogs**
  1. Open two main windows (Window A, Window B)
  2. Open settings dialog from Window A
  3. Verify: dialog is owned by Window A (stays above A, not B)
  4. Close Window A → settings dialog closes
  5. Window B remains open

- [ ] **Scenario: Dialog from tear-off window**
  1. Tear off a tab to create Window B
  2. Open settings from Window B
  3. Verify: dialog is owned by Window B
  4. Close Window B → dialog closes
  5. Original Window A unaffected

- [ ] **Scenario: Rapid dialog open/close**
  1. Open settings dialog
  2. Immediately close it (Escape or close button)
  3. Repeat 10 times rapidly
  4. Verify: no GPU resource leaks, no stale entries in WindowManager

- [ ] **Scenario: Dialog during tear-off**
  1. Have a settings dialog open on Window A
  2. Tear off a tab from Window A
  3. Verify: dialog stays with Window A, not the new tear-off window
  4. Verify: tear-off window is fully functional

- [ ] **Scenario: Last main window closes with dialog open**
  1. Single main window with settings dialog open
  2. Close main window
  3. Verify: dialog closes first, then app exits (no crash, no orphan dialog)

- [ ] **Scenario: Config reload with settings dialog open**
  1. Open settings dialog
  2. Modify config file externally (triggers config reload)
  3. Verify: settings dialog reflects new config OR shows a warning about external changes

---

## 08.3 Cross-Platform Validation

Test platform-specific behavior on each OS.

- [ ] **Windows:**
  - Dialog has DWM shadow (visible on light backgrounds)
  - Dialog does NOT appear in taskbar (WS_EX_TOOLWINDOW)
  - Dialog minimizes when parent minimizes
  - Modal dialog disables parent window (EnableWindow(false))
  - Aero Snap works on main windows, not on dialogs
  - Win32 drag loop works for tear-off

- [ ] **macOS:**
  - Dialog has NSWindow shadow
  - Dialog floats above parent (NSWindow child window)
  - Dialog does NOT appear in Dock
  - Dialog responds to Mission Control (not separately listed)
  - `performWindowDragWithEvent:` works for tear-off
  - Full-screen main window: dialogs appear on top (not on separate Space)

- [ ] **Linux/X11:**
  - Dialog has compositor shadow (if compositor running)
  - Dialog has `_NET_WM_WINDOW_TYPE_DIALOG` hint
  - Dialog appears above parent in WM stacking
  - `_NET_WM_MOVERESIZE` works for tear-off drag
  - Tested with: GNOME (Mutter), KDE (KWin), i3/Sway

- [ ] **Linux/Wayland:**
  - Dialog creation works (xdg_toplevel)
  - `set_parent` sets transient relationship (if accessible via winit)
  - Compositor shadow present
  - Tear-off: `xdg_toplevel::move_` works (if serial available)
  - Fallback: application-level drag if platform drag unavailable
  - Merge detection is disabled (known limitation -- verify no crash/panic when dragging over another window's tab bar area)
  - Dialog close-with-parent works even without `set_parent` (app-level cascade)

---

## 08.4 Visual Regression

- [ ] Capture reference screenshots for:
  - Main window with OS shadow
  - Settings dialog window with OS shadow
  - Confirmation dialog (modal, parent dimmed if applicable)
  - Multiple windows overlapping (z-order correct)
  - Tear-off in progress (window following cursor)

- [ ] Compare across platforms for parity (shadows, chrome, positioning)

- [ ] Test at multiple DPI settings:
  - 100% (96 DPI)
  - 125% (120 DPI)
  - 150% (144 DPI)
  - 200% (192 DPI)

- [ ] Test multi-monitor: dialog on different monitor than parent (different DPI)

---

## 08.5 Performance Validation

- [ ] **Baseline:** Measure frame time with 1 main window (existing)
- [ ] **With dialog:** Measure frame time with 1 main window + 1 dialog
  - Target: < 1ms overhead per additional window
- [ ] **Multiple windows:** Measure frame time with 3 main windows + 2 dialogs
  - Target: < 16.6ms total (60fps)
- [ ] **GPU memory:** Measure GPU memory per window type
  - Main window: baseline
  - Dialog window: should be < 50% of main window GPU memory
- [ ] **Window creation latency:** Time from request to visible
  - Target: < 100ms for dialog, < 200ms for main window

---

## 08.6 Regression Testing

Verify that the window management refactor introduces no regressions in existing functionality.

- [ ] **Terminal rendering**: PTY output renders correctly in all windows
- [ ] **Copy/paste**: Selection, copy, paste work in multi-window setup
- [ ] **Tab management**: New tab, close tab, switch tab, reorder tabs — all work
- [ ] **Pane splitting**: Split, close pane, navigate panes — all work
- [ ] **Config hot-reload**: Changing config file updates all windows
- [ ] **Search**: Search works in the focused window
- [ ] **Mark mode**: Mark mode works in the focused window
- [ ] **Context menu**: Right-click menu works
- [ ] **Mouse reporting**: Applications that use mouse (htop, vim) work correctly
- [ ] **IME input**: CJK input method works in terminal
- [ ] **DPI change**: Moving window between monitors handles DPI correctly
- [ ] **Fullscreen**: Fullscreen toggle works on main windows
- [ ] **Transparency/blur**: Background transparency renders correctly

---

## 08.7 Completion Checklist

- [ ] Unit tests for WindowManager: registry, hierarchy, lifecycle, focus, modal
- [ ] Integration scenarios tested on Windows
- [ ] Integration scenarios tested on macOS
- [ ] Integration scenarios tested on Linux (X11)
- [ ] Integration scenarios tested on Linux (Wayland)
- [ ] Visual regression screenshots captured
- [ ] Multi-DPI verification done
- [ ] Performance baselines within targets
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] No regressions in existing terminal functionality

**Exit Criteria:** All integration scenarios pass on all three platforms. WindowManager unit tests cover all CRUD, hierarchy, focus, and modal operations. Frame time with dialogs open is within 1ms of baseline. GPU memory for dialogs is less than 50% of a main terminal window. No regressions in `./test-all.sh`.
