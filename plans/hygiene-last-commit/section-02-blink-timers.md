---
section: "02"
title: "Blink Timer Fixes"
status: not-started
reviewed: true
goal: "Fix CursorBlink internal duplication, schedule_blink_wakeup thread waste, text_blink_active hardcoding, and dead next_toggle API"
depends_on: []
third_party_review:
  status: resolved
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

<!-- reviewed: architecture fix — the original plan proposed a tuple-returning `phase_at()` that
     only returns (fade_out_start, hidden_plateau_end, cycle_pos). This is insufficient:
     `next_change()` also needs `cycle_start_secs` and `in_secs` (to check `cycle_pos < in_secs`
     for the fade-out/hidden boundary). `intensity_at()` also needs `fade_out_dur` and `fade_in_dur`
     (derivable from `in_secs * ff` and `out_secs * ff`). A struct is needed instead of a tuple. -->

- [ ] Extract a private helper struct and computation method. The tuple `(f64, f64, f64)` proposed originally is insufficient because `next_change()` needs `cycle_start_secs` and `in_secs`, and `intensity_at()` needs `fade_out_dur` and `fade_in_dur`. Use a struct:
  ```rust
  /// Precomputed phase boundaries for one blink cycle.
  struct PhaseBounds {
      /// Position within the current cycle (seconds).
      cycle_pos: f64,
      /// Elapsed seconds at the start of the current cycle.
      cycle_start_secs: f64,
      /// Start of fade-out transition (end of visible plateau), in cycle-local seconds.
      fade_out_start: f64,
      /// End of visible phase / start of hidden plateau (= in_secs), in cycle-local seconds.
      fade_out_end: f64,
      /// End of hidden plateau / start of fade-in, in cycle-local seconds.
      hidden_plateau_end: f64,
      /// Duration of fade-out transition (in_secs * FADE_FRACTION).
      fade_out_dur: f64,
      /// Duration of fade-in transition (out_secs * FADE_FRACTION).
      fade_in_dur: f64,
  }
  ```
  Note: the test file already has a similar `PhaseBounds` struct (tests.rs:7-16) but without `cycle_pos` and `cycle_start_secs`. The production struct serves a different purpose (per-call computation) so keeping them separate is fine.

- [ ] Add a private method that computes the struct:
  ```rust
  /// Compute phase boundaries at a given elapsed duration from epoch.
  /// Returns `None` if the total cycle duration is zero.
  fn phase_bounds(&self, elapsed: Duration) -> Option<PhaseBounds> {
      let total = self.in_duration + self.out_duration;
      if total.is_zero() { return None; }
      let total_secs = total.as_secs_f64();
      let cycle_pos = elapsed.as_secs_f64() % total_secs;
      let in_secs = self.in_duration.as_secs_f64();
      let out_secs = self.out_duration.as_secs_f64();
      let ff = f64::from(FADE_FRACTION);
      Some(PhaseBounds {
          cycle_pos,
          cycle_start_secs: elapsed.as_secs_f64() - cycle_pos,
          fade_out_start: in_secs * (1.0 - ff),
          fade_out_end: in_secs,
          hidden_plateau_end: in_secs + out_secs * (1.0 - ff),
          fade_out_dur: in_secs * ff,
          fade_in_dur: out_secs * ff,
      })
  }
  ```

- [ ] Refactor `intensity_at()` to call `phase_bounds()`. The `None` (zero-duration) case returns `1.0` (existing behavior). The phase match uses `pb.fade_out_start`, `pb.fade_out_end`, `pb.hidden_plateau_end`, `pb.fade_out_dur`, and `pb.fade_in_dur`.

- [ ] Refactor `next_change()` to call `phase_bounds()`. The `None` case returns `self.epoch + Duration::from_secs(1)` (existing behavior). The phase match uses `pb.cycle_start_secs` to compute absolute instants for plateau-end boundaries, and returns `Instant::now() + ANIMATION_FRAME_INTERVAL` for fade regions. <!-- reviewed: next_change needs cycle_start_secs for absolute Instant computation — this field was missing from original tuple proposal -->

- [ ] Refactor `is_animating()` to call `phase_bounds()`. The `None` case returns `false` (existing behavior). The animating check is `(cycle_pos >= fade_out_start && cycle_pos < fade_out_end) || cycle_pos >= hidden_plateau_end`. <!-- reviewed: is_animating checks cycle_pos < in_secs (= fade_out_end), not cycle_pos < in_secs from a separate local — the struct provides this as fade_out_end -->

- [ ] Ensure existing `cursor_blink/tests.rs` still passes (all 20 tests). <!-- reviewed: accuracy fix — actual count is 20 tests, not 18 -->

---

## 02.2 Schedule Blink Wakeup Thread Deduplication

**File(s):** `oriterm/src/app/event_loop_helpers/mod.rs`, `oriterm/src/app/mod.rs`, `oriterm/src/app/constructors.rs`

`schedule_blink_wakeup()` (line 241) spawns `std::thread::spawn` every `about_to_wait` tick. During 60fps fade animation, that's ~60 threads/sec created and destroyed. During idle plateau, it's ~2/sec (still wasteful). No deduplication — multiple pending wakeup threads can overlap.

<!-- reviewed: clarified thread rate during idle vs animation, added file list for fields -->

**Approach: Atomic guard** (recommended — simplest, sufficient):
Add an `Arc<AtomicBool>` field to `App` (e.g., `blink_wakeup_pending`). Check it before spawning (skip if already `true`). Set it `true` before spawning. The spawned thread sets it back to `false` after sending the wakeup. This limits to at most 1 pending wakeup thread at any time.

- [ ] Add `use std::sync::atomic::AtomicBool;` import to `mod.rs` (in the `std` import group at line 52, alongside the existing `use std::sync::Arc;`). <!-- reviewed: accuracy fix — AtomicBool not currently imported in mod.rs -->

- [ ] Add field to `App` in `mod.rs`:
  ```rust
  // Atomic guard: at most one pending blink wakeup thread.
  blink_wakeup_pending: Arc<AtomicBool>,
  ```
  Place it near the existing `cursor_blink` / `text_blink` fields (around line 170).

- [ ] Add `use std::sync::atomic::AtomicBool;` import to `constructors.rs` (in the `std` import group at line 4, alongside the existing `use std::sync::Arc;`). <!-- reviewed: accuracy fix — AtomicBool not currently imported in constructors.rs -->

- [ ] Initialize in `constructors.rs` (around line 130, near the other blink fields):
  ```rust
  blink_wakeup_pending: Arc::new(AtomicBool::new(false)),
  ```

- [ ] Update `schedule_blink_wakeup()` in `event_loop_helpers/mod.rs` to check and set the guard:
  ```rust
  pub(super) fn schedule_blink_wakeup(&self) {
      // Skip if a wakeup thread is already pending.
      if self.blink_wakeup_pending.load(Ordering::Acquire) {
          return;
      }

      let delay = /* ... existing delay computation ... */;

      self.blink_wakeup_pending.store(true, Ordering::Release);
      let sender = self.event_proxy.clone();
      let pending = self.blink_wakeup_pending.clone();
      std::thread::spawn(move || {
          std::thread::sleep(delay);
          sender.send(crate::event::TermEvent::MuxWakeup);
          pending.store(false, Ordering::Release);
      });
  }
  ```
  <!-- reviewed: Acquire/Release ordering is correct here — the main thread's store-Release
       synchronizes-with the spawned thread's load, and the spawned thread's store-Release
       synchronizes-with the main thread's next load-Acquire. No data races on non-atomic state. -->
  <!-- reviewed: accuracy fix — use `.clone()` not `Arc::clone()` to match codebase style
       (the codebase uses method-style `.clone()` throughout, see event_loop_helpers/mod.rs:258) -->

- [ ] Add required imports to `event_loop_helpers/mod.rs`: `use std::sync::atomic::Ordering;`. No `Arc` import needed — `.clone()` works via method resolution on the `Arc<AtomicBool>` field. <!-- reviewed: accuracy fix — clarified that Arc import is not needed -->

- [ ] Verify idle CPU behavior is unchanged: cursor blink still works at correct cadence, text blink still works. The guard prevents overlapping threads but does not prevent new threads after the previous one completes (the `false` store happens after `MuxWakeup` is sent, so the next `about_to_wait` tick can spawn again). **Edge case**: if the event loop processes `MuxWakeup` before the thread stores `false`, the next `schedule_blink_wakeup` call sees the guard as `true` and skips spawning. This is benign — the wakeup was already delivered, and the next `about_to_wait` tick (from any subsequent event) will see `false` and spawn if needed. At worst, one blink phase boundary wakeup is delayed by one event cycle. <!-- reviewed: documented benign race window -->

---

## 02.3 Remove text_blink_active Field

**File(s):** `oriterm/src/app/event_loop.rs`, `oriterm/src/app/event_loop_helpers/mod.rs`, `oriterm/src/app/event_loop_helpers/tests.rs`

<!-- reviewed: architecture fix — the original plan presented this as an open "evaluate" question.
     The design decision was already made in plans/completed/vttest-conformance/section-05b-text-blink.md
     line 96: "text_blink_active is true when any visible cell has the BLINK flag. To avoid scanning
     all cells each frame, use a conservative approach: set text_blink_active = true always (the timer
     runs cheaply even with no BLINK cells; the only cost is a plateau wakeup every 500ms which is
     negligible)." This is a deliberate, documented decision. The field should be removed (not made
     dynamic), and the text blink logic folded in unconditionally. Making it dynamic would require
     scanning grid cells from the event loop level, crossing crate boundaries. -->

At line 478 of `event_loop.rs`, `text_blink_active: true` is hardcoded. This means `compute_control_flow()` can never return `ControlFlowDecision::Wait` — it always returns `WaitUntil(next_text_blink_change)`, causing ~2 wakeups/sec. This is a deliberate design choice (vttest-conformance section-05b, line 96) — the timer is cheap and avoids scanning all visible cells for the BLINK flag each frame.

Since the value is always `true` by design, the `text_blink_active` field is dead code in `ControlFlowInput`. Remove it and fold the text blink timer unconditionally into `compute_control_flow()`.

- [ ] Remove the `text_blink_active: bool` field from `ControlFlowInput` (line 338 of `event_loop_helpers/mod.rs`). Also remove its doc comment (line 337).

- [ ] Update the doc comment on `next_text_blink_change` (line 339 after removal) — currently says "only meaningful if `text_blink_active`", should say "Next text blink phase boundary (always active — unconditional timer)." <!-- reviewed: accuracy fix — the doc comment references the removed field -->

- [ ] Remove the `text_blink_active: true` assignment from the `ControlFlowInput` construction in `event_loop.rs` (line 478).

- [ ] Update `compute_control_flow()` in `event_loop_helpers/mod.rs` to always consider text blink. The current logic:
  ```rust
  } else if input.blinking_active || input.text_blink_active {
  ```
  Becomes:
  ```rust
  } else {
      // Text blink timer always contributes (any cell could have BLINK flag;
      // scanning cells each frame is too expensive, so the timer runs
      // unconditionally at ~2 wakeups/sec — negligible cost).
  ```
  The `wake_at` computation simplifies: always start from `next_text_blink_change`, then `min` with `next_blink_change` when `blinking_active`. The final `else` branch (`ControlFlowDecision::Wait`) is removed — text blink always provides a `WaitUntil`. <!-- reviewed: Wait is no longer reachable when text blink is unconditional, but the scheduler_wake path still needs to pick the earlier of text blink and scheduler wake -->

- [ ] Verify the updated `compute_control_flow()` logic handles all combinations correctly:
  - `blinking_active = true`: `wake_at = min(next_text_blink_change, next_blink_change)`; then `min` with `scheduler_wake` if present.
  - `blinking_active = false`: `wake_at = next_text_blink_change`; then `min` with `scheduler_wake` if present.
  - `scheduler_wake = None`: just `WaitUntil(wake_at)`.
  - `scheduler_wake = Some(t)`: `WaitUntil(min(wake_at, t))`.
  - **IMPORTANT**: the `ControlFlowDecision::Wait` variant becomes unreachable (never constructed) once text blink is unconditional. The workspace has `dead_code = "deny"`, so Rust will error on the unused variant. Two options: (a) remove the `Wait` variant entirely and update the match in `event_loop.rs:484-489` to be exhaustive over the remaining `WaitUntil` variant, or (b) keep the variant with `#[allow(dead_code, reason = "reserved for future use when all timers can be disabled")]`. Option (a) is cleaner — a one-variant enum can be simplified to just return the `Instant` directly from `compute_control_flow()` and eliminate the enum entirely. However, that is a larger refactor. Prefer (b) for now to keep the change scoped. <!-- reviewed: accuracy fix — dead_code = "deny" at workspace level will reject the unused variant; original plan did not account for this -->

- [ ] If clippy/build errors on `ControlFlowDecision::Wait` as dead code, add `#[allow(dead_code, reason = "reserved for future use when all timers can be disabled")]` on the `Wait` variant. Check by running `./build-all.sh` after removing the field. <!-- reviewed: guard step for dead_code = "deny" -->

- [ ] Update `idle_input()` in `event_loop_helpers/tests.rs`: remove the `text_blink_active: false` field.

- [ ] Update the `idle_returns_wait` test (tests.rs:24) — this test asserts `ControlFlowDecision::Wait` for the idle case, but with text blink unconditional, idle now returns `WaitUntil(next_text_blink_change)`. Update the assertion:
  ```rust
  #[test]
  fn idle_returns_text_blink_wait() {
      let input = idle_input();
      // Text blink timer always contributes — true idle is WaitUntil, not Wait.
      let result = compute_control_flow(&input);
      assert_eq!(result, ControlFlowDecision::WaitUntil(input.next_text_blink_change));
  }
  ```

- [ ] Update the `not_still_dirty_goes_idle` test (tests.rs:30) — same issue, same fix. Remove this test entirely — it tests the exact same condition as `idle_returns_wait` with a different name and no additional setup. After the `idle_returns_wait` rename and assertion update, this test is redundant. <!-- reviewed: executability fix — "may be merged" was ambiguous; made the decision explicit -->

- [ ] Review all other tests in the file that use `idle_input()` to ensure they still pass. Most set `blinking_active = true` or `has_animations = true`, overriding the base idle state, so they should be unaffected by the removal of `text_blink_active`. But verify:
  - `blinking_returns_next_blink_change` (line 58): sets `blinking_active = true`, `next_blink_change = now + 530ms`. With text blink also active (`next_text_blink_change = now + 1s`), the result should be `WaitUntil(now + 530ms)` (blink is earlier). **Unchanged — still passes.**
  - `scheduler_wake_picks_earlier_of_blink_and_wake` (line 105): sets `blinking_active = true`, `scheduler_wake = now + 100ms`, `next_blink_change = now + 530ms`. Text blink at `now + 1s`. Result: `WaitUntil(now + 100ms)` (scheduler is earliest). **Unchanged.**
  - `scheduler_wake_returns_wait_until_when_idle` (line 95): sets `scheduler_wake = now + 200ms`. With text blink at `now + 1s`, result should be `WaitUntil(min(1s, 200ms)) = WaitUntil(now + 200ms)`. **Unchanged — scheduler wake is earlier.**

---

## 02.4 Remove Dead next_toggle API

**File(s):** `oriterm_ui/src/animation/cursor_blink/mod.rs`, `oriterm_ui/src/animation/cursor_blink/tests.rs`

`next_toggle()` (line 134) is a deprecated alias for `next_change()`. Zero production consumers — the only caller is test `next_toggle_delegates_to_next_change` in `tests.rs:285`.

- [ ] Remove `next_toggle()` from `cursor_blink/mod.rs` (lines 131-136).
- [ ] Remove the test `next_toggle_delegates_to_next_change` from `tests.rs` (lines 284-298, including the `#[test]` attribute). <!-- reviewed: accuracy fix — line 284 is the #[test] attribute, 285 is the fn -->
- [ ] Verify no other callers exist via grep (already confirmed — only plan files and the test reference it). <!-- reviewed: verified via grep — only plan docs, the method def, and one test call it -->

---

## 02.R Third Party Review Findings

- [x] `[TPR-02-001][medium]` `oriterm/src/app/event_loop.rs:491-496`, `oriterm/src/app/event_loop_helpers/mod.rs:235-263` — `schedule_blink_wakeup()` still spawns a brand-new detached thread on every `about_to_wait` pass and has no pending-wakeup guard. During fade cadence that means roughly one thread every 16 ms, and during any busy loop multiple sleepers can overlap and all send `MuxWakeup` later. This matches the waste described in Section 02.2 and is still unresolved in the current tree.

- [x] `[TPR-02-002][medium]` `oriterm/src/app/event_loop.rs:471-479`, `oriterm/src/app/event_loop_helpers/mod.rs:337-395`, `oriterm/src/app/event_loop_helpers/tests.rs:5-26` — production hardcodes `text_blink_active: true`, so `compute_control_flow()` cannot reach `ControlFlowDecision::Wait` once dirty work drains; it will always schedule `WaitUntil(next_text_blink_change)` instead. The tests still model idle with `text_blink_active: false`, so the suite passes while asserting a state that production never constructs. This revalidates the Section 02.3 concern and the associated test drift.

---

## 02.N Completion Checklist

- [ ] `CursorBlink` phase boundary computation exists in exactly one private method (`phase_bounds()`) returning a `PhaseBounds` struct
- [ ] `schedule_blink_wakeup` has an `Arc<AtomicBool>` guard (at most 1 pending wakeup thread)
- [ ] `text_blink_active` field removed from `ControlFlowInput`; text blink timer unconditionally contributes to `compute_control_flow()`
- [ ] `idle_returns_wait` test renamed to `idle_returns_text_blink_wait` and updated to expect `WaitUntil` (not `Wait`); `not_still_dirty_goes_idle` removed (redundant) <!-- reviewed: consistency fix — matches 02.3 decision to remove redundant test -->
- [ ] `next_toggle()` removed from public API and its test removed
- [ ] `./test-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** `CursorBlink` has no duplicated phase calculations. Thread spawn rate during idle is bounded (0-1 pending threads). The `ControlFlowInput` struct accurately represents variable state (no always-true fields).
