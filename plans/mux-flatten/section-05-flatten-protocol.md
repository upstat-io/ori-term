---
section: "05"
title: "Flatten Protocol & Server"
status: not-started
goal: "Wire protocol and daemon server operate on panes only — no tab/window concepts"
depends_on: ["03"]
sections:
  - id: "05.1"
    title: "Strip Protocol Messages"
    status: not-started
  - id: "05.2"
    title: "Strip Server Dispatch"
    status: not-started
  - id: "05.3"
    title: "Strip Server State"
    status: not-started
  - id: "05.4"
    title: "Simplify MuxBackend Trait"
    status: not-started
  - id: "05.5"
    title: "Update Backends"
    status: not-started
  - id: "05.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Flatten Protocol & Server

**Status:** Not Started
**Goal:** The wire protocol, server dispatch, and `MuxBackend` trait
operate exclusively on panes. No tab/window messages, no session state
in the daemon.

**Context:** The daemon currently tracks windows, tabs, and client-window
ownership. After flattening, it's a pane process supervisor. Clients
connect, spawn panes, subscribe to pane events, and manage their own
session state.

**Depends on:** Section 03 (mux core flattened).

---

## 05.1 Strip Protocol Messages

**WARNING: `messages.rs` is 761 lines (over 500-line limit).** After
removing tab/window message types, verify it drops below 500 lines.
If not, split into `messages/requests.rs`, `messages/responses.rs`,
`messages/push.rs` submodules.

**File(s):** `oriterm_mux/src/protocol/messages.rs`,
`oriterm_mux/src/protocol/mod.rs`

- [ ] Remove all tab/window message types from `MuxPdu`:
  - Requests: `CreateWindow`, `CreateTab`, `CloseTab`, `CloseWindow`,
    `MoveTabToWindow`, `SetActiveTab`, `CycleTab`, `ClaimWindow`,
    `ListWindows`, `ListTabs`, `SplitPane`, `SpawnFloatingPane`
  - Responses: `WindowCreated`, `TabCreated`, `TabClosed`, `TabMovedAck`,
    `WindowList`, `TabList`, `PaneSplit`, `ActiveTabChanged`,
    `WindowClosed` (response variant), `WindowClaimed`, `FloatingPaneSpawned`
  - Push: `NotifyWindowTabsChanged`, `NotifyTabMoved`, `NotifyTabLayoutChanged`
- [ ] Keep pane-centric messages (actual variant names from `messages.rs`):
  - `Hello` / `HelloAck` — client handshake
  - `ClosePane { pane_id }` / `PaneClosedAck`
  - `Input { pane_id, data }` — fire-and-forget PTY write
  - `Resize { pane_id, cols, rows }` — fire-and-forget PTY resize
  - `Subscribe { pane_id }` / `Subscribed { snapshot }` / `Unsubscribe` / `Unsubscribed`
  - `GetPaneSnapshot { pane_id }` / `PaneSnapshotResp { snapshot }`
  - `ScrollDisplay`, `ScrollToBottom`, `ScrollToPrompt` / `ScrollToPromptAck`
  - `SetTheme`, `SetCursorShape`, `MarkAllDirty` — fire-and-forget
  - `OpenSearch`, `CloseSearch`, `SearchSetQuery`, `SearchNextMatch`, `SearchPrevMatch`
  - `ExtractText` / `ExtractTextResp`, `ExtractHtml` / `ExtractHtmlResp`
  - `SetCapabilities { flags }` — fire-and-forget
  - `Ping` / `PingAck`, `Shutdown` / `ShutdownAck`
  - Push: `NotifyPaneOutput`, `NotifyPaneExited`, `NotifyPaneTitleChanged`,
    `NotifyPaneBell`, `NotifyPaneSnapshot`
  - **New:** `SpawnPane { config }` / `SpawnPaneResponse { pane_id }` (replaces `CreateTab`)
  - **New:** `ListPanes` / `ListPanesResponse { pane_ids }` (replaces `ListWindows`/`ListTabs`)
- [ ] Remove `MuxTabInfo`, `MuxWindowInfo` from `protocol/snapshot.rs`
- [ ] Remove tab/window imports from `protocol/snapshot.rs`:
      `use crate::id::{TabId, WindowId}`, `use crate::layout::*`
- [ ] Update `MsgType` discriminants (remove tab/window entries)
- [ ] Update codec serialization/deserialization
- [ ] Update `protocol/tests.rs`:
  - Remove all roundtrip tests for deleted PDU variants (CreateWindow, CreateTab,
    CloseTab, CloseWindow, MoveTabToWindow, ListWindows, ListTabs, SplitPane,
    CycleTab, SetActiveTab, ClaimWindow, SpawnFloatingPane and their responses)
  - Remove `MuxTabInfo`, `MuxWindowInfo` test data
  - Remove `TabId`, `WindowId` imports
  - Add roundtrip tests for new `SpawnPane` / `SpawnPaneResponse` and
    `ListPanes` / `ListPanesResponse` variants

---

## 05.2 Strip Server Dispatch

**File(s):** `oriterm_mux/src/server/dispatch/mod.rs`,
`oriterm_mux/src/server/dispatch/helpers.rs`,
`oriterm_mux/src/server/dispatch/types.rs`

- [ ] Remove dispatch arms for all tab/window messages:
  - `CreateWindow`, `CreateTab`, `CloseTab`, `CloseWindow`
  - `MoveTabToWindow`, `CycleTab`, `SetActiveTab`
  - `ClaimWindow`, `ListWindows`, `ListTabs`
  - `SplitPane`, `SpawnFloatingPane`
- [ ] Keep dispatch arms for pane messages only:
  - `Hello`, `ClosePane`, `Input`, `Resize`
  - `Subscribe`, `Unsubscribe`, `GetPaneSnapshot`
  - `ScrollDisplay`, `ScrollToBottom`, `ScrollToPrompt`
  - `SetTheme`, `SetCursorShape`, `MarkAllDirty`
  - `OpenSearch`, `CloseSearch`, `SearchSetQuery`, `SearchNextMatch`, `SearchPrevMatch`
  - `ExtractText`, `ExtractHtml`
  - `SetCapabilities`, `Ping`, `Shutdown`
  - **New**: `SpawnPane` (replaces `CreateTab`)
  - **New**: `ListPanes` (replaces `ListWindows`/`ListTabs`)
- [ ] Simplify `DispatchResult` in `types.rs`:
  - Remove `claimed_window: Option<WindowId>` field
  - Remove `closed_window: Option<WindowId>` field
  - Keep: `response`, `sub_changed`, `unsubscribed_pane`
- [ ] Simplify `DispatchContext` in `types.rs`:
  - No changes needed (already pane-scoped: `mux`, `panes`, `wakeup`,
    `closed_panes`, `snapshot_cache`, `immediate_push`)
- [ ] Remove `conn.track_created_window()` call from `CreateWindow` dispatch arm
- [ ] Remove `conn.add_window_id()` call from `ClaimWindow` dispatch arm
- [ ] Remove `ListWindows` dispatch arm that calls `snapshot::build_window_list()`
- [ ] Remove `ListTabs` dispatch arm that calls `snapshot::build_tab_list()`
- [ ] In `dispatch_request()` pre-match: remove `claim_wid` and `close_wid` extraction

---

## 05.3 Strip Server State

**File(s):** `oriterm_mux/src/server/mod.rs`,
`oriterm_mux/src/server/clients.rs`,
`oriterm_mux/src/server/connection.rs`,
`oriterm_mux/src/server/notify/mod.rs`,
`oriterm_mux/src/server/snapshot.rs`,
`oriterm_mux/src/server/push/mod.rs`

### 05.3.1 Strip `MuxServer` struct fields

- [ ] Remove `window_to_client: HashMap<WindowId, ClientId>` field from `MuxServer`
- [ ] Remove `window_to_client` initialization from `MuxServer::with_paths()`
- [ ] Remove `use crate::WindowId` import from `server/mod.rs`
- [ ] Simplify subscriptions: `HashMap<PaneId, Vec<ClientId>>` stays
      (clients subscribe to specific panes)

### 05.3.2 Rewrite `should_exit()`

- [ ] Rewrite `MuxServer::should_exit()`: change
      `self.connections.is_empty() && self.mux.session().window_count() == 0`
      to `self.connections.is_empty() && self.panes.is_empty()`
      (exit when no clients AND no live panes)

### 05.3.3 Strip `ClientConnection`

- [ ] Remove `window_ids: HashSet<WindowId>` field from `ClientConnection`
- [ ] Remove `created_windows: Vec<WindowId>` field from `ClientConnection`
- [ ] Remove methods: `window_ids()`, `add_window_id()`, `remove_window_id()`,
      `track_created_window()`, `created_windows()`
- [ ] Remove `use crate::WindowId` import from `connection.rs`
- [ ] Keep: `id()`, `stream_mut()`, `token()`, `frame_reader_mut()`, `queue_frame()`,
      `flush_writes()`, `has_pending_writes()`, `subscribe()`, `unsubscribe()`,
      `is_subscribed()`, `subscribed_panes()`, `set_capabilities()`, `has_capability()`,
      `pending_write_bytes()`

### 05.3.4 Rewrite `disconnect_client()`

- [ ] Rewrite `disconnect_client()` in `server/clients.rs`:
  - Remove the `windows_to_close` loop that iterates `conn.window_ids()` +
    `conn.created_windows()` and calls `self.mux.close_window()`
  - Remove `self.window_to_client.remove(wid)` calls
  - New behavior: for each pane the client was subscribed to, check if any
    other client is still subscribed. If not, close the pane via
    `self.mux.close_pane(pid)` and drop the `Pane` on a background thread
  - Keep: mio deregistration, token cleanup, subscription cleanup, pending_push cleanup
- [ ] Remove `use crate::WindowId` import from `clients.rs`

### 05.3.5 Rewrite `handle_decoded_frame()` post-dispatch

- [ ] Remove the `claimed_window` block:
      `if let Some(wid) = result.claimed_window { self.window_to_client.insert(wid, client_id); }`
- [ ] Remove the `closed_window` block:
      `if let Some(wid) = result.closed_window { self.window_to_client.remove(&wid); ... }`

### 05.3.6 Rewrite `drain_mux_events()` notification routing

- [ ] Remove the `TargetClients::WindowClient` match arm from `drain_mux_events()`
- [ ] Remove `self.mux.session()` argument from `notification_to_pdu()` call
      (the function signature changes — see 05.3.7)
- [ ] Verify `MuxNotification::PaneOutput` references are correct
      (renamed from `PaneDirty` in section 03.2, already complete by this point)

### 05.3.7 Strip `server/notify/mod.rs`

- [ ] Remove `TargetClients::WindowClient(WindowId)` variant from `TargetClients` enum
- [ ] Remove `session: &SessionRegistry` parameter from `notification_to_pdu()` signature
- [ ] Remove `WindowTabsChanged` match arm
- [ ] Remove `TabLayoutChanged` / `FloatingPaneChanged` match arm
- [ ] Remove `WindowClosed` / `LastWindowClosed` match arms (these return `None`
      currently but the variants no longer exist after section 03.2)
- [ ] Delete `tab_layout_changed_pdu()` helper function entirely
- [ ] Verify `MuxNotification::PaneOutput` references are correct (renamed in 03.2)
- [ ] Verify `MuxNotification::PaneBell` references are correct (renamed in 03.2)
- [ ] Remove imports: `use crate::{SessionRegistry, WindowId}` and
      `use crate::registry::SessionRegistry`

### 05.3.8 Delete snapshot functions

- [ ] Delete `build_window_list()` function from `server/snapshot.rs`
- [ ] Delete `build_tab_list()` function from `server/snapshot.rs`
- [ ] Remove imports: `MuxTabInfo`, `MuxWindowInfo`, `SessionRegistry`, `WindowId`
      from `server/snapshot.rs`
- [ ] Keep: `SnapshotCache`, `build_snapshot()`, `build_snapshot_into()` and all
      pane snapshot infrastructure

### 05.3.9 Update server tests

- [ ] Update `server/tests.rs` — remove tests that exercise window/tab dispatch
- [ ] Update `server/notify/tests.rs` — remove tests for `WindowTabsChanged`,
      `TabLayoutChanged` routing
- [ ] Verify remaining tests cover pane subscribe/unsubscribe, pane snapshot,
      pane close, client disconnect cleanup

---

## 05.4 Simplify MuxBackend Trait

**File(s):** `oriterm_mux/src/backend/mod.rs`

- [ ] Remove all tab/window/layout methods from `MuxBackend`:
  - Session: `session() -> &SessionRegistry`, `active_tab_id()`, `set_active_pane()`
  - Window: `create_window()`, `close_window()`, `claim_window()`, `refresh_window_tabs()`
  - Tab: `create_tab()`, `close_tab()`, `switch_active_tab()`, `cycle_active_tab()`,
    `reorder_tab()`, `move_tab_to_window()`, `move_tab_to_window_at()`
  - Split/layout: `split_pane()`, `toggle_zoom()`, `unzoom_silent()`, `equalize_panes()`,
    `set_divider_ratio()`, `resize_pane()` (tab-scoped layout resize), `undo_split()`, `redo_split()`
  - Floating: `spawn_floating_pane()`, `move_pane_to_floating()`, `move_pane_to_tiled()`,
    `move_floating_pane()`, `resize_floating_pane()`, `set_floating_pane_rect()`, `raise_floating_pane()`
- [ ] Keep pane methods (actual method names from trait):
  - `poll_events()`, `drain_notifications()`, `discard_notifications()`
  - `get_pane_entry(pane_id) -> Option<PaneEntry>`
  - `is_last_pane(pane_id) -> bool`
  - `close_pane(pane_id) -> ClosePaneResult` (simplified — no tab/window cascade)
  - `resize_pane_grid(pane_id, rows, cols)` — PTY resize
  - `pane_mode(pane_id) -> Option<u32>`
  - `set_pane_theme()`, `set_cursor_shape()`, `mark_all_dirty()`
  - `scroll_display()`, `scroll_to_bottom()`, `scroll_to_previous_prompt()`, `scroll_to_next_prompt()`
  - `open_search()`, `close_search()`, `search_set_query()`, `search_next_match()`, `search_prev_match()`, `is_search_active()`
  - `extract_text()`, `extract_html()`
  - `send_input(pane_id, data)` — PTY write
  - `set_bell()`, `clear_bell()`, `cleanup_closed_pane()`
  - `select_command_output()`, `select_command_input()`
  - `pane_ids() -> Vec<PaneId>`, `pane_cwd()`
  - `event_tx()`, `default_domain()`
  - `is_connected()`, `is_daemon_mode()`
  - `swap_renderable_content()`, `pane_snapshot()`, `is_pane_snapshot_dirty()`, `refresh_pane_snapshot()`, `clear_pane_snapshot_dirty()`
  - **New:** `spawn_pane(config, wakeup) -> Result<(PaneId, ...)>` (replaces `create_tab`)

---

## 05.5 Update Backends

**File(s):** `oriterm_mux/src/backend/embedded/mod.rs`,
`oriterm_mux/src/backend/client/mod.rs`,
`oriterm_mux/src/backend/client/rpc_methods.rs`,
`oriterm_mux/src/backend/client/notification.rs`

### 05.5.1 EmbeddedMux

**NOTE: `embedded/mod.rs` is 499 lines (at the 500-line limit).** Removing
tab/window methods should bring it under. Verify after changes.

- [ ] After section 03 flattens `InProcessMux`, the delegated
      tab/window methods (create_tab, close_tab, switch_active_tab, etc.)
      no longer exist — remove those trait method implementations.
      `snapshot_dirty`, `snapshot_cache`, and `renderable_cache` are pane render
      state and stay.
- [ ] Add `spawn_pane()` implementation delegating to `self.mux.spawn_pane()`

### 05.5.2 MuxClient struct fields

- [ ] Remove `local_session: SessionRegistry` mirror field
- [ ] Remove `pane_registry: PaneRegistry` field (MuxClient mirrored
      the server's registry locally for tab-scoped lookups; no longer needed
      because session state is GUI-owned and pane lookups go through RPC)
- [ ] `dirty_panes: HashSet<PaneId>` stays (pane render state)
- [ ] Remove imports from `client/mod.rs`: `SessionRegistry`, `PaneRegistry`,
      `MuxTab`, `MuxWindow`, `TabId`, `WindowId`

### 05.5.3 MuxClient RPC methods (`rpc_methods.rs`)

**WARNING: `rpc_methods.rs` is 825 lines (over 500-line limit).** After
removing tab/window methods, verify it drops below 500. If not, split
into submodules by concern.
- [ ] Remove `session() -> &SessionRegistry` — returns `&self.local_session`
- [ ] Remove `active_tab_id()` — reads from `local_session`
- [ ] Remove `set_active_pane()` — mutates `local_session`
- [ ] Remove `create_window()` — sends `CreateWindow` PDU
- [ ] Remove `close_window()` — sends `CloseWindow` PDU
- [ ] Remove `claim_window()` — sends `ClaimWindow` PDU
- [ ] Remove `refresh_window_tabs()` — sends `ListTabs` PDU, rebuilds
      `MuxTab`/`MuxWindow` from response
- [ ] Remove `create_tab()` — sends `CreateTab` PDU, creates local `MuxTab`,
      registers `PaneEntry { tab: tab_id }`
- [ ] Remove `close_tab()` — sends `CloseTab` PDU
- [ ] Remove `switch_active_tab()` / `cycle_active_tab()` — sends `SetActiveTab`/`CycleTab`
- [ ] Remove `reorder_tab()` / `move_tab_to_window()` / `move_tab_to_window_at()`
- [ ] Remove `split_pane()` — sends `SplitPane` PDU
- [ ] Remove `toggle_zoom()` / `unzoom_silent()` / `equalize_panes()`
- [ ] Remove `set_divider_ratio()` / `resize_pane()` (tab-scoped layout resize)
- [ ] Remove `undo_split()` / `redo_split()`
- [ ] Remove all floating pane RPC methods: `spawn_floating_pane()`,
      `move_pane_to_floating()`, `move_pane_to_tiled()`, etc.
- [ ] Add `spawn_pane()` — sends new `SpawnPane` PDU, returns `(PaneId, Pane)`
- [ ] Add `list_panes()` — sends new `ListPanes` PDU (if kept)
- [ ] Remove imports: `MuxTab`, `MuxWindow`, `SessionRegistry`, `PaneEntry`,
      `TabId`, `WindowId`, `SplitTree`, `FloatingLayer`, `SplitDirection`

### 05.5.4 MuxClient notification handling (`notification.rs`)

- [ ] Remove `NotifyWindowTabsChanged` mapping (PDU deleted in 05.1)
- [ ] Remove `NotifyTabLayoutChanged` mapping and `apply_layout_update()` call
      (PDU deleted in 05.1, method deleted in 05.5.3)
- [ ] Verify `NotifyPaneBell` maps to `MuxNotification::PaneBell`
      (renamed from `Alert` in section 03.2, already complete by this point)
- [ ] Keep: `NotifyPaneOutput`, `NotifyPaneExited`, `NotifyPaneTitleChanged`,
      `NotifyPaneBell`, `NotifyPaneSnapshot` mappings

### 05.5.5 MuxClient transport layer (`transport.rs`)

**NOTE: `transport.rs` is 525 lines (slightly over 500-line limit).** After
removing `TabLayoutUpdate` and related code, verify it drops below 500.

- [ ] Delete `TabLayoutUpdate` struct from `transport.rs`
- [ ] Remove `pushed_layouts: Arc<Mutex<HashMap<TabId, TabLayoutUpdate>>>` field
      from `ClientTransport`
- [ ] Remove `take_pushed_layout()` method from `ClientTransport`
- [ ] Remove `pushed_layouts` initialization and cloning from `connect()`
- [ ] Remove `pushed_layouts` parameter from `reader_loop()` signature
- [ ] Remove `pushed_layouts` parameter from `dispatch_notification()` signature
- [ ] Remove `NotifyTabLayoutChanged` match arm from `dispatch_notification()`
- [ ] Verify `MuxNotification::PaneOutput` references in `dispatch_notification()`
      are correct (renamed from `PaneDirty` in section 03.2, already complete)
- [ ] Remove `MuxNotification::TabLayoutChanged` send from `dispatch_notification()`
- [ ] Remove imports: `use crate::layout::floating::FloatingLayer`,
      `use crate::layout::split_tree::SplitTree`, `use crate::TabId`

### 05.5.6 MuxClient `apply_layout_update()`

- [ ] Delete `apply_layout_update()` method entirely
- [ ] Remove call to `apply_layout_update()` from `poll_events()` event loop

### 05.5.7 Backend tests

- [ ] Update `embedded/tests.rs` — remove tests that call tab/window methods
- [ ] Update `client/tests.rs` — remove tests that call tab/window RPCs
- [ ] Add tests for new `spawn_pane()` method on both backends

---

## 05.6 Completion Checklist

- [ ] `grep -rn "TabId\|WindowId\|MuxTab\|MuxWindow\|SessionRegistry" oriterm_mux/src/protocol/ oriterm_mux/src/server/ oriterm_mux/src/backend/`
      returns zero results
- [ ] `MuxBackend` trait has only pane methods
- [ ] Server has no `window_to_client` mapping
- [ ] Protocol has no tab/window message types
- [ ] Daemon can serve pane requests without any session state
- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes

**Exit Criteria:** The wire protocol, daemon server, and backend trait
are pane-only. A non-GUI client could connect and interact with panes
without implementing any session model.
