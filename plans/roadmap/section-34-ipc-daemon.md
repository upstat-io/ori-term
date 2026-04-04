---
section: 34
title: IPC Protocol + Daemon Mode
status: in-progress
reviewed: true
tier: 7A
last_verified: "2026-04-04"
third_party_review:
  status: none
  updated: null
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
- WezTerm: mux server with SSH domains, codec protocol, poll-based rendering (140ms -- we beat this)
- tmux: server/client architecture, session persistence across terminal restarts
- Zellij: server mode with WASM plugin isolation

**Key improvement over WezTerm:** Push-based rendering with 4ms coalesce (vs WezTerm's 140ms poll interval). The server pushes dirty pane notifications to clients; clients don't poll. This gives near-local responsiveness even over the IPC boundary.

---

## 34.1 Wire Protocol

Binary protocol for communication between the mux daemon and GUI clients. Designed for low latency, low overhead, and forward compatibility.

**File:** `oriterm_mux/src/protocol/mod.rs`, `oriterm_mux/src/protocol/codec.rs`, `oriterm_mux/src/protocol/messages.rs`

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] Frame format (10-byte header): type(u16) + seq(u32) + payload_len(u32). (verified 2026-03-29)
  - [x] Type: message type ID (u16) via `MsgType` module (verified 2026-03-29)
  - [x] Payload length: u32 (verified 2026-03-29)
  - [x] Sequence: u32 request ID for request/response correlation (verified 2026-03-29)
- [x] Serialization: `bincode` for payload encoding (verified 2026-03-29)
- [x] Message types: 51 `MuxPdu` variants covering spawn, close, input, resize, subscribe, search, theme, cursor, images, clipboard (verified 2026-04-04)
- [x] Transport: Unix domain socket (Linux/macOS), named pipe (Windows) via `oriterm_ipc` crate (verified 2026-03-29)
- [x] `ProtocolCodec` with streaming decode, forward-compat (skips unknown msg_type) (verified 2026-03-29)
- [x] Frame encode/decode roundtrip tests: 1091 lines in `oriterm_mux/src/protocol/tests.rs` (verified 2026-04-04)
- [x] Wire snapshot types (`WireCell`, `WireCursor`, etc.) in `oriterm_mux/src/protocol/snapshot.rs` (verified 2026-03-29)

**Architecture notes for remaining work:**

The header change from 10 to 14 bytes is a breaking wire-format migration. All encode/decode paths must be updated atomically in a single commit:
- `FrameHeader::encode()` / `FrameHeader::decode()` in `protocol/mod.rs`
- `ProtocolCodec::encode_frame()` / `try_decode()` in `protocol/codec.rs`
- `FrameReader::try_decode()` in `server/frame_io.rs`
- `FrameWriter::queue()` in `server/frame_io.rs`
- `one_shot::request_new_tab()` in `one_shot.rs` (uses `ProtocolCodec` directly -- no explicit header changes needed if encode_frame/decode_frame are updated, but verify the handshake still works)
- `ClientTransport::connect()` in `backend/client/transport/mod.rs` (same -- uses `ProtocolCodec` directly)

Bincode backward-compat for Hello/HelloAck: bincode 1.3 uses sequential field encoding. Adding fields to the END of existing struct variants is NOT backward-compatible -- old clients sending `Hello{pid}` will produce fewer bytes than new servers expect for `Hello{pid, protocol_version, features}`. Since there is no released wire format, this is acceptable. But if a future version needs to maintain compat, new fields must go in NEW PDU variants (e.g., `HelloV2`) rather than appended to existing ones.

**Encode path duplication:** There are TWO independent encode paths that must both be updated for compression:
1. `ProtocolCodec::encode_frame()` in `protocol/codec.rs` -- used by `one_shot.rs` and `ClientTransport::connect()`
2. `FrameWriter::queue()` in `server/frame_io.rs` -- used by the server's non-blocking write path

These share identical logic (bincode serialize, validate size, construct header, write). Consider extracting a shared `encode_to_buf(buf: &mut Vec<u8>, seq: u32, pdu: &MuxPdu, flags: u8) -> io::Result<()>` into `protocol/encode.rs` before adding compression, mirroring the decode extraction. Otherwise compression logic must be implemented twice.

**Remaining hardening (genuinely not started):**
- [ ] Extract shared decode logic to eliminate algorithmic duplication (MUST happen before header change):
  - [ ] Create `oriterm_mux/src/protocol/decode.rs` with shared `try_decode_from_buf(buf: &mut Vec<u8>) -> Option<Result<DecodedFrame, DecodeError>>`
  - [ ] Add `mod decode;` to `protocol/mod.rs`
  - [ ] Refactor `ProtocolCodec::try_decode()` in `codec.rs` to call `decode::try_decode_from_buf()`
  - [ ] Refactor `FrameReader::try_decode()` in `frame_io.rs` to call `decode::try_decode_from_buf()`
  - [ ] Verify all existing tests pass (no behavioral change)
- [ ] Extract shared encode logic (recommended before compression):
  - [ ] Create `oriterm_mux/src/protocol/encode.rs` with shared `encode_to_buf(buf: &mut Vec<u8>, seq: u32, pdu: &MuxPdu, flags: u8) -> io::Result<()>`
  - [ ] Add `mod encode;` to `protocol/mod.rs`
  - [ ] Refactor `ProtocolCodec::encode_frame()` in `codec.rs` to delegate to it
  - [ ] Refactor `FrameWriter::queue()` in `frame_io.rs` to delegate to it
  - [ ] Verify all existing tests pass (no behavioral change)
- [ ] Extend frame header from 10 to 14 bytes: `magic(u16) + version(u8) + flags(u8) + type(u16) + seq(u32) + payload_len(u32)`
  - [ ] Add `FrameHeader` fields: `magic: u16`, `version: u8`, `flags: u8`
  - [ ] Magic bytes `0x4F54` ("OT") -- reject streams that don't start with magic (early detection of non-oriterm connections)
  - [ ] Version field `u8` -- protocol version, currently `1`. Increment on breaking wire changes.
  - [ ] Flags field `u8` -- `COMPRESSED = 0x01` (payload is zstd-compressed). Reserved bits ignored on decode.
  - [ ] Update `HEADER_LEN` constant from 10 to 14 in `protocol/mod.rs`
  - [ ] Update `FrameHeader::encode()`: write magic(2) + version(1) + flags(1) + type(2) + seq(4) + payload_len(4) = 14 bytes
  - [ ] Update `FrameHeader::decode()`: read 14 bytes, validate magic before parsing rest
  - [ ] Add `DecodeError::BadMagic(u16)` variant for magic validation failures
  - [ ] After decode/encode extraction, only the shared functions need updating -- verify all call sites work
  - [ ] Update all test roundtrips in `protocol/tests.rs` (header_roundtrip, header_zero_values, header_max_values, etc.)
- [ ] Compression: `zstd` (level 1) for payloads > 4KB
  - [ ] Add `zstd` dependency to `oriterm_mux/Cargo.toml`
  - [ ] In shared encode path: after bincode serialize, if `payload.len() > 4096`, compress with zstd level 1. If compressed is smaller, set `COMPRESSED` flag in header and write compressed payload. If compressed is larger, clear flag and write original.
  - [ ] In shared decode path: after reading payload bytes, if `COMPRESSED` flag set, decompress before bincode deserialization
  - [ ] Threshold: compress only when `payload_len > 4096` -- `NotifyPaneSnapshot` with an 80x24 grid is the primary target (~50-200KB bincode, compresses well due to repeated cell structures)
- [ ] Version negotiation via extended Hello/HelloAck:
  - [ ] Add `protocol_version: u8` and `features: u64` fields to `MuxPdu::Hello` (alongside existing `pid`). This is a breaking bincode change (see architecture notes).
  - [ ] Add `protocol_version: u8` and `features: u64` fields to `MuxPdu::HelloAck` (alongside existing `client_id`)
  - [ ] Add constants: `CURRENT_PROTOCOL_VERSION: u8 = 1`, `FEAT_ZSTD: u64 = 1`
  - [ ] Server `dispatch_request` Hello handler: if `Hello.protocol_version > CURRENT_PROTOCOL_VERSION`, respond `Error { message: "version mismatch" }` and disconnect. If equal or less, proceed (backward compat for older clients).
  - [ ] Server: negotiate features as intersection of client + server capabilities, store negotiated features per-connection in `ClientConnection` (add `negotiated_features: u64` field)
  - [ ] Client: if `HelloAck.protocol_version != CURRENT_PROTOCOL_VERSION`, log error and disconnect
  - [ ] Feature flag `FEAT_ZSTD = 1`: compression only used when both sides negotiated it. Encode path checks `negotiated_features & FEAT_ZSTD != 0` before compressing.
  - [ ] Update `ClientTransport::connect()` to send version/features in Hello and validate HelloAck version
  - [ ] Update `dispatch_request` Hello handler to validate version and negotiate features
  - [ ] Update `one_shot::request_new_tab()` to send version/features in Hello (add `protocol_version: CURRENT_PROTOCOL_VERSION, features: 0` -- one-shot doesn't need compression)

**Remaining tests** (all in `oriterm_mux/src/protocol/tests.rs` unless noted):

Pre-existing bug fixes (fix first):
- [x] `msg_type_roundtrip_all`: add `NotifyCommandComplete`, `NotifyClipboardStore`, `NotifyClipboardLoad` -- currently missing from the roundtrip array (verified 2026-04-04, fixed 2026-04-04)
- [x] `roundtrip_notify_command_complete`: new roundtrip test -- `MuxPdu::NotifyCommandComplete { pane_id, duration_ms: 1234 }` (added 2026-04-04)
- [x] `roundtrip_notify_clipboard_store`: new roundtrip test -- `MuxPdu::NotifyClipboardStore { pane_id, clipboard_type: 0, text: "hello" }` (added 2026-04-04)
- [x] `roundtrip_notify_clipboard_load`: new roundtrip test -- `MuxPdu::NotifyClipboardLoad { pane_id, clipboard_type: 1 }` (added 2026-04-04)

Header extension tests:
- [ ] `header_14byte_roundtrip`: encode/decode a 14-byte header with magic=0x4F54, version=1, flags=0, type=0x0101, seq=42, payload_len=1024. Assert all fields survive roundtrip.
- [ ] `header_14byte_zero_values`: all fields zero except magic (which must always be 0x4F54). Assert roundtrip.
- [ ] `header_14byte_max_values`: max u16/u8/u32 values. Assert roundtrip.
- [ ] `header_bad_magic_rejected`: first 2 bytes = 0x0000 -> `DecodeError::BadMagic(0x0000)`. Verify stream is not consumed past header.
- [ ] `header_bad_magic_random_bytes`: first 2 bytes = 0xDEAD -> `DecodeError::BadMagic(0xDEAD)`.
- [ ] `header_unknown_flags_ignored`: flags=0xFF (all bits set, only COMPRESSED=0x01 defined) -- decode succeeds, unknown bits silently ignored. Verifies forward compatibility.
- [ ] `header_version_field_preserved`: version=5 in header -> decoded header.version == 5. (Header decode doesn't reject versions; version negotiation is at PDU level.)

Compression tests:
- [ ] `compression_roundtrip_large_payload`: encode a `NotifyPaneSnapshot` with 200x50 grid (>4KB payload). Assert COMPRESSED flag is set in wire bytes. Decode and assert equality.
- [ ] `compression_small_payload_uncompressed`: encode a `MuxPdu::Ping` (<4KB). Assert COMPRESSED flag is NOT set. Decode and assert equality.
- [ ] `compression_incompressible_sent_uncompressed`: encode with random-byte payload >4KB. If zstd output is larger than input, assert COMPRESSED flag is NOT set (fallback to uncompressed).
- [ ] `compression_max_payload_still_enforced`: 16MB limit applies to pre-compression payload size. A payload just over MAX_PAYLOAD should still be rejected even if it would compress below.
- [ ] `compression_flag_without_actual_compression`: craft a frame with COMPRESSED flag set but uncompressed payload -- should produce a zstd decompression error (not a panic).

Version negotiation tests (IPC tests in `server/tests.rs`, Unix-gated):
- [ ] `version_negotiation_same_version`: client sends Hello with protocol_version=1, server responds HelloAck with protocol_version=1. Handshake succeeds.
- [ ] `version_negotiation_higher_client_rejected`: client sends Hello with protocol_version=2 (> server's 1). Server responds with Error PDU and drops connection.
- [ ] `version_negotiation_lower_client_accepted`: client sends Hello with protocol_version=0 (< server's 1). Server responds HelloAck (backward compat).
- [ ] `feature_negotiation_intersection`: client sends features=FEAT_ZSTD, server supports FEAT_ZSTD -> HelloAck.features = FEAT_ZSTD (intersection).
- [ ] `feature_negotiation_no_features`: client sends features=0 -> HelloAck.features = 0. No compression used even for large payloads on this connection.
- [ ] `feature_negotiation_unknown_bits_preserved`: client sends features with unknown bit set (e.g., 0x80) -> server ignores unknown bits in intersection (result has only known bits).

Forward compat tests (update existing):
- [ ] Update `forward_compat_codec_skips_unknown_and_stays_aligned` to use 14-byte header format
- [ ] Update `frame_reader_forward_compat_skips_unknown_and_stays_aligned` in `server/tests.rs` to use 14-byte header

---

## 34.2 MuxServer Daemon

The `oriterm-mux` daemon process. Keeps all terminal sessions alive. Accepts connections from GUI clients. Routes pane output to subscribed clients.

**File:** `oriterm_mux/src/server/mod.rs`, `oriterm_mux/src/server/connection.rs`, `oriterm_mux/src/server/push/mod.rs`

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] `MuxServer` struct with `InProcessMux`, mio event loop, client connections, subscriptions -- `oriterm_mux/src/server/mod.rs` (390 lines) + 8 submodules (clients, connection, dispatch, frame_io, ipc, notify, pid_file, push) (verified 2026-04-04)
- [x] `ClientConnection` with write backpressure -- `oriterm_mux/src/server/connection.rs`, `clients.rs` (verified 2026-03-29)
- [x] Server event loop: single-threaded with `mio`, accepts connections, reads messages, dispatches to `InProcessMux`, pushes notifications (verified 2026-03-29)
- [x] Connection lifecycle: client disconnect unsubscribes panes, panes stay alive (verified 2026-03-29)
- [x] Daemon lifecycle: `oriterm-mux --daemon`/`--stop`, PID file management, signal handling (Unix+Windows) -- `oriterm_mux/src/bin/oriterm_mux.rs` (332 lines) (verified 2026-04-04)
- [x] PID file management -- `oriterm_mux/src/server/pid_file.rs` (verified 2026-03-29)
- [x] Tests: 470 tests pass (verified 2026-04-04)

**Architecture notes for remaining work:**

The server already maintains a `SnapshotCache` (`HashMap<PaneId, PaneSnapshot>`) in `server/snapshot.rs`. `Subscribe` already returns a full `PaneSnapshot` via `build_and_take()` (see `dispatch/mod.rs:260-266`). Verified 2026-04-04:
- `cleanup_pane_state()` removes snapshot cache entries ONLY on pane close, NOT on client disconnect (correct behavior).
- `disconnect_client()` (`clients.rs:266`) closes ORPHANED panes only.
- When ALL clients disconnect from a pane that is still alive, the push loop stops building snapshots (no subscribers). The SnapshotCache entry from the last push survives but goes stale.
- Subscribe handler calls `build_and_take()` which swaps a fresh snapshot from the IO thread -- so even if the cache is stale, the subscribe path gets fresh data. The staleness gap is already covered for the reconnection use case.

**Remaining hardening (genuinely not started):**
- [ ] Shadow snapshot (for reconnection):
  - [ ] Verify `SnapshotCache` entries persist when all clients unsubscribe from a pane (currently, `cleanup_pane_state` only runs on pane close per `server/mod.rs:360` -- this is correct behavior, no change needed)
  - [ ] When the last subscriber disconnects, keep the cached snapshot (it will go stale, but `build_and_take()` on Subscribe always swaps a fresh snapshot from the IO thread -- so staleness is not a problem for reconnecting clients)
  - [ ] On client `Subscribe(pane_id)`: the existing path already sends a full `PaneSnapshot` via `build_and_take()` -- **no additional work needed here** (verified in `dispatch/mod.rs:264`)
  - [ ] **Optional optimization:** Add periodic snapshot refresh for unsubscribed-but-alive panes (e.g., every 1s) so the cache stays warm. This is a nice-to-have -- the Subscribe path already gets fresh data, so this only saves the IO-thread snapshot-swap latency on reconnect (~sub-millisecond). Deprioritize.

**Remaining tests (IPC tests in `server/tests.rs`, Unix-gated):**
- [ ] `snapshot_cache_survives_unsubscribe`: connect client, subscribe to pane, unsubscribe, verify `SnapshotCache` still contains entry (not cleared). Requires inspecting `server.snapshot_cache` or re-subscribing and checking that `build_and_take` succeeds.
- [ ] `snapshot_cache_cleared_on_pane_close`: connect client, subscribe, close pane -> verify cache entry removed via `cleanup_pane_state`.
- [ ] `resubscribe_after_disconnect_gets_fresh_snapshot`: connect client A, subscribe to pane, disconnect A. Connect client B, subscribe to same pane -> receives a valid `PaneSnapshot` (not stale or empty).

---

## 34.3 OutputCoalescer

The push-based rendering engine. Coalesces rapid pane output (e.g., `cat large_file.txt`) into batched notifications with configurable latency targets. This is what makes mux-mode rendering fast.

**File:** `oriterm_mux/src/server/push/mod.rs`

**Reference:** WezTerm's 140ms poll (we beat this with push + coalesce)

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] Push-based snapshot delivery at ~250fps: 4ms throttle (`SNAPSHOT_PUSH_INTERVAL`), trailing-edge flush, backpressure deferral -- `oriterm_mux/src/server/push/mod.rs` (253 lines) (verified 2026-03-29)
- [x] Backpressure: slow clients above `WRITE_HIGH_WATER` (512KB) get deferred, latest state always delivered (verified 2026-03-29)

**Architecture notes for remaining work:**

Tiered coalescing requires the server to know each pane's visibility state per client. The current protocol has no such message -- a new `SetPanePriority` PDU is needed. The current flat 4ms interval is already very fast for the focused pane; tiered coalescing's main value is reducing CPU work for HIDDEN panes, not speeding up focused ones.

`PaneOutput` enrichment with `dirty_rows` tracking would require the IO thread to diff snapshots, which is not currently implemented and adds significant complexity. `cursor_changed` and `title_changed` are already covered by separate notification PDUs (`NotifyPaneMetadataChanged`). Dirty rows is a stretch goal.

**Remaining hardening (genuinely not started):**
- [ ] Tiered coalescing (different push intervals based on pane visibility):
  - [ ] Add `SetPanePriority` PDU: `{ pane_id: PaneId, priority: u8 }` where 0=focused, 1=visible, 2=hidden
    - [ ] Add `SetPanePriority` variant to `MuxPdu` in `protocol/messages.rs` (append at end for wire compat)
    - [ ] **FILE SIZE WARNING:** `messages.rs` is currently 485 lines. Adding `SetPanePriority` + extending `Hello`/`HelloAck` with version/features fields (34.1) will push it past 500 lines. Plan to split: extract `MuxPdu::msg_type()`, `is_fire_and_forget()`, and `is_notification()` match arms into a separate `protocol/pdu_traits.rs` submodule (~120 lines) before or during this work.
    - [ ] Add `MsgType::SetPanePriority` with a new ID (e.g., `0x0129`) in `protocol/msg_type.rs`
    - [ ] Add `MsgType::from_u16` match arm for the new ID
    - [ ] Add `MuxPdu::msg_type()` match arm returning `MsgType::SetPanePriority`
    - [ ] Add to `MuxPdu::is_fire_and_forget()` match (this is a fire-and-forget message -- client sends, no response expected)
    - [ ] Verify NOT added to `MuxPdu::is_notification()` -- this is a client-to-server request, not a server push notification
    - [ ] Handle in `dispatch_request` in `server/dispatch/mod.rs` -- add match arm that calls `conn.set_pane_priority(pane_id, priority)` and returns `None` (fire-and-forget, no response PDU)
  - [ ] Add per-pane priority storage to `ClientConnection`:
    - [ ] Add `pane_priorities: HashMap<PaneId, u8>` field to `ClientConnection` in `server/connection.rs`
    - [ ] Add `set_pane_priority(&mut self, pane_id: PaneId, priority: u8)` method
    - [ ] Add `pane_priority(&self, pane_id: PaneId) -> u8` method (returns 0/focused if not set)
    - [ ] Clean up pane priority entries in `unsubscribe()` method
  - [ ] Per-pane push interval based on priority:
    - [ ] **Focused (priority 0)**: 4ms (current `SNAPSHOT_PUSH_INTERVAL` -- already near-instant)
    - [ ] **Visible unfocused (priority 1)**: 16ms (~60 FPS) -- smooth but reduces CPU for multi-pane layouts
    - [ ] **Hidden (priority 2)**: 100ms -- low overhead for background tabs
    - [ ] Add constants: `VISIBLE_PUSH_INTERVAL = Duration::from_millis(16)`, `HIDDEN_PUSH_INTERVAL = Duration::from_millis(100)` in `server/push/mod.rs`
  - [ ] Multi-client priority resolution: when multiple clients subscribe to the same pane with different priorities (e.g., client A has it focused, client B has it hidden), use the HIGHEST priority (lowest number) among all subscribers. The push rate is determined by the most-interested client.
    - [ ] In `push_or_defer_pane()`: look up the minimum priority across all subscribers for this pane
    - [ ] Select the interval based on that minimum priority
  - [ ] `push_or_defer_pane` reads per-subscriber priority to select interval via `should_push()` call
  - [ ] `trailing_edge_flush` reads priority the same way
  - [ ] Client sends `SetPanePriority` when active tab changes or pane visibility changes (in `oriterm/src/app/` -- needs a call site in the tab/focus change handler)
  - [ ] Default priority for new subscriptions: 0 (focused) -- matches current behavior
- [ ] **Stretch/optional:** `PaneOutput` dirty row tracking:
  - [ ] Add `dirty_rows: Option<Vec<u16>>` to `NotifyPaneSnapshot` (which rows changed since last push)
  - [ ] Requires IO thread to diff current vs previous snapshot -- significant complexity, defer unless profiling shows benefit

**Remaining tests:**

Protocol tests (in `protocol/tests.rs`):
- [ ] `roundtrip_set_pane_priority`: encode/decode `MuxPdu::SetPanePriority { pane_id, priority: 1 }`. Assert roundtrip equality.
- [ ] `set_pane_priority_is_fire_and_forget`: assert `MuxPdu::SetPanePriority { .. }.is_fire_and_forget()` returns true.
- [ ] `msg_type_roundtrip_all`: add `MsgType::SetPanePriority` to the roundtrip array.

Push logic tests (in `server/push/tests.rs`):
- [ ] `should_push_respects_custom_interval`: `should_push(now, Some(now - 10ms), Duration::from_millis(16))` returns false (within 16ms visible interval). `should_push(now, Some(now - 20ms), Duration::from_millis(16))` returns true.
- [ ] `focused_pane_uses_4ms_interval`: simulate `push_or_defer_pane` with priority=0 -> `should_push` called with `SNAPSHOT_PUSH_INTERVAL` (4ms).
- [ ] `visible_pane_uses_16ms_interval`: simulate with priority=1 -> `should_push` called with `VISIBLE_PUSH_INTERVAL` (16ms).
- [ ] `hidden_pane_uses_100ms_interval`: simulate with priority=2 -> `should_push` called with `HIDDEN_PUSH_INTERVAL` (100ms).
- [ ] `priority_change_immediate_effect`: set priority=2 (hidden), advance 50ms, push deferred (within 100ms). Change to priority=0, push succeeds (past 4ms).
- [ ] `default_priority_is_focused`: newly subscribed pane with no `SetPanePriority` sent -> uses 4ms interval.

Server dispatch tests (IPC tests in `server/tests.rs`, Unix-gated):
- [ ] `set_pane_priority_dispatch`: send `SetPanePriority { pane_id, priority: 1 }` -> no response (fire-and-forget). Send Ping -> PingAck (stream not disrupted).
- [ ] `multi_client_priority_uses_highest`: client A sets priority=0 (focused), client B sets priority=2 (hidden) for same pane -> push uses 4ms (highest priority = lowest number among subscribers).

---

## 34.4 MuxClient + Auto-Start

The GUI's connection to the mux daemon. `MuxClient` implements the same API as `InProcessMux` so the App doesn't care whether it's local or daemon mode.

**File:** `oriterm_mux/src/backend/client/mod.rs`, `oriterm_mux/src/backend/client/transport/mod.rs`, `oriterm_mux/src/discovery/mod.rs`

**Already implemented (via Section 44, verified 2026-03-29):**
- [x] `MuxClient` with RPC + push notifications, background reader thread, dirty tracking, snapshot cache -- `oriterm_mux/src/backend/client/mod.rs` + submodules (verified 2026-03-29)
- [x] `MuxBackend` trait (~50 methods), implemented by both `EmbeddedMux` and `MuxClient` -- `oriterm_mux/src/backend/mod.rs` (340 lines) (verified 2026-04-04)
- [x] Auto-start daemon: `ensure_daemon()`, exponential backoff, stale PID cleanup -- `oriterm_mux/src/discovery/mod.rs` (140 lines) (verified 2026-04-04)
- [x] Tests: compile-time trait check, round-trip spawn, subscribe, auto-start all passing (verified 2026-03-29)

**Architecture notes for remaining work:**

Reconnection is more complex than it appears. The `ClientTransport` owns a reader thread with a self-pipe (Unix), pending RPC map, and health-check ping state. Reconnecting means:
1. Drop the old `ClientTransport` (joins reader thread, closes self-pipe -- synchronous, verified in `transport/mod.rs` Drop impl)
2. Establish a new connection (new Hello handshake, new `ClientId`)
3. Re-subscribe to all panes
4. Bridge the gap for any in-flight RPCs (they fail with `BrokenPipe` from `mpsc::RecvTimeoutError::Disconnected` -- callers already handle `io::Error` returns, no additional work needed)

Key ownership detail: `MuxClient::connect()` currently takes `(socket_path, wakeup)` and passes them through to `ClientTransport::connect()`. Neither the path nor the wakeup closure are stored on `MuxClient` -- they would need to be stored for reconnection.

The `pane_snapshots` cache survives reconnection (field on `MuxClient`, not `ClientTransport`), so the UI won't blank out during reconnect attempts.

**Remaining hardening (genuinely not started):**
- [ ] Reconnection:
  - [ ] Detect connection loss: `ClientTransport::is_alive()` returns false when reader thread exits (already implemented via `alive` AtomicBool in `transport/mod.rs:74`)
  - [ ] Store socket path and wakeup closure in `MuxClient` for reconnection:
    - [ ] Add `socket_path: PathBuf` field to `MuxClient`
    - [ ] Add `wakeup: Arc<dyn Fn() + Send + Sync>` field to `MuxClient`
    - [ ] Update `MuxClient::connect()` to store both before creating transport
  - [ ] Add `MuxClient::reconnect()` method:
    - [ ] Set `self.transport = None` -- this drops the old `ClientTransport`, which: (1) closes send channel, (2) signals wake pipe, (3) joins reader thread, (4) closes self-pipe write end (Unix). All synchronous.
    - [ ] Call `ClientTransport::connect(&self.socket_path, Arc::clone(&self.wakeup))`
    - [ ] This performs a fresh Hello handshake (gets a new `ClientId`)
    - [ ] `SetCapabilities` with `CAP_SNAPSHOT_PUSH` is sent inside `ClientTransport::connect()` already (seq 2) -- no additional work needed
    - [ ] Collect pane IDs from `self.pane_snapshots.keys()` into a Vec before iterating (borrow checker)
    - [ ] For each pane ID: call `subscribe_pane(pane_id)` which sends `Subscribe` RPC and caches the returned snapshot
    - [ ] If tiered coalescing is implemented: re-send `SetPanePriority` for all panes (needs stored priority state in MuxClient -- add `pane_priorities: HashMap<PaneId, u8>` field)
    - [ ] Store the new transport in `self.transport = Some(transport)`
  - [ ] Reconnection policy: attempt every 500ms, max 3 attempts, then surface error to GUI
    - [ ] Add `reconnect_with_backoff(&mut self) -> io::Result<()>` that loops with sleep
    - [ ] On final failure, return `Err` -- caller (App event loop) decides what to do (show error bar, fall back to embedded mode, etc.)
  - [ ] `pane_snapshots` cache survives reconnection -- it's a field on `MuxClient`, not on `ClientTransport`. UI shows last-known state during reconnection attempt.
  - [ ] `dirty_panes` and `pending_refresh` state: clear `pending_refresh` on reconnect (stale). Mark all panes in `pane_snapshots` as dirty so the render loop fetches fresh snapshots post-reconnect.
  - [ ] App event loop integration: check `backend.is_connected()` periodically (e.g., on timer or after failed operations), trigger reconnect. `MuxBackend::is_connected()` already has a default returning `true`, and `MuxClient` overrides it via `ClientTransport::is_alive()`.

**Remaining tests:**

Note: reconnection tests require a live daemon (or a mock transport). The existing `MuxClient::new()` test stub has no transport, so reconnection-specific tests need either (a) IPC integration tests in `server/tests.rs` (Unix-gated), or (b) a `MockTransport` test helper. The simpler path is IPC integration tests in `server/tests.rs` where a real daemon is spun up.

Unit tests (in `backend/client/tests.rs`, using `MuxClient::new()` stub where possible):
- [ ] `reconnect_clears_pending_refresh`: inject items into `pending_refresh`, call reconnect logic, verify `pending_refresh` is empty.
- [ ] `reconnect_marks_all_panes_dirty`: inject `pane_snapshots` entries, call reconnect logic, verify all pane IDs are in `dirty_panes`.
- [ ] `pane_snapshots_survive_reconnect`: verify `pane_snapshots` cache is NOT cleared during reconnect -- entries exist throughout.
- [ ] `is_connected_false_after_disconnect`: `MuxClient::is_connected()` returns false when transport's `alive` AtomicBool is false.

IPC integration tests (in `server/tests.rs`, Unix-gated, require live daemon):
- [ ] `reconnect_restores_subscriptions`: create `MuxClient`, subscribe to 3 panes, simulate disconnect, call `reconnect()` -> assert all 3 panes re-subscribed.
- [ ] `reconnect_failure_after_max_attempts`: create `MuxClient`, simulate disconnect, daemon not reachable -> `reconnect_with_backoff()` returns `Err` after 3 attempts.
- [ ] `inflight_rpc_returns_error_on_disconnect`: start an RPC, drop transport mid-flight -> `rpc()` returns `io::Error` (does not hang or panic).
- [ ] `reconnect_resends_pane_priorities`: if tiered coalescing is implemented, `reconnect()` re-sends `SetPanePriority` for all panes using stored priorities.

---

## 34.5 Section Completion

**Already complete (via Section 44, verified 2026-03-29):**
- [x] Wire protocol: 10-byte header, bincode payload, streaming codec (verified 2026-03-29)
- [x] MuxServer: accepts connections, routes messages, pushes output (verified 2026-03-29)
- [x] OutputCoalescer: 4ms flat push with backpressure (verified 2026-03-29)
- [x] MuxClient: same API as InProcessMux, transparent backend switching (verified 2026-03-29)
- [x] Auto-start daemon: seamless to user, fallback to in-process (verified 2026-03-29)
- [x] `cargo build --target x86_64-pc-windows-gnu` -- compiles (verified 2026-03-29)
- [x] `cargo clippy --target x86_64-pc-windows-gnu` -- no warnings (verified 2026-03-29)
- [x] `cargo test` -- 470 tests pass (verified 2026-04-04)

**Remaining for full completion:**
- [x] **Pre-requisite fixes** (found during review, fixed 2026-04-04):
  - [x] Fix stale comment in `server/mod.rs:203`: says "16ms" but `SNAPSHOT_PUSH_INTERVAL` is 4ms
  - [x] Fix stale module doc in `server/push/mod.rs:4`: says "~60fps (16ms interval)" but constant is 4ms (250fps)
  - [x] Fix stale doc comment in `protocol/messages.rs:365`: `NotifyPaneSnapshot` says "throttled to ~60fps" but actual rate is 250fps (4ms)
  - [x] Fix `msg_type_roundtrip_all` test: add missing `NotifyCommandComplete`, `NotifyClipboardStore`, `NotifyClipboardLoad` variants
- [ ] All 34.1-34.4 hardening items complete
- [ ] **Implementation order** (must be sequential due to dependencies):
  0. **Pre-existing fixes** (34.5 prerequisite): fix stale comments in `server/mod.rs:203`, `server/push/mod.rs:4`, and `protocol/messages.rs:365`. Fix missing `msg_type_roundtrip_all` variants. Add missing PDU roundtrip tests for `NotifyCommandComplete`/`NotifyClipboardStore`/`NotifyClipboardLoad`.
  1. **Extract shared decode logic** (34.1 prerequisite): create `protocol/decode.rs`, refactor both `ProtocolCodec::try_decode()` and `FrameReader::try_decode()` to use it. Verify all existing tests pass. This MUST happen before the header change -- otherwise two code paths must be updated identically.
  2. **Extract shared encode logic** (34.1 recommended): create `protocol/encode.rs`, refactor both `ProtocolCodec::encode_frame()` and `FrameWriter::queue()` to use it. Same rationale -- without this, compression must be implemented in two encode paths.
  3. **Wire protocol header extension** (34.1): magic + version + flags -- now only one decode path and one encode path to update. **Single-commit change.**
  4. **`messages.rs` split** (hygiene): extract `msg_type()`, `is_fire_and_forget()`, `is_notification()` into `protocol/pdu_traits.rs` to keep `messages.rs` under 500 lines before adding new variants.
  5. **Version negotiation** (34.1): extend Hello/HelloAck with version + features fields. **Single-commit** -- changes bincode encoding.
  6. **Compression** (34.1): zstd behind `FEAT_ZSTD` feature flag -- only enabled after version negotiation confirms both sides support it.
  7. **Shadow snapshot** (34.2): verify cache survives client disconnect (mostly already works, may be no-op).
  8. **Tiered coalescing** (34.3): `SetPanePriority` PDU + per-pane push intervals.
  9. **Reconnection** (34.4): `MuxClient::reconnect()` + re-subscribe + cache survival.
- [ ] **Latency test**: keystroke -> screen update < 5ms through daemon
- [ ] **Throughput test**: `cat large_file.txt` renders smoothly, no dropped frames
- [ ] **Reconnection test**: kill GUI, relaunch -> sessions restored instantly (pane snapshots cached)
- [ ] **Multi-client test**: two GUI windows connected to same daemon, both receive push snapshots

**Hygiene compliance checklist:**
- [ ] No source file (excluding `tests.rs`) exceeds 500 lines after all changes. Key files to watch:
  - `protocol/messages.rs` (485 lines now -- will exceed with Hello/HelloAck changes + SetPanePriority. Split required.)
  - `server/dispatch/mod.rs` (357 lines -- safe margin, but monitor with new match arm)
  - `server/mod.rs` (390 lines -- safe)
- [ ] All new tests follow sibling `tests.rs` pattern (no inline test modules)
- [ ] All new modules are directory modules if they have tests
- [ ] Crate boundaries respected: all protocol/server/client changes in `oriterm_mux`, client-side call site for `SetPanePriority` in `oriterm/src/app/`
- [ ] No algorithmic duplication: shared decode extracted to `protocol/decode.rs`, shared encode extracted to `protocol/encode.rs`
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` pass after each commit

**Exit Criteria:** Full server/client architecture. The daemon keeps sessions alive across GUI restarts. Push-based rendering with 4ms coalesce beats WezTerm's 140ms poll. Transparent backend switching lets the App work identically in local and daemon modes. Auto-start and graceful fallback make the daemon invisible to users who don't need it. Version negotiation enables forward-compatible protocol evolution. Compression reduces IPC bandwidth for large grid snapshots.

---

## 34.R Third Party Review Findings

**Review 2026-04-04 (Agent 2 verification pass):**

Source code bugs found during plan verification:
1. **Stale comment** in `server/mod.rs:203`: `// 16ms -- retries fire promptly.` but `SNAPSHOT_PUSH_INTERVAL` is `Duration::from_millis(4)` (4ms). Severity: cosmetic, misleading.
2. **Stale module doc** in `server/push/mod.rs:4`: `"Push rate is throttled to ~60fps (16ms interval)."` but constant is 4ms (250fps). Severity: cosmetic, misleading.
3. **Missing test coverage** in `protocol/tests.rs`: `msg_type_roundtrip_all` omits `NotifyCommandComplete` (0x0305), `NotifyClipboardStore` (0x0306), and `NotifyClipboardLoad` (0x0308). These MsgType variants exist in the enum but are not exercised in the roundtrip test. Severity: minor (the variants are used in production code, but test coverage has a gap).
4. **Algorithmic duplication** between `ProtocolCodec::try_decode()` (`protocol/codec.rs:179-223`) and `FrameReader::try_decode()` (`server/frame_io.rs:54-100`). Both implement identical header parsing, validation, payload deserialization logic. When the header format changes (34.1), both must be updated identically. Risk of drift. Consider extracting shared header validation before implementing the header extension.

**Review 2026-04-04 (Agent 3 completeness/hygiene pass):**

Changes made:
1. Expanded all test specifications from vague descriptions to concrete test function names with specific scenarios, inputs, and expected outputs.
2. Added 34.2 tests -- section had zero test specifications.
3. Added `protocol/decode.rs` extraction step as mandatory prerequisite before header change.
4. Added `messages.rs` file size warning and split task.
5. Added missing MuxPdu sync points for SetPanePriority.
6. Reordered implementation sequence with duplication extraction and file split inserted before dependent steps.
7. Added hygiene compliance checklist to 34.5 exit criteria.
8. Added missing edge case tests.

**Review 2026-04-04 (Agent 4 final review):**

Changes made:
1. Removed all `<!-- reviewed: ... -->` HTML comments from previous agent passes. Review notes belong in 34.R, not scattered inline.
2. Added `protocol/encode.rs` extraction step (step 2 in implementation order). Prior agents only identified decode duplication, but there are TWO independent encode paths (`ProtocolCodec::encode_frame()` and `FrameWriter::queue()`) with identical logic. Without extraction, compression must be implemented twice. Same risk as decode duplication.
3. Added missing stale comment: `protocol/messages.rs:365` says "throttled to ~60fps" but actual rate is 250fps (4ms). Prior agents found the `server/mod.rs` and `push/mod.rs` stale comments but missed this one.
4. Corrected reconnection test placement: split tests into unit tests (using `MuxClient::new()` stub) and IPC integration tests (requiring live daemon). Prior agents placed all reconnection tests in `backend/client/tests.rs`, but tests like `reconnect_restores_subscriptions` require a live daemon and belong in `server/tests.rs`.
5. Updated `last_verified` in frontmatter to 2026-04-04 to reflect this review pass.
6. Cleaned up formatting: converted em-dashes and special Unicode arrows to ASCII equivalents for consistency.
