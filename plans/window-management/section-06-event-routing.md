---
section: "06"
title: "Event Routing & Focus Management"
status: in-progress
goal: "Correct event dispatch across all window kinds with proper focus hierarchy and modal blocking"
inspired_by:
  - "Chromium WindowEventDispatcher event flow (ui/aura/window_event_dispatcher.h)"
  - "Chromium FocusClient for focus policy (ui/aura/client/focus_client.h)"
  - "Chromium WindowTargeter hit-test hierarchy (ui/aura/window_targeter.h)"
depends_on: ["03", "04", "05"]  # Only needs 05.1+05.4 (Phase 3b), not 05.2+05.3 (Phase 3c)
sections:
  - id: "06.1"
    title: "Event Dispatch by Window Kind"
    status: complete
  - id: "06.2"
    title: "Focus Hierarchy"
    status: complete
  - id: "06.3"
    title: "Modal Behavior"
    status: complete
  - id: "06.4"
    title: "Keyboard and IME Routing"
    status: in-progress
  - id: "06.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Event Routing & Focus Management

**Status:** In Progress (06.1 + 06.2 + 06.3 complete, 06.4 in progress)
**Goal:** Every winit event is dispatched to the correct handler based on window kind. Focus tracks correctly across main windows and dialogs. Modal dialogs block input to their parent. Keyboard input (including IME composition) routes to the active widget in the focused window, regardless of window kind.

**Context:** Currently, the event loop in `event_loop.rs` routes window events by winit `WindowId`. Most handlers go through `self.focused_ctx()` / `self.focused_ctx_mut()` which look up `self.windows[&focused_window_id]`. The `about_to_wait` handler collects dirty windows and renders each via a temporary focus-swap pattern (saves `focused_window_id` + `active_window`, swaps to each dirty window, calls `handle_redraw()`, then restores). With dialog windows stored in `self.dialogs`, the event loop needs a dispatch layer that checks the window kind and routes to the appropriate handler. Focus management is currently `focused_window_id: Option<WindowId>` (winit ID) plus `active_window: Option<SessionWindowId>` (session model) -- neither accounts for dialog ownership or modal blocking.

**Reference implementations:**
- **Chromium** `ui/aura/window_event_dispatcher.h`: Dispatches events through a targeting phase (find target window), then handles capture, IME pre-processing, and event delivery. Modal windows steal focus and block events to other windows in the same root.
- **Chromium** `ui/aura/client/focus_client.h`: Focus policy interface — determines which window can receive focus, handles focus-stealing prevention.
- **Chromium** `ui/aura/window_targeter.h`: Hit-test hierarchy — mouse events find their target window through parent→child traversal.

**Depends on:** Section 03 (main windows in WM), Section 04 (dialog windows exist), Section 05 (tear-off windows in WM).

**COMPLEXITY WARNING:** The event loop is the central nervous system. Changes here are immediately user-visible if wrong. The focus/blink/PTY-escape-sequence interaction (06.2) is particularly subtle: dialog focus must stop cursor blinking without sending a focus-out escape to the terminal. Test each sub-step independently before moving to the next.

---

## 06.1 Event Dispatch by Window Kind

**File(s):** `oriterm/src/app/event_loop.rs`

**WARNING: `event_loop.rs` is 403 lines.** Adding dialog event dispatch will exceed 500 lines. Extract `handle_dialog_window_event()` and dialog-specific `about_to_wait` logic into a new submodule (e.g., `oriterm/src/app/dialog_events.rs`) and call into it from `event_loop.rs`. Keep the top-level `window_event()` dispatch (the kind-based match) in `event_loop.rs`.

Add a dispatch layer that routes events based on window kind.

- [x] Implement window-kind-aware event dispatch
- [x] Extract current `WindowEvent` handling into `handle_terminal_window_event()`
- [x] Implement `handle_dialog_window_event()` for dialog-specific events
- [x] Handle `about_to_wait` for dialog windows
- [x] Update animation-active check in `about_to_wait` to include dialog windows

---

## 06.2 Focus Hierarchy

**File(s):** `oriterm/src/app/mod.rs`, `oriterm/src/window_manager/mod.rs`

Implement focus tracking that understands window ownership.

- [x] Add focus tracking to `WindowManager` (`focused_id`, `set_focused`, `active_main_window`)
- [x] Update `Focused(true)` handler to use `WindowManager` focus tracking
- [x] Update `Focused(false)` handler — defer focus-out via `PendingFocusOut`
- [ ] Visual feedback: main window shows inactive state when its dialog is focused
- [x] When dialog gains focus: cursor stops blinking, PTY focus-out suppressed
- [x] When dialog loses focus back to main window: resume normal focus handling
- [x] Verify `Focused(false)` handler behavior when focus moves to owned dialog

---

## 06.3 Modal Behavior

**File(s):** `oriterm/src/app/event_loop.rs`, `oriterm/src/window_manager/mod.rs`

Implement modal blocking: when a modal dialog is open, its parent window ignores input.

- [x] Add `DialogKind::is_modal() -> bool` method
- [x] Add `WindowManager::is_modal_blocked()` using `DialogKind::is_modal()`
- [x] Add `WindowManager::find_modal_child()` to locate the blocking dialog
- [x] Block input events to modal-blocked windows in `event_loop.rs` (non-input events like resize, redraw, scale change, focus, theme still pass through)
- [x] When user clicks a modal-blocked window, bring the modal dialog to front via `focus_window()`
- [x] Settings dialog: non-modal (no blocking — `Settings.is_modal() == false`)
- [x] Confirmation dialog: modal (blocks parent — `Confirmation.is_modal() == true`)
- [x] About dialog: non-modal (`About.is_modal() == false`)

**Note:** Platform-conditional modal click handling (Windows `EnableWindow(false)`, macOS/Linux app-level) deferred to Section 08 verification. The current app-level `focus_window()` approach works cross-platform.

---

## 06.4 Keyboard and IME Routing

**File(s):** `oriterm/src/app/dialog_context/event_handling.rs`, `oriterm/src/keybindings/mod.rs`

Ensure keyboard input routes correctly to the focused window's active widget.

- [x] Main/TearOff window focused: keyboard goes to terminal grid (existing behavior, unchanged)
- [x] Dialog window focused: keyboard dispatched via `handle_dialog_keyboard()`
- [ ] IME composition: routes to the focused window (needs dialog text input widgets)
- [x] `Action::is_global()` method distinguishes global actions (`NewWindow`, `NewTab`, `OpenSettings`, `ReloadConfig`) from terminal-only actions (`Copy`, `Paste`, `ScrollUp`, etc.)
- [x] In `handle_dialog_keyboard`: check global bindings first, then dialog-specific handling
- [x] In terminal handler: keep existing dispatch (all bindings available)
- [x] Handle Escape in dialogs: closes the dialog (universal dialog behavior)
- [x] `ModifiersChanged` tracked in dialog event handler for correct keybinding lookup
- [ ] Handle Tab in dialogs: cycle focus between form controls (needs widget focus system)
- [ ] Handle Enter in dialogs: activate the focused button (needs widget focus system)

---

## 06.5 Completion Checklist

- [x] Event dispatch routes correctly by window kind (terminal vs. dialog)
- [x] Focus tracking works across main and dialog windows
- [x] `active_main_window()` returns correct window when dialog is focused
- [x] Modal blocking: confirmation dialog blocks parent input
- [x] Non-modal: settings dialog allows parent input
- [x] Click on modal-blocked window brings dialog to front
- [x] Keyboard input routes to correct window
- [ ] IME composition works in dialogs with text inputs
- [x] Global shortcuts work regardless of focused window
- [x] Escape closes dialogs
- [ ] Tab cycles focus in dialog forms
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** With a settings dialog open, keyboard input in the dialog goes to dialog controls; clicking the main window behind it activates the main window and the dialog stays visible. With a confirmation dialog open, clicking the main window brings the confirmation dialog to front (modal blocking). All keyboard shortcuts work correctly regardless of which window is focused.
