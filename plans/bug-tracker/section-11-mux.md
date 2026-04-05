---
section: 11
title: "Mux & Pane I/O"
domain: "oriterm_mux — pane server, PTY I/O, IO thread, pane lifecycle, backpressure"
status: in-progress
---

# Section 11: Mux & Pane I/O

Bugs in the pane multiplexer — PTY I/O, IO thread behavior, pane lifecycle, memory management.

## Open Bugs

(none)

- [ ] `[BUG-11-2][high]` **Memory (RSS) grows during output flooding and does not decrease after killing panes** — found by manual.
  Repro: Open multiple panes. Flood each with sustained output (e.g., `yes`, `cat /dev/urandom`). Observe RSS climbing. Kill the panes (close tabs). RSS does not decrease.
  Subsystem: `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm_mux/src/pane/io_thread/snapshot/mod.rs`, `oriterm_core/src/grid/`
  Analysis: During flooding, scrollback grows up to `max_scrollback` and the grid allocates rows. When a pane is killed, `Pane::drop()` → `PaneIoHandle::drop()` shuts down the IO thread, and `Term<T>` drop should free grid memory. Possible causes: (1) System allocator on Windows doesn't eagerly return freed pages to OS (RSS stays high even after dealloc — common with large allocations). (2) Snapshot double buffer retains large capacity allocations that aren't shrunk. (3) GPU-side buffers (instance writers, atlas entries) for killed panes aren't cleaned up. (4) Genuine leak — something holds an Arc or reference to pane data after removal.
  Found: 2026-04-01 | Source: manual
  Note: Roadmap section 50 (runtime efficiency) covers memory discipline.

## Resolved Bugs

- [x] `[BUG-11-1][critical]` **All input blocked during sustained output flooding (even single pane)** — found by manual.
  Resolved: 2026-04-05. Two-part fix: (1) PTY writer thread now sets a `write_stalled` AtomicBool flag before potentially-blocking `write()` calls — the main thread reads this to detect when the writer is stuck on a full kernel buffer. (2) When Ctrl+C is pressed and the writer is stalled, SIGINT is sent directly to the child process group via `kill(-pid, SIGINT)` on Unix / `GenerateConsoleCtrlEvent` on Windows, bypassing the blocked PTY pipe. Writer thread also improved: coalesces pending input, uses `recv_timeout` instead of blocking `recv`, and flushes pending data before shutdown. Files: `oriterm_mux/src/pty/mod.rs` (writer thread), `oriterm_mux/src/pane/mod.rs` (Signal enum, signal delivery), `oriterm_mux/src/backend/embedded/mod.rs` + `mod.rs` (MuxBackend trait), `oriterm/src/app/keyboard_input/mod.rs` (Ctrl+C bypass).
