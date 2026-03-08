---
section: "03"
title: "WASTE Fixes"
status: complete
goal: "Eliminate unnecessary allocations in server hot paths and remove duplicated trait/inherent method bodies"
depends_on: []
sections:
  - id: "03.1"
    title: "Downgrade Sync Output Log Level"
    status: complete
  - id: "03.2"
    title: "Subscribe Handler Snapshot Cache Bypass"
    status: complete
  - id: "03.3"
    title: "Per-Cycle Vec Allocation in Server"
    status: complete
  - id: "03.4"
    title: "Per-Pane Vec in Push Trailing Edge"
    status: complete
  - id: "03.5"
    title: "Duplicate from_raw/raw Methods in ID Types"
    status: complete
  - id: "03.6"
    title: "Completion Checklist"
    status: complete
---

# Section 03: WASTE Fixes

**Status:** Complete
**Goal:** Server hot paths reuse existing scratch buffers instead of allocating. Log levels match actual severity. ID type methods delegate instead of duplicating.

**Context:** WASTE findings are about unnecessary work — allocations that could be avoided, log levels that cause noise, and code duplication that increases maintenance burden. These don't affect correctness but degrade performance and readability.

---

## 03.1 Downgrade Sync Output Log Level

**File(s):** `oriterm_mux/src/pty/event_loop/mod.rs`

**Finding 11:** `log::warn!` fires on every parse cycle during synchronized output (Mode 2026). This allocates a format string each call and floods logs during normal operation. Synchronized output buffering is expected behavior, not a warning condition.

- [x] Change `log::warn!` to `log::trace!` for the synchronized output buffering message
- [x] Verify the message is only about sync buffering, not an actual error condition

---

## 03.2 Subscribe Handler Snapshot Cache Bypass

**File(s):** `oriterm_mux/src/server/dispatch/mod.rs`

**Finding 23:** The Subscribe handler calls `snapshot::build_snapshot(pane)` directly, bypassing the `SnapshotCache` that the adjacent `GetPaneSnapshot` handler uses. This allocates fresh buffers unnecessarily when the cache already has (or could have) the data.

- [x] Replace `snapshot::build_snapshot(pane)` with `ctx.snapshot_cache.build_and_take(pane_id, pane)` in the Subscribe handler
- [x] Verify the `SnapshotCache` API supports this usage (check method signature and return type)
- [x] If the Subscribe handler needs the snapshot in a different format, adapt as needed but still use the cache

---

## 03.3 Per-Cycle Vec Allocation in Server

**File(s):** `oriterm_mux/src/server/mod.rs`

**Finding 24:** The server allocates a new `Vec` each cycle to collect closed pane IDs. The `Server` struct already has a `scratch_panes` buffer available for exactly this purpose.

- [x] Replace `let closed: Vec<_> = ...` (or similar) with clearing and reusing `self.scratch_panes`
- [x] Verify `scratch_panes` has the correct type (`Vec<PaneId>` or compatible)
- [x] If `scratch_panes` doesn't exist yet, check the finding context — it may be named differently

---

## 03.4 Per-Pane Vec in Push Trailing Edge

**File(s):** `oriterm_mux/src/server/push/mod.rs`

**Finding 25:** The trailing edge flush allocates a new `Vec::new()` per pane. `PushContext` already has a `scratch` buffer available for this purpose.

- [x] Replace `Vec::new()` with clearing and reusing `ctx.scratch`
- [x] Verify `ctx.scratch` has the correct type for this usage
- [x] If types differ, consider adding a second scratch buffer to `PushContext` rather than allocating per-pane

---

## 03.5 Duplicate from_raw/raw Methods in ID Types

**File(s):** `oriterm_mux/src/id/mod.rs`

**Finding 34:** `PaneId`, `DomainId`, and `ClientId` each have `from_raw`/`raw` methods implemented twice — once as `MuxId` trait impls and once as inherent impls — with identical bodies. This is 76 lines of duplication.

- [x] Have trait impls delegate to inherent methods (or vice versa)
- [x] Alternatively, if `MuxId` trait is made `pub(crate)` (see Section 04, finding 36), consider whether the trait is still needed at all — if only used for generic constraints, the inherent methods may suffice
- [x] Verify no external consumers depend on the `MuxId` trait

---

## 03.6 Completion Checklist

- [x] Sync output log is `trace!` level, not `warn!`
- [x] Subscribe handler uses `SnapshotCache`, not direct `build_snapshot`
- [x] Server event loop reuses `scratch_panes` buffer instead of allocating per-cycle
- [x] Push trailing edge reuses `ctx.scratch` instead of allocating per-pane
- [x] ID type `from_raw`/`raw` methods have single source of truth (no body duplication)
- [x] `cargo test -p oriterm_mux` passes
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green

**Exit Criteria:** Zero per-cycle/per-pane allocations in the server event loop where scratch buffers exist. Log levels match actual severity. No duplicated method bodies in ID types.
