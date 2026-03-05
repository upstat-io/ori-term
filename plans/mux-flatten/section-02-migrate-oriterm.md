---
section: "02"
title: "Migrate oriterm to Own Session Types"
status: not-started
goal: "All oriterm code uses local session types instead of mux-owned tab/window/layout types"
depends_on: ["01"]
sections:  # Listed in execution order (02.5 before 02.2 because sync is prerequisite for queries)
  - id: "02.1"
    title: "Swap ID Types"
    status: not-started
  - id: "02.5"
    title: "Synchronize Session State from Mux Events"
    status: not-started
  - id: "02.2"
    title: "Swap Session Queries"
    status: not-started
  - id: "02.3"
    title: "Swap Notification Handling"
    status: not-started
  - id: "02.4"
    title: "Update MuxBackend Consumers"
    status: not-started
  - id: "02.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Migrate oriterm to Own Session Types

**Status:** Not Started
**Goal:** Every import of `TabId`, `WindowId`, `MuxTab`, `MuxWindow`,
`SessionRegistry` in `oriterm` comes from `crate::session`, not from
`oriterm_mux`. The mux types are no longer consumed by the GUI.

**Context:** The GUI currently imports these types from `oriterm_mux` and
queries them via `mux.session()`. After this section, the GUI maintains its
own session state and only talks to the mux for pane operations.

**Depends on:** Section 01 (GUI session types must exist first).

**RISK: MEDIUM.** The semantic migration (02.2, 02.4, 02.5) is harder than
the mechanical import swaps (02.1, 02.3). The GUI must maintain its own
session state synchronized with mux pane events. **Recommended execution
order: 02.1 -> 02.5 -> 02.2 -> 02.3 -> 02.4 -> 02.6** (establish sync
mechanism before swapping queries and mutations).

---

## 02.1 Swap ID Types

**File(s):** All files in `oriterm/src/` that import `TabId` or `WindowId`
from `oriterm_mux`

Known consumers (from audit):
- `oriterm/src/window/mod.rs` — `WindowId as MuxWindowId`
- `oriterm/src/app/mod.rs` — `WindowId as MuxWindowId`
- `oriterm/src/app/window_management.rs` — `WindowId as MuxWindowId`
- `oriterm/src/app/tab_management/mod.rs` — `TabId, WindowId as MuxWindowId`
- `oriterm/src/app/pane_ops/helpers.rs` — `PaneId, TabId`
- `oriterm/src/app/mux_pump/mod.rs` — `PaneId, WindowId as MuxWindowId`
- `oriterm/src/app/constructors.rs` — `oriterm_mux::WindowId::from_raw` (inline qualified path)
- `oriterm/src/app/tab_drag/mod.rs`, `tear_off.rs`, `merge.rs` — `TabId`
- `oriterm/src/app/tab_management/move_ops.rs` — `TabId, WindowId as MuxWindowId`
- `oriterm/src/app/tests.rs` — `TabId, WindowId, MuxTab, MuxWindow`
- `oriterm/src/app/tab_management/tests.rs` — `TabId, WindowId, MuxWindow`

- [ ] Replace all `use oriterm_mux::{TabId, WindowId}` with
      `use crate::session::{TabId, WindowId}` across oriterm
- [ ] Remove the `as MuxWindowId` alias — it's now just `WindowId`
      (no collision since mux no longer exports it to GUI)
- [ ] Rename `TermWindow.mux_window_id` field and `mux_window_id()` accessor
      to `session_window_id` / `session_window_id()` (or keep the `mux_` prefix
      until fully migrated — renaming to bare `window_id` would collide with
      `winit::window::WindowId` which is already in scope as `WindowId`)
- [ ] Verify: `cargo build --target x86_64-pc-windows-gnu` succeeds

---

## 02.2 Swap Session Queries

**File(s):** `oriterm/src/app/tab_management/mod.rs`,
`oriterm/src/app/tests.rs`

The GUI currently calls `mux.session().get_window(id)` and
`mux.session().get_tab(id)` to read `MuxWindow`/`MuxTab`, where `mux`
is obtained from the `MuxBackend` trait object. These need to read from
the GUI's own `SessionRegistry` instead.

Known callers of `mux.session()` (from audit):
- `oriterm/src/app/mod.rs` — `active_pane_context()` helper
- `oriterm/src/app/pane_ops/mod.rs` — 3 calls
- `oriterm/src/app/pane_ops/helpers.rs` — 3 calls (`active_pane_context`)
- `oriterm/src/app/redraw/multi_pane.rs` — window + tab lookup
- `oriterm/src/app/tab_management/mod.rs` — `tab_count`, `window_for_tab`
- `oriterm/src/app/tab_drag/mod.rs` — window tab list
- `oriterm/src/app/tab_drag/tear_off.rs` — window tab list
- `oriterm/src/app/floating_drag.rs` — tab lookup (3 calls)

- [ ] Add `session: SessionRegistry` field to `App` struct
- [ ] Replace all `mux.session().get_window(id)` with
      `self.session.get_window(id)` throughout
- [ ] Replace all `mux.session().get_tab(id)` with
      `self.session.get_tab(id)` throughout
- [ ] Tab bar building reads from `self.session` instead of mux
- [ ] `active_pane_context()` in `pane_ops/helpers.rs` resolves
      window -> tab -> active_pane from local session
- [ ] Update test helpers to build local session state instead of
      injecting into mux
- [ ] **GPU consumer files** that import layout types from `oriterm_mux`:
  - `oriterm/src/gpu/pane_cache/mod.rs` — `oriterm_mux::layout::PaneLayout`
  - `oriterm/src/gpu/pane_cache/tests.rs` — `PaneLayout`, `Rect`
  - `oriterm/src/gpu/window_renderer/multi_pane.rs` — `DividerLayout`, `Rect`
  - `oriterm/src/app/redraw/multi_pane.rs` — `DividerLayout`, `LayoutDescriptor`, `PaneLayout`, `Rect`, `compute_all`
  - `oriterm/src/app/divider_drag.rs` — `SplitDirection`, `DividerLayout`, `Rect`
  - `oriterm/src/app/floating_drag.rs` — `Rect`, `snap_to_edge`
  - `oriterm/src/app/window_context.rs` — `DividerLayout`
  - `oriterm/src/app/pane_ops/helpers.rs` — `Rect`, `SplitDirection`
  - `oriterm/src/app/pane_ops/mod.rs` — `SplitDirection`, `Direction`
  These switch to `crate::session::` imports after section 04 lands.
  **Coordinate with section 04** so imports point to the right place.

---

## 02.3 Swap Notification Handling

**File(s):** `oriterm/src/app/mux_pump/mod.rs`

The mux currently emits `MuxNotification` variants that reference tabs and
windows. After flattening, the mux only emits pane-level notifications.
The GUI needs to translate pane events into its own session updates.

**Note:** These checklist items use the post-rename variant names (`PaneOutput`,
`PaneBell`) from section 03.2. At execution time, section 02.3 runs before
section 03.2. Handle these in the pre-rename form (`PaneDirty`, `Alert`)
during implementation, then update names when section 03.2 lands.

- [ ] Map `MuxNotification::PaneDirty(pid)` (renamed to `PaneOutput` in 03.2) to:
      mark pane content changed, invalidate selection/URL cache, request redraw
- [ ] Map `MuxNotification::PaneClosed(pid)` to: find which tab contains
      this pane (local session lookup), remove from split tree, handle
      last-pane-in-tab / last-tab-in-window / last-window cases locally
- [ ] Map `MuxNotification::PaneTitleChanged(pid)` to: update tab bar
- [ ] Map `MuxNotification::Alert(pid)` (renamed to `PaneBell` in 03.2) to:
      ring bell on tab bar
- [ ] Stop consuming: `TabLayoutChanged`, `FloatingPaneChanged`,
      `WindowTabsChanged`, `WindowClosed`, `LastWindowClosed` — replace
      with no-ops or remove match arms (these variants are deleted from
      the mux in section 03.2; removing consumers here unblocks that)
- [ ] The GUI generates its own "session changed" events internally
      when it mutates tabs/windows

---

## 02.4 Update MuxBackend Consumers

**File(s):** `oriterm/src/app/` (various)

The `MuxBackend` trait currently has methods for tab/window operations.
After flattening, it only has pane operations. The GUI's tab/window
operations become local mutations.

- [ ] Tab creation: `mux.create_tab(...)` becomes `mux.spawn_pane(...)` +
      local `Tab::new()` + `self.session.add_tab()` + `window.add_tab()`
- [ ] Tab closing: `mux.close_tab(...)` becomes local
      `self.session.remove_tab()` + `mux.close_pane(pid)` for each pane in the tab
- [ ] Window creation: `mux.create_window(...)` becomes local
      `Window::new()` + `self.session.add_window()`
- [ ] Tab moves: `mux.move_tab_to_window(...)` becomes local
      session mutations (remove from source window, add to target window)
- [ ] Splits: `mux.split_pane(...)` becomes `mux.spawn_pane(...)` for
      the new pane + local split tree mutation via `tab.replace_layout()`
- [ ] Verify: all tab/window/layout operations are local; only pane
      spawn/close/resize/write go through the mux

---

## 02.5 Synchronize Session State from Mux Events

The current `create_tab` flow is `GUI -> mux.create_tab(window_id, config, theme)` which
returns `(TabId, PaneId)`. After flattening, the mux has no tab concept, so the flow becomes:

1. GUI calls `mux.spawn_pane(config, theme)` to get `PaneId`
2. GUI creates a local `Tab` via `Tab::new(tab_id, pane_id)`
3. GUI registers the tab via `self.session.add_tab(tab)` and
   `self.session.get_window_mut(wid).add_tab(tab_id)`
4. GUI's session now owns the full state

- [ ] Implement the new spawn flow in `App`:
  - `spawn_pane` on mux returns `(PaneId, Pane)` (no tab context)
  - `App` creates a local `Tab`, inserts the pane, adds tab to window
- [ ] Implement the new close flow:
  - `App` removes pane from local session, calls `mux.close_pane(pane_id)`
  - `App` checks if tab/window is now empty and handles locally
- [ ] Implement the new split flow:
  - `App` calls `mux.spawn_pane()` for the new pane
  - `App` locally inserts the new pane into the tab's split tree
- [ ] Implement ID allocation for `TabId`/`WindowId` locally in `SessionRegistry`
  (the mux no longer allocates these)

---

## 02.6 Completion Checklist

- [ ] Zero imports of `TabId`, `WindowId`, `MuxTab`, `MuxWindow`,
      `SessionRegistry` from `oriterm_mux` in `oriterm/src/`
- [ ] `App` owns a local `SessionRegistry`
- [ ] All session queries read from local state
- [ ] All session mutations are local
- [ ] Only pane operations go through `MuxBackend`
- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes

**Exit Criteria:** `grep -r "oriterm_mux.*TabId\|oriterm_mux.*WindowId\|oriterm_mux.*MuxTab\|oriterm_mux.*MuxWindow\|oriterm_mux.*SessionRegistry" oriterm/src/`
returns zero results. All builds and tests green.
