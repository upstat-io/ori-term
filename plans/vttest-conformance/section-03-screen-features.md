---
section: "03"
title: "Screen Features & DECCOLM"
status: not-started
reviewed: true
goal: "vttest menu 2 screens pass at all sizes — wrap, tabs, column mode, scroll, SGR rendition"
inspired_by:
  - "WezTerm DECCOLM (term/src/terminalstate/mod.rs set_dec_mode)"
  - "xterm DECCOLM (charproc.c RequestResize)"
depends_on: ["02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "DECCOLM Reflow (132-Column Mode)"
    status: not-started
  - id: "03.2"
    title: "Wrap-Around Mode (DECAWM)"
    status: not-started
  - id: "03.3"
    title: "Tab Stops (HTS/TBC)"
    status: not-started
  - id: "03.4"
    title: "Scroll Modes & Origin Mode Screen Tests"
    status: not-started
  - id: "03.5"
    title: "SGR Graphic Rendition"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Screen Features & DECCOLM

**Status:** Not Started
**Goal:** vttest menu 2 (screen features) passes at all terminal sizes. DECCOLM reflows content to the current terminal width rather than physically resizing. Wrap-around, tab stops, scroll modes, and SGR rendition all produce correct output.

**Context:** Menu 2 tests 15 screens covering wrap-around (DECAWM), tab stops, 132-column mode, soft/jump scroll, origin mode screen placement, and SGR graphic rendition (bold, underline, blink, inverse). DECCOLM is currently stubbed out. The user's design decision: DECCOLM should NOT resize the window — content should reflow to the current width. Tab stops and SGR are likely working but need verification.

**Reference implementations:**
- **WezTerm** `term/src/terminalstate/mod.rs`: handles DECCOLM by reflowing, not resizing.
- **xterm** `charproc.c`: physically resizes the window — the spec-correct but UX-hostile approach.

**Depends on:** Section 02 (origin mode fixes are needed for menu 2 screens 11-12).

---

## 03.1 DECCOLM Reflow (132-Column Mode)

**File(s):** `oriterm_core/src/term/handler/modes.rs:94-96`

Currently DECCOLM is stubbed in `apply_decset()` at `modes.rs:94`:
```rust
NamedPrivateMode::ColumnMode => {
    debug!("Ignoring DECSET for unimplemented mode {named:?}");
}
```
And mirrored in `apply_decrst()` at `modes.rs:160`.

Per the user's decision, DECCOLM should NOT resize. Instead:
1. When DECCOLM is set (CSI ? 3 h): clear the screen, home the cursor, reset scroll region. The grid width stays unchanged.
2. When DECCOLM is reset (CSI ? 3 l): same -- clear, home, reset region.
3. This matches what happens when DECCOLM changes the column count, minus the actual resize. vttest's 132-column mode screens will render at the current width.

Implementation: reset scroll region via `self.grid_mut().set_scroll_region(1, None)`, clear screen via `self.grid_mut().erase_display(DisplayEraseMode::All)` and `self.clear_images_after_ed(&ClearMode::All)` (matching the `clear_screen` handler at `handler/mod.rs:172`), then home cursor via `self.goto_origin_aware(0, 0)`. Order matters: reset scroll region first (so `goto_origin_aware` uses the full screen), then clear, then home.

- [ ] Implement DECCOLM set: reset scroll region, clear screen (ED 2), home cursor
- [ ] Implement DECCOLM reset: same behavior
- [ ] Add unit test: `deccolm_set_clears_screen` -- set DECCOLM, verify all cells are blank and cursor is at (0,0)
- [ ] Add unit test: `deccolm_reset_clears_screen` -- reset DECCOLM, verify same side effects
- [ ] Add unit test: `deccolm_preserves_grid_dimensions` -- grid is still 80x24 (or whatever size) after DECCOLM toggle
- [ ] Check whether the DECCOLM-allowed flag (CSI ? 40 h, "allow 132 column mode") affects behavior. If vttest sets it before DECCOLM tests, document that our implementation always allows DECCOLM (no gating on the allow flag). If vttest does NOT set it, DECCOLM may need to be gated.
- [ ] Verify menu 2 screens 03-06 render -- content designed for 132 columns will wrap at the current width. This is expected behavior per the design decision (no resize). Verify DECCOLM side effects are correct (clear screen, home cursor, reset scroll region) even though the visual output differs from xterm. Count these as partial passes in the scoring.

---

## 03.2 Wrap-Around Mode (DECAWM)

**File(s):** `oriterm_core/src/grid/editing/mod.rs`, `oriterm_core/src/term/handler/modes.rs`

Menu 2, screen 01: "three identical lines of *'s completely filling the top of the screen without any empty lines between." This tests DECAWM (auto-wrap).

- [ ] Verify wrap-around mode (LINE_WRAP flag) is correctly set/reset via DECSET/DECRST 7
- [ ] Add structural assertion: top 3 rows fully filled with `*` at all widths (already partially done in `assert_wrap_fills_top`)
- [ ] Verify wrap-around with control characters (screen 02: mixing control and print characters)
- [ ] Fix any wrap edge cases found
- [ ] Add unit test: `decawm_wrap_fills_line` -- write exactly `cols` characters with DECAWM on, verify cursor wraps to next line column 0
- [ ] Add unit test: `decawm_off_no_wrap` -- write `cols + 5` characters with DECAWM off, verify last character overwrites the rightmost column repeatedly, cursor stays at last column
- [ ] Add unit test: `decawm_with_control_chars` -- write a mix of printable and control characters (BS, CR, LF), verify wrapping interacts correctly with control character processing

---

## 03.3 Tab Stops (HTS/TBC)

**File(s):** `oriterm_core/src/grid/mod.rs:47` (tab_stops field), `oriterm_core/src/grid/navigation/mod.rs:132-185` (tab/tab_backward/set_tab_stop/clear_tab_stop), `oriterm_core/src/term/handler/mod.rs:85-90` (put_tab), `oriterm_core/src/term/handler/mod.rs:256-266` (set_horizontal_tabstop, clear_tabs)

Menu 2, screen 02: "These two lines should look the same" — tests tab stop setting and resetting.

Tab stops already have test coverage in `grid/navigation/tests.rs` (set/clear tests) and `grid/tests.rs` (default every-8 tests). Focus on vttest-specific gaps.

- [ ] Verify HTS (set tab stop at current column) implementation at `grid/navigation/mod.rs:165`
- [ ] Verify TBC (clear tab stop) modes: 0 = current, 3 = all at `grid/navigation/mod.rs:173`
- [ ] Verify CHT (cursor forward tab) at `grid/navigation/mod.rs:132` and CBT (cursor backward tab) at `grid/navigation/mod.rs:148`
- [ ] Add structural assertion: tab-aligned `*` characters match expected positions
- [ ] Fix any tab stop bugs found
- [ ] Add unit test (if missing): `tab_stop_across_set_and_clear` -- set custom tab stops, clear specific ones, verify tab advances to the correct remaining stops
- [ ] Add unit test (if missing): `tab_stop_at_right_margin` -- tab at last tab stop wraps or stops at right margin (depends on DECAWM)

---

## 03.4 Scroll Modes & Origin Mode Screen Tests

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`

Menu 2, screens 07-12: soft/jump scroll in various region configurations, origin mode placement tests.

- [ ] Verify smooth scroll mode (DECSCLM) -- oriterm should ignore this (always jump scroll, modern terminals don't do smooth scroll). Verify DECSCLM (CSI ? 4 h) is in the NamedPrivateMode enum. If missing, add as a no-op to avoid "unrecognized mode" debug spam during vttest.
- [ ] Add structural assertions for scroll region content placement
- [ ] Verify origin mode screen 11: "This line should be at the bottom of the screen"
- [ ] Verify origin mode screen 12: "This line should be at the top of the screen"
- [ ] Fix any scroll region + origin mode interaction bugs (depends on Section 02 fixes)
- [ ] Add structural assertion for screen 11: last visible row contains "This line should be at the bottom of the screen" (or a substring)
- [ ] Add structural assertion for screen 12: first visible row contains "This line should be at the top of the screen" (or a substring)

---

## 03.5 SGR Graphic Rendition

**File(s):** `oriterm_core/src/term/handler/sgr.rs`, `oriterm_core/src/cell.rs`

Menu 2, screens 13-14: graphic rendition test pattern (vanilla, bold, underline, blink, inverse, and combinations).

SGR handling already has 40+ unit tests in `handler/tests.rs` covering bold, italic, underline variants, blink, inverse, hidden, strikethrough, 256-color, truecolor, reset, etc. The vttest screens are primarily a visual verification that the GPU renderer correctly translates `CellFlags` to pixels.

- [ ] Verify all SGR attributes render correctly: bold, underline, blink, inverse (negative)
- [ ] Verify combined attributes: bold+underline, bold+blink, underline+blink, etc.
- [ ] Verify light vs dark background switching (reverse video mode)
- [ ] Add golden image test covering the SGR pattern screen -- this is where colors/attributes are visually verified
- [ ] Fix any missing SGR attribute handling (unlikely given existing coverage)

Menu 2, screen 15: SAVE/RESTORE cursor with character set switching.

- [ ] Verify DECSC/DECRC (save/restore cursor) preserves position and attributes
- [ ] Verify character set switching (G0/G1 designate) works across save/restore
- [ ] Add structural assertion: "5 x 4 A's filling the top left of the screen"
- [ ] Add unit test (if missing): `decsc_decrc_preserves_attributes` -- set bold + underline + cursor position + charset, DECSC, change all of them, DECRC, verify all restored
- [ ] Add unit test (if missing): `decsc_decrc_preserves_origin_mode_state` -- DECOM on, DECSC, DECOM off, DECRC, verify DECOM restored to on

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] DECCOLM set/reset clears screen without resizing
- [ ] Wrap-around test (menu 2 screen 01) passes at all sizes
- [ ] Tab stop test (menu 2 screen 02) passes
- [ ] Scroll region tests (menu 2 screens 07-12) pass
- [ ] SGR rendition test (menu 2 screens 13-14) renders correctly
- [ ] Save/restore cursor test (menu 2 screen 15) passes
- [ ] All menu 2 golden PNGs regenerated
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** vttest menu 2 screens produce correct output at 80x24, verified by structural assertions and golden image comparison. DECCOLM screens (03-06) show wrapped content at current width (expected -- no resize per design decision). Menu 2 screens at 97x33 and 120x40 also pass where applicable. Target: 11/15 screens pass fully, 4 partial passes (DECCOLM screens have correct side effects but wrapped visual output).
