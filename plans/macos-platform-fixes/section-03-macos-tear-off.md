---
section: "03"
title: "macOS Tab Tear-Off"
status: complete
goal: "Tab tear-off and merge works on macOS using Cocoa APIs, matching the Windows behavior"
inspired_by:
  - "Chrome macOS tab drag: uses [NSWindow performWindowDrag:] for modal drag"
  - "WezTerm: uses winit drag_window() cross-platform for window drag"
depends_on: ["01"]
sections:
  - id: "03.1"
    title: "Platform Abstraction for Tear-Off APIs"
    status: complete
  - id: "03.2"
    title: "macOS Tear-Off Implementation"
    status: complete
  - id: "03.3"
    title: "macOS Merge Detection"
    status: complete
  - id: "03.4"
    title: "Completion Checklist"
    status: complete
---

# Section 03: macOS Tab Tear-Off

**Status:** Complete
**Goal:** Tab tear-off on macOS creates a new window, positions it under the cursor, and enters an OS drag session. Merge detection finds target windows when the dragged window is released over another window's tab bar. Behavior matches the existing Windows implementation.

**Context:** The `tear_off.rs` and `merge.rs` modules are gated with `#[cfg(target_os = "windows")]` in `tab_drag/mod.rs` (lines 7-10). The in-bar drag code (`update_drag_in_bar`) has a fallback on non-Windows that just logs `"tear-off not supported on this platform"` (line 272). The Windows implementation uses `oriterm_ui::platform_windows` APIs that don't exist on macOS: `cursor_screen_pos()`, `begin_os_drag()`, `set_transitions_enabled()`, `visible_frame_bounds()`, `show_window()`, `take_os_drag_result()`.

**Root cause:** Tab tear-off was implemented Windows-first using Win32 `WM_MOVING` interception for real-time merge detection during the OS modal drag loop. macOS has no equivalent — `[NSWindow performWindowDrag:]` (invoked by winit's `drag_window()`) enters a modal tracking loop but provides no per-move callbacks.

**Reference implementations:**
- **Chrome** `chrome/browser/ui/views/tabs/tab_drag_controller.cc`: Uses platform-specific drag loops. On macOS, uses `CocoaWindowMoveLoop` which tracks `NSEventTypeLeftMouseDragged`.
- **WezTerm**: Does not support tab tear-off/merge across windows.

**Depends on:** Section 01 (correct chrome metrics on macOS needed for merge rect computation).

---

## 03.1 Platform Abstraction for Tear-Off APIs

**File(s):** `oriterm_ui/src/platform_macos.rs` (add new functions), `oriterm_ui/src/platform_linux.rs` (add stubs), `oriterm/src/app/tab_drag/tear_off.rs` and `merge.rs` (update imports)

The tear-off code currently calls `platform_windows` APIs directly. Add matching functions to `platform_macos.rs` and `platform_linux.rs`, then update imports in `tear_off.rs` and `merge.rs` with `#[cfg]`-gated `use` statements.

File size impact: `platform_macos.rs` is currently 40 lines; adding 4 functions (~15-20 lines each) brings it to ~120-140 lines. `platform_linux.rs` is 30 lines, similar additions bring it to ~100-130 lines. Both well under the 500-line limit.

- [x] Define platform functions needed by tear-off:
  ```rust
  // In oriterm_ui/src/platform_macos.rs (and platform_linux.rs equivalents):

  /// Get the current cursor position in screen coordinates (top-left origin).
  pub fn cursor_screen_pos() -> (i32, i32);

  /// Get the visible frame bounds (left, top, right, bottom) in screen pixels (top-left origin).
  pub fn visible_frame_bounds(window: &Window) -> Option<(i32, i32, i32, i32)>;

  /// Disable/enable window transition animations. No-op on macOS and Linux.
  pub fn set_transitions_enabled(window: &Window, enabled: bool);

  /// Show a hidden window.
  pub fn show_window(window: &Window);
  ```

- [x] **Prerequisite: Add Cocoa FFI dependency.** Add `objc2` + `objc2-app-kit` + `objc2-foundation` to `oriterm_ui/Cargo.toml` as `[target.'cfg(target_os = "macos")'.dependencies]`. These are the idiomatic Rust bindings for Apple frameworks (successor to the older `cocoa` crate). `NSEvent::mouseLocation` and `NSScreen::mainScreen` require AppKit bindings.

- [x] **Add `#![allow(unsafe_code, reason = "macOS Cocoa FFI via objc2")]`** to the top of `platform_macos.rs`. This follows the same pattern as `platform_windows/mod.rs` line 11. Platform FFI modules are the designated exception to the `unsafe_code = "deny"` project policy (per CLAUDE.md: "Only justified platform FFI in clearly marked modules").

- [x] macOS implementation of `cursor_screen_pos()`: Use `[NSEvent mouseLocation]` and convert from Cocoa's bottom-left origin to top-left using `NSScreen.mainScreen.frame.size.height`.
  ```rust
  pub fn cursor_screen_pos() -> (i32, i32) {
      // NSEvent::mouseLocation returns point in screen coords (bottom-left origin).
      // Convert to top-left origin: top_left_y = screen_height - cocoa_y.
  }
  ```

- [x] macOS implementation of `visible_frame_bounds()`: Use `[NSWindow frame]` with the same Y-flip coordinate conversion.

- [x] macOS `set_transitions_enabled()`: Implement as a no-op — macOS does not have DWM-style transition animations that need suppression. If fine-grained control is needed later, set `NSWindow.animationBehavior` (a property, not a callable), but a no-op is correct for tear-off.

- [x] macOS `show_window()`: Call `window.set_visible(true)` (winit handles the Cocoa call).

- [x] **WARNING: Coordinate conversion is error-prone.** macOS Cocoa uses bottom-left screen origin; oriterm uses top-left. Every function returning screen coordinates must perform the Y-flip: `top_left_y = screen_height - cocoa_y`. Get `screen_height` from `NSScreen.mainScreen.frame.size.height`. `NSEvent.mouseLocation` and `NSWindow.frame` both use the primary screen's bottom-left as global origin, so the Y-flip formula works correctly across all displays. **Test on a multi-display setup** to verify merge detection works when source and target windows are on different displays.

- [x] Add matching stubs in `oriterm_ui/src/platform_linux.rs`:

  **Linux implementation notes:**
  - `cursor_screen_pos()` on **X11**: Use `XQueryPointer` via `x11rb` or raw X11 bindings. Winit does not expose global cursor position.
  - `cursor_screen_pos()` on **Wayland**: **No global cursor position API exists in the Wayland protocol.** `wl_pointer.motion` events only provide surface-relative coordinates. Recommended: return `window.outer_position() + last_surface_relative_pos` as approximation, and disable merge detection on Wayland (tear-off still creates a new window, merge silently disabled).
  - `visible_frame_bounds()`: Use `window.outer_position()` + `window.outer_size()` from winit (works on both X11 and Wayland).
  - `set_transitions_enabled()`: No-op (Linux has no DWM-style transitions).
  - `show_window()`: Call `window.set_visible(true)` (winit handles it).

- [x] Update `tear_off.rs` and `merge.rs` to import from the correct platform module:
  ```rust
  #[cfg(target_os = "windows")]
  use oriterm_ui::platform_windows as platform;
  #[cfg(target_os = "macos")]
  use oriterm_ui::platform_macos as platform;
  #[cfg(target_os = "linux")]
  use oriterm_ui::platform_linux as platform;
  ```

---

## 03.2 macOS Tear-Off Implementation

**File(s):** `oriterm/src/app/tab_drag/tear_off.rs`, `oriterm/src/app/tab_drag/mod.rs`

Remove the `#[cfg(target_os = "windows")]` gate on `tear_off.rs` and make the tear-off logic cross-platform.

- [x] Remove `#[cfg(target_os = "windows")]` from `mod tear_off;` in `tab_drag/mod.rs` line 10.

- [x] **WARNING: High change density in `tear_off.rs`.** This file (255 lines) has the following `platform_windows::` call sites that must ALL be replaced. Missing even one causes a compile error on non-Windows:
  | Line | Call | Replacement |
  |------|------|-------------|
  | 13 | `use oriterm_ui::platform_windows::{self, OsDragConfig}` | `#[cfg]`-gated import; `OsDragConfig` is Windows-only |
  | 96 | `platform_windows::cursor_screen_pos()` | `platform::cursor_screen_pos()` |
  | 133 | `platform_windows::set_transitions_enabled(..., false)` | `platform::set_transitions_enabled(...)` |
  | 135 | `platform_windows::set_transitions_enabled(..., true)` | `platform::set_transitions_enabled(...)` |
  | 170-178 | `platform_windows::begin_os_drag(...)` | `#[cfg(target_os = "windows")]` block |
  | 248 | `platform_windows::visible_frame_bounds(...)` in `collect_merge_rects()` | `platform::visible_frame_bounds(...)` |

- [x] In `tear_off.rs`, replace all `platform_windows::` calls with `platform::` calls (from the abstraction in 03.1). The `OsDragConfig` import at line 13 must be gated with `#[cfg(target_os = "windows")]` since it is only used inside the `#[cfg(target_os = "windows")]` block in `begin_os_tab_drag()`.

- [x] Handle the `begin_os_drag` difference per platform:
  - **Windows**: `platform_windows::begin_os_drag()` sets up `WM_MOVING` interception + merge rect detection before `drag_window()`.
  - **macOS**: No pre-drag setup needed. Just call `window.drag_window()` directly. Merge detection happens after drag ends (see 03.3).
  - **Linux**: Same as macOS — no pre-drag setup.

- [x] Modify `begin_os_tab_drag()` to be cross-platform:
  ```rust
  fn begin_os_tab_drag(&mut self, winit_id: WindowId, tab_id: TabId, ...) {
      #[cfg(target_os = "windows")]
      {
          let merge_rects = self.collect_merge_rects(winit_id);
          if let Some(ctx) = self.windows.get(&winit_id) {
              platform::begin_os_drag(ctx.window.window(), OsDragConfig {
                  grab_offset,
                  merge_rects,
                  skip_count: 3,
              });
          }
      }

      self.torn_off_pending = Some(TornOffPending { winit_id, tab_id, mouse_offset });

      if let Some(ctx) = self.windows.get(&winit_id) {
          if let Err(e) = ctx.window.window().drag_window() {
              log::warn!("drag_window failed: {e}");
          }
      }
  }
  ```

- [x] Remove `#[cfg(target_os = "windows")]` from `TornOffPending` struct in `tab_drag/mod.rs` line 66.

- [x] Remove `#[cfg(target_os = "windows")]` from `torn_off_pending` field on `App` (at `oriterm/src/app/mod.rs` line 215).

- [x] Remove `#[cfg(target_os = "windows")]` from `torn_off_pending: None` initialization in BOTH constructor paths in `oriterm/src/app/constructors.rs` (lines 99-100 and lines 172-173).

- [x] Remove `#[cfg_attr(not(target_os = "windows"), allow(dead_code))]` from `TabDragState::tab_id` (mod.rs line 36).

- [x] In `update_drag_in_bar()` (mod.rs line 268), replace the platform-split with an unconditional tear-off call:
  ```rust
  if exceeds_tear_off(logical_y, info.bar_y, info.bar_bottom) {
      self.tear_off_tab(event_loop);
      return;
  }
  ```

- [x] Pass `event_loop` to `update_drag_in_bar` and `update_tab_drag` unconditionally — remove `#[cfg(target_os = "windows")]` from those parameters at all three locations:
  - `update_tab_drag` definition at mod.rs line 185
  - `update_drag_in_bar` definition at mod.rs line 265
  - All call sites of `update_drag_in_bar` inside `update_tab_drag` at lines 241-242 and 251-252

- [x] Remove `#[cfg(target_os = "windows")]` from `begin_single_tab_os_drag` call in `update_tab_drag` (mod.rs lines 224-228) — single-tab window drag with merge detection should also work on macOS.

- [x] Remove `#[cfg(target_os = "windows")]` from `self.check_torn_off_merge()` call in `event_loop.rs` lines 342-343 — merge checking must run on all platforms.

- [x] Remove `#[cfg(target_os = "windows")]` from the `event_loop` parameter in the `update_tab_drag` call site in `event_loop.rs` lines 188-189.

- [x] Remove `#[cfg_attr(not(target_os = "windows"), allow(dead_code))]` from `DragInfo::tab_count` in `tab_drag/mod.rs` line 424 — it will be used on all platforms once tear-off is ungated.

---

## 03.3 macOS Merge Detection

**File(s):** `oriterm/src/app/tab_drag/merge.rs`, `oriterm/src/app/tab_drag/mod.rs`

On Windows, merge detection happens during the drag (via `WM_MOVING` interception). On macOS, `drag_window()` blocks until mouse-up with no per-move callback. Merge detection must happen after the drag ends.

- [x] Remove `#[cfg(target_os = "windows")]` from `mod merge;` in `tab_drag/mod.rs` line 8.

- [x] **Move `OsDragResult` to a shared location**: Currently `OsDragResult` is defined in `oriterm_ui/src/platform_windows/mod.rs` (lines 68-80). Since merge detection must be cross-platform, move this enum to a shared module.

  **Recommended location**: `oriterm_ui/src/drag_types.rs` (new file, ~15 lines), re-exported from `oriterm_ui/src/lib.rs`.
  ```rust
  //! Shared types for OS-level drag sessions.
  //!
  //! Extracted from `platform_windows` so that `tear_off.rs` and `merge.rs`
  //! can reference these types on all platforms.

  /// Result of an OS drag session.
  #[derive(Debug)]
  pub enum OsDragResult {
      /// OS drag ended normally (user released mouse).
      DragEnded { cursor: (i32, i32) },
      /// Merge detected during drag (Windows WM_MOVING only).
      MergeDetected { cursor: (i32, i32) },
  }
  ```

  **Sync points for the move:**
  | File | Change |
  |------|--------|
  | `oriterm_ui/src/platform_windows/mod.rs` | Remove `OsDragResult` definition, re-export from `drag_types` |
  | `oriterm_ui/src/lib.rs` | Add `pub mod drag_types;` and `pub use drag_types::OsDragResult;` |
  | `oriterm/src/app/tab_drag/merge.rs` | Change import from `oriterm_ui::platform_windows::OsDragResult` to `oriterm_ui::OsDragResult` |

  `OsDragConfig` stays Windows-only since only Windows uses `begin_os_drag()`.

- [x] Create platform-specific merge detection in `check_torn_off_merge()`:

  **Key insight**: On macOS/Linux, `drag_window()` blocks in `begin_os_tab_drag` until the user releases the mouse. So when `check_torn_off_merge` runs in `about_to_wait`, the drag is already complete — there is no "polling". The result is immediately available. On Windows, `WM_MOVING` can produce a `MergeDetected` result mid-drag, so `take_os_drag_result` returns `None` while the drag is ongoing.

  ```rust
  pub(in crate::app) fn check_torn_off_merge(&mut self) {
      let Some(pending) = &self.torn_off_pending else { return; };
      let winit_id = pending.winit_id;
      let tab_id = pending.tab_id;
      let mouse_offset = pending.mouse_offset;

      #[cfg(target_os = "windows")]
      let result = {
          let Some(ctx) = self.windows.get(&winit_id) else {
              self.torn_off_pending = None;
              return;
          };
          platform_windows::take_os_drag_result(ctx.window.window())
      };

      #[cfg(not(target_os = "windows"))]
      let result = {
          // drag_window() already returned — cursor position is final.
          Some(OsDragResult::DragEnded {
              cursor: platform::cursor_screen_pos(),
          })
      };

      let Some(result) = result else {
          return; // Windows: drag still in progress.
      };
      self.torn_off_pending = None;
      // ... rest of merge logic (find_merge_target, compute_drop_index, etc.)
  }
  ```

- [x] **macOS/Linux: `is_live` is always `false`** since `MergeDetected` only comes from Windows `WM_MOVING`. The `begin_seamless_drag_after_merge` path (live merge with synthesized new in-bar drag) will never execute on macOS/Linux. This is correct — seamless drag continuation requires OS-level mouse capture control that macOS/Linux do not provide.

- [x] **WARNING: High change density in `merge.rs`.** This file (253 lines) has the following `platform_windows::` call sites that must ALL be replaced:
  | Line | Call | Replacement |
  |------|------|-------------|
  | 13 | `use oriterm_ui::platform_windows::{self, OsDragResult}` | `#[cfg]`-gated platform import + shared `OsDragResult` |
  | 40 | `platform_windows::take_os_drag_result(...)` | `#[cfg(target_os = "windows")]` block (macOS/Linux use immediate result) |
  | 119 | `platform_windows::show_window(...)` | `platform::show_window(...)` |
  | 147 | `platform_windows::visible_frame_bounds(...)` in `find_merge_target()` | `platform::visible_frame_bounds(...)` |
  | 168 | `platform_windows::visible_frame_bounds(...)` in `compute_drop_index()` | `platform::visible_frame_bounds(...)` |
  | 197 | `platform_windows::visible_frame_bounds(...)` in `begin_seamless_drag_after_merge()` | `platform::visible_frame_bounds(...)` |

- [x] Make `find_merge_target()` and `compute_drop_index()` cross-platform by replacing `platform_windows::visible_frame_bounds()` with `platform::visible_frame_bounds()`.

- [x] Make `begin_seamless_drag_after_merge()` cross-platform by replacing `platform_windows::visible_frame_bounds()` at line 197 with `platform::visible_frame_bounds()`. Note: this method only executes when `is_live` is true (Windows `MergeDetected` only), but it must still compile on all platforms since the module is no longer `#[cfg]`-gated.

- [x] Handle `collect_merge_rects()` in `tear_off.rs`: only needed on Windows (passed to `begin_os_drag`). On macOS/Linux, merge rects are not used during the drag since merge detection happens post-drag via `find_merge_target` which calls `visible_frame_bounds` directly. Either: (a) skip `collect_merge_rects` on non-Windows via `#[cfg]`, or (b) keep it cross-platform (compile cost only, no runtime cost since `begin_os_drag` is `#[cfg(windows)]`-gated).

- [x] Handle coordinate system differences:
  - **Windows**: screen coordinates are top-left origin. No conversion needed.
  - **macOS**: Cocoa uses bottom-left origin — `visible_frame_bounds()` and `cursor_screen_pos()` must convert to top-left using `NSScreen.mainScreen.frame.size.height` for the Y-flip.
  - **Multi-display**: `NSEvent.mouseLocation` and `NSWindow.frame` both use the primary screen's bottom-left as global origin. The Y-flip formula `top_left_y = primary_screen_height - cocoa_y` works correctly across all displays. **Test on a multi-display setup** to verify merge detection when source and target windows are on different displays.
  - **Linux (X11)**: coordinates are already top-left origin.
  - **Linux (Wayland)**: no global cursor position API — `cursor_screen_pos()` returns an approximation. Merge detection may not work; tear-off (creating a new window) still works.

---

## 03.4 Completion Checklist

- [x] `#[cfg(target_os = "windows")]` removed from `mod tear_off;` and `mod merge;`
- [x] `TornOffPending` and `torn_off_pending` field are unconditional
- [x] `torn_off_pending: None` initialization in BOTH constructors in `constructors.rs` is unconditional
- [x] `TabDragState::tab_id` has no `#[cfg_attr]` dead_code allowance
- [x] `DragInfo::tab_count` has no `#[cfg_attr]` dead_code allowance
- [x] `update_tab_drag` and `update_drag_in_bar` take `event_loop` on all platforms
- [x] `begin_single_tab_os_drag` call in `update_tab_drag` is unconditional
- [x] `check_torn_off_merge` call in `event_loop.rs` is unconditional
- [x] `event_loop` parameter in `update_tab_drag` call in `event_loop.rs` is unconditional
- [x] `OsDragResult` moved to `oriterm_ui/src/drag_types.rs` and re-exported
- [x] macOS `cursor_screen_pos()` returns correct top-left-origin coordinates (Y-flip via `NSScreen.mainScreen.frame.size.height`)
- [x] macOS `visible_frame_bounds()` returns correct top-left-origin bounds
- [x] Linux `cursor_screen_pos()` compiles (X11: works via `XQueryPointer`; Wayland: approximation)
- [x] Linux `visible_frame_bounds()` compiles (uses `window.outer_position()` + `window.outer_size()`)
- [x] Tab tear-off creates a new window on macOS
- [x] Tab merge detects target window on macOS after drag ends
- [x] Seamless drag continuation after live merge works on Windows (no regression)
- [x] `tear_off.rs` imports use `#[cfg]`-gated `use platform_*` (not hardcoded `platform_windows`)
- [x] `merge.rs` imports use `#[cfg]`-gated `use platform_*` and shared `OsDragResult`
- [x] Existing tab drag tests in `tab_drag/tests.rs` still pass (pure computation, platform-independent)
- [x] `./build-all.sh` succeeds
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes

**Sync points — ALL locations where `#[cfg(target_os = "windows")]` must be removed or changed:**
| File | Line(s) | Current gate | New state |
|------|---------|-------------|-----------|
| `tab_drag/mod.rs` | 7-8 | `#[cfg(target_os = "windows")] mod merge;` | Unconditional |
| `tab_drag/mod.rs` | 9-10 | `#[cfg(target_os = "windows")] mod tear_off;` | Unconditional |
| `tab_drag/mod.rs` | 36 | `#[cfg_attr(not(windows), allow(dead_code))]` on `tab_id` | Remove |
| `tab_drag/mod.rs` | 66 | `#[cfg(target_os = "windows")]` on `TornOffPending` | Remove |
| `tab_drag/mod.rs` | 185 | `#[cfg(target_os = "windows")]` on `event_loop` param in `update_tab_drag` | Remove |
| `tab_drag/mod.rs` | 224-228 | `#[cfg(target_os = "windows")]` on `begin_single_tab_os_drag` call | Remove |
| `tab_drag/mod.rs` | 241-242 | `#[cfg(target_os = "windows")]` on `event_loop` in first `update_drag_in_bar` call | Remove |
| `tab_drag/mod.rs` | 251-252 | `#[cfg(target_os = "windows")]` on `event_loop` in second `update_drag_in_bar` call | Remove |
| `tab_drag/mod.rs` | 265 | `#[cfg(target_os = "windows")]` on `event_loop` param in `update_drag_in_bar` def | Remove |
| `tab_drag/mod.rs` | 269-273 | `#[cfg(windows)]`/`#[cfg(not(windows))]` on tear-off call | Remove both, call unconditionally |
| `tab_drag/mod.rs` | 424 | `#[cfg_attr(not(windows), allow(dead_code))]` on `tab_count` | Remove |
| `app/mod.rs` | 215-216 | `#[cfg(target_os = "windows")]` on `torn_off_pending` | Remove |
| `app/constructors.rs` | 99-100 | `#[cfg(target_os = "windows")]` on `torn_off_pending: None` | Remove |
| `app/constructors.rs` | 172-173 | `#[cfg(target_os = "windows")]` on `torn_off_pending: None` | Remove |
| `app/event_loop.rs` | 188-189 | `#[cfg(target_os = "windows")]` on `event_loop` in `update_tab_drag` call | Remove |
| `app/event_loop.rs` | 342-343 | `#[cfg(target_os = "windows")]` on `check_torn_off_merge()` | Remove |
| `tear_off.rs` | 13 | `use oriterm_ui::platform_windows` | `#[cfg]`-gated platform import |
| `merge.rs` | 13 | `use oriterm_ui::platform_windows::{self, OsDragResult}` | `#[cfg]`-gated + shared `OsDragResult` |

**Exit Criteria:** Dragging a tab past the tear-off threshold on macOS creates a new window with the tab. Releasing the window over another window's tab bar merges the tab. All existing Windows tear-off tests pass. All platforms build cleanly.
