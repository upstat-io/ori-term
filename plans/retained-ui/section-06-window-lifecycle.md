---
section: "06"
title: "Window Lifecycle State Machine"
status: complete
goal: "Secondary windows (dialogs, tooltips, future panels) follow a framework-managed lifecycle: create hidden, render first frame, then show. No flash of uninitialized content on open, no leaked state on close."
inspired_by:
  - "Chromium Widget::Show() / Widget::Close() sequencing (ui/views/widget/widget.cc)"
  - "Ptyxis GtkWindow realize → map → first-frame → visible sequencing"
depends_on: []
sections:
  - id: "06.1"
    title: "SurfaceLifecycle State Machine"
    status: complete
  - id: "06.2"
    title: "Framework-Managed Visibility"
    status: complete
  - id: "06.3"
    title: "Integration with Dialog Management"
    status: complete
  - id: "06.4"
    title: "Completion Checklist"
    status: complete
reviewed: true
---

# Section 06: Window Lifecycle State Machine

**Status:** Not Started
**Goal:** The framework manages window visibility transitions — no secondary window is ever visible with uninitialized content, and no window leaks GPU state or event handlers after close. The lifecycle is: `CreatedHidden → Primed → Visible → Closing → Destroyed`.

**Context:** Today `finalize_dialog()` (dialog_management.rs:160) creates the context, stores it, renders the first frame (line 186), then calls `set_visible(true)` (line 191). This mostly works, but the visibility transition is manual and dialog-specific. On Windows, the code explicitly disables/enables DWM transitions to suppress the flash (dialog_management.rs:189-193). This is a per-dialog-kind hack that must be repeated for every new secondary window type.

The close path (`close_dialog()` at dialog_management.rs:204-230) also has manual sequencing: hide, clear modal, unregister, remove context. If any step is missed (e.g. a future window type forgets to call `clear_modal()`), state leaks.

The fix is a state machine that the framework drives. Window hosts transition through states, and each transition has guaranteed entry/exit actions. The framework enforces that `Visible` is only entered after a successful first-frame render, and `Destroyed` is only entered after `Closing` completes cleanup.

**Reference implementations:**
- **Chromium** `ui/views/widget/widget.cc`: `Show()` is a state transition that checks readiness. `Close()` is async — hides first, then destroys after the event loop settles.
- **Ptyxis**: GTK4 window realize → map → draw → visible pipeline. The window is not mapped until the first frame is drawn.

**Depends on:** Nothing — pure abstraction addition.

---

## 06.1 SurfaceLifecycle State Machine

**File(s):** `oriterm_ui/src/surface/mod.rs` (extending the module from Section 05)

- [x] Define `SurfaceLifecycle`:
  ```rust
  /// Lifecycle state for a secondary surface (dialog, tooltip, panel).
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum SurfaceLifecycle {
      /// OS window created, GPU surface configured, but not yet visible.
      /// Content is being built / first frame is being rendered.
      CreatedHidden,

      /// First frame rendered successfully. Ready to become visible.
      /// The framework will show the window on the next event loop tick.
      Primed,

      /// Window is visible and interactive.
      Visible,

      /// Close requested. Window is hidden, input suppressed.
      /// Cleanup (modal release, GPU teardown) is in progress.
      Closing,

      /// Fully destroyed. Context will be removed from the map.
      Destroyed,
  }
  ```

- [x] Define valid transitions:
  - `CreatedHidden → Primed` (first render succeeds)
  - `Primed → Visible` (framework shows window)
  - `Visible → Closing` (close requested)
  - `Closing → Destroyed` (cleanup complete)
  - `CreatedHidden → Destroyed` (creation failed, bail out)

- [x] Invalid transitions panic in debug mode (debug_assert).

- [x] **Event suppression:** Winit may deliver `CursorMoved`, `MouseInput`, `KeyboardInput`, and other events to a window before it transitions to `Visible` (OS-level window exists even if not shown). Events arriving when lifecycle is `CreatedHidden` or `Primed` must be suppressed -- early return from `handle_dialog_window_event()` (dialog_context/event_handling.rs:47). Events during `Closing` must also be suppressed (already specified in 06.3). Add a guard at the top of `handle_dialog_window_event()`:
  ```rust
  let lifecycle = ctx.lifecycle();
  if !matches!(lifecycle, SurfaceLifecycle::Visible) {
      return; // Suppress events outside Visible state.
  }
  ```
  Exception: `WindowEvent::Resized` and `WindowEvent::ScaleFactorChanged` must still be handled in `CreatedHidden` (the surface needs reconfiguration before the first render).

---

## 06.2 Framework-Managed Visibility

**File(s):** `oriterm/src/app/dialog_management.rs`, `oriterm/src/app/event_loop.rs`

Replace manual `set_visible(true)` calls with lifecycle-driven visibility.

- [x] In `finalize_dialog()` (dialog_management.rs:160), set lifecycle to `CreatedHidden`. Do NOT call `set_visible(true)` here (currently at line 191).

- [x] After `render_dialog()` succeeds (dialog_management.rs:186), transition to `Primed`.

- [x] In the event loop's `AboutToWait` handler (or equivalent winit callback), find all `Primed` windows and transition them to `Visible` by calling `set_visible(true)`. This guarantees the first frame is committed before the window appears.

- [x] Platform-specific flash suppression (disable/enable DWM transitions on Windows) is handled inside the lifecycle transition, not by each caller. The `CreatedHidden → Primed → Visible` chain encapsulates it.

---

## 06.3 Integration with Dialog Management

**File(s):** `oriterm/src/app/dialog_management.rs`

Refactor `close_dialog()` to use lifecycle transitions.

- [x] `close_dialog()` transitions to `Closing`:
  1. Set lifecycle to `Closing`.
  2. Call `set_visible(false)`.
  3. Clear modal state.
  4. Suppress input (events to `Closing` windows are dropped).

- [x] A follow-up pass (next event loop tick) transitions `Closing → Destroyed`:
  1. Unregister from window manager.
  2. Remove context (drops GPU resources).

- [x] This two-tick close sequence prevents the close callback from running inside a mutable borrow of the dialog context map (which can happen today if close triggers further events during cleanup). **Complexity warning:** The deferred destruction mechanism relies on winit's `AboutToWait` firing once per event batch. Verify this assumption in the winit documentation. If `AboutToWait` can fire mid-batch on some platforms, the pending_destroy drain could run while events for the destroyed window are still queued.

- [x] **Deferred destruction mechanism:** The `Closing → Destroyed` transition happens on the "next event loop tick". Implementation: collect windows in `Closing` state during `AboutToWait` (or the equivalent winit callback), then destroy them after all pending events are drained. Store a `Vec<WindowId>` of `pending_destroy` windows on `App`, populated during `close_dialog()` (which sets `Closing`), consumed during `AboutToWait`:
  ```rust
  // In AboutToWait handler:
  for wid in self.pending_destroy.drain(..) {
      // Unregister + remove context (drops GPU resources).
      self.window_manager.unregister(wid);
      self.dialogs.remove(&wid);
  }
  ```

- [x] **Cross-platform visibility:** macOS uses `NSWindow::orderFront` / `NSWindow::makeKeyAndOrderFront` which have different semantics than Windows `ShowWindow`. The lifecycle transition `Primed → Visible` must use the platform-appropriate visibility call. Currently `set_visible(true)` abstracts this via winit, which is sufficient. Document that `set_visible()` is the only call — no direct platform APIs for visibility.

---

## 06.4 Completion Checklist

**Sync point:** Adding `lifecycle: SurfaceLifecycle` to `DialogWindowContext` requires updating:
- `DialogWindowContext::new()` (dialog_context/mod.rs:135) -- initialize to `CreatedHidden`
- `handle_dialog_window_event()` (dialog_context/event_handling.rs:47) -- add lifecycle guard
- `render_dialog()` (dialog_rendering.rs:20) -- check lifecycle before rendering
- `App` struct -- add `pending_destroy: Vec<WindowId>` field
- `App::new()` -- initialize `pending_destroy: Vec::new()`
- `AboutToWait` handler in event loop -- drain `pending_destroy`

- [x] `SurfaceLifecycle` enum is defined with valid transition enforcement
- [x] `DialogWindowContext` carries a `lifecycle: SurfaceLifecycle` field (import from `oriterm_ui::surface::SurfaceLifecycle`)
- [x] `finalize_dialog()` uses lifecycle instead of manual `set_visible(true)`
- [x] `close_dialog()` uses lifecycle instead of manual `set_visible(false)` + immediate removal
- [x] No flash of uninitialized content when opening settings dialog (visual test)
- [x] No leaked modal state when closing confirmation dialog
- [x] Platform-specific visibility hacks are encapsulated in lifecycle transitions
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Opening and closing a Settings dialog follows the lifecycle state machine. No platform-specific `set_visible()` calls outside the lifecycle transition methods. Opening the dialog shows no flash — the window appears with content already rendered. Verified visually on Windows.
