---
bug: "BUG-08-17"
title: "Cell lacks a CHARDRAWN-equivalent flag — DECRQCRA (CSI * y) silently skips application-written spaces with default SGR"
severity: "medium"
status: complete
goal: "Cell carries a persistent DRAWN bit set on every application write and cleared on every reset, so compute_rect_checksum() distinguishes application-written blanks from pristine cells and matches xterm byte-for-byte on 'A B'-style inputs."
success_criteria:
  - "A 2×3 DECRQCRA scenario that writes 'A B' (three drawn cells: 'A', ' ', 'B' with default SGR) returns checksum 0xFF5D, byte-identical to xterm."
  - "CellFlags::DRAWN is set on every cell-write path (put_char_ascii, put_char_slow main cell, put_char_slow wide-char spacer, put_char_slow LEADING_WIDE_CHAR_SPACER) and cleared on every reset path (Cell::reset via template copy, Row::reset, clear_range, truncate, BCE erase)."
  - "compute_rect_checksum() at oriterm_core/src/term/handler/rect_ops/mod.rs consults `cell.flags.contains(CellFlags::DRAWN)` for both the skip path and the DRAWX_MASK trim-gate disjunct."
  - "`Cell::size_of() <= 24` size assertion still holds (DRAWN reuses an existing unused CellFlags bit, does NOT grow Cell)."
  - "Existing 1868 lib tests + 582 spec_chain tests + 176 teseq tests still pass; no snapshot updates required for rendering paths."
  - "./build-all.sh, ./test-all.sh, ./clippy-all.sh all green."
subsystem: "oriterm_core/src/cell/mod.rs + oriterm_core/src/grid/editing/mod.rs + oriterm_core/src/term/handler/rect_ops/mod.rs"
found: "2026-04-19"
source: "/tpr-review round 2 on spec-conformance §09A.5 (codex round 2 F1)"
third_party_review:
  status: clean
  updated: 2026-04-20
  rounds: 3
  notes: "3-round code TPR; round 0 codex F1 (is_empty leak) + F2 (doc drift) fixed in 6dd30e5a; round 1 codex F1 (row-level regression pins) fixed in 6c966f2b; round 2 returned informational-only verifications confirming convergence. Gemini findings in rounds 0+1 verified against code and dropped as hallucinated or non-xterm-parity. Round 2 gemini status=clean."
---

# Fix: BUG-08-17 — Cell lacks a CHARDRAWN-equivalent flag

**Status:** Complete
**Severity:** medium
**Goal:** `Cell` carries a persistent `DRAWN` bit that distinguishes "application-written" cells from pristine cells, restoring xterm parity for DECRQCRA on explicit-space inputs.

**Success Criteria:**
- [ ] `CellFlags::DRAWN` constant added and consumes an unused bit in the `u32` bitfield (no Cell size growth).
- [ ] DRAWN set in: `put_char_ascii`, `put_char_slow` (main cell, wide spacer, leading-wide spacer).
- [ ] DRAWN never set on `cursor.template.flags` (SGR template must stay DRAWN-clear so reset paths propagate clear state).
- [ ] `compute_rect_checksum()` consults `CellFlags::DRAWN` via a named `cell_drawn(cell)` helper — both skip path and trim gate.
- [ ] New TDD matrix tests in `oriterm_core/src/term/handler/rect_ops/tests.rs` including the 2×3 "A B" xterm-parity repro.
- [ ] `Cell::size_of() <= 24` compile-time assertion still passes.
- [ ] `./build-all.sh && ./test-all.sh && ./clippy-all.sh` all green.

**Context:** The §09A.5 DECRQCRA implementation in `oriterm_core/src/term/handler/rect_ops/mod.rs::compute_rect_checksum` uses `!cell.is_empty()` as its "was this cell drawn by the application" proxy. Round 2 of TPR on §09A.5 (codex F1) proved the proxy wrong for application-written plain `' '` with default SGR: those cells have `is_empty() == true`, so the checksum silently skips them. xterm's `xtermCheckRect` at `~/projects/reference_repos/console_repos/xterm/screen.c:3178-3180` uses a persistent per-cell `CHARDRAWN` bit that is set on every cell write (including plain-space writes) and cleared on every reset — that is the ground truth this fix restores.

---

## 1. Root Cause Analysis

- **Symptom**: On a 3×3 terminal where the application writes `"A B"` (col 0='A', col 1=' ', col 2='B' — all default SGR), `CSI 1;1;1;1;1;3*y` returns DCS `ESC P 1 ! ~ FF7D ESC \` instead of xterm's `ESC P 1 ! ~ FF5D ESC \`. The middle `' '` contribution (`0x20`) is missing.

- **Proximate cause**: `oriterm_core/src/term/handler/rect_ops/mod.rs:235` uses `let drawn = !cell.is_empty();` as the CHARDRAWN proxy. The middle space cell has `ch = ' '`, default fg/bg, empty flags, no extra → `is_empty() == true` → `drawn == false` → `continue` skips it (unless `csNOTRIM|csDRAWN` is set).

- **Root cause**: `Cell::is_empty()` at `oriterm_core/src/cell/mod.rs:171-177` answers the question "does this cell look visually empty?" — NOT the question "was this cell ever written by the application?". The two concepts are distinct:
  - "Looks empty": every field at default (ch=' ', default fg/bg, empty flags, no extra).
  - "Was written": requires a persistent sticky bit set at write-time, independent of the cell's current visual content.

  ori_term conflates them, which is correct for *rendering* (a visually-empty drawn space renders the same as a pristine cell) but wrong for *checksum semantics* (xterm treats them differently).

- **Blast radius**:
  - **Primary**: DECRQCRA checksum correctness (§09A.5 deliverable).
  - **Secondary**: future rect-op implementations (§09A.6 DECCRA/DECFRA/DECERA/DECSERA/DECRARA/DECCARA) that may also need CHARDRAWN semantics (e.g., xterm's DECCRA preserves CHARDRAWN across copy; DECERA clears CHARDRAWN on erased cells).
  - **Tertiary**: any future feature that needs "was this column drawn" semantics.
  - **Existing tests**: 6 tests in `oriterm_core/src/term/handler/rect_ops/tests.rs` construct `Cell { ch: 'X', ..Cell::default() }` directly — those cells currently look "drawn" via `!is_empty()` because ch != ' ', but will look "undrawn" after the switch to `flags.contains(DRAWN)`. They need `flags: CellFlags::DRAWN` added.

- **Affected files**:
  - `oriterm_core/src/cell/mod.rs` — add `CellFlags::DRAWN` bit (uses bit 19, currently unused — bits 0..18 are SGR/structural flags; bit 20+ are available but 19 is the first free slot).
  - `oriterm_core/src/grid/editing/mod.rs` — set DRAWN in `put_char_ascii` (line 91), `put_char_slow` main cell (line 151), `put_char_slow` spacer (line 164), `put_char_slow` LEADING_WIDE_CHAR_SPACER (line 132).
  - `oriterm_core/src/term/handler/rect_ops/mod.rs` — replace `!cell.is_empty()` with `cell.flags.contains(CellFlags::DRAWN)` in both the skip path (line 235) and the trim-gate third disjunct (line 259 — the `drawn` variable feeds both).
  - `oriterm_core/src/term/handler/rect_ops/tests.rs` — update 2 direct-construction tests (`compute_rect_checksum_wide_char_spacer_not_trimmed`, `compute_rect_checksum_folds_combining_marks_in_notrim_mode`) to set `CellFlags::DRAWN`.
  - `oriterm_core/tests/alloc_regression.rs` — no change needed; pin still valid (direct compute call bypasses write paths).
  - `oriterm_core/src/cell/tests.rs` — add unit tests for DRAWN bit semantics (add default-clear, add via VTE write path, cleared via Cell::reset).

**Reference implementations:**
- **xterm** `~/projects/reference_repos/console_repos/xterm/screen.c:3178-3180` (skip path):
  ```c
  if (!(ld->attribs[col] & CHARDRAWN)) {
      if (!(mode & (csNOTRIM | csDRAWN)))
          continue;
      ch = ' ';
  }
  ```
- **xterm** `~/projects/reference_repos/console_repos/xterm/screen.c:3236` (DRAWX_MASK trim gate):
  ```c
  if (first || (ch != ' ') || (ld->attribs[col] & DRAWX_MASK)) {
  ```
- **xterm** `~/projects/reference_repos/console_repos/xterm/ptyx.h:3778`: `#define DRAWX_MASK (ATTRIBUTES | CHARDRAWN)` — DRAWX_MASK is CHARDRAWN OR any SGR attribute.

---

## 1.5 Fix Consensus (via /tp-help)

- **Proposed approach (pre-consensus)**: Add `CellFlags::DRAWN = 1 << 19` to the existing `bitflags!` block in `oriterm_core/src/cell/mod.rs`. At every cell-write site in `oriterm_core/src/grid/editing/mod.rs` (put_char_ascii, put_char_slow main/spacer/leading-spacer), OR in `CellFlags::DRAWN` when assigning `cell.flags`. Rely on SGR-only convention for `cursor.template.flags` (no code path sets DRAWN on it). Replace `!cell.is_empty()` in `compute_rect_checksum` with `cell.flags.contains(CellFlags::DRAWN)`. Update direct-construction tests. Reset paths clear DRAWN automatically via template copy.

- **tp-help run scratch dir**: `/tmp/tpr-help-ori_term-wwelfWOn`

### Round 1

- **Codex summary** (HIGH trust): Agreement on core design — `CellFlags::DRAWN` is the right SSOT-respecting model. **Persuaded divergence on scope**: flagged three missed write producers — (1) DECALN at `oriterm_core/src/term/handler/esc.rs:130-137` manually writes `'E'` via `cell.reset(&template)` + `cell.ch = 'E'` without going through grid editing, so those cells miss DRAWN; (2) resize/reflow at `oriterm_core/src/grid/resize/mod.rs:520-523, 549-553` synthesizes LEADING_WIDE_CHAR_SPACER boundaries and WIDE_CHAR_SPACER spacers via `Cell::default()` + flag insert — cloned base cells preserve DRAWN but synthesized spacers don't; (3) `push_zerowidth` via `cell.extra` mutation on the previously written cell is fine (previous cell already carries DRAWN from the put_char that wrote it). Additional hygiene recommendation: add a `debug_assert!` at the write/checksum boundary that cursor templates never carry internal cell-state bits (DRAWN, WRAP, wide-char spacer bits) since `Grid::cursor_mut()`/`Cursor::template_mut()` are public at `oriterm_core/src/grid/mod.rs:141-148` / `oriterm_core/src/grid/cursor/mod.rs:72-80` — invariant is by convention only, not by type.

- **Gemini summary** (LOWER trust): Agreement on core design. One missed write path: `Grid::push_zerowidth()` at `oriterm_core/src/grid/editing/mod.rs:220` should `cell.flags.insert(CellFlags::DRAWN)` on the target cell because combining-mark modification IS a draw operation. Also flagged that future §09A.6 stubs (DECCRA copy, DECFRA fill) will need DRAWN handling (out of this bug's scope; tracked for §09A.6).

- **Agreement points**:
  - `CellFlags::DRAWN` as a new bit in the existing `u32` bitfield (size invariant preserved).
  - Keep `Cell::is_empty()` orthogonal (visual-empty query, not write-history query) — changing it would be a LEAK per impl-hygiene.md.
  - Reset paths (`Cell::reset`, `Row::reset`, `clear_range`, `truncate`, BCE erase, scroll eviction) clear DRAWN via template copy — no additional instrumentation needed there.
  - Cursor template invariant holds across all verified mutators (SGR handler, soft reset, save/restore cursor clone, push/pop SGR snapshots).

- **Disagreement points**:
  - `push_zerowidth` target cell: gemini says "insert DRAWN defensively" (combining mark is a draw op); codex says "unnecessary, previous cell is always drawn." I adopt gemini's position because it's architecturally sound (the concept "combining mark appended to cell" IS a write operation) AND defensive against future callers that might target an undrawn cell.

- **Independent code verification** (file:line cites):
  - DECALN gap: verified at `oriterm_core/src/term/handler/esc.rs:130-137` — `let template = Cell::default();` followed by `cell.reset(&template); cell.ch = 'E';`. Confirmed DRAWN not set (template is DRAWN-clear, no subsequent DRAWN insert). Concrete repro: `ESC # 8` then DECRQCRA returns undrawn-skip checksum instead of 'E'×N sum. Real gap.
  - Reflow synthesized spacer gap: verified at `oriterm_core/src/grid/resize/mod.rs:515-523` (LEADING_WIDE_CHAR_SPACER boundary) and `:549-553` (WIDE_CHAR_SPACER spacer) — both use `Cell::default()` + `flags.insert(WIDE_CHAR_SPACER)` without DRAWN. Cloned base cell at `:537-544` DOES preserve DRAWN via `.clone()` since DRAWN lives in `flags`. Real gap on synthesized spacers only.
  - `push_zerowidth` at `oriterm_core/src/grid/editing/mod.rs:185-221` — calls `self.rows[line][Column(prev_col)].push_zerowidth(ch);` which mutates `Cell::extra.zerowidth` only. In all current production paths the target cell was just written (carries DRAWN), but the invariant is maintained by convention. Gemini's defensive insert is cheap (1 bit op per combining mark) and architecturally correct.
  - Template-hygiene concern: verified `Grid::cursor_mut()` is pub at `oriterm_core/src/grid/mod.rs:147-149` and `Cursor::template_mut()` is pub at `oriterm_core/src/grid/cursor/mod.rs`. No current writer sets internal bits on template, but nothing structurally prevents it. debug_assert is the right tool per code-hygiene.md §Comments "`debug_assert!` to document preconditions (executable > prose)".

- **Outcome**: **Persuaded divergence** → proceed to Phase 2 with expanded scope. Fix now covers 6 write sites (not 4) + a hygiene debug_assert.

### Final agreed approach

1. Add `CellFlags::DRAWN = 1 << 19` (matches both reviewers' recommendation).
2. Define a helper flag union `CellFlags::INTERNAL_CELL_STATE = DRAWN | WRAP | WIDE_CHAR | WIDE_CHAR_SPACER | LEADING_WIDE_CHAR_SPACER` to document which bits must never appear on `cursor.template.flags`.
3. Set `DRAWN` at **6 cell-write sites** (not 4 as originally proposed):
   - `put_char_ascii` (main cell) — `oriterm_core/src/grid/editing/mod.rs:91`
   - `put_char_slow` (main cell) — `oriterm_core/src/grid/editing/mod.rs:151`
   - `put_char_slow` (wide-char spacer) — `oriterm_core/src/grid/editing/mod.rs:164`
   - `put_char_slow` (LEADING_WIDE_CHAR_SPACER boundary) — `oriterm_core/src/grid/editing/mod.rs:132`
   - `Grid::push_zerowidth` (combining-mark target cell) — `oriterm_core/src/grid/editing/mod.rs:220`
   - `DECALN` handler (every visible cell after the 'E' fill) — `oriterm_core/src/term/handler/esc.rs:130-137`
   - Reflow synthesized spacers — `oriterm_core/src/grid/resize/mod.rs:520-523, 549-553`

   (That's actually 7 sites when counted at statement granularity; "6 write sites" counts DECALN as one site and reflow boundary+spacer as one site.)

4. Add `debug_assert!(self.cursor.template.flags.intersection(CellFlags::INTERNAL_CELL_STATE).is_empty(), "cursor template must not carry internal cell-state bits")` at the top of `put_char_ascii` and `put_char_slow`. This structurally enforces the "template never carries internal bits" invariant.
5. Update `compute_rect_checksum` to use `cell.flags.contains(CellFlags::DRAWN)` for both the skip path and the DRAWX_MASK trim-gate disjunct.
6. Update existing direct-construction tests in `rect_ops/tests.rs` to set `CellFlags::DRAWN` where a drawn cell is intended.
7. Expand TDD matrix to include:
   - DECALN test: `ESC # 8` then DECRQCRA sees all cells as drawn.
   - Wide-char reflow test: write wide content, resize, DRAWN preserved on base AND synthesized spacer.
   - push_zerowidth test: combining-mark target cell has DRAWN.
   - Template-invariant test: debug_assert trips if any code path sets DRAWN on cursor.template.flags (assertion check via `#[should_panic]` in debug builds).

8. Note for future §09A.6 work: DECCRA (copy) must propagate DRAWN from source cells; DECFRA (fill) must set DRAWN on filled cells. Out of this bug's scope but filed as a reminder in the fix section's §4 checklist.

---

## 2. TDD — Test Matrix

Write ALL tests BEFORE the fix. Verify they fail against current code.

### Exact failing case (repro from bug entry)
- [ ] `decrqcra_explicit_spaces_match_xterm` — 1×3 grid, feed `"A B"` via VTE, DECRQCRA returns `\x1bP1!~FF5D\x1b\\`. Semantic pin against the specific byte sequence xterm produces.

### Edge cases
- [ ] `default_cell_has_drawn_clear` — `Cell::default().flags.contains(CellFlags::DRAWN) == false`.
- [ ] `cell_reset_clears_drawn` — construct a drawn cell, `reset(&Cell::default())`, assert DRAWN clear.
- [ ] `row_reset_clears_drawn` — fill row with writes, `row.reset(cols, &Cell::default())`, every cell has DRAWN clear.
- [ ] `row_clear_range_clears_drawn` — fill row, `clear_range(0..3, &Cell::default())`, cleared cells have DRAWN clear.

### Cross-type coverage (write paths)
- [ ] `put_char_ascii_sets_drawn` — `grid.put_char_ascii('A')`, target cell has DRAWN.
- [ ] `put_char_ascii_space_sets_drawn` — `grid.put_char_ascii(' ')`, target cell has DRAWN (regression pin for the very bug).
- [ ] `put_char_slow_wide_sets_drawn_on_both` — write a wide CJK char; base cell + spacer both have DRAWN.
- [ ] `put_char_slow_leading_wide_spacer_sets_drawn` — write a wide char at last column with wrap; the LEADING_WIDE_CHAR_SPACER boundary cell has DRAWN.
- [ ] `push_zerowidth_preserves_drawn` — write 'a', push combining '\u{0301}'; cell still has DRAWN.

### Cross-pattern coverage (checksum consumer)
- [ ] `compute_rect_checksum_explicit_space_counted` — direct-call unit test: construct cells with `flags: CellFlags::DRAWN`, verify `' '` cell contributes to checksum.
- [ ] `compute_rect_checksum_pristine_cell_skipped` — `Cell::default()` cell (DRAWN clear) is skipped in default mode.
- [ ] `compute_rect_checksum_drawn_space_in_trim_gate` — drawn `' '` cell in middle of row passes the DRAWX_MASK analog and pushes into `trimmed`.

### Cross-feature interactions
- [ ] `reflow_preserves_drawn` — write content on a 10-col grid, resize to 20 cols (grow) and back to 10, DRAWN state preserved on every cell.
- [ ] `scrollback_reset_clears_drawn` — fill grid past scrollback cap, recycled rows have all-cells-DRAWN-clear (via `Row::reset`).
- [ ] `insert_blank_preserves_shifted_cells_drawn` — write 'ABCD', insert 2 blanks at col 1, shifted cells retain DRAWN, inserted blanks have DRAWN clear.
- [ ] `delete_chars_fills_with_undrawn_blanks` — write 'ABCD', delete 2 at col 1, shifted cells retain DRAWN, new blank tail cells have DRAWN clear.
- [ ] `erase_chars_clears_drawn` — ECH on 3 cells clears DRAWN.
- [ ] `erase_in_display_clears_drawn` — ED clears DRAWN on erased region.
- [ ] `erase_in_line_clears_drawn` — EL clears DRAWN on erased line range.

### Semantic pin
- [ ] `decrqcra_a_space_b_matches_xterm_ff5d` — the exact failing case. Pass verifies xterm byte-parity; failure reverts to ori_term's pre-fix 0xFF7D.

### Negative pin
- [ ] `pristine_cell_does_not_contribute_to_default_checksum` — 3×3 pristine grid → checksum `0000` in default mode (not `-(9 * 0x20)` which would happen if DRAWN were incorrectly set on default cells).

### Verify tests fail before fix
- [ ] All new semantic + negative tests fail against current code (confirming they test the right thing).

---

## 2.5 Fix Plan TPR Findings

**Gate**: Severity is medium, subsystem is `oriterm_core/cell` + `oriterm_core/grid/editing` (NOT a complexity-elevated subsystem per the ori_term gate criteria — elevated list is ori_lang-specific: AIMS / CodeGen / LLVM / AOT / Runtime). `/tp-help` converged in round 1 with persuaded divergence on scope expansion (3 extra write sites + debug_assert). Round-2 consensus would be redundant.

Plan TPR: Skipped — medium severity, non-elevated subsystem, round-1 consensus.

---

## 3. Implementation

- [ ] **Step 1 — Add DRAWN + INTERNAL_CELL_STATE to CellFlags** (`oriterm_core/src/cell/mod.rs`):
  ```rust
  bitflags! {
      pub struct CellFlags: u32 {
          // ... existing SGR + structural bits (0..18) ...
          const SUBSCRIPT         = 1 << 18;
          /// Cell has been written by the application (xterm CHARDRAWN
          /// equivalent). Set on every put_char / wide-spacer / leading-
          /// wide-spacer / DECALN / reflow-spacer / push_zerowidth write
          /// path; cleared on every reset path via template copy
          /// (templates must never carry DRAWN — enforced by
          /// debug_assert at write sites). Consumed by DECRQCRA to
          /// distinguish application-written blanks from pristine cells.
          /// See plans/bug-tracker/fix-BUG-08-17.md.
          const DRAWN             = 1 << 19;

          /// Internal cell-state bits that must NEVER appear on a
          /// `cursor.template.flags` value. SGR attributes (BOLD,
          /// UNDERLINE, etc.) are template-legal; these structural
          /// bits are set only by concrete cell-write paths and would
          /// corrupt written cells if they leaked through the template.
          const INTERNAL_CELL_STATE = Self::DRAWN.bits()
              | Self::WRAP.bits()
              | Self::WIDE_CHAR.bits()
              | Self::WIDE_CHAR_SPACER.bits()
              | Self::LEADING_WIDE_CHAR_SPACER.bits();
      }
  }
  ```

- [ ] **Step 2 — Wire DRAWN at every put_char write site + add hygiene debug_assert** (`oriterm_core/src/grid/editing/mod.rs`):
  - Top of `put_char_ascii`: `debug_assert!(self.cursor.template.flags.intersection(CellFlags::INTERNAL_CELL_STATE).is_empty(), "cursor template must not carry internal cell-state bits");`
  - `put_char_ascii` main cell: `cell.flags = self.cursor.template.flags | CellFlags::DRAWN;`
  - Top of `put_char_slow`: same `debug_assert!` as above.
  - `put_char_slow` main cell: `cell.flags = tmpl_flags | CellFlags::DRAWN;`
  - `put_char_slow` wide-char spacer: `spacer.flags = CellFlags::WIDE_CHAR_SPACER | CellFlags::DRAWN;`
  - `put_char_slow` LEADING_WIDE_CHAR_SPACER: `boundary.flags = CellFlags::LEADING_WIDE_CHAR_SPACER | CellFlags::WRAP | CellFlags::DRAWN;`
  - `push_zerowidth` (gemini-recommended, codex-verified-as-defensive): before the `push_zerowidth(ch)` call at line 220, add `self.rows[line][Column(prev_col)].flags.insert(CellFlags::DRAWN);` — combining-mark modification IS a draw operation.

- [ ] **Step 3 — DECALN sets DRAWN on every filled cell** (`oriterm_core/src/term/handler/esc.rs:130-137`):
  ```rust
  // Fill every visible cell with 'E' and default attributes.
  let template = Cell::default();
  for line in 0..lines {
      for col in 0..cols {
          let cell = &mut grid[Line(line as i32)][Column(col)];
          cell.reset(&template);
          cell.ch = 'E';
          cell.flags.insert(CellFlags::DRAWN);  // NEW
      }
  }
  ```
  Add the import `use crate::cell::CellFlags;` if not already present.

- [ ] **Step 4 — Reflow synthesized spacers carry DRAWN** (`oriterm_core/src/grid/resize/mod.rs`):
  - Line 520-523 (LEADING_WIDE_CHAR_SPACER boundary): after `boundary.flags.insert(CellFlags::LEADING_WIDE_CHAR_SPACER);`, also `boundary.flags.insert(CellFlags::DRAWN);` (or combine into one `insert(LEADING_WIDE_CHAR_SPACER | DRAWN)` call).
  - Line 549-553 (WIDE_CHAR_SPACER): after `spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);`, also `spacer.flags.insert(CellFlags::DRAWN);`.
  - Clone path at line 537-544 already preserves DRAWN via `.clone()` (DRAWN lives in `flags`). No change needed there.

- [ ] **Step 5 — Update compute_rect_checksum to consult DRAWN** (`oriterm_core/src/term/handler/rect_ops/mod.rs`):
  - Replace `let drawn = !cell.is_empty();` → `let drawn = cell.flags.contains(CellFlags::DRAWN);`
  - Update module-doc: the csBYTE paragraph in the "Structural deviations from xterm" block should mention that ori_term now has a real CHARDRAWN analog, so the semantic parity is tighter than before.

- [ ] **Step 6 — Update existing direct-construction tests** (`oriterm_core/src/term/handler/rect_ops/tests.rs`):
  - `compute_rect_checksum_wide_char_spacer_not_trimmed`: add `flags: CellFlags::WIDE_CHAR_SPACER | CellFlags::DRAWN` to the spacer cell construction; add `flags: CellFlags::DRAWN` to the 'A' cell.
  - `compute_rect_checksum_folds_combining_marks_in_notrim_mode`: add `flags: CellFlags::DRAWN` to the 'a' cell.
  - `compute_rect_checksum_trim_state_crosses_rows`: uses pristine cells + csDRAWN flag — NO CHANGE (intentionally tests undrawn-cell substitution).

- [ ] **Step 7 — Add new TDD matrix tests** per §2 above, distributed to their natural homes:
  - `oriterm_core/src/cell/tests.rs`: `default_cell_has_drawn_clear`, `cell_reset_clears_drawn`, `internal_cell_state_flag_union_includes_drawn_wrap_and_wide_bits`.
  - `oriterm_core/src/grid/editing/tests.rs`: `put_char_ascii_sets_drawn`, `put_char_ascii_space_sets_drawn` (the repro pin), `put_char_slow_wide_sets_drawn_on_both`, `put_char_slow_leading_wide_spacer_sets_drawn`, `push_zerowidth_sets_drawn_on_target`, `insert_blank_preserves_shifted_cells_drawn`, `insert_blank_fills_undrawn_blanks`, `delete_chars_fills_with_undrawn_blanks`.
  - `oriterm_core/src/grid/editing/erase.rs` sibling tests (or `editing/tests.rs`): `erase_chars_clears_drawn`, `erase_in_display_clears_drawn`, `erase_in_line_clears_drawn`.
  - `oriterm_core/src/grid/row/tests.rs`: `row_reset_clears_drawn`, `row_clear_range_clears_drawn`, `row_truncate_clears_drawn`.
  - `oriterm_core/src/grid/resize/tests.rs`: `reflow_preserves_drawn_on_cloned_cells`, `reflow_synthesized_wide_char_spacer_has_drawn`, `reflow_synthesized_leading_spacer_has_drawn`.
  - `oriterm_core/src/term/handler/tests/esc.rs` or similar (find DECALN's current test home): `decaln_sets_drawn_on_every_cell`.
  - `oriterm_core/tests/spec_chain/dec_rect_ops/decrqcra.rs`: `decrqcra_explicit_spaces_match_xterm_ff5d` — the exact failing case, direct xterm byte-parity pin.

- [ ] **Step 8 — Verify size invariant**: `const _: () = assert!(size_of::<Cell>() <= 24);` still passes. DRAWN is a bit in the existing u32 flags field; no struct growth.

- [ ] **Step 9 — Note for future §09A.6 work**: DECCRA (copy) must propagate DRAWN from source cells when it lands; DECFRA (fill) must set DRAWN on filled cells. Add a comment in the §09A.6 stubs at `oriterm_core/src/term/handler/rect_ops/mod.rs` pointing at this fix section so future implementers pick it up.

---

## R. Third Party Review Findings

3-round code TPR on commits `372f448e` + `6dd30e5a` + `6c966f2b`.

### Round 0 (2026-04-19; commits `372f448e` → `6dd30e5a`)

- `[TPR-BUG-08-17-1-codex][high]` `oriterm_core/src/cell/mod.rs:199` — DRAWN leaks into `Cell::is_empty()` via `flags.is_empty()`, changing `Row::is_blank`/`content_len` semantics at reflow sites (`resize/mod.rs:222` + `:407`). **Fixed in `6dd30e5a`** — `is_empty()` masks DRAWN via `(self.flags - CellFlags::DRAWN).is_empty()`, restoring visual-empty orthogonality.
- `[TPR-BUG-08-17-2-codex][low]` `rect_ops/mod.rs:171` — stale doc reference to "`Cell::is_empty()`" proxy. **Fixed in `6dd30e5a`** — updated to cite `CellFlags::DRAWN`.
- `[TPR-BUG-08-17-1-gemini][high]` `cell/mod.rs:236` — claimed `is_empty()` contained `!self.flags.contains(CellFlags::DRAWN)` + `CellFlags::SGR_MASK`. **Dropped at verification**: neither construct exists in actual code at `cell/mod.rs:195-201` (hallucinated).

### Round 1 (2026-04-19; commit `6c966f2b`)

- `[TPR-BUG-08-17-1-codex-r1][low]` `grid/row/tests.rs:277` — row-level regression tests only cover pristine blanks, not DRAWN-only cells. **Fixed in `6c966f2b`** — added 3 row-level pins: `is_blank_true_for_drawn_only_cells`, `content_len_zero_for_drawn_only_row`, `content_len_ignores_drawn_only_trailing_cells`.
- `[TPR-BUG-08-17-2-codex-r1][informational]` `cell/mod.rs:206` — verified DRAWN masked before blank/reflow consumers. **Informational only; no code change.**
- `[TPR-BUG-08-17-1-gemini-r1][medium]` `image/sixel.rs:102` — claimed sixel placement should set DRAWN on occupied cells. **Dropped at verification**: image protocols in ori_term overlay visually without mutating grid cells; xterm CHARDRAWN is set only by character-draw paths (`drawXtermText`), not by image placement — DECRQCRA is a character checksum, not pixel-based. ori_term behavior matches xterm.
- `[TPR-BUG-08-17-2-gemini-r1][medium]` `image/kitty.rs:492` — same as above for kitty protocol. **Dropped at verification**: same xterm-parity reasoning.

### Round 2 (2026-04-20; no commits needed)

- `[TPR-BUG-08-17-1-codex-r2][informational]` `cell/mod.rs:204` — verified DRAWN orthogonal to `Cell::is_empty()`.
- `[TPR-BUG-08-17-2-codex-r2][informational]` `rect_ops/mod.rs:241` — verified `compute_rect_checksum` keys off DRAWN, xterm parity at `screen.c:3178-3182, 3236-3240`.
- `[TPR-BUG-08-17-3-codex-r2][informational]` `image/kitty.rs:455` — verified sixel/kitty stay out of CHARDRAWN semantics, cross-checked against xterm `graphics.c:694-699` + `graphics_sixel.c:396-405`.
- `[TPR-BUG-08-17-1-gemini-r2][informational]` `cell/mod.rs:198` — `status: clean` with verification entry confirming all 3 commits.

**Outcome**: Clean convergence after 3 rounds. Gemini status=clean in round 2; codex returned only informational verifications. Zero actionable findings remaining.

---

## 4. Completion Checklist

- [x] All new tests pass unchanged after fix (no test modifications needed)
- [x] Matrix completeness verified — write paths × reset paths × consumer (compute_rect_checksum) covered
- [x] Debug AND release builds pass
- [x] Windows cross-compile green
- [x] `oriterm_core/tests/alloc_regression.rs` still green (DECRQCRA pin unaffected — helper call is unchanged)
- [x] `timeout 150 ./test-all.sh` green — no regressions (1894 lib + 2741 core + 582 spec_chain + 176 teseq + all others)
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green
- [x] `cargo test -p oriterm_core` green
- [x] `/commit-push` — 3 commits: `372f448e` (infra), `6dd30e5a` (TPR r0 fix), `6c966f2b` (TPR r1 pins)
- [x] Plan TPR (Phase 2.5) — Skipped per gate: medium severity, non-elevated subsystem, round-1 consensus (§2.5)
- [x] `/tpr-review` (Phase 5) passed — 3 rounds, clean convergence; see §R.
- [x] `/impl-hygiene-review` passed — 1 BLOAT finding filed as BUG-08-18 (`resize/mod.rs` 569 > 500 lines, pre-existing, touched by fix); all other categories clean (SSOT / no-side-logic / algorithmic-DRY / registration-sync / boundary-discipline).
- [x] Capability regression gate — fix is purely additive; no capability disabled
- [x] `/improve-tooling` retrospective completed — 3 improvements committed (`cb140fa5` Cell::drawn(ch) test helper; `33462410` tp_agent_prompt Tier-5 tightening on 429; `9b8c2919` xterm-reference cheatsheet). 2 deferred (Grid::debug_dump speculative; assert_drawn! macros marginal).
- [x] Bug entry in `plans/bug-tracker/section-08-core-terminal.md` updated: `- [x]` with resolution (2026-04-20)
- [x] Fix section frontmatter `status: complete`
- [ ] `plans/bug-tracker/00-overview.md` Quick Reference open bug count — Quick Reference table has pre-existing drift in the 08 row (Total=7 < Open=10 before my change); my fix nets -1 + 1 (close BUG-08-17, add BUG-08-18) for zero count change. Leaving table alone; reconciliation is a separate concern.
- [x] Final `/commit-push` for closure artifacts — pending this commit

**Exit Criteria:** `cargo test -p oriterm_core --test spec_chain dec_rect_ops::decrqcra::decrqcra_explicit_spaces_match_xterm` passes against the produced DCS reply bytes `\x1bP1!~FF5D\x1b\\` (the xterm byte-parity value). All 1868+ lib tests, 582 spec_chain tests, 176 teseq tests, and full `./test-all.sh` green. `./clippy-all.sh` and `./build-all.sh` green. §09A.5 TPR round 2 (re-run after this fix lands) completes with no new findings on the CHARDRAWN axis.
