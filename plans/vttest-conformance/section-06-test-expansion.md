---
section: "06"
title: "Test Automation Expansion"
status: not-started
reviewed: true
goal: "Automated vttest coverage for menus 1-8 with structural assertions and golden images at all 3 sizes"
inspired_by:
  - "Existing vttest infrastructure (oriterm_core/tests/vttest.rs)"
  - "Existing GPU golden tests (oriterm/src/gpu/visual_regression/vttest.rs)"
depends_on: ["01", "02", "03", "04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "06.1"
    title: "Menu Navigation Automation (Menus 4-8)"
    status: not-started
  - id: "06.2"
    title: "Structural Assertions per Menu"
    status: not-started
  - id: "06.3"
    title: "Golden Image Generation"
    status: not-started
  - id: "06.4"
    title: "CI Integration"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Test Automation Expansion

**Status:** Not Started
**Goal:** Complete automated vttest coverage for menus 1-8 at 80x24, 97x33, and 120x40 — text snapshots, golden images, and structural assertions for every screen.

**Context:** Menus 1-2 are already automated. This section extends coverage to menus 3-8: character sets (3), double-size characters (4), keyboard (5), terminal reports (6), VT52 mode (7), VT102 features (8). Each menu needs: (a) scripted navigation, (b) text snapshots, (c) golden images, (d) structural assertions where applicable.

**Reference implementations:**
- Existing `VtTestSession` infrastructure in `oriterm_core/tests/vttest.rs`.
- Existing GPU golden pipeline in `oriterm/src/gpu/visual_regression/vttest.rs`.

**Depends on:** Sections 01-04 (terminal size fix unblocks multi-size tests; VTE fixes must land so tests capture correct behavior, not bugs).

---

## 06.1 Menu Navigation Automation (Menus 4-8)

**File(s):** `oriterm_core/tests/vttest.rs`, `oriterm/src/gpu/visual_regression/vttest.rs`

Each menu requires scripted navigation. Some menus have sub-menus or interactive prompts.

- [ ] Menu 4 (double-size characters): navigate with `4\r`, advance through screens with `\r`
  - **Known limitation:** DECDHL/DECDWL escape sequences are NOT implemented in oriterm_core (grep confirms zero matches). These screens will render at normal size. Document as known limitation, capture snapshots for baseline.
- [ ] Menu 5 (keyboard): this menu waits for keypress input — may need special handling (send specific keys and verify echo)
  - Assessment: keyboard tests may be partially automatable (LED tests, auto-repeat)
  - Skip interactive tests that require human judgment
- [ ] Menu 6 (terminal reports): navigate with `6\r`, advance through screens
  - Tests DA, DSR, DECRQM — handlers exist at `handler/status.rs` (DA1/DA2, DSR, report_mode, report_private_mode)
- [ ] Menu 7 (VT52 mode): navigate with `7\r`, advance through screens
  - **Known limitation:** VT52 compatibility mode is NOT implemented in oriterm_core (grep confirms zero matches). This entire menu will fail. Document as known limitation and skip from pass-rate counting.
- [ ] Menu 8 (VT102 features): navigate with `8\r`, advance through screens
  - Tests ICH/DCH/IL/DL — covered by Section 04 fixes
- [ ] Add `run_menuN_*` functions following the existing pattern for each automatable menu
- [ ] Handle menus that require specific keypress responses (menu 5) differently from advance-with-enter menus
- [ ] Menu 5 automatable sub-tests: (1) LED tests -- verify CSI response to DECRQM for keyboard LED modes; (2) auto-repeat -- send a key via PTY, wait 500ms, verify echo count > 1. Skip: keyboard layout tests (require human judgment on physical key mapping).
- [ ] Menu 6 (terminal reports) sub-tests to automate: (1) DA1 -- verify response matches expected format; (2) DA2 -- same; (3) DSR cursor position -- verify row;col matches; (4) DECRQM -- verify mode reporting for known modes; (5) DECID -- verify identification string
- [ ] **Accept VtTestSession duplication**: `VtTestSession` and `PtyResponder` are independently defined in both `oriterm_core/tests/vttest.rs` and `oriterm/src/gpu/visual_regression/vttest.rs`. Extraction is impractical -- the GPU version adds `frame_input()` and `assert_golden()` which depend on GPU types (`FrameInput`, `GpuState`, etc.) not available in `oriterm_core`. The shared core (~100 lines: spawn, drain, wait, send) changes rarely. When adding new menu navigation, duplicate the `run_menuN_*` pattern in both files.

---

## 06.2 Structural Assertions per Menu

**File(s):** `oriterm_core/tests/vttest.rs`

Add programmatic assertions for each menu's key test screens. Not every screen needs a structural assertion -- focus on screens with clear pass/fail criteria described in the vttest prompts.

- [ ] Menu 3: line drawing characters present at expected positions (structural)
- [ ] Menu 4: snapshot-only baseline (DECDHL/DECDWL not implemented — text renders at normal size)
- [ ] Menu 6: terminal report responses match expected format (structural — verify DA/DSR output in grid)
- [ ] Menu 7: snapshot-only baseline (VT52 mode not implemented — document as known limitation)
- [ ] Menu 8: ICH/DCH/IL/DL result verification — character positions after operations (structural)
- [ ] `/tpr-review` checkpoint

### Concrete Test Functions

- [ ] `vttest_menu3_line_drawing_chars` — structural: verify cells at specific positions contain box-drawing Unicode codepoints (U+2500-U+257F range)
- [ ] `vttest_menu6_da1_response_in_grid` — structural: verify the DA response string appears in the grid output (vttest echoes it)
- [ ] `vttest_menu6_dsr_cursor_position` — structural: after DSR 6, verify the cursor position report in grid matches expected format
- [ ] `vttest_menu8_ich_shifts_right` — structural: after ICH, verify characters shifted right by expected count
- [ ] `vttest_menu8_dch_shifts_left` — structural: after DCH, verify characters shifted left with blanks at right margin
- [ ] `vttest_menu8_il_inserts_blank_line` — structural: after IL, verify blank line inserted at cursor row within scroll region
- [ ] `vttest_menu8_dl_deletes_line` -- structural: after DL, verify line removed and blank line added at bottom of scroll region

---

## 06.3 Golden Image Generation

**File(s):** `oriterm/src/gpu/visual_regression/vttest.rs`

Generate golden PNGs for all new menus at all 3 sizes.

- [ ] Add `run_menu3_golden` through `run_menu8_golden` functions
- [ ] Generate golden PNGs: `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests vttest_golden`
- [ ] Verify golden images look correct (spot-check SGR colors, line drawing, cursor positioning)
- [ ] Verify golden images look correct -- compare against xterm reference output where possible
- [ ] Commit golden PNGs to `oriterm/tests/references/` (existing vttest PNGs for menus 1-2 are already here). Estimated ~50-80 new PNGs (menus 3-8 have roughly 5-15 screens each).
- [ ] Verify golden PNG file sizes are reasonable (each should be <100KB for terminal text) -- if larger, investigate whether the test grid dimensions are unnecessarily large
- [ ] Record actual screen counts per menu during this section's work (needed by Section 07 for the scoring table)

---

## 06.4 CI Integration

**File(s):** `.github/workflows/ci.yml`

vttest must be available in CI for the structural tests to run. The golden image tests require GPU (skip in CI if no adapter).

- [ ] Add `vttest` to CI's test-linux `apt-get install` step at `ci.yml:97` (the test job's system dependencies block -- NOT the clippy job at line 53 which doesn't run tests)
- [ ] Verify text-based vttest tests run in CI (no GPU needed — these spawn a PTY and parse VTE output)
- [ ] Verify GPU golden tests gracefully skip in CI if no GPU adapter (they already do via `headless_env()` returning `None`)
- [ ] Note: no macOS CI job exists currently — vttest CI is Linux-only for now
- [ ] Ensure vttest tests respect the timeout policy: `timeout 150 cargo test -p oriterm_core --test vttest` (vttest tests involve PTY I/O with sleep-based synchronization -- verify they complete within 150s)
- [ ] Document: `cargo test -p oriterm_core --test vttest` for text tests, `cargo test -p oriterm --features gpu-tests vttest_golden` for GPU tests (note: GPU tests require `feature = "gpu-tests"` gate and use `headless_env()` graceful skip)

---

## 06.R Third Party Review Findings

- None.

---

## 06.N Completion Checklist

- [ ] Menus 3-8 automated with scripted navigation
- [ ] Text snapshots generated for all menus at all 3 sizes
- [ ] Golden PNGs generated for all menus at all 3 sizes
- [ ] Structural assertions for key screens in each menu
- [ ] vttest available in CI (Linux at minimum)
- [ ] All tests pass locally
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** `cargo test -p oriterm_core --test vttest` runs 50+ tests covering menus 1-8 at 3 sizes. `cargo test -p oriterm -- visual_regression::vttest` generates and compares 100+ golden PNGs (currently 66 exist for menus 1-2 at 3 sizes). Structural assertions catch regressions automatically. Note: menu 2 text snapshots currently only exist at 80x24 (15 snapshots) -- non-80x24 snapshots will be generated after Section 01 fixes terminal size reporting. Actual screen counts per menu are recorded for the Section 07 scoring table.
