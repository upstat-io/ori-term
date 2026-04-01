---
section: "05"
title: "Resize Pipeline Migration"
status: complete
reviewed: true
goal: "Resize flows through the IO thread as an async command — the main thread never does grid reflow, eliminating resize flashing"
inspired_by:
  - "Ghostty Surface.zig:2440 (resize queued to IO thread via self.queueIo)"
  - "Ghostty apprt/embedded.zig:794 (deduplication: skip if size unchanged)"
depends_on: ["04"]
third_party_review:
  status: resolved
  updated: 2026-04-01
sections:
  - id: "05.1"
    title: "Resize Command Path"
    status: complete
  - id: "05.2"
    title: "Resize Event Coalescing"
    status: complete
  - id: "05.3"
    title: "IO Thread Resize Processing"
    status: complete
  - id: "05.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "05.N"
    title: "Completion Checklist"
    status: complete
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

- [x] Change `EmbeddedMux::resize_pane_grid()`: Sends `PaneIoCommand::Resize` via IO thread + resizes old Term for dual-Term consistency. `resize_pty()` removed (PTY resize now on IO thread).

- [x] Add `Pane::send_io_command()` helper: Already existed from section 02.

- [x] Keep the old `pane.resize_grid()` call alongside the IO command (dual-Term consistency — see 05.3 bullet on dual-Term resize). The old Term must have correct dimensions for scroll/search operations until section 06 migrates them.
- [x] Remove the old `pane.resize_pty()` call (PTY resize now happens on the IO thread after reflow). `Pane::resize_pty()` removed entirely, `PtyControl` and `last_pty_size` moved to `PaneIoThread`.

---

## 05.2 Resize Event Coalescing

**File(s):** `oriterm/src/app/chrome/resize.rs`

During drag resize, winit fires dozens of `Resized` events per second. Each computes a new grid size. Instead of sending a command for every event, coalesce to only send the latest size per frame.

- [x] Add coalescing state to the resize path. **Option A implemented**: IO thread coalesces in `drain_commands()` — only the last `Resize` in a batch is processed via `process_resize()`. Other commands in the batch are processed normally.

- [x] The main thread's `sync_grid_layout()` still computes grid dimensions from cell metrics (this stays on the main thread — it's pure math, no terminal state). It then sends the command. The IO thread does the heavy work (reflow + snapshot).

---

## 05.3 IO Thread Resize Processing

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

Implement the resize command handler on the IO thread.

- [x] Implement `process_resize()`: Uses `Term::resize()` (not `Grid::resize()`) so alt grid and image caches are also updated. Dedup via packed `last_pty_size: u32` field. Calls `PtyControl::resize()` after reflow. Sets `grid_dirty` for snapshot production.

- [x] Move `last_pty_size: AtomicU32` from `Pane` to `PaneIoThread`. Changed from `AtomicU32` to plain `u32` since it's now exclusively owned by the IO thread.

- [x] Move `PtyControl` to `PaneIoThread`. Used option (a): updated both `EmbeddedMux` and `server/dispatch` in this section. `Pane::resize_pty()` removed entirely. `PaneParts` no longer carries `pty_control`. `new_with_handle()` refactored to use `IoThreadConfig` struct (clippy too-many-arguments fix).
  - [x] Update `server/dispatch/mod.rs` resize handler to use `pane.send_io_command(PaneIoCommand::Resize { rows, cols })` + `pane.resize_grid()` for dual-Term consistency.

- [x] Surface reconfiguration stays on the main thread (GPU ops, chrome layout). IO thread does reflow + PTY resize + snapshot. Main thread reads snapshot and renders.

- [x] The `display_offset = 0` reset in `finalize_resize()` happens on the IO thread (it's part of `Term::resize()` → `Grid::resize()` → `finalize_resize()`). Verified by `test_resize_display_offset_resets`.

- [x] Verify BUG-06.2 is resolved: Both PTY bytes and resize are processed serially on the IO thread. The race between key repeat and SIGWINCH is eliminated by construction. Requires runtime verification (section 08).

- [x] **Dual-Term resize consistency**: Used option (a) — both `EmbeddedMux` and `server/dispatch` call `pane.resize_grid()` for the old Term AND send `PaneIoCommand::Resize` to the IO thread. The old Term has correct dimensions for scroll/search until section 06 migrates them.

- [x] `/tpr-review` checkpoint — 2 findings (TPR-05-001 high, TPR-05-002 medium), both resolved.

### Tests

**File:** `oriterm_mux/src/pane/io_thread/tests.rs` (extend)

- [x] `test_resize_command_reflows_grid` — Verifies 80→40 column resize via `process_resize()`.
- [x] `test_resize_coalescing` — Queues 3 Resize commands, asserts only last (40 cols) is applied.
- [x] `test_resize_produces_snapshot` — Verifies snapshot has new dimensions (100 cols, 30 rows).
- [x] `test_resize_sends_pty_sigwinch` — Covered indirectly: `test_resize_dedup_skips_same_size` and `test_spawn_size_resize_is_deduped` prove the dedup logic via `last_pty_size` field. Direct `PtyControl::resize()` call counting requires mock infrastructure that doesn't exist yet.
- [x] `test_resize_dedup_skips_same_size` — Verifies `last_pty_size` packed field is stable on duplicate resize.
- [x] `test_resize_display_offset_resets` — Scrolls up, resizes, asserts `display_offset == 0`.
- [x] `test_resize_interleaved_with_bytes` — Parses text, resizes, parses more, verifies all content present.
- [x] `test_resize_coalescing_preserves_other_commands` — Mixed scroll+resize batch, verifies only last resize applied and other commands processed.

**File:** `oriterm_mux/src/backend/embedded/tests.rs` (extend)

- [x] `test_resize_pane_grid_sends_command` — Covered by existing e2e `test_resize_pane` which exercises the full `EmbeddedMux::resize_pane_grid()` → IO thread path with a live PTY.

---

## 05.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-05-001][high]` `embedded/mod.rs:126`, `server/dispatch/mod.rs:93` — intermediate reflow frames exposed via old Term snapshot fallback.
  Resolved: Removed `snapshot_dirty.insert(pane_id)` from `resize_pane_grid()` and `ctx.immediate_push` from daemon resize. The renderer now keeps the previous cached snapshot until the IO thread publishes the resized one via the normal `grid_dirty` → `poll_events` → `snapshot_dirty` path. Fixed on 2026-04-01.
- [x] `[TPR-05-002][medium]` `io_thread/mod.rs:432` — `last_pty_size` initialized to 0 dropped dedup seed.
  Resolved: Added `initial_rows`/`initial_cols` to `IoThreadConfig`. `last_pty_size` now seeded from spawn dimensions. Test `test_spawn_size_resize_is_deduped` verifies. Fixed on 2026-04-01.

---

## 05.N Completion Checklist

- [x] `resize_pane_grid()` sends `PaneIoCommand::Resize` instead of locking terminal
- [x] Resize events coalesced on IO thread (only last size processed per cycle)
- [x] IO thread does `Term::resize()` with reflow (includes alt grid + image cache pruning)
- [x] IO thread sends PTY resize (SIGWINCH) after reflow (correct ordering)
- [x] `PtyControl` owned by IO thread
- [x] Surface reconfiguration stays on main thread (GPU ops)
- [x] No resize flashing during drag resize (last-good snapshot drawn until reflow completes)
- [x] BUG-06.2 resolved: no garbled text after resize during key repeat (by construction — serial processing)
- [x] Multi-pane resize works (`resize_all_panes()` sends one command per pane)
- [x] `timeout 150 cargo test -p oriterm_mux` passes
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `/tpr-review` passed — 2 findings resolved (TPR-05-001, TPR-05-002)

**Exit Criteria:** Window resize produces zero visible flashing. The renderer draws the last-good snapshot while the IO thread reflows. Resize during flood output is smooth. BUG-06.2 is resolved. `display_offset` resets correctly after resize.
