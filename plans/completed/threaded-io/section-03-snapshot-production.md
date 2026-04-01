---
section: "03"
title: "Snapshot Production & Transfer"
status: complete
reviewed: true
goal: "IO thread produces RenderableContent snapshots and publishes them to a shared buffer that the main thread can read with minimal contention"
inspired_by:
  - "Ghostty Surface.zig (renderer reads terminal state snapshot, not live mutex)"
  - "Alacritty display/content.rs (RenderableContent iterator, lock held briefly for snapshot)"
depends_on: ["02"]
third_party_review:
  status: findings
  updated: 2026-03-31
sections:
  - id: "03.1"
    title: "Shared Snapshot Buffer"
    status: complete
  - id: "03.2"
    title: "IO Thread Snapshot Production"
    status: complete
  - id: "03.3"
    title: "Wakeup After Publish"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "03.N"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Snapshot Production & Transfer

**Status:** Complete
**Goal:** The IO thread produces `RenderableContent` snapshots after processing bytes/commands and publishes them to a shared buffer. The main thread reads the latest snapshot with minimal contention (brief Mutex swap, not long-held lock).

**Context:** Today, `build_snapshot_into()` runs on the main thread under `FairMutex::lock()`. This means the renderer blocks while the PTY reader holds the lease. After this section, the IO thread produces snapshots after each processing cycle and publishes them. The main thread reads the latest published snapshot without waiting for terminal state.

**Reference implementations:**
- **Ghostty** `src/Surface.zig`: The renderer reads a snapshot of terminal state. The IO thread publishes the snapshot after processing resize/PTY data. The renderer never blocks on the IO thread.
- **Alacritty** `alacritty/src/display/content.rs`: `RenderableContent` is built from `&Term` with an iterator. Lock held during iteration, then explicitly dropped before GPU work.

**Depends on:** Section 02 (IO thread parses VTE, owns `Term`).

---

## 03.1 Shared Snapshot Buffer

**File(s):** `oriterm_mux/src/pane/io_thread/snapshot.rs` (new)

Create the shared buffer that transfers `RenderableContent` from the IO thread to the main thread.

- [x] Create `oriterm_mux/src/pane/io_thread/snapshot/` as a directory module: `snapshot/mod.rs` + `snapshot/tests.rs` with `#[cfg(test)] mod tests;` at the bottom of `mod.rs`
- [x] Create `snapshot/mod.rs` with `DoubleBuffer`:
  ```rust
  //! Double-buffered snapshot transfer between IO thread and main thread.
  //!
  //! The IO thread produces a `RenderableContent` into the back buffer,
  //! then flips. The main thread reads from the front buffer. Both
  //! sides hold the lock only for the flip (two pointer swaps) —
  //! nanoseconds, not microseconds.
  //!
  //! **Why not Option::take()?** A latest-only slot loses damage state
  //! from skipped snapshots and breaks buffer reuse (the producer gets
  //! None back and loses Vec allocations). The double buffer ensures:
  //! 1. Every consumed snapshot has valid, cumulative damage
  //! 2. Both sides always have a buffer with retained allocations
  //! 3. Skipped snapshots merge damage into the next one
  
  use std::sync::Arc;
  use parking_lot::Mutex;  // already in oriterm_mux/Cargo.toml
  use oriterm_core::RenderableContent;
  
  /// Double-buffered snapshot transfer.
  ///
  /// The IO thread writes to `back`, flips to make it the new `front`.
  /// The main thread reads `front`. Lock is held only for the swap.
  #[derive(Clone)]
  pub struct SnapshotDoubleBuffer {
      inner: Arc<Mutex<DoubleBufferSlot>>,
  }
  
  struct DoubleBufferSlot {
      /// Front buffer — latest completed snapshot for the main thread.
      front: RenderableContent,
      /// Sequence number incremented on each flip.
      seqno: u64,
      /// Sequence number the main thread last read.
      consumed_seqno: u64,
  }
  
  impl SnapshotDoubleBuffer {
      pub fn new() -> Self {
          Self {
              inner: Arc::new(Mutex::new(DoubleBufferSlot {
                  front: RenderableContent::default(),
                  seqno: 0,
                  consumed_seqno: 0,
              })),
          }
      }
      
      /// Flip: the IO thread's completed buffer becomes front.
      ///
      /// Swaps the caller's buffer with the front in-place. After this call,
      /// `buf` contains the old front (with retained Vec allocations) for the
      /// IO thread to reuse. If the main thread hasn't consumed the previous
      /// front, damage accumulates (all_dirty is set on the new front).
      pub fn flip_swap(&self, buf: &mut RenderableContent) {
          let mut slot = self.inner.lock();
          // If main thread hasn't consumed, mark new frame as all_dirty
          // to avoid losing damage from the skipped frame.
          let skipped = slot.seqno > slot.consumed_seqno;
          slot.seqno += 1;
          std::mem::swap(&mut slot.front, buf);
          if skipped {
              slot.front.all_dirty = true;
          }
          // buf now holds the old front — IO thread reuses its allocations.
      }
      
      /// Swap the front buffer with the caller's buffer.
      ///
      /// The caller (main thread) gives its old buffer and receives
      /// the latest snapshot. Both sides retain Vec allocations.
      pub fn swap_front(&self, caller_buf: &mut RenderableContent) -> bool {
          let mut slot = self.inner.lock();
          if slot.seqno == slot.consumed_seqno {
              return false; // no new snapshot
          }
          std::mem::swap(&mut slot.front, caller_buf);
          slot.consumed_seqno = slot.seqno;
          true
      }
      
      /// Whether a new snapshot is available.
      pub fn has_new(&self) -> bool {
          let slot = self.inner.lock();
          slot.seqno > slot.consumed_seqno
      }
  }
  ```

- [x] Add `SnapshotDoubleBuffer` to `PaneIoThread` and `PaneIoHandle`:
  - IO thread holds `SnapshotDoubleBuffer` for flipping
  - `PaneIoHandle` holds `SnapshotDoubleBuffer` for the main thread to read

- [x] Zero-allocation design: The IO thread fills its work buffer via `renderable_content_into()`, then calls `flip()` which gives back the old front buffer (with retained Vec allocations). The main thread calls `swap_front()` which exchanges buffers. After warmup, no allocations occur on either side.

---

## 03.2 IO Thread Snapshot Production

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

After processing bytes or commands, the IO thread produces a snapshot and publishes it.

- [x] Add snapshot production to the IO thread:
  ```rust
  /// Produce a rendering snapshot and publish it.
  ///
  /// Called after processing bytes or commands that change terminal state.
  /// Reuses buffer allocations via the double-buffer flip — after warmup,
  /// this is zero-allocation.
  fn produce_snapshot(&mut self) {
      self.terminal.renderable_content_into(&mut self.snapshot_buf);
      // Drain damage flags so next snapshot only shows new changes.
      self.terminal.reset_damage();
      // Flip: our filled buffer becomes the front, and we get back the
      // old front buffer (with retained allocations) for next frame.
      // Uses flip_swap() which swaps in-place — no mem::take needed.
      self.double_buffer.flip_swap(&mut self.snapshot_buf);
  }
  ```

- [x] Add `snapshot_buf: RenderableContent` to `PaneIoThread` for buffer reuse
- [x] Add `double_buffer: SnapshotDoubleBuffer` to `PaneIoThread`
- [x] **Damage accumulation**: If the main thread hasn't consumed the previous snapshot when a new one is flipped in, the `flip()` method sets `all_dirty = true` on the new front buffer. This ensures no damage is lost from skipped frames — the renderer does a full repaint. This is correct because skipped frames mean the renderer was behind; a full repaint catches up.

- [x] Call `produce_snapshot()` at the right points in the main loop:
  ```rust
  fn run(mut self) {
      loop {
          self.drain_commands();
          if self.shutdown.load(Ordering::Acquire) { return; }
          
          let had_bytes = self.process_pending_bytes();
          let had_commands = self.had_commands_this_cycle;
          
          if had_bytes || had_commands {
              self.produce_snapshot();
              // Wakeup main thread (section 03.3).
              (self.wakeup)();
          }
          
          // Block on next message.
          // ...
      }
  }
  ```

- [x] Respect synchronized output (Mode 2026): When sync mode is active, the VTE processor buffers output. Only produce a snapshot when sync bytes count is zero (matching the current `try_parse()` behavior at `event_loop.rs:216-223`):
  ```rust
  let sync_bytes = self.processor.sync_bytes_count();
  if sync_bytes == 0 && (had_bytes || had_commands) {
      self.produce_snapshot();
      (self.wakeup)();
  }
  ```

### Tests

**File:** `oriterm_mux/src/pane/io_thread/snapshot/tests.rs` (new — sibling tests for `snapshot.rs`)

- [x] `test_double_buffer_flip_swap_exchanges_buffers` — create `SnapshotDoubleBuffer`, fill a `RenderableContent` with 10 cells, call `flip_swap()`. Assert `swap_front()` returns `true` and the swapped-out content has 10 cells. Verifies the basic flip mechanism.
- [x] `test_double_buffer_no_new_when_not_flipped` — create buffer, call `swap_front()` without any `flip_swap()`. Assert returns `false`. Verifies `has_new()` correctness.
- [x] `test_double_buffer_skipped_frame_sets_all_dirty` — flip twice without consuming. Assert the second flip sets `all_dirty = true` on the front buffer. Verifies damage accumulation for skipped frames.
- [x] `test_double_buffer_allocation_reuse` — flip, swap_front (to get the old buffer), fill it, flip again. Assert the buffer received back from flip has non-zero capacity (allocations retained, not dropped).
- [x] `test_double_buffer_seqno_monotonic` — flip 100 times, consume every 3rd. Assert `has_new()` is true when behind and false when caught up.
- [x] `test_double_buffer_is_send_sync` — static assertion: `fn assert_send_sync<T: Send + Sync>() {} assert_send_sync::<SnapshotDoubleBuffer>();`. Required because both IO thread and main thread hold a clone.

**File:** `oriterm_mux/src/pane/io_thread/tests.rs` (extend)

- [x] `test_produce_snapshot_fills_cells` — create IO thread with a `Term` containing "hello" on line 0. Call `produce_snapshot()`. Assert the snapshot buffer has cells matching "hello".
- [x] `test_produce_snapshot_resets_damage` — mark a line dirty, call `produce_snapshot()`. Assert `terminal.damage()` is cleared after production (damage was consumed by the snapshot).
- [x] `test_produce_snapshot_respects_sync_mode` — enable Mode 2026 (synchronized output), send bytes. Assert `produce_snapshot()` is NOT called while sync_bytes_count > 0 (the main loop gates on this).
- [x] `test_produce_snapshot_wakeup_only_when_dirty` — process bytes that don't set `grid_dirty`, call the snapshot path. Assert the wakeup callback was NOT invoked (no spurious wakeups).

- [x] `/tpr-review` checkpoint

---

## 03.3 Wakeup After Publish

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm_mux/src/mux_event/mod.rs`

Ensure the main thread is woken after a new snapshot is published. The wakeup must come AFTER the snapshot is in the shared buffer — not during VTE parsing like today.

- [x] The IO thread calls `(self.wakeup)()` after `produce_snapshot()` — this is the same `Arc<dyn Fn() + Send + Sync>` wakeup callback used by the current system.

- [x] The wakeup callback should trigger the same `TermEvent::MuxWakeup` path that exists today. The main thread then calls `pump_mux_events()` → sees dirty pane → renders.

- [x] Coalescing: the IO thread may process multiple byte batches and commands in one cycle before producing a single snapshot. The wakeup should be sent at most once per snapshot production, not per byte batch. Use an atomic flag (same pattern as `wakeup_pending` in `MuxEventProxy`).

- [x] **Wire render wakeup timing.** The `IoThreadEventProxy` (created in section 02.3) already suppresses all events except `grid_dirty`. This section wires the wakeup timing: the IO thread sends a render wakeup AFTER publishing a snapshot (not during VTE parsing). The wakeup callback is the same `Arc<dyn Fn() + Send + Sync>` the current system uses.

  **Render wakeups** (`Event::Wakeup`): The `IoThreadEventProxy` holds its own `grid_dirty: Arc<AtomicBool>` (shared with `PaneIoThread`). On `Wakeup`, it sets `grid_dirty` but does NOT call the wakeup callback. The IO thread checks `grid_dirty` after `produce_snapshot()` and fires the wakeup itself. Note: `MuxEventProxy.grid_dirty` is private, so `IoThreadEventProxy` cannot delegate to `inner.grid_dirty` — it must hold its own Arc.

  **Metadata events** (Title, Bell, CWD, PtyWrite, Clipboard, etc.): During dual-Term (sections 02-06), suppressed by `IoThreadEventProxy` to avoid duplicates. The old `Term`'s `MuxEventProxy` handles them. After section 07 removes the old `Term`, `suppress_metadata` is flipped to `false` and the IO thread's proxy becomes the sole event source.

  **Important**: `Event::PtyWrite` (DA responses, DECRPM replies) is critical for terminal protocol correctness. During dual-Term, the old `MuxEventProxy` on the old `Term` handles these. After section 07, `IoThreadEventProxy` wraps a `MuxEventProxy` that already has the `mpsc::Sender<MuxEvent>` channel, so `PtyWrite` events flow through `inner.send_event()` → `MuxEvent::PtyWrite` → main thread → PTY writer automatically.

- [x] After `produce_snapshot()`, check `grid_dirty` and call the render wakeup:
  ```rust
  fn produce_snapshot(&mut self) {
      // ... (snapshot production) ...
      // Send render wakeup after snapshot is published.
      if self.grid_dirty.swap(false, Ordering::AcqRel) {
          (self.wakeup)();
      }
  }
  ```

---

## 03.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-03-001][medium]` `oriterm_mux/src/pane/io_thread/mod.rs:75`, `oriterm_mux/src/pane/io_thread/mod.rs:87`, `oriterm_mux/src/pane/io_thread/mod.rs:207` — the blocking receive path can drop the final parsed frame on shutdown.
  Evidence: when the idle `select!` arm receives PTY bytes, it only calls `handle_bytes_chunked(&bytes)` and then ends the loop iteration. Snapshot publication happens later via `maybe_produce_snapshot()` at the top of the next iteration. If `Shutdown` is queued while that byte batch is being parsed, the next iteration exits in `drain_commands()` before `maybe_produce_snapshot()` runs, so the bytes that were just parsed never get flipped into `SnapshotDoubleBuffer` and no post-publish wakeup is sent.
  **Fix:** Added `self.maybe_produce_snapshot()` before `return` in both the `drain_commands()` shutdown check and the `select!` Shutdown arm. Test: `shutdown_flushes_final_snapshot`.
- [x] `[TPR-03-002][low]` `plans/threaded-io/section-03-snapshot-production.md:34`, `plans/threaded-io/section-03-snapshot-production.md:37`, `plans/threaded-io/section-04-render-migration.md:55`, `plans/threaded-io/section-04-render-migration.md:59` — Section 03 is marked complete with "main thread reads the latest published snapshot" even though that consumer path is still deferred to Section 04.
  Evidence: the current tree adds `PaneIoHandle::double_buffer()` but no `Pane`/`EmbeddedMux`/daemon caller reads it. Section 04 still owns the first real integration steps (`Pane::swap_io_snapshot(...)` and the `refresh_pane_snapshot()` rewrite). That makes the Section 03 status/body overstate what is actually finished today.
  **Fix:** Updated exit criteria to clarify that `swap_front()` is available but no production consumer reads it yet — section 04 wires the consumer.

---

## 03.N Completion Checklist

- [x] `SnapshotDoubleBuffer` type provides `flip()` and `swap_front()`
- [x] IO thread produces snapshots after processing bytes/commands
- [x] Buffer reuse: `RenderableContent` allocations recycled via swap
- [x] Synchronized output mode respected (no snapshot during sync buffer)
- [x] Wakeup sent to main thread after snapshot publish (not during parsing)
- [x] Non-wakeup events (title, bell, CWD) still reach main thread
- [x] `timeout 150 cargo test -p oriterm_mux` passes
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `/tpr-review` passed

**Exit Criteria:** The IO thread produces valid `RenderableContent` snapshots and publishes them to `SnapshotDoubleBuffer` via `flip_swap()`. The `swap_front()` API is available for the main thread but no production consumer reads it yet — section 04 wires `Pane::swap_io_snapshot()` and the `refresh_pane_snapshot()` rewrite. Wakeups are correctly timed (after publish). The existing render path (via `Arc<FairMutex>`) still works — this section doesn't switch the render path.
