---
section: "02"
title: "oriterm_mux — Server/Protocol/PTY Boundaries"
status: complete
goal: "Fix drifted logic, close IPC gaps, reduce allocations, and clean up dead code in oriterm_mux"
depends_on: []
sections:
  - id: "02.1"
    title: "DRIFTs — Fix Duplicated and Inconsistent Logic"
    status: complete
  - id: "02.2"
    title: "GAPs — Close Missing Functionality"
    status: complete
  - id: "02.3"
    title: "WASTEs — Reduce Per-Frame and Per-Call Allocations"
    status: complete
  - id: "02.4"
    title: "EXPOSUREs — Remove Dead Code and Fix Comments"
    status: complete
  - id: "02.5"
    title: "Completion Checklist"
    status: complete
---

# Section 02: oriterm_mux — Server/Protocol/PTY Boundaries

**Status:** Complete
**Goal:** DomainId allocation consistent. IPC forwarding complete for CommandComplete, ClipboardStore, ClipboardLoad. O(n*m) subscription sync replaced with O(n+m). Dead `WireColor` removed. All scratch buffers reuse allocations.

**Context:** The mux layer was built incrementally across the last 40 commits as the daemon architecture took shape. Several IPC event types were stubbed but never wired through. The push notification path allocates per-call where scratch buffers should be reused. The client backend has a DomainId inconsistency between `default_domain()` (returns 0) and the allocator (starts at 1).

---

## 02.1 DRIFTs — Fix Duplicated and Inconsistent Logic

**File(s):** `oriterm_mux/src/shell_integration/inject.rs`, `oriterm_mux/src/backend/client/rpc_methods.rs`, `oriterm_mux/src/backend/embedded/mod.rs`

- [x] **Finding 1**: `inject.rs:27` — Removed duplicate `set_common_env` call from `setup_injection`. The canonical call is in `spawn.rs:build_command()`. Updated doc comment on `setup_injection` to clarify.

- [x] **Finding 3**: `rpc_methods.rs:348` — Added `DomainId::LOCAL` constant (value 0). `InProcessMux::new()` now uses `DomainId::LOCAL` directly instead of allocating via the allocator. Allocator starts at 1 for dynamic domains (WSL/SSH). Client backend uses `DomainId::LOCAL`. Updated allocator doc to clarify "0 is reserved for well-known constants."

- [x] **Finding 4**: `embedded/mod.rs:116-118` — Documented the two-phase removal pattern on `close_pane`: Phase 1 unregisters from pane registry and pushes PaneClosed notification; Phase 2 (`cleanup_closed_pane`) drops the Pane on a background thread.

- [x] **Finding 5**: `rpc_methods.rs:351-353` — Documented the `is_connected` trait/inherent indirection: trait impl delegates to inherent `MuxClient::is_connected` which checks transport liveness, overriding the trait's default `true`.

---

## 02.2 GAPs — Close Missing Functionality

**File(s):** `oriterm_mux/src/server/notify/mod.rs`, `oriterm_mux/src/in_process/event_pump.rs`, `oriterm_mux/src/backend/mod.rs`, `oriterm_mux/src/backend/client/transport/mod.rs`, `oriterm_mux/src/protocol/messages.rs`, `oriterm_mux/src/protocol/msg_type.rs`, `oriterm_mux/src/backend/client/notification.rs`

- [x] **Finding 15**: Added `NotifyCommandComplete` PDU (0x0305) with `pane_id` and `duration_ms`. Server-side `notification_to_pdu` now forwards `MuxNotification::CommandComplete` over IPC. Client-side `pdu_to_notification` converts back to `MuxNotification::CommandComplete` with `Duration::from_millis()`.

- [x] **Finding 16**: Added `NotifyClipboardStore` (0x0306) and `NotifyClipboardLoad` (0x0308) PDUs. Server-side forwards both over IPC. Client-side reconstructs `MuxNotification::ClipboardStore` from wire format and `MuxNotification::ClipboardLoad` with a reconstructed OSC 52 response formatter (BEL-terminated, standard clipboard letter).

- [x] **Finding 11**: `event_pump.rs` — Added `pane.set_bell()` call in the `PaneBell` handler before pushing the notification, matching the pattern used for `CommandComplete`.

- [x] **Finding 10**: Changed `pane_cwd` return type from `Option<String>` to `Option<&str>`, using `s.cwd.as_deref()` instead of `.clone()`.

- [x] **Finding 14**: Added return value checks for both `fcntl(F_SETFL, O_NONBLOCK)` and `fcntl(F_SETFD, FD_CLOEXEC)` calls. Logs warning on failure (non-fatal).

---

## 02.3 WASTEs — Reduce Per-Frame and Per-Call Allocations

**File(s):** `oriterm_mux/src/server/push/mod.rs`, `oriterm_mux/src/server/clients.rs`

- [x] **Finding 6**: Added `scratch_panes: &'a mut Vec<PaneId>` to `PushContext`. `trailing_edge_flush` now uses this scratch buffer instead of allocating a new `Vec<PaneId>` per call.

- [x] **Finding 7**: `disconnect_client` now uses `self.scratch_panes` instead of allocating a new Vec for subscribed pane IDs.

- [x] **Finding 9**: `sync_subscriptions` now builds a `HashSet<PaneId>` from the conn's subscriptions for O(1) lookup during the retain loop, replacing O(n*m) `Vec::contains()`.

- [x] **Finding 2 [PLANNED]**: `snapshot.rs:200-203` — Deferred to roadmap Section 23. **Not implemented here.**

- [x] **Finding 8 [PLANNED]**: `push/mod.rs:59` — Deferred to roadmap Section 23. **Not implemented here.**

- [x] **Finding 12 [PLANNED]**: `frame_io.rs` — Deferred to roadmap Section 23. **Not implemented here.**

- [x] **Finding 13 [PLANNED]**: `codec.rs:79` — Deferred to roadmap Section 23. **Not implemented here.**

---

## 02.4 EXPOSUREs — Remove Dead Code and Fix Comments

**File(s):** `oriterm_mux/src/protocol/snapshot.rs`, `oriterm_mux/src/server/snapshot.rs`

- [x] **Finding 17**: `WireColor` already has `#[allow(dead_code, reason = "reserved for future incremental wire format")]` and a doc comment explaining the planned use. Justified — no action needed.

- [x] **Finding 18**: Changed `// SAFETY:` to `// Invariant:` on the non-unsafe `entry().or_default()` guarantee in `server/snapshot.rs`.

---

## 02.5 Completion Checklist

- [x] `set_common_env` called exactly once per spawn (not in `setup_injection`)
- [x] `default_domain()` returns a DomainId consistent with the allocator
- [x] `close_pane` lifecycle documented with two-phase removal comment
- [x] `CommandComplete` forwarded over IPC to daemon clients
- [x] `ClipboardStore`/`ClipboardLoad` forwarded over IPC (OSC 52 works in daemon mode)
- [x] `PaneBell` sets bell flag on pane
- [x] `pane_cwd` returns `Option<&str>` (no unnecessary clone)
- [x] `fcntl` return values checked on macOS
- [x] `trailing_edge_flush` uses scratch buffer (no per-call Vec)
- [x] `sync_subscriptions` is O(n+m) via HashSet
- [x] `WireColor` justified (reserved for future incremental wire format)
- [x] `SAFETY` comment on non-unsafe code reworded to `Invariant`
- [x] `./test-all.sh` passes
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` succeeds

**Exit Criteria:** IPC forwarding complete for all terminal events. Zero per-call Vec allocations in push/disconnect paths. DomainId allocation consistent. `./test-all.sh && ./clippy-all.sh && ./build-all.sh` all green.
