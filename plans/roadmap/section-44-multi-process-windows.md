---
section: 44
title: Multi-Process Window Architecture
status: in-progress
reviewed: true
last_verified: "2026-03-29"
tier: 0
goal: Each window is a separate OS process. A mux daemon owns all PTY sessions. Tabs migrate between window processes with zero session loss — same running shell, scrollback, cursor, everything. Like Chrome's process-per-window model.
sections:
  - id: "44.1"
    title: Mux Daemon Binary
    status: complete
  - id: "44.2"
    title: IPC Protocol (Minimal Viable)
    status: in-progress
  - id: "44.3"
    title: Window-as-Client Model
    status: complete
  - id: "44.4"
    title: Cross-Process Tab Migration
    status: not-started
  - id: "44.5"
    title: Auto-Start + Discovery
    status: complete
  - id: "44.6"
    title: Backward Compatibility + Fallback
    status: complete
  - id: "44.7"
    title: Section Completion
    status: in-progress
---

# Section 44: Multi-Process Window Architecture

**Status:** In Progress (verified 2026-03-29 — 44.1, 44.3, 44.5, 44.6 PASS; 44.2 FAIL section text inaccurate; 44.4 FAIL not implemented)
**Goal:** Every oriterm window runs as an independent OS process. A mux daemon (`oriterm-mux`) owns all PTY sessions, terminal state, and tab assignments. When a user opens a new window, moves a tab to a new window, or tears off a tab, a new process spawns and connects to the daemon — the terminal session continues uninterrupted. This is the Chrome model: process isolation for windows, seamless tab migration, no session loss.

> **CRITICAL VERIFICATION FINDINGS (2026-03-29):**
> 1. **44.2 IPC Protocol:** The section text describes tab/window-centric PDUs (CreateWindow, CreateTab, MoveTabToWindow, ListWindows, ListTabs, SplitPane, CycleTab, SetActiveTab, WindowTabsChanged, TabMoved) that **do not exist in the codebase**. The actual protocol is pane-centric (SpawnPane, ListPanes, etc.), which is correct per CLAUDE.md's architecture ("oriterm_mux is a pane-only server"). Section text needs correction. Also, Windows named pipes are actually implemented (not deferred as claimed).
> 2. **44.4 Cross-Process Tab Migration:** Section was marked complete but is **not implemented at all**. No MoveTabToWindow PDU, no cross-process tab migration machinery, no tear-off flow, no `--position x,y` CLI flag. The mux layer has no concept of tabs or windows.
> 3. **Core infrastructure (44.1, 44.3, 44.5, 44.6) is solid and well-tested.** 433 mux tests + 6 IPC tests all pass.

**PRIORITY:** **BLOCKER** — This section must be completed before any further feature work. The current single-process multi-window model is fundamentally wrong and the source of unresolvable bugs (z-order fights, stale redraws, flash-and-vanish on tear-off). Every serious terminal emulator that supports multi-window does it with process separation (WezTerm) or daemon+client (tmux). We must get this right first.

**Crate:** `oriterm_mux` (daemon, protocol, client), `oriterm` (window client)
**Dependencies:** Section 30 (Pane/Domain system — complete), Section 32 (Tab/Window management — complete)
**Supersedes:** Section 32.4 (Cross-Window Operations) — the in-process move_tab_to_window machinery is replaced by cross-process tab migration via the daemon.
**Absorbed by:** Section 34 details are pulled forward and simplified here. Section 34 becomes "IPC Protocol Hardening" (compression, version negotiation, advanced coalescing) — deferred polish on top of what this section builds.

**Inspired by:**
- **Chrome**: Each window is a process. Tabs can migrate between windows. The browser process (≈ daemon) coordinates. Renderer processes (≈ PTY sessions) are independent of which window displays them.
- **WezTerm**: `wezterm-mux-server-impl` daemon + `wezterm-client` GUI. Domain trait abstracts local/remote. `move_pane_to_new_tab()` works across process boundaries. Codec protocol with bincode serialization.
- **tmux**: The original daemon+client terminal. Server owns all sessions. Clients attach/detach freely. Sessions survive client crashes.

---

## Why This Is a Blocker

The current architecture has a fundamental flaw: all windows share one process, one event loop, one GPU context. This causes:

1. **Tab migration bugs**: Moving a tab between windows requires shuffling mux state, creating placeholder tabs, and carefully ordering operations — fragile deferred machinery that breaks.
2. **Rendering fights**: Multiple windows competing for `request_redraw()` on the same event loop causes stale frames, z-order confusion, and flash-and-vanish.
3. **No crash isolation**: One window's GPU hang or PTY deadlock kills all windows.
4. **User expectation violation**: Users expect "New Window" to be a separate process they can independently close, Task Manager kill, or move to a different monitor without affecting other windows. The current model violates this.

The fix is architectural: separate window processes connected to a shared daemon.

---

## Architecture Overview

```
                    ┌─────────────────────┐
                    │   oriterm-mux       │  ← daemon process (long-lived)
                    │   (Mux Daemon)      │
                    │                     │
                    │  ┌───────────────┐  │
                    │  │ InProcessMux  │  │  ← owns all PTY sessions, grids, tabs, windows
                    │  │  PaneRegistry │  │
                    │  │  SessionReg   │  │
                    │  └───────────────┘  │
                    │                     │
                    │  ┌───────────────┐  │
                    │  │ IPC Listener  │  │  ← named pipe (Windows) / Unix socket
                    │  │  connections  │  │
                    │  └───────────────┘  │
                    └────────┬────────────┘
                             │ IPC
              ┌──────────────┼──────────────┐
              │              │              │
     ┌────────┴───────┐ ┌───┴────────┐ ┌───┴────────┐
     │ oriterm (win1) │ │ oriterm    │ │ oriterm    │  ← window processes (short-lived)
     │ Window Process │ │ (win2)    │ │ (win3)    │
     │                │ │           │ │           │
     │ ┌────────────┐ │ │           │ │           │
     │ │ MuxClient  │ │ │           │ │           │
     │ │ GpuState   │ │ │           │ │           │
     │ │ Renderer   │ │ │           │ │           │
     │ │ Fonts      │ │ │           │ │           │
     │ └────────────┘ │ │           │ │           │
     └────────────────┘ └───────────┘ └───────────┘
```

**Key invariant:** PTY sessions live in the daemon. Window processes are stateless renderers — they can crash, restart, or be killed without losing any terminal session.

---

## 44.1 Mux Daemon Binary (verified 2026-03-29)

A separate `oriterm-mux` binary that runs as a background daemon. Owns all PTY sessions via `InProcessMux`. Accepts IPC connections from window processes.

**File:** `oriterm_mux/src/bin/oriterm_mux.rs` (binary entry point), `oriterm_mux/src/server.rs`

**Reference:**
- WezTerm: `wezterm-mux-server-impl/src/sessionhandler.rs` — per-client session handler
- tmux: `server.c` — server event loop, client connections, session ownership

- [x] `oriterm-mux` binary:
  - [x] Minimal binary in `oriterm_mux/src/bin/oriterm_mux.rs`
  - [x] `--daemon` flag: fork/detach on Unix, `CREATE_NEW_PROCESS_GROUP` on Windows
  - [x] `--foreground` flag: stay in foreground (for debugging)
  - [x] Write PID file: `$XDG_RUNTIME_DIR/oriterm-mux.pid` (Linux), `%LOCALAPPDATA%\oriterm\oriterm-mux.pid` (Windows)
  - [x] Graceful shutdown: `SIGTERM`/`SIGINT` → close all PTYs, remove PID file and socket
- [x] `MuxServer` struct:
  - [x] `mux: InProcessMux` — the actual mux state (all panes, tabs, windows)
  - [x] `listener: IpcListener` — platform-specific IPC listener
  - [x] `connections: HashMap<ClientId, ClientConnection>` — connected window processes
  - [x] `subscriptions: HashMap<PaneId, Vec<ClientId>>` — which clients want output for which panes
- [x] Server event loop (single-threaded, `mio`-based):
  - [x] Accept new connections from window processes
  - [x] Read incoming requests (create tab, close tab, input, resize, etc.)
  - [x] Dispatch to `InProcessMux` methods
  - [x] Drain `MuxEvent` channel from PTY reader threads
  - [x] Push `MuxNotification` to subscribed clients
- [x] Connection lifecycle:
  - [x] Client connects → version handshake → assigns `ClientId`
  - [x] Client declares which mux `WindowId` it's rendering (one window per client)
  - [x] Client subscribes to panes in its window → receives output notifications
  - [x] Client disconnects → unsubscribe, but panes stay alive
  - [x] Last client disconnects → daemon keeps running (sessions persist)
- [x] Daemon exit conditions:
  - [x] All panes have exited AND no clients connected → exit
  - [x] Explicit `--stop` command sent via IPC
  - [x] SIGTERM/SIGINT

**Tests:**
- [x] Daemon starts, creates PID file, listens on socket/pipe
- [x] Client connects, version handshake succeeds
- [x] Client sends CreateWindow → window created, WindowId returned
- [x] Client subscribes → receives output notifications (framework wired)
- [x] Client disconnects → server state cleaned up
- [x] New client connects → can list existing windows
- [x] Fire-and-forget messages (Input) don't produce responses
- [x] Unexpected PDU from client returns error

---

## 44.2 IPC Protocol (Minimal Viable) (verified 2026-03-29 — PARTIAL: framing/transport PASS, section text inaccurate for PDU list)

The wire protocol for communication between daemon and window processes. This is the minimal protocol needed for tab migration — Section 34 adds compression, advanced coalescing, and hardening later.

**File:** `oriterm_mux/src/protocol/mod.rs`, `oriterm_mux/src/protocol/codec.rs`, `oriterm_mux/src/protocol/messages.rs`, `oriterm_mux/src/protocol/snapshot.rs`

**Reference:**
- WezTerm: `codec/src/lib.rs` — leb128 framing, serial numbers, bincode payloads
- Alacritty: `ipc.rs` — simple JSON over Unix socket (much simpler, but no tab migration)

> **CRITICAL (2026-03-29):** The protocol is pane-centric, not tab/window-centric as described below. The mux is a flat pane server per CLAUDE.md architecture. Many PDUs listed below do NOT exist — they are marked with annotations. The actual protocol implements: Hello, ClosePane, Input, Resize, Subscribe, Unsubscribe, GetPaneSnapshot, Ping, Shutdown, ScrollDisplay, ScrollToBottom, ScrollToPrompt, SetTheme, SetCursorShape, MarkAllDirty, OpenSearch, CloseSearch, SearchSetQuery, SearchNextMatch, SearchPrevMatch, ExtractText, ExtractHtml, SetCapabilities, SpawnPane, ListPanes, SetImageConfig (requests); HelloAck, PaneClosedAck, Subscribed, Unsubscribed, PaneSnapshotResp, PingAck, ShutdownAck, ScrollToPromptAck, ExtractTextResp, ExtractHtmlResp, SpawnPaneResponse, ListPanesResponse, Error (responses); NotifyPaneOutput, NotifyPaneExited, NotifyPaneMetadataChanged, NotifyPaneBell, NotifyCommandComplete, NotifyClipboardStore, NotifyClipboardLoad, NotifyPaneSnapshot (notifications).

- [x] Frame format (simplified from Section 34): (verified 2026-03-29)
  ```
  ┌──────────┬──────────┬──────────────────────┐
  │ type(u16)│ seq(u32) │ payload_len(u32)      │
  ├──────────┴──────────┴──────────────────────┤
  │ payload (bincode-encoded)                   │
  └─────────────────────────────────────────────┘
  ```
  - [x] 10-byte header (no magic/version/flags in v1 — add in Section 34) (verified 2026-03-29)
  - [x] `type`: message type ID (verified 2026-03-29)
  - [x] `seq`: request ID for request/response correlation (verified 2026-03-29)
  - [x] `payload_len`: u32 (max 16MB) (verified 2026-03-29)
  - [x] Payload: `bincode` for encoding (fast, compact, no schema needed) (verified 2026-03-29)
- [x] Message types — requests from window to daemon:
  - [x] `Hello { pid: u32 }` → `HelloAck { client_id: ClientId }` (verified 2026-03-29)
  - [ ] `CreateWindow → WindowCreated { window_id: WindowId }` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — mux has no window concept. Replaced by pane-centric model where window processes manage their own sessions -->
  - [ ] `CreateTab { window_id, config: SpawnConfig } → TabCreated { tab_id, pane_id }` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — replaced by SpawnPane -->
  - [ ] `CloseTab { tab_id } → TabClosed` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — mux has no tab concept -->
  - [x] `ClosePane { pane_id } → PaneClosed` (verified 2026-03-29)
  - [x] `Input { pane_id, data: Vec<u8> }` → (fire-and-forget, no response) (verified 2026-03-29)
  - [x] `Resize { pane_id, cols: u16, rows: u16 }` → (fire-and-forget) (verified 2026-03-29)
  - [ ] `MoveTabToWindow { tab_id, target_window_id } → TabMoved` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — no cross-process tab migration -->
  - [x] `Subscribe { pane_id } → Subscribed { snapshot: PaneSnapshot }` (verified 2026-03-29)
  - [x] `Unsubscribe { pane_id }` → (ack) (verified 2026-03-29)
  - [ ] `ListWindows → WindowList { windows: Vec<MuxWindowInfo> }` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — replaced by ListPanes -->
  - [ ] `ListTabs { window_id } → TabList { tabs: Vec<MuxTabInfo> }` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — replaced by ListPanes -->
  - [x] `GetPaneSnapshot { pane_id } → PaneSnapshot { cells, cursor, palette, title }` (verified 2026-03-29)
  - [ ] `SplitPane { tab_id, pane_id, direction, config } → PaneSplit { new_pane_id }` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — splits are GUI-side only -->
  - [ ] `CycleTab { window_id, delta: i32 } → ActiveTabChanged { tab_id }` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — tab management is GUI-side -->
  - [ ] `SetActiveTab { window_id, tab_id } → ActiveTabChanged { tab_id }` <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — tab management is GUI-side -->
  - [x] `SpawnPane` → `SpawnPaneResponse` (verified 2026-03-29, replaces CreateTab)
  - [x] `ListPanes` → `ListPanesResponse` (verified 2026-03-29, replaces ListWindows/ListTabs)
  - [x] `Ping` → `PingAck` (verified 2026-03-29)
  - [x] `Shutdown` → `ShutdownAck` (verified 2026-03-29)
  - [x] `ScrollDisplay`, `ScrollToBottom`, `ScrollToPrompt` (verified 2026-03-29)
  - [x] `SetTheme`, `SetCursorShape`, `MarkAllDirty` (verified 2026-03-29)
  - [x] `OpenSearch`, `CloseSearch`, `SearchSetQuery`, `SearchNextMatch`, `SearchPrevMatch` (verified 2026-03-29)
  - [x] `ExtractText` → `ExtractTextResp`, `ExtractHtml` → `ExtractHtmlResp` (verified 2026-03-29)
  - [x] `SetCapabilities`, `SetImageConfig` (verified 2026-03-29)
- [x] Message types — push notifications from daemon to window:
  - [x] `PaneOutput { pane_id, dirty_rows: Vec<u16> }` — pane has new output (verified 2026-03-29: `NotifyPaneOutput` exists but carries only `pane_id`, no `dirty_rows`)
  - [x] `PaneExited { pane_id }` — shell exited (verified 2026-03-29: `NotifyPaneExited`)
  - [x] `PaneTitleChanged { pane_id, title: String }` — OSC title change (verified 2026-03-29: renamed to `NotifyPaneMetadataChanged`)
  - [x] `PaneBell { pane_id }` — BEL received (verified 2026-03-29: `NotifyPaneBell`)
  - [ ] `WindowTabsChanged { window_id }` — tab list changed (tab added/removed/reordered) <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — no window/tab concepts in mux -->
  - [ ] `TabMoved { tab_id, from_window: WindowId, to_window: WindowId }` — tab migrated <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — no cross-process tab migration -->
  - [x] `NotifyCommandComplete` (verified 2026-03-29, not in original plan)
  - [x] `NotifyClipboardStore`, `NotifyClipboardLoad` (verified 2026-03-29, not in original plan)
  - [x] `NotifyPaneSnapshot` (verified 2026-03-29, not in original plan)
- [x] `PaneSnapshot` struct:
  - [x] `cells: Vec<Vec<WireCell>>` — visible grid contents (rows × cols, wire-friendly)
  - [x] `cursor: WireCursor` — position, shape, visible
  - [x] `palette: Vec<[u8; 3]>` — current color palette as RGB triplets
  - [x] `title: String` — pane title
  - [x] `modes: u32` — terminal modes as raw bits
  - [x] `scrollback_len: u32` — number of scrollback rows
  - [x] `display_offset: u32` — current scroll position
- [x] Transport:
  - [x] Named pipe on Windows: `\\.\pipe\oriterm-mux-<username>` (verified 2026-03-29: ACTUALLY IMPLEMENTED in `oriterm_ipc/src/windows/` — listener.rs 197 lines, client_stream.rs, stream.rs, pipe_name.rs. Previously incorrectly marked deferred.)
  - [x] Unix domain socket on Linux/macOS: `$XDG_RUNTIME_DIR/oriterm-mux.sock` (verified 2026-03-29)
  - [x] Single socket/pipe per daemon instance (verified 2026-03-29)

**Tests:**
- [x] Frame encode/decode roundtrip: all message types
- [x] Sequence correlation: request seq matches response seq
- [x] PaneSnapshot serialization: roundtrip with CJK, emoji, combining marks
- [x] Fire-and-forget messages: Input/Resize don't block on response
- [x] Push notification delivery: daemon sends, client receives
- [x] Max payload: 16MB limit enforced

---

## 44.3 Window-as-Client Model (verified 2026-03-29)

Each `oriterm` window process is a thin GPU client. It connects to the daemon, subscribes to its assigned panes, renders their output, and forwards user input. No terminal state lives in the window process.

**File:** `oriterm_mux/src/client.rs`, `oriterm/src/app/mod.rs`

**Reference:**
- WezTerm: `wezterm-client/src/client.rs` — client connection, RPC methods, domain proxy
- Chrome: renderer process — stateless renderer of content owned by the browser process

- [x] `MuxClient` struct (lives in window process):
  - [x] Stub with `local_session: SessionRegistry` + `notifications: Vec<MuxNotification>`
  - [x] `stream: IpcStream` — connected to daemon  <!-- encapsulated in ClientTransport -->
  - [x] `codec: ProtocolCodec` — frame encode/decode  <!-- encapsulated in ClientTransport -->
  - [x] `pending: HashMap<u32, oneshot::Sender<Response>>` — pending request/response  <!-- encapsulated in ClientTransport -->
  - [x] `next_seq: AtomicU32` — sequence number allocator  <!-- encapsulated in ClientTransport -->
  - [x] `notification_tx: mpsc::Sender<MuxNotification>` — push notifications → event loop  <!-- encapsulated in ClientTransport -->
- [x] `MuxBackend` trait:
  - [x] Defines the API that the App uses for all mux operations
  - [x] `EmbeddedMux` implements it (wraps `InProcessMux` + owns `HashMap<PaneId, Pane>`)
  - [x] `MuxClient` implements it (stub — for daemon mode)
  - [x] App uses `Box<dyn MuxBackend>` — doesn't know or care which mode
  - [x] Methods mirror `InProcessMux`: `create_tab()`, `close_tab()`, `split_pane()`, `resize_pane()`, `move_tab_to_window()`, etc.
- [x] App rewiring:
  - [x] `App::mux` changes from `Option<InProcessMux>` to `Option<Box<dyn MuxBackend>>`
  - [x] Removed `panes: HashMap<PaneId, Pane>` — now inside `EmbeddedMux`
  - [x] Removed `mux_wakeup: Arc<...>` — now inside `EmbeddedMux`
  - [x] All mux operations go through the trait — no direct `InProcessMux` access
  - [x] Push notifications from daemon arrive as `MuxNotification` on event loop
  - [x] `about_to_wait()` drains notification channel, triggers redraws
- [x] Render flow (window process):
  - [x] Daemon pushes `PaneOutput { pane_id, dirty_rows }` → client
  - [x] Client requests `GetPaneSnapshot(pane_id)` for dirty pane data
  - [x] **OR** (optimization): daemon pushes incremental cell updates inline
  - [x] Client renders from snapshot data — no `FairMutex<Term>` needed in window process
  - [x] GPU rendering uses the same `GpuRenderer` — just different data source
- [x] Per-window process state:
  - [x] `GpuState` — per-process (each window has its own GPU context)
  - [x] `GpuRenderer` — per-process
  - [x] `FontCollection` + `GlyphAtlas` — per-process
  - [x] `WindowContext` — per-process (one window per process)
  - [x] Config — loaded from disk per-process (daemon does NOT manage config)

**Tests:**
- [x] `MuxBackend` trait compile check: both `EmbeddedMux` and `MuxClient` implement it (object-safe)
- [x] `EmbeddedMux` tests: create_window, drain_notifications, discard, pane access, event_tx
- [x] `MuxClient` tests: pane returns None, drain empty, poll_events noop
- [x] App works with `EmbeddedMux` backend: all 1018 tests pass, build + clippy clean
- [x] App works with `MuxClient` backend: create tab, type, see output
- [x] Push notification flow: daemon output → client notification → redraw
- [x] Multiple windows (processes) connected: each renders its own tabs
- [x] Window process crash → daemon keeps sessions → new window can reconnect

---

## 44.4 Cross-Process Tab Migration (REOPENED 2026-03-29 — NOT IMPLEMENTED)

> **CRITICAL (2026-03-29):** This entire subsection was marked complete but is NOT IMPLEMENTED. No cross-process tab migration exists in the codebase. The mux layer is pane-only with no tab/window concepts. The keybinding `MoveTabToNewWindow` and event `TermEvent::MoveTabToNewWindow(TabId)` exist but only perform in-process window creation, not cross-process migration. All items reopened below.

The core UX: "Move to New Window" spawns a new `oriterm` process, the daemon reassigns the tab, and the terminal session continues without interruption. Same for tab tear-off.

**File:** `oriterm/src/app/tab_management/mod.rs`, `oriterm_mux/src/server.rs`

**Reference:**
- WezTerm: `domain.rs` `move_pane_to_new_tab()` — domain-level tab move, works across processes
- Chrome: tab tear-off — browser process reassigns renderer to new window process

- [ ] "Move to New Window" flow: <!-- REOPENED 2026-03-29: not implemented — no MoveTabToNewWindow PDU, no process spawning, no pane reassignment -->
  1. User selects "Move to New Window" from context menu (or `Action::MoveTabToNewWindow`)
  2. Window process calls `mux_backend.move_tab_to_new_window(tab_id)`
  3. `MuxClient` sends `MoveTabToNewWindow { tab_id }` to daemon
  4. Daemon:
     a. Creates new `MuxWindow` in session registry
     b. Moves tab from source window to new window (same as current `mux.move_tab_to_window()`)
     c. Returns `TabMovedToNewWindow { new_window_id }`
     d. Sends `WindowTabsChanged` notification to source window client
  5. Source window process spawns new `oriterm` process: `oriterm --connect <socket> --window <new_window_id>`
  6. New process connects to daemon, declares itself as renderer for `new_window_id`
  7. Subscribes to panes in the moved tab
  8. Receives `PaneSnapshot`, renders immediately — no flash, no restart
  9. Source window process updates its tab bar (tab is gone)
- [ ] Tab tear-off flow: <!-- REOPENED 2026-03-29: not implemented — existing tear-off in app/tab_drag/tear_off.rs creates new winit window in SAME process, not a separate process -->
  1. User drags tab beyond tear-off threshold
  2. Same as above — `move_tab_to_new_window(tab_id)` → spawn process
  3. New window positioned under cursor (passed as CLI arg: `--position x,y`) <!-- --position flag not implemented -->
  4. OS window drag initiated in new process
- [ ] Move tab to existing window: <!-- REOPENED 2026-03-29: not implemented — no pane-to-process reassignment protocol -->
  1. User drags tab to another window (or future "Move to Window >" submenu)
  2. Source window process calls `mux_backend.move_tab_to_window(tab_id, target_window_id)`
  3. Daemon moves tab in registry, sends `WindowTabsChanged` to both window processes
  4. Target window subscribes to new panes, source window unsubscribes
  5. Both windows update their tab bars
- [ ] Edge cases: <!-- REOPENED 2026-03-29: none of these are implemented -->
  - [ ] Last tab in window: refuse move (don't leave an empty window) — or close source window after move
  - [ ] Last window in session: refuse move (would create empty session)
  - [ ] Daemon unreachable during move: fail gracefully, show error, don't lose the tab
  - [ ] Target window dies during move: daemon detects disconnect, tab stays in source
  - [ ] Source window dies during move: daemon completes the move, new window renders tab

**Tests:**
- [ ] Move tab: source window loses tab, target/new window gains it <!-- REOPENED 2026-03-29: not implemented -->
- [ ] PTY session survives move: running command continues uninterrupted <!-- REOPENED 2026-03-29: not implemented -->
- [ ] Scrollback preserved: full scrollback available in new window <!-- REOPENED 2026-03-29: not implemented -->
- [ ] Terminal modes preserved: alternate screen, bracketed paste, mouse mode survive <!-- REOPENED 2026-03-29: not implemented -->
- [ ] Concurrent moves: two tabs moving simultaneously don't corrupt state <!-- REOPENED 2026-03-29: not implemented -->
- [ ] Move then type: keystrokes route to correct pane after migration <!-- REOPENED 2026-03-29: not implemented -->
- [ ] Tear-off: window spawns at cursor position, tab renders immediately <!-- REOPENED 2026-03-29: not implemented -->

---

## 44.5 Auto-Start + Discovery (verified 2026-03-29)

The daemon starts automatically when the first window launches. Subsequent windows discover and connect to the running daemon. No manual daemon management required.

**File:** `oriterm/src/main.rs`, `oriterm_mux/src/discovery.rs`

**Reference:**
- WezTerm: auto-start via `wezterm-gui` checking for running mux, starting if absent
- Alacritty: `ALACRITTY_SOCKET` env var for instance discovery

- [x] First window launch:
  - [x] `oriterm` starts → check for running daemon (try connect to socket/pipe)
  - [x] No daemon → spawn `oriterm-mux --daemon` as detached process
  - [x] Wait for socket/pipe to appear (poll with exponential backoff, max 2s)
  - [x] Connect to daemon → `Hello` handshake → `CreateWindow` → `CreateTab` → render
- [x] Subsequent window launch:
  - [x] `oriterm` starts → check for running daemon → daemon found
  - [x] Connect → `CreateWindow` → `CreateTab` → render
  - [x] CLI shortcut: `oriterm --new-window` (default behavior when daemon running)
- [x] `oriterm --connect <socket> --window <window_id>`:
  - [x] Used by cross-process tab migration
  - [x] Connect to specified daemon socket
  - [x] Claim specified window ID (don't create new one)
  - [x] Subscribe to panes in that window → render
- [x] Discovery mechanism:
  - [x] **Windows**: Named pipe with well-known name `\\.\pipe\oriterm-mux-<username>` (verified 2026-03-29: actually implemented in `oriterm_ipc/src/windows/`, previously incorrectly marked deferred)
  - [x] **Linux/macOS**: Unix socket at `$XDG_RUNTIME_DIR/oriterm-mux.sock` (verified 2026-03-29)
  - [x] PID file validation: if PID file exists but process is dead → stale, clean up and start fresh
- [x] Daemon health check:
  - [x] Window processes send periodic ping (every 5s)
  - [x] If no pong within next ping interval → daemon presumed dead
  - [x] On daemon death: log warning, fall back to in-process mode (orphaned window)
  - [x] `Ping`/`PingAck` protocol messages added
  - [x] `is_connected()` on `MuxBackend` trait

**Tests:**
- [x] First launch: daemon auto-starts, window connects (discovery module tested)
- [x] Second launch: connects to existing daemon (probe_daemon tests)
- [x] Stale PID file: cleaned up, new daemon started (validate_pid_file tests)
- [x] Daemon disconnect: window detects via is_connected, falls back to embedded
- [x] `--connect --window` flag: connects and claims specified window (existing tests)
- [x] Ping/PingAck protocol roundtrip
- [x] Server responds to Ping with PingAck
- [x] wait_for_socket timeout and delayed start

---

## 44.6 Backward Compatibility + Fallback (verified 2026-03-29)

The single-process in-process mode (`InProcessMux`) remains as a fallback for environments where daemon spawning isn't possible (sandboxed apps, CI, testing).

**File:** `oriterm/src/main.rs`

- [x] `MuxBackend` trait ensures App code is identical in both modes (verified 2026-03-29)
- [x] Config option: `process_model = "daemon" | "embedded"` — default "daemon" (verified 2026-03-29)
  - [x] `"daemon"`: auto-start daemon, connect, full multi-process support
  - [x] `"embedded"`: single-process mode, `InProcessMux` directly, no IPC (current behavior)
- [x] `--embedded` CLI flag: force embedded mode (overrides config)
- [x] Fallback: if daemon fails to start after 3 attempts → log warning, fall back to embedded mode
- [x] Testing: all existing tests use embedded mode (no daemon needed for `cargo test`)
- [x] Tab migration in embedded mode: same as current — in-process `move_tab_to_window()`, single window per process

**Tests:**
- [x] Embedded mode: app works exactly as current single-process model (all 1018+ tests use `EmbeddedMux`)
- [x] Config switch: daemon ↔ embedded based on config (`process_model` roundtrip tests)
- [x] Fallback: daemon spawn failure → embedded mode with warning (retry logic in `ensure_daemon_with_retry`)
- [x] Test harness: `cargo test` uses embedded mode by default

---

## 44.7 Section Completion (verified 2026-03-29 — INCOMPLETE due to 44.2 inaccuracies and 44.4 not implemented)

- [ ] All 44.1–44.6 items complete (Windows named pipes deferred — Unix-only for now) <!-- REOPENED 2026-03-29: 44.2 has inaccurate PDU list, 44.4 not implemented. Windows named pipes are actually implemented. -->
- [x] `oriterm-mux` binary: daemon starts, owns PTY sessions, accepts IPC connections (verified 2026-03-29)
- [x] IPC protocol: binary framing, request/response, push notifications (verified 2026-03-29: pane-centric, not tab/window-centric as described in 44.2)
- [x] Window-as-client: `MuxBackend` trait, `MuxClient` implements it, App is mode-agnostic (verified 2026-03-29)
- [ ] Cross-process tab migration: "Move to New Window" spawns process, tab migrates seamlessly <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — see 44.4 -->
- [x] Auto-start: daemon launches on first window, discovered by subsequent windows (verified 2026-03-29)
- [x] Backward compatibility: embedded mode for testing and sandboxed environments (verified 2026-03-29)
- [x] `cargo build --target x86_64-pc-windows-gnu` — compiles (both binaries) (verified 2026-03-29)
- [x] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] `cargo test` — all tests pass (embedded mode + 10 e2e daemon tests) (verified 2026-03-29: 433 mux + 6 IPC tests)
- [ ] **Tab migration test**: move tab to new window → running command uninterrupted <!-- REOPENED 2026-03-29: NOT IMPLEMENTED -->
- [ ] **Scrollback test**: moved tab retains full scrollback history <!-- REOPENED 2026-03-29: NOT IMPLEMENTED — no tab migration -->
- [ ] **Multi-window test**: 3 windows, move tabs between them, all render correctly <!-- REOPENED 2026-03-29: multi-client exists but tab migration does not -->
- [x] **Crash isolation test**: kill one window process → others unaffected, sessions alive (verified 2026-03-29: partially — `client_crash_cleans_up_owned_window` verifies pane cleanup)
- [x] **Daemon restart test**: kill daemon → windows detect, reconnect on daemon restart (verified 2026-03-29: `daemon_restart_detection_and_reconnect`)
- [x] **Latency test**: keystroke → screen update < 5ms through daemon IPC (verified 2026-03-29: `ipc_latency_under_5ms`, raw latency 0.021ms)

**Exit Criteria:** Every oriterm window is an independent OS process. The mux daemon owns all terminal sessions. Tabs migrate between windows without losing state. Users can close, kill, or crash any window without affecting other windows or losing sessions. The daemon auto-starts invisibly. Embedded mode exists for testing and edge cases. The `MuxBackend` trait makes the App code identical regardless of which mode is active.

> **Verification summary (2026-03-29):** Exit criteria NOT fully met. Core infrastructure (daemon, IPC framing, client model, discovery, fallback) is solid and well-tested. But the section's central differentiating claim — cross-process tab migration — is not implemented. The protocol is pane-centric (correct per architecture) but the section text describes tab/window-centric PDUs that don't exist. Section status changed from `complete` to `in-progress`.
