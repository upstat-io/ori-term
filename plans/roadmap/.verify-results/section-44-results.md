# Section 44: Multi-Process Window Architecture — Verification Results

**Context loaded:** CLAUDE.md (read), rules/*.md (3 files read: code-hygiene.md, test-organization.md, impl-hygiene.md), crate-boundaries.md (loaded via system reminder), reference: wezterm (`wezterm-mux-server-impl/src/sessionhandler.rs`), tmux (referenced in CLAUDE.md)

**Section status:** `complete` (all 7 subsections marked complete)
**Reviewed:** `true`

---

## Test Execution

```
cargo test -p oriterm_mux: 433 tests passed (390 unit + 20 contract + 23 e2e)
cargo test -p oriterm_ipc: 6 tests passed
cargo build --target x86_64-pc-windows-gnu -p oriterm_mux -p oriterm_ipc: OK (compiles)
```

All tests run with `timeout 150`. No hangs. No failures.

---

## 44.1 Mux Daemon Binary

### Items Verified

| Item | Status | Evidence |
|------|--------|----------|
| `oriterm-mux` binary at `oriterm_mux/src/bin/oriterm_mux.rs` | PASS | 332 lines, implements `Mode::Foreground`, `Mode::Daemon`, `Mode::Stop` |
| `--daemon` flag: fork/detach | PASS | `run_daemon()` uses `Command::new(exe).arg("--foreground")` with `Stdio::null()` pipes; Windows uses `creation_flags(0x200 \| 0x8)` (CREATE_NEW_PROCESS_GROUP \| DETACHED_PROCESS) |
| `--foreground` flag | PASS | `run_foreground()` calls `MuxServer::new()` then `server.run()` |
| PID file creation | PASS | `PidFile::create_at()` in `server/pid_file.rs`, tested by `pid_file_creates_and_removes_on_drop` |
| Graceful shutdown: SIGTERM/SIGINT | PASS | `register_shutdown_handler()` on Unix uses `signal_hook::flag::register(SIGTERM\|SIGINT)`. Windows uses `SetConsoleCtrlHandler` with `CTRL_C_EVENT`, `CTRL_BREAK_EVENT`, `CTRL_CLOSE_EVENT` |
| `MuxServer` struct | PASS | Owns `InProcessMux`, `IpcListener`, `HashMap<ClientId, ClientConnection>`, `HashMap<PaneId, Vec<ClientId>>` subscriptions |
| Server event loop (mio-based) | PASS | `run()` in `server/mod.rs` uses `Poll::poll()` with `LISTENER`, `WAKER`, `CLIENT_BASE` tokens |
| Connection lifecycle (Hello, subscribe, disconnect, persist) | PASS | Tests: `hello_handshake_roundtrip`, `disconnect_removes_client`, `server_exits_after_client_disconnects_and_no_panes` |
| Daemon exit conditions | PASS | `should_exit()`: grace period (5s), `had_client` flag, `connections.empty() && panes.empty()`. Tests: `server_does_not_exit_during_grace_period`, `server_does_not_exit_before_first_client`, `server_exits_after_client_disconnects_and_no_panes` |
| `--stop` command via IPC | PASS | `run_stop()` sends Hello then Shutdown PDU; falls back to SIGTERM on Unix. `ShutdownAck` round-trip tested in `shutdown_without_hello_sets_flag` |

### Tests (semantic pins)

- `server_creates_pid_file_and_socket` — verifies PID file and socket creation at start
- `hello_handshake_roundtrip` — client connects, sends Hello, receives HelloAck with non-zero client_id
- `disconnect_removes_client` — dropping client connection causes `client_count()` to reach 0
- `server_shutdown_flag_stops_event_loop` — setting shutdown flag causes `run()` to return
- `input_is_fire_and_forget` — Input PDU produces no response; subsequent Ping returns PingAck
- `unexpected_pdu_returns_error` — sending PaneClosedAck (a response) returns Error PDU

### Verdict: PASS

---

## 44.2 IPC Protocol (Minimal Viable)

### Items Verified

| Item | Status | Evidence |
|------|--------|----------|
| 10-byte frame header (type u16, seq u32, payload_len u32) | PASS | `FrameHeader` in `protocol/mod.rs`, header `encode()`/`decode()` tested by `header_roundtrip`, `header_zero_values`, `header_max_values` |
| Bincode payload encoding | PASS | `ProtocolCodec::encode_frame` serializes via `bincode::serialize`, decoded by `bincode::deserialize` |
| Max 16MB payload enforced | PASS | `MAX_PAYLOAD = 16 * 1024 * 1024`. Tests: `encode_rejects_payload_exceeding_max`, `decode_payload_too_large` |
| `MsgType` wire IDs (0x01xx requests, 0x02xx responses, 0x03xx notifications) | PASS | `msg_type.rs` with `from_u16` roundtrip; `msg_type_roundtrip_all` tests every variant |
| Hello/HelloAck handshake | PASS | `roundtrip_hello`, `roundtrip_hello_ack` |
| Fire-and-forget messages (Input, Resize) | PASS | `roundtrip_input_fire_and_forget`, `roundtrip_resize_fire_and_forget`, `fire_and_forget_no_block` |
| Push notifications | PASS | `notification_delivery` — encodes 3 notifications, decodes all with `is_notification()` true |
| Sequence correlation | PASS | `sequence_correlation` — request and response share seq, different requests have different seq |
| `PaneSnapshot` with CJK, emoji, combining marks | PASS | `snapshot_with_cjk_emoji_combining` — tests CJK `'漢'`, emoji `'🦀'`, combining marks `'\u{0301}'` |
| Large snapshot roundtrip (200x50) | PASS | `roundtrip_large_pane_snapshot` — 200 cols x 50 rows, spot-checks cells |
| Forward compatibility (unknown msg_type) | PASS | `forward_compat_codec_skips_unknown_and_stays_aligned` — unknown frame consumed, next frame decoded correctly |
| Wire byte stability | PASS | `wire_bytes_stable_for_hello` — pins header bytes and bincode encoding for Hello{pid:42} |

### Protocol Message Inventory

**Actually implemented PDUs (from `messages.rs`):**

Requests: `Hello`, `ClosePane`, `Input`, `Resize`, `Subscribe`, `Unsubscribe`, `GetPaneSnapshot`, `Ping`, `Shutdown`, `ScrollDisplay`, `ScrollToBottom`, `ScrollToPrompt`, `SetTheme`, `SetCursorShape`, `MarkAllDirty`, `OpenSearch`, `CloseSearch`, `SearchSetQuery`, `SearchNextMatch`, `SearchPrevMatch`, `ExtractText`, `ExtractHtml`, `SetCapabilities`, `SpawnPane`, `ListPanes`, `SetImageConfig`

Responses: `HelloAck`, `PaneClosedAck`, `Subscribed`, `Unsubscribed`, `PaneSnapshotResp`, `PingAck`, `ShutdownAck`, `ScrollToPromptAck`, `ExtractTextResp`, `ExtractHtmlResp`, `SpawnPaneResponse`, `ListPanesResponse`, `Error`

Notifications: `NotifyPaneOutput`, `NotifyPaneExited`, `NotifyPaneMetadataChanged`, `NotifyPaneBell`, `NotifyCommandComplete`, `NotifyClipboardStore`, `NotifyClipboardLoad`, `NotifyPaneSnapshot`

### CRITICAL FINDING: Section vs Reality Mismatch

The section lists the following PDUs as implemented (checked `[x]`) but they **do not exist in the codebase**:

| Claimed PDU | Actual Status |
|------------|---------------|
| `CreateWindow` / `WindowCreated` | NOT IMPLEMENTED. No `WindowId` in `oriterm_mux` at all. |
| `CreateTab` / `TabCreated` | NOT IMPLEMENTED. The mux is pane-only (no tab/window concepts). |
| `CloseTab` / `TabClosed` | NOT IMPLEMENTED. |
| `MoveTabToWindow` / `TabMoved` | NOT IMPLEMENTED. |
| `ListWindows` / `WindowList` | NOT IMPLEMENTED. |
| `ListTabs` / `TabList` | NOT IMPLEMENTED. |
| `SplitPane` / `PaneSplit` | NOT IMPLEMENTED. |
| `CycleTab` / `ActiveTabChanged` | NOT IMPLEMENTED. |
| `SetActiveTab` / `ActiveTabChanged` | NOT IMPLEMENTED. |
| `PaneOutput { dirty_rows }` | PARTIALLY. `NotifyPaneOutput` exists but carries only `pane_id`, no `dirty_rows`. |
| `PaneTitleChanged` | RENAMED to `NotifyPaneMetadataChanged` (with `title` field). |
| `WindowTabsChanged` notification | NOT IMPLEMENTED (no window/tab concepts in mux). |
| `TabMoved` notification | NOT IMPLEMENTED. |

**What the protocol actually implements instead:** A pane-centric model. `SpawnPane` replaces `CreateTab`. `ListPanes` replaces `ListWindows`/`ListTabs`. No tab/window grouping exists in the mux layer -- this is consistent with CLAUDE.md's statement that "oriterm_mux is a pane-only server -- no tabs, windows, sessions, or layouts."

**Impact:** The section's 44.2 checklist is inaccurate. The protocol was designed pane-first (correct per architecture), but the plan text still describes a tab/window-centric protocol that was never built. The section should be updated to reflect the actual protocol design.

### Transport

| Item | Status | Evidence |
|------|--------|----------|
| Unix domain socket | PASS | `oriterm_ipc/src/unix/` — listener, stream, client_stream implementations |
| Named pipe on Windows | IMPLEMENTED (marked deferred in section) | `oriterm_ipc/src/windows/` — `IpcListener` uses `CreateNamedPipeW`, pipe name at `\\.\pipe\oriterm-mux-{USERNAME}`. Cross-compiles for Windows. |

**Finding:** Section claims Windows named pipes are deferred (`<!-- deferred: Windows platform support -->`), but `oriterm_ipc/src/windows/` has a full implementation (listener.rs: 197 lines, client_stream.rs, stream.rs, pipe_name.rs). The section understates the actual progress.

### Verdict: FAIL (section text inaccurate -- many claimed PDUs don't exist; actual protocol is pane-centric)

---

## 44.3 Window-as-Client Model

### Items Verified

| Item | Status | Evidence |
|------|--------|----------|
| `MuxClient` struct | PASS | `backend/client/mod.rs` — wraps `ClientTransport`, `HashMap<PaneId, PaneSnapshot>`, `HashSet<PaneId>` dirty tracking |
| `MuxBackend` trait | PASS | `backend/mod.rs` — 287 lines, defines 30+ methods covering event pump, pane ops, grid ops, scroll, search, clipboard, snapshot |
| `EmbeddedMux` implements `MuxBackend` | PASS | `backend/embedded/mod.rs` — full impl, 381 lines |
| `MuxClient` implements `MuxBackend` | PASS | `backend/client/rpc_methods.rs` implements all trait methods via RPC or fire-and-forget |
| Object safety (Box<dyn MuxBackend>) | PASS | Tests `object_safe` in both `embedded/tests.rs` and `client/tests.rs` |
| App uses `Box<dyn MuxBackend>` | PASS | `oriterm/src/app/mod.rs` — `mux: Option<Box<dyn MuxBackend>>` |
| `App::new_daemon()` constructor | PASS | `oriterm/src/app/constructors.rs` — connects `MuxClient` to socket path |
| Render flow (snapshot-based) | PASS | `EmbeddedMux::refresh_pane_snapshot` builds via `build_snapshot_into`; `MuxClient::refresh_pane_snapshot` uses RPC `GetPaneSnapshot` or pushed snapshots |
| Push notifications from daemon | PASS | `ClientTransport` has background reader thread that routes notifications via `notif_tx` channel |
| Per-window process state (GPU, fonts) | PASS | Confirmed by architecture -- `oriterm` binary owns all GPU/font state independently |

### Tests (semantic pins)

**Unit tests (13):**
- `embedded/tests.rs`: `object_safe`, `drain_empty`, `discard_notifications`, `poll_events_empty`, `event_tx_available`, `pane_ids_empty`, `get_pane_entry_after_inject`, `pane_entry_gone_after_close`, `close_pane_emits_notification`, `embedded_mux_is_send`, `is_not_daemon_mode`
- `client/tests.rs`: `object_safe`, `drain_empty`, `poll_events_noop`, `drain_returns_injected_notifications`, `discard_clears_notifications`, `is_daemon_mode`, `event_tx_none`, `pane_ids_empty`, plus snapshot cache tests (`cache_snapshot_then_retrieve`, `cache_snapshot_overwrites`, `pane_snapshot_returns_none_for_unknown`, `remove_snapshot_evicts_and_clears_dirty`)

**Contract tests (20 — 10 per backend):** `contract_spawn_pane_and_see_output`, `contract_resize`, `contract_snapshot_lifecycle`, `contract_cursor_shape`, `contract_mode_query`, `contract_search`, `contract_scroll`, `contract_extract_text`, `contract_flood_output`, `contract_flood_render_loop` -- run identically for both `EmbeddedMux` and daemon-backed `MuxClient`.

**E2E tests (23):** `client_spawn_pane_type_see_output`, `push_notification_triggers_dirty_flag`, `multiple_clients_independent_windows`, `client_crash_cleans_up_owned_window`, plus operations tests (resize, scroll, search, cursor shape, snapshot dirty flag, extract text, flood tests, IPC latency).

### Verdict: PASS

---

## 44.4 Cross-Process Tab Migration

### CRITICAL FINDING: Not Implemented

The section claims this subsection is complete, but **no cross-process tab migration exists in the codebase**:

1. No `MoveTabToWindow` PDU in the protocol.
2. No `MoveTabToNewWindow` PDU in the protocol.
3. No `TabMoved` notification.
4. No `WindowTabsChanged` notification.
5. No code in `oriterm/src/app/tab_management/` that spawns a new `oriterm` process for tab migration.
6. No `--position x,y` CLI argument for tear-off window positioning.
7. The mux layer has no concept of tabs or windows -- it is pane-only.

**What does exist:**
- The keybinding `MoveTabToNewWindow` exists in `oriterm/src/keybindings/mod.rs` and event `TermEvent::MoveTabToNewWindow(TabId)` exists in `oriterm/src/event.rs`.
- The old `_old/` prototype had in-process tab-to-window movement.
- But no daemon-based cross-process tab migration machinery exists.

**The section text describes a detailed 9-step flow, a tear-off flow, and move-to-existing-window flow, along with 7 test items -- none of which exist.**

### Verdict: FAIL (not implemented; section falsely marked complete)

---

## 44.5 Auto-Start + Discovery

### Items Verified

| Item | Status | Evidence |
|------|--------|----------|
| `ensure_daemon()` function | PASS | `discovery/mod.rs` — checks PID file, probes socket, spawns daemon, waits with backoff |
| `start_daemon()` spawns `oriterm-mux --daemon` | PASS | Locates sibling binary, spawns with `Stdio::null()`, Windows detach flags |
| `wait_for_socket()` with exponential backoff | PASS | Starts at 10ms, doubles, max 2550ms total |
| `validate_pid_file()` stale cleanup | PASS | Checks `oriterm_ipc::validate_pid()`, removes stale PID + socket files |
| `--connect <socket> --window <window_id>` | PASS | CLI args in `cli/mod.rs`, `--window` requires `--connect`. `App::new_daemon()` stores `active_window` |
| Discovery: Unix socket | PASS | `$XDG_RUNTIME_DIR/oriterm-mux.sock` via `oriterm_ipc::ipc_addr()` |
| Ping/PingAck health check | PASS | `MuxPdu::Ping`/`PingAck` in protocol. `MuxClient::ping_rpc()` measures round-trip. Reader thread sends periodic pings (`PING_INTERVAL = 5s`) |
| `is_connected()` on `MuxBackend` | PASS | `MuxClient::is_connected()` delegates to `ClientTransport::is_alive()` |
| Daemon auto-start retry + fallback | PASS | `ensure_daemon_with_retry()` in `main.rs` tries 3 times, falls back to embedded mode |

### Tests (13 discovery tests)

- `probe_daemon_success` / `probe_daemon_no_server` / `probe_daemon_stale_socket_file`
- `validate_pid_file_live_process` / `validate_pid_file_stale_cleanup` / `validate_pid_file_missing` / `validate_pid_file_invalid_content` / `validate_pid_file_trailing_whitespace`
- `wait_for_socket_already_available` / `wait_for_socket_timeout` / `wait_for_socket_delayed_start`
- `ensure_daemon_with_existing_daemon` / `multiple_sockets_dead_pruning`

Plus e2e test: `daemon_restart_detection_and_reconnect` — kills daemon, client detects via `is_connected()`.

### Finding

The section claims `Ping/PingAck` was added but does not list it in the 44.2 protocol message types. In reality, `Ping`/`PingAck` are fully implemented in the protocol, the server dispatch, and the client transport (periodic health-check pings every 5s).

### Verdict: PASS

---

## 44.6 Backward Compatibility + Fallback

### Items Verified

| Item | Status | Evidence |
|------|--------|----------|
| `MuxBackend` trait ensures identical app code | PASS | App uses `Option<Box<dyn MuxBackend>>` -- no direct `InProcessMux` access |
| `process_model = "daemon" \| "embedded"` config | PASS | `ProcessModel` enum in `config/mod.rs` with `#[default] Daemon`. Tests: `process_model_defaults_to_daemon`, `process_model_embedded_parses`, `process_model_ignores_invalid` |
| `--embedded` CLI flag | PASS | `cli/mod.rs` `embedded: bool`, tested by `embedded_flag_parses`, `embedded_flag_defaults_to_false` |
| Fallback: 3 retries then embedded | PASS | `ensure_daemon_with_retry()` in `main.rs` |
| Testing uses embedded mode | PASS | All 390 unit tests + 20 contract tests work in embedded mode; e2e tests explicitly start a TestDaemon |

### Verdict: PASS

---

## 44.7 Section Completion

### Completion Criteria Assessment

| Criterion | Status | Notes |
|-----------|--------|-------|
| `oriterm-mux` binary | PASS | Compiles, starts, accepts connections |
| IPC protocol | PASS (pane-centric, not tab/window-centric as described) | Binary framing, request/response, push notifications all working |
| Window-as-client (`MuxBackend` trait) | PASS | Both backends implement it, App is mode-agnostic |
| Cross-process tab migration | FAIL | NOT IMPLEMENTED |
| Auto-start discovery | PASS | Full flow with retries and fallback |
| Backward compatibility | PASS | Embedded mode works, config switch, CLI flag |
| Windows cross-compile | PASS | `cargo build --target x86_64-pc-windows-gnu` succeeds |
| All tests pass | PASS | 433 mux tests + 6 IPC tests |
| IPC latency < 5ms | PASS | `ipc_latency_under_5ms` e2e test (0.021ms measured in `raw_socket_latency_baseline`) |

### Missing from Completion Criteria

- **Tab migration test** ("move tab to new window -> running command uninterrupted") — not implemented
- **Scrollback test** ("moved tab retains full scrollback history") — not implemented (no tab migration)
- **Multi-window test** ("3 windows, move tabs between them") — multi-client exists but tab migration does not
- **Crash isolation test** ("kill one window process -> others unaffected") — partially tested by `client_crash_cleans_up_owned_window` but that test verifies pane cleanup, not session survival for other windows
- **Daemon restart test** — tested by `daemon_restart_detection_and_reconnect`

---

## Code Hygiene Audit

### File Size (500-line limit)

All implementation files under 500 lines. Largest: `protocol/messages.rs` at 458 lines.

### Test Organization

All test files follow the sibling `tests.rs` pattern with `#[cfg(test)] mod tests;` at the bottom of source files. No inline test bodies. Import style follows `super::` / `crate::` convention.

### Module Organization

- `//!` module docs on every file read.
- Import groups properly separated (std, external, crate).
- `#[allow(clippy)]` always has `reason = "..."`.
- No `unwrap()` in library code (found one `expect` in codec's `try_decode` for a length-checked slice, which is acceptable).
- No `println!`/`eprintln!` in library code (binary uses `eprintln!` for CLI output, which is appropriate).

### Platform Support

Both Unix and Windows implementations exist for IPC (`oriterm_ipc`). The daemon binary has proper `#[cfg(unix)]` and `#[cfg(windows)]` signal handler branches. Self-pipe wakeup on Unix, timeout-based polling on Windows.

### Error Handling

- `MuxServer::new()` returns `io::Result`.
- `MuxClient::connect()` returns `io::Result`.
- `ClientTransport::rpc()` returns `io::Result` with 5s timeout.
- Protocol decode errors are handled gracefully (unknown msg_type consumes full frame, next frame decoded correctly).
- `DispatchResult` wraps responses; protocol violations return `MuxPdu::Error`.

### Performance

- Reusable scratch buffers: `notification_buf`, `scratch_clients`, `scratch_panes` on `MuxServer` avoid per-cycle allocation.
- Snapshot cache with `build_snapshot_into` reuses `RenderableContent` Vec allocations.
- Push throttling at ~60fps (`SNAPSHOT_PUSH_INTERVAL = 16ms`) with trailing-edge flush.
- Wakeup coalescing via `AtomicBool` guard prevents flood of `PostMessage` syscalls.

---

## Gap Analysis

### Section Goal Fulfillment

The stated goal: "Every oriterm window runs as an independent OS process. A mux daemon owns all PTY sessions. When a user opens a new window, moves a tab to a new window, or tears off a tab, a new process spawns and connects to the daemon."

**What IS fulfilled:**
1. The daemon binary (`oriterm-mux`) runs and owns all PTY sessions.
2. Window processes connect to the daemon and render via snapshots.
3. Multiple independent windows (processes) can connect simultaneously.
4. Window crash does not lose PTY sessions.
5. Auto-start and discovery work.
6. Embedded mode fallback works.
7. The `MuxBackend` trait makes app code mode-agnostic.

**What is NOT fulfilled:**
1. **Cross-process tab migration does not exist.** There is no mechanism to move a tab (pane grouping) from one window process to another via the daemon. The protocol is pane-centric with no tab/window concepts.
2. **Tab tear-off does not spawn a new process.** The existing tear-off code in `oriterm/src/app/tab_drag/tear_off.rs` creates a new winit window in the SAME process, not a separate process.
3. **"Move to New Window" does not spawn a new OS process.** It creates a new in-process window.

### Specific Missing Items

1. **Pane-to-process reassignment protocol** — No mechanism for one client to "hand off" a pane subscription to a newly spawned client process.
2. **CLI `--position x,y` flag** — Not implemented (section 44.4 claims new windows spawn at cursor position during tear-off).
3. **Cross-process window lifecycle** — When a client disconnects, the daemon cleans up its panes. There is no "session persistence" where panes survive client disconnect and can be claimed by a new client (the `client_crash_cleans_up_owned_window` e2e test confirms panes are cleaned up on disconnect).

### Summary of Discrepancies

The section describes a Chrome-like model where the daemon owns tab/window grouping and tab migration crosses process boundaries. The actual implementation is simpler and correct for the architecture stated in CLAUDE.md: the mux is a **flat pane server** with no tab/window knowledge. Window processes own their own session models. This is a valid design (similar to tmux where the server owns sessions but clients can detach/reattach), but it is NOT the design described in the section text.

The section needs to be rewritten to accurately describe what was built:
- Pane-centric protocol (not tab/window-centric)
- Session model stays in the window process
- Cross-process tab migration is deferred (not complete)
- Windows named pipes are actually implemented (not deferred)

---

## Verdict Summary

| Subsection | Verdict | Notes |
|------------|---------|-------|
| 44.1 Mux Daemon Binary | PASS | Fully implemented and tested |
| 44.2 IPC Protocol | FAIL | Protocol is implemented and works well, but section text describes PDUs that don't exist (tab/window-level). Actual protocol is pane-centric. |
| 44.3 Window-as-Client Model | PASS | MuxBackend trait, both backends, App rewiring all complete |
| 44.4 Cross-Process Tab Migration | FAIL | Not implemented at all. Section falsely marked complete. |
| 44.5 Auto-Start + Discovery | PASS | Fully implemented and tested |
| 44.6 Backward Compatibility + Fallback | PASS | Fully implemented and tested |
| 44.7 Section Completion | FAIL | Exit criteria not met due to 44.4 |

**Overall Section Verdict: FAIL**

The core infrastructure (daemon, IPC, client model, discovery, fallback) is solid and well-tested. But the section's central differentiating claim -- cross-process tab migration -- is not implemented, and the section text inaccurately describes a tab/window-centric protocol that does not match the actual pane-centric implementation. The section should be corrected to accurately reflect the current state and 44.4 should be marked `not-started` or split into a future section.
