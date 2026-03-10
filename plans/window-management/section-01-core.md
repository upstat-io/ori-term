---
section: "01"
title: "Window Manager Core"
status: complete
goal: "Central WindowManager that tracks all OS windows, their kinds, hierarchy, and lifecycle"
inspired_by:
  - "Chromium Aura WindowTreeHost/Window hierarchy (ui/aura/window.h)"
  - "WezTerm Mux window registry (mux/src/lib.rs)"
depends_on: []
sections:
  - id: "01.1"
    title: "Window Kind and ManagedWindow Types"
    status: complete
  - id: "01.2"
    title: "WindowManager Registry"
    status: complete
  - id: "01.3"
    title: "Window Hierarchy and Ownership"
    status: complete
  - id: "01.4"
    title: "Window Lifecycle Management"
    status: complete
  - id: "01.5"
    title: "Completion Checklist"
    status: complete
---

# Section 01: Window Manager Core

**Status:** Complete
**Goal:** A `WindowManager` that can create, register, look up, and destroy windows of any kind (main, dialog, tear-off), maintaining parent-child ownership relationships and providing a single point of truth for "what windows exist."

**Context:** Currently, `App` directly owns a `HashMap<WindowId, WindowContext>` and manages windows through methods in `oriterm/src/app/window_management.rs` (`create_window`, `create_window_bare`, `close_window`, `close_empty_session_window`, `remove_empty_window`). Tear-off windows use the same `create_window_bare()` but are initiated from `tab_drag/tear_off.rs`. There's no concept of window hierarchy or window kinds — all windows are terminal windows, and dialogs are overlays inside a window (pushed into `OverlayManager` via `push_modal()`). This section introduces the foundational types and registry that all subsequent sections build on.

**Reference implementations:**
- **Chromium** `ui/aura/window.h`: Window hierarchy with parent/children, owned-by-parent semantics, WindowDelegate for per-window behavior. Key insight: every visual element is a Window in a tree.
- **WezTerm** `mux/src/lib.rs`: `Mux` holds `HashMap<WindowId, Window>` as the single registry. Windows are model objects, not rendering objects.
- **Chromium** `ui/aura/client/transient_window_client.h`: Transient (owned) window relationships — dialogs destroyed when owner destroyed, always stack above owner.

**Depends on:** Nothing — this is the foundation.

**Impl-hygiene compliance:** `WindowManager` stores only metadata (IDs, kinds, hierarchy). It does NOT hold `winit::Window`, `wgpu::Surface`, `EventLoopProxy`, or any runtime resource. This makes it fully testable in headless `#[test]` without platform gymnastics.

---

## 01.1 Window Kind and ManagedWindow Types

**File(s):** `oriterm/src/window_manager/types.rs` (new)

Define the vocabulary types for the window management system. Every OS window in the application is represented as a `ManagedWindow` with a `WindowKind` discriminant.

- [x] Define `WindowKind` enum (no `Popup` variant — context menus and dropdowns stay as in-window overlays via `OverlayManager`)
  ```rust
  /// Discriminates the role and behavior of a managed window.
  pub enum WindowKind {
      /// Primary terminal window with tab bar, grid, chrome.
      /// Contains full terminal rendering pipeline.
      Main,
      /// Dialog window (settings, confirmation, about).
      /// Owned by a parent Main window. Has UI-only rendering.
      /// Destroyed when parent closes.
      Dialog(DialogKind),
      /// Tear-off window — a Main window created by dragging a tab
      /// out of an existing window. Behaviorally identical to Main
      /// after creation; the kind tracks origin for merge detection.
      TearOff,
  }

  /// Specific dialog types with their own content and behavior.
  pub enum DialogKind {
      Settings,
      Confirmation,
      About,
  }
  ```

- [x] Define `ManagedWindow` struct
  ```rust
  /// A tracked OS window in the window manager.
  pub struct ManagedWindow {
      /// Winit window ID (for event routing from the OS).
      pub winit_id: winit::window::WindowId,
      /// Window kind (determines behavior and rendering pipeline).
      pub kind: WindowKind,
      /// Parent window (for dialogs and initially for tear-offs).
      /// None for root-level main windows.
      pub parent: Option<winit::window::WindowId>,
      /// Child windows owned by this window.
      /// Destroyed when this window closes.
      pub children: Vec<winit::window::WindowId>,
      /// Whether the window is currently visible.
      pub visible: bool,
  }
  ```

- [x] **DECISION: No `WmWindowId` newtype.** Use `winit::window::WindowId` directly as the registry key. Winit's ID is already unique per OS window and is the key the event loop dispatches on. Adding a second ID layer creates mapping overhead with no benefit.

- [x] Derive `Debug`, `Clone`, `PartialEq`, `Eq` on `WindowKind` and `DialogKind`
- [x] Implement `WindowKind::is_main()`, `WindowKind::is_dialog()`, `WindowKind::is_tear_off()` convenience predicates
- [x] Verify types compile and satisfy `Debug`, `Clone`, `PartialEq` derives

---

## 01.2 WindowManager Registry

**File(s):** `oriterm/src/window_manager/mod.rs` (new)

The `WindowManager` is the single source of truth for all OS windows. It replaces the `App.windows: HashMap<WindowId, WindowContext>` as the registry, though `WindowContext` payloads still live in `App` during migration (Section 03).

- [x] Define `WindowManager` struct
  ```rust
  pub struct WindowManager {
      /// All managed windows, keyed by winit WindowId.
      windows: HashMap<winit::window::WindowId, ManagedWindow>,
      /// The currently focused window (set on Focused(true) events).
      focused_id: Option<winit::window::WindowId>,
  }
  ```

- [x] Implement core lookup methods
  ```rust
  impl WindowManager {
      pub fn new() -> Self { ... }

      /// Get a managed window by winit ID.
      pub fn get(&self, id: winit::window::WindowId) -> Option<&ManagedWindow> { ... }
      pub fn get_mut(&mut self, id: winit::window::WindowId) -> Option<&mut ManagedWindow> { ... }

      /// Iterate all windows of a specific kind.
      pub fn windows_of_kind(&self, kind_match: fn(&WindowKind) -> bool)
          -> impl Iterator<Item = &ManagedWindow> { ... }

      /// Get all main windows (for tab merge detection, etc.).
      pub fn main_windows(&self) -> impl Iterator<Item = &ManagedWindow> { ... }

      /// Get children of a window.
      pub fn children_of(&self, parent: winit::window::WindowId)
          -> impl Iterator<Item = &ManagedWindow> { ... }

      /// Check if a window exists.
      pub fn contains(&self, id: winit::window::WindowId) -> bool { ... }

      /// Total number of managed windows.
      pub fn len(&self) -> usize { ... }

      /// Total number of main windows (for "last window closes app" logic).
      pub fn main_window_count(&self) -> usize { ... }
  }
  ```

- [x] Write unit tests for registry CRUD operations

---

## 01.3 Window Hierarchy and Ownership

**File(s):** `oriterm/src/window_manager/hierarchy.rs` (new)

Implement parent-child relationships following Chromium's transient window pattern. When a parent window closes, all its children are closed first. Children always have a valid parent (or None for root windows).

- [x] Implement `register` method that establishes parent-child links
  ```rust
  impl WindowManager {
      /// Register a new window. If `parent` is Some, adds to parent's children list.
      pub fn register(&mut self, window: ManagedWindow) {
          let id = window.winit_id;
          if let Some(parent_id) = window.parent {
              if let Some(parent) = self.windows.get_mut(&parent_id) {
                  parent.children.push(id);
              }
          }
          self.windows.insert(id, window);
      }
  }
  ```

- [x] Implement `unregister` method with cascading child cleanup
  ```rust
  impl WindowManager {
      /// Unregister a window. Returns the window and all its descendants
      /// (depth-first) that must be closed, ordered children-first.
      pub fn unregister(&mut self, id: winit::window::WindowId)
          -> Vec<ManagedWindow>
      {
          let mut to_close = Vec::new();
          self.collect_descendants(id, &mut to_close);
          // Remove from parent's children list
          if let Some(window) = self.windows.get(&id) {
              if let Some(parent_id) = window.parent {
                  if let Some(parent) = self.windows.get_mut(&parent_id) {
                      parent.children.retain(|c| *c != id);
                  }
              }
          }
          // Remove self last
          if let Some(window) = self.windows.remove(&id) {
              to_close.push(window);
          }
          // Clear focus if any removed window was focused.
          if to_close.iter().any(|w| self.focused_id == Some(w.winit_id)) {
              self.focused_id = None;
          }
          to_close
      }

      fn collect_descendants(&mut self, id: winit::window::WindowId,
                              out: &mut Vec<ManagedWindow>) {
          let children = match self.windows.get(&id) {
              Some(window) => window.children.clone(),
              None => return,
          };
          // Clear the parent's children vec to avoid stale references.
          if let Some(window) = self.windows.get_mut(&id) {
              window.children.clear();
          }
          for child_id in children {
              self.collect_descendants(child_id, out);
              if let Some(child) = self.windows.remove(&child_id) {
                  out.push(child);
              }
          }
      }
  }
  ```

- [x] Implement `reparent` method (for tear-off merge: window becomes child-less after tab moves)
  ```rust
  impl WindowManager {
      /// Change a window's parent. Removes from old parent's children,
      /// adds to new parent's children.
      pub fn reparent(&mut self, id: winit::window::WindowId,
                       new_parent: Option<winit::window::WindowId>) { ... }
  }
  ```

- [x] Write tests for hierarchy: parent closes → children collected, reparent updates both parents

---

## 01.4 Window Lifecycle Management

**File(s):** `oriterm/src/window_manager/lifecycle.rs` (new)

Define the lifecycle state machine and the interface between WindowManager and the actual OS window operations.

- [x] Define `WindowRequest` enum for creation requests
  ```rust
  /// Request to create a new OS window through the window manager.
  pub struct WindowRequest {
      pub kind: WindowKind,
      pub parent: Option<winit::window::WindowId>,
      pub title: String,
      pub size: Option<(u32, u32)>,
      pub position: Option<(i32, i32)>,
      pub visible: bool,
      pub decorations: bool,
  }
  ```

- [x] Implement `request_window` that builds a `ManagedWindow` and returns an intent
  ```rust
  impl WindowManager {
      /// Prepare a window creation request. The caller (App) is responsible
      /// for actually creating the winit Window and GPU resources, then
      /// calling `register()` with the result.
      ///
      /// This split exists because winit window creation requires
      /// `&ActiveEventLoop` which WindowManager doesn't own.
      pub fn prepare_create(&self, request: &WindowRequest)
          -> WindowAttributes { ... }
  }
  ```

- [x] Implement `should_exit_on_close` logic
  ```rust
  impl WindowManager {
      /// Returns true if closing this window should exit the application.
      /// True when the last Main/TearOff window closes.
      pub fn should_exit_on_close(&self, id: winit::window::WindowId) -> bool {
          let remaining_main = self.windows.values()
              .filter(|w| w.winit_id != id)
              .filter(|w| matches!(w.kind, WindowKind::Main | WindowKind::TearOff))
              .count();
          remaining_main == 0
      }
  }
  ```

- [x] Implement `find_parent_for_dialog` helper
  ```rust
  impl WindowManager {
      /// Find the appropriate parent for a new dialog.
      /// Uses the focused main window, or falls back to any main window.
      pub fn find_dialog_parent(&self,
          focused: Option<winit::window::WindowId>
      ) -> Option<winit::window::WindowId> { ... }
  }
  ```

- [x] Write tests for lifecycle: exit-on-close logic, dialog parent finding

- [x] Test: `should_exit_on_close` returns false when a TearOff remains (TearOff counts as a "main" window)
- [x] Test: `find_dialog_parent` returns `None` when no main windows exist (all closed)
- [x] Test: `prepare_create` applies correct `WindowAttributes` per `WindowKind` (dialogs are not resizable by default, no maximize button)

---

## 01.5 Completion Checklist

- [x] `WindowKind`, `ManagedWindow`, `WindowManager` types compile
- [x] Registry CRUD (register, unregister, get, lookup) tested
- [x] Hierarchy (parent-child, cascading close, reparent) tested
- [x] Lifecycle helpers (should_exit, find_parent) tested
- [x] Module structure: `oriterm/src/window_manager/mod.rs` + `types.rs` + `hierarchy.rs` + `lifecycle.rs` + `tests.rs`
- [x] `#[cfg(test)] mod tests;` at bottom of `mod.rs`, sibling `tests.rs` with all WindowManager unit tests
- [x] If `hierarchy.rs` or `lifecycle.rs` exceed 200 lines and have their own tests, convert to directory modules (`hierarchy/mod.rs` + `hierarchy/tests.rs`) per test-organization rules
- [x] All files under 500 lines
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** `WindowManager` can register windows with parent-child relationships, unregister with cascading cleanup, and correctly determine exit-on-close behavior. All operations covered by unit tests. No integration with App yet — that's Section 03.
