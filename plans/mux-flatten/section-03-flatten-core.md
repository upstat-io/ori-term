---
section: "03"
title: "Flatten Mux Core"
status: not-started
goal: "oriterm_mux has zero tab/window/session concepts — only pane lifecycle and I/O"
depends_on: ["02"]
sections:
  - id: "03.1"
    title: "Strip InProcessMux"
    status: not-started
  - id: "03.2"
    title: "Simplify MuxNotification"
    status: not-started
  - id: "03.3"
    title: "Remove Session Types"
    status: not-started
  - id: "03.4"
    title: "Remove ID Types"
    status: not-started
  - id: "03.5"
    title: "Flatten PaneRegistry"
    status: not-started
  - id: "03.6"
    title: "Purge UI Comments"
    status: not-started
  - id: "03.7"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Flatten Mux Core

**Status:** Not Started
**Goal:** `oriterm_mux` contains zero references to tabs, windows, sessions,
layouts, GUI, winit, tab bars, or any presentation concept. It is a flat
pane server.

**Context:** After Section 02, no external code depends on mux session types.
This section deletes them.

**Depends on:** Section 02 (all consumers migrated away).

**RISK: HIGH.** This section modifies the mux's public API contracts
(`ClosePaneResult`, `spawn_pane` signature, `PaneEntry` fields,
`MuxNotification` variants). The test file `in_process/tests.rs` is
5,310 lines. Execute subsections in strict order with build+test
verification between each step.

---

## 03.1 Strip InProcessMux

**File(s):** `oriterm_mux/src/in_process/mod.rs`,
`oriterm_mux/src/in_process/event_pump.rs`,
`oriterm_mux/src/in_process/tab_ops.rs`,
`oriterm_mux/src/in_process/floating_ops.rs`

The `InProcessMux` currently orchestrates pane/tab/window CRUD. After
flattening, it only does pane CRUD.

- [ ] Delete `in_process/tab_ops.rs` entirely (split, zoom, equalize —
      all GUI operations now)
- [ ] Delete `in_process/floating_ops.rs` entirely (floating pane CRUD
      is GUI-owned)
- [ ] Strip `event_pump.rs` of all tab/window operations:
  - Delete `active_tab_id()`, `set_active_pane()` (session-level),
    `session()` (returns `&SessionRegistry`),
    `switch_active_tab()`, `cycle_active_tab()`, `reorder_tab()`,
    `move_tab_to_window()`, `move_tab_to_window_at()`, `move_tab_impl()`
  - Keep: `poll_events()`, `drain_notifications()`, `discard_notifications()`,
    `pane_registry()`, `event_tx()`, `default_domain()`
  - Remove imports: `use crate::{DomainId, MuxWindow, PaneId, SessionRegistry, TabId, WindowId};`
    becomes `use crate::{DomainId, PaneId};`
  - Remove `use crate::domain::Domain;` (only used by `default_domain()` which stays,
    but Domain trait is used via `self.local_domain.id()` — verify import is still needed)
- [ ] Strip `InProcessMux` struct of:
  - `session: SessionRegistry` field
  - `tab_alloc: IdAllocator<TabId>` field
  - `window_alloc: IdAllocator<WindowId>` field
  - All `create_window()`, `close_window()`, `create_tab()`,
    `close_tab()` methods
  - `handle_window_after_tab_removal()` helper
- [ ] Simplify `close_pane()` — no longer needs to check tab/window
      membership. Just unregisters from `PaneRegistry` and returns.
- [ ] Simplify `spawn_pane()` — remove the `tab_id: TabId` first parameter.
      Current signature: `spawn_pane(&mut self, tab_id: TabId, config: &SpawnConfig, theme: Theme, wakeup: &Arc<...>)`.
      New signature: `spawn_pane(&mut self, config: &SpawnConfig, theme: Theme, wakeup: &Arc<...>)`.
      Also remove `tab: tab_id` from the `PaneEntry` registration.
- [ ] Simplify `ClosePaneResult` enum:
  - Remove `TabClosed { tab_id: TabId }` variant (no tabs)
  - Remove `LastWindow` variant (GUI decides when to exit)
  - Keep: `PaneRemoved`, `NotFound`
- [ ] Delete test helpers: `inject_test_tab()`, `inject_split()`
- [ ] Rewrite `in_process/tests.rs`:
  - Rewrite `one_pane_setup()` / `two_pane_setup()`: remove MuxTab/MuxWindow/
    SessionRegistry setup, use flat `spawn_pane()` instead
  - Remove tests for tab/window operations (create_window, close_window,
    create_tab, close_tab, switch_active_tab, etc.)
  - Keep tests for pane lifecycle (spawn, close, is_last_pane) with updated setup
  - Remove `TabId`, `WindowId`, `MuxTab`, `MuxWindow` imports
- [ ] Update `InProcessMux::new()` — no session, no tab/window allocators
- [ ] Rewrite `is_last_pane()`: change from `self.session.is_last_pane(pane_id)` to
      `self.pane_registry.len() == 1 && self.pane_registry.get(pane_id).is_some()`

---

## 03.2 Simplify MuxNotification

**File(s):** `oriterm_mux/src/mux_event/mod.rs`

- [ ] Remove these tab/window variants from `MuxNotification`:
  - `TabLayoutChanged(TabId)` — GUI tracks its own layout
  - `FloatingPaneChanged(TabId)` — GUI tracks its own floating state
  - `WindowTabsChanged(WindowId)` — GUI tracks its own tab lists
  - `WindowClosed(WindowId)` — GUI manages windows
  - `LastWindowClosed` — GUI decides when to exit
- [ ] Rename `PaneDirty(PaneId)` to `PaneOutput(PaneId)` (matches `MuxEvent::PaneOutput`)
- [ ] Rename `Alert(PaneId)` to `PaneBell(PaneId)` (matches `MuxEvent::PaneBell`)
- [ ] Remaining variants:
  - `PaneOutput(PaneId)`
  - `PaneClosed(PaneId)`
  - `PaneTitleChanged(PaneId)`
  - `PaneBell(PaneId)`
  - `CommandComplete { pane_id, duration }`
  - `ClipboardStore { pane_id, clipboard_type, text }`
  - `ClipboardLoad { pane_id, clipboard_type, formatter }`
- [ ] Update `Debug` impl to match new variants
- [ ] Remove `TabId` and `WindowId` from `use crate::{PaneId, TabId, WindowId};`
      import (only `PaneId` remains)
- [ ] Update emit sites in `event_pump.rs`:
  - `MuxNotification::PaneDirty(id)` -> `MuxNotification::PaneOutput(id)`
  - `MuxNotification::Alert(id)` -> `MuxNotification::PaneBell(id)`
- [ ] Update `mux_event/tests.rs`:
  - Remove Debug format tests for deleted variants
  - Update tests for renamed variants (`PaneDirty` -> `PaneOutput`, `Alert` -> `PaneBell`)
  - Remove `TabId`, `WindowId` imports from tests
- [ ] Update module doc: "Pane lifecycle notifications" not
      "mux-to-GUI notifications"

---

## 03.3 Remove Session Types

**File(s):** `oriterm_mux/src/session/mod.rs`,
`oriterm_mux/src/session/tests.rs`

- [ ] Delete `oriterm_mux/src/session/` entirely (includes `mod.rs` and `tests.rs`)
- [ ] Remove `pub mod session;` from `lib.rs`
- [ ] Remove `pub use session::{MuxTab, MuxWindow};` from `lib.rs`

---

## 03.4 Remove ID Types

**File(s):** `oriterm_mux/src/id/mod.rs`

- [ ] Remove `TabId`, `WindowId`, `SessionId` from `id/mod.rs`
- [ ] Remove their `MuxId` impls, `sealed::Sealed` impls, `Display` impls,
      `from_raw`/`raw` convenience impls
- [ ] Remove `IdAllocator<TabId>`, `IdAllocator<WindowId>`,
      `IdAllocator<SessionId>` (generic impl stays, just fewer instantiations)
- [ ] Update `lib.rs` re-export: change
      `pub use id::{ClientId, DomainId, IdAllocator, MuxId, PaneId, SessionId, TabId, WindowId};`
      to `pub use id::{ClientId, DomainId, IdAllocator, MuxId, PaneId};`
- [ ] Remove `sealed::Sealed` impls for `TabId`, `WindowId`, `SessionId`
- [ ] Update `id/tests.rs`: remove tests for `TabId`, `WindowId`, `SessionId`
      (keep tests for `PaneId`, `DomainId`, `ClientId`)
- [ ] Keep: `PaneId`, `DomainId`, `ClientId`, `MuxId`, `IdAllocator`

---

## 03.5 Flatten PaneRegistry

**File(s):** `oriterm_mux/src/registry/mod.rs`

- [ ] Remove `SessionRegistry` struct and its `impl` block from `registry/mod.rs`
- [ ] Remove `PaneEntry.tab: TabId` field — panes are no longer scoped to tabs
      (current fields: `pane: PaneId`, `tab: TabId`, `domain: DomainId`)
- [ ] `PaneEntry` becomes: `{ pane: PaneId, domain: DomainId }`
- [ ] Remove `panes_in_tab()` method from `PaneRegistry` (no tabs)
- [ ] Update `lib.rs` re-export: change
      `pub use registry::{PaneEntry, PaneRegistry, SessionRegistry};`
      to `pub use registry::{PaneEntry, PaneRegistry};`
- [ ] Remove `use crate::session::{MuxTab, MuxWindow};` import from `registry/mod.rs`
- [ ] Remove `use crate::id::{TabId, WindowId};` from `registry/mod.rs`
- [ ] Update `registry/tests.rs`:
  - Update PaneEntry construction: remove `tab: TabId::from_raw(...)` field
  - Remove `panes_in_tab()` test entirely
  - Delete all `SessionRegistry` tests (lines 70+)
  - Remove `TabId`, `WindowId` imports
  - Keep: PaneEntry register/get/remove tests

---

## 03.6 Purge UI Comments

**File(s):** All files in `oriterm_mux/src/`

- [ ] Remove all references to: GUI, winit, tab bar, frontend, render,
      dirty, window (in the UI sense), "re-sync tab bar", "after a
      winit wakeup", "mux-to-GUI"
- [ ] Rewrite `lib.rs` module doc:
  ```rust
  //! Pane server for oriterm.
  //!
  //! This crate manages terminal panes: spawning shell processes,
  //! reading PTY output, routing I/O, and tracking pane metadata.
  //! It has no knowledge of how panes are presented — that is the
  //! client's responsibility.
  ```
- [ ] Rewrite `mux_event/mod.rs` module doc to describe pane events,
      not "GUI notifications"
- [ ] Rewrite `MuxEventProxy` doc to remove "winit wakeup"
- [ ] Rewrite `MuxNotification` doc to describe pane state changes,
      not "notifications to the GUI"
- [ ] Scan every file: `grep -rn "GUI\|winit\|tab.bar\|frontend\|re-sync" oriterm_mux/src/`
      should return zero results. Note: `render` and `dirty` are legitimate
      in pane data contexts (RenderableContent, grid dirty flags).

---

## 03.7 Completion Checklist

- [ ] `grep -rn "TabId\|WindowId\|SessionId\|MuxTab\|MuxWindow\|SessionRegistry" oriterm_mux/src/`
      returns zero results
- [ ] `grep -rn "GUI\|winit\|tab.bar\|frontend" oriterm_mux/src/`
      returns zero results (excluding test files if justified)
- [ ] `InProcessMux` has only pane methods: `spawn_pane`, `close_pane`,
      `is_last_pane`, `get_pane_entry`, `poll_events`, `drain_notifications`,
      `discard_notifications`, `pane_registry`, `event_tx`, `default_domain`
- [ ] `MuxNotification` has only pane variants
- [ ] `PaneEntry` has no `tab` field
- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes

**Exit Criteria:** `oriterm_mux` is a flat pane server. Zero references to
tabs, windows, sessions, GUI, or any presentation concept. All builds and
tests green.
