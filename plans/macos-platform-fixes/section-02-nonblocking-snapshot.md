---
section: "02"
title: "Non-Blocking Snapshot Refresh"
status: not-started
goal: "Tab switch renders instantly using stale/placeholder data; no synchronous RPC on the UI thread"
inspired_by:
  - "WezTerm wezterm-client/src/domain.rs: async snapshot fetch via promise + poll pattern"
  - "Chrome tab switching: instant switch with progressive content load"
depends_on: []
sections:
  - id: "02.1"
    title: "Diagnose the Blocking Path"
    status: not-started
  - id: "02.2"
    title: "Non-Blocking Fallback Strategy"
    status: not-started
  - id: "02.3"
    title: "Pre-Subscribe Optimization"
    status: not-started
  - id: "02.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Non-Blocking Snapshot Refresh

**Status:** Not Started
**Goal:** Tab switching is instant on all platforms and process models (embedded and daemon). The render path never blocks on synchronous RPC. When a pushed snapshot is not yet available, rendering uses the last known snapshot or renders a blank frame, then retries on the next event loop tick.

**Context:** When switching tabs in daemon mode, `handle_redraw()` detects `pane_changed` and calls `mux.refresh_pane_snapshot(pane_id)`. The `MuxClient` implementation first checks for a pushed snapshot (fast path), but if none is available, falls back to a synchronous `self.rpc(GetPaneSnapshot)` call that blocks the UI thread for up to 5 seconds (`RPC_TIMEOUT`). On macOS this manifests as a visible hang because the pushed snapshot for the new pane hasn't arrived yet — the daemon hasn't been told to push snapshots for this pane because the client just subscribed.

**Root cause chain:**
1. Tab switch sets `pane_changed = true` in `redraw/mod.rs:91`
2. `content_changed = true` calls `mux.refresh_pane_snapshot(pane_id)` at line 97
3. `MuxClient::refresh_pane_snapshot` (rpc_methods.rs:362) checks pushed snapshot — none available
4. Falls back to sync RPC: `self.rpc(GetPaneSnapshot)` at line 375 — blocks for up to 5s
5. UI thread frozen until response arrives or timeout

**Reference implementations:**
- **WezTerm** `wezterm-mux-server-impl/src/sessionhandler.rs`: Uses async `Pdu::GetPaneRenderChanges` with a promise-based system — never blocks the GUI thread.

**Depends on:** None.

---

## 02.1 Diagnose the Blocking Path

**File(s):** `oriterm_mux/src/backend/client/rpc_methods.rs`, `oriterm/src/app/redraw/mod.rs`

Map every call site that invokes `refresh_pane_snapshot` to understand which are in the render hot path vs. cold paths (initialization, user actions).

- [ ] Audit all call sites of `refresh_pane_snapshot`:
  - `redraw/mod.rs:97` — **HOT**: called every frame when content changes
  - `redraw/multi_pane.rs:199` — **HOT**: multi-pane variant
  - `keyboard_input/mod.rs:134` — WARM: after keystroke (acceptable latency)
  - `keyboard_input/overlay_dispatch.rs:243` — WARM
  - `mouse_input.rs:237,290,324` — WARM: after mouse actions
  - `pane_accessors.rs:60` — WARM: accessor

- [ ] Confirm: the sync RPC fallback path (rpc_methods.rs:375) is the sole source of the hang. Log timestamps around the `self.rpc()` call to verify on macOS.

---

## 02.2 Non-Blocking Fallback Strategy

**File(s):** `oriterm_mux/src/backend/client/rpc_methods.rs`, `oriterm/src/app/redraw/mod.rs`

Replace the synchronous RPC fallback with a non-blocking pattern: if no pushed snapshot is available, use the stale snapshot and schedule an async fetch.

**Fix approach — 2 options:**

**(a) MarkAllDirty + stale snapshot** (recommended — simplest, no new async machinery):

In `refresh_pane_snapshot`, when no pushed snapshot is available:
1. Send a fire-and-forget `MarkAllDirty` request (non-blocking, triggers pushed snapshot from daemon)
2. Return the existing stale snapshot from `self.pane_snapshots` (may be `None` for brand-new panes)
3. Mark the pane dirty so the next event loop tick retries
4. The daemon will respond with a pushed `NotifyPaneSnapshot` (already subscribed)

**Why MarkAllDirty, not GetPaneSnapshot:** `GetPaneSnapshot` is a request-response PDU — the daemon builds a snapshot and sends `PaneSnapshotResp` back. But `fire_and_forget` sends with `reply_tx: None`, so the reader thread (reader.rs:277-282) logs a warning and drops the response. `MarkAllDirty` is designed for fire-and-forget: it triggers the daemon to push a `NotifyPaneSnapshot` via the existing subscription channel, which the pushed-snapshot path picks up on the next tick.

**Why `invalidate_pushed_snapshot` must NOT be called:** The existing `MuxBackend::mark_all_dirty()` wrapper (rpc_methods.rs:138-144) calls `transport.invalidate_pushed_snapshot(pane_id)`, which would drop the very snapshot we are waiting for. The non-blocking fallback must call `transport.fire_and_forget(MuxPdu::MarkAllDirty { pane_id })` directly, bypassing the wrapper.

```rust
fn refresh_pane_snapshot(&mut self, pane_id: PaneId) -> Option<&PaneSnapshot> {
    // Fast path: server-pushed snapshot available.
    let pushed = self
        .transport
        .as_ref()
        .and_then(|t| t.take_pushed_snapshot(pane_id));
    if let Some(snapshot) = pushed {
        self.pane_snapshots.insert(pane_id, snapshot);
        self.pending_refresh.remove(&pane_id);  // Fresh data arrived.
        return self.pane_snapshots.get(&pane_id);
    }

    // Non-blocking: trigger a pushed snapshot via MarkAllDirty, return stale data.
    if !self.pending_refresh.contains(&pane_id) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::MarkAllDirty { pane_id });
        }
        self.pending_refresh.insert(pane_id);
    }
    // Keep pane dirty so next tick retries.
    self.dirty_panes.insert(pane_id);
    self.pane_snapshots.get(&pane_id)
}
```

**Why this is best:** Zero new types, zero async machinery, zero architectural changes. The pushed snapshot system already delivers snapshots asynchronously — we just stop blocking while waiting for it. The pane stays dirty, so the next `poll_events` + `handle_redraw` cycle picks up the pushed snapshot.

**Trade-off:** First frame after tab switch may show stale content or a blank terminal. This is acceptable — Chrome does the same (tab switch shows cached thumbnail, then live content).

**(b) Async RPC with channel** (not recommended — over-engineered):
Spawn a background thread per request, return via channel. Adds complexity for the same result since the pushed snapshot path already provides async delivery.

**Recommended path:** Option (a).

- [ ] Implement option (a) in `MuxClient::refresh_pane_snapshot` (rpc_methods.rs:362-388). Replace the ~15-line sync RPC fallback with the ~20-line non-blocking version above. Net change: ~+5 lines, bringing rpc_methods.rs from 394 to ~399 lines.

- [ ] Verify `None` handling in `redraw/mod.rs` — already exists at lines 110-113 (`ctx.dirty = true; return;`). No code change needed, but confirm the `dirty` flag propagates to `request_redraw()` before the next frame.

- [ ] Verify `None` handling in `redraw/multi_pane.rs` — already exists at lines 212-215 (`ctx.dirty = true; continue;`). Same verification needed.

- [ ] **Critical dirty-flag lifecycle**: After `refresh_pane_snapshot` returns stale data, `clear_pane_snapshot_dirty(pane_id)` is called at `redraw/mod.rs:137` and `multi_pane.rs:249`, which clears the dirty flag that `refresh_pane_snapshot` just set. This means the retry will not happen automatically.

  **Fix**: Add a `pending_refresh: HashSet<PaneId>` field to `MuxClient`. When the non-blocking fallback fires, insert the pane_id. In `clear_pane_snapshot_dirty`, if the pane is in `pending_refresh`, skip the remove (or re-insert into `dirty_panes`). When a pushed snapshot arrives in the fast path, remove the pane from `pending_refresh`.

  ```rust
  fn clear_pane_snapshot_dirty(&mut self, pane_id: PaneId) {
      if self.pending_refresh.contains(&pane_id) {
          // Don't clear — async refresh still outstanding.
          return;
      }
      self.dirty_panes.remove(&pane_id);
  }
  ```

- [ ] Add `pending_refresh: HashSet<PaneId>` field to `MuxClient` struct in `oriterm_mux/src/backend/client/mod.rs` — initialize as empty `HashSet::new()` in both `connect()` (line 59) and `new()` (line 72, test stub). Note: `mod.rs` is currently 153 lines; adding one field + one line per constructor keeps it well under the 500-line limit.

- [ ] **Cleanup on pane close**: `remove_snapshot()` in `mod.rs:86` must also call `self.pending_refresh.remove(&pane_id)` to prevent a leak of pane IDs in the `pending_refresh` set after panes are closed.

- [ ] **Embedded backend is unaffected**: `EmbeddedMux::refresh_pane_snapshot` builds the snapshot synchronously from the local terminal lock. No change needed. Verify it still compiles.

---

## 02.3 Pre-Subscribe Optimization (Optional Enhancement)

**File(s):** `oriterm_mux/src/backend/client/mod.rs`

**This subsection is an optional enhancement.** The core fix (02.2) makes tab switch non-blocking regardless. This optimization reduces how often the non-blocking fallback fires by ensuring pushed snapshots are already flowing for all panes before the user switches tabs.

The real reason pushed snapshots aren't available on tab switch is timing: the client subscribes to the new pane's notifications, but the first pushed snapshot hasn't arrived yet. Optimize by eagerly subscribing to all panes at connection time, not just on first render.

- [ ] In `MuxClient::connect` (or the initial handshake after `HelloAck`), subscribe to all existing panes using `ListPanes` + `Subscribe` for each
- [ ] When `spawn_pane` returns, the subscription already happens (line 64 of rpc_methods.rs) — verify this is correct
- [ ] Consider: on tab switch, send a `MarkAllDirty` for the new pane to trigger an immediate pushed snapshot from the daemon

---

## 02.4 Completion Checklist

- [ ] `refresh_pane_snapshot` never calls `self.rpc()` (sync) — only `fire_and_forget`
- [ ] `clear_pane_snapshot_dirty` respects `pending_refresh` — does not clear dirty while async refresh is outstanding
- [ ] `pending_refresh: HashSet<PaneId>` added to `MuxClient` and initialized in both constructors
- [ ] `invalidate_pushed_snapshot` is NOT called in the non-blocking fallback path
- [ ] `remove_snapshot` cleans up `pending_refresh` for closed panes
- [ ] Tab switch on macOS is instant (no visible hang)
- [ ] Tab switch on Windows remains instant (no regression)
- [ ] Tab switch in embedded mode remains instant (no regression)
- [ ] New pane spawn renders content within 1-2 event loop ticks
- [ ] `./build-all.sh` succeeds
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] Log output shows no `refresh_pane_snapshot: RPC failed` errors during normal tab switch
- [ ] Unit tests for `MuxClient` in `oriterm_mux/src/backend/client/tests.rs` updated: verify `refresh_pane_snapshot` returns stale data (not `None`) on second call, verify `pending_refresh` lifecycle

**Exit Criteria:** Switching between tabs in daemon mode on macOS takes <16ms (one frame). No synchronous RPC calls remain in the render path. The `RPC_TIMEOUT` path is only used for cold-start operations (spawn, close, extract text/html, scroll-to-prompt) where blocking is acceptable.
