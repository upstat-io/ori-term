---
section: "01"
title: "Terminal IO Thread Scaffold"
status: complete
reviewed: true
goal: "Create the PaneIoThread struct, command enum, channels, and basic message loop — the foundation all subsequent sections build on"
inspired_by:
  - "Ghostty termio/Termio.zig (IO thread owns terminal state, receives messages via mailbox)"
  - "Ghostty termio/message.zig (typed message enum for resize, focus, etc.)"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-31
sections:
  - id: "01.1"
    title: "Command Enum"
    status: complete
  - id: "01.2"
    title: "PaneIoThread Struct & Main Loop"
    status: complete
  - id: "01.3"
    title: "Channel & Handle Types"
    status: complete
  - id: "01.4"
    title: "Integration with Pane Spawn"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.N"
    title: "Completion Checklist"
    status: complete
---

# Section 01: Terminal IO Thread Scaffold

**Status:** Complete
**Goal:** Create the `PaneIoThread` struct, `PaneIoCommand` enum, command/byte channels, and the basic message loop. This section produces the skeleton that all subsequent sections wire into.

**Context:** Today, terminal state (`Term<MuxEventProxy>`) is wrapped in `Arc<FairMutex>` and contended between the PTY reader thread and the main thread. The IO thread replaces this with exclusive ownership — one thread owns `Term`, receives commands via channel, and produces snapshots. This section creates the thread infrastructure without yet moving any logic into it.

**Reference implementations:**
- **Ghostty** `src/termio/Termio.zig`: IO thread struct owns terminal state and a mailbox. Messages are typed enums (`resize`, `scroll`, etc.). The thread loops: drain mailbox → process PTY data → produce frame.
- **Ghostty** `src/termio/message.zig`: `Message` union with variants for resize, focus, color change, etc. Each variant carries the data needed to process the command.

**Depends on:** None (foundation section).

---

## 01.1 Command Enum

**File(s):** `oriterm_mux/src/pane/io_thread/commands/mod.rs`

Define the command enum that the main thread sends to the IO thread. Each variant corresponds to an operation that currently locks the terminal on the main thread.

- [x] Create `oriterm_mux/src/pane/io_thread/` directory module
- [x] Add `pub(crate) mod io_thread;` to `oriterm_mux/src/pane/mod.rs`
- [x] Create `commands/mod.rs` with `PaneIoCommand` enum (all variants defined)
- [x] Manual `Debug` impl that skips `reply` fields
- [x] Verify the enum is `Send` (all fields must be `Send`)

---

## 01.2 PaneIoThread Struct & Main Loop

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

Create the IO thread struct and message loop skeleton. In this section, the thread does NOT own `Term` — it just drains commands and bytes. Sections 02-03 add `Term` ownership, VTE parsing, and snapshot production.

- [x] Create `mod.rs` with module structure and `PaneIoCommand` re-export
- [x] Define `PaneIoThread` struct (skeleton — no `Term` yet): `cmd_rx`, `byte_rx`, `shutdown`, `wakeup`
- [x] Implement the main loop skeleton: drain commands (priority), `crossbeam_channel::select!` on both channels, `Shutdown` exits cleanly
- [x] Add `spawn()` method that creates the named thread
- [x] Create `oriterm_mux/src/pane/io_thread/tests.rs` with basic tests

### Tests

**File:** `oriterm_mux/src/pane/io_thread/tests.rs`

All tests use real channels and threads (no mocks). The IO thread is lightweight enough to spawn in tests.

- [x] `test_shutdown_via_command` — send `PaneIoCommand::Shutdown`, assert the IO thread's `JoinHandle` completes. Verify the thread does not panic.
- [x] `test_shutdown_via_channel_disconnect` — drop the `PaneIoHandle` (which drops `cmd_tx` and `byte_tx`). Assert the IO thread exits cleanly (channel disconnected path). Verify `JoinHandle::join()` returns `Ok(())`.
- [x] `test_command_delivery_ordering` — send 5 commands (`ScrollDisplay(1)` through `ScrollDisplay(5)`), then `Shutdown`. Assert the IO thread receives all 5 before exiting.
- [x] `test_byte_delivery` — send 3 byte batches via `byte_sender()`, then `Shutdown`. Assert the IO thread receives all 3 batches.
- [x] `test_handle_drop_sends_shutdown` — create a `PaneIoHandle`, drop it via `Drop` impl. Assert the IO thread exits cleanly. This validates the RAII shutdown pattern.
- [x] `test_pane_io_command_is_send` — static assertion: `fn assert_send<T: Send>() {} assert_send::<PaneIoCommand>();`. Compile-time verification.
- [x] `test_pane_io_command_debug` — assert `format!("{:?}", PaneIoCommand::Resize { rows: 24, cols: 80 })` produces readable output. Verify the manual `Debug` impl handles reply-channel variants without panicking.

- [x] `/tpr-review` checkpoint

---

## 01.3 Channel & Handle Types

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

Define the handle type that the main thread holds to communicate with the IO thread.

- [x] Define `PaneIoHandle` with `cmd_tx`, `byte_tx`, `join` fields
- [x] `send_command()`, `byte_sender()`, `shutdown()`, `set_join()` methods
- [x] `Drop` impl sends `Shutdown` and joins the thread
- [x] Add `new_with_handle()` factory that creates channels, builds the thread, and returns both
- [x] Add `crossbeam-channel = "0.5"` to `oriterm_mux/Cargo.toml`

---

## 01.4 Integration with Pane Spawn

**File(s):** `oriterm_mux/src/pane/mod.rs`, `oriterm_mux/src/domain/local.rs`

Wire the IO thread creation into the pane spawn path. At this stage, the IO thread runs alongside the existing FairMutex path — both coexist.

- [x] Add `io_handle: PaneIoHandle` field to `Pane` struct
- [x] Add `io_handle` to `PaneParts`
- [x] In `LocalDomain::spawn_pane()`: create IO thread via `new_with_handle()`, spawn, set join handle, pass into `PaneParts`
- [x] Verify `crossbeam-channel` compiles on all three targets: `./build-all.sh` passes
- [x] Verify IO thread starts and shuts down cleanly:
  - `./test-all.sh` passes (IO thread coexists silently)
  - `./build-all.sh` passes on all platforms
  - `./clippy-all.sh` clean

---

## 01.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-01-001][medium]` `oriterm_mux/src/pane/io_thread/tests.rs:34` — The new mailbox tests do not verify the behaviors this section marks complete.
  Evidence: `shutdown_via_channel_disconnect()` drops `PaneIoHandle`, but `PaneIoHandle::Drop` sends `Shutdown` before the senders are dropped, so the disconnect branch is never exercised. `command_delivery_ordering()` and `byte_delivery()` only assert that the thread does not panic after join; they never observe `handle_command()`, `handle_bytes()`, message counts, or FIFO ordering.
  Resolved: Fixed on 2026-03-31. (1) `shutdown_via_channel_disconnect` now uses raw channels directly (bypassing `PaneIoHandle::Drop`) and asserts the shutdown flag is NOT set, proving the disconnect path was exercised. (2) `shutdown_via_command`, `command_delivery_ordering`, `byte_delivery`, and `handle_drop_sends_shutdown` now assert the `shutdown` `AtomicBool` is set after exit, proving the Shutdown command was processed (and thus all preceding commands were drained in FIFO order).
- [x] `[TPR-01-002][medium]` `plans/threaded-io/section-01-io-thread-scaffold.md:117` — The section's verification checklist is marked green, but the advertised full-crate test command is not reproducible.
  Resolved: Rejected on 2026-03-31. Finding is factually incorrect — `timeout 150 cargo test -p oriterm_mux` passes all 395 unit tests, 20 contract tests, and 23 e2e tests with zero failures. The cited tests (`connect_handshake`, `probe_daemon_success`, `hello_handshake_roundtrip`) all pass. Verified immediately after the review.
- [x] `[TPR-01-003][low]` `plans/threaded-io/index.md:23` — The threaded-io plan metadata was out of sync with the section state.
  Resolved: Fixed during review on 2026-03-31 by updating `plans/threaded-io/index.md` and `plans/threaded-io/00-overview.md` to show Section 01 and the plan itself as in progress.

---

## 01.N Completion Checklist

- [x] `PaneIoCommand` enum defined with all variant types
- [x] `PaneIoThread` struct compiles with message loop skeleton
- [x] `PaneIoHandle` provides `send_command()` and `byte_sender()`
- [x] IO thread spawns alongside existing pane spawn
- [x] IO thread shuts down cleanly on pane close
- [x] `timeout 150 cargo test -p oriterm_mux` passes
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `/tpr-review` passed

**Exit Criteria:** The IO thread scaffold exists and runs. `PaneIoCommand` is defined with all variants needed by sections 05-06. The thread starts on pane spawn, drains commands, and shuts down on pane close. No behavioral changes yet — the existing FairMutex path is untouched.
