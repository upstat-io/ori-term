---
section: "04"
title: "EXPOSURE Fixes"
status: complete
goal: "Narrow visibility of all internal-only pub items to pub(crate) or pub(super)"
depends_on: []
sections:
  - id: "04.1"
    title: "MuxEvent/MuxEventProxy Visibility"
    status: complete
  - id: "04.2"
    title: "PTY Spawn Function Visibility"
    status: complete
  - id: "04.3"
    title: "Dead Accessor Removal"
    status: complete
  - id: "04.4"
    title: "Server Push Function Visibility"
    status: complete
  - id: "04.5"
    title: "Server/Protocol Type Visibility"
    status: complete
  - id: "04.6"
    title: "ClientConnection Method Visibility"
    status: complete
  - id: "04.7"
    title: "ID Type Visibility"
    status: complete
  - id: "04.8"
    title: "Registry Module Visibility"
    status: complete
  - id: "04.9"
    title: "Completion Checklist"
    status: complete
---

# Section 04: EXPOSURE Fixes

**Status:** Complete
**Goal:** Every `pub` item in `oriterm_mux` is either genuinely part of the crate's public API or narrowed to `pub(crate)`/`pub(super)`. No dead public accessors remain.

**Context:** EXPOSURE findings represent API surface that is wider than intended. `pub` items used only within the crate create fragile coupling — downstream code could start depending on internals that were never meant to be stable. Narrowing visibility is a safe, mechanical change that makes the actual API boundary explicit.

**Note:** Applied Section 01 (DRIFT) changes first since they modify the same types (e.g., `MuxEvent`), then narrowed visibility here.

---

## 04.1 MuxEvent/MuxEventProxy Visibility

**File(s):** `oriterm_mux/src/mux_event/mod.rs`

**Finding 5:** `MuxEvent` and `MuxEventProxy` are `pub` but initially appeared to be only used within `oriterm_mux`.

**Deviation:** Both `MuxEvent` and `MuxEventProxy` were kept `pub` because:
- `MuxEvent` is referenced in `MuxBackend::event_tx()` return type, which is a `pub` trait used cross-crate by `oriterm`
- `MuxEventProxy` appears in `Pane::terminal()` return type (`Arc<FairMutex<Term<MuxEventProxy>>>`)

- [x] Change `pub enum MuxEvent` to `pub(crate) enum MuxEvent` — **Reverted: kept `pub`** (cross-crate usage via `MuxBackend` trait)
- [x] Change `pub struct MuxEventProxy` to `pub(crate) struct MuxEventProxy` — **Reverted: kept `pub`** (appears in public `Pane::terminal()` return type)
- [x] Verify no external crate (`oriterm`, `oriterm_core`, etc.) imports these types — **Found cross-crate usage**
- [x] If external usage exists, keep `pub` and document the API contract — **Done**

---

## 04.2 PTY Spawn Function Visibility

**File(s):** `oriterm_mux/src/pty/spawn.rs`

**Finding 12:** `build_command` and `default_shell` are `pub` but only used within `spawn.rs`.

**Finding 13:** `compute_wslenv` is `pub` but only used within `spawn.rs`.

- [x] Change `pub fn build_command` to `pub(crate) fn build_command`
- [x] Change `pub fn default_shell` to `pub(crate) fn default_shell`
- [x] Change `pub fn compute_wslenv` to `pub(crate) fn compute_wslenv`
- [x] Section 01.4 (WSLENV unification) moved `compute_wslenv` re-export to `pty/mod.rs` — visibility adjusted accordingly

---

## 04.3 Dead Accessor Removal

**File(s):** `oriterm_mux/src/in_process/event_pump.rs`

**Finding 18:** `pane_registry()` accessor is `pub` with zero call sites.

- [x] Search the entire workspace for calls to `pane_registry()` to confirm it is dead
- [x] Remove the method entirely if dead — **Removed**

---

## 04.4 Server Push Function Visibility

**File(s):** `oriterm_mux/src/server/push/mod.rs`

**Finding 26:** `should_push`, `push_snapshot_to_subscribers`, and `defer_all_subscribers` are `pub fn` but only used within `push/mod.rs`.

- [x] Change all three to `fn` (private)
- [x] Verify tests in `push/tests.rs` access them via `super::` (which works for private items in the parent module)

---

## 04.5 Server/Protocol Type Visibility

**File(s):** `oriterm_mux/src/server/mod.rs`, `oriterm_mux/src/protocol/mod.rs`

**Finding 27:** `ClientConnection` is re-exported as `pub` from `server/mod.rs` but unused outside the crate.

**Finding 28:** `FrameHeader` and `MsgType` are `pub` in `protocol/mod.rs` but crate-internal.

- [x] Change `pub use connection::ClientConnection` to `pub(crate) use connection::ClientConnection` in `server/mod.rs`
- [x] Change `FrameHeader` visibility to `pub(crate)` in `protocol/mod.rs`
- [x] Change `MsgType` visibility to `pub(crate)` in `protocol/mod.rs`
- [x] Verify no external crate imports these types
- [x] Change `MuxPdu::msg_type()` to `pub(crate)` (since `MsgType` is now `pub(crate)`)

---

## 04.6 ClientConnection Method Visibility

**File(s):** `oriterm_mux/src/server/connection.rs`

**Finding 29:** All 14 methods on `ClientConnection` are `pub` but only used within `server/`.

- [x] Change all methods to `pub(super)` (visible within `server/` module tree)
- [x] Verify with `cargo build` that no external module accesses these methods
- [x] Removed dead `is_subscribed()` method (zero callers)

---

## 04.7 ID Type Visibility

**File(s):** `oriterm_mux/src/id/mod.rs`

**Finding 36:** `MuxId` trait and `IdAllocator` are `pub` but only used within `oriterm_mux`.

- [x] Change `pub trait MuxId` to `pub(crate) trait MuxId`
- [x] Change `pub struct IdAllocator` to `pub(crate) struct IdAllocator`
- [x] Verify `PaneId`, `DomainId`, `ClientId` remain `pub` (they are part of the public API)
- [x] Added `#[allow(dead_code, reason = "trait completeness")]` on `MuxId::raw()` (inherent methods used instead)

---

## 04.8 Registry Module Visibility

**File(s):** `oriterm_mux/src/lib.rs`

**Finding 21:** `registry` module is `pub` but only consumed within the crate.

- [x] Change `pub mod registry` to `pub(crate) mod registry` in `lib.rs`
- [x] Re-exported `PaneEntry` from `lib.rs` (needed in `MuxBackend` trait signatures)
- [x] Verify with `cargo build` across the workspace

---

## 04.9 Completion Checklist

- [x] `MuxEvent` and `MuxEventProxy` kept `pub` (cross-crate usage confirmed)
- [x] `build_command`, `default_shell`, `compute_wslenv` are `pub(crate)`
- [x] Dead `pane_registry()` accessor removed
- [x] `should_push`, `push_snapshot_to_subscribers`, `defer_all_subscribers` are private
- [x] `ClientConnection` re-export is `pub(crate)`
- [x] `FrameHeader` and `MsgType` are `pub(crate)`
- [x] All `ClientConnection` methods are `pub(super)`
- [x] `MuxId` trait and `IdAllocator` are `pub(crate)`
- [x] `registry` module is `pub(crate)` with necessary re-exports
- [x] `cargo test -p oriterm_mux` passes
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green

**Exit Criteria:** Every `pub` item in `oriterm_mux` is either part of the documented public API (`PaneId`, `MuxNotification`, `MuxBackend`, etc.) or narrowed to the minimum required visibility. Zero dead public accessors.
