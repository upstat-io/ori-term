---
section: "05"
title: "Tear-Off Window Unification"
status: in-progress
goal: "Migrate tear-off tab creation to WindowManager and add macOS/Linux support"
inspired_by:
  - "Chrome tab tear-off (real OS window, OS drag loop)"
  - "Ptyxis ParkingLot detach/reattach pattern (src/ptyxis-window.c)"
  - "WezTerm tab move between windows (mux/src/window.rs)"
depends_on: ["02", "03"]
sections:
  - id: "05.1"
    title: "Tear-Off Through WindowManager"
    status: complete
  - id: "05.2"
    title: "macOS Tear-Off Support"
    status: not-started
  - id: "05.3"
    title: "Linux Tear-Off Support"
    status: not-started
  - id: "05.4"
    title: "Merge Detection Unification"
    status: complete
  - id: "05.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Tear-Off Window Unification

**Status:** In Progress (Phase 3b complete — 05.1 + 05.4 done; Phase 3c deferred)
**Goal:** Tab tear-off creates windows through the WindowManager (same path as `create_window()`), and tear-off works on macOS and Linux in addition to the existing Windows implementation. Merge detection (dragging a torn-off tab back into another window) uses WindowManager's registry to find merge targets.

**Context:** Currently tear-off is Windows-only, implemented in `oriterm/src/app/tab_drag/tear_off.rs`. It calls `create_window_bare()` directly (bypassing any centralized window management), uses `TornOffPending` state, and calls winit's cross-platform `drag_window()` to enter the OS modal drag loop. The Windows-specific parts are: (1) `WM_MOVING` cursor-to-grab-offset correction in the `WndProc` subclass (`oriterm_ui/src/platform_windows/subclass.rs`), (2) merge rect detection against other windows' tab bar zones during the drag, and (3) the `OsDragResult` / `OsDragConfig` types. The merge detection in `merge.rs` collects merge rects from other windows by iterating `self.windows` directly.

After Section 03 migrates main windows to WindowManager, tear-off should use the same registration path. The platform-specific drag loops need macOS and Linux implementations.

**Reference implementations:**
- **Chrome**: Tab tear-off creates a real OS window, enters a platform drag loop (Win32 `DefWindowProc(WM_NCLBUTTONDOWN, HTCAPTION)` or Cocoa `performWindowDrag`), detects merge via cursor position over other windows' tab bars.
- **Ptyxis**: Uses `AdwTabView` built-in drag-and-drop with ParkingLot as intermediate storage for detached tabs.
- **WezTerm**: Tab move is a Mux operation — remove from source window, add to destination window.

**Depends on:** Section 03 (WindowManager integrated, main windows tracked), Section 02 (platform abstractions for cursor/bounds queries).

**COMPLEXITY WARNING:** This is the highest-risk section. It combines refactoring existing Windows-only code (removing `#[cfg]` gates, abstracting platform calls, moving drag state) with new platform implementations (macOS/Linux drag loops). The overview splits this into Phase 3b (05.1+05.4: refactor existing code to use WindowManager, verify on Windows alone) and Phase 3c (05.2+05.3: implement macOS/Linux platform drag, higher risk, can be deferred).

**Critical prerequisite -- removing `#[cfg(target_os = "windows")]` gates:**

The following locations are currently Windows-gated and must be made cross-platform:
- [ ] `oriterm/src/app/mod.rs` line 186-187: `#[cfg(target_os = "windows")] torn_off_pending` — remove `#[cfg]`, make field always present
- [ ] `oriterm/src/app/constructors.rs` lines 95, 164: `torn_off_pending: None` — remove surrounding `#[cfg]` if present
- [ ] `oriterm/src/app/event_loop.rs` `about_to_wait()` line 296-297: `#[cfg(target_os = "windows")] self.check_torn_off_merge()` — remove `#[cfg]`
- [ ] `oriterm/src/app/tab_drag/tear_off.rs`: All `platform_windows::` calls must be replaced with cross-platform abstractions
- [ ] `oriterm/src/app/tab_drag/merge.rs`: All `platform_windows::` calls (cursor_screen_pos, visible_frame_bounds, show_window, take_os_drag_result) must be abstracted
- [ ] `oriterm/src/app/tab_drag/mod.rs`: Check for any `#[cfg]` gates on drag state types

**Cross-platform abstractions needed** (extend `NativeWindowOps` trait from Section 02):
- [ ] `cursor_screen_pos() -> Option<(i32, i32)>` — Windows: `GetCursorPos`, macOS: `NSEvent.mouseLocation`, Linux: `XQueryPointer` / winit events. Returns `None` on failure.
- [ ] `visible_frame_bounds(window) -> Option<(i32, i32, i32, i32)>` — Windows: DWM extended frame, macOS: `NSWindow.frame`, Linux: `XGetWindowAttributes` / winit `outer_position + outer_size`
- [ ] `show_window(window)` — Windows: `ShowWindow(SW_SHOW)`, macOS/Linux: `window.set_visible(true)`. Consider whether this should just use winit's `set_visible()` directly (it should — no platform FFI needed).
- [ ] `set_transitions_enabled(window, bool)` — Windows: DWM, macOS/Linux: no-op
- [ ] `supports_merge_detection() -> bool` — Windows: true (live during drag), macOS: true (post-drag), X11: true (post-drag), Wayland: false

---

## 05.1 Tear-Off Through WindowManager

**File(s):** `oriterm/src/app/tab_drag/tear_off.rs`

Refactor `tear_off_tab()` to register the new window with WindowManager.

- [x] After `create_window_bare()`, register with WindowManager as `WindowKind::TearOff`
  ```rust
  // In tear_off_tab():
  let (new_winit_id, new_session_wid) = self.create_window_bare(event_loop)?;

  // NEW: Register with WindowManager
  self.window_manager.register(ManagedWindow {
      winit_id: new_winit_id,
      kind: WindowKind::TearOff,
      parent: None,  // Tear-offs are independent, not owned by source
      children: Vec::new(),
      visible: false,
  });

  // Rest of tear-off flow unchanged...
  ```

- [x] Update merge detection (`find_merge_target` in `merge.rs`) to use WindowManager registry instead of iterating `self.windows` directly
  ```rust
  // In find_merge_target():
  // Before: iterate self.windows directly
  // After: iterate self.window_manager.main_windows()
  // The actual merge target detection uses platform_windows::visible_frame_bounds()
  // to get screen-space window rects and checks if the cursor is within the
  // tab bar zone. This needs cross-platform abstraction.
  ```

- [x] After successful merge, update WindowManager (unregister torn-off window)

- [ ] Refactor drag state storage: move `OsDragConfig` fields (grab_offset, merge_rects) from platform_windows SnapData (per-HWND subclass data, deeply Windows-specific) into App-level state (e.g., extend `TornOffPending` or add `OsDragSession` field on App)
- [ ] Refactor `OsDragResult` into a cross-platform enum (not in `platform_windows` module)
- [ ] Move `OsDragResult` and `OsDragConfig` to `oriterm/src/app/tab_drag/` (cross-platform location)

---

## 05.2 macOS Tear-Off Support

**File(s):** `oriterm/src/app/tab_drag/platform_macos.rs` (new)

Implement the platform drag loop for macOS using Cocoa's `performWindowDragWithEvent:`.

- [ ] Implement `begin_os_tab_drag` for macOS
  ```rust
  #[cfg(target_os = "macos")]
  pub(crate) fn begin_os_tab_drag(
      window: &winit::window::Window,
      _config: &OsDragConfig,
  ) {
      // macOS approach: use winit's cross-platform drag_window() API.
      // Under the hood, winit calls NSWindow.performWindowDragWithEvent:
      // which enters a modal drag loop managed by the window server.
      // This is the same pattern used by platform_macos.rs::start_drag().
      //
      // NOTE: Unlike Windows, macOS's performWindowDragWithEvent: does NOT
      // provide WM_MOVING-style callbacks during the drag. Merge detection
      // will need a different approach (e.g., timer-based polling or
      // post-drag cursor position check).
      if let Err(e) = window.drag_window() {
          log::warn!("macOS drag_window failed: {e}");
      }
  }
  ```

- [ ] Handle merge detection on macOS
  - During drag, use `NSEvent.mouseLocation` to track cursor position
  - Compare against other windows' tab bar rects (in screen coordinates)
  - macOS screen coordinates: origin at bottom-left (flip Y)

- [ ] Handle the modal nature of `performWindowDragWithEvent:` — it blocks like Win32's drag loop. Need timer-based rendering like Windows implementation.

- [ ] Test: drag tab out of window, new window follows cursor, release creates window at drop position

- [ ] **macOS merge approach**: Post-drag only. After `drag_window()` returns (mouse released), check cursor position against merge rects. If over a tab bar, merge. If not, the window stays where it was dropped. No live merge detection -- accept this platform limitation. (Windows has live merge via `WM_MOVING` callbacks; macOS `performWindowDragWithEvent:` blocks with no callbacks.)
- [ ] macOS Y-coordinate flip: `NSEvent.mouseLocation` uses bottom-left origin. Convert to top-left before comparing with merge rects.

---

## 05.3 Linux Tear-Off Support

**File(s):** `oriterm/src/app/tab_drag/platform_linux.rs` (new)

Linux tear-off is more complex due to X11/Wayland differences.

- [ ] **X11 approach**: Use `XMoveWindow` in response to cursor motion events
  ```rust
  #[cfg(target_os = "linux")]
  pub(crate) fn begin_os_tab_drag(
      window: &winit::window::Window,
      config: &OsDragConfig,
  ) {
      // X11: No built-in modal window drag loop like Win32/Cocoa.
      // Options:
      // (a) Use _NET_WM_MOVERESIZE client message (EWMH) — tells the WM
      //     to start an interactive move. Best approach, works with compositors.
      // (b) Manual: grab pointer, track motion, XMoveWindow on each event.
      //     Works but fights the WM.
      //
      // Recommended: (a) _NET_WM_MOVERESIZE
      send_net_wm_moveresize(window, MoveResizeDirection::Move);
  }
  ```

- [ ] **Wayland approach**: Use `xdg_toplevel::move_` request
  ```rust
  // Wayland: The compositor owns all positioning.
  // xdg_toplevel::move_ initiates an interactive move.
  // This requires the serial from the initiating input event.
  //
  // winit may expose this via drag_window() or similar API.
  // Check winit's Wayland backend for move support.
  ```

- [ ] **Fallback**: If platform drag isn't available, implement application-level drag
  - Track cursor globally via `CursorMoved` events
  - Move the window programmatically via `set_outer_position()`
  - Less smooth than OS-level drag but works everywhere

- [ ] Merge detection on Linux
  - Use `set_outer_position` return or cursor tracking for position
  - Compare cursor against other windows' screen-space tab bar rects
  - On Wayland: window positioning is limited (compositor controls it). Merge detection may need to use cursor position relative to known window positions.

- [ ] **Wayland merge limitation**: Merge detection is not possible on pure Wayland -- the compositor does not expose window positions to applications. Tear-off creates a new window, but merge-back is disabled. Document this as a known limitation. XWayland apps may work via X11 path.
- [ ] Add `fn supports_merge_detection() -> bool` to the platform abstraction — returns `true` on Windows, `true` on macOS (post-drag), `true` on X11, `false` on Wayland

---

## 05.4 Merge Detection Unification

**File(s):** `oriterm/src/app/tab_drag/tear_off.rs` (`collect_merge_rects`), `oriterm/src/app/tab_drag/merge.rs` (`check_torn_off_merge`, `find_merge_target`, `compute_drop_index`)

Unify merge detection across platforms using WindowManager.

- [x] Refactor `collect_merge_rects` (currently in `tear_off.rs`) to use WindowManager registry
  ```rust
  impl App {
      /// Collect tab bar merge rects for all main windows except `exclude`.
      /// Currently returns Vec<[i32; 4]> using platform_windows::visible_frame_bounds.
      /// Refactor to use WindowManager and abstract screen-space rect computation
      /// per platform.
      fn collect_merge_rects(&self, exclude: WindowId) -> Vec<[i32; 4]> {
          self.window_manager
              .windows_of_kind(|k| matches!(k, WindowKind::Main | WindowKind::TearOff))
              .filter(|w| w.winit_id != exclude)
              .filter_map(|w| {
                  let ctx = self.windows.get(&w.winit_id)?;
                  // Platform-specific: get screen-space bounds
                  screen_space_tab_bar_rect(ctx)
              })
              .collect()
      }
  }
  ```

- [ ] Platform-specific cursor-to-screen coordinate conversion
  - Windows: `GetCursorPos` (already used)
  - macOS: `NSEvent.mouseLocation` (flip Y from screen bottom)
  - Linux/X11: `XQueryPointer` or event coordinates
  - Linux/Wayland: limited — may need to track via winit events

- [ ] Merge action: move tab from torn-off window to target window
  - Uses session registry to move tab between session windows
  - Pane in mux is unaffected (just a reference)
  - Close the now-empty torn-off window via WindowManager

---

## 05.5 Completion Checklist

- [x] Tear-off creates windows through WindowManager (registered as `WindowKind::TearOff`)
- [x] Merge detection uses WindowManager registry
- [ ] Windows tear-off: unchanged behavior (regression test)
- [ ] macOS tear-off: tab can be dragged out to create new window
- [ ] Linux/X11 tear-off: tab can be dragged out to create new window
- [ ] Linux/Wayland tear-off: best-effort (may be limited by compositor)
- [ ] Merge works: torn-off tab can be dragged back into another window
- [ ] Empty window cleanup: torn-off window with no tabs closes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green

**Exit Criteria:** Tab tear-off works on all three platforms. Dragging a tab out of a window creates a new OS window that follows the cursor. Releasing the cursor drops the window in place. On Windows (live merge during drag), macOS (post-drag merge), and Linux/X11 (post-drag merge), dragging over another window's tab bar merges the tab back. On Wayland, merge detection is disabled (known limitation -- tear-off still creates a new window). All tear-off windows are tracked in WindowManager.
