---
section: "02"
title: "Implementation"
status: complete
goal: "Modify fullscreen transition handlers to hide/show the NSTitlebarContainerView, eliminating the traffic light jump artifact"
reviewed: true
inspired_by:
  - "Electron WindowButtonsProxy setVisible: (shell/browser/ui/cocoa/window_buttons_proxy.mm)"
  - "Electron NativeWindowMac NotifyWindowWillLeaveFullScreen (shell/browser/native_window_mac.mm)"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "Extract set_titlebar_container_hidden Helper"
    status: complete
  - id: "02.2"
    title: "Hide Container in willExit"
    status: complete
  - id: "02.3"
    title: "Show Container in didExit"
    status: complete
  - id: "02.4"
    title: "Safety: willEnter Handler"
    status: complete
  - id: "02.5"
    title: "Update Doc Comments and Stale Frame-Change Comment"
    status: complete
  - id: "02.6"
    title: "Multi-Window Considerations"
    status: complete
  - id: "02.7"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Implementation

**Status:** Not Started
**Goal:** Traffic lights do not jump, flash, or pop during fullscreen exit transitions. Buttons appear at the correct vertically-centered position after the animation completes.

**Context:** The current implementation in `fullscreen.rs` centers buttons in `handle_will_exit_fs`, observes `NSViewFrameDidChangeNotification` for synchronous re-centering, and centers again in `handle_did_exit_fs`. Despite this triple-centering approach, a race with macOS's animation snapshot timing can still produce a visible jump. The fix is simple: hide the container before the animation, show it after repositioning.

**Reference implementation:**
- **Electron** `shell/browser/native_window_mac.mm`: `NotifyWindowWillLeaveFullScreen()` calls `[buttons_proxy_ setVisible:NO]`; `NotifyWindowLeaveFullScreen()` calls `[buttons_proxy_ redraw]` then `[buttons_proxy_ setVisible:YES]`

**Depends on:** Section 01 (Research — understanding of the root cause and recommended approach).

---

## 02.1 Extract `set_titlebar_container_hidden` Helper

**File:** `oriterm/src/window_manager/platform/macos/mod.rs`

> **File size warning:** `mod.rs` is currently 439 lines. This helper adds ~20 lines (~460 total). Monitor this file — if future changes push it past 480 lines, proactively extract the `NSPoint`/`NSRect`/`Encode` impls (lines 254-294) into a `types.rs` submodule before hitting the 500-line hard limit.

Extract the container-discovery + `setHidden` logic into a helper in `mod.rs` (alongside `center_and_disable_drag_raw`), then call it from `fullscreen.rs` via `super::set_titlebar_container_hidden`. This avoids duplicating the superview chain traversal and follows the existing pattern where fullscreen handlers call `super::center_and_disable_drag_raw`.

- [ ] Add the helper function in `mod.rs`, placed immediately after `center_and_disable_drag_raw` (after line 232):
  ```rust
  /// Set the hidden state of the `NSTitlebarContainerView`.
  ///
  /// The container is discovered via the view hierarchy:
  /// `standardWindowButton(0)` -> superview (`NSTitlebarView`) -> superview (`NSTitlebarContainerView`).
  ///
  /// Called from fullscreen transition handlers to hide the container before the
  /// exit animation (preventing traffic light jump) and show it after repositioning.
  ///
  /// # Safety
  ///
  /// `nswindow` must be a valid, non-null `NSWindow` pointer.
  unsafe fn set_titlebar_container_hidden(nswindow: *mut AnyObject, hidden: bool) {
      let close_btn: *mut AnyObject = msg_send![nswindow, standardWindowButton: 0i64];
      if close_btn.is_null() { return; }
      let titlebar_view: *mut AnyObject = msg_send![close_btn, superview];
      if titlebar_view.is_null() { return; }
      let container: *mut AnyObject = msg_send![titlebar_view, superview];
      if container.is_null() { return; }
      // Disable implicit Core Animation so the show/hide is instant.
      let ca = AnyClass::get("CATransaction").expect("CATransaction not found");
      let _: () = msg_send![ca, begin];
      let _: () = msg_send![ca, setDisableActions: true];
      let _: () = msg_send![container, setHidden: hidden];
      let _: () = msg_send![ca, commit];
  }
  ```

- [ ] The helper function visibility must be plain `unsafe fn` (private to `mod.rs`). Since `fullscreen.rs` is a submodule of `macos/`, it accesses the helper via `super::set_titlebar_container_hidden`, and private items are visible to submodules in Rust — no `pub(super)` needed.

- [ ] Verify the `unsafe` code compiles without lint errors: `mod.rs` already has `#[allow(unsafe_code, reason = "Objective-C FFI via objc2")]` at the module level (line 10), and the helper follows the same FFI pattern as `center_and_disable_drag_raw`. No additional `#[allow]` needed.

---

## 02.2 Hide Container in willExit

**File:** `oriterm/src/window_manager/platform/macos/fullscreen.rs`

In `handle_will_exit_fs`, after the existing centering call, hide the `NSTitlebarContainerView` so the macOS animation snapshot does not show buttons at their default positions. **Depends on 02.1** (the `set_titlebar_container_hidden` helper must exist).

- [ ] After the existing `center_and_disable_drag_raw(nswindow)` call in `handle_will_exit_fs`, hide the titlebar container:
  ```rust
  // Hide the titlebar container to prevent traffic light jump during
  // the exit animation. Electron uses this same pattern.
  unsafe { super::set_titlebar_container_hidden(nswindow, true) };
  ```

- [ ] Keep the existing `center_and_disable_drag_raw(nswindow)` call before the hide — it ensures the container is correctly positioned if the hide somehow fails or is delayed. Do not remove it.

- [ ] **Verify no event-loop changes needed:** Confirm that `process_fullscreen_events()` in `event_loop_helpers.rs` still works correctly — it calls `reapply_traffic_lights()` (centering + drag disable), which is harmless while the container is hidden and ensures correct positions for the eventual show. No code changes required.

- [ ] **Verify no frame-change observer changes needed:** Confirm that `handle_frame_change` still works correctly — it may fire while the container is hidden (macOS still rebuilds the container), but centering while hidden is a visual no-op that pre-positions buttons correctly. No code changes required.

---

## 02.3 Show Container in didExit

**File:** `oriterm/src/window_manager/platform/macos/fullscreen.rs`

In `handle_did_exit_fs`, after the existing centering call, show the container. The order matters: center first, show second (matches Electron's `redraw` then `setVisible:YES`). **Depends on 02.1** (the `set_titlebar_container_hidden` helper must exist).

**macOS 26 (Tahoe) note:** Electron discovered that toggling `setHidden:` on macOS 26 causes AppKit to re-layout the container and reset its frame. If this affects us, a second `center_and_disable_drag_raw` call after `set_titlebar_container_hidden(nswindow, false)` may be needed. The existing `handle_did_exit_fs` already calls centering first, so this should be safe — but worth verifying on macOS 26 if available.

- [ ] After the existing `center_and_disable_drag_raw(nswindow)` call in `handle_did_exit_fs`, show the titlebar container, then re-center as a safety net for macOS 26:
  ```rust
  // Show the titlebar container now that buttons are correctly positioned.
  unsafe { super::set_titlebar_container_hidden(nswindow, false) };
  // Re-center after show — macOS 26 (Tahoe) re-layouts the container
  // when setHidden: is toggled, potentially resetting button positions.
  unsafe { super::center_and_disable_drag_raw(nswindow) };
  ```

---

## 02.4 Safety: willEnter Handler

**File:** `oriterm/src/window_manager/platform/macos/fullscreen.rs`

When entering fullscreen, ensure the container is visible. This handles the edge case where a failed or interrupted fullscreen exit left the container hidden. **Depends on 02.1** (the `set_titlebar_container_hidden` helper must exist).

- [ ] Rename `_notif` parameter to `notif` in `handle_will_enter_fs` (currently unused, prefixed with underscore).
- [ ] Add a safety-net show:
  ```rust
  // Ensure container is visible when entering fullscreen (safety net for
  // interrupted exit transitions that may have left it hidden).
  if !notif.is_null() {
      let nswindow: *mut AnyObject = msg_send![notif, object];
      if !nswindow.is_null() {
          unsafe { super::set_titlebar_container_hidden(nswindow, false) };
      }
  }
  ```

- [ ] Verify the safety-net show is defensive-only: the container should already be visible (shown in `handle_did_exit_fs`), but interrupted transitions or rapid enter/exit cycles could leave it hidden. The show call is a no-op when the container is already visible.

---

## 02.5 Update Doc Comments and Stale Frame-Change Comment

Five comments must be updated: three doc comments that explicitly say the module does NOT toggle traffic light visibility, one inline comment that overstates the frame-change observer's effectiveness, and one function doc that makes the same overstatement. All must reflect the new hide/show behavior.

- [ ] **`fullscreen.rs` module doc (lines 8-10):** The current doc says "The handlers center traffic lights synchronously (before macOS captures animation snapshots) and set atomic flags..." Update to mention that the handlers also hide/show the `NSTitlebarContainerView` during exit transitions (Electron pattern).

- [ ] **`reapply_traffic_lights` doc in `mod.rs` (lines 152-154):** Remove the sentence "Traffic light visibility is NOT toggled — macOS manages it natively." Replace with a note that fullscreen transition visibility toggling is handled by the notification observers in `fullscreen.rs`.

- [ ] **`install_fullscreen_observers` doc in `fullscreen.rs` (line 73):** Remove "No traffic light visibility toggling — macOS handles it natively." Replace with a note that `willExit`/`didExit` handlers toggle `NSTitlebarContainerView` visibility (Electron pattern).

- [ ] **Frame-change observer inline comment in `fullscreen.rs` (lines 122-125):** The current comment says the observer fires "re-centering buttons before macOS captures the animation snapshot — eliminating both the 'bump' and 'pop' artifacts." This overstates the observer's effectiveness — the frame-change observer alone cannot reliably prevent the artifact due to the snapshot timing race documented in Section 01.1. Update to say the observer re-centers buttons as a supplementary measure, and that the primary artifact prevention is the hide/show pattern in the `willExit`/`didExit` handlers.

- [ ] **`handle_frame_change` doc comment in `fullscreen.rs` (lines 240-245):** The doc says "buttons are positioned correctly before macOS captures the animation snapshot — no visible bump or pop." Update to match the corrected inline comment: re-centering is a supplementary measure, not the primary artifact fix.

---

## 02.6 Multi-Window Considerations

The notification observers are registered per-window (the `addObserver:...object:nswindow` call filters notifications by the specific `NSWindow` instance). This means hide/show operates correctly per-window — only the window exiting fullscreen has its container hidden.

However, `process_fullscreen_events` in `event_loop_helpers.rs` uses a single global `AtomicU8` bitfield (`FULLSCREEN_EVENTS`). If two windows exit fullscreen simultaneously, their flags would merge. This is a pre-existing limitation (not introduced by this plan) and is extremely unlikely in practice. No changes needed, but document the assumption:

- [ ] Add a comment in `fullscreen.rs` near `FULLSCREEN_EVENTS` noting that the global atomic works correctly for hide/show (which happens in the notification handler per-window) but is a simplification for the event-loop flags (which apply to the focused window only). This is acceptable because only one window can be focused during a fullscreen transition.

---

## 02.7 Completion Checklist

- [ ] Helper function `set_titlebar_container_hidden` added in `mod.rs` (02.1)
- [ ] `handle_will_exit_fs` hides `NSTitlebarContainerView` after centering (02.2)
- [ ] `handle_did_exit_fs` centers buttons, shows `NSTitlebarContainerView`, then re-centers for macOS 26 safety (02.3)
- [ ] `handle_will_enter_fs` ensures container is visible as safety net (02.4)
- [ ] Doc comments updated: module doc, `reapply_traffic_lights`, `install_fullscreen_observers`, frame-change observer inline comment, `handle_frame_change` doc (02.5)
- [ ] Multi-window comment added near `FULLSCREEN_EVENTS` (02.6)
- [ ] `NSViewFrameDidChangeNotification` observer kept as existing safety net (no changes)
- [ ] `process_fullscreen_events()` in `event_loop_helpers.rs` unchanged — centering while hidden is correct
- [ ] `mod.rs` stays under 500 lines after helper addition (~460 lines expected, currently 439)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] No regressions on Windows or Linux builds (macOS-only code is `#[cfg]`-gated)

**Exit Criteria:** Code compiles on all platforms. Fullscreen exit shows no traffic light jump (verified manually in Section 03).
