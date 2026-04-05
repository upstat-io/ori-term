---
section: "02"
title: "Basic Scenario Suite"
status: in-progress
reviewed: true
goal: "Create foundational scenario files for C0 controls and basic CSI sequences that validate the harness end-to-end and establish the scenario authoring pattern"
success_criteria:
  - "C0 control scenarios pass: CR, LF, BS, TAB, BEL, FF, VT, SO, SI (8 scenarios)"
  - "CSI cursor movement scenarios pass: CUP, CUU, CUD, CUF, CUB, VPA, HPA, CHA with boundary checks"
  - "CSI erase scenarios pass: ED modes 0-3 (including ED 3 scrollback erase), EL modes 0-2"
  - "CSI insert/delete scenarios pass: ICH, DCH, IL, DL with content verification (scroll region interactions deferred to Section 04)"
  - "ESC sequence scenarios pass: DECSC/DECRC (save/restore cursor), RIS, SCS (G0/G1 charset designation), IND, RI"
  - "All scenarios run at 80x24; cursor movement also at 97x33 and 120x40"
  - "30+ scenarios pass with insta golden snapshots"
  - "Satisfies mission criteria: C0, basic CSI, and ESC scenario coverage"
inspired_by:
  - "WezTerm test organization (term/src/test/c0.rs, c1.rs, csi.rs) — by sequence type"
  - "ori_term handler/tests.rs — individual sequence tests as reference for expected behavior"
depends_on: ["01"]
third_party_review:
  status: findings
  updated: 2026-04-05
sections:
  - id: "02.1"
    title: "C0 Control Character Scenarios"
    status: complete
  - id: "02.2"
    title: "CSI Cursor Movement Scenarios"
    status: complete
  - id: "02.3"
    title: "CSI Erase Scenarios"
    status: complete
  - id: "02.4"
    title: "CSI Insert/Delete Scenarios"
    status: complete
  - id: "02.5"
    title: "ESC Sequence Scenarios"
    status: complete
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Basic Scenario Suite

**Status:** Not Started
**Goal:** Create the foundational scenario files that validate the TeseqHarness works end-to-end and establish the pattern for all subsequent scenario sections. These scenarios cover well-understood sequences where the expected behavior is already tested by handler unit tests — the value here is validating the harness pipeline, establishing the authoring pattern, and creating human-readable scenario documentation.

**Success Criteria:**

- [x] 8 C0 control scenarios pass with grid + event snapshots (cr, lf, bs, tab, bel, ff, vt, so_si)
- [x] 8+ CSI cursor movement scenarios pass (including boundary edge cases)
- [x] 7+ CSI erase scenarios pass (ED modes 0-3 and EL variants)
- [x] 4+ CSI insert/delete scenarios pass
- [x] 5+ ESC sequence scenarios pass (save/restore, charset, IND, RI)
- [x] Total: 30+ scenarios, all with insta golden snapshots
- [x] Satisfies mission criteria for C0, cursor, erase, insert/delete, and ESC coverage

**Context:** The existing handler unit tests (`oriterm_core/src/term/handler/tests.rs`, 5,860 lines) already validate each of these sequences individually via byte-feed + grid-cell assertions. This section's scenarios deliberately overlap with handler tests for three specific purposes: (1) validating the teseq harness pipeline end-to-end before building complex scenarios, (2) establishing the scenario authoring pattern that Sections 03-06 follow, and (3) producing full-grid insta snapshots (vs. handler tests' per-cell assertions) that catch grid-wide side effects. If a scenario would test nothing beyond what handler/tests.rs already covers AND its only purpose is coverage, it does not belong here — keep the scenario count lean. Multi-sequence interaction scenarios (the framework's primary value-add) come in Sections 04-06.

**Assertion pipeline scope:** Section 02 scenarios use grid text snapshots, cursor position, and event sequence assertions. This pipeline validates grid content and cursor state but cannot directly observe mode flags (DECOM, DECAWM, IRM), wrap-pending state, or saved cursor attributes (SGR, charset). These state dimensions require either (a) assertion helpers added in later sections that inspect `Term` mode state, or (b) workflow scenarios where mode effects are indirectly visible through grid content changes (Section 04/06 approach). This is not a gap for Section 02 — all C0/CSI/ESC sequences here produce observable grid content changes.

**Reference implementations:**
- **WezTerm** `term/src/test/c0.rs`: C0 tests organized by control character (BS, LF, CR, TAB)
- **WezTerm** `term/src/test/c1.rs`: C1 tests (IND, NEL, HTS, RI)
- **ori_term** `handler/tests.rs:70-120`: `hello_places_cells_and_advances_cursor`, `carriage_return_overwrites`, `tab_advances_to_column_8` — reference for expected behavior

**Depends on:** Section 01 (TeseqHarness infrastructure).

**Snapshot location:** All insta snapshots land in `oriterm_core/tests/teseq/harness/snapshots/` because `insta::assert_snapshot!` is called from `harness/assertions.rs`. Family modules delegate to `harness::assert_spec()` which calls the assertion functions in `assertions.rs`, so the call-site module path is always `teseq::harness::assertions`. Snapshot filenames follow the pattern `teseq__harness__assertions__<name>.snap`. This means snapshot names must be globally unique across all families — the plan enforces this by prefixing every name with the family (e.g., `c0_cr_grid`, `csi_cursor_cup_basic_grid`).

**Snapshot workflow:** When creating new scenarios, snapshots do not exist yet. First run generates `.snap.new` files. Accept them with `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq` or `cargo insta review`. After accepting, inspect each `.snap` file to verify the grid content matches the expected trace. Commit the `.snap` files alongside the scenario files.

**Directory creation:** The `scenarios/c0/` directory already exists (from the Section 01 smoke test). The following directories must be created before writing scenario files: `scenarios/csi/cursor/`, `scenarios/csi/erase/`, `scenarios/csi/insert_delete/`, `scenarios/esc/`. Create them as the first step in each subsection.

**main.rs housekeeping:** Section 01 added `#![allow(dead_code)]` to suppress warnings for the incrementally-built harness. Most harness code becomes reachable through the family modules, but some items remain unused until later sections (`discover_scenarios` used in Section 07, `teseq_available` in Section 03, `RecordedListener::pty_writes` in Section 03). Keep the `#![allow(dead_code)]` attribute in `main.rs` until the final section removes it. Migrate the `smoke_bel` test from `main.rs` into the `c0` family module (remove it from `main.rs` entirely) — the `c0::bel()` test serves the same purpose and having both is redundant. Delete the old `smoke_bel_grid` and `smoke_bel_events` snapshot files. After migration, `main.rs` should contain only `#![allow(dead_code)]`, `mod harness;`, and the five family `mod` declarations.

---

## 02.1 C0 Control Character Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/c0/*.teseq`, `oriterm_core/tests/teseq/c0.rs`

Create scenario files for each C0 control character that ori_term handles. Each `.teseq` file describes the input sequence, and the insta snapshot captures the resulting grid state.

**CRITICAL: `|text|.` produces LF only (0x0A), NOT CR+LF.** LF in ori_term preserves the cursor column (confirmed in handler tests: `linefeed_moves_down`, `hello_newline_world`). This means `|hello|.` followed by `|world|` writes "world" starting at the column where "hello" left the cursor, NOT at column 0. For multi-line scenarios where each line must start at column 0, use explicit CR+LF:
```
|Line 1|
. CR/^M LF/^J
|Line 2|
```
This produces `Line 1\r\nLine 2` — the CR resets to column 0 before the LF moves down.

- [x] Create family module `oriterm_core/tests/teseq/c0.rs` with `run_scenario` helper and one `#[test]` per scenario:
  ```rust
  //! C0 control character scenarios.
  use std::path::Path;
  use super::harness::{self, TeseqHarness, reseq_available};

  fn run_scenario(name: &str) {
      if !reseq_available() {
          eprintln!("reseq not installed, skipping");
          return;
      }
      let path = Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("tests/teseq/scenarios/c0")
          .join(format!("{name}.teseq"));
      let mut h = TeseqHarness::from_scenario(&path);
      let outcome = h.run(&path);
      harness::assert_spec(&outcome, h.spec(), &format!("c0_{name}"));
  }

  #[test] fn cr() { run_scenario("cr"); }
  #[test] fn lf() { run_scenario("lf"); }
  #[test] fn bs() { run_scenario("bs"); }
  #[test] fn tab() { run_scenario("tab"); }
  #[test] fn bel() { run_scenario("bel"); }
  #[test] fn ff() { run_scenario("ff"); }
  #[test] fn vt() { run_scenario("vt"); }
  #[test] fn so_si() { run_scenario("so_si"); }
  ```

- [x] Create scenario files (each `.teseq` with optional `.toml` sidecar):

  **`cr.teseq`** — Carriage return moves cursor to column 0:
  ```
  |hello|
  . CR/^M
  |world|
  ```
  `cr.toml`: `[expect] cursor = { col = 5, line = 0 }`
  Trace: "hello" -> col=5. CR -> col=0. "world" -> col=5 (overwrites "hello"). Grid shows "world" on line 0.

  **`lf.teseq`** — Line feed moves cursor down (column preserved):
  ```
  |hello|
  . LF/^J
  |world|
  ```
  `lf.toml`: `[expect] cursor = { col = 10, line = 1 }`
  Trace: "hello" -> col=5, line=0. LF preserves column -> col=5, line=1. "world" writes at cols 5-9 on line 1, cursor lands at col=10. Grid shows "hello" on line 0 at cols 0-4, "world" on line 1 at cols 5-9.

  **`bs.teseq`** — Backspace moves cursor left:
  ```
  |hello|
  . BS/^H
  |X|
  ```
  `bs.toml`: `[expect] cursor = { col = 5, line = 0 }`
  Trace: "hello" -> col=5. BS -> col=4. "X" overwrites at col=4 -> col=5. Grid shows "hellX" on line 0.

  **`tab.teseq`** — Tab advances to next tab stop (column 8):
  ```
  . HT/^I
  |X|
  ```
  `tab.toml`: `[expect] cursor = { col = 9, line = 0 }`
  Trace: TAB from col=0 -> col=8 (first tab stop). "X" at col=8 -> col=9.

  **`bel.teseq`** — Bell triggers Bell event (already created as smoke test):
  ```
  |Hello|
  . BEL/^G
  | World|
  ```
  `bel.toml`: `[expect] cursor = { col = 11, line = 0 } events = ["Bell"]`
  Trace: "Hello" (5 chars) + BEL (no movement) + " World" (6 chars) = col 11.
  Note: This scenario already exists from Section 01 (`bel.teseq` + `bel.toml` in `scenarios/c0/`). No new files needed. The `c0::bel()` test replaces the `smoke_bel` test in `main.rs` (remove `smoke_bel` as part of the main.rs housekeeping above). The old `smoke_bel_grid` and `smoke_bel_events` snapshot files in `harness/snapshots/` should be deleted after the migration since `c0::bel` generates `c0_bel_grid` and `c0_bel_events` snapshots instead.

  **`ff.teseq`** — Form feed same as LF (confirmed in handler test `form_feed_same_as_lf`):
  ```
  |hello|
  . FF/^L
  |world|
  ```
  `ff.toml`: `[expect] cursor = { col = 10, line = 1 }`
  Trace: Same as lf.teseq. "hello" -> col=5, line=0. FF -> col=5, line=1. "world" at cols 5-9 -> col=10, line=1.

  **`vt.teseq`** — Vertical tab same as LF (confirmed in handler test `vertical_tab_same_as_lf`):
  ```
  |hello|
  . VT/^K
  |world|
  ```
  `vt.toml`: `[expect] cursor = { col = 10, line = 1 }`
  Trace: Same as lf.teseq. "hello" -> col=5, line=0. VT -> col=5, line=1. "world" at cols 5-9 -> col=10, line=1.

  **`so_si.teseq`** — Shift Out activates G1, Shift In reverts to G0:
  ```
  . SO/^N
  |qqqqq|
  . SI/^O
  |--------|
  ```
  `so_si.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b)0"]

  [expect]
  cursor = { col = 13, line = 0 }
  ```
  Pre-feed designates G1 as DEC Special Graphics (`ESC ) 0`). Verified: `ESC )` sets G1, `ESC (` sets G0 (confirmed in handler tests `so_activates_g1_charset`, `si_activates_g0_charset`). After SO, `qqqqq` (5 chars) maps to line-drawing characters in DEC Special Graphics. After SI, `--------` (8 chars) renders as literal ASCII dashes. All content on line 0 (no LF), cursor at col=13. Grid snapshot captures the Unicode line-drawing characters followed by dashes.

- [x] Register family module in `main.rs`: add `mod c0;`
- [x] Remove `smoke_bel` test function and its imports from `main.rs` (replaced by `c0::bel`)
- [x] Delete old snapshot files: `harness/snapshots/teseq__harness__assertions__smoke_bel_grid.snap` and `teseq__harness__assertions__smoke_bel_events.snap`
- [x] Accept new snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq -- c0`
- [x] Inspect each `.snap` file in `harness/snapshots/` to verify grid content matches the trace in this plan
- [x] Verify all C0 scenarios pass: `timeout 150 cargo test -p oriterm_core --test teseq -- c0`

---

## 02.2 CSI Cursor Movement Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/cursor/*.teseq`, `oriterm_core/tests/teseq/csi_cursor.rs`

Cursor movement sequences with boundary conditions.

- [x] Create scenario directory: `mkdir -p oriterm_core/tests/teseq/scenarios/csi/cursor`
- [x] Create family module `oriterm_core/tests/teseq/csi_cursor.rs` with `run_scenario` helper (same pattern as `c0.rs`, path = `scenarios/csi/cursor/{name}.teseq`, prefix = `csi_cursor_{name}`)
- [x] Create scenario files:

  **`cup_basic.teseq`** — CUP moves cursor to absolute position:
  ```
  : Esc [ 5 ; 10 H
  |X|
  ```
  `cup_basic.toml`: `[expect] cursor = { col = 10, line = 4 }`
  Trace: CUP(5,10) -> VTE passes goto(4, 9). Cursor at (4, 9). "X" at col=9 -> col=10, line=4.

  **`cup_origin.teseq`** — CUP to (1,1) is home:
  ```
  |padding text|
  : Esc [ 1 ; 1 H
  |X|
  ```
  `cup_origin.toml`: `[expect] cursor = { col = 1, line = 0 }`
  Trace: "padding text" on line 0. CUP(1,1) -> goto(0, 0). "X" at (0,0) -> col=1.

  **`cup_clamp.teseq`** — CUP beyond grid clamps to last row/col:
  ```
  : Esc [ 999 ; 999 H
  ```
  `cup_clamp.toml`: `[expect] cursor = { col = 79, line = 23 }`
  Trace: CUP(999,999) -> goto(998, 998). `goto_origin_aware` clamps: line=min(998, 23)=23, col=min(998, 79)=79. Confirmed by handler test `cha_overflow_clamps_to_last_column` and grid navigation test `move_to_clamps_out_of_bounds`.

  **`cuu_cud.teseq`** — CUU/CUD relative movement:
  ```
  : Esc [ 10 ; 10 H
  : Esc [ 3 A
  : Esc [ 5 B
  ```
  `cuu_cud.toml`: `[expect] cursor = { col = 9, line = 11 }`
  Trace: CUP(10,10) -> (9, 9). CUU 3 -> line=6. CUD 5 -> line=11. Col stays at 9.

  **`cuf_cub.teseq`** — CUF/CUB horizontal movement:
  ```
  : Esc [ 1 ; 1 H
  : Esc [ 20 C
  : Esc [ 5 D
  ```
  `cuf_cub.toml`: `[expect] cursor = { col = 15, line = 0 }`
  Trace: CUP(1,1) -> (0, 0). CUF 20 -> col=20. CUB 5 -> col=15.

  **`vpa.teseq`** — VPA (vertical position absolute, CSI d):
  ```
  : Esc [ 10 ; 5 H
  : Esc [ 15 d
  ```
  `vpa.toml`: `[expect] cursor = { col = 4, line = 14 }`
  Trace: CUP(10,5) -> (9, 4). VPA 15 -> VTE calls `goto_line(14)` which calls `goto_origin_aware(14, current_col=4)` -> line=14, col preserved=4. Confirmed by handler test `origin_mode_vpa_relative_to_scroll_region` (col preservation).

  **`hpa.teseq`** — HPA (horizontal position absolute, CSI `` ` ``):
  ```
  : Esc [ 10 ; 5 H
  : Esc [ 30 `
  ```
  `hpa.toml`: `[expect] cursor = { col = 29, line = 9 }`
  Trace: CUP(10,5) -> (9, 4). HPA 30 -> VTE calls `goto_col(29)` -> col=29, line preserved=9. Note: HPA uses backtick (`` ` ``, 0x60), NOT `G`. Both CHA and HPA map to `goto_col()` in VTE dispatch, but they use different final bytes. In teseq, use `` : Esc [ 30 ` `` for HPA vs `: Esc [ 30 G` for CHA.

  **`cha.teseq`** — CHA (cursor horizontal absolute, CSI G):
  ```
  : Esc [ 5 ; 20 H
  : Esc [ 1 G
  ```
  `cha.toml`: `[expect] cursor = { col = 0, line = 4 }`
  Trace: CUP(5,20) -> (4, 19). CHA 1 -> VTE calls `goto_col(0)` -> col=0. Line preserved=4. Confirmed by handler test `cha_default_param_goes_to_column_0`.

- [x] Create multi-size test variants for cursor clamping (97x33, 120x40).
  Multi-size variants duplicate the `.teseq` content with a different `.toml` sidecar. For small scenario files (< 10 lines), this duplication is acceptable and simpler than a shared-content mechanism:
  ```
  cup_clamp_97x33.teseq: (same content as cup_clamp.teseq)
  cup_clamp_97x33.toml:  [terminal] cols = 97 rows = 33
                          [expect] cursor = { col = 96, line = 32 }
  cup_clamp_120x40.teseq: (same content)
  cup_clamp_120x40.toml:  [terminal] cols = 120 rows = 40
                           [expect] cursor = { col = 119, line = 39 }
  ```
  The `csi_cursor.rs` family module must include `#[test]` functions for these variants:
  ```rust
  #[test] fn cup_clamp_97x33() { run_scenario("cup_clamp_97x33"); }
  #[test] fn cup_clamp_120x40() { run_scenario("cup_clamp_120x40"); }
  ```

- [x] Register family module in `main.rs`: add `mod csi_cursor;`
- [x] Accept new snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq -- csi_cursor`
- [x] Inspect each new `.snap` file to verify grid content matches traces
- [x] Verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_cursor`

- [ ] **TPR checkpoint** — `/tpr-review` covering 02.1-02.2 implementation work

---

## 02.3 CSI Erase Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/erase/*.teseq`, `oriterm_core/tests/teseq/csi_erase.rs`

Erase operations (ED and EL) with pre-populated grid content.

**IMPORTANT:** Multi-line grid content MUST use explicit CR+LF (`. CR/^M LF/^J`) between lines, not `|text|.` (which produces LF-only and preserves the cursor column). See the critical note in Section 02.1.

- [x] Create scenario directory: `mkdir -p oriterm_core/tests/teseq/scenarios/csi/erase`
- [x] Create family module `oriterm_core/tests/teseq/csi_erase.rs` with `run_scenario` helper (same pattern as `c0.rs`, path = `scenarios/csi/erase/{name}.teseq`, prefix = `csi_erase_{name}`). Standard `#[test]` functions for: `ed_below`, `ed_above`, `ed_all`, `el_right`, `el_left`, `el_all`. The `ed_scrollback` test uses a custom function (see below).
- [x] Create scenario files:

  **`ed_below.teseq`** — ED 0 (erase below cursor):
  ```
  |AAAAAAAAAA|
  . CR/^M LF/^J
  |BBBBBBBBBB|
  . CR/^M LF/^J
  |CCCCCCCCCC|
  : Esc [ 2 ; 5 H
  : Esc [ 0 J
  ```
  `ed_below.toml`: `[expect] cursor = { col = 4, line = 1 }`
  Trace: "AAAAAAAAAA" on line 0 at col 0. CR+LF -> col=0, line=1. "BBBBBBBBBB" on line 1. CR+LF -> col=0, line=2. "CCCCCCCCCC" on line 2. CUP(2,5) -> line=1, col=4. ED 0 erases from (1,4) to end of display. Cursor stays at (4, 1). Grid snapshot shows: line 0 = "AAAAAAAAAA", line 1 = "BBBB" + blanks, lines 2+ = blank.

  **`ed_above.teseq`** — ED 1 (erase above cursor):
  ```
  |AAAAAAAAAA|
  . CR/^M LF/^J
  |BBBBBBBBBB|
  . CR/^M LF/^J
  |CCCCCCCCCC|
  : Esc [ 2 ; 5 H
  : Esc [ 1 J
  ```
  `ed_above.toml`: `[expect] cursor = { col = 4, line = 1 }`
  Trace: Same layout. CUP(2,5) -> line=1, col=4. ED 1 erases from start of display through cursor position inclusive. Cursor stays at (4, 1). Grid: line 0 = blank, line 1 = blanks at cols 0-4 (inclusive) + "BBBBB" (cols 5-9), line 2 = "CCCCCCCCCC". Confirmed by grid test `erase_display_above` (cursor col is erased, col+1 preserved).

  **`ed_all.teseq`** — ED 2 (erase entire display):
  ```
  |AAAAAAAAAA|
  . CR/^M LF/^J
  |BBBBBBBBBB|
  : Esc [ 2 J
  ```
  `ed_all.toml`: `[expect] cursor = { col = 10, line = 1 }`
  Trace: "AAAAAAAAAA" on line 0 (10 chars -> col=10). CR+LF -> col=0, line=1. "BBBBBBBBBB" on line 1 (10 chars -> col=10). ED 2 erases entire display. Cursor stays at (10, 1). Grid all blank. Confirmed by handler test `ed_clears_screen`.

  **`ed_scrollback.teseq`** — ED 3 (erase scrollback buffer):
  The scenario must first push content into scrollback (by writing more lines than the visible grid), then issue ED 3 to clear it. A 24-row terminal with 30 lines of CR+LF content puts **7 lines** into scrollback (see trace below).

  The `.teseq` file writes 30 lines using CR+LF, then issues ED 3:
  ```
  |Line 01|
  . CR/^M LF/^J
  |Line 02|
  . CR/^M LF/^J
  |Line 03|
  . CR/^M LF/^J
  ... (Lines 04-29 follow the same |Line XX| + CR/^M LF/^J pattern)
  |Line 30|
  . CR/^M LF/^J
  : Esc [ 3 J
  ```
  The file is long (62 lines for 30 lines of content + CR+LF pairs + the ED 3 command) but straightforward. Each line pair is `|Line XX|` followed by `. CR/^M LF/^J`.

  Scrollback trace (30 CR+LF lines in 24-row terminal):
  - Lines 01-23 fill rows 0-22. After each CR+LF, cursor advances to next row.
  - "Line 24" on row 23. CR+LF: at bottom, LF scrolls. Scrollback: 1 (Line 01). Cursor: (0, 23).
  - "Line 25" + CR+LF: scrollback=2 (Lines 01-02).
  - "Line 26" + CR+LF: scrollback=3.
  - "Line 27" + CR+LF: scrollback=4.
  - "Line 28" + CR+LF: scrollback=5.
  - "Line 29" + CR+LF: scrollback=6.
  - "Line 30" + CR+LF: scrollback=7 (Lines 01-07).
  After all 30 lines: **7 lines in scrollback (Lines 01-07)**, viewport shows Lines 08-30 on rows 0-22, row 23 blank.

  `ed_scrollback.toml`:
  ```toml
  [terminal]
  scrollback = 100

  [expect]
  cursor = { col = 0, line = 23 }
  ```
  After ED 3: scrollback cleared (scrollback_len = 0), visible content preserved. Grid snapshot validates visible content. The `csi_erase_ed_scrollback` test must also call `harness::assert_scrollback_empty(&outcome)` (defined in Section 01.5). `ScenarioOutcome::scrollback_len` is already populated from `content.scrollback_len` (Section 01.4).

  **`el_right.teseq`** — EL 0 (erase to right of cursor):
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 0 K
  ```
  `el_right.toml`: `[expect] cursor = { col = 4, line = 0 }`
  Trace: "AAAAAAAAAA" on line 0 (cols 0-9). CUP(1,5) -> (0, 4). EL 0 erases cols 4-79 on line 0. Cursor stays at (4, 0). Grid: "AAAA" + blanks. Confirmed by handler test `el_clears_to_end_of_line`.

  **`el_left.teseq`** — EL 1 (erase to left of cursor):
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 1 K
  ```
  `el_left.toml`: `[expect] cursor = { col = 4, line = 0 }`
  Trace: "AAAAAAAAAA" on line 0. CUP(1,5) -> (0, 4). EL 1 erases cols 0-4 on line 0. Cursor stays at (4, 0). Grid: blanks + "AAAAA" (cols 5-9).

  **`el_all.teseq`** — EL 2 (erase entire line):
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 2 K
  ```
  `el_all.toml`: `[expect] cursor = { col = 4, line = 0 }`
  Trace: "AAAAAAAAAA" on line 0. CUP(1,5) -> (0, 4). EL 2 erases entire line 0. Cursor stays at (4, 0). Grid: all blank on line 0.

- [x] Each erase scenario's insta snapshot captures the grid after erasure, making the erased region visible as blank cells.
- [x] The `ed_scrollback` test in `csi_erase.rs` must NOT use `run_scenario()`. It needs a custom `#[test]` function that calls `harness::assert_spec()` AND `harness::assert_scrollback_empty(&outcome)`:
  ```rust
  #[test]
  fn ed_scrollback() {
      if !reseq_available() {
          eprintln!("reseq not installed, skipping");
          return;
      }
      let path = Path::new(env!("CARGO_MANIFEST_DIR"))
          .join("tests/teseq/scenarios/csi/erase/ed_scrollback.teseq");
      let mut h = TeseqHarness::from_scenario(&path);
      let outcome = h.run(&path);
      harness::assert_spec(&outcome, h.spec(), "csi_erase_ed_scrollback");
      harness::assert_scrollback_empty(&outcome);
  }
  ```
- [x] Register family module in `main.rs`: add `mod csi_erase;`
- [x] Accept new snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq -- csi_erase`
- [x] Inspect each new `.snap` file to verify grid content matches traces
- [x] Verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_erase`

---

## 02.4 CSI Insert/Delete Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/insert_delete/*.teseq`, `oriterm_core/tests/teseq/csi_insert_delete.rs`

**Scope note:** The mission criterion says "ICH, DCH, IL, DL with scroll region interactions." This section covers basic insert/delete without scroll regions — validating the harness pipeline with well-understood sequences. IL/DL behavior within DECSTBM scroll regions (where lines outside the region must NOT shift) is a mode-interaction concern that belongs in Section 04 (Mode Interaction Scenarios). Section 04 must include IL/DL-within-scroll-region scenarios to satisfy the mission criterion fully.

- [x] Create scenario directory: `mkdir -p oriterm_core/tests/teseq/scenarios/csi/insert_delete`
- [x] Create family module `oriterm_core/tests/teseq/csi_insert_delete.rs` with `run_scenario` helper (path = `scenarios/csi/insert_delete/{name}.teseq`, prefix = `csi_insert_delete_{name}`)
- [x] Create scenario files:

  **`ich.teseq`** — ICH (insert characters, shift right):
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 3 @
  ```
  `ich.toml`: `[expect] cursor = { col = 3, line = 0 }`
  Trace: "ABCDEFGHIJ" on line 0 (single-line, no LF needed). CUP(1,4) -> (0, 3). ICH 3 inserts 3 blanks at col 3, shifting D-J right. Cursor stays at (3, 0). Grid: "ABC" + 3 blanks + "DEFGHIJ" on 80-col terminal. Confirmed by handler test `ich_inserts_5_blanks` (different count but same mechanism).

  **`dch.teseq`** — DCH (delete characters, shift left):
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 3 P
  ```
  `dch.toml`: `[expect] cursor = { col = 3, line = 0 }`
  Trace: "ABCDEFGHIJ" on line 0. CUP(1,4) -> (0, 3). DCH 3 deletes 3 chars at col 3 (D, E, F), shifting G-J left. Cursor stays at (3, 0). Grid: "ABCGHIJ" + trailing blanks. Confirmed by handler test `dch_deletes_3_chars`.

  **`il.teseq`** — IL (insert lines, shift down):
  ```
  |Line 1|
  . CR/^M LF/^J
  |Line 2|
  . CR/^M LF/^J
  |Line 3|
  . CR/^M LF/^J
  |Line 4|
  : Esc [ 2 ; 1 H
  : Esc [ 2 L
  ```
  `il.toml`: `[expect] cursor = { col = 0, line = 1 }`
  Trace: "Line 1" on row 0, CR+LF -> row 1. "Line 2" on row 1, CR+LF -> row 2. "Line 3" on row 2, CR+LF -> row 3. "Line 4" on row 3. CUP(2,1) -> (1, 0). IL 2 inserts 2 blank lines at row 1, pushing Lines 2-4 down. Cursor stays at (0, 1). Grid: row 0="Line 1", rows 1-2=blank, row 3="Line 2", row 4="Line 3", row 5="Line 4". Confirmed by handler test `il_inserts_2_lines`.

  **`dl.teseq`** — DL (delete lines, shift up):
  ```
  |Line 1|
  . CR/^M LF/^J
  |Line 2|
  . CR/^M LF/^J
  |Line 3|
  . CR/^M LF/^J
  |Line 4|
  : Esc [ 2 ; 1 H
  : Esc [ 2 M
  ```
  `dl.toml`: `[expect] cursor = { col = 0, line = 1 }`
  Trace: Same layout as IL. CUP(2,1) -> (1, 0). DL 2 deletes 2 lines at row 1 (Lines 2 and 3), pulling Line 4 up. Cursor stays at (0, 1). Grid: row 0="Line 1", row 1="Line 4", rows 2+=blank. Confirmed by handler test `dl_deletes_3_lines` (different count but same mechanism).

- [x] Register family module in `main.rs`: add `mod csi_insert_delete;`
- [x] Accept new snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq -- csi_insert_delete`
- [x] Inspect each new `.snap` file to verify grid content matches traces
- [x] Verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_insert_delete`

---

## 02.5 ESC Sequence Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/esc/*.teseq`, `oriterm_core/tests/teseq/esc.rs`

- [x] Create scenario directory: `mkdir -p oriterm_core/tests/teseq/scenarios/esc`
- [x] Create family module `oriterm_core/tests/teseq/esc.rs` with `run_scenario` helper (path = `scenarios/esc/{name}.teseq`, prefix = `esc_{name}`)
- [x] Create scenario files:

  **`decsc_decrc.teseq`** — Save and restore cursor position (ESC 7 / ESC 8):
  ```
  : Esc [ 10 ; 20 H
  |X|
  : Esc 7
  : Esc [ 1 ; 1 H
  |Y|
  : Esc 8
  |Z|
  ```
  `decsc_decrc.toml`: `[expect] cursor = { col = 21, line = 9 }`
  Trace: CUP(10,20) -> (9, 19). "X" at (9,19) -> cursor (9,20). ESC 7 (DECSC) saves (9,20). CUP(1,1) -> (0,0). "Y" at (0,0) -> cursor (0,1). ESC 8 (DECRC) restores to (9,20). "Z" at (9,20) -> cursor (9,21). Grid: "Y" at (0,0), "X" at (9,19), "Z" at (9,20). Confirmed by handler test `decsc_decrc_saves_and_restores_cursor_position`.
  **Scope note:** This scenario tests cursor *position* save/restore only. DECSC also saves SGR attributes, active charset (G0/G1), origin mode flag, and wrap-pending state. Those additional saved-state dimensions require mode-interaction scenarios (Section 04) and workflow scenarios (Section 06) to validate, as they depend on mode setup and the current assertion pipeline (grid text + cursor position) cannot directly observe them.

  **`ris.teseq`** — RIS (full reset, ESC c):
  ```
  |Some content|
  : Esc [ 5 ; 10 H
  : Esc c
  ```
  `ris.toml`: `[expect] cursor = { col = 0, line = 0 }`
  Trace: "Some content" on line 0. CUP(5,10) -> (4, 9). ESC c (RIS) clears grid and resets cursor to home (0, 0). Grid all blank. Confirmed by handler tests `ris_clears_grid_content` and `ris_clears_all_visible_lines`.

  **`scs_g0.teseq`** — Designate G0 charset to DEC Special Graphics:
  ```
  : Esc ( 0
  |qqqqq|
  : Esc ( B
  |-----|
  ```
  `scs_g0.toml`: `[expect] cursor = { col = 10, line = 0 }`
  Trace: ESC ( 0 designates G0 to DEC Special Graphics. "qqqqq" (5 chars) renders as line-drawing characters (q = horizontal line in DEC Special Graphics) -> cursor at (5, 0). ESC ( B restores G0 to ASCII. "-----" (5 chars) renders as literal dashes -> cursor at (10, 0). All on line 0 (no LF). Grid snapshot shows line-drawing chars followed by dashes.
  Note: `ESC (` sets G0 (confirmed in handler test pattern), `ESC )` sets G1. This scenario tests G0 designation, complementing the SO/SI scenario in 02.1 which tests G1.

  **`ind.teseq`** — IND (ESC D) — index (scroll up at bottom):
  ```
  |TOP|
  : Esc [ 24 ; 1 H
  |bottom|
  : Esc D
  . CR/^M
  |scrolled|
  ```
  `ind.toml`: `[expect] cursor = { col = 8, line = 23 }`
  Trace: "TOP" on line 0 (cols 0-2). CUP(24,1) -> (23, 0). "bottom" (6 chars) on line 23 -> cursor at (6, 23). ESC D (IND) at bottom of screen: scrolls up one line, cursor column preserved -> (6, 23). "TOP" scrolls to scrollback (if enabled) or is lost. "bottom" moves to line 22. Line 23 is now blank. CR resets cursor to col 0. "scrolled" (8 chars) written at (0, 23) -> cursor at (8, 23). Grid snapshot: line 22 = "bottom", line 23 = "scrolled", other lines shifted up. Confirmed by handler test `esc_d_index_at_bottom_scrolls`.

  **`ri.teseq`** — RI (ESC M) — reverse index (scroll down at top):
  ```
  |LINE0|
  . CR/^M LF/^J
  |LINE1|
  . CR/^M LF/^J
  |LINE2|
  : Esc [ 1 ; 1 H
  : Esc M
  |inserted|
  ```
  `ri.toml`: `[expect] cursor = { col = 8, line = 0 }`
  Trace: "LINE0" (5 chars) on row 0. CR+LF -> row 1. "LINE1" on row 1. CR+LF -> row 2. "LINE2" on row 2. CUP(1,1) -> (0, 0). ESC M (RI) at top of screen: scrolls content down one line. Row 0 becomes blank. "LINE0" moves to row 1, "LINE1" to row 2, "LINE2" to row 3. Cursor stays at (0, 0). "inserted" (8 chars) written at (0, 0) -> cursor at (8, 0). Grid: row 0="inserted", row 1="LINE0", row 2="LINE1", row 3="LINE2". Confirmed by handler tests `ri_at_top_of_scroll_region_scrolls_down` and `esc_m_reverse_index_at_top_scrolls_down`.

- [x] Register family module in `main.rs`: add `mod esc;`
- [x] Accept new snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq -- esc`
- [x] Inspect each new `.snap` file to verify grid content matches traces
- [x] Verify: `timeout 150 cargo test -p oriterm_core --test teseq -- esc`

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [ ] `[TPR-02-001][major]` Section 02 delegates IL/DL scroll-region coverage to Section 04, but Section 04 does not currently own any IL/DL-within-DECSTBM scenarios. The mission criterion "ICH, DCH, IL, DL with scroll region interactions" has no owning section. **Action required before Section 04 implementation:** Add IL/DL-within-DECSTBM scenarios to `section-04-mode-interactions.md` subsection 04.1 (Origin Mode + Scroll Region). Specifically: (1) `il_in_scroll_region.teseq` — IL within DECSTBM, verify lines outside region do not shift; (2) `dl_in_scroll_region.teseq` — DL within DECSTBM, verify lines outside region do not shift. This is a Section 04 plan edit, not a Section 02 implementation task.
- [x] `[TPR-02-002][low]` ~~The ED 3 note named the snapshot as `c0_ed_scrollback`, but it lives in the CSI erase family.~~ **Fixed:** Renamed to `csi_erase_ed_scrollback` throughout the plan and added a custom test function in 02.3 that calls both `assert_spec` and `assert_scrollback_empty`.

---

## 02.N Completion Checklist

- [x] Scenario directories created: `scenarios/csi/cursor/`, `scenarios/csi/erase/`, `scenarios/csi/insert_delete/`, `scenarios/esc/`
- [x] C0 scenario files created: cr, lf, bs, tab, bel, ff, vt, so_si (8 scenarios)
- [x] CSI cursor scenarios created: cup_basic, cup_origin, cup_clamp, cup_clamp_97x33, cup_clamp_120x40, cuu_cud, cuf_cub, vpa, hpa, cha (10 scenarios)
- [x] CSI erase scenarios created: ed_below, ed_above, ed_all, ed_scrollback, el_right, el_left, el_all (7 scenarios)
- [x] CSI insert/delete scenarios created: ich, dch, il, dl (4 scenarios, scroll-region variants in Section 04)
- [x] ESC scenarios created: decsc_decrc, ris, scs_g0, ind, ri (5 scenarios)
- [x] Family modules registered in main.rs: c0, csi_cursor, csi_erase, csi_insert_delete, esc
- [x] `smoke_bel` test removed from `main.rs` (replaced by `c0::bel`)
- [x] Old `smoke_bel_grid` and `smoke_bel_events` snapshot files deleted from `harness/snapshots/`
- [x] Multi-size cursor clamping scenarios pass at 97x33 and 120x40
- [x] All multi-line scenarios use CR+LF (`. CR/^M LF/^J`), NOT `|text|.` (LF-only)
- [x] `ed_scrollback` test uses custom function calling both `assert_spec()` and `assert_scrollback_empty()`
- [x] Every scenario has a TOML sidecar with an explicit `cursor = { col, line }` assertion (no scenario relies solely on grid snapshots for cursor verification)
- [x] All insta snapshots accepted (`INSTA_UPDATE=1` or `cargo insta review`) and manually inspected
- [x] All snapshots in `harness/snapshots/` with globally unique names (family-prefixed)
- [x] Total: 34 scenario files with golden snapshots (8 + 10 + 7 + 4 + 5)
- [x] `./build-all.sh` green, `./clippy-all.sh` green
- [x] `timeout 150 ./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [x] This section's frontmatter `status` → `complete`
  - [x] `00-overview.md` Quick Reference table updated
  - [x] `index.md` section status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq` passes with 30+ scenario tests covering C0 controls, CSI cursor movement, CSI erase, CSI insert/delete, and ESC sequences. All scenarios have insta golden snapshots reviewed for correctness. Multi-size variants pass for cursor clamping. All multi-line scenarios correctly use CR+LF for column-zero line starts. `main.rs` contains only `#![allow(dead_code)]`, `mod harness;`, and the five family `mod` declarations (no `smoke_bel` test function). Zero regressions.
