# Section 34: IPC Protocol + Daemon Mode -- Verification Results

**Verified:** 2026-03-29
**Status in plan:** not-started
**Actual status:** MASSIVELY OUTDATED -- the plan describes building infrastructure that already exists and is production-quality

---

## Critical Finding: Plan Is Stale

Section 34's frontmatter says it depends on Section 44 and adds "production hardening" on top of the working daemon. However, the section body (34.1-34.4) describes building the *entire* wire protocol, MuxServer, MuxClient, and MuxBackend from scratch. In reality, all of this already exists and is fully operational:

### What Already Exists (Built in Section 44)

| Plan Item | Already Implemented | Evidence |
|-----------|-------------------|----------|
| Wire protocol with binary framing | YES -- 10-byte header (type u16, seq u32, payload_len u32) + bincode payload | `oriterm_mux/src/protocol/mod.rs` (75 lines), `codec.rs`, `messages.rs` |
| Frame encode/decode + roundtrip tests | YES -- extensive test suite (1060+ lines) | `oriterm_mux/src/protocol/tests.rs` |
| MuxPdu enum with all message types | YES -- 40+ variants covering spawn, close, input, resize, subscribe, search, theme, cursor, images, clipboard | `oriterm_mux/src/protocol/messages.rs` (459 lines) |
| MsgType wire IDs for pre-routing | YES -- full msg_type module | `oriterm_mux/src/protocol/msg_type.rs` |
| Wire snapshot types (WireCell, WireCursor, etc.) | YES | `oriterm_mux/src/protocol/snapshot.rs` |
| ProtocolCodec with streaming decode | YES -- handles partial reads, forward-compat (skips unknown msg_type) | `oriterm_mux/src/protocol/codec.rs` |
| MuxServer with mio event loop | YES -- single-threaded, accepts connections, dispatches requests, pushes notifications | `oriterm_mux/src/server/mod.rs` (364 lines) + 7 submodules |
| Client connections with write backpressure | YES | `oriterm_mux/src/server/connection.rs`, `clients.rs` |
| PID file management | YES | `oriterm_mux/src/server/pid_file.rs` |
| Push-based snapshot delivery (~60fps) | YES -- 16ms throttle, trailing-edge flush, backpressure deferral | `oriterm_mux/src/server/push/mod.rs` (248 lines) |
| MuxClient with RPC + push notifications | YES -- background reader thread, dirty tracking, snapshot cache | `oriterm_mux/src/backend/client/mod.rs` + submodules |
| MuxBackend trait (unified API) | YES -- 47 methods, implemented by both EmbeddedMux and MuxClient | `oriterm_mux/src/backend/mod.rs` (287 lines) |
| oriterm-mux binary (daemon/foreground/stop) | YES -- full CLI, signal handling (Unix+Windows), IPC shutdown | `oriterm_mux/src/bin/oriterm_mux.rs` (333 lines) |
| Auto-start daemon + discovery | YES -- ensure_daemon(), exponential backoff, stale PID cleanup | `oriterm_mux/src/discovery/mod.rs` (140 lines) |
| Unix domain sockets + Windows named pipes | YES -- full cross-platform IPC in oriterm_ipc crate | `oriterm_ipc/src/` (unix + windows modules) |
| Transport layer (local IPC) | YES | `oriterm_mux/src/backend/client/transport/` |
| Tests: 433 tests pass (390 unit + 20 contract + 23 e2e) | YES | Per section-44 verification |

### What Section 34 Claims To Add (the "hardening")

The section NOTE at line 28-31 says this section adds hardening on top of Section 44. But the actual checkboxes (34.1-34.4) describe building everything from scratch:

1. **34.1 Wire Protocol** -- describes a 15-byte header with magic bytes, version, flags. The *actual* implementation uses a 10-byte header (no magic, no version field, no flags). The plan also lists zstd compression and version negotiation.
2. **34.2 MuxServer Daemon** -- describes building MuxServer from scratch. Already exists.
3. **34.3 OutputCoalescer** -- describes a tiered 1ms/16ms/100ms coalescer. The actual implementation uses a flat 16ms push interval with backpressure deferral and trailing-edge flush. The tiered approach (focused/visible/hidden) does not exist.
4. **34.4 MuxClient + Auto-Start** -- describes building MuxClient from scratch. Already exists.

### What Is Actually Not Started (the real remaining work)

1. **zstd compression** for large payloads (> 4KB) -- not implemented, no zstd dependency
2. **Version negotiation** (Hello/HelloAck with version + feature flags) -- not implemented. Current Hello only sends PID.
3. **Tiered coalescing** (1ms focused / 16ms visible / 100ms hidden) -- not implemented. All panes use flat 16ms.
4. **COMPRESSED flag** in frame header -- not implemented (no flags field)
5. **Magic bytes** in header -- not implemented
6. **Reconnection** logic in MuxClient -- not implemented (connection drop = dead)

---

## TODOs/FIXMEs Found

- `oriterm_mux/src/domain/wsl.rs`: "WSL domain stub" -- `can_spawn() = false`, placeholder for Section 35
- `oriterm_core/src/term/handler/dcs.rs`: `modifyOtherKeys` stub implementations (lines 98-117)

---

## Gap Analysis

The plan text is severely misleading. Someone reading 34.1-34.4 would think none of this infrastructure exists. The plan needs a complete rewrite:

1. **Remove all items that already exist** (MuxServer, MuxClient, MuxBackend, wire protocol, daemon binary, auto-start, etc.)
2. **Retain only genuine hardening items**: zstd compression, version negotiation, tiered coalescing, reconnection, magic bytes
3. **Update the frame format** -- plan says 15-byte header, actual is 10-byte. Decide whether to add magic/version/flags or not.
4. **Dependency is inverted** -- Plan says "depends on Section 44." In reality, Section 44 already built everything. This section should just describe incremental improvements.

The section is NOT "not-started" -- it's 80%+ complete via Section 44. Only the polish items (compression, version negotiation, tiered coalescing, reconnection) are truly not started.

---

## Recommendation

**Rewrite this section** to accurately describe only the remaining hardening work. Current plan text would cause massive confusion and wasted effort if someone tried to implement it as written.
