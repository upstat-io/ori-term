---
section: "04"
title: "Mode Interaction Scenarios"
status: not-started
reviewed: false
goal: "Create multi-sequence scenarios testing mode combinations that individual handler tests don't cover — DECOM+scroll, DECCOLM transitions, alt screen roundtrips, and IRM"
success_criteria:
  - "DECOM+DECSTBM scenarios validate cursor positioning within scroll regions"
  - "DECCOLM scenarios validate 80↔132 column transitions with content clearing"
  - "Alt screen (1049) scenarios validate enter/exit roundtrip preserving primary screen"
  - "IRM scenarios validate insert mode character insertion"
  - "Mode combination scenarios run at 80x24, 97x33, and 120x40"
  - "Satisfies mission criteria: DECOM+DECSTBM, DECCOLM+DECAWM, alt screen (1049), IRM coverage"
inspired_by:
  - "ori_term vttest menu1 (DECCOLM) — 132-column mode transitions"
  - "ori_term vttest menu2 (origin mode, scroll regions) — DECOM+DECSTBM interactions"
  - "ori_term handler/tests.rs — origin mode, scroll region, insert mode individual tests"
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Origin Mode + Scroll Region Scenarios"
    status: not-started
  - id: "04.2"
    title: "DECCOLM Column Mode Scenarios"
    status: not-started
  - id: "04.3"
    title: "Alt Screen Scenarios"
    status: not-started
  - id: "04.4"
    title: "Insert Mode & Wrap Scenarios"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Mode Interaction Scenarios

**Status:** Not Started
**Goal:** Test multi-mode combinations where individual handler tests fall short. The existing handler tests validate each mode in isolation. These scenarios test the *interaction* between modes — the edge cases that only manifest when multiple modes are active simultaneously.

**Success Criteria:**

- [ ] DECOM+DECSTBM: cursor stays within scroll region, scrolling respects origin
- [ ] DECCOLM: 80→132 and 132→80 transitions clear screen, preserve modes
- [ ] Alt screen: 1049 enter/exit preserves primary screen content
- [ ] IRM: insert mode shifts existing characters correctly
- [ ] Multi-size variants for mode interactions (97x33, 120x40)
- [ ] 12+ mode interaction scenarios pass

**Context:** The vttest conformance plan fixed several mode interaction bugs: DECOM cursor positioning was garbled (fixed in Section 02), DECCOLM was a no-op (fixed in Section 03), border fills failed at non-80x24 sizes. These teseq scenarios serve as permanent regression guards for those fixes and explore additional interaction patterns.

**Reference implementations:**
- **ori_term** `handler/tests.rs` — origin mode tests, scroll region tests, insert mode tests
- **ori_term** `vttest/menu1.rs` — DECCOLM 132-column transitions
- **ori_term** `vttest/menu2.rs` — origin mode + scroll region interactions

**Depends on:** Section 01 (TeseqHarness), Section 02 (basic scenario pattern).

---

## 04.1 Origin Mode + Scroll Region Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/origin_*.teseq`, `oriterm_core/tests/teseq/mode_interactions.rs`

- [ ] **`origin_scroll_basic.teseq`** — Set scroll region, enable DECOM, verify cursor is relative:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 1 ; 1 H
  |Origin top|.
  : Esc [ 16 ; 1 H
  |Origin bottom|
  ```
  Grid snapshot shows "Origin top" at absolute row 4 (scroll region top - 1), "Origin bottom" at absolute row 19.

- [ ] **`origin_scroll_overflow.teseq`** — Fill scroll region past capacity, verify scrolling:
  ```
  : Esc [ 10 ; 15 r
  : Esc [ ? 6 h
  : Esc [ 1 ; 1 H
  |Line 01|.
  |Line 02|.
  |Line 03|.
  |Line 04|.
  |Line 05|.
  |Line 06|.
  |Line 07|.
  |Line 08|.
  ```
  With 6-line scroll region (rows 10-15), writing 8 lines causes scrolling within the region.

- [ ] **`origin_cursor_save_restore.teseq`** — DECSC/DECRC with origin mode:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 3 ; 10 H
  : Esc 7
  : Esc [ 1 ; 1 H
  |moved|
  : Esc 8
  |restored|
  ```

- [ ] Multi-size variants with separate `.toml` sidecars for 97x33 and 120x40.

---

## 04.2 DECCOLM Column Mode Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/deccolm_*.teseq`

Mode 40 (ENABLE_MODE_3) must be enabled for DECCOLM to work. Scenarios verify grid resize + screen clear behavior.

- [ ] **`deccolm_80_to_132.teseq`** — Switch to 132 columns:
  ```
  |Content at 80 cols|.
  : Esc [ ? 3 h
  |Now at 132 columns|
  ```
  `deccolm_80_to_132.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  [terminal]
  cols = 80
  rows = 24
  ```
  Grid snapshot shows screen cleared on mode switch, "Now at 132 columns" visible, grid is 132 columns wide.

- [ ] **`deccolm_132_to_80.teseq`** — Switch back to 80:
  ```
  : Esc [ ? 3 h
  |Wide content at 132|.
  : Esc [ ? 3 l
  |Back to 80|
  ```
  Grid snapshot shows screen cleared again, grid back to 80 columns.

- [ ] **`deccolm_wrap_interaction.teseq`** — DECCOLM + DECAWM interaction at 132 columns:
  ```
  : Esc [ ? 3 h
  : Esc [ ? 7 h
  |AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAXX|
  ```
  `deccolm_wrap_interaction.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  ```
  (132 A's + "xx" at 132 columns — wraps to next line with DECAWM active in wide mode)

- [ ] **`deccolm_preserves_scroll_region.teseq`** — DECCOLM resets scroll region:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 3 h
  : Esc [ 999 ; 1 H
  |At bottom|
  ```
  After DECCOLM switch, scroll region is reset to full screen.

- [ ] **TPR checkpoint** — `/tpr-review` covering 04.1–04.2 implementation work

---

## 04.3 Alt Screen Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/altscreen_*.teseq`

- [ ] **`altscreen_roundtrip.teseq`** — Enter and exit alt screen preserving primary:
  ```
  |Primary screen content|.
  |Line two|.
  : Esc [ ? 1049 h
  |Alt screen content|.
  : Esc [ ? 1049 l
  ```
  Grid snapshot after exit should show primary screen content restored.

- [ ] **`altscreen_cursor.teseq`** — Cursor position saved/restored:
  ```
  : Esc [ 10 ; 20 H
  : Esc [ ? 1049 h
  : Esc [ 1 ; 1 H
  |alt|
  : Esc [ ? 1049 l
  ```
  `altscreen_cursor.toml`: `[expect] cursor = { col = 19, line = 9 }`
  (Cursor restored to pre-alt-screen position)

- [ ] **`altscreen_content_isolation.teseq`** — Alt screen doesn't bleed to primary:
  ```
  |Primary|.
  : Esc [ ? 1049 h
  |AAAAAAAAAA|.
  |BBBBBBBBBB|.
  : Esc [ ? 1049 l
  ```
  Grid snapshot confirms primary content preserved, alt content gone.

---

## 04.4 Insert Mode & Wrap Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/csi/modes/insert_wrap_*.teseq`

- [ ] **`irm_insert.teseq`** — Insert mode shifts characters right:
  ```
  |ABCDEFGHIJ|
  : Esc [ 1 ; 4 H
  : Esc [ 4 h
  |XY|
  : Esc [ 4 l
  ```
  (Enable IRM, type "XY" at col 3, existing chars shift right)

- [ ] **`wrap_at_margin.teseq`** — Auto-wrap at right margin:
  ```
  : Esc [ 1 ; 1 H
  |AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAXX|
  ```
  (80 A's + "xx" — wraps to next line)
  Grid snapshot shows 80 chars on row 0, "xx" on row 1.

- [ ] **`wrap_disabled.teseq`** — DECAWM off prevents wrap:
  ```
  : Esc [ ? 7 l
  : Esc [ 1 ; 1 H
  |AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAXX|
  ```
  Grid snapshot shows chars overwrite at column 79, no wrap.

---

## 04.R Third Party Review Findings

- None.

---

## 04.N Completion Checklist

- [ ] Origin mode + scroll region scenarios: basic, overflow, cursor save/restore (3+ scenarios)
- [ ] DECCOLM scenarios: 80→132, 132→80, preserves/resets scroll region (3+ scenarios)
- [ ] Alt screen scenarios: roundtrip, cursor, content isolation (3+ scenarios)
- [ ] Insert mode and wrap scenarios: IRM insert, wrap at margin, wrap disabled (3+ scenarios)
- [ ] Multi-size variants for mode interactions (97x33, 120x40)
- [ ] 12+ total mode interaction scenarios pass
- [ ] All insta snapshots reviewed for correctness
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `cargo test -p oriterm_core --test teseq -- mode_interactions` passes with 12+ scenarios testing mode combinations. DECOM+DECSTBM, DECCOLM transitions, alt screen roundtrips, and IRM all validated at multiple sizes. These scenarios serve as permanent regression guards for the bugs fixed during vttest conformance. Zero regressions.
