---
section: "04"
title: "Dialog Window System"
status: in-progress
goal: "Settings, confirmations, and about dialogs as real OS windows with native shadows, moveable outside parent bounds"
inspired_by:
  - "Chromium TransientWindowClient for dialog ownership (ui/aura/client/transient_window_client.h)"
  - "Ghostty GTK4 dialog windows (src/apprt/gtk/class/window.zig)"
  - "VS Code settings as a separate editor window"
depends_on: ["02", "03", "07"]
sections:
  - id: "04.1"
    title: "DialogWindowContext Type"
    status: complete
  - id: "04.2"
    title: "Dialog Window Creation"
    status: complete
  - id: "04.3"
    title: "Settings Dialog Migration"
    status: complete
    note: "Control wiring (mouse events, dropdowns, save/cancel) completed"
  - id: "04.4"
    title: "Confirmation Dialog Migration"
    status: not-started
  - id: "04.5"
    title: "Dialog Positioning and Chrome"
    status: complete
  - id: "04.6"
    title: "Completion Checklist"
    status: in-progress
---

# Section 04: Dialog Window System

**Status:** Not Started
**Goal:** The settings dialog opens as a real OS window — with native shadows, its own title bar (or custom chrome), moveable independently of the main window, able to be dragged outside the main window's bounds. Same for confirmation dialogs and any future dialog types. Each dialog is owned by a parent main window: it stays above the parent, closes when the parent closes, and gets proper OS treatment on all three platforms.

**Context:** Currently the settings dialog is an overlay pushed into the active window's `OverlayManager` via `push_modal()`. It's centered within the terminal window and cannot leave its bounds. Confirmation dialogs use a `DialogWidget` in the same overlay system. Both feel like in-app panels rather than real windows. The user explicitly wants these to be proper OS windows with real windowing behavior.

The `OverlayManager` will still exist for lightweight popups (context menus, dropdowns, command palette) that should stay within the window. Only heavy modals (settings, confirmations) become real windows.

**Reference implementations:**
- **Chromium** `ui/aura/client/transient_window_client.h`: Transient windows are owned by a parent — destroyed when parent closes, always stacked above.
- **VS Code**: Settings opens in a new editor tab or a separate panel. Preferences dialogs in Electron apps are real BrowserWindows.
- **Ghostty**: Uses GTK4 native dialogs (`GtkDialog`, `AdwPreferencesWindow`) which are real OS windows.

**Depends on:** Section 02 (platform native layer for ownership/shadows), Section 03 (WindowManager integrated into App), Section 07 (UiOnly GPU renderer for dialog windows).

---

## 04.1 DialogWindowContext Type

**File(s):** `oriterm/src/app/dialog_context.rs` (new)

Dialogs need their own window state, separate from `WindowContext` (which is terminal-specific with tab bars, grids, pane caches). Define a lighter-weight context for dialog windows.

- [x] Define `DialogWindowContext` struct
  ```rust
  /// Per-dialog-window state. Lighter than WindowContext — no terminal grid,
  /// no tab bar, no pane cache. Just UI rendering.
  /// Unlike WindowContext, this does NOT hold a session_window_id — dialogs
  /// are not part of the session model.
  pub(crate) struct DialogWindowContext {
      /// The native window handle.
      pub window: Arc<Window>,
      /// wgpu rendering surface bound to this dialog window.
      pub surface: wgpu::Surface<'static>,
      /// Surface configuration.
      pub surface_config: wgpu::SurfaceConfiguration,
      /// Per-window GPU renderer (UI-only — no grid pipelines needed).
      pub renderer: Option<WindowRenderer>,
      /// The dialog content widget.
      pub content: DialogContent,
      /// Window chrome widget (custom title bar for frameless dialog).
      pub chrome: WindowChromeWidget,
      /// Draw list for the dialog frame.
      pub draw_list: DrawList,
      /// Dirty flag for redraw.
      pub dirty: bool,
  }
  ```

- [x] Define `DialogContent` enum for dialog-specific content
  ```rust
  pub(crate) enum DialogContent {
      Settings {
          panel: SettingsPanel,
          pending_config: Config,
          original_config: Config,
      },
      Confirmation {
          dialog: DialogWidget,
          on_confirm: Box<dyn FnOnce() + Send>,
      },
      About {
          // Version info, credits, etc.
      },
  }
  ```

- [x] Decide: should dialogs use the same `WindowContext` type with an enum payload, or a completely separate type? The separate type is cleaner (no Option fields for grid/tab_bar/etc.) but means the event loop needs two code paths.

  **Recommended:** Separate type (`DialogWindowContext`). The event loop already matches on window kind via WindowManager — it can dispatch to the right handler based on kind.

  **Impl-hygiene note:** `DialogWindowContext` holding `Arc<Window>` and `wgpu::Surface` is correct — it is a wiring/rendering layer (analogous to `WindowContext` for terminal windows), not a logic layer.

- [x] Store dialog contexts in App alongside window contexts
  ```rust
  pub(crate) struct App {
      windows: HashMap<WindowId, WindowContext>,       // Main/TearOff windows
      dialogs: HashMap<WindowId, DialogWindowContext>,  // Dialog windows
      window_manager: WindowManager,                    // Tracks all
      // ...
  }
  ```

- [x] **Proactive split**: Create as `dialog_context/mod.rs` from the start (not `dialog_context.rs`). Place rendering in `dialog_context/rendering.rs` and event handling in `dialog_context/event_handling.rs`. This avoids a rename-to-directory refactor later.
- [ ] Add `#[cfg(test)] mod tests;` at bottom of `dialog_context/mod.rs` with sibling `dialog_context/tests.rs`
- [ ] `DialogContent::Confirmation::on_confirm` uses `Box<dyn FnOnce()>` — verify this works with the `Send` bound (it does if the closure captures only `Send` types). Consider `Box<dyn FnOnce() + Send>` explicitly.

**Sync point -- adding `dialogs: HashMap<WindowId, DialogWindowContext>` to `App`:**
- [x] `oriterm/src/app/mod.rs` — add field declaration
- [x] `oriterm/src/app/constructors.rs` — initialize in `App::new()` and `App::new_daemon()` as `HashMap::new()`
- [x] `oriterm/src/app/event_loop.rs` — `window_event()` dispatch must check `self.dialogs` (see Section 06)
- [ ] `oriterm/src/app/event_loop.rs` — `about_to_wait()` must iterate `self.dialogs` for dirty checking

---

## 04.2 Dialog Window Creation

**File(s):** `oriterm/src/app/dialog_management.rs` (new)

Centralized dialog creation that goes through WindowManager and applies platform native ops.

- [x] Implement `open_dialog()` method on App (implemented as `open_settings_dialog()`)
  ```rust
  impl App {
      pub(super) fn open_dialog(
          &mut self,
          event_loop: &ActiveEventLoop,
          kind: DialogKind,
          parent_winit_id: WindowId,
      ) -> Option<WindowId> {
          // 1. Determine dialog size based on kind
          let (width, height) = match &kind {
              DialogKind::Settings => (720, 560),
              DialogKind::Confirmation => (440, 240),
              DialogKind::About => (400, 300),
          };

          // 2. Get parent window for positioning
          let parent_window = &self.windows[&parent_winit_id].window;
          let parent_pos = parent_window.window().outer_position()
              .unwrap_or_default();
          let parent_size = parent_window.window().outer_size();

          // 3. Center dialog on parent
          let dialog_x = parent_pos.x + (parent_size.width as i32 - width) / 2;
          let dialog_y = parent_pos.y + (parent_size.height as i32 - height) / 2;

          // 4. Create winit window via existing WindowConfig pattern
          let window_config = oriterm_ui::window::WindowConfig {
              title: dialog_title(&kind).into(),
              inner_size: oriterm_ui::geometry::Size::new(width as f32, height as f32),
              transparent: false,
              blur: false,
              opacity: 1.0,
              position: Some(oriterm_ui::geometry::Point::new(
                  dialog_x as f32, dialog_y as f32,
              )),
              resizable: kind.is_resizable(),
          };
          let window = oriterm_ui::window::create_window(event_loop, &window_config).ok()?;
          let winit_id = window.id();

          // 5. Apply platform native ops
          let parent_raw = parent_window.window();
          let ops = platform_ops();
          ops.set_owner(&window, parent_raw);
          ops.enable_shadow(&window);
          ops.set_window_type(&window, &WindowKind::Dialog(kind.clone()));
          // NOTE: Do NOT call enable_snap() for dialog windows — dialogs
          // should not have Aero Snap behavior (no WS_THICKFRAME, no
          // WS_MAXIMIZEBOX). The enable_shadow() path uses the 4-sided
          // DWM margin (1px all sides) instead of the main window's
          // top-only margin + WS_THICKFRAME combination.

          // 6. Create GPU surface and renderer
          // Dialog windows bypass TermWindow — they use Arc<Window> + surface directly.
          // oriterm_ui::window::create_window returns Arc<Window>.
          // GpuState::create_surface(window) returns (Surface, SurfaceConfiguration).
          let gpu = self.gpu.as_ref()?;
          let pipelines = self.pipelines.as_ref()?;
          let (surface, surface_config) = gpu.create_surface(&window).ok()?;
          // Dialog renderer: use UiOnly mode (Section 07) to skip grid buffers.
          // Dialog windows only need ui_font_collection (proportional sans-serif),
          // not the terminal mono font_collection. Use self.ui_font_set to create one.
          // NOTE: WindowRenderer::new() currently requires both font_collection and
          // ui_font_collection. Section 07 must add a UiOnly constructor that only
          // takes ui_font_collection (or make font_collection Optional).

          // 7. Register with WindowManager
          self.window_manager.register(ManagedWindow {
              winit_id,
              kind: WindowKind::Dialog(kind.clone()),
              parent: Some(parent_winit_id),
              children: Vec::new(),
              visible: false,
          });

          // 8. Create DialogWindowContext and store
          let ctx = DialogWindowContext { /* ... */ };
          self.dialogs.insert(winit_id, ctx);

          // 9. Render first frame, then show
          self.render_dialog(winit_id);
          window.set_visible(true);

          Some(winit_id)
      }
  }
  ```

- [x] Implement `close_dialog()` method
  ```rust
  impl App {
      pub(super) fn close_dialog(&mut self, winit_id: WindowId) {
          // 1. Unregister from WindowManager
          self.window_manager.unregister(winit_id);

          // 2. Remove DialogWindowContext (drops GPU resources)
          self.dialogs.remove(&winit_id);

          // 3. Clear modal state on parent if this was modal
          // (platform_ops().clear_modal() called here)
      }
  }
  ```

- [x] Prevent duplicate dialogs — check if a dialog of the same kind is already open for this parent

---

## 04.3 Settings Dialog Migration

**File(s):** `oriterm/src/app/settings_overlay/mod.rs` → refactor to use dialog window

Replace the current overlay-based settings dialog with a real OS window.

- [x] Change settings dispatch to call `open_dialog(DialogKind::Settings)` instead of `open_settings_overlay()`

  Two call sites need updating (both currently call `self.open_settings_overlay()` directly):

  ```rust
  // Path 1: action_dispatch.rs line 228 (keybinding handler)
  // CURRENT: self.open_settings_overlay() — called directly.
  // PROBLEM: execute_action() does NOT have &ActiveEventLoop.
  //   Dialog creation needs the event loop to create a winit window.
  // FIX: Post TermEvent::OpenSettings to defer to user_event().
  // Before:
  Action::OpenSettings => {
      self.open_settings_overlay();
  }
  // After:
  Action::OpenSettings => {
      let _ = self.event_proxy.send_event(TermEvent::OpenSettings);
  }

  // Path 2: event_loop.rs line 287 (TermEvent handler — has &ActiveEventLoop)
  // Before:
  TermEvent::OpenSettings => {
      self.open_settings_overlay();
  }
  // After:
  TermEvent::OpenSettings => {
      if let Some(wid) = self.focused_window_id {
          self.open_dialog(event_loop, DialogKind::Settings, wid);
      }
  }
  ```

- [x] Migrate `SettingsPanel` widget to work inside `DialogWindowContext`
  - The widget itself is reusable — it draws a form with controls
  - Now receives events directly via `event_handling.rs` / `content_actions.rs`
  - Mouse clicks, hover, and scroll all route to the panel widget tree

- [x] Implement settings dialog event handling
  - `event_handling.rs`: routes WindowEvent to chrome, content, overlays
  - `content_actions.rs`: handles WidgetAction (Save, Cancel, dropdown, toggle)
  - Dropdown popups work within dialog's own `OverlayManager` + `LayerTree`

- [x] Handle settings "Save" action: apply config, close dialog window
  - `save_dialog_settings()` clones `pending_config`, applies via `apply_settings_change()`, persists via `config.save()`
- [x] Handle settings "Cancel" action: discard changes, close dialog window
  - `CancelSettings` action calls `close_dialog()` which discards pending config
- [x] Handle Escape key: same as Cancel (dismisses dropdown first if open)
- [x] Remove (or gate) the old overlay-based settings code
  - `open_settings_overlay()` retained with `#[allow(dead_code)]` as fallback

**Sync point -- all settings entry points:**
- [x] `Action::OpenSettings` sends `TermEvent::OpenSettings` (done in earlier work)
- [x] `TermEvent::OpenSettings` in `user_event()` calls `open_settings_dialog()` (done in earlier work)
- [x] Context menu "Settings" sends `TermEvent::OpenSettings` (verified)
- [x] All call sites updated — `open_settings_overlay()` is dead code (retained as fallback)

- [ ] Call `window.set_ime_allowed(true)` on dialog windows that contain text inputs (matching the pattern in `TermWindow::new()`)

---

## 04.4 Confirmation Dialog Migration

**File(s):** `oriterm/src/app/dialog_management.rs`

Migrate confirmation dialogs (e.g., close-with-running-process) from overlay to real window.

- [ ] Implement confirmation dialog creation
  ```rust
  pub(super) fn open_confirmation(
      &mut self,
      event_loop: &ActiveEventLoop,
      parent: WindowId,
      title: &str,
      message: &str,
      on_confirm: impl FnOnce() + Send + 'static,
  ) -> Option<WindowId> {
      self.open_dialog(event_loop, DialogKind::Confirmation, parent)
      // Configure the DialogContent::Confirmation with message and callback
  }
  ```

- [ ] Confirmation dialogs should be modal — block input to parent
  - Call `platform_ops().set_modal(dialog, parent)` after creation
  - Call `platform_ops().clear_modal(dialog, parent)` on close

- [ ] Handle OK button: invoke callback, close dialog
- [ ] Handle Cancel button: close dialog without invoking callback

---

## 04.5 Dialog Positioning and Chrome

**File(s):** `oriterm/src/app/dialog_management.rs`, `oriterm_ui/src/widgets/window_chrome/`

Dialogs need custom chrome (title bar) since they're frameless, and proper positioning.

- [x] Reuse `WindowChromeWidget` for dialog chrome
  - Settings dialog: title bar with close button only (no minimize/maximize)
  - Confirmation dialog: minimal title bar with close button
  - Added `ChromeMode` enum (Full vs Dialog) to `window_chrome/layout.rs`
  - Added `WindowChromeWidget::dialog()` constructor

- [x] Implement dialog dragging via title bar
  - Uses `window.drag_window()` (winit cross-platform API) on left-click in caption
  - Chrome button clicks intercepted, non-button caption clicks trigger drag

- [x] Center dialog on parent when opened
- [ ] Clamp dialog position to screen bounds (don't open off-screen)

- [x] Set min/max inner size for dialog windows:
  - Settings: `set_min_inner_size(Some(LogicalSize::new(600, 400)))` — prevent shrinking below usable size
  - Confirmation: `resizable: false` in WindowConfig — fixed size
  - Apply via `window.set_min_inner_size()` / `window.set_max_inner_size()` after window creation (avoids adding fields to `WindowConfig` in `oriterm_ui`)
- [x] Handle parent window move -- expected platform-dependent behavior (document, do not unify):
  - Windows (GWL_HWNDPARENT owner): dialog stays in place. Correct.
  - macOS (addChildWindow): dialog moves with parent. Correct.
  - Linux (transient_for): WM-dependent, usually stays in place.
- [x] Handle `WindowEvent::Resized` for dialog windows -- reconfigure wgpu surface (same pattern as `TermWindow::resize_surface` but on `DialogWindowContext`'s surface directly)

- [x] Handle `WindowEvent::ScaleFactorChanged` for dialog windows -- re-rasterize UI fonts at new DPI, reconfigure surface (DPI detection done, font re-rasterization TODO)

---

## 04.6 Completion Checklist

- [x] `DialogWindowContext` type defined and stores dialog-specific state
- [x] `open_dialog()` creates real OS window with platform native ownership and shadow
- [x] Settings dialog opens as a real OS window (not overlay)
- [x] Settings dialog can be moved outside main window bounds (via `drag_window()`)
- [x] Settings dialog has OS shadow on all platforms (via `enable_shadow()`)
- [x] Settings dialog stays above parent window (z-order via `set_owner()`)
- [x] Settings dialog closes when parent closes (cascading unregister)
- [ ] Confirmation dialogs work as real OS windows
- [ ] Confirmation dialogs are modal (block parent input)
- [x] Dialog chrome: custom title bar with close button, draggable
- [x] Duplicate dialog prevention (can't open two settings dialogs)
- [x] Old overlay-based settings code removed or gated
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Pressing the settings keybinding opens a real OS window centered on the parent terminal window. The dialog has an OS shadow, can be dragged anywhere on screen (including outside the parent), stays above the parent in z-order, and closes when the parent closes. Confirmation dialogs block input to the parent until dismissed. Works on Windows, macOS, and Linux.
