---
section: "02"
title: "Blink Timer Fixes"
status: not-started
reviewed: false
goal: "Fix CursorBlink internal duplication, schedule_blink_wakeup thread waste, text_blink_active hardcoding, and dead next_toggle API"
depends_on: []
third_party_review:
  status: findings
  updated: 2026-04-03
sections:
  - id: "02.1"
    title: "CursorBlink Phase Boundary Extraction"
    status: not-started
  - id: "02.2"
    title: "Schedule Blink Wakeup Thread Deduplication"
    status: not-started
  - id: "02.3"
    title: "Text Blink Active Hardcoding"
    status: not-started
  - id: "02.4"
    title: "Remove Dead next_toggle API"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Blink Timer Fixes

**Status:** Not Started
**Goal:** Fix internal duplication in `CursorBlink`, eliminate per-tick thread spawning in `schedule_blink_wakeup`, resolve `text_blink_active` hardcoding, and remove dead `next_toggle` API.

**Context:** The blink timer area has 4 distinct issues: (1) `CursorBlink` computes identical phase boundaries in 3 methods, (2) `schedule_blink_wakeup` spawns a new OS thread every `about_to_wait` tick with no deduplication, (3) `text_blink_active: true` is hardcoded making `ControlFlow::Wait` unreachable, and (4) `next_toggle()` is a deprecated alias with zero production consumers.

---

## 02.1 CursorBlink Phase Boundary Extraction

**File(s):** `oriterm_ui/src/animation/cursor_blink/mod.rs`

Three methods compute identical phase boundaries from `in_duration`, `out_duration`, and `FADE_FRACTION`:
- `intensity_at()` (line 193)
- `next_change()` (line 90)
- `is_animating()` (line 149)

All three compute `fade_out_start` and `hidden_plateau_end` identically (~8 shared setup lines).

- [ ] Extract a private helper:
  ```rust
  /// Phase boundaries for the current cycle.
  /// Returns (fade_out_start, hidden_plateau_end) in seconds within a single cycle,
  /// plus the cycle_pos (position within current cycle).
  fn phase_at(&self, elapsed: Duration) -> Option<(f64, f64, f64)> {
      let total = self.in_duration + self.out_duration;
      if total.is_zero() { return None; }
      let total_secs = total.as_secs_f64();
      let cycle_pos = elapsed.as_secs_f64() % total_secs;
      let in_secs = self.in_duration.as_secs_f64();
      let out_secs = self.out_duration.as_secs_f64();
      let ff = f64::from(FADE_FRACTION);
      Some((in_secs * (1.0 - ff), in_secs + out_secs * (1.0 - ff), cycle_pos))
  }
  ```
- [ ] Refactor `intensity_at()`, `next_change()`, and `is_animating()` to call `phase_at()`.
- [ ] Ensure existing `cursor_blink/tests.rs` still passes.

---

## 02.2 Schedule Blink Wakeup Thread Deduplication

**File(s):** `oriterm/src/app/event_loop_helpers/mod.rs`

`schedule_blink_wakeup()` (line 241) spawns `std::thread::spawn` every `about_to_wait` tick. During 60fps animation, that's 60 threads/sec created and destroyed. No deduplication — multiple pending wakeup threads can overlap.

- [ ] Investigate proper fix approach. Two options:

  **(a) Atomic guard** (recommended — simplest):
  Add an `Arc<AtomicBool>` field to `App` (e.g., `blink_wakeup_pending`). Set it `true` before spawning, check it before spawning (skip if already `true`). The spawned thread sets it back to `false` after sending the wakeup. This limits to at most 1 pending wakeup thread.

  **(b) Dedicated wakeup thread with channel**:
  A single long-lived thread receives "wake at Instant(t)" messages via `mpsc`. It parks/sleeps until the target instant, sends `MuxWakeup`, then waits for the next message. Zero thread creation/destruction overhead.

- [ ] Implement the chosen approach.
- [ ] Verify idle CPU behavior is unchanged (blink still works at correct cadence).

---

## 02.3 Text Blink Active Hardcoding

**File(s):** `oriterm/src/app/event_loop.rs`

At line 478, `text_blink_active: true` is hardcoded in the `ControlFlowInput` construction. This means `compute_control_flow()` can never return `ControlFlowDecision::Wait` — it always returns `WaitUntil(next_text_blink_change)`, causing ~2 wakeups/sec even with zero blinking text cells. This was a deliberate design choice per `plans/completed/vttest-conformance/section-05-fade-blink.md` ("the timer runs cheaply").

- [ ] Evaluate whether `text_blink_active` should remain always-true:
  - **If kept always-true**: Remove the `text_blink_active` field from `ControlFlowInput` entirely and fold the text blink logic into `compute_control_flow()` unconditionally. Document why in a comment. Update the test helper `idle_input()` which currently sets `text_blink_active: false` — this represents a state that never occurs in production.
  - **If made dynamic**: Compute `text_blink_active` from whether any visible cell has the `BLINK` flag. This is more correct but requires scanning cells or maintaining a count.
- [ ] Update `event_loop_helpers/tests.rs` — the `idle_input()` helper at line 6 sets `text_blink_active: false`. Either update it to `true` (matching production) or remove the field.

---

## 02.4 Remove Dead next_toggle API

**File(s):** `oriterm_ui/src/animation/cursor_blink/mod.rs`, `oriterm_ui/src/animation/cursor_blink/tests.rs`

`next_toggle()` (line 134) is a deprecated alias for `next_change()`. Zero production consumers — the only caller is test `next_toggle_delegates_to_next_change` in `tests.rs:285`.

- [ ] Remove `next_toggle()` from `cursor_blink/mod.rs`.
- [ ] Remove or convert the test `next_toggle_delegates_to_next_change` in `tests.rs`.
- [ ] Verify no other callers exist (already confirmed via grep).

---

## 02.R Third Party Review Findings

- [x] `[TPR-02-001][medium]` `oriterm/src/app/event_loop.rs:491-496`, `oriterm/src/app/event_loop_helpers/mod.rs:235-263` — `schedule_blink_wakeup()` still spawns a brand-new detached thread on every `about_to_wait` pass and has no pending-wakeup guard. During fade cadence that means roughly one thread every 16 ms, and during any busy loop multiple sleepers can overlap and all send `MuxWakeup` later. This matches the waste described in Section 02.2 and is still unresolved in the current tree.

- [x] `[TPR-02-002][medium]` `oriterm/src/app/event_loop.rs:471-479`, `oriterm/src/app/event_loop_helpers/mod.rs:337-395`, `oriterm/src/app/event_loop_helpers/tests.rs:5-26` — production hardcodes `text_blink_active: true`, so `compute_control_flow()` cannot reach `ControlFlowDecision::Wait` once dirty work drains; it will always schedule `WaitUntil(next_text_blink_change)` instead. The tests still model idle with `text_blink_active: false`, so the suite passes while asserting a state that production never constructs. This revalidates the Section 02.3 concern and the associated test drift.

---

## 02.N Completion Checklist

- [ ] `CursorBlink` phase boundary computation exists in exactly one private method
- [ ] `schedule_blink_wakeup` has a deduplication mechanism (at most 1 pending wakeup)
- [ ] `text_blink_active` either removed from `ControlFlowInput` or computed dynamically
- [ ] `next_toggle()` removed from public API
- [ ] `./test-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** `CursorBlink` has no duplicated phase calculations. Thread spawn rate during idle is bounded (0-1 pending threads). The `ControlFlowInput` struct accurately represents variable state.
