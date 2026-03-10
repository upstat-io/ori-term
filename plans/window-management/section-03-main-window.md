---
section: "03"
title: "Main Window Migration"
status: complete
goal: "Fold existing TermWindow/WindowContext into the WindowManager so main windows are managed through the same system as dialogs and tear-offs"
inspired_by:
  - "WezTerm TermWindow binding to Mux WindowId (wezterm-gui/src/termwindow/mod.rs)"
  - "Chromium WindowTreeHost as root window anchor (ui/aura/window_tree_host.h)"
depends_on: ["01", "02"]
sections:
  - id: "03.1"
    title: "WindowManager Integration into App"
    status: complete
  - id: "03.2"
    title: "Window Creation Through WindowManager"
    status: complete
  - id: "03.3"
    title: "Window Closure Through WindowManager"
    status: complete
  - id: "03.4"
    title: "Event Loop Integration"
    status: complete
  - id: "03.5"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Main Window Migration

**Status:** Complete
**Goal:** All main terminal windows are created, tracked, and destroyed through the `WindowManager`. The `App.windows` HashMap is replaced by (or backed by) the WindowManager registry. Existing single-window and multi-window behavior is preserved exactly — this is a pure refactor with no user-visible changes.

**Context:** Currently `App` directly owns `windows: HashMap<WindowId, WindowContext>` with window lifecycle methods (`create_window()`, `create_window_bare()`, `close_window()`, `close_empty_session_window()`, `remove_empty_window()`) centralized in `oriterm/src/app/window_management.rs`. The tear-off path in `tab_drag/tear_off.rs` calls `create_window_bare()` directly without any registry beyond the `windows` HashMap. The `about_to_wait` handler iterates dirty windows with a temporary focus-swap pattern (saves/restores `focused_window_id` + `active_window` per dirty window). All of this needs to flow through `WindowManager` so that when we add dialog windows (Section 04) and unify tear-offs (Section 05), they use the same code paths.

**Reference implementations:**
- **WezTerm** `wezterm-gui/src/termwindow/mod.rs`: `TermWindow` holds `mux_window_id` (model reference) and owns rendering state. The Mux registry is separate from the GUI window state.
- **Chromium** `ui/aura/window_tree_host.h`: Root window host owns the native window handle and compositor. Created independently, then registered with the environment.

**Depends on:** Section 01 (WindowManager types), Section 02 (platform native ops for shadow/snap on main window).

---

## 03.1 WindowManager Integration into App

**File(s):** `oriterm/src/app/mod.rs`

Add `WindowManager` as a field on `App` alongside the existing `windows` HashMap. During migration, both coexist; at the end, the HashMap is accessed through WindowManager.

**WARNING: `oriterm/src/app/mod.rs` is already at the 500-line limit (501 lines).** Adding the `window_manager` field declaration is acceptable (1 line + 1 import). Any new methods related to WindowManager must go in `window_management.rs` or a new submodule, NOT in `mod.rs`.

- [ ] Add `window_manager: WindowManager` field to `App`
  ```rust
  pub(crate) struct App {
      window_manager: WindowManager,
      // windows: HashMap<WindowId, WindowContext> — kept during migration,
      // eventually WindowContext is stored alongside ManagedWindow or
      // in a parallel HashMap keyed the same way.
      windows: HashMap<WindowId, WindowContext>,
      // ... rest unchanged
  }
  ```

- [ ] Initialize `WindowManager::new()` in `App::new()` and `App::new_daemon()` (`oriterm/src/app/constructors.rs`) — the struct field is constructed here. Register the initial window in `try_init()` (`oriterm/src/app/init/mod.rs`) after the first window is created.
- [ ] Register the initial window in WindowManager during init
  ```rust
  // In init, after creating TermWindow:
  self.window_manager.register(ManagedWindow {
      winit_id: winit_id,
      kind: WindowKind::Main,
      parent: None,
      children: Vec::new(),
      visible: true,
  });
  ```

- [ ] Apply platform native ops to initial window (shadow already handled by `enable_snap`, verify)

---

## 03.2 Window Creation Through WindowManager

**File(s):** `oriterm/src/app/window_management.rs`

Migrate `create_window()` and `create_window_bare()` to route through WindowManager.

- [ ] Refactor `create_window()` to register with WindowManager
  ```rust
  pub(super) fn create_window(&mut self, event_loop: &ActiveEventLoop)
      -> Option<WindowId>
  {
      // 1. Create OS window (unchanged — winit + wgpu surface)
      let (winit_id, session_wid) = self.create_window_bare(event_loop)?;

      // 2. Register with WindowManager
      self.window_manager.register(ManagedWindow {
          winit_id,
          kind: WindowKind::Main,
          parent: None,
          children: Vec::new(),
          visible: false, // shown after first render
      });

      // 3. Apply platform native ops (main windows get shadow via enable_snap()
      //    during init — no separate enable_shadow() needed for Main/TearOff).

      // 4. Create tab, pane, show (unchanged)
      // ...

      Some(winit_id)
  }
  ```

- [ ] Refactor `create_window_bare()` — this is used by both `create_window()` and tear-off. Keep it as the low-level "create OS window + GPU resources" path. WindowManager registration happens in the caller.

- [ ] **Clarify dual-map design**: `self.windows` (HashMap of WindowContext) stores rendering state; `self.window_manager` (WindowManager) stores metadata (kind, parent, children, visibility). Both are keyed by winit `WindowId`. The WindowManager does NOT replace `self.windows` — it supplements it. `create_window_bare()` continues to insert into `self.windows`.
- [ ] Ensure every call to `self.windows.insert()` is followed by `self.window_manager.register()` (or vice versa) — keep the two maps in sync
- [ ] Ensure every call to `self.windows.remove()` is preceded by `self.window_manager.unregister()` — keep the two maps in sync

---

## 03.3 Window Closure Through WindowManager

**File(s):** `oriterm/src/app/window_management.rs`, `oriterm/src/app/event_loop.rs`

Migrate `close_window()` to route through WindowManager, which handles cascading child cleanup.

- [ ] Refactor `close_window()` to use WindowManager
  ```rust
  pub(super) fn close_window(&mut self, winit_id: WindowId,
                               event_loop: &ActiveEventLoop) {
      // 1. Ask WindowManager to unregister (returns self + descendants)
      let to_close = self.window_manager.unregister(winit_id);

      // 2. Close each window (children first, parent last)
      for managed in &to_close {
          // Remove WindowContext
          if let Some(ctx) = self.windows.remove(&managed.winit_id) {
              // Close tabs/panes associated with this window
              self.close_window_contents(&ctx);
          }
      }

      // 3. Check if app should exit
      if self.window_manager.main_window_count() == 0 {
          event_loop.exit();
      }
  }
  ```

- [ ] Handle `CloseRequested` event — check if window has children, close them first
- [ ] Handle dialog parent closing — dialogs should close before parent

- [ ] Migrate `exit_app()` check from `self.windows.len() <= 1` to `self.window_manager.main_window_count() <= 1` — the `self.windows` map only contains terminal windows, but `self.dialogs` (Section 04) will add non-terminal entries. The window manager's `main_window_count()` is the correct check. (Note: `exit_app()` uses `process::exit(0)` which is a hard kill -- dialog GPU resources won't be dropped cleanly, but this is acceptable because ConPTY safety requires it.)
- [ ] Update `close_empty_session_window()` to use WindowManager for the exit check (same `main_window_count()` pattern)
- [ ] Update `remove_empty_window()` to call `self.window_manager.unregister(winit_id)` before removing from `self.windows`

---

## 03.4 Event Loop Integration

**File(s):** `oriterm/src/app/event_loop.rs`

The event loop routes winit events by `WindowId`. Verify that WindowManager lookups are used for window-kind-specific routing.

- [ ] Add WindowManager lookup in event dispatch
  ```rust
  WindowEvent::CloseRequested => {
      // Check window kind for kind-specific close behavior
      match self.window_manager.get(window_id) {
          Some(managed) => match &managed.kind {
              WindowKind::Dialog(_) => {
                  // Dialog close: just close the dialog, don't exit app
                  self.close_dialog(window_id);
              }
              _ => {
                  // Main/TearOff close: may exit app
                  self.close_window(window_id, event_loop);
              }
          },
          None => {} // Unknown window, ignore
      }
  }
  ```

- [ ] Verify `about_to_wait` dirty-window iteration still works — it should iterate `self.windows` (WindowContext map) which is still the rendering-state map

- [ ] Verify `Focused` event updates WindowManager's awareness of focused window (for dialog parent finding)

- [ ] No behavioral changes — existing single-window and tear-off behavior must be identical

- [ ] Verify `transfer_focus_from()` still works correctly -- it iterates `self.windows.keys()` which is fine during this section (no dialogs yet), but will need updating in Section 06 to prefer main windows over dialogs

- [ ] Add `// TODO(window-management/04): also mark self.dialogs dirty` comment in `mark_all_windows_dirty()` for future Section 04 work

---

## 03.5 Completion Checklist

- [ ] `App` has `window_manager: WindowManager` field
- [ ] Initial window registered in WindowManager during init
- [ ] `create_window()` registers with WindowManager
- [ ] `close_window()` uses WindowManager for cascading close
- [ ] Event loop routes `CloseRequested` through WindowManager for kind-specific behavior
- [ ] Existing single-window behavior unchanged (manual test)
- [ ] Existing tear-off behavior unchanged (manual test on Windows)
- [ ] Dual-map invariant verified: every key in `self.windows` has a corresponding entry in `self.window_manager`, and vice versa (for Main/TearOff kinds). Add a `debug_assert!` in `about_to_wait` to validate this during development.
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green

**Exit Criteria:** All existing main windows and tear-off windows are registered in WindowManager. Creating and closing windows flows through WindowManager. No user-visible behavior changes. The system is ready for dialog windows (Section 04) and tear-off unification (Section 05) to plug into the same WindowManager.
