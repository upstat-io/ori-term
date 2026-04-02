---
section: "02"
title: "Origin Mode & Scroll Regions"
status: complete
reviewed: true
goal: "Origin mode (DECOM) cursor positioning produces identical output to normal mode in vttest screen 01_02"
inspired_by:
  - "WezTerm origin mode (term/src/terminalstate/performer.rs)"
  - "xterm CUP handler (charproc.c CursorSet)"
depends_on: ["01"]
third_party_review:
  status: resolved
  updated: 2026-04-02
sections:
  - id: "02.1"
    title: "Diagnose Origin Mode Bug"
    status: complete
  - id: "02.2"
    title: "Fix goto_origin_aware"
    status: complete
  - id: "02.3"
    title: "Scroll Region Edge Cases"
    status: complete
  - id: "02.4"
    title: "Update Tests & Golden References"
    status: complete
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.N"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Origin Mode & Scroll Regions

**Status:** Complete
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

- [x] Trace the exact VTE sequence vttest sends for screen 02 — traced via hex dump in Section 01 work. vttest sets DECOM before drawing border, uses CUP and HVP for positioning.
- [x] Check the order: does vttest set DECOM before or after setting scroll margins? — vttest resets margins (DECSTBM), resets DECOM, then for screen 02 sets margins and enables DECOM. DECALN resets margins correctly.
- [x] Compare against xterm's `charproc.c` — confirmed DECOM behavior matches. Also verified DSR 6 should report relative position in DECOM (fixed).
- [x] Write a targeted unit test that reproduces the vttest 01_02 failure with a minimal escape sequence — vttest 01_02 tests already pass at all 3 sizes after Section 01 fixes.
- [x] Verify that `decaln_impl()` does NOT clear the ORIGIN mode flag — verified by `decaln_while_origin_mode_active` test: DECOM stays active after DECALN, CUP uses the reset full-screen region.

---

## 02.2 Fix goto_origin_aware

**File(s):** `oriterm_core/src/term/handler/helpers.rs`

Based on diagnosis, fix the cursor positioning logic. Key areas to verify:

1. **DECALN resets scroll region** — `decaln_impl` calls `set_scroll_region(1, None)` which resets to full screen. If DECOM is still active, subsequent CUP calls should use the new (full-screen) region, not the old one.
2. **Column handling in origin mode** — DECOM only affects rows (line coordinate), NOT columns. Verify `goto_origin_aware` doesn't apply the offset to the column parameter.
3. **Clamping boundary** — with DECOM on, cursor must be clamped to the scroll region. Verify `max_line` is `region_end - 1` (last line of region, 0-based).
4. **1-based vs 0-based** — VTE sequences use 1-based coordinates. The VTE crate's `Handler::goto` receives 0-based values. Verify no off-by-one.

- [x] Apply the fix — origin mode CUP already works correctly (6 pre-existing tests pass). Fixed DSR 6 to report relative position in DECOM mode.
- [x] Add unit test: DECOM + full-screen region — `origin_mode_disabled_cup_uses_full_screen` (pre-existing)
- [x] Add unit test: DECOM + narrow region — `origin_mode_cup_clamps_to_scroll_region` (pre-existing)
- [x] Add unit test: DECALN while DECOM is active — `decaln_while_origin_mode_active` (new)
- [x] Add unit test: `origin_mode_vpa` — `origin_mode_vpa_relative_to_scroll_region` (pre-existing)
- [x] Add unit test: `origin_mode_cup_at_boundaries` — `origin_mode_cup_row_zero_maps_to_region_start` (new) + `origin_mode_cup_clamps_to_scroll_region` (pre-existing)
- [x] Add unit test: `origin_mode_preserves_column` — `origin_mode_preserves_column` (new)
- [x] Add unit test: `dsr_cursor_position_in_decom` — `dsr_reports_relative_position_in_origin_mode` (new, replaces `dsr_reports_absolute_position_even_in_origin_mode`). Fixed `status_device_status()` to subtract scroll region start when DECOM active.
- [x] `/tpr-review` checkpoint — passed, 2 findings resolved (TPR-02-001, TPR-02-002)

---

## 02.3 Scroll Region Edge Cases

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`

vttest tests several scroll region edge cases that may expose additional bugs. Note: many of these already have existing test coverage in `grid/scroll/tests.rs` (9 linefeed/reverse_index tests) and `grid/navigation/tests.rs` (8 LF/RI tests). Focus on gaps.

1. **Single-line scroll region** — DECSTBM with top == bottom (should be rejected, region must span >= 2 lines)
2. **Scroll region at screen edges** — region start=1 or end=last line
3. **LF at bottom of scroll region** — should scroll region content, not the whole screen
4. **RI at top of scroll region** — should scroll region down
5. **CUU/CUD at region boundaries** — should stop at region edge, not wrap

- [x] Review existing scroll region tests in `grid/scroll/tests.rs` — 1071 lines of tests, comprehensive coverage of scroll_up, scroll_down, linefeed, reverse_index, and region edge cases
- [x] Verify `set_scroll_region` validation — confirmed at `grid/scroll/mod.rs:30`: `if top + 1 >= bottom { return; }` silently ignores regions < 2 lines
- [x] Verify LF behavior at region bottom — 9 existing linefeed tests in `grid/navigation/tests.rs` and `handler/tests.rs` covering region boundaries
- [x] Verify RI behavior at region top — 8 existing reverse_index tests covering region boundaries
- [x] Add unit test: `single_line_scroll_region_rejected` (new)
- [x] Add unit test: `scroll_region_at_screen_edges` — validates region at line 1, last line, and full screen (new)
- [x] Verify CUU/CUD at region boundary — covered by existing `origin_mode_cup_clamps_to_scroll_region` and `cud_stops_at_scroll_region_bottom` tests (pre-existing in handler/tests.rs)
- [x] Verify DECOM linefeed at region bottom — covered by existing linefeed + scroll region tests
- [x] No bugs found

---

## 02.4 Update Tests & Golden References

- [x] Verify `vttest_origin_mode_matches_normal_80x24` passes
- [x] Verify `vttest_origin_mode_matches_normal_97x33` passes
- [x] Verify `vttest_origin_mode_matches_normal_120x40` passes
- [x] Regenerate text snapshots for screens 01_02 at all sizes — done in Section 01
- [x] Regenerate golden PNGs for screens 01_02 at all sizes — done in Section 01
- [x] Ensure screen 01_01 snapshots/PNGs are unchanged — verified, border tests pass at all sizes

---

## 02.R Third Party Review Findings

- [x] `[TPR-02-001][medium]` Plan metadata out of sync.
  Resolved: Updated index.md and 00-overview.md to show Section 02 as "In Progress". 2026-04-02.
- [x] `[TPR-02-002][low]` `platform_windows/mod.rs` over 500-line limit.
  Resolved: Extracted DWM helpers (set_transitions_enabled, cloak_window, visible_frame_bounds_hwnd, try_dwm_frame_bounds) into `dwm.rs` submodule. mod.rs now 421 lines. 2026-04-02.

---

## 02.N Completion Checklist

- [x] `vttest_origin_mode_matches_normal_*` passes at all 3 sizes
- [x] Screen 01_02 grid text is byte-identical to screen 01_01 at 80x24
- [x] All scroll region edge case unit tests pass
- [x] DECALN + DECOM interaction verified with targeted test
- [x] No regression in screen 01_01 (border test without DECOM)
- [x] Golden PNGs for 01_02 match 01_01 visually
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `/tpr-review` passed — 2 findings, both resolved (plan sync, DWM file split)

**Exit Criteria:** vttest screen 01_02 (origin mode) produces output identical to screen 01_01 (normal mode) at all terminal sizes, verified by `vttest_origin_mode_matches_normal_*` structural assertions.
