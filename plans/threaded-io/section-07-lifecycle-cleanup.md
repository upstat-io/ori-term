---
section: "07"
title: "Pane Lifecycle & FairMutex Removal"
status: in-progress
reviewed: true
goal: "Remove Arc<FairMutex<Term>> from Pane, clean up the old parsing path, and verify all terminal access goes through the IO thread"
inspired_by:
  - "Ghostty (no shared mutex on terminal state — IO thread owns exclusively)"
depends_on: ["05", "06"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "07.1"
    title: "Pane Struct Refactor"
    status: complete
  - id: "07.2"
    title: "Old PtyEventLoop Removal"
    status: complete
  - id: "07.3"
    title: "FairMutex Assessment"
    status: complete
  - id: "07.4"
    title: "Daemon Mode Compatibility"
    status: complete
  - id: "07.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "07.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: Pane Lifecycle & FairMutex Removal

**Status:** In Progress
**Goal:** Remove `Arc<FairMutex<Term<MuxEventProxy>>>` from `Pane`. Remove the old `PtyEventLoop` parsing code. Clean up `Pane` to hold only `PaneIoHandle` for terminal access. Verify all terminal state flows through the IO thread exclusively.

**Context:** After sections 04-06, no code outside the IO thread accesses `Term` directly. The `Arc<FairMutex>` field in `Pane` is dead weight. Removing it completes the architectural migration and eliminates the contention that caused resize flashing.

**Depends on:** Sections 05 and 06 (all operations migrated to IO commands).

---

## 07.1 Pane Struct Refactor

**File(s):** `oriterm_mux/src/pane/mod.rs`, `oriterm_mux/src/pane/shutdown.rs`

Remove the FairMutex-wrapped terminal from `Pane` and replace with the IO handle.

- [x] Remove `terminal: Arc<FairMutex<Term<MuxEventProxy>>>` from `Pane`
- [x] Remove `pub fn terminal(&self) -> &Arc<FairMutex<Term<MuxEventProxy>>>` accessor
- [x] Replace with:
  ```rust
  /// IO thread handle — all terminal access goes through commands.
  io_handle: PaneIoHandle,
  ```
- [x] Make `io_handle` non-optional (was `Option<PaneIoHandle>` during transition)
- [x] Switch `IoThreadEventProxy.suppress_metadata` to `false` in the `PaneIoThread` constructor now that the old `Term` is removed. The IO thread's proxy becomes the sole source of metadata events (title, CWD, bell, clipboard, PtyWrite).

- [x] Update `PaneParts`:
  - Remove `terminal: Arc<FairMutex<Term<MuxEventProxy>>>`
  - Add `io_handle: PaneIoHandle`

- [x] Update `Pane::from_parts()` — construct from IO handle
- [x] Update `LocalDomain::spawn_pane()` to create only ONE `Term` (for the IO thread). Remove the old `Arc<FairMutex<Term>>` creation, the old `PtyEventLoop` VTE parsing setup, and the dual-Term byte forwarding. The spawn path becomes: create Term → create PaneIoThread (owns Term) → create PtyReader (byte forwarder only) → create PtyWriter (unchanged).

- [x] Remove methods that locked the terminal directly:
  - `resize_grid()` — replaced by `send_io_command(Resize)`
  - `resize_pty()` — handled by IO thread
  - `scroll_to_bottom()` — replaced by `send_io_command(ScrollToBottom)`
  - `scroll_display()` — replaced by `send_io_command(ScrollDisplay)`
  - `scroll_to_previous_prompt()` / `scroll_to_next_prompt()` — via commands
  - `enter_mark_mode()` — via command with reply

- [x] Keep methods that don't need terminal access:
  - All title/CWD/bell/unseen_output methods (Pane-local state)
  - `write_input()` — sends to PTY writer thread (unchanged)
  - `mark_cursor()`, `exit_mark_mode()`, `set_mark_cursor()` — Pane-local
  - `is_mark_mode()`, `selection()` — Pane-local
  - Note: `search()` accessor is REMOVED — search state moved to IO thread in section 06.4. Search data for rendering comes from `RenderableContent` snapshot. For daemon snapshot metadata, search fields are read from the IO thread's snapshot (already in `RenderableContent`). `is_search_active()` reads the `search_active` atomic added in section 06.4.

- [x] Update `Pane` shutdown (`shutdown.rs`):
  - Call `io_handle.shutdown()` which sends `PaneIoCommand::Shutdown` and joins the IO thread
  - The IO thread sets the shared `shutdown` flag, causing the PTY reader and writer threads to exit
  - `cleanup_closed_pane()` drops `Pane` on a background thread (existing pattern)

- [x] **Fix grid_dirty signaling after old Term removal.** Currently `EmbeddedMux::poll_events()` checks `pane.grid_dirty()` (set by old `MuxEventProxy`). After the old `Term` is removed, this flag is never set. Two options:
  
  **Option A (recommended)**: Have `poll_events()` check `SnapshotDoubleBuffer::has_new()` instead of `pane.grid_dirty()`. The IO thread publishes a new snapshot whenever the grid changes. The main thread detects this via `has_new()`. Remove `grid_dirty` and `wakeup_pending` Arcs from `Pane` since they're no longer used.

  **Option B**: The `IoThreadEventProxy` (with `suppress_metadata = false`) still sets its own `grid_dirty` Arc. Share this Arc with `Pane` so `pane.grid_dirty()` reads the IO thread's flag. Requires wiring the Arc through `PaneIoHandle`.

  Both work. Option A is cleaner because it eliminates the intermediate flag — the snapshot buffer IS the signal.

- [x] If using Option A: update `EmbeddedMux::poll_events()`:
  ```rust
  for (&pane_id, pane) in &self.panes {
      if pane.has_new_snapshot() {  // delegates to io_handle.double_buffer.has_new()
          self.snapshot_dirty.insert(pane_id);
      }
  }
  ```
- [x] Remove `grid_dirty: Arc<AtomicBool>` and `wakeup_pending: Arc<AtomicBool>` from `Pane` and `PaneParts`
- [x] Remove `Pane::grid_dirty()`, `Pane::clear_grid_dirty()`, `Pane::clear_wakeup()` accessors
- [x] Update `MuxNotification::PaneOutput` handling — this notification was fired by `MuxEventProxy` on the old path. After section 07, the IO thread's wakeup callback (which fires the guarded wakeup) is the only trigger. The `PaneOutput` notification is no longer needed for render — only for unseen output tracking.

### Tests (07.1)

**File:** `oriterm_mux/src/pane/tests.rs` (update existing)

- [x] `test_pane_no_terminal_accessor` — compile-time verification: assert that `Pane` does NOT have a `terminal()` method. Any test that calls `pane.terminal()` must be updated. Grep for `terminal()` in `oriterm_mux/src/pane/tests.rs` and update all call sites.
- [x] `test_pane_from_parts_requires_io_handle` — construct `PaneParts` without `io_handle`. Assert compilation fails (field is non-optional).
- [x] `test_pane_send_io_command_delegates` — create a `Pane`, send a command via `send_io_command()`. Assert it arrives on the IO thread's `cmd_rx`.
- [x] `test_pane_has_new_snapshot_delegates` — produce a snapshot on the IO thread. Assert `pane.has_new_snapshot()` returns `true`.

**File:** `oriterm_mux/src/pane/io_thread/event_proxy/tests.rs` (extend)

- [x] `test_io_thread_event_proxy_unsuppressed_forwards_all` — set `suppress_metadata = false` (post-section-07 mode). Send `Event::Title("test")`, `Event::PtyWrite("data")`, `Event::Bell`. Assert ALL events are forwarded to the inner `MuxEventProxy`. This verifies the proxy works as sole event source.

**File:** `oriterm_mux/src/backend/embedded/tests.rs` (extend)

- [x] `test_poll_events_uses_has_new_snapshot` — after section 07, `poll_events()` should check `pane.has_new_snapshot()` instead of `pane.grid_dirty()`. Verify a snapshot flip triggers `snapshot_dirty` insertion.
- [x] `test_cleanup_closed_pane_with_io_thread` — spawn a pane, close it via `cleanup_closed_pane()`. Assert the IO thread is shut down, the JoinHandle is joined, and no threads leak.

- [ ] `/tpr-review` checkpoint

### Implementation Notes (07.1)

**Additional fixes discovered during implementation:**

1. **IO thread `grid_dirty` after parsing**: The IO thread's `handle_bytes()` was not setting `grid_dirty` after VTE parsing. The old `PtyEventLoop` explicitly sent `Event::Wakeup` after each parse chunk, but `Event::Wakeup` is never generated by oriterm_core's VTE handler — it's the parse loop's responsibility. Fixed by setting `grid_dirty = true` in `handle_bytes()` after `processor.advance()`, respecting Mode 2026 sync output.

2. **Flood output snapshot starvation**: `process_pending_bytes()` drained the entire byte queue before producing a snapshot. During flood output, bytes arrived faster than parsing, so `maybe_produce_snapshot()` never ran. Fixed by calling `maybe_produce_snapshot()` between each byte message in the drain loop.

3. **PtyReader created**: New `oriterm_mux/src/pty/reader/` module — simple byte forwarder replacing the VTE-parsing `PtyEventLoop` in `spawn_pane()`. The old `PtyEventLoop` code remains for 07.2 cleanup.

4. **IoThreadEventProxy metadata forwarding**: Added `pane_id`, `mux_tx`, `wakeup` fields to forward title/CWD/bell/PtyWrite/clipboard events when `suppress_metadata` is false. Matches `MuxEventProxy` event handling.

---

## 07.2 Old PtyEventLoop Removal

**File(s):** `oriterm_mux/src/pty/event_loop/mod.rs`

The old `PtyEventLoop` still has full VTE parsing (unchanged through sections 02-06, only byte forwarding was added in section 02). Now that all operations route through the IO thread, the old parsing path is dead weight. Convert `PtyEventLoop` to a simple byte forwarder.

- [x] Remove all VTE parsing infrastructure from `PtyEventLoop`:
  - Remove `terminal: Arc<FairMutex<Term<T>>>` field
  - Remove `processor`, `raw_parser` fields
  - Remove `mode_cache: Arc<AtomicU32>` field
  - Remove `try_parse()`, `parse_chunk()` methods
  - Remove `MAX_LOCKED_PARSE` constant
  - Remove `FairMutex` imports and usage

- [x] The remaining `PtyEventLoop` is a simple byte forwarder:
  ```rust
  /// PTY byte forwarder — reads shell output and sends to the IO thread.
  ///
  /// Formerly `PtyEventLoop` when it owned VTE parsing. Now a simple
  /// read loop that forwards raw bytes via channel.
  pub struct PtyReader {
      reader: Box<dyn Read + Send>,
      byte_tx: crossbeam_channel::Sender<Vec<u8>>,
      shutdown: Arc<AtomicBool>,
  }
  ```

- [x] Rename `oriterm_mux/src/pty/event_loop/` directory to `oriterm_mux/src/pty/reader/` (or keep as `event_loop/` with the `PtyReader` type inside — either works, directory rename is optional)
- [x] Update `pub use` in `oriterm_mux/src/pty/mod.rs`: `pub use event_loop::PtyEventLoop;` → `pub use reader::PtyReader;` (or `pub use event_loop::PtyReader;` if not renaming directory)
- [x] Update `crate::pty::PtyEventLoop` import in `oriterm_mux/src/domain/local.rs` → `crate::pty::PtyReader`
- [x] Update `PtyReader::new()` constructor: takes `(reader, byte_tx, shutdown)` instead of `(terminal, reader, shutdown, mode_cache)`
- [x] Remove the old `PtyEventLoop` tests that tested VTE parsing path. Add new tests for `PtyReader` byte forwarding (simple: write bytes to pipe, verify they arrive on the channel).

### Tests (07.2)

**File:** `oriterm_mux/src/pty/reader/tests.rs` (new — replaces old `event_loop/tests.rs`)

- [x] `test_pty_reader_forwards_bytes` — write known bytes to a pipe, create `PtyReader` reading from it. Assert the bytes arrive on `byte_rx` channel.
- [x] `test_pty_reader_shutdown_flag_stops_loop` — set `shutdown` `AtomicBool` to `true`. Assert the reader thread exits without reading further.
- [x] `test_pty_reader_eof_exits_cleanly` — close the write end of the pipe. Assert the reader thread exits via `JoinHandle::join() == Ok(())`.
- [x] `test_pty_reader_interrupted_read_retries` — platform-specific: on Unix, send EINTR during read. Assert the reader retries (not a fatal error).
- [x] `test_pty_reader_large_buffer_forwarding` — write 500KB of data. Assert all data arrives on the channel (may be split across multiple messages).

- [x] **Audit all test files in `oriterm_mux/src/` that call `pane.terminal()` or `terminal().lock()`**. These tests will fail after 07.1 removes the accessor. Each must be updated to either: (a) use the IO thread's command/reply pattern, or (b) construct a standalone `Term` for unit testing (not via `Pane`). Run `grep -rn "terminal()" oriterm_mux/src/**/tests.rs` to find all call sites.

---

## 07.3 FairMutex Assessment

**File(s):** `oriterm_core/src/sync/mod.rs`

Assess whether `FairMutex` is still needed anywhere.

- [x] `grep -r "FairMutex" oriterm_*/src/` — check all remaining usages
- [x] If `FairMutex` is only used by the now-removed `Arc<FairMutex<Term>>` pattern, remove the `pub use` from `oriterm_core/src/lib.rs` and mark the module as `pub(crate)` or remove it entirely.
- [x] If the daemon server (`oriterm_mux/src/server/`) still needs FairMutex for its own synchronization, keep it. Otherwise, remove.
- [x] Keep `oriterm_core/src/sync/mod.rs` and `tests.rs` if the type is still used. Otherwise, remove both files.
- [x] Update `oriterm_core/src/lib.rs` exports accordingly.

---

## 07.4 Daemon Mode Compatibility

**File(s):** `oriterm_mux/src/server/`, `oriterm_mux/src/backend/daemon/`

Verify the daemon mode still works with the IO thread architecture.

- [x] The daemon server spawns panes via `InProcessMux::spawn_standalone_pane()`. With the IO thread, this creates the same `PaneIoThread` as embedded mode.

- [x] The daemon server builds snapshots via `SnapshotCache::build()`. Section 04 updated this to read from `SnapshotDoubleBuffer` — verify it works in the daemon context.

- [x] The daemon client (`DaemonMux`) communicates via IPC. It doesn't access terminal state directly — it receives `PaneSnapshot` via the wire protocol. No changes needed for the client side.

- [x] The daemon server receives resize commands via IPC (`ServerCommand::Resize`). These should route through `InProcessMux::resize_pane_grid()` → IO thread command, same as embedded mode.

---

## 07.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 07.N Completion Checklist

- [x] `Pane` no longer holds `Arc<FairMutex<Term<MuxEventProxy>>>`
- [x] `Pane::terminal()` accessor removed
- [x] All direct terminal lock call sites removed from `oriterm_mux`
- [x] `PtyEventLoop` renamed to `PtyReader`, VTE parsing code removed
- [x] `FairMutex` usage assessed and removed if unused
- [x] Daemon mode builds and works with IO thread architecture
- [x] Pane spawn creates IO thread + PTY reader + PTY writer (3 threads per pane)
- [x] Pane shutdown gracefully stops all 3 threads
- [x] `timeout 150 cargo test -p oriterm_mux` passes
- [x] `timeout 150 cargo test -p oriterm_core` passes
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** `grep -rn "FairMutex" oriterm_mux/src/` returns zero results (or only daemon-specific uses). `grep -rn "terminal().lock()" oriterm_mux/src/` returns zero results. The `Pane` struct holds `PaneIoHandle` as its only terminal access path.
