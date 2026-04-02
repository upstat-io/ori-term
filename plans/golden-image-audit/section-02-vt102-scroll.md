---
section: "02"
title: "VT102 Insert/Delete with Scroll Regions"
status: not-started
reviewed: false
goal: "vttest VT102 tests 09-11 show correct top-line content when scroll regions are active"
inspired_by:
  - "Alacritty insert_lines/delete_lines (alacritty_terminal/src/grid/mod.rs)"
  - "WezTerm insert_lines (term/src/terminalstate/performer.rs)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Diagnose Scroll Region IL/DL Bug"
    status: not-started
  - id: "02.2"
    title: "Fix and Test"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: VT102 Insert/Delete with Scroll Regions

**Status:** Not Started
**Goal:** vttest VT102 tests 09-14 (the second round, which operate with a scroll region) produce correct output — the top line shows the expected content after IL/DL operations.

**Context:** vttest VT102 tests 01-08 (no scroll region) produce correct output. Tests 09-14 (WITH scroll region via DECSTBM) show wrong top-line content: test 09 expects A's on the top line but shows B's; test 10 expects the top line to start with A but starts with B; test 11 expects "AB" but shows just "B". The bug is in how `insert_lines()` / `delete_lines()` in `grid/scroll/mod.rs` interact with DECSTBM scroll region boundaries.

**Reference implementations:**
- **Alacritty** `alacritty_terminal/src/grid/mod.rs`: IL/DL clamp to scroll region, use `rotate()` on the visible row slice.
- **WezTerm** `term/src/terminalstate/performer.rs`: Explicit scroll region boundary enforcement in insert/delete operations.

**Depends on:** None.

---

## 02.1 Diagnose Scroll Region IL/DL Bug

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/term/handler/mod.rs`

The bug manifests as wrong top-line content after IL/DL within a scroll region. The first round (no region) works, the second round (with region) fails.

- [ ] Read `insert_lines()` and `delete_lines()` in `grid/scroll/mod.rs` (lines 102-123). Trace the exact rotation logic step by step for a concrete case (e.g., 24-line grid, scroll region 1..23, cursor at line 1, insert 1 line).
- [ ] Read `set_scroll_region()` (lines 24-35). Verify the 1-based to 0-based conversion is correct.
- [ ] Read the VTE handler dispatch for IL (`CSI L`) and DL (`CSI M`). Check if cursor position is modified before/after the operation.
- [ ] Compare against Alacritty's implementation — specifically how they handle the scroll region boundaries and cursor position during IL/DL.
- [ ] Write a minimal reproduction: unit test that sets a scroll region, fills rows with distinct content (A, B, C...), performs an insert_lines(1) at line 1, and asserts the top line content.
- [ ] Identify the exact off-by-one or boundary error.

---

## 02.2 Fix and Test

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/tests/vttest.rs`

- [ ] Fix the identified bug in `insert_lines()` / `delete_lines()` / `insert_blank()` / `delete_chars()` as needed
- [ ] Run the reproduction test — verify correct behavior
- [ ] Run vttest PTY tests: `cargo test -p oriterm_core vttest` — all 12 pass
- [ ] Re-render vttest golden images for 08_vt102_09, 08_vt102_10, 08_vt102_11 with `ORITERM_UPDATE_GOLDEN=1`
- [ ] **Visually inspect** each re-rendered image — verify correct top-line content
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green

---

## 02.R Third Party Review Findings

- None.

---

## 02.N Completion Checklist

- [ ] vttest VT102 test 09: top line shows A's (not B's)
- [ ] vttest VT102 test 10: top line starts with A (not B)
- [ ] vttest VT102 test 11: top line shows "AB" (not just "B")
- [ ] First-round tests (01-08) still pass (no regression)
- [ ] All 3 re-rendered golden images visually verified
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** IL/DL operations produce correct results both with and without active scroll regions. vttest VT102 second-round tests match expected output.
