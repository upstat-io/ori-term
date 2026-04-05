---
section: 11
title: "Mux & Pane I/O"
domain: "oriterm_mux — pane server, PTY I/O, IO thread, pane lifecycle, backpressure"
status: in-progress
---

# Section 11: Mux & Pane I/O

Bugs in the pane multiplexer — PTY I/O, IO thread behavior, pane lifecycle, memory management.

## Open Bugs

- [ ] `[BUG-11-1][critical]` **All input blocked during sustained output flooding (even single pane)** — found by manual.
  Repro: Run a flood script (e.g., `yes`, `cat /dev/urandom`, or any rapid-output loop) in a single pane. Attempt Ctrl+C or any keyboard input — nothing is accepted. All forms of input are blocked for the duration of the flood. Worsens with multiple panes but occurs with just one.
  Subsystem: `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm/src/app/event_loop.rs`, `oriterm/src/app/keyboard_input/mod.rs`, `oriterm/src/app/pane_accessors.rs`
  Analysis: Input path is winit keyboard event → `handle_keyboard_input()` → `encode_key_to_pty()` → `write_pane_input()` → `notifier.notify()` → PTY writer thread → `write()` syscall. During flooding, multiple bottlenecks compound: (1) The PTY writer thread blocks on `write()` when the kernel PTY buffer fills — the child process is too busy writing output to drain its input fd, so keyboard bytes queue indefinitely. (2) The main thread event loop tightens around `pump_mux_events()` → snapshot consumption → `render_dirty_windows()`, starving winit keyboard event dispatch. (3) IO thread parses 64KB chunks and produces snapshots continuously, keeping the main thread in a tight render loop. The result is that keyboard events either never reach the PTY (writer thread blocked) or are never dispatched from winit (main thread starved). This is not just a multi-pane issue — a single pane flooding at sufficient throughput triggers both bottlenecks.
  Found: 2026-04-01 | Source: manual | Updated: 2026-04-04 (severity escalated from high → critical, confirmed single-pane repro)
  Note: Roadmap section 23 (performance) and section 50 (runtime efficiency) touch this area.

- [ ] `[BUG-11-2][high]` **Memory (RSS) grows during output flooding and does not decrease after killing panes** — found by manual.
  Repro: Open multiple panes. Flood each with sustained output (e.g., `yes`, `cat /dev/urandom`). Observe RSS climbing. Kill the panes (close tabs). RSS does not decrease.
  Subsystem: `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm_mux/src/pane/io_thread/snapshot/mod.rs`, `oriterm_core/src/grid/`
  Analysis: During flooding, scrollback grows up to `max_scrollback` and the grid allocates rows. When a pane is killed, `Pane::drop()` → `PaneIoHandle::drop()` shuts down the IO thread, and `Term<T>` drop should free grid memory. Possible causes: (1) System allocator on Windows doesn't eagerly return freed pages to OS (RSS stays high even after dealloc — common with large allocations). (2) Snapshot double buffer retains large capacity allocations that aren't shrunk. (3) GPU-side buffers (instance writers, atlas entries) for killed panes aren't cleaned up. (4) Genuine leak — something holds an Arc or reference to pane data after removal.
  Found: 2026-04-01 | Source: manual
  Note: Roadmap section 50 (runtime efficiency) covers memory discipline.

## Resolved Bugs

(none yet)
