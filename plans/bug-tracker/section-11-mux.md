---
section: 11
title: "Mux & Pane I/O"
domain: "oriterm_mux — pane server, PTY I/O, IO thread, pane lifecycle, backpressure"
status: in-progress
---

# Section 11: Mux & Pane I/O

Bugs in the pane multiplexer — PTY I/O, IO thread behavior, pane lifecycle, memory management.

## Open Bugs

- [ ] `[BUG-11-7][low]` **MuxClient transport layer BLOAT: over-length functions + excessive nesting** — found by impl-hygiene-review.
  Repro: `.claude/skills/impl-hygiene-review/hygiene-lint.sh --scope oriterm_mux/src/backend/client --check fn-length,nesting-depth` reports four BLOAT findings.
  Subsystem: `oriterm_mux/src/backend/client/transport/mod.rs:93` (`fn connect` — 119 lines, limit 100); `oriterm_mux/src/backend/client/transport/reader.rs:132` (`fn reader_loop` — 112 lines, limit 100); `reader.rs:167` (`fn reader_loop` nesting depth 6, limit 4); `reader.rs:269` (`fn read_and_dispatch_frames` nesting depth 5, limit 4).
  Found: 2026-04-16 | Source: impl-hygiene-review
  Note: Surfaced during the Section 09 (DEC Private Modes) /impl-hygiene-review close-out gate. Not touched by Section 09 work — the new daemon-side bridge test in `oriterm_mux/src/backend/client/tests.rs` is clean; these findings are in `transport/` code that predates Section 09. Candidate refactors: extract PDU handshake steps out of `connect`; split `reader_loop` into `accept_frame` / `dispatch_frame` helpers; flatten the inner `if let` chains in `reader_loop` + `read_and_dispatch_frames` via guard clauses or extracted helper functions returning `Option`.

- [ ] `[BUG-11-4][high]` **DA1/DA2/DA3, DSR 5/6, CSI 18t, and DECRQM produce no response in the live terminal** — found by manual.
  Repro: Run `printf '\e[c'` (DA1), `printf '\e[5n'` (DSR 5), `printf '\e[>c'` (DA2), `printf '\e[18t'` (CSI 18t) in oriterm — no response received by the requesting process.
  Subsystem: `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` (PtyWrite forwarding), `oriterm_mux/src/in_process/event_pump.rs` (PtyWrite handling), `oriterm_mux/src/pty/mod.rs` (writer thread)
  Analysis: Code analysis shows the full pipeline appears wired: core handlers emit `Event::PtyWrite(response)` in `oriterm_core/src/term/handler/status.rs`, the IO-thread event proxy forwards these as `MuxEvent::PtyWrite`, and the event pump at `event_pump.rs:66` calls `pane.write_input()` which enqueues to the PTY writer thread. Despite the code paths existing, live testing confirms no response is received. Root cause unknown — needs runtime instrumentation (logging at each hop: event proxy send, event pump receive, writer thread write) to identify where the response is lost. Possible causes: (1) timing/ordering issue where the event pump doesn't run between the response being generated and the shell timing out its read, (2) ConPTY on Windows intercepting or dropping the response, (3) event loop wakeup not triggering `pump_mux_events()` in time, (4) some other runtime condition not visible to static analysis.
  Found: 2026-04-05 | Source: manual
  Note: Roadmap sections 02/30/38 document these handlers as implemented and verified by unit tests, but end-to-end mux round-trip was never tested in the live app. The vttest integration tests (`oriterm_core/tests/vttest/`) bypass the mux entirely.

- [ ] `[BUG-11-3][high]` **OSC 10/11/12 color queries silently dropped ��� no reply sent to requesting app** — found by manual.
  Repro: Run `printf '\e]10;?\e\\'` in oriterm ��� no response. Apps querying terminal colors (e.g., vim, neovim, delta, bat) get no reply, causing hangs or incorrect color detection.
  Subsystem: `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` (line 150 — ColorRequest dropped), `oriterm_mux/src/mux_event/tests.rs` (line 445 — test asserts no MuxEvent for ColorRequest)
  Analysis: Core correctly emits `Event::ColorRequest(index, closure)` in `oriterm_core/src/term/handler/osc.rs:94`. The closure takes the current `Rgb` color and returns the OSC response string that should be written back to the PTY. However, the IO-thread event proxy at `event_proxy/mod.rs:150` groups ColorRequest with CursorBlinkingChange and MouseCursorDirty as "events that don't need mux routing" and only wakes the event loop. The response string is never generated and never written to the PTY. The fix requires the event proxy to invoke the closure with the current color from the palette, then write the resulting response bytes back to the PTY (similar to how PtyWrite events are handled). The existing test asserting no MuxEvent for ColorRequest must also be updated.
  Found: 2026-04-05 | Source: manual
  Note: Roadmap section 38 (protocol extensions) documents OSC 10/11/12 core implementation as complete but did not verify end-to-end mux routing. Section 30 (pane domain) explicitly documented ColorRequest as "non-routed" — this was a design gap, not intentional.

- [ ] `[BUG-11-2][high]` **Memory (RSS) grows during output flooding and does not decrease after killing panes** — found by manual.
  Repro: Open multiple panes. Flood each with sustained output (e.g., `yes`, `cat /dev/urandom`). Observe RSS climbing. Kill the panes (close tabs). RSS does not decrease.
  Subsystem: `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm_mux/src/pane/io_thread/snapshot/mod.rs`, `oriterm_core/src/grid/`
  Analysis: During flooding, scrollback grows up to `max_scrollback` and the grid allocates rows. When a pane is killed, `Pane::drop()` → `PaneIoHandle::drop()` shuts down the IO thread, and `Term<T>` drop should free grid memory. Possible causes: (1) System allocator on Windows doesn't eagerly return freed pages to OS (RSS stays high even after dealloc — common with large allocations). (2) Snapshot double buffer retains large capacity allocations that aren't shrunk. (3) GPU-side buffers (instance writers, atlas entries) for killed panes aren't cleaned up. (4) Genuine leak — something holds an Arc or reference to pane data after removal.
  Found: 2026-04-01 | Source: manual
  Note: Roadmap section 50 (runtime efficiency) covers memory discipline.

- [ ] `[BUG-11-5][medium]` **signal_child() sends SIGINT to shell PGID instead of foreground PGID** — found by tpr-review.
  Repro: Spawn a pane, run `yes` (foreground job). Call `signal_child(Signal::Interrupt)`. SIGINT goes to the shell's process group, not the foreground process group running `yes`.
  Subsystem: `oriterm_mux/src/pane/mod.rs:442` (`send_signal_platform`)
  Analysis: `kill(-pid, SIGINT)` uses the shell PID as the process group ID. This works when the shell IS the foreground process, but misses foreground jobs (`yes`, `cat`, etc.) that run in their own process group. The fix is to use `tcgetpgrp(pty_fd)` to obtain the foreground PGID and signal that instead. Two e2e tests are `#[ignore]` waiting for this: `e2e.rs:1487` and `e2e.rs:1654`.
  Found: 2026-04-05 | Source: tpr-review
  Note: Keyboard Ctrl+C (BUG-11-1) works correctly — the kernel PTY layer handles `\x03` → SIGINT to the foreground PGID. This bug only affects the programmatic `signal_child()` path.

- [ ] `[BUG-11-6][medium]` **`push_notification_triggers_dirty_flag` e2e test flaky — asserts on first dirty snapshot**
  Repro: `cargo test -p oriterm_mux --test e2e push_notification_triggers_dirty_flag` in a loop; fails intermittently.
  Subsystem: `oriterm_mux/tests/e2e.rs:280`
  Analysis: Test sends `echo PUSH_TEST\n`, waits for `is_pane_snapshot_dirty`, then asserts the refreshed snapshot contains "PUSH_TEST". The snapshot can become dirty from the command echo or prompt redraw before the actual output lands. Fix: continue the loop when snapshot doesn't contain "PUSH_TEST" instead of asserting on the first dirty snapshot.
  Found: 2026-04-12 | Source: continue-roadmap

## Resolved Bugs

- [x] `[BUG-11-1][critical]` **All input blocked during sustained output flooding (even single pane)** — found by manual.
  Resolved: 2026-04-05. Two-part fix: (1) PTY writer thread now sets a `write_stalled` AtomicBool flag before potentially-blocking `write()` calls — the main thread reads this to detect when the writer is stuck on a full kernel buffer. (2) When Ctrl+C is pressed and the writer is stalled, SIGINT is sent directly to the child process group via `kill(-pid, SIGINT)` on Unix / `GenerateConsoleCtrlEvent` on Windows, bypassing the blocked PTY pipe. Writer thread also improved: coalesces pending input, uses `recv_timeout` instead of blocking `recv`, and flushes pending data before shutdown. Files: `oriterm_mux/src/pty/mod.rs` (writer thread), `oriterm_mux/src/pane/mod.rs` (Signal enum, signal delivery), `oriterm_mux/src/backend/embedded/mod.rs` + `mod.rs` (MuxBackend trait), `oriterm/src/app/keyboard_input/mod.rs` (Ctrl+C bypass).
