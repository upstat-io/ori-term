---
section: "08"
title: "Verification"
status: not-started
reviewed: true
goal: "Verify performance invariants, cross-platform correctness, and regression-free operation of the threaded IO architecture"
inspired_by:
  - "ori_term alloc_regression.rs (zero-allocation enforcement)"
  - "ori_term event_loop_helpers/tests.rs (idle CPU verification)"
depends_on: ["07"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "08.1"
    title: "Performance Invariants"
    status: not-started
  - id: "08.2"
    title: "Resize Quality Verification"
    status: not-started
  - id: "08.3"
    title: "Cross-Platform Testing"
    status: not-started
  - id: "08.4"
    title: "Threading Stress Tests"
    status: not-started
  - id: "08.5"
    title: "Build & Verify"
    status: not-started
  - id: "08.6"
    title: "Documentation"
    status: not-started
  - id: "08.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "08.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 08: Verification

**Status:** Not Started
**Goal:** Comprehensive verification that the threaded IO architecture preserves all performance invariants, works correctly across platforms, and eliminates the resize flashing that motivated this plan.

**Context:** This plan changes the fundamental synchronization model of the terminal. Every performance invariant must be re-verified. Cross-platform behavior (especially Windows ConPTY) must be tested. The resize quality improvement must be confirmed visually.

**Depends on:** Section 07 (full migration complete).

---

## 08.1 Performance Invariants

**File(s):** `oriterm_core/tests/alloc_regression.rs`, `oriterm/src/app/event_loop_helpers/tests.rs`

- [ ] **Zero idle CPU beyond cursor blink**: Verify `compute_control_flow()` still returns `Wait` when idle. The IO thread should block on channel recv when there's no PTY data or commands — not spinning.
  - Check: `cargo test -p oriterm --test architecture` (if control flow tests exist there)
  - Check: IO thread uses `crossbeam_channel::select!` (blocking), not `try_recv()` in a spin loop

- [ ] **Zero allocations in hot render path**: The render path now reads from `SnapshotDoubleBuffer::swap_front()` instead of `build_snapshot_into()`. Verify:
  - `swap_front()` exchanges `RenderableContent` buffers — the main thread reuses Vec allocations, no copy or allocation
  - `swap_renderable_content()` still does `std::mem::swap` — zero allocation
  - Run `alloc_regression` test: `timeout 150 cargo test -p oriterm_core --test alloc_regression`
  - The alloc regression test currently calls `renderable_content_into()` directly on `Term`. It may need updating to test the snapshot transfer path instead, or kept as-is to verify the IO thread's internal snapshot production.

- [ ] **Stable RSS under sustained output**: The IO thread reuses its `snapshot_buf` via the publish/reclaim swap. The main thread reuses its `renderable_cache` via `swap_renderable_content`. No new unbounded growth vectors.

- [ ] **Buffer shrink discipline**: `maybe_shrink()` must be called on the IO thread's snapshot buffer and on the main thread's render cache. Verify both paths call `RenderableContent::maybe_shrink()` after use.

- [ ] **IO thread is not a CPU hog**: During sustained PTY output, the IO thread processes bytes and produces snapshots. It should yield between cycles to give the main thread CPU time. Verify the loop structure includes blocking waits (not busy-polling).
  - **Concrete check**: When no PTY output or commands for 5 seconds, the IO thread must be blocked in `crossbeam_channel::select!` (sleeping, not spinning). Verify by checking that CPU usage of the `terminal-io` thread is <0.1% during idle. On Linux, read `/proc/<pid>/task/<tid>/stat` for the IO thread.

- [ ] **alloc_regression test update**: The existing `alloc_regression.rs` test calls `Term::renderable_content_into()` directly. After the threaded IO migration, the hot render path is `SnapshotDoubleBuffer::swap_front()` + `EmbeddedMux::swap_renderable_content()`. Add a companion test (or update the existing one) that verifies the full IO-thread-to-render path is zero-allocation after warmup. Use `#[global_allocator]` counting allocator pattern.

---

## 08.2 Resize Quality Verification

Manual and automated verification that resize flashing is eliminated.

- [ ] **Visual test — drag resize**: Open ori_term, run `ls -la /usr/bin` (fill screen with text), drag-resize the window width. Verify:
  - No visible text reflowing frame-by-frame
  - No cursor jumping to unexpected positions
  - No blank/garbled frames during resize
  - Text transitions smoothly from old layout to new layout

- [ ] **BUG-06.2 verification**: Hold a key to fill the screen with text, release, resize. Verify no garbled characters appear after resize.

- [ ] **Rapid resize test**: Programmatically resize the window 50 times in rapid succession (e.g. via xdotool or Win32 API). Verify:
  - Terminal settles to correct final dimensions
  - No orphaned resize commands in the channel
  - PTY reports correct final size

- [ ] **Resize during flood output**: Run `yes` in the terminal, resize while output is streaming. Verify:
  - No deadlock or hang
  - Terminal settles to correct dimensions after `yes` is killed
  - No excessive memory growth

- [ ] **Multi-pane resize**: With 2+ split panes, resize the window. Verify all panes resize correctly.

---

## 08.3 Cross-Platform Testing

- [ ] **Windows (ConPTY)**:
  - Build: `cargo build --target x86_64-pc-windows-gnu`
  - Test ConPTY resize: `ResizePseudoConsole` called correctly from IO thread
  - Verify no `WINDOW_BUFFER_SIZE_EVENT` spam (dedup via `last_pty_size`)
  - PowerShell prompt not lost on startup (the original dedup motivation)

- [ ] **Linux (PTY/ioctl)**:
  - Build: `cargo build`
  - Test `ioctl(TIOCSWINSZ)` from IO thread
  - SIGWINCH delivered correctly

- [ ] **macOS (PTY)**:
  - Build: verify cross-compilation
  - `pty_control.resize()` works from IO thread

- [ ] **All platforms**: Verify `crossbeam-channel` works correctly on all three platforms. No platform-specific channel behavior issues.

---

## 08.4 Threading Stress Tests

**File(s):** `oriterm_mux/tests/io_thread_stress.rs` (new integration test file)

- [ ] `test_concurrent_resize_and_pty_output` — spawn a pane, write flood output from a thread, send 100 resize commands from another thread. Assert:
  - No panic (JoinHandle returns Ok for all threads)
  - Final grid dimensions match the last resize command
  - Snapshot cells are valid (no zero-width chars in non-empty cells)
  - IO thread exits cleanly after shutdown

- [ ] `test_pane_close_during_flood_output` — spawn a pane writing continuous output, close it mid-stream via `PaneIoCommand::Shutdown`. Assert:
  - IO thread join completes within 2 seconds
  - PTY reader thread exits (channel disconnect)
  - Thread count returns to pre-spawn level (no leaked threads)

- [ ] `test_multiple_panes_concurrent_resize` — spawn 3 panes, each with active output. Send resize to all 3 simultaneously. Assert:
  - All panes reach the target dimensions
  - No cross-pane state corruption (each pane's snapshot has its own content)
  - All 3 IO threads shut down on exit

- [ ] `test_command_channel_flood` — send 1000 `MarkAllDirty` commands rapidly. Assert:
  - IO thread drains all 1000 (verify with a counter)
  - No channel disconnection (unbounded channel doesn't block sender)
  - Post-drain, IO thread is idle and responsive to new commands

- [ ] `test_snapshot_swap_under_contention` — one thread flips snapshots rapidly (simulating IO thread), another thread calls `swap_front()` rapidly (simulating main thread). Run for 1 second. Assert no panic, no data corruption, seqno is monotonically increasing.

- [ ] `test_io_thread_panic_does_not_crash_app` — inject a panic in the IO thread (e.g., via a specially crafted command). Assert:
  - `PaneIoHandle::shutdown()` returns without hanging (JoinHandle catches the panic)
  - The main thread can still close the pane cleanly
  - No other panes are affected

---

## 08.5 Build & Verify

- [ ] `./build-all.sh` green (all platforms)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `./test-all.sh` green (all tests pass)
- [ ] Architecture tests pass: `timeout 150 cargo test -p oriterm --test architecture`
- [ ] E2E tests pass: `timeout 150 cargo test -p oriterm_mux --test e2e`
- [ ] Contract tests pass: `timeout 150 cargo test -p oriterm_mux --test contract`

---

## 08.6 Documentation

- [ ] Update CLAUDE.md:
  - Update "Key Paths" section to document `oriterm_mux/src/pane/io_thread/`
  - Update architecture description to mention IO thread model
  - Update "Performance Invariants" if any changed
  - Add IO thread to the threading model description

- [ ] Update `.claude/rules/crate-boundaries.md` if the mux crate boundary changed

- [ ] Update `plans/bug-tracker/section-06-rendering-perf.md`:
  - Mark BUG-06.2 as resolved
  - Add note about resize flashing fix

---

## 08.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 08.N Completion Checklist

- [ ] Zero idle CPU verified (control flow tests pass)
- [ ] Zero allocation in render path verified (alloc regression test passes)
- [ ] Stable RSS verified under sustained output
- [ ] Buffer shrink discipline verified on both IO and main threads
- [ ] Resize flashing eliminated (visual verification)
- [ ] BUG-06.2 resolved (no garbled text after resize during key repeat)
- [ ] Rapid resize stress test passes
- [ ] Flood output + resize stress test passes
- [ ] Pane close during processing — no leaked threads
- [ ] Multi-pane concurrent — no cross-pane corruption
- [ ] Windows ConPTY works correctly
- [ ] Linux PTY works correctly
- [ ] macOS PTY compiles and works
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] CLAUDE.md updated with IO thread documentation
- [ ] Bug tracker updated (BUG-06.2 resolved)
- [ ] `/tpr-review` passed clean

**Exit Criteria:** All performance invariants hold. Resize is visually smooth. No regression in any existing test. Cross-platform builds clean. BUG-06.2 is resolved. The threaded IO architecture is documented.
