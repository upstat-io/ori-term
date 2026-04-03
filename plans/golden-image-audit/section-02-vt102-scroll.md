---
section: "02"
title: "VT102 Insert/Delete Line with Scroll Regions"
status: not-started
reviewed: true
goal: "vttest VT102 second-round screens (08-12) show correct top-line content when scroll regions are active"
inspired_by:
  - "Alacritty insert_blank_lines/delete_lines + scroll_down_relative/scroll_up_relative (alacritty_terminal/src/term/mod.rs:1497-1516, 742-790)"
  - "WezTerm insert_lines (term/src/terminalstate/performer.rs)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Diagnose Scroll Region IL/DL Bug"
    status: not-started
    tests: 3
  - id: "02.2"
    title: "Fix and Test"
    status: not-started
    tests: 15
  - id: "02.3"
    title: "Add Structural Assertions for Second-Round Screens"
    status: not-started
    tests: 9
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: VT102 Insert/Delete Line with Scroll Regions

**Status:** Not Started
**Goal:** vttest VT102 second-round screens (08-12, which operate with a scroll region via DECSTBM) produce correct output — the top line shows the expected content after IL/DL operations.

**Context:** vttest menu 8 has 14 screens. The first 7 (01-07) run without a scroll region and produce correct output. Screens 08-14 repeat the same tests WITH a scroll region set via DECSTBM. Of these, screens 08-12 show wrong top-line content: the A row that should be preserved outside the scroll region is missing, replaced by B's. Screens 13-14 are correct (screen 13 is the ICH stagger, which uses character-level insert operations unaffected by scroll regions; screen 14 is the ICH ANSI test).

**Specific failures observed in current insta snapshots (80x24):**
- **Screen 08** (accordion with scroll region): row 0 shows B's instead of A's.
- **Screen 09** (IL/DL result with scroll region): row 0 shows B's; expected A's. Prompt says "Top line: A's, bottom line: X's".
- **Screen 10** (insert mode with scroll region): row 0 starts with `B*...` instead of `A*...*B`. Prompt says "top line should be 'A*** ... ***B'".
- **Screen 11** (delete char with scroll region): row 0 is `B` instead of `AB`. Prompt says "top line should be 'AB'".
- **Screen 12** (DCH stagger with scroll region): row 0 shows B's instead of A's.

**Bug pattern:** In every failing screen, the A row (which should be outside the scroll region on row 0) is missing, replaced by B content from row 1. This affects ALL second-round screens 08-12 — including screen 12 (DCH stagger), which uses character-level `delete_chars()` and should not interact with scroll regions at all. This means the bug originates in screen 08 (the accordion) and the broken row-0 state cascades to all subsequent screens. The root cause is in screen 08's IL/DL accordion within the scroll region — something about the accordion sequence corrupts or overwrites row 0. The exact mechanism needs diagnosis (see 02.1).

**What is NOT broken:** `insert_blank()` and `delete_chars()` (ICH/DCH — `CSI @`/`CSI P`) are per-character operations on a single row. They do not interact with scroll regions. Screen 14 (ICH test with scroll region) confirms this.

**Reference implementations:**
- **Alacritty** `alacritty_terminal/src/term/mod.rs:1497-1516`: `insert_blank_lines()` calls `scroll_down_relative(origin, lines)` where origin = cursor line. The effective clamp is `min(count, region.end - origin)` — equivalent to what oriterm does. **The clamping logic is equivalent and is not the source of the bug.** Neither Alacritty nor oriterm resets cursor column to 0 in `insert_blank_lines()`.
- **WezTerm** `term/src/terminalstate/performer.rs`: Explicit scroll region boundary enforcement in insert/delete operations.

**Depends on:** None.

---

## 02.1 Diagnose Scroll Region IL/DL Bug

**File(s):** `oriterm_core/src/grid/scroll/mod.rs` (lines 42-123), `oriterm_core/src/grid/navigation/mod.rs` (lines 99-109), `oriterm_core/src/term/handler/mod.rs` (lines 233-241)

The bug manifests as wrong top-line content after IL/DL within a scroll region. The first round (no region) works, the second round (with region) fails. Five screens (08-12) are affected, including screen 12 (DCH stagger) which does not use IL/DL at all — confirming the bug originates in screen 08 (the accordion) and cascades.

**Code review findings:** The `insert_lines()` and `delete_lines()` implementations in `grid/scroll/mod.rs` (lines 102-123) are provably equivalent to Alacritty's clamping and range logic. The bug is NOT in the clamping. The root cause is more subtle — likely in the interaction between the accordion's repeated IL/DL sequence, the scroll region boundaries, and possibly `linefeed()` or `scroll_up()` behavior during the screen fill phase.

**Existing test gap:** `scroll/tests.rs` (1096 lines) has extensive IL/DL tests but ALL operate on the default full-screen scroll region (`0..N`). There is NO test where `insert_lines()` or `delete_lines()` operates inside a sub-region (e.g., `1..4`). The `insert_lines_outside_region_is_noop` test sets `scroll_region = 1..4` but only tests the cursor-outside-region case.

- [ ] Read `insert_lines()` and `delete_lines()` in `grid/scroll/mod.rs` (lines 102-123). Verify range construction and clamping are correct. They are equivalent to Alacritty — this is a confirmation step, not where the bug lives.
- [ ] Read the VTE handler dispatch for IL (`CSI L`) and DL (`CSI M`) in `handler/mod.rs` lines 233-241. Confirm no cursor position modification occurs before/after the grid operation.
- [ ] Read `set_scroll_region()` in `grid/scroll/mod.rs` (lines 24-35). Verify the 1-based to 0-based conversion and that the region spans at least 2 lines.
- [ ] Read `linefeed()` in `grid/navigation/mod.rs` (lines 99-109). Verify: when cursor is at `scroll_region.end - 1`, calls `scroll_up(1)` which scrolls only within the region. Look for any edge case where `linefeed()` could affect row 0 when row 0 is outside the scroll region.
- [ ] Read `scroll_up()` in `grid/scroll/mod.rs` (lines 42-86). When `scroll_region != 0..lines` (sub-region), `is_full_screen` is false, and only `scroll_range_up(start..end, count)` runs. Verify no code path touches rows outside `start..end`.
- [ ] Check whether the handler for IL/DL resets the cursor column to 0. Note: Alacritty also does NOT reset cursor column in `insert_blank_lines()`, so a missing reset is unlikely to be the root cause. Still worth verifying for ECMA-48 compliance.
- [ ] Write a minimal reproduction: handler-level test that feeds `\x1b[2;24r` (DECSTBM, scroll region 0-based `1..24`), writes A's on row 0, fills rows 1-23 with B-X via prints + linefeeds, positions cursor at line 1, performs `\x1b[1L` (IL 1), and asserts row 0 still contains A's.
- [ ] If the single-step reproduction passes, escalate to the full accordion: repeated IL/DL cycles within the scroll region, capturing row 0 content after each cycle. The bug may require multiple cycles to manifest.
- [ ] Also test: does the screen fill itself (writing characters + linefeeds with DECSTBM active) corrupt row 0? Feed the fill sequence with DECSTBM active and verify row 0 is preserved before any IL/DL.
- [ ] Identify the exact root cause. Document it before proceeding to 02.2. Do NOT assume the fix is in `insert_lines`/`delete_lines` — the bug may be in `linefeed()`, `scroll_up()`, `set_scroll_region()`, or the interaction between them.
- [ ] Document root cause in this section as a markdown block: function name, line number, exact mechanism, and why the first-round (no region) passes but the second-round (with region) fails.

---

## 02.2 Fix and Test

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs` (if `linefeed()` is implicated), `oriterm_core/src/grid/scroll/tests.rs`, `oriterm_core/src/term/handler/tests.rs`

**Hygiene note:** `scroll/tests.rs` (1096 lines) and `handler/tests.rs` (5733 lines) are test files, exempt from the 500-line limit.

### Fix

- [ ] Fix the identified bug at whatever location 02.1 diagnosed. The fix may be in `insert_lines()`/`delete_lines()`, `linefeed()`, `scroll_up()`, or their interaction. Do NOT assume the fix is in `grid/scroll/mod.rs` until the root cause is confirmed.
- [ ] Verify modified source files remain under 500 lines. `scroll/mod.rs` is 174 lines and `navigation/mod.rs` is 207 lines — ample headroom.
- [ ] Update 02.1 with a "Root Cause" block documenting the exact fix (function, line, before/after logic).

### Unit tests in `grid/scroll/tests.rs`

**Core sub-region IL/DL tests (the gap that enabled this bug):**

- [ ] `insert_lines_with_scroll_region_preserves_outside` — 5-row grid filled A-E, `scroll_region = 1..5`, cursor at line 1, `insert_lines(1)`. Assert: row 0 = 'A' (outside region, untouched), row 1 = blank (inserted), row 2 = 'B', row 3 = 'C', row 4 = 'D'. E pushed off bottom of scroll region.
- [ ] `delete_lines_with_scroll_region_preserves_outside` — same setup, `delete_lines(1)` at line 1. Assert: row 0 = 'A', row 1 = 'C', row 2 = 'D', row 3 = 'E', row 4 = blank.
- [ ] `insert_lines_at_region_start_preserves_outside` — 6-row grid filled A-F, `scroll_region = 2..5`, cursor at line 2 (region start), `insert_lines(1)`. Assert: row 0 = 'A', row 1 = 'B' (both outside region), row 2 = blank (inserted), row 3 = 'C', row 4 = 'D'. E pushed off. Row 5 = 'F' (below region, untouched).
- [ ] `delete_lines_at_region_start_preserves_outside` — same 6-row grid, cursor at line 2, `delete_lines(1)`. Assert: row 0 = 'A', row 1 = 'B' (above region), row 2 = 'D', row 3 = 'E', row 4 = blank. Row 5 = 'F' (below region, untouched).

**Boundary edge cases:**

- [ ] `insert_lines_cursor_at_region_end_minus_one` — 5-row grid filled A-E, `scroll_region = 1..5`, cursor at line 4 (last line of region), `insert_lines(1)`. Assert: row 0 = 'A', rows 1-3 = B/C/D (untouched within region), row 4 = blank (E pushed off by inserting at the last possible position).
- [ ] `delete_lines_cursor_at_region_end_minus_one` — same setup, `delete_lines(1)` at line 4. Assert: row 0 = 'A', rows 1-3 = B/C/D (untouched), row 4 = blank (E deleted, blank appears at bottom).
- [ ] `insert_lines_count_exceeds_sub_region` — 6-row grid, `scroll_region = 1..4`, cursor at line 2, `insert_lines(100)`. Assert: row 0 = 'A' (above), rows 2-3 = blank (all remaining region lines blanked), row 1 = 'B' (untouched within region above cursor). Row 4 = 'E', row 5 = 'F' (below region, untouched).
- [ ] `delete_lines_count_exceeds_sub_region` — same setup, `delete_lines(100)`. Assert: row 0 = 'A' (above), rows 2-3 = blank, row 1 = 'B' (untouched). Row 4 = 'E', row 5 = 'F' (below, untouched).

**Linefeed with sub-region preserving outside rows:**

- [ ] `linefeed_at_sub_region_bottom_preserves_rows_outside` — 6-row grid filled A-F, `scroll_region = 2..5`, cursor at line 4 (bottom of region), `linefeed()`. Assert: row 0 = 'A', row 1 = 'B' (above region, untouched), row 2 = 'D' (C scrolled off), row 3 = 'E', row 4 = blank. Row 5 = 'F' (below region, untouched).
- [ ] `repeated_linefeeds_in_sub_region_never_touch_outside` — 6-row grid filled A-F, `scroll_region = 2..5`. Cursor at line 4, perform 10 linefeeds. Assert: row 0 = 'A', row 1 = 'B', row 5 = 'F' (all outside rows preserved). All rows in region (2-4) should be blank after 10 linefeeds (all content scrolled off).

**Accordion pattern (direct regression test for the vttest bug):**

- [ ] `accordion_il_dl_cycle_with_sub_region` — 6-row grid, fill all rows with identifiable content, `scroll_region = 1..5`. Perform 3 accordion cycles: cursor to region start, `insert_lines(1)`, cursor to region end-1, `delete_lines(1)`. After each cycle, assert row 0 (above region) and row 5 (below region) are untouched.

### Handler-level tests in `handler/tests.rs`

- [ ] `accordion_with_scroll_region_preserves_row_zero` — feed escape sequences to set DECSTBM (`\x1b[2;24r`), fill row 0 with A's, fill rows 1-23 with B-X via prints + linefeeds, then perform the full accordion cycle (repeated IL/DL pairs). Assert row 0 contains A's after every cycle.
- [ ] `screen_fill_with_scroll_region_preserves_row_zero` — feed DECSTBM, write A's on row 0, then fill remaining rows via prints + linefeeds. Assert row 0 is still A's before any IL/DL. (This isolates whether the fill itself is buggy.)
- [ ] `il_within_scroll_region_preserves_row_zero` — feed `\x1b[2;24r` (DECSTBM), write A's on row 0, write B's on row 1, position cursor at row 1, feed `\x1b[1L` (IL 1). Assert row 0 = A's, row 1 = blank. Simplest handler-level sub-region IL test.
- [ ] `dl_within_scroll_region_preserves_row_zero` — same setup, feed `\x1b[1M` (DL 1) instead. Assert row 0 = A's, row 1 = content that was on row 2.

### Verification

- [ ] Run the reproduction tests — verify correct behavior after the fix.
- [ ] Update insta snapshots: `INSTA_UPDATE=always cargo test -p oriterm_core --test vttest vttest_menu8_80x24`. Visually inspect each updated snapshot for screens 08-12.
- [ ] Re-render GPU golden images: `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm -- vttest_golden_menu8` for screens 08-12. (Note: vttest golden tests do NOT require `--features gpu-tests`; they use `headless_env()` which returns `None` when no GPU is available.)
- [ ] **Visually inspect** each re-rendered golden image — verify correct top-line content.
- [ ] Verify screens 01-07 (first round, no scroll region) still produce correct output (no regression).
- [ ] Verify screens 13-14 still produce correct output (no regression).
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green.

---

## 02.3 Add Structural Assertions for Second-Round Screens

**File(s):** `oriterm_core/tests/vttest.rs` — `assert_vt102_screen_structure()` (line 787)

The existing `assert_vt102_screen_structure()` function has match arms only for screens 2, 3, 4, 5. Screen 1 (first-round accordion result) and all second-round screens (08-14) fall through to `_ => {}` with no assertions. A future regression could break screens 08-12 and the tests would still pass as long as insta snapshots are blindly re-approved.

**Hygiene note:** `vttest.rs` is 847 lines. It is an integration test file, not exempt from the 500-line limit. Adding ~40 lines of match arms is acceptable, but if future sections add multi-size coverage the file should be split.

### First-round gap

- [ ] Add match arm for screen 1: top row should be all A's (`grid[0].iter().all(|&c| c == 'A')`). This is the first-round accordion result — currently has no structural assertion despite being the most basic IL/DL correctness check.

### Second-round assertions (screens 08-12)

- [ ] Add match arm for screen 8: top row should be all A's (mirrors screen 1). `grid[0].iter().all(|&c| c == 'A')`.
- [ ] Add match arm for screen 9: top row should be all A's (`grid[0].iter().all(|&c| c == 'A')`). Note: the bottom row in the second round may not be all X's at full width because the X fill happens within the scroll region. Assert `grid[0]` only; do NOT mirror the screen 2 bottom-row assertion unless verified post-fix.
- [ ] Add match arm for screen 10: `grid[0][0] == 'A'` and last non-space on row 0 is 'B' (mirrors screen 3 assertion).
- [ ] Add match arm for screen 11: `grid[0][0] == 'A'` and `grid[0][1] == 'B'` (mirrors screen 4 assertion).
- [ ] Add match arm for screen 12: `grid[0][0]` should be 'A' (not 'B'). The stagger pattern (row 0 longer than row 1) should hold, but verify after the fix — the second-round stagger may differ slightly from the first round due to scroll region effects.

### Correct-screens regression guard

- [ ] Add match arm for screen 13 (ICH stagger with scroll region): `grid[0][0]` should be printable (not blank/B). The ICH stagger is character-level, unaffected by scroll regions. Adding an assertion prevents a future regression from silently breaking this screen.
- [ ] Add match arm for screen 14 (ICH ANSI test with scroll region): verify row 0 has expected content. The specific assertion depends on post-fix visual inspection — add a placeholder assertion (e.g., `grid[0][0] != ' '`) and tighten after inspection.

### Post-fix tightening

- [ ] After the fix lands, visually inspect the corrected snapshots for screens 08-12 and tighten assertions if more specific patterns are apparent.
- [ ] Replace the `_ => {}` catch-all in the match with a bounded check: for screens `1..=14`, every screen should have an assertion. Add a comment documenting that screens 6, 7 are intentionally not asserted (they are line-feed-only screens with no distinctive row-0 pattern). Screens outside `1..=14` can remain `_ => {}`.
- [ ] `./build-all.sh`, `./test-all.sh` green.

**Note:** The menu 8 vttest currently runs only at 80x24 (one test function `vttest_menu8_80x24`), unlike menus 1 and 2 which run at 3 sizes. Adding multi-size coverage (97x33, 120x40) is valuable but out of scope for this section — it's a separate enhancement.

---

## 02.R Third Party Review Findings

- None.

---

## 02.N Completion Checklist

### Root cause and fix

- [ ] Root cause documented in 02.1 with markdown block (function, line, mechanism, why first-round passes)
- [ ] Bug fixed — may be in `grid/scroll/mod.rs`, `grid/navigation/mod.rs`, or handler-level interaction
- [ ] Modified source files remain under 500 lines

### vttest screen correctness

- [ ] vttest VT102 screen 08: top row shows A's (not B's) — accordion with scroll region
- [ ] vttest VT102 screen 09: top row shows A's (not B's) — IL/DL result with scroll region
- [ ] vttest VT102 screen 10: top row starts with A and ends with B — insert mode with scroll region
- [ ] vttest VT102 screen 11: top row shows "AB" (not just "B") — delete char with scroll region
- [ ] vttest VT102 screen 12: top row shows A's (not B's) — DCH stagger with scroll region
- [ ] First-round screens (01-07) still pass (no regression)
- [ ] Screens 13-14 still pass (no regression)

### Unit tests in `grid/scroll/tests.rs`

- [ ] `insert_lines_with_scroll_region_preserves_outside` — core sub-region IL test
- [ ] `delete_lines_with_scroll_region_preserves_outside` — core sub-region DL test
- [ ] `insert_lines_at_region_start_preserves_outside` — region start boundary, rows above AND below preserved
- [ ] `delete_lines_at_region_start_preserves_outside` — region start boundary DL variant
- [ ] `insert_lines_cursor_at_region_end_minus_one` — last-line-of-region edge case
- [ ] `delete_lines_cursor_at_region_end_minus_one` — last-line-of-region edge case
- [ ] `insert_lines_count_exceeds_sub_region` — count clamping within sub-region
- [ ] `delete_lines_count_exceeds_sub_region` — count clamping within sub-region
- [ ] `linefeed_at_sub_region_bottom_preserves_rows_outside` — linefeed scroll isolation
- [ ] `repeated_linefeeds_in_sub_region_never_touch_outside` — stress test for linefeed isolation
- [ ] `accordion_il_dl_cycle_with_sub_region` — direct regression test for vttest accordion pattern

### Handler-level tests in `handler/tests.rs`

- [ ] `accordion_with_scroll_region_preserves_row_zero` — full accordion via escape sequences
- [ ] `screen_fill_with_scroll_region_preserves_row_zero` — isolates fill phase from IL/DL
- [ ] `il_within_scroll_region_preserves_row_zero` — simplest handler-level sub-region IL
- [ ] `dl_within_scroll_region_preserves_row_zero` — simplest handler-level sub-region DL

### Structural assertions in `assert_vt102_screen_structure()`

- [ ] Screen 1 assertion added (first-round accordion: row 0 = all A's)
- [ ] Screens 08-12 assertions added (second-round screens)
- [ ] Screens 13-14 assertions added (regression guard for currently-correct screens)
- [ ] `_ => {}` catch-all documented (screens 6, 7 intentionally unasserted; screen range noted)

### Visual verification and build

- [ ] All 5 re-rendered insta snapshots visually verified (08-12)
- [ ] All 5 re-rendered GPU golden images visually verified (08-12)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** IL/DL operations produce correct results both with and without active scroll regions. vttest VT102 second-round screens 08-12 match expected output. Structural assertions prevent future regressions from being silently accepted via snapshot approval. Every behavior change has a corresponding unit test that would fail if the bug regressed.
