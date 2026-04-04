---
section: 34
title: IPC Protocol + Daemon Mode
status: in-progress
reviewed: false
tier: 7A
last_verified: "2026-03-29"
goal: Wire protocol for mux server/client communication, MuxServer daemon, OutputCoalescer for push-based rendering, MuxClient for GUI, auto-start daemon
sections:
  - id: "34.1"
    title: Wire Protocol
    status: in-progress
  - id: "34.2"
    title: MuxServer Daemon
    status: in-progress
  - id: "34.3"
    title: OutputCoalescer
    status: in-progress
  - id: "34.4"
    title: MuxClient + Auto-Start
    status: in-progress
  - id: "34.5"
    title: Section Completion
    status: not-started
---

# Section 34: IPC Protocol Hardening + Advanced Coalescing

**Status:** In Progress (~80% complete via Section 44)
**Goal:** Harden the IPC protocol established in Section 44 with compression, version negotiation, advanced output coalescing, and forward compatibility. This is polish on top of the working daemon built in Section 44.

**NOTE:** The core daemon, IPC protocol, MuxClient, MuxBackend trait, and cross-process tab migration are built in **Section 44 (Multi-Process Window Architecture)**. This section adds production hardening: zstd compression for large payloads, version negotiation for forward compatibility, tiered output coalescing for optimal rendering latency, and reconnection resilience.

**Crate:** `oriterm_mux` (protocol, server, client), `oriterm` (client integration)
**Dependencies:** Section 44 (multi-process window architecture working)
**Prerequisite:** Section 44 complete.

**Inspired by:**
- WezTerm: mux server with SSH domains, codec protocol, poll-based rendering (140ms — we beat this)
- tmux: server/client architecture, session persistence across terminal restarts
- Zellij: server mode with WASM plugin isolation

**Key improvement over WezTerm:** Push-based rendering with 1ms coalesce (vs WezTerm's 140ms poll interval). The server pushes dirty pane notifications to clients; clients don't poll. This gives near-local responsiveness even over the IPC boundary.

---

## 34.1 Wire Protocol

Binary protocol for communication between the mux daemon and GUI clients. Designed for low latency, low overhead, and forward compatibility.

**File:** `oriterm_mux/src/protocol.rs`, `oriterm_mux/src/protocol/codec.rs`

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] Frame format (10-byte header): type(u16) + seq(u32) + payload_len(u32). Actual implementation uses a simpler header than the 15-byte plan. (verified 2026-03-29)
  - [x] Type: message type ID (u16) via `MsgType` module (verified 2026-03-29)
  - [x] Payload length: u32 (verified 2026-03-29)
  - [x] Sequence: u32 request ID for request/response correlation (verified 2026-03-29)
- [x] Serialization: `bincode` for payload encoding (verified 2026-03-29)
- [x] Message types: 40+ `MuxPdu` variants covering spawn, close, input, resize, subscribe, search, theme, cursor, images, clipboard (verified 2026-03-29)
- [x] Transport: Unix domain socket (Linux/macOS), named pipe (Windows) via `oriterm_ipc` crate (verified 2026-03-29)
- [x] `ProtocolCodec` with streaming decode, forward-compat (skips unknown msg_type) (verified 2026-03-29)
- [x] Frame encode/decode roundtrip tests: 1060+ lines in `oriterm_mux/src/protocol/tests.rs` (verified 2026-03-29)
- [x] Wire snapshot types (`WireCell`, `WireCursor`, etc.) in `oriterm_mux/src/protocol/snapshot.rs` (verified 2026-03-29) <!-- unblocks:39.5 --><!-- unblocks:39.6 -->

**Remaining hardening (genuinely not started):**
- [ ] Magic bytes: `0x4F54` ("OT") in header -- not implemented (no magic field in current 10-byte header)
- [ ] Version field in header -- not implemented
- [ ] Flags field: `COMPRESSED = 0x01`, `RESPONSE = 0x02` -- not implemented (no flags in current header)
- [ ] Compression: `zstd` (level 1) for payloads > 4KB -- not implemented, no zstd dependency
- [ ] Version negotiation:
  - [ ] Client sends `Hello { version: u8, features: u64 }` on connect -- current Hello only sends PID
  - [ ] Server responds `HelloAck { version: u8, features: u64 }` -- not implemented
  - [ ] Incompatible versions: server returns `VersionMismatch` and closes -- not implemented

**Remaining tests:**
- [ ] Version negotiation: compatible versions -> success
- [ ] Version mismatch: server rejects incompatible client
- [ ] Compression: payloads > 4KB compressed with zstd, < 4KB uncompressed
- [ ] Max payload: 16MB limit enforced

---

## 34.2 MuxServer Daemon

The `oriterm-mux` daemon process. Keeps all terminal sessions alive. Accepts connections from GUI clients. Routes pane output to subscribed clients.

**File:** `oriterm_mux/src/server.rs`, `oriterm_mux/src/server/connection.rs`

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] `MuxServer` struct with `InProcessMux`, mio event loop, client connections, subscriptions -- `oriterm_mux/src/server/mod.rs` (364 lines) + 7 submodules (verified 2026-03-29)
- [x] `ClientConnection` with write backpressure -- `oriterm_mux/src/server/connection.rs`, `clients.rs` (verified 2026-03-29)
- [x] Server event loop: single-threaded with `mio`, accepts connections, reads messages, dispatches to `InProcessMux`, pushes notifications (verified 2026-03-29)
- [x] Connection lifecycle: client disconnect unsubscribes panes, panes stay alive (verified 2026-03-29)
- [x] Daemon lifecycle: `oriterm-mux --daemon`/`--stop`, PID file management, signal handling (Unix+Windows) -- `oriterm_mux/src/bin/oriterm_mux.rs` (333 lines) (verified 2026-03-29)
- [x] PID file management -- `oriterm_mux/src/server/pid_file.rs` (verified 2026-03-29)
- [x] Tests: 433 tests pass (390 unit + 20 contract + 23 e2e) (verified 2026-03-29)

**Remaining hardening (genuinely not started):**
- [ ] Shadow grid (for reconnection):
  - [ ] Server maintains last-known `RenderableContent` for each pane
  - [ ] On client `Subscribe(pane_id)`: send full `PaneContent` first (cold start), then push incremental updates
  - [ ] Enables instant display on reconnect — no waiting for shell to redraw

---

## 34.3 OutputCoalescer

The push-based rendering engine. Coalesces rapid pane output (e.g., `cat large_file.txt`) into batched notifications with configurable latency targets. This is what makes mux-mode rendering fast.

**File:** `oriterm_mux/src/server/coalescer.rs`

**Reference:** WezTerm's 140ms poll (we beat this with push + coalesce)

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] Push-based snapshot delivery at ~60fps: 16ms throttle, trailing-edge flush, backpressure deferral -- `oriterm_mux/src/server/push/mod.rs` (248 lines) (verified 2026-03-29)
- [x] Backpressure: slow clients get deferred, latest state always delivered (verified 2026-03-29)

**Remaining hardening (genuinely not started):**
- [ ] Tiered coalescing (different intervals based on pane visibility):
  - [ ] **Focused pane**: 1ms coalesce — near-instant rendering
  - [ ] **Visible unfocused pane**: 16ms coalesce (~60 FPS) — smooth but efficient
  - [ ] **Hidden pane** (scrolled offscreen, in background tab): 100ms coalesce — low overhead
  - [ ] Focus change: pane promoted from hidden -> focused gets tighter coalesce
- [ ] `PaneOutput` notification content enrichment:
  - [ ] `dirty_rows: Option<Vec<u16>>` — which rows changed (for incremental rendering)
  - [ ] `cursor_changed: bool` — cursor position or shape changed
  - [ ] `title_changed: Option<String>` — new title if changed during this batch

**Remaining tests:**
- [ ] Tiered coalescing: focused pane at 1ms, visible at 16ms, hidden at 100ms
- [ ] Focus change: pane promoted from hidden -> focused gets tighter coalesce

---

## 34.4 MuxClient + Auto-Start

The GUI's connection to the mux daemon. `MuxClient` implements the same API as `InProcessMux` so the App doesn't care whether it's local or daemon mode.

**File:** `oriterm_mux/src/client.rs`, `oriterm/src/app/mod.rs`

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] `MuxClient` with RPC + push notifications, background reader thread, dirty tracking, snapshot cache -- `oriterm_mux/src/backend/client/mod.rs` + submodules (verified 2026-03-29)
- [x] `MuxBackend` trait (47 methods), implemented by both `EmbeddedMux` and `MuxClient` -- `oriterm_mux/src/backend/mod.rs` (287 lines) (verified 2026-03-29)
- [x] Auto-start daemon: `ensure_daemon()`, exponential backoff, stale PID cleanup -- `oriterm_mux/src/discovery/mod.rs` (140 lines) (verified 2026-03-29)
- [x] Tests: compile-time trait check, round-trip spawn, subscribe, auto-start all passing (verified 2026-03-29)

**Remaining hardening (genuinely not started):**
- [ ] Reconnection:
  - [ ] If daemon connection drops: attempt reconnect every 500ms (3 attempts)
  - [ ] On reconnect: re-subscribe to all previously subscribed panes
  - [ ] Shadow grid enables instant display — no blank screen during reconnect

**Remaining tests:**
- [ ] Reconnection: simulated disconnect -> reconnect -> re-subscribe

---

## 34.5 Section Completion

**Already complete (via Section 44, verified 2026-03-29):**
- [x] Wire protocol: 10-byte header, bincode payload, streaming codec (verified 2026-03-29)
- [x] MuxServer: accepts connections, routes messages, pushes output (verified 2026-03-29)
- [x] OutputCoalescer: 16ms flat push with backpressure (verified 2026-03-29)
- [x] MuxClient: same API as InProcessMux, transparent backend switching (verified 2026-03-29)
- [x] Auto-start daemon: seamless to user, fallback to in-process (verified 2026-03-29)
- [x] `cargo build --target x86_64-pc-windows-gnu` — compiles (verified 2026-03-29)
- [x] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] `cargo test` — 433 tests pass (verified 2026-03-29)

**Remaining for full completion:**
- [ ] All 34.1–34.4 hardening items complete
- [ ] Wire protocol hardening: magic bytes, version field, zstd compression
- [ ] Version negotiation: Hello/HelloAck with feature flags
- [ ] Tiered coalescing: 1ms/16ms/100ms by pane visibility
- [ ] Reconnection: client reconnect + re-subscribe
- [ ] **Latency test**: keystroke -> screen update < 5ms through daemon
- [ ] **Throughput test**: `cat large_file.txt` renders smoothly, no dropped frames
- [ ] **Reconnection test**: kill GUI, relaunch -> sessions restored instantly
- [ ] **Multi-client test**: two GUI windows connected to same daemon

**Exit Criteria:** Full server/client architecture. The daemon keeps sessions alive across GUI restarts. Push-based rendering with 1ms coalesce beats WezTerm's 140ms poll. Transparent backend switching lets the App work identically in local and daemon modes. Auto-start and graceful fallback make the daemon invisible to users who don't need it.
