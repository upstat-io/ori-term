---
section: "02"
title: "Origin Mode & Scroll Regions"
status: not-started
reviewed: true
goal: "Origin mode (DECOM) cursor positioning produces identical output to normal mode in vttest screen 01_02"
inspired_by:
  - "WezTerm origin mode (term/src/terminalstate/performer.rs)"
  - "xterm CUP handler (charproc.c CursorSet)"
depends_on: ["01"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Diagnose Origin Mode Bug"
    status: not-started
  - id: "02.2"
    title: "Fix goto_origin_aware"
    status: not-started
  - id: "02.3"
    title: "Scroll Region Edge Cases"
    status: not-started
  - id: "02.4"
    title: "Update Tests & Golden References"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Origin Mode & Scroll Regions

**Status:** Not Started
**Goal:** vttest screen 01_02 (origin mode border test) produces output identical to screen 01_01 (normal mode border test) at all terminal sizes.

**Context:** vttest menu 1, screen 02 draws the same `*`/`+`/`E` border as screen 01 but with DECOM (origin mode) enabled and scroll margins set. The output should be identical. Currently, the E frame is garbled, borders are missing on interior rows, and text wraps incorrectly. The `goto_origin_aware` function at `handler/helpers.rs:124-146` is the prime suspect.

Even WezTerm fails this test, so getting it right puts oriterm ahead.

**Reference implementations:**
- **WezTerm** `term/src/terminalstate/performer.rs`: origin mode CUP handling.
- **xterm** `charproc.c`: `CursorSet()` function — the canonical reference for DECOM behavior.

**Depends on:** Section 01 (terminal size fix) — origin mode tests at non-80-column sizes require correct size reporting.

---

## 02.1 Diagnose Origin Mode Bug

**File(s):** `oriterm_core/src/term/handler/helpers.rs:130-146`, `oriterm_core/src/term/handler/tests.rs`

The existing `goto_origin_aware` implementation (helpers.rs:130) looks correct in isolation (offset by region start, clamp to region end). The bug must be in how vttest's specific sequence of operations interacts with the implementation.

vttest's screen 02 sequence:
1. Set scroll region (DECSTBM) to a subset of the screen
2. Enable origin mode (DECSET 6)
3. Fill screen with 'E' via DECALN (ESC#8)
4. Draw border using CUP positioning

The DECALN step is critical — `decaln_impl()` at `esc.rs:68` resets the scroll region to full screen via `grid.set_scroll_region(1, None)`, then calls `self.goto_origin_aware(0, 0)`. If DECOM is still active after DECALN, subsequent CUP calls should use the new (full-screen) region. The key question: does DECALN correctly interact with the origin-mode coordinate space, or does it leave stale state?

Note: `set_scrolling_region()` at `handler/mod.rs:268` ALSO calls `self.goto_origin_aware(0, 0)` after setting the region — so the cursor homes twice. This is correct per spec but worth verifying the interaction.

- [ ] Trace the exact VTE sequence vttest sends for screen 02 by adding debug logging to `goto_origin_aware`, `set_scrolling_region`, `decaln_impl`, and `apply_decset`
- [ ] Check the order: does vttest set DECOM before or after setting scroll margins? Does DECALN interact correctly with DECOM?
- [ ] Compare against xterm's `charproc.c` for the exact DECOM + DECSTBM + DECALN interaction semantics
- [ ] Write a targeted unit test that reproduces the vttest 01_02 failure with a minimal escape sequence
- [ ] Verify that `decaln_impl()` does NOT clear the ORIGIN mode flag -- DECALN resets the scroll region but must not change mode flags. Check if `set_scroll_region` has any mode-clearing side effects.

---

## 02.2 Fix goto_origin_aware

**File(s):** `oriterm_core/src/term/handler/helpers.rs`

Based on diagnosis, fix the cursor positioning logic. Key areas to verify:

1. **DECALN resets scroll region** — `decaln_impl` calls `set_scroll_region(1, None)` which resets to full screen. If DECOM is still active, subsequent CUP calls should use the new (full-screen) region, not the old one.
2. **Column handling in origin mode** — DECOM only affects rows (line coordinate), NOT columns. Verify `goto_origin_aware` doesn't apply the offset to the column parameter.
3. **Clamping boundary** — with DECOM on, cursor must be clamped to the scroll region. Verify `max_line` is `region_end - 1` (last line of region, 0-based).
4. **1-based vs 0-based** — VTE sequences use 1-based coordinates. The VTE crate's `Handler::goto` receives 0-based values. Verify no off-by-one.

- [ ] Apply the fix
- [ ] Add unit test: DECOM + full-screen region -- CUP 1;1 produces cursor at (0,0) in both DECOM and normal mode
- [ ] Add unit test: DECOM + narrow region -- CUP clamps correctly at boundaries
- [ ] Add unit test: DECALN while DECOM is active -- scroll region resets, subsequent CUP uses full screen
- [ ] Add unit test: `origin_mode_vpa` -- VPA (CSI n d) in DECOM mode positions relative to scroll region top, not screen top. vttest uses VPA for some positioning sequences.
- [ ] Add unit test: `origin_mode_cup_at_boundaries` -- CUP with row=0 and row=region_height in DECOM mode: row=0 maps to region start, row >= region_height clamps to region end - 1
- [ ] Add unit test: `origin_mode_preserves_column` -- verify goto_origin_aware does NOT offset the column (DECOM only affects row)
- [ ] Add unit test: `dsr_cursor_position_in_decom` -- DSR 6 when DECOM is active should report cursor position relative to scroll region origin, not absolute screen position. Current `status_device_status()` at `status.rs:86` always reports absolute coordinates. Per DEC spec, DECOM affects the cursor position report.
- [ ] `/tpr-review` checkpoint

---

## 02.3 Scroll Region Edge Cases

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`

vttest tests several scroll region edge cases that may expose additional bugs. Note: many of these already have existing test coverage in `grid/scroll/tests.rs` (9 linefeed/reverse_index tests) and `grid/navigation/tests.rs` (8 LF/RI tests). Focus on gaps.

1. **Single-line scroll region** — DECSTBM with top == bottom (should be rejected, region must span >= 2 lines)
2. **Scroll region at screen edges** — region start=1 or end=last line
3. **LF at bottom of scroll region** — should scroll region content, not the whole screen
4. **RI at top of scroll region** — should scroll region down
5. **CUU/CUD at region boundaries** — should stop at region edge, not wrap

- [ ] Review existing scroll region tests in `grid/scroll/tests.rs` to identify gaps
- [ ] Verify `set_scroll_region` validation: region < 2 lines is silently ignored -- check `set_scroll_region` at `grid/scroll/mod.rs:24` for the actual validation
- [ ] Verify LF behavior at region bottom: `linefeed()` at `grid/navigation/mod.rs:99`
- [ ] Verify RI behavior at region top: `reverse_index()` at `grid/navigation/mod.rs:113`
- [ ] Add unit test (if missing): `single_line_scroll_region_rejected` -- DECSTBM with top == bottom is silently ignored, region stays at previous value
- [ ] Add unit test (if missing): `cuu_cud_stop_at_region_boundary` -- CUU at region top does not move above region; CUD at region bottom does not move below
- [ ] Add unit test (if missing): `decom_linefeed_at_region_bottom` -- with DECOM active, LF at the last line of the scroll region scrolls the region content up (not the full screen)
- [ ] Fix any bugs found

---

## 02.4 Update Tests & Golden References

- [ ] Verify `vttest_origin_mode_matches_normal_80x24` passes
- [ ] Verify `vttest_origin_mode_matches_normal_97x33` passes
- [ ] Verify `vttest_origin_mode_matches_normal_120x40` passes
- [ ] Regenerate text snapshots for screens 01_02 at all sizes
- [ ] Regenerate golden PNGs for screens 01_02 at all sizes
- [ ] Ensure screen 01_01 snapshots/PNGs are unchanged (no regressions)

---

## 02.R Third Party Review Findings

- None.

---

## 02.N Completion Checklist

- [ ] `vttest_origin_mode_matches_normal_*` passes at all 3 sizes
- [ ] Screen 01_02 grid text is byte-identical to screen 01_01 at 80x24
- [ ] All scroll region edge case unit tests pass
- [ ] DECALN + DECOM interaction verified with targeted test
- [ ] No regression in screen 01_01 (border test without DECOM)
- [ ] Golden PNGs for 01_02 match 01_01 visually
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** vttest screen 01_02 (origin mode) produces output identical to screen 01_01 (normal mode) at all terminal sizes, verified by `vttest_origin_mode_matches_normal_*` structural assertions.
