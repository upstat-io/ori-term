---
section: "05"
title: "Resize Pipeline Migration"
status: not-started
reviewed: true
goal: "Resize flows through the IO thread as an async command — the main thread never does grid reflow, eliminating resize flashing"
inspired_by:
  - "Ghostty Surface.zig:2440 (resize queued to IO thread via self.queueIo)"
  - "Ghostty apprt/embedded.zig:794 (deduplication: skip if size unchanged)"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Resize Command Path"
    status: not-started
  - id: "05.2"
    title: "Resize Event Coalescing"
    status: not-started
  - id: "05.3"
    title: "IO Thread Resize Processing"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Resize Pipeline Migration

**Status:** Not Started
**Goal:** Window resize flows through the IO thread as an async command. The main thread sends `Resize { rows, cols }`, the IO thread does `Grid::resize()` with reflow, produces a new snapshot, and sends SIGWINCH. The renderer always draws the last completed snapshot — never an intermediate reflow state. This eliminates resize flashing, text jumping, and cursor repositioning.

**Context:** This is the payoff section. Today, `chrome/resize.rs:sync_grid_layout()` → `mux.resize_pane_grid()` → `pane.resize_grid()` → `terminal.lock().resize(rows, cols, true)` runs synchronously on the main thread. The grid reflow (`reflow_cols()`) iterates every cell. During drag resize, this happens dozens of times per second, each producing a visible frame with intermediate column widths.

After this section, the main thread sends a `Resize` command to the IO thread and continues rendering with the previous snapshot. The IO thread processes the resize, produces a new snapshot, and wakes the main thread. The renderer transitions from the old snapshot to the new one in a single frame — no intermediate states.

**Reference implementations:**
- **Ghostty** `src/Surface.zig:2440-2461`: `fn resize()` queues the resize to the IO thread via `self.queueIo(.{ .resize = self.size }, .unlocked)`. The renderer keeps drawing with the previous state.
- **Ghostty** `src/apprt/embedded.zig:794-811`: `fn updateSize()` deduplicates: returns immediately if dimensions haven't changed.

**Depends on:** Section 04 (render pipeline reads from IO thread snapshots).

---

## 05.1 Resize Command Path

**File(s):** `oriterm_mux/src/backend/embedded/mod.rs`, `oriterm_mux/src/pane/mod.rs`

Route `resize_pane_grid()` through the IO thread command channel instead of locking the terminal.

- [ ] Change `EmbeddedMux::resize_pane_grid()`:
  ```rust
  fn resize_pane_grid(&mut self, pane_id: PaneId, rows: u16, cols: u16) {
      if let Some(pane) = self.panes.get(&pane_id) {
          // Send resize command to IO thread (non-blocking).
          pane.send_io_command(PaneIoCommand::Resize { rows, cols });
          // Also resize old Term for dual-Term consistency (scroll/search
          // still use old Term until section 06 migrates them).
          pane.resize_grid(rows, cols);
      }
      self.snapshot_dirty.insert(pane_id);
  }
  ```

- [ ] Add `Pane::send_io_command()` helper:
  ```rust
  pub fn send_io_command(&self, cmd: PaneIoCommand) {
      if let Some(ref handle) = self.io_handle {
          handle.send_command(cmd);
      }
  }
  ```

- [ ] Keep the old `pane.resize_grid()` call alongside the IO command (dual-Term consistency — see 05.3 bullet on dual-Term resize). The old Term must have correct dimensions for scroll/search operations until section 06 migrates them.
- [ ] Remove the old `pane.resize_pty()` call (PTY resize now happens on the IO thread after reflow)

---

## 05.2 Resize Event Coalescing

**File(s):** `oriterm/src/app/chrome/resize.rs`

During drag resize, winit fires dozens of `Resized` events per second. Each computes a new grid size. Instead of sending a command for every event, coalesce to only send the latest size per frame.

- [ ] Add coalescing state to the resize path. Two approaches:

  **Option A (recommended)**: Let the IO thread handle coalescing. The channel naturally queues multiple `Resize` commands. The IO thread drains all commands before processing — it sees `Resize{83,24}`, `Resize{82,24}`, `Resize{81,24}` and processes only the last one before producing a snapshot.

  **Why this is best**: No changes needed in `chrome/resize.rs`. The IO thread's `drain_commands()` naturally coalesces by processing all pending commands before snapshotting. For `Resize`, only the last one's dimensions matter.

  **Option B**: Coalesce on the main thread — track `pending_resize: Option<(u16, u16)>` and only send when entering `about_to_wait()`. More complex, requires new state on the App.

  **Recommended**: Option A. The IO thread's loop already drains all commands first:
  ```rust
  fn drain_commands(&mut self) {
      let mut last_resize = None;
      while let Ok(cmd) = self.cmd_rx.try_recv() {
          match cmd {
              PaneIoCommand::Resize { rows, cols } => {
                  // Coalesce: only keep the last resize.
                  last_resize = Some((rows, cols));
              }
              PaneIoCommand::Shutdown => {
                  self.shutdown.store(true, Ordering::Release);
                  return;
              }
              other => self.handle_command(other),
          }
      }
      // Process the coalesced resize last.
      if let Some((rows, cols)) = last_resize {
          self.process_resize(rows, cols);
      }
  }
  ```

- [ ] The main thread's `sync_grid_layout()` still computes grid dimensions from cell metrics (this stays on the main thread — it's pure math, no terminal state). It then sends the command. The IO thread does the heavy work (reflow + snapshot).

---

## 05.3 IO Thread Resize Processing

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

Implement the resize command handler on the IO thread.

- [ ] Implement `process_resize()`:
  ```rust
  fn process_resize(&mut self, rows: u16, cols: u16) {
      // Reflow the grid on the IO thread.
      self.terminal.resize(rows as usize, cols as usize, true);
      
      // Notify the PTY (SIGWINCH) — dedup check included.
      let packed = (rows as u32) << 16 | cols as u32;
      if self.last_pty_size.swap(packed, Ordering::Relaxed) != packed {
          if let Err(e) = self.pty_control.resize(rows, cols) {
              log::warn!("PTY resize failed: {e}");
          }
      }
      
      // Snapshot will be produced in the main loop after drain_commands().
  }
  ```

- [ ] Move `last_pty_size: AtomicU32` from `Pane` to `PaneIoThread` (it's only used by the resize path, which now runs on the IO thread).

- [ ] Move `PtyControl` to `PaneIoThread` (resize is the only operation that uses it, and it must happen on the IO thread after reflow to maintain correct ordering: reflow → SIGWINCH).
  - **WARNING**: `server/dispatch/mod.rs:100` also calls `pane.resize_pty()`. When `PtyControl` moves to the IO thread, `Pane::resize_pty()` can no longer access it directly. Two options: (a) also update `server/dispatch` resize in this section (not waiting for 06.6), or (b) keep a temporary `PtyControl` clone on `Pane` until 06.6. **Recommended: (a)** — update both `EmbeddedMux::resize_pane_grid()` and `server/dispatch` resize to send `PaneIoCommand::Resize` in this section. This is a small scope increase but prevents a broken daemon path between sections 05 and 06.
  - [ ] Update `server/dispatch/mod.rs` resize handler to use `pane.send_io_command(PaneIoCommand::Resize { rows, cols })` instead of calling `pane.resize_grid()` + `pane.resize_pty()` directly.

- [ ] Surface reconfiguration stays on the main thread:
  ```
  Main thread: Resized event → compute grid dims → send Resize command
                              → resize GPU surface (deferred)
                              → update chrome layout (tab bar, status bar)
  IO thread:                  → Grid::resize() with reflow
                              → produce snapshot
                              → PTY resize (SIGWINCH)
                              → wakeup main thread
  Main thread:                → read new snapshot → render
  ```

- [ ] The `display_offset = 0` reset in `finalize_resize()` happens on the IO thread (it's part of `Grid::resize()`). This is correct — the snapshot will reflect the reset offset.

- [ ] Verify BUG-06.2 is resolved: hold a key to fill the screen, release, resize. The race between key repeat and SIGWINCH is eliminated because both PTY bytes and resize are processed serially on the IO thread. The IO thread processes the resize command, which reflows the grid, then the shell receives SIGWINCH and sends new output — which the IO thread processes in order.

- [ ] **Dual-Term resize consistency**: During sections 05-06, the old `Term` in `Arc<FairMutex>` also needs to be resized to keep the dual-Term state consistent. Options: (a) send `Resize` to the IO thread AND call the old `pane.resize_grid()` for the old Term, or (b) accept that the old Term has stale dimensions (acceptable since rendering now uses IO thread snapshots). **Recommended: (a)** — resize the old Term too. The old Term still handles scroll/search/extract operations until section 06 migrates them. Stale grid dimensions on the old Term cause `display_offset` clamping to use wrong bounds (e.g., scroll thinks there are 24 rows when there are now 30), which produces visibly wrong scroll behavior between sections 05 and 06. The cost of resizing the old Term is negligible (it runs on the main thread once per resize, no VTE parsing needed). Section 07 removes it.

- [ ] `/tpr-review` checkpoint

### Tests

**File:** `oriterm_mux/src/pane/io_thread/tests.rs` (extend)

- [ ] `test_resize_command_reflows_grid` — create IO thread with 80x24 Term, send `Resize { rows: 24, cols: 40 }`. Assert the IO thread's `Term` grid has 40 columns after processing.
- [ ] `test_resize_coalescing` — send `Resize { rows: 24, cols: 80 }`, `Resize { rows: 24, cols: 60 }`, `Resize { rows: 24, cols: 40 }` in rapid succession. Assert only the last resize (40 cols) is applied (one reflow, not three). Verify by checking final grid dimensions.
- [ ] `test_resize_produces_snapshot` — send `Resize`, assert `SnapshotDoubleBuffer::has_new()` returns `true` after processing. Verify the snapshot reflects the new dimensions.
- [ ] `test_resize_sends_pty_sigwinch` — use a mock `PtyControl` that records resize calls. Send `Resize { rows: 30, cols: 100 }`. Assert `pty_control.resize(30, 100)` was called exactly once.
- [ ] `test_resize_dedup_skips_same_size` — send two `Resize` commands with identical dimensions. Assert `PtyControl::resize()` is called only once (dedup via `last_pty_size`).
- [ ] `test_resize_display_offset_resets` — scroll up (display_offset > 0), then send `Resize`. Assert display_offset is 0 in the snapshot (Grid::resize calls finalize_resize which resets it).
- [ ] `test_resize_interleaved_with_bytes` — send bytes, then `Resize`, then more bytes. Assert the grid content reflects parsing before and after the resize (no data loss).

**File:** `oriterm_mux/src/backend/embedded/tests.rs` (extend)

- [ ] `test_resize_pane_grid_sends_command` — call `EmbeddedMux::resize_pane_grid()`. Assert the pane's IO thread received a `Resize` command (not a direct terminal lock).

---

## 05.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 05.N Completion Checklist

- [ ] `resize_pane_grid()` sends `PaneIoCommand::Resize` instead of locking terminal
- [ ] Resize events coalesced on IO thread (only last size processed per cycle)
- [ ] IO thread does `Grid::resize()` with reflow
- [ ] IO thread sends PTY resize (SIGWINCH) after reflow (correct ordering)
- [ ] `PtyControl` owned by IO thread
- [ ] Surface reconfiguration stays on main thread (GPU ops)
- [ ] No resize flashing during drag resize (last-good snapshot drawn until reflow completes)
- [ ] BUG-06.2 resolved: no garbled text after resize during key repeat
- [ ] Multi-pane resize works (`resize_all_panes()` sends one command per pane)
- [ ] `timeout 150 cargo test -p oriterm_mux` passes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Window resize produces zero visible flashing. The renderer draws the last-good snapshot while the IO thread reflows. Resize during flood output is smooth. BUG-06.2 is resolved. `display_offset` resets correctly after resize.
