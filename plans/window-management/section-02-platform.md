---
section: "02"
title: "Platform Native Window Layer"
status: complete
goal: "Cross-platform native window operations for ownership, shadows, and window type hints on Windows, macOS, and Linux"
inspired_by:
  - "Chromium Aura WindowTreeHost platform bridge (ui/aura/window_tree_host.h)"
  - "ori_term existing platform modules (oriterm_ui/src/platform_windows/, platform_macos.rs, platform_linux.rs)"
  - "Ghostty GTK4/Adwaita window class (src/apprt/gtk/class/window.zig)"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "Platform Abstraction Trait"
    status: complete
  - id: "02.2"
    title: "Windows Implementation"
    status: complete
  - id: "02.3"
    title: "macOS Implementation"
    status: complete
  - id: "02.4"
    title: "Linux Implementation"
    status: complete
  - id: "02.5"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Platform Native Window Layer

**Status:** Complete
**Goal:** A `NativeWindowOps` trait with platform-specific implementations that can set window ownership, configure OS shadows on frameless windows, and set window type hints — all using the real OS APIs on Windows, macOS, and Linux. After this section, we can create a dialog window that is owned by a parent, has an OS shadow, and behaves natively on all three platforms.

**Context:** ori_term already has `platform_windows` in `oriterm_ui` with HWND subclassing for Aero Snap, DWM shadow, hit testing, DPI handling, and modal drag loops for tab tear-off (`WM_MOVING` merge detection). macOS (`platform_macos.rs`) and Linux (`platform_linux.rs`) have thin platform layers that delegate drag/resize to winit's cross-platform APIs (`drag_window()`, `drag_resize_window()`), but neither has window ownership, window type hints, or shadow management. We need a cross-platform abstraction that adds the native operations required for proper windowed dialogs: ownership, shadows on frameless windows, window type hints, and modal state.

**Reference implementations:**
- **Chromium** `ui/aura/window_tree_host.h`: Platform-specific subclass implements `ShowImpl()`, `SetBoundsInPixels()`, `GetAcceleratedWidget()`. The host IS the platform bridge.
- **WezTerm** `window/src/os/windows/window.rs`: Per-platform window modules behind `#[cfg()]`.
- **winit** `raw_window_handle()`: Provides `RawWindowHandle` (HWND, NSView, XlibWindow, WaylandSurface) for platform-specific operations.

**Depends on:** Section 01 (needs `WindowKind` to determine what platform hints to apply).

**Architectural note:** The new `NativeWindowOps` trait lives in `oriterm/src/window_manager/platform/` because it depends on `WindowKind` (an oriterm type). The existing `oriterm_ui::platform_windows` module (enable_snap, DWM shadow, WndProc subclass) stays where it is — the new platform layer calls into it for shared functionality like `hwnd_from_window()`. The `oriterm_ui` platform modules remain thin winit wrappers for drag/resize; the new `oriterm` platform modules add window-management-specific operations (ownership, type hints, modal state).

---

## 02.1 Platform Abstraction Trait

**File(s):** `oriterm/src/window_manager/platform/mod.rs` (new)

Define the trait that platform modules implement. Keep it minimal — only operations we actually need.

- [ ] Define `NativeWindowOps` trait
  ```rust
  use winit::window::Window;

  /// Platform-native window operations that go beyond what winit provides.
  /// Each method is best-effort — platforms that don't support an operation
  /// silently no-op (e.g., Wayland doesn't support window positioning).
  pub(crate) trait NativeWindowOps {
      /// Set the owner/parent of a window at the OS level.
      /// On Windows: sets the owner HWND (GWL_HWNDPARENT).
      /// On macOS: calls addChildWindow:ordered:.
      /// On X11: sets _NET_WM_TRANSIENT_FOR / XSetTransientForHint.
      /// On Wayland: sets xdg_toplevel parent.
      fn set_owner(&self, child: &Window, parent: &Window);

      /// Remove owner/parent relationship.
      fn clear_owner(&self, child: &Window);

      /// Enable OS-level shadow on a frameless window.
      /// On Windows: DWM frame extension (1px trick).
      /// On macOS: NSWindow.hasShadow = true.
      /// On Linux: compositor-dependent (usually automatic for normal windows).
      fn enable_shadow(&self, window: &Window);

      /// Set window type hint for the OS.
      /// Affects taskbar visibility, z-ordering behavior, etc.
      fn set_window_type(&self, window: &Window, kind: &super::types::WindowKind);

      /// Set the window as modal relative to its owner.
      /// On Windows: disables the owner window.
      /// On macOS: runModal or sheet presentation.
      /// On Linux: _NET_WM_STATE_MODAL hint.
      fn set_modal(&self, dialog: &Window, owner: &Window);

      /// Clear modal state, re-enable the owner window.
      fn clear_modal(&self, dialog: &Window, owner: &Window);
  }
  ```

- [ ] Create platform dispatcher that selects the correct implementation at compile time
  ```rust
  #[cfg(target_os = "windows")]
  mod windows;
  #[cfg(target_os = "macos")]
  mod macos;
  #[cfg(target_os = "linux")]
  mod linux;

  pub(crate) fn platform_ops() -> &'static dyn NativeWindowOps {
      #[cfg(target_os = "windows")]
      { &windows::WindowsNativeOps }
      #[cfg(target_os = "macos")]
      { &macos::MacosNativeOps }
      #[cfg(target_os = "linux")]
      { &linux::LinuxNativeOps }
  }
  ```

- [ ] Verify `#[cfg(target_os)]` gates (not `#[cfg(unix)]`) so cross-compilation from Linux to Windows works correctly (the `linux` module must not compile when targeting `x86_64-pc-windows-gnu` from WSL)
- [ ] Verify trait compiles on all targets: `cargo build --target x86_64-pc-windows-gnu` (from WSL), native Linux build, macOS CI

---

## 02.2 Windows Implementation

**File(s):** `oriterm/src/window_manager/platform/windows.rs` (new)

Windows has the most mature support since we already have `platform_windows`. This implementation uses Win32 APIs through the `windows-sys` crate (the same FFI crate used by `platform_windows/mod.rs`).

- [ ] Implement `set_owner` using Win32
  ```rust
  use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
  use windows_sys::Win32::UI::WindowsAndMessaging::{
      GWL_HWNDPARENT, SetWindowLongPtrW,
  };

  fn set_owner(&self, child: &Window, parent: &Window) {
      let child_hwnd = extract_hwnd(child);
      let parent_hwnd = extract_hwnd(parent);
      // GWL_HWNDPARENT sets the owner (not parent in MDI sense).
      // Owner relationship means:
      // - Child always appears above owner in z-order
      // - Child is destroyed when owner is destroyed (we handle this ourselves)
      // - Child is hidden when owner is minimized
      unsafe {
          SetWindowLongPtrW(child_hwnd, GWL_HWNDPARENT, parent_hwnd as isize);
      }
  }
  ```

- [ ] Implement `enable_shadow` using DWM
  ```rust
  use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
  use windows_sys::Win32::UI::Controls::MARGINS;

  fn enable_shadow(&self, window: &Window) {
      let hwnd = extract_hwnd(window);
      // For dialog windows (no WS_THICKFRAME), extend frame 1px on all
      // sides to get DWM shadow. The main window in enable_snap() only
      // extends cyTopHeight:1 because WS_THICKFRAME provides the other
      // edges. Dialog windows lack those style bits, so all four margins
      // are needed.
      let margins = MARGINS {
          cxLeftWidth: 1,
          cxRightWidth: 1,
          cyTopHeight: 1,
          cyBottomHeight: 1,
      };
      unsafe {
          DwmExtendFrameIntoClientArea(hwnd, &raw const margins);
      }
  }
  ```

- [ ] Implement `set_window_type` — for dialogs, apply `WS_EX_TOOLWINDOW` to hide from taskbar
  ```rust
  use windows_sys::Win32::UI::WindowsAndMessaging::{
      GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_TOOLWINDOW,
  };

  fn set_window_type(&self, window: &Window, kind: &WindowKind) {
      let hwnd = extract_hwnd(window);
      match kind {
          WindowKind::Dialog(_) => {
              // WS_EX_TOOLWINDOW: no taskbar button, smaller title bar
              unsafe {
                  let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                  SetWindowLongPtrW(hwnd, GWL_EXSTYLE,
                      ex_style | WS_EX_TOOLWINDOW as isize);
              }
          }
          _ => {} // Main and TearOff use default style
      }
  }
  ```

- [ ] Implement `set_modal` / `clear_modal` — disable/enable owner HWND via `EnableWindow` (takes `BOOL` i32, not Rust `bool`)
  ```rust
  use windows_sys::Win32::UI::WindowsAndMessaging::EnableWindow;

  fn set_modal(&self, _dialog: &Window, owner: &Window) {
      let owner_hwnd = extract_hwnd(owner);
      unsafe { EnableWindow(owner_hwnd, 0); } // FALSE = disabled
  }

  fn clear_modal(&self, _dialog: &Window, owner: &Window) {
      let owner_hwnd = extract_hwnd(owner);
      unsafe { EnableWindow(owner_hwnd, 1); } // TRUE = enabled
  }
  ```

- [ ] Reuse or factor out `hwnd_from_window()` from `oriterm_ui/src/platform_windows/mod.rs`
  - Existing pattern: `h.hwnd.get() as HWND` (via `NonZero::get()`)
  - Currently private (`fn`) — either make `pub(crate)` or duplicate the 5-line helper

- [ ] All Win32 FFI calls must check return values and log warnings on failure (not panic)
  - `SetWindowLongPtrW` returns 0 on failure (check `GetLastError`)
  - `DwmExtendFrameIntoClientArea` returns `HRESULT` (check for `S_OK`)
  - `EnableWindow` has no meaningful error return, but log the call for debugging
  - Match the error-handling pattern in existing `oriterm_ui::platform_windows::enable_snap()`

- [ ] Add `#![allow(unsafe_code)]` at the top of `windows.rs` (FFI module — same pattern as `oriterm_ui/src/platform_windows/mod.rs`)

- [ ] Integration test: create two windows, set ownership, verify z-order behavior

---

## 02.3 macOS Implementation

**File(s):** `oriterm/src/window_manager/platform/macos.rs` (new)

macOS uses Objective-C APIs through `objc2` / `icrate` crates or raw `objc` calls. winit provides `NSView` handle; we need to get the `NSWindow` from it.

- [ ] Add `objc2` dependency (or use `objc` crate depending on what winit uses)

- [ ] Implement `set_owner` using NSWindow child window API
  ```rust
  fn set_owner(&self, child: &Window, parent: &Window) {
      let parent_nswindow = get_nswindow(parent);
      let child_nswindow = get_nswindow(child);
      // addChildWindow:ordered: makes child float above parent.
      // NSWindowAbove keeps it on top. The child moves with parent.
      unsafe {
          let _: () = msg_send![parent_nswindow, addChildWindow: child_nswindow
                                                  ordered: NSWindowAbove];
      }
  }
  ```

- [ ] Implement `enable_shadow`
  ```rust
  fn enable_shadow(&self, window: &Window) {
      let nswindow = get_nswindow(window);
      // macOS frameless windows (styleMask = borderless) don't get
      // shadows by default. Setting hasShadow explicitly enables them.
      unsafe {
          let _: () = msg_send![nswindow, setHasShadow: YES];
      }
  }
  ```

- [ ] Implement `set_window_type` — use NSWindow level for dialogs
  ```rust
  fn set_window_type(&self, window: &Window, kind: &WindowKind) {
      let nswindow = get_nswindow(window);
      match kind {
          WindowKind::Dialog(_) => {
              // NSFloatingWindowLevel keeps dialog above normal windows.
              // For modal dialogs, NSModalPanelWindowLevel is appropriate.
              unsafe {
                  let _: () = msg_send![nswindow, setLevel: NSFloatingWindowLevel];
              }
          }
          _ => {}
      }
  }
  ```

- [ ] Implement `set_modal` / `clear_modal`
  ```rust
  fn set_modal(&self, dialog: &Window, _owner: &Window) {
      // On macOS, modal behavior is typically done via
      // NSApp.runModalForWindow: or sheet presentation.
      // For our use case, we'll use the window level approach
      // and handle input blocking at the application level,
      // since we control our own event loop.
      let nswindow = get_nswindow(dialog);
      unsafe {
          let _: () = msg_send![nswindow, setLevel: NSModalPanelWindowLevel];
      }
  }
  ```

- [ ] Implement `get_nswindow` helper (from NSView → NSWindow via `window` property)
  ```rust
  fn get_nswindow(window: &Window) -> *mut Object {
      match window.window_handle().unwrap().as_raw() {
          RawWindowHandle::AppKit(handle) => {
              let nsview = handle.ns_view.as_ptr() as *mut Object;
              unsafe { msg_send![nsview, window] }
          }
          _ => panic!("expected AppKit handle on macOS"),
      }
  }
  ```

- [ ] **IMPORTANT**: Do NOT call `addChildWindow:ordered:` for `WindowKind::TearOff` — tear-off windows must be independent (they move on their own during drag). Only Dialog windows should be child windows.

- [ ] Implement `clear_owner` using `removeChildWindow:`
  ```rust
  fn clear_owner(&self, child: &Window) {
      let child_nswindow = get_nswindow(child);
      // Get the parent window from the child.
      let parent: *mut Object = unsafe { msg_send![child_nswindow, parentWindow] };
      if !parent.is_null() {
          unsafe {
              let _: () = msg_send![parent, removeChildWindow: child_nswindow];
          }
      }
  }
  ```

- [ ] Add `#![allow(unsafe_code)]` at the top of `macos.rs` (ObjC FFI)

---

## 02.4 Linux Implementation

**File(s):** `oriterm/src/window_manager/platform/linux.rs` (new)

Linux has two display server protocols: X11 and Wayland. Both need handling. winit abstracts the window creation but we need raw handles for native hints.

- [ ] Implement X11 path using `XSetTransientForHint`
  ```rust
  fn set_owner_x11(&self, child: &Window, parent: &Window) {
      // RawWindowHandle::Xlib gives us x11 window IDs.
      let child_xid = extract_xlib_window(child);
      let parent_xid = extract_xlib_window(parent);
      let display = extract_xlib_display(child);
      unsafe {
          x11::xlib::XSetTransientForHint(display, child_xid, parent_xid);
      }
  }
  ```

- [ ] Implement Wayland path using `xdg_toplevel::set_parent`
  ```rust
  fn set_owner_wayland(&self, child: &Window, parent: &Window) {
      // Wayland's xdg_toplevel::set_parent() sets the transient relationship.
      // However, accessing the xdg_toplevel from winit requires going through
      // the raw display handle and the wayland-client protocol.
      //
      // NOTE: This may require winit to expose the xdg_toplevel or we may
      // need to use wayland-client directly. Evaluate winit's Wayland
      // extension traits.
      //
      // Fallback: no-op on Wayland if winit doesn't expose the toplevel.
      // The window will still work, just without proper stacking hints.
  }
  ```

- [ ] Implement `enable_shadow` — typically automatic on Linux compositors
  ```rust
  fn enable_shadow(&self, _window: &Window) {
      // On X11 with a compositor (picom, mutter, kwin), frameless windows
      // typically receive shadows automatically based on window type.
      // Setting _NET_WM_WINDOW_TYPE_DIALOG may help.
      //
      // On Wayland, shadows are compositor-managed and always present
      // for xdg_toplevel windows.
      //
      // No explicit action needed in most cases.
  }
  ```

- [ ] Implement `set_window_type` using `_NET_WM_WINDOW_TYPE` on X11
  ```rust
  fn set_window_type_x11(&self, window: &Window, kind: &WindowKind) {
      let xid = extract_xlib_window(window);
      let display = extract_xlib_display(window);
      match kind {
          WindowKind::Dialog(_) => {
              // Set _NET_WM_WINDOW_TYPE to _NET_WM_WINDOW_TYPE_DIALOG
              // This tells the WM to:
              // - Not show in taskbar
              // - Stack above parent
              // - Use dialog decoration style
              set_atom_property(display, xid,
                  "_NET_WM_WINDOW_TYPE",
                  "_NET_WM_WINDOW_TYPE_DIALOG");
          }
          _ => {}
      }
  }
  ```

- [ ] Implement `set_modal` on X11 using `_NET_WM_STATE_MODAL`
  ```rust
  fn set_modal_x11(&self, dialog: &Window, _owner: &Window) {
      let xid = extract_xlib_window(dialog);
      let display = extract_xlib_display(dialog);
      add_atom_state(display, xid, "_NET_WM_STATE_MODAL");
  }
  ```

- [ ] Implement runtime detection of X11 vs. Wayland
  ```rust
  fn set_owner(&self, child: &Window, parent: &Window) {
      match child.window_handle().unwrap().as_raw() {
          RawWindowHandle::Xlib(_) => self.set_owner_x11(child, parent),
          RawWindowHandle::Wayland(_) => self.set_owner_wayland(child, parent),
          _ => {} // Unknown Linux display server, no-op
      }
  }
  ```

- [ ] Add `x11-dl` as a direct dependency (already transitive via winit; declaring it explicitly is needed for direct X11 API calls like `XSetTransientForHint`, `XChangeProperty`)
- [ ] Add Wayland protocol crate if needed (evaluate whether winit exposes enough)
- [ ] Graceful degradation: every method silently no-ops if the platform call is unavailable

- [ ] **X11 Display handle**: Use `winit::raw_window_handle::HasDisplayHandle` to get the `Display*` from the event loop or window. The `RawDisplayHandle::Xlib` variant provides `display: *mut c_void`. Cast to `*mut x11::xlib::Display` for X11 calls. (Handle is only valid while the event loop runs, which is always true in practice.)
- [ ] **X11 property helpers**: Implement `set_atom_property` and `add_atom_state` using `XChangeProperty` and `XSendEvent` (for `_NET_WM_STATE` changes via client messages to the root window — required by EWMH spec)
- [ ] Add `#![allow(unsafe_code)]` at the top of `linux.rs` (X11 FFI)
- [ ] **Wayland `set_parent` limitation**: Document that winit 0.30+ does not expose `xdg_toplevel` directly. Options: (a) use `winit::platform::wayland::WindowExtWayland` if available, (b) contribute upstream to winit, (c) accept the no-op for now. The window still works on Wayland — just without WM stacking hints.

---

## 02.5 Completion Checklist

- [ ] `NativeWindowOps` trait defined with all required methods
- [ ] Windows implementation: ownership, shadow, window type, modal — all using Win32 APIs
- [ ] macOS implementation: child window, shadow, window level, modal
- [ ] Linux implementation: X11 transient_for + window type hints, Wayland best-effort
- [ ] Cross-compile verification: `cargo build --target x86_64-pc-windows-gnu` (from Linux)
- [ ] macOS compilation verified (CI or cross-compile)
- [ ] Platform dispatch (`platform_ops()`) routes correctly per `#[cfg(target_os)]`
- [ ] Platform modules: no `tests.rs` needed (FFI wrappers, integration-tested in Section 08)
- [ ] All files under 500 lines
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green

**Note for Section 05:** The `NativeWindowOps` trait will be extended with drag-related methods (`cursor_screen_pos`, `visible_frame_bounds`, `supports_merge_detection`, `set_transitions_enabled`) when Section 05 is implemented. The trait is `pub(crate)` so this is a non-breaking addition.

**Exit Criteria:** A dialog window created via winit, with `set_owner()` and `enable_shadow()` called, appears as a native owned window with OS shadows on Windows, macOS, and Linux. The window disappears from the taskbar on Windows (WS_EX_TOOLWINDOW), floats above its owner on macOS (NSWindow level), and has dialog type hints on X11 (_NET_WM_WINDOW_TYPE_DIALOG).
