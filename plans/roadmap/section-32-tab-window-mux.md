---
section: 32
title: Tab & Window Management (Mux-Aware)
status: in-progress
tier: 4M
goal: Multi-tab with mux integration, multi-window with shared GPU, tab CRUD, window lifecycle, cross-window tab movement, ConPTY-safe shutdown
sections:
  - id: "32.1"
    title: Mux-Aware Tab Management
    status: complete
  - id: "32.2"
    title: Multi-Window + Shared GPU
    status: in-progress
  - id: "32.3"
    title: Window Lifecycle
    status: not-started
  - id: "32.4"
    title: Cross-Window Operations
    status: not-started
  - id: "32.5"
    title: Section Completion
    status: not-started
---

# Section 32: Tab & Window Management (Mux-Aware)

**Status:** In Progress
**Goal:** Full tab and window management built on the mux layer. Multiple tabs per window, multiple windows with shared GPU device. Tab CRUD, window lifecycle with no-flash startup, DPI handling, ConPTY-safe cleanup. Cross-window tab movement preserving pane state.

**Crate:** `oriterm` (App, TermWindow), `oriterm_mux` (MuxTab, MuxWindow)
**Dependencies:** Section 31 (InProcessMux wired into App)
**Prerequisite:** Section 31 complete.

**Absorbs:** Section 15.2 (Tab Management Operations) and Section 18 (Multi-Window & Window Lifecycle). All hard-won patterns preserved: ConPTY-safe shutdown ordering, exit-before-drop, no-flash window creation, DPI handling, Aero Snap, background thread cleanup, CWD inheritance.

---

## 32.1 Mux-Aware Tab Management

Tab CRUD operations that go through the mux layer. The mux owns tab state (MuxTab with SplitTree); the GUI owns rendering state (tab bar layout, animation offsets).

**File:** `oriterm/src/app/tab_management.rs`

**Reference:** `_old/src/app/tab_management.rs`, Section 15.2 design (preserved patterns)

- [x] New tab:
  - [x] `App::new_tab_in_window(&mut self, window_id: WindowId)`
  - [x] Inherit CWD from active pane in current tab (via `pane.cwd()`)
  - [x] Build `SpawnConfig` with shell, scrollback, cursor shape from config
  - [x] Call `mux.create_tab(mux_window_id, config)` — creates MuxTab with one Leaf pane
  - [x] Map mux `TabId` → GUI tab bar entry
  - [x] Clear `tab_width_lock` (tab count changed)
  - [x] Mark `tab_bar_dirty`, request redraw
- [x] Close tab:
  - [x] `App::close_tab(&mut self, tab_id: TabId)`
  - [x] Call `mux.close_tab(tab_id)` — closes all panes in tab, updates MuxWindow
  - [x] If window now empty and last terminal window: call `shutdown()` **immediately** (ConPTY)
  - [x] If window now empty but other windows exist: close the empty window
  - [x] Background thread drops for all Pane structs
  - [x] Mark `tab_bar_dirty`
- [x] Duplicate tab:
  - [x] `App::duplicate_active_tab(&mut self)`
  - [x] Clone CWD from source tab's active pane
  - [x] Create new tab via mux (fresh shell, inherited directory)
- [x] Cycle tabs:
  - [x] `App::cycle_tab(&mut self, delta: isize)`
  - [x] Update `MuxWindow.active_tab` via mux: wrapping arithmetic
  - [x] Clear bell badge on newly active tab
  - [x] Mark dirty, request redraw
- [x] Switch to specific tab:
  - [x] `App::switch_to_tab(&mut self, tab_id: TabId)` — find window, set active
- [x] Reorder tabs:
  - [x] `App::move_tab(&mut self, from: usize, to: usize)` (wired to drag in Section 17)
  - [x] Update `MuxWindow.tabs` vec order via mux
  - [x] Adjust `active_tab` index to track the same tab
- [x] Auto-close on PTY exit:
  - [x] `MuxEvent::PaneExited` → `close_pane` → tab auto-removed if last pane → `WindowTabsChanged`/`LastWindowClosed`

**Tests:**
- [x] Create 3 tabs: IDs are unique, window contains all 3
- [x] Close middle tab: remaining tabs order preserved, active_tab adjusts
- [x] Cycle wrap: tab 2 of 3 → next → tab 0
- [x] CWD inheritance: new tab starts in active pane's directory (via CWD in SpawnConfig)
- [x] Closing last tab in last window triggers `shutdown()`
- [x] Pane drop on background thread (via `std::thread::spawn(move || drop(pane))`)

---

## 32.2 Multi-Window + Shared GPU

Multiple windows, each a thin GUI shell. All windows share the same GPU device, font collection, and config. The mux tracks window state; the GUI maps `winit::window::WindowId` to `oriterm_mux::WindowId`.

**File:** `oriterm/src/app/window_management.rs`, `oriterm/src/window.rs`

**Reference:** `_old/src/app/window_management.rs`, Section 18.1 design (preserved patterns)

- [x] `TermWindow` struct (GUI-level window):
  - [x] `winit_window: Arc<Window>` — winit window handle
  - [x] `surface: wgpu::Surface<'static>` — GPU surface
  - [x] `surface_config: wgpu::SurfaceConfiguration`
  - [x] `mux_window_id: WindowId` — link to mux MuxWindow
  - [x] `is_maximized: bool`
  - [x] `scale_factor: f64` — current DPI scale
- [x] Window ID mapping:
  - [x] `App::windows: HashMap<winit::window::WindowId, TermWindow>` — maps winit ID → TermWindow (which contains mux_window_id)
  - [x] `App::focused_window_id: Option<winit::window::WindowId>` — tracks focused OS window
- [x] `TermWindow` methods:
  - [x] `resize_surface(&mut self, width: u32, height: u32, gpu: &GpuState)` — reconfigure surface
- [x] Shared resources across windows:
  - [x] `GpuState` (device, queue, adapter) — created once, shared
  - [x] `FontCollection` — created once, shared (rebuilt on DPI change)
  - [x] `GlyphAtlas` — created once, shared across windows
  - [x] Config — single source of truth
- [x] Focus tracking:
  - [x] `WindowEvent::Focused(true)` → send focus-in to active pane's terminal (if `FOCUS_IN_OUT` mode)
  - [x] `WindowEvent::Focused(false)` → send focus-out

**Tests:**
- [ ] Create two windows: both share same GPU device <!-- blocked-by:32.3 -->
- [x] Focus tracking: mode gating and multi-window session tests verify focus event dispatch
- [x] Window ID mapping: multi-window session tests verify mux ID → pane resolution per window

---

## 32.3 Window Lifecycle

Window creation, resize, DPI changes, and destruction. All operations coordinated with the mux.

**File:** `oriterm/src/app/window_management.rs`

**Reference:** Section 18.2 design (all patterns preserved)

- [ ] `create_window(&mut self, event_loop: &ActiveEventLoop, visible: bool) -> Option<WindowId>`
  - [ ] Calculate window size from font metrics + grid dimensions + `TAB_BAR_HEIGHT`
  - [ ] Request transparency if opacity < 1.0
  - [ ] Enable `WS_EX_NOREDIRECTIONBITMAP` on Windows
  - [ ] Create winit window
  - [ ] Capture initial DPI scale factor
  - [ ] **First window only**: initialize `GpuState` and `GpuRenderer`
  - [ ] Create wgpu `Surface` for this window
  - [ ] **Render clear frame BEFORE showing** (prevent gray/white flash):
    1. Build black/themed background frame
    2. Submit to GPU
    3. `device.poll(Maintain::Wait)` — synchronous
  - [ ] Apply compositor effects (Mica/acrylic on Windows, vibrancy on macOS)
  - [ ] Enable Aero Snap on Windows (WndProc subclass for `WM_NCHITTEST`)
  - [ ] Register mux window: `mux.create_window()` → `WindowId`
  - [ ] Map winit `WindowId` ↔ mux `WindowId`
  - [ ] Show window
- [ ] `handle_resize(&mut self, winit_id: winit::window::WindowId, width: u32, height: u32)`
  - [ ] Map to mux WindowId, get TermWindow
  - [ ] Clear `tab_width_lock`
  - [ ] Resize wgpu surface
  - [ ] If DPI changed: reload fonts, rebuild atlas
  - [ ] Compute new grid dimensions
  - [ ] **Resize ALL panes in ALL tabs of this window** (not just active):
    - [ ] For each tab in window, compute layout with new dimensions
    - [ ] Resize each pane's PTY with its per-pane cell dimensions
  - [ ] Mark dirty, request redraw
- [ ] `close_window(&mut self, winit_id: winit::window::WindowId, event_loop: &ActiveEventLoop)`
  - [ ] Map to mux WindowId
  - [ ] If **last** terminal window: call `exit_app()` **before** dropping panes (ConPTY)
  - [ ] Close all tabs via mux: `mux.close_window(window_id)`
  - [ ] Drop all Pane structs on background threads
  - [ ] Remove TermWindow and ID mappings
- [ ] `exit_app(&mut self)`
  - [ ] Save window positions to disk
  - [ ] Save GPU pipeline cache to disk
  - [ ] Shutdown all panes
  - [ ] Release mouse capture
  - [ ] `process::exit(0)` — **must not return**
- [ ] Fullscreen toggle:
  - [ ] Query `window.fullscreen()`, toggle between `Some(Borderless(None))` and `None`
  - [ ] Wired to `Action::ToggleFullscreen` keybinding
- [ ] DPI change:
  - [ ] `handle_scale_factor_changed(&mut self, winit_id, new_scale: f64)`
  - [ ] Reload fonts at `config.font.size * new_scale`
  - [ ] Rebuild glyph atlas
  - [ ] Resize all panes in all windows (cell size changed)

**Tests:**
- [ ] No-flash: window opens with themed background, no gray/white flash
- [ ] DPI change: fonts reload, grids reflow, no artifacts
- [ ] Multi-window: tear-off creates new window, close last tab closes window
- [ ] Exit ordering: last window → `exit_app()` before dropping panes
- [ ] Resize: all panes in all tabs resized, not just active

---

## 32.4 Cross-Window Operations

Move tabs between windows. Tab identity (TabId) preserved — same panes, same layout tree, different window.

**File:** `oriterm/src/app/window_management.rs`

- [ ] `move_tab_to_window(&mut self, tab_id: TabId, target_window: WindowId)`
  - [ ] Remove tab from source `MuxWindow.tabs`
  - [ ] Add to target `MuxWindow.tabs`
  - [ ] If source window now empty: close it (unless it's the last)
  - [ ] Resize all panes in moved tab to target window dimensions
  - [ ] Mark both windows dirty
- [ ] `move_tab_to_new_window(&mut self, tab_id: TabId, event_loop: &ActiveEventLoop)`
  - [ ] Refuse if it's the last tab in the last window
  - [ ] Create new window via `create_window()`
  - [ ] Move tab to new window
- [ ] Tab tear-off integration (built on Section 17 drag infrastructure):
  - [ ] Drag tab beyond `TEAR_OFF_THRESHOLD` → `move_tab_to_new_window`
  - [ ] Drag tab to another window → `move_tab_to_window`
  - [ ] Multi-pane tabs move as a unit (entire SplitTree preserved)

**Tests:**
- [ ] Move tab from window A to window B: tab appears in B, removed from A
- [ ] Move tab: panes resized to target window dimensions
- [ ] Move last tab: source window closes (not the app, if other windows exist)
- [ ] Tear-off: creates new window with the dragged tab
- [ ] Multi-pane tab: split layout preserved after cross-window move

---

## 32.5 Section Completion

- [ ] All 32.1–32.4 items complete
- [ ] Tab management: create, close, duplicate, cycle, reorder — all through mux
- [ ] Multi-window: shared GPU, font collection, config. Correct lifecycle.
- [ ] No-flash window startup, DPI handling, Aero Snap
- [ ] ConPTY-safe shutdown: exit_app before drop, background thread cleanup
- [ ] Cross-window tab movement preserves pane state and layout tree
- [ ] `cargo build --target x86_64-pc-windows-gnu` — compiles
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all tests pass
- [ ] **Tab lifecycle test**: create 5 tabs, close 3, cycle remaining, verify state
- [ ] **Multi-window test**: 2 windows, move tab between, close one window
- [ ] **Stress test**: rapidly create/close tabs — no freeze, no orphaned PTYs

**Exit Criteria:** Complete tab and window management through the mux layer. All patterns from superseded Sections 15 and 18 are implemented: ConPTY safety, no-flash startup, DPI handling, CWD inheritance, background thread drops, exit-before-drop ordering. Cross-window tab movement works with multi-pane tabs.
