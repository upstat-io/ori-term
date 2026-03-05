---
section: "04"
title: "Relocate Layout Modules"
status: not-started
goal: "SplitTree, FloatingLayer, Rect, layout compute, and nav all live in oriterm, not oriterm_mux"
depends_on: ["01"]
late_depends_on: ["03", "05"]
sections:
  - id: "04.1"
    title: "Copy SplitTree to oriterm"
    status: not-started
  - id: "04.2"
    title: "Copy FloatingLayer to oriterm"
    status: not-started
  - id: "04.3"
    title: "Copy Rect and Layout Compute to oriterm"
    status: not-started
  - id: "04.4"
    title: "Copy Nav to oriterm"
    status: not-started
  - id: "04.5"
    title: "Delete Mux Layout Module (LATE — after sections 03+05)"
    status: not-started
  - id: "04.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Relocate Layout Modules

**Status:** Not Started
**Goal:** All layout and navigation code lives in `oriterm` (the GUI binary).
`oriterm_mux` has no `layout/` or `nav/` modules.

**Context:** The mux currently owns `SplitTree`, `FloatingLayer`, `Rect`,
layout computation, and directional navigation. These are all presentation
concepts — they describe how panes are spatially arranged for rendering.
A non-GUI client (SSH attach) would not use any of this.

**Depends on:** Section 01 (GUI session types exist as the landing zone).

**IMPORTANT: Two-phase execution.** Steps 04.1-04.4 COPY layout code into
`oriterm/src/session/`. These are additive and can run in parallel with
section 02. Step 04.5 DELETES `layout/` and `nav/` from `oriterm_mux` and
depends on sections 03 AND 05 completing first (all internal mux consumers
must be stripped before the modules can be deleted).

**Co-implementation with Section 02:** The GUI session types created in
Section 01 need these layout modules. Section 02 (migrating oriterm)
and Section 04.1-04.4 (copying layout) should land together so the GUI's
`Tab` struct can own a `SplitTree`.

---

## 04.1 Copy SplitTree to oriterm

**File(s):**
- Source: `oriterm_mux/src/layout/split_tree/mod.rs` (177 lines),
  `oriterm_mux/src/layout/split_tree/mutations.rs` (381 lines),
  `oriterm_mux/src/layout/split_tree/tests.rs` (769 lines)
- Destination: `oriterm/src/session/split_tree/`

- [ ] Copy `split_tree/` directory to `oriterm/src/session/split_tree/`
      (do NOT delete from mux yet — that happens in step 04.5)
- [ ] Update imports: `crate::id::PaneId` becomes
      `oriterm_mux::PaneId` (SplitTree only needs PaneId)
- [ ] Update `SplitDirection` imports if it moved
- [ ] Ensure sibling `tests.rs` pattern is maintained (test-organization.md):
      `mod.rs` ends with `#[cfg(test)] mod tests;`, `tests.rs` has no wrapper module
- [ ] Re-export from `oriterm/src/session/mod.rs`:
      `pub use split_tree::{SplitDirection, SplitTree};`
- [ ] Verify tests pass in new location

---

## 04.2 Copy FloatingLayer to oriterm

**File(s):**
- Source: `oriterm_mux/src/layout/floating/mod.rs` (305 lines),
  `oriterm_mux/src/layout/floating/tests.rs` (430 lines)
- Destination: `oriterm/src/session/floating/`

- [ ] Copy `floating/` to `oriterm/src/session/floating/`
      (do NOT delete from mux yet — that happens in step 04.5)
- [ ] Update imports: `PaneId` from `oriterm_mux`, `Rect` from local
- [ ] Keep pixel-space operations (hit_test, snap_to_edge, centered) —
      these are GUI-owned presentation logic, appropriate in this location
- [ ] Ensure sibling `tests.rs` pattern is maintained (test-organization.md)
- [ ] Re-export from session: `pub use floating::{FloatingLayer, FloatingPane};`
- [ ] Verify tests pass

---

## 04.3 Copy Rect and Layout Compute to oriterm

**File(s):**
- Source: `oriterm_mux/src/layout/rect.rs` (29 lines),
  `oriterm_mux/src/layout/compute/mod.rs` (349 lines),
  `oriterm_mux/src/layout/compute/tests.rs` (968 lines)
- Destination: `oriterm/src/session/compute/` (or `oriterm/src/session/layout/`)

- [ ] Copy `rect.rs` to `oriterm/src/session/rect.rs`
      (do NOT delete from mux yet — that happens in step 04.5)
- [ ] Copy `compute/` to `oriterm/src/session/compute/`
      (do NOT delete from mux yet — that happens in step 04.5)
- [ ] Update imports: `PaneId` from `oriterm_mux`, layout types from
      local session module
- [ ] `PaneLayout`, `DividerLayout`, `LayoutParams` — all GUI types now
- [ ] Ensure sibling `tests.rs` pattern is maintained (test-organization.md)
- [ ] Re-export key types from session module

---

## 04.4 Copy Nav to oriterm

**File(s):**
- Source: `oriterm_mux/src/nav/mod.rs` (235 lines),
  `oriterm_mux/src/nav/tests.rs` (727 lines)
- Destination: `oriterm/src/session/nav/`

- [ ] Copy `nav/` to `oriterm/src/session/nav/`
      (do NOT delete from mux yet — that happens in step 04.5)
- [ ] Update imports: `PaneLayout` from local session, `PaneId` from
      `oriterm_mux`, `Direction` stays with nav
- [ ] Ensure sibling `tests.rs` pattern is maintained (test-organization.md)
- [ ] Re-export: `pub use nav::Direction;`
- [ ] Verify tests pass

---

## 04.5 Delete Mux Layout Module

**WARNING: This step depends on sections 03 AND 05 being complete.** It runs
in Phase 3b of the implementation sequence, not during Phase 1 with 04.1-04.4.

**File(s):** `oriterm_mux/src/layout/` (entire directory)

- [ ] **Pre-condition check:** Verify these internal consumers are already removed:
  - `session/mod.rs` — deleted in 03.3
  - `in_process/tab_ops.rs` — deleted in 03.1
  - `in_process/floating_ops.rs` — deleted in 03.1
  - `in_process/mod.rs` — stripped in 03.1 (no more SplitTree/FloatingLayer refs)
  - `mux_event/mod.rs` — stripped in 03.2 (no more `TabLayoutChanged(TabId)`)
  - `protocol/messages.rs` — stripped in 05.1 (no more `NotifyTabLayoutChanged`)
  - `protocol/snapshot.rs` — `MuxTabInfo` deleted in 05.1 (uses SplitTree, FloatingLayer)
  - `server/dispatch/mod.rs` — stripped in 05.2 (no more SplitPane dispatch)
  - `server/notify/mod.rs` — stripped in 05.3 (no more `tab_layout_changed_pdu()`)
  - `backend/mod.rs` (MuxBackend trait) — stripped in 05.4
  - `backend/embedded/mod.rs` — stripped in 05.5
  - `backend/client/rpc_methods.rs` — stripped in 05.5
  - `backend/client/transport.rs` — stripped in 05.5 (TabLayoutUpdate uses SplitTree, FloatingLayer)
- [ ] Delete `oriterm_mux/src/layout/` entirely
- [ ] Delete `oriterm_mux/src/nav/` entirely
- [ ] Remove `pub mod layout;` and `pub mod nav;` from `lib.rs`
- [ ] Remove all layout/nav re-exports from `lib.rs`:
      `SplitTree`, `SplitDirection`, `Direction`, `FloatingLayer`, `FloatingPane`,
      `Rect`, `PaneLayout`, `DividerLayout`, `LayoutParams`, `LayoutDescriptor`,
      `compute_all`, `snap_to_edge`, `navigate`
- [ ] Verify: `grep -rn "layout::\|nav::" oriterm_mux/src/` returns zero results

---

## 04.6 Completion Checklist

### Phase 1 gate (after 04.1-04.4):
- [ ] `oriterm/src/session/` contains: `split_tree/`, `floating/`,
      `rect.rs` (or `layout/`), `nav/`
- [ ] All layout tests pass in their new location
- [ ] Mux layout modules still exist (not deleted yet)
- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes

### Phase 3b gate (after 04.5, which requires 03+05 complete):
- [ ] `oriterm_mux/src/layout/` does not exist
- [ ] `oriterm_mux/src/nav/` does not exist
- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes

**Exit Criteria (Phase 1):** Layout and navigation code compiles and
passes tests in `oriterm/src/session/`. Mux copies still exist.

**Exit Criteria (Phase 3b):** Mux has no layout or nav modules. All
builds and tests green.
