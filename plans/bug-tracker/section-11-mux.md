---
section: 11
title: "Mux & Pane I/O"
domain: "oriterm_mux — pane server, PTY I/O, IO thread, pane lifecycle, backpressure"
status: in-progress
---

# Section 11: Mux & Pane I/O

Bugs in the pane multiplexer — PTY I/O, IO thread behavior, pane lifecycle, memory management.

## Open Bugs

- [ ] `[BUG-11-1][high]` **Ctrl+C unresponsive during sustained output flooding across multiple panes** — found by manual.
  Repro: Open 2+ panes. Run `yes` or `cat /dev/urandom` in each to flood output. Press Ctrl+C — it fails to interrupt the processes. Single-pane flooding may still respond; the issue worsens with multiple simultaneous floods.
  Subsystem: `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm/src/app/event_loop.rs`
  Analysis: PTY input path is: winit keyboard event → main thread dispatch → `PaneNotifier::notify()` → writer thread → PTY fd. During multi-pane flooding, the main thread may be starved by constant snapshot consumption, redraw requests, and frame rendering for multiple dirty panes. Keyboard events from winit queue up but aren't dispatched to the PTY fast enough. Alternatively, the OS-level PTY write buffer may be full if the child process is too busy writing output to read its input fd. The IO thread itself has a 64KB parse chunk limit with command draining between chunks, but this doesn't help if the bottleneck is upstream (main thread) or downstream (OS PTY buffer).
  Found: 2026-04-01 | Source: manual
  Note: Roadmap section 23 (performance) and section 50 (runtime efficiency) touch this area.

- [ ] `[BUG-11-2][high]` **Memory (RSS) grows during output flooding and does not decrease after killing panes** — found by manual.
  Repro: Open multiple panes. Flood each with sustained output (e.g., `yes`, `cat /dev/urandom`). Observe RSS climbing. Kill the panes (close tabs). RSS does not decrease.
  Subsystem: `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm_mux/src/pane/io_thread/snapshot/mod.rs`, `oriterm_core/src/grid/`
  Analysis: During flooding, scrollback grows up to `max_scrollback` and the grid allocates rows. When a pane is killed, `Pane::drop()` → `PaneIoHandle::drop()` shuts down the IO thread, and `Term<T>` drop should free grid memory. Possible causes: (1) System allocator on Windows doesn't eagerly return freed pages to OS (RSS stays high even after dealloc — common with large allocations). (2) Snapshot double buffer retains large capacity allocations that aren't shrunk. (3) GPU-side buffers (instance writers, atlas entries) for killed panes aren't cleaned up. (4) Genuine leak — something holds an Arc or reference to pane data after removal.
  Found: 2026-04-01 | Source: manual
  Note: Roadmap section 50 (runtime efficiency) covers memory discipline.

## Resolved Bugs

(none yet)
