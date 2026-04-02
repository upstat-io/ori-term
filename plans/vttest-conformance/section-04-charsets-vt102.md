---
section: "04"
title: "Character Sets & VT102 Features"
status: not-started
reviewed: true
goal: "vttest menus 3 and 8 pass — character set switching and VT102 insert/delete operations"
inspired_by:
  - "WezTerm charset handling (term/src/terminalstate/charset.rs)"
  - "Alacritty charset (alacritty_terminal/src/term/mod.rs CharsetMapping)"
depends_on: ["02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Character Set Switching (Menu 3)"
    status: not-started
  - id: "04.2"
    title: "VT102 Insert/Delete (Menu 8)"
    status: not-started
  - id: "04.3"
    title: "Test Automation & Assertions"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Character Sets & VT102 Features

**Status:** Not Started
**Goal:** vttest menus 3 (character sets) and 8 (VT102 insert/delete) pass at 80x24 with structural assertions.

**Context:** Menu 3 tests G0/G1 character set designation (SCS) and the DEC Special Graphics set (line drawing characters). Menu 8 tests VT102 features: ICH (insert character), DCH (delete character), IL (insert line), DL (delete line). These are fundamental operations that many TUI applications depend on.

**Reference implementations:**
- **WezTerm** `term/src/terminalstate/charset.rs`: charset designation and translation.
- **Alacritty** `alacritty_terminal/src/term/mod.rs`: `CharsetMapping` for G0/G1 translation tables.

**Depends on:** Section 02 (scroll region fixes needed for insert/delete line operations).

---

## 04.1 Character Set Switching (Menu 3)

**File(s):** `oriterm_core/src/term/charset/mod.rs`, `oriterm_core/src/term/charset/tests.rs`, `oriterm_core/src/term/handler/mod.rs:40-76` (input() fast path at :50 + charset translate at :59)

Menu 3 tests:
1. DEC Special Graphics set (line drawing: `jklmnopqrstuvwxyz` map to box-drawing glyphs)
2. UK National set (# to pound)
3. US ASCII set (baseline)
4. G0/G1 designation and invocation (SO/SI)

Charset infrastructure exists: `CharsetState` at `charset/mod.rs` has `translate()`, `set_charset()`, `set_active()`, `set_single_shift()`. The actual character mapping is delegated to `vte::ansi::StandardCharset::map()` in the vendored VTE crate. Existing tests in `charset/tests.rs` cover basic translation.

- [ ] Run vttest menu 3 headlessly, capture all screens
- [ ] Verify DEC Special Graphics mapping: characters `jklmnopqrstuvwxyz` map to box-drawing glyphs via `StandardCharset::map()`
- [ ] Verify SO (Shift Out, 0x0E) invokes G1 via `set_active_charset(G1)` at `handler/mod.rs:121`, and SI (Shift In, 0x0F) invokes G0 via `set_active_charset(G0)` -- both dispatch through the VTE crate's C0 handler
- [ ] Verify SCS designator sequences: `ESC ( 0` (G0=Special), `ESC ) 0` (G1=Special), `ESC ( B` (G0=ASCII) -- dispatch through `configure_charset()` at `handler/mod.rs:126`
- [ ] Add structural assertions for line drawing characters at expected positions
- [ ] Fix any charset translation bugs found
- [ ] Verify the `input()` fast path at `handler/mod.rs:50` correctly skips the fast path when charset is non-ASCII (it checks `self.charset.is_ascii()` -- verify this works for G1 invocation)
- [ ] Add unit test: `charset_dec_special_graphics_via_handler` -- feed `ESC ( 0` then printable chars `jklmnopqrstuvwxyz` through the full VTE handler (`Term::input`), verify the grid cells contain the corresponding box-drawing Unicode characters
- [ ] Add unit test: `charset_so_si_invocation` -- feed `ESC ) 0` (designate G1=Special), then SO (0x0E, invoke G1), then printable `q`, then SI (0x0F, invoke G0), then printable `q`. Verify: first `q` maps to box-drawing, second `q` is literal ASCII `q`.
- [ ] Add unit test: `charset_single_shift_ss2_ss3` -- if single shift (SS2/SS3) is supported, verify it maps one character then reverts. If not supported, document as known gap.

---

## 04.2 VT102 Insert/Delete (Menu 8)

**File(s):**
- ICH: `oriterm_core/src/grid/editing/mod.rs:225` (`insert_blank`)
- DCH: `oriterm_core/src/grid/editing/mod.rs:284` (`delete_chars`)
- IL: `oriterm_core/src/grid/scroll/mod.rs:102` (`insert_lines`)
- DL: `oriterm_core/src/grid/scroll/mod.rs:116` (`delete_lines`)
- Handler dispatch: `oriterm_core/src/term/handler/mod.rs:201-219` (insert_blank:201, delete_chars:206, insert_blank_lines:211, delete_lines:216)

Menu 8 tests:
1. ICH -- Insert Character (CSI n @): shift characters right, insert blanks
2. DCH -- Delete Character (CSI n P): shift characters left, fill blanks at right margin
3. IL -- Insert Line (CSI n L): shift lines down within scroll region, insert blank lines
4. DL -- Delete Line (CSI n M): shift lines up within scroll region, fill blanks at bottom

IL/DL already have extensive tests in `grid/scroll/tests.rs` (insert_lines: 9 tests, delete_lines: 8 tests). ICH/DCH have tests in `grid/editing/tests.rs`. Focus on vttest-specific interaction gaps.

- [ ] Run vttest menu 8 headlessly, capture all screens
- [ ] Verify ICH implementation: characters shift right, no wrap, blanks inserted at cursor
- [ ] Verify DCH implementation: characters shift left, blanks fill from right margin
- [ ] Verify IL implementation: operates within scroll region, blank lines inserted at cursor
- [ ] Verify DL implementation: operates within scroll region, blank lines added at bottom of region
- [ ] Add structural assertions for each operation
- [ ] Fix any insert/delete bugs found (especially interaction with scroll regions)
- [ ] Add unit test (if missing): `ich_at_right_margin` -- ICH when cursor is at the last column: characters should NOT wrap to next line; content beyond right margin is lost
- [ ] Add unit test (if missing): `dch_fills_from_right_margin` -- DCH removes N chars at cursor, remaining chars shift left, blanks fill from the right margin of the line (not the screen)
- [ ] Add unit test (if missing): `il_dl_within_scroll_region` -- IL/DL at cursor row when scroll region is active: lines shift within the region only, content outside the region is untouched
- [ ] Add unit test (if missing): `irm_insert_mode` -- IRM (CSI 4 h) enables insert mode: each typed character shifts existing characters right instead of overwriting. Verify with a 5-char string, position cursor at col 2, type 'X', verify 'X' inserted and last char pushed to col 6 (or lost if at margin)
- [ ] `/tpr-review` checkpoint

---

## 04.3 Test Automation & Assertions

- [ ] Add `run_menu3_golden` and `run_menu8_golden` to GPU visual regression tests
- [ ] Add `run_menu3_cursor_movement` and `run_menu8_vt102` to text snapshot tests
- [ ] Add structural assertions for character set display (verify line drawing chars are present)
- [ ] Add structural assertions for insert/delete results (character positions after operations)
- [ ] Regenerate all golden references

---

## 04.R Third Party Review Findings

- None.

---

## 04.N Completion Checklist

- [ ] Menu 3 (character sets) renders DEC Special Graphics correctly
- [ ] Menu 8 (VT102) ICH/DCH/IL/DL produce correct results
- [ ] Character set switching (G0/G1, SO/SI) verified
- [ ] Insert/delete operations work correctly within scroll regions
- [ ] Golden PNGs generated for menus 3 and 8
- [ ] Structural assertions for key screens
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** vttest menus 3 and 8 produce correct output at 80x24, verified by structural assertions and golden images.
