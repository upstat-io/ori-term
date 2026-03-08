---
section: "02"
title: "GAP Fixes"
status: complete
goal: "Close data propagation gaps and fix potential panics in edge cases"
depends_on: []
sections:
  - id: "02.1"
    title: "Exit Code Propagation"
    status: complete
  - id: "02.2"
    title: "Dead PaneOutput Arm in notification_to_pdu"
    status: complete
  - id: "02.3"
    title: "Trace Log Panic on Backwards Slice"
    status: complete
  - id: "02.4"
    title: "IdAllocator Overflow Protection"
    status: complete
  - id: "02.5"
    title: "Completion Checklist"
    status: complete
---

# Section 02: GAP Fixes

**Status:** Complete
**Goal:** No data is silently discarded at boundary crossings, no panics on edge-case inputs, and ID allocation has documented overflow behavior.

**Context:** GAP findings represent missing functionality or safety issues — places where the implementation fails to propagate important data or where edge cases can cause panics. These are correctness issues that affect runtime behavior.

---

## 02.1 Exit Code Propagation

**File(s):** `oriterm_mux/src/in_process/event_pump.rs`, `oriterm_mux/src/mux_event/mod.rs`

**Finding 3:** `MuxEvent::PaneExited { pane_id, .. }` handler discards `exit_code`. `MuxNotification::PaneClosed(PaneId)` carries no exit code. The app cannot distinguish a clean exit (code 0) from an error exit (code 1+), which matters for visual indicators (e.g., red tab dot on error exit).

- [x] Add `exit_code: i32` field to `MuxNotification::PaneClosed`
- [x] Update `InProcessMux::close_pane()` in `in_process/mod.rs` to accept an `exit_code: i32` parameter and propagate it into the notification
  - Client-initiated closes (from `MuxPdu::ClosePane` dispatch) should pass `0` as exit code
- [x] Update the event pump handler for `MuxEvent::PaneExited` to pass `exit_code` to `close_pane()`
- [x] Update the `Debug` impl for `MuxNotification` in `mux_event/mod.rs` to format the new fields
- [x] Update all match arms consuming `MuxNotification::PaneClosed` across the workspace
  - `oriterm_mux/src/in_process/event_pump.rs` (event pump itself)
  - `oriterm_mux/src/server/notify/mod.rs` (`notification_to_pdu`)
  - `oriterm_mux/src/server/mod.rs` (`drain_mux_events` closed pane collection)
  - `oriterm/src/app/mux_pump/mod.rs` (line 66)
  - `oriterm/src/app/window_management.rs` (line 408)
- [x] Update `notification_to_pdu` in `server/notify/mod.rs` to include exit code in the wire PDU
- [x] Update the wire protocol PDU definition (`MuxPdu::NotifyPaneExited`) to carry exit code
- [x] Update `in_process/tests.rs` tests: `close_pane_emits_pane_closed`, `close_pane_not_found`, etc.
- [x] Add test verifying exit code propagation from `MuxEvent::PaneExited` through to `MuxNotification::PaneClosed`

---

## 02.2 Dead PaneOutput Arm in notification_to_pdu

**File(s):** `oriterm_mux/src/server/notify/mod.rs`

**Finding 4:** The `PaneOutput` arm in `notification_to_pdu` is dead in production. The server's `drain_mux_events` intercepts `PaneOutput` before it reaches `notification_to_pdu`. The code compiles and looks intentional, but it can never execute.

**Fix applied:** Merged into the `None`-returning arm with other non-IPC notifications, with a comment explaining it's included for match exhaustiveness.

- [x] Replace the `PaneOutput` arm body with comment and `None` return
- [x] Add comment explaining why this arm is unreachable in normal operation

---

## 02.3 Trace Log Panic on Backwards Slice

**File(s):** `oriterm_mux/src/pty/event_loop/mod.rs`

**Finding 10:** A trace log at line 144-147 can panic when `unprocessed > 200` and the slice range computes backwards (e.g., `buf[450..200]`). This is a debug-only issue but violates the "no panics on user input" principle.

- [x] Cap the slice start index to prevent backwards range
- [x] Add a unit test with `unprocessed > 200` to verify no panic

---

## 02.4 IdAllocator Overflow Protection

**File(s):** `oriterm_mux/src/id/mod.rs`

**Finding 35:** `IdAllocator::alloc` increments a `u64` counter with no overflow protection. If the counter wraps to 0, it violates the "IDs start at 1" invariant. While u64 overflow is practically impossible in normal usage, the invariant should be documented.

- [x] Add `debug_assert!(self.counter < u64::MAX, "IdAllocator counter overflow")` before the increment in `alloc()`
- [x] Add doc comment on `IdAllocator::alloc` documenting:
  - IDs start at 1 (counter is initialized to 1, used then post-incremented)
  - u64 overflow is practically impossible but guarded in debug builds

---

## 02.5 Completion Checklist

- [x] `MuxNotification::PaneClosed` carries `exit_code: i32`
- [x] Exit code flows from `MuxEvent::PaneExited` through event pump to notification
- [x] `PaneOutput` arm in `notification_to_pdu` has comment and merged with None-returning arms
- [x] Trace log slice in `event_loop/mod.rs` cannot panic on any input
- [x] `IdAllocator::alloc` has debug overflow assertion
- [x] `cargo test -p oriterm_mux` passes
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green

**Exit Criteria:** No data is silently discarded at event boundaries. No panic paths exist in debug logging. ID allocation invariants are documented and guarded.
