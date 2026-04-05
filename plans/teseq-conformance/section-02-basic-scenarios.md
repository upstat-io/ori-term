---
section: "02"
title: "Basic Scenario Suite"
status: not-started
reviewed: false
goal: "Create foundational scenario files for C0 controls and basic CSI sequences that validate the harness end-to-end and establish the scenario authoring pattern"
success_criteria:
  - "C0 control scenarios pass: CR, LF, BS, TAB, BEL, FF, VT, SO, SI"
  - "CSI cursor movement scenarios pass: CUP, CUU, CUD, CUF, CUB, VPA, HPA, CHA with boundary checks"
  - "CSI erase scenarios pass: ED modes 0-3 (including ED 3 scrollback erase), EL modes 0-2"
  - "CSI insert/delete scenarios pass: ICH, DCH, IL, DL with content verification"
  - "ESC sequence scenarios pass: DECSC/DECRC (save/restore cursor), RIS, SCS (G0/G1 charset designation)"
  - "All scenarios run at 80x24; cursor movement also at 97x33 and 120x40"
  - "30+ scenarios pass with insta golden snapshots"
  - "Satisfies mission criteria: C0, basic CSI, and ESC scenario coverage"
inspired_by:
  - "WezTerm test organization (term/src/test/c0.rs, c1.rs, csi.rs) — by sequence type"
  - "ori_term handler/tests.rs — individual sequence tests as reference for expected behavior"
depends_on: ["01"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "C0 Control Character Scenarios"
    status: not-started
  - id: "02.2"
    title: "CSI Cursor Movement Scenarios"
    status: not-started
  - id: "02.3"
    title: "CSI Erase Scenarios"
    status: not-started
  - id: "02.4"
    title: "CSI Insert/Delete Scenarios"
    status: not-started
  - id: "02.5"
    title: "ESC Sequence Scenarios"
    status: not-started
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

- [ ] 9+ C0 control scenarios pass with grid + event snapshots
- [ ] 8+ CSI cursor movement scenarios pass (including boundary edge cases)
- [ ] 7+ CSI erase scenarios pass (ED modes 0-3 and EL variants)
- [ ] 4+ CSI insert/delete scenarios pass
- [ ] 4+ ESC sequence scenarios pass (save/restore, charset)
- [ ] Total: 30+ scenarios, all with insta golden snapshots
- [ ] Satisfies mission criteria for C0, cursor, erase, insert/delete, and ESC coverage

**Context:** The existing handler unit tests (`oriterm_core/src/term/handler/tests.rs`, 5,860 lines) already validate each of these sequences individually via byte-feed + grid-cell assertions. This section's scenarios deliberately overlap with handler tests for three specific purposes: (1) validating the teseq harness pipeline end-to-end before building complex scenarios, (2) establishing the scenario authoring pattern that Sections 03-06 follow, and (3) producing full-grid insta snapshots (vs. handler tests' per-cell assertions) that catch grid-wide side effects. If a scenario would test nothing beyond what handler/tests.rs already covers AND its only purpose is coverage, it does not belong here — keep the scenario count lean. Multi-sequence interaction scenarios (the framework's primary value-add) come in Sections 04-06.

**Reference implementations:**
- **WezTerm** `term/src/test/c0.rs`: C0 tests organized by control character (BS, LF, CR, TAB)
- **WezTerm** `term/src/test/c1.rs`: C1 tests (IND, NEL, HTS, RI)
- **ori_term** `handler/tests.rs:70-120`: `hello_places_cells_and_advances_cursor`, `carriage_return_overwrites`, `tab_advances_to_column_8` — reference for expected behavior

**Depends on:** Section 01 (TeseqHarness infrastructure).

---

## 02.1 C0 Control Character Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/c0/*.teseq`, `oriterm_core/tests/teseq/c0.rs`

Create scenario files for each C0 control character that ori_term handles. Each `.teseq` file describes the input sequence, and the insta snapshot captures the resulting grid state.

- [ ] Create family module `oriterm_core/tests/teseq/c0.rs` with auto-discovery:
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
  // ... one #[test] per scenario file
  ```

- [ ] Create scenario files (each `.teseq` with optional `.toml` sidecar):

  **`cr.teseq`** — Carriage return moves cursor to column 0:
  ```
  |hello|
  . CR/^M
  |world|
  ```
  `cr.toml`: `[expect] cursor = { col = 5, line = 0 }`

  **`lf.teseq`** — Line feed moves cursor down (column preserved):
  ```
  |hello|
  . LF/^J
  |world|
  ```
  `lf.toml`: `[expect] cursor = { col = 10, line = 1 }`
  Note: LF preserves column (VTE behavior). "hello" ends at col 5, LF moves to (5, 1), then "world" writes at cols 5-9, cursor lands at col 10. Grid shows "hello" on line 0 and "     world" on line 1 (5 spaces + world).

  **`bs.teseq`** — Backspace moves cursor left:
  ```
  |hello|
  . BS/^H
  |X|
  ```
  `bs.toml`: `[expect] cursor = { col = 5, line = 0 }`

  **`tab.teseq`** — Tab advances to next tab stop (column 8):
  ```
  . HT/^I
  |X|
  ```
  `tab.toml`: `[expect] cursor = { col = 9, line = 0 }`

  **`bel.teseq`** — Bell triggers Bell event (already created as smoke test):
  ```
  |Hello|
  . BEL/^G
  | World|
  ```
  `bel.toml`: `[expect] cursor = { col = 11, line = 0 } events = ["Bell"]`
  Cursor: "Hello" (5 chars) + BEL (no movement) + " World" (6 chars) = col 11.

  **`ff.teseq`** — Form feed same as LF:
  ```
  |hello|
  . FF/^L
  |world|
  ```

  **`vt.teseq`** — Vertical tab same as LF:
  ```
  |hello|
  . VT/^K
  |world|
  ```

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
  ```
  Pre-feed designates G1 as DEC Special Graphics (`ESC ) 0`). After SO, `qqqqq` maps to `─────` (horizontal line, U+2500) in DEC Special Graphics. After SI, `--------` renders as literal ASCII dashes. Grid snapshot should show the Unicode line-drawing characters followed by dashes.

- [ ] Register family module in `main.rs`: `mod c0;`
- [ ] Verify all C0 scenarios pass: `timeout 150 cargo test -p oriterm_core --test teseq -- c0`

---

## 02.2 CSI Cursor Movement Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/cursor/*.teseq`, `oriterm_core/tests/teseq/csi_cursor.rs`

Cursor movement sequences with boundary conditions.

- [ ] Create family module `csi_cursor.rs` and scenario files:

  **`cup_basic.teseq`** — CUP moves cursor to absolute position:
  ```
  : Esc [ 5 ; 10 H
  |X|
  ```
  `cup_basic.toml`: `[expect] cursor = { col = 10, line = 4 }`
  (CUP is 1-based: row 5 = line 4, col 10 = col 9 + 1 char)

  **`cup_origin.teseq`** — CUP to (1,1) is home:
  ```
  |padding text|
  : Esc [ 1 ; 1 H
  |X|
  ```
  `cup_origin.toml`: `[expect] cursor = { col = 1, line = 0 }`

  **`cup_clamp.teseq`** — CUP beyond grid clamps to last row/col:
  ```
  : Esc [ 999 ; 999 H
  ```
  `cup_clamp.toml`: `[expect] cursor = { col = 79, line = 23 }`

  **`cuu_cud.teseq`** — CUU/CUD relative movement:
  ```
  : Esc [ 10 ; 10 H
  : Esc [ 3 A
  : Esc [ 5 B
  ```
  `cuu_cud.toml`: `[expect] cursor = { col = 9, line = 11 }`

  **`cuf_cub.teseq`** — CUF/CUB horizontal movement:
  ```
  : Esc [ 1 ; 1 H
  : Esc [ 20 C
  : Esc [ 5 D
  ```
  `cuf_cub.toml`: `[expect] cursor = { col = 15, line = 0 }`

  **`vpa.teseq`** — VPA (vertical position absolute):
  ```
  : Esc [ 10 ; 5 H
  : Esc [ 15 d
  ```
  `vpa.toml`: `[expect] cursor = { col = 4, line = 14 }`

  **`hpa.teseq`** — HPA (horizontal position absolute):
  ```
  : Esc [ 10 ; 5 H
  : Esc [ 30 G
  ```
  `hpa.toml`: `[expect] cursor = { col = 29, line = 9 }`

  **`cha.teseq`** — CHA (cursor horizontal absolute, same as HPA in practice):
  ```
  : Esc [ 5 ; 20 H
  : Esc [ 1 G
  ```
  `cha.toml`: `[expect] cursor = { col = 0, line = 4 }`

- [ ] Create multi-size test variants for cursor clamping (97x33, 120x40).
  Multi-size variants duplicate the `.teseq` content with a different `.toml` sidecar. For small scenario files (< 10 lines), this duplication is acceptable and simpler than a shared-content mechanism:
  ```
  cup_clamp_97x33.teseq: (same content as cup_clamp.teseq)
  cup_clamp_97x33.toml:  [terminal] cols = 97 rows = 33
                          [expect] cursor = { col = 96, line = 32 }
  cup_clamp_120x40.teseq: (same content)
  cup_clamp_120x40.toml:  [terminal] cols = 120 rows = 40
                           [expect] cursor = { col = 119, line = 39 }
  ```

- [ ] Register and verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_cursor`

- [ ] **TPR checkpoint** — `/tpr-review` covering 02.1–02.2 implementation work

---

## 02.3 CSI Erase Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/erase/*.teseq`, `oriterm_core/tests/teseq/csi_erase.rs`

Erase operations (ED and EL) with pre-populated grid content.

- [ ] Create scenario files:

  **`ed_below.teseq`** — ED 0 (erase below cursor):
  ```
  |AAAAAAAAAA|.
  |BBBBBBBBBB|.
  |CCCCCCCCCC|.
  : Esc [ 2 ; 5 H
  : Esc [ 0 J
  ```
  (Erases from cursor to end of display)

  **`ed_above.teseq`** — ED 1 (erase above cursor):
  ```
  |AAAAAAAAAA|.
  |BBBBBBBBBB|.
  |CCCCCCCCCC|.
  : Esc [ 2 ; 5 H
  : Esc [ 1 J
  ```

  **`ed_all.teseq`** — ED 2 (erase entire display):
  ```
  |AAAAAAAAAA|.
  |BBBBBBBBBB|.
  : Esc [ 2 J
  ```

  **`ed_scrollback.teseq`** — ED 3 (erase scrollback buffer):
  The scenario must first push content into scrollback (by writing more lines than the visible grid), then issue ED 3 to clear it. A 24-line grid with 30 lines of content puts 6 lines into scrollback.
  ```
  |Line 01|.
  |Line 02|.
  |Line 03|.
  |Line 04|.
  |Line 05|.
  |Line 06|.
  |Line 07|.
  |Line 08|.
  |Line 09|.
  |Line 10|.
  |Line 11|.
  |Line 12|.
  |Line 13|.
  |Line 14|.
  |Line 15|.
  |Line 16|.
  |Line 17|.
  |Line 18|.
  |Line 19|.
  |Line 20|.
  |Line 21|.
  |Line 22|.
  |Line 23|.
  |Line 24|.
  |Line 25|.
  |Line 26|.
  |Line 27|.
  |Line 28|.
  |Line 29|.
  |Line 30|.
  : Esc [ 3 J
  ```
  `ed_scrollback.toml`:
  ```toml
  [terminal]
  scrollback = 100
  ```
  Before ED 3: 6 lines in scrollback (30 written - 24 visible). After ED 3: scrollback cleared (scrollback_len = 0), visible content preserved (Lines 07-30 visible, Lines 01-06 gone). Grid snapshot validates visible content. Assert `scrollback_len == 0` via `assert_scrollback_empty()` (defined in Section 01.5). `ScenarioOutcome::scrollback_len` is already populated from `content.scrollback_len` (Section 01.4).

  **`el_right.teseq`** — EL 0 (erase to right of cursor):
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 0 K
  ```

  **`el_left.teseq`** — EL 1 (erase to left of cursor):
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 1 K
  ```

  **`el_all.teseq`** — EL 2 (erase entire line):
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 2 K
  ```

- [ ] Each erase scenario's insta snapshot captures the grid after erasure, making the erased region visible as blank cells.
- [ ] Register and verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_erase`

---

## 02.4 CSI Insert/Delete Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/insert_delete/*.teseq`, `oriterm_core/tests/teseq/csi_insert_delete.rs`

- [ ] Create scenario files:

  **`ich.teseq`** — ICH (insert characters, shift right):
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 3 @
  ```
  (Insert 3 blanks at col 3, shifting D-J right)

  **`dch.teseq`** — DCH (delete characters, shift left):
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 3 P
  ```
  (Delete 3 chars at col 3, shifting G-J left, trailing blanks)

  **`il.teseq`** — IL (insert lines, shift down):
  ```
  |Line 1|.
  |Line 2|.
  |Line 3|.
  |Line 4|.
  : Esc [ 2 ; 1 H
  : Esc [ 2 L
  ```
  (Insert 2 blank lines at row 2, push lines 2-4 down)

  **`dl.teseq`** — DL (delete lines, shift up):
  ```
  |Line 1|.
  |Line 2|.
  |Line 3|.
  |Line 4|.
  : Esc [ 2 ; 1 H
  : Esc [ 2 M
  ```
  (Delete 2 lines at row 2, pull lines 4+ up, trailing blank lines)

- [ ] Register and verify: `timeout 150 cargo test -p oriterm_core --test teseq -- csi_insert_delete`

---

## 02.5 ESC Sequence Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/esc/*.teseq`, `oriterm_core/tests/teseq/esc.rs`

- [ ] Create scenario files:

  **`decsc_decrc.teseq`** — Save and restore cursor (ESC 7 / ESC 8):
  ```
  : Esc [ 10 ; 20 H
  |X|
  : Esc 7
  : Esc [ 1 ; 1 H
  |Y|
  : Esc 8
  |Z|
  ```
  CUP 10;20 = line 9, col 19 (0-based). X writes at (9,19), cursor advances to (9,20). DECSC saves (9,20). CUP 1;1 moves to (0,0). Y writes at (0,0). DECRC restores to (9,20). Z writes at (9,20), cursor at (9,21).

  **`ris.teseq`** — RIS (full reset, ESC c):
  ```
  |Some content|.
  : Esc [ 5 ; 10 H
  : Esc c
  ```
  `ris.toml`: `[expect] cursor = { col = 0, line = 0 }`
  (Grid cleared, cursor at home)

  **`scs_g0.teseq`** — Designate G0 charset to DEC Special Graphics:
  ```
  : Esc ( 0
  |qqqqq|
  : Esc ( B
  |-----|
  ```
  (First `qqqqq` renders as horizontal line in DEC Special Graphics, then back to ASCII)

  **`ind_ri.teseq`** — IND (ESC D) and RI (ESC M) — index and reverse index:
  ```
  : Esc [ 24 ; 1 H
  |bottom|
  : Esc D
  |scrolled|
  : Esc [ 1 ; 1 H
  : Esc M
  |top inserted|
  ```

- [ ] Register and verify: `timeout 150 cargo test -p oriterm_core --test teseq -- esc`

---

## 02.R Third Party Review Findings

- None.

---

## 02.N Completion Checklist

- [ ] C0 scenario files created: cr, lf, bs, tab, bel, ff, vt, so_si (8+ scenarios)
- [ ] CSI cursor scenarios created: cup_basic, cup_origin, cup_clamp, cuu_cud, cuf_cub, vpa, hpa, cha (8+ scenarios)
- [ ] CSI erase scenarios created: ed_below, ed_above, ed_all, ed_scrollback, el_right, el_left, el_all (7 scenarios)
- [ ] CSI insert/delete scenarios created: ich, dch, il, dl (4 scenarios)
- [ ] ESC scenarios created: decsc_decrc, ris, scs_g0, ind_ri (4 scenarios)
- [ ] Family modules registered in main.rs: c0, csi_cursor, csi_erase, csi_insert_delete, esc
- [ ] Multi-size cursor clamping scenarios pass at 97x33 and 120x40
- [ ] All insta snapshots generated and reviewed for correctness
- [ ] Total: 30+ scenario files with golden snapshots
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `index.md` section status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq` passes with 30+ scenario tests covering C0 controls, CSI cursor movement, CSI erase, CSI insert/delete, and ESC sequences. All scenarios have insta golden snapshots reviewed for correctness. Multi-size variants pass for cursor clamping. Zero regressions.
