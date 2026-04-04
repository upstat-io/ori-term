---
section: "03"
title: "Screen Features & DECCOLM"
status: complete
reviewed: true
goal: "vttest menu 2 screens pass at all sizes — wrap, tabs, column mode, scroll, SGR rendition"
inspired_by:
  - "WezTerm DECCOLM (term/src/terminalstate/mod.rs set_dec_mode)"
  - "xterm DECCOLM (charproc.c RequestResize)"
depends_on: ["02"]
third_party_review:
  status: resolved
  updated: 2026-04-02
sections:
  - id: "03.1"
    title: "DECCOLM Reflow (132-Column Mode)"
    status: complete
  - id: "03.2"
    title: "Wrap-Around Mode (DECAWM)"
    status: complete
  - id: "03.3"
    title: "Tab Stops (HTS/TBC)"
    status: complete
  - id: "03.4"
    title: "Scroll Modes & Origin Mode Screen Tests"
    status: complete
  - id: "03.5"
    title: "SGR Graphic Rendition"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "03.N"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Screen Features & DECCOLM

**Status:** Complete
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

- [x] Implement DECCOLM set: reset scroll region, clear screen (ED 2), home cursor — `apply_deccolm()` at modes.rs:172. Fixed prior implementation that incorrectly resized the grid to 132 cols (corrupting `default_cols`). Removed `default_cols` field entirely.
- [x] Implement DECCOLM reset: same behavior — same `apply_deccolm()` called for both set and reset.
- [x] Add unit test: `deccolm_set_clears_screen` — verifies screen cleared and cursor at (0,0) after CSI ? 3 h
- [x] Add unit test: `deccolm_reset_clears_screen` — verifies same side effects for CSI ? 3 l
- [x] Add unit test: `deccolm_preserves_grid_dimensions` — grid stays 80x24 after set and reset
- [x] Check whether the DECCOLM-allowed flag (CSI ? 40 h, "allow 132 column mode") affects behavior. CSI ? 40 h is not in the VTE parser's NamedPrivateMode enum. Neither WezTerm nor Alacritty gate on it. Our implementation always allows DECCOLM (no gating). vttest does not require it.
- [x] Verify menu 2 screens 03-06 render — content wraps at current width as expected. DECCOLM side effects correct (clear screen, home cursor, reset scroll region). Insta snapshots and golden images regenerated. `vttest_deccolm_preserves_grid_width` structural test passes. Additional tests: `deccolm_resets_scroll_region`, `deccolm_set_then_reset_roundtrip`.

---

## 03.2 Wrap-Around Mode (DECAWM)

**File(s):** `oriterm_core/src/grid/editing/mod.rs`, `oriterm_core/src/term/handler/modes.rs`

Menu 2, screen 01: "three identical lines of *'s completely filling the top of the screen without any empty lines between." This tests DECAWM (auto-wrap).

- [x] Verify wrap-around mode (LINE_WRAP flag) is correctly set/reset via DECSET/DECRST 7 — confirmed at modes.rs:23 (DECSET) and modes.rs:107 (DECRST). LINE_WRAP is on by default.
- [x] Add structural assertion: top 3 rows fully filled with `*` at all widths — verified via insta snapshots at all 3 sizes. vttest screen 01 now correctly shows 3 lines of `*`s (was 4 due to DECAWM-off bug).
- [x] Verify wrap-around with control characters (screen 02: mixing control and print characters) — verified via `decawm_with_control_chars` unit test (BS at wrap boundary).
- [x] Fix any wrap edge cases found — **Major fix: DECAWM off was broken.** Grid always wrapped at EOL regardless of DECAWM. Added DECAWM-off handling in `Term::input()`: (1) snap cursor back from wrap-pending to last column, (2) skip wide chars that don't fit. Matches Alacritty's approach.
- [x] Add unit test: `decawm_wrap_fills_line` — write 81 chars with DECAWM on, verify wrap to next line
- [x] Add unit test: `decawm_off_no_wrap` — write 85 chars with DECAWM off, verify cursor stays on line 0, last column overwritten
- [x] Add unit test: `decawm_with_control_chars` — BS at column boundary, then characters wrap correctly

---

## 03.3 Tab Stops (HTS/TBC)

**File(s):** `oriterm_core/src/grid/mod.rs:47` (tab_stops field), `oriterm_core/src/grid/navigation/mod.rs:132-185` (tab/tab_backward/set_tab_stop/clear_tab_stop), `oriterm_core/src/term/handler/mod.rs:85-90` (put_tab), `oriterm_core/src/term/handler/mod.rs:256-266` (set_horizontal_tabstop, clear_tabs)

Menu 2, screen 02: "These two lines should look the same" — tests tab stop setting and resetting.

Tab stops already have test coverage in `grid/navigation/tests.rs` (set/clear tests) and `grid/tests.rs` (default every-8 tests). Focus on vttest-specific gaps.

- [x] Verify HTS (set tab stop at current column) implementation at `grid/navigation/mod.rs:165` — correct: sets `tab_stops[col] = true` with bounds check
- [x] Verify TBC (clear tab stop) modes: 0 = current, 3 = all at `grid/navigation/mod.rs:173` — correct: `Current` clears single stop, `All` fills false
- [x] Verify CHT (cursor forward tab) at `grid/navigation/mod.rs:132` and CBT (cursor backward tab) at `grid/navigation/mod.rs:148` — both correct, with 10 existing tests
- [x] Add structural assertion: tab-aligned `*` characters match expected positions — verified via vttest menu 2 screen 02 insta snapshot: "These two lines should look the same" → both lines show identical `*` positions
- [x] Fix any tab stop bugs found — no bugs found. Tab implementation is correct.
- [x] Add unit test (if missing): `tab_stop_across_set_and_clear` — added `tab_stop_across_set_and_clear_sequence`: clears all, sets at 6/12/18, clears 12, verifies tab skips cleared stop
- [x] Add unit test (if missing): `tab_stop_at_right_margin` — added `tab_stop_at_right_margin_no_wrap`: cursor at col 79, tab stays at 79

---

## 03.4 Scroll Modes & Origin Mode Screen Tests

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`

Menu 2, screens 07-12: soft/jump scroll in various region configurations, origin mode placement tests.

- [x] Verify smooth scroll mode (DECSCLM) — DECSCLM (CSI ? 4 h) is not in NamedPrivateMode; falls through to `PrivateMode::Unknown(4)` with debug log. This is correct — oriterm always does jump scroll. Debug spam is minimal (one log line per set/reset).
- [x] Add structural assertions for scroll region content placement — screens 07-10 show soft/jump scroll content correctly in insta snapshots
- [x] Verify origin mode screen 11: "This line should be at the bottom of the screen" — confirmed on last visible row at all 3 sizes
- [x] Verify origin mode screen 12: "This line should be at the top of the screen" — confirmed on first visible row at all 3 sizes
- [x] Fix any scroll region + origin mode interaction bugs (depends on Section 02 fixes) — no bugs found. Section 02 fixes resolved all DECOM issues.
- [x] Add structural assertion for screen 11: `text.contains("bottom of the screen")` added to `run_menu2_screen_features`
- [x] Add structural assertion for screen 12: first line contains "top of the screen" assertion added

---

## 03.5 SGR Graphic Rendition

**File(s):** `oriterm_core/src/term/handler/sgr.rs`, `oriterm_core/src/cell.rs`

Menu 2, screens 13-14: graphic rendition test pattern (vanilla, bold, underline, blink, inverse, and combinations).

SGR handling already has 40+ unit tests in `handler/tests.rs` covering bold, italic, underline variants, blink, inverse, hidden, strikethrough, 256-color, truecolor, reset, etc. The vttest screens are primarily a visual verification that the GPU renderer correctly translates `CellFlags` to pixels.

- [x] Verify all SGR attributes render correctly: bold, underline, blink, inverse (negative) — verified via vttest menu 2 screens 13-14 insta snapshots. All attributes present in correct positions.
- [x] Verify combined attributes: bold+underline, bold+blink, underline+blink, etc. — all combinations visible in screen 13 snapshot (4x4 grid of combinations)
- [x] Verify light vs dark background switching (reverse video mode) — screen 13 "Dark background", screen 14 "Light background" both render correctly
- [x] Add golden image test covering the SGR pattern screen — covered by `vttest_golden_menu2_*` tests at all 3 sizes. Golden images regenerated with current output.
- [x] Fix any missing SGR attribute handling (unlikely given existing coverage) — no bugs found. 40+ existing SGR unit tests in handler/tests.rs.

Menu 2, screen 15: SAVE/RESTORE cursor with character set switching.

- [x] Verify DECSC/DECRC (save/restore cursor) preserves position and attributes — verified by existing `esc7_esc8_preserves_sgr_attributes` and `decsc_decrc_saves_and_restores_cursor_position` tests
- [x] Verify character set switching (G0/G1 designate) works across save/restore — verified by vttest screen 15 snapshot showing correct line drawing chars alongside regular chars
- [x] Add structural assertion: "5 x 4 A's filling the top left of the screen" — added assertion in `run_menu2_screen_features` verifying top 4 rows start with "AAAAA"
- [x] Add unit test (if missing): `decsc_decrc_preserves_attributes` — already exists: `esc7_esc8_preserves_sgr_attributes` (bold save/restore) and `esc7_esc8_preserves_wrap_pending` (wrap state)
- [x] Add unit test (if missing): `decsc_decrc_preserves_origin_mode_state` — skipped: DECSC does not save terminal modes (DECOM, DECAWM) in our implementation or in Alacritty/WezTerm. vttest does not test this interaction. DEC spec says to save it but modern terminals don't.

---

## 03.R Third Party Review Findings

- [x] `[TPR-03-001][low]` `plans/vttest-conformance/section-03-screen-features.md:176` — The recorded `./test-all.sh` verification is not reproducible in the current review environment.
  Resolved: Rejected on 2026-04-02. The IPC round-trip test failures are a Codex sandbox limitation (Unix domain sockets require permissions the sandbox doesn't grant). `./test-all.sh` passes locally in the dev environment where IPC tests run correctly. All Section 03-specific tests confirmed passing by the reviewer.

---

## 03.N Completion Checklist

- [x] DECCOLM set/reset clears screen without resizing — `apply_deccolm()` does side effects only, no resize. 5 unit tests + vttest structural test.
- [x] Wrap-around test (menu 2 screen 01) passes at all sizes — 3 lines of `*`s (was 4 due to DECAWM-off bug). Major fix: DECAWM off now correctly prevents wrapping.
- [x] Tab stop test (menu 2 screen 02) passes — two identical lines of tab-aligned `*`s at all sizes
- [x] Scroll region tests (menu 2 screens 07-12) pass — insta snapshots verified at all 3 sizes
- [x] SGR rendition test (menu 2 screens 13-14) renders correctly — all SGR combinations present in snapshots + golden images
- [x] Save/restore cursor test (menu 2 screen 15) passes — 5x4 A's at top-left, charset switching correct
- [x] All menu 2 golden PNGs regenerated — 6 golden images updated via ORITERM_UPDATE_GOLDEN=1
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `/tpr-review` passed — 1 finding (low), rejected as sandbox limitation. No code regressions found.

**Exit Criteria:** vttest menu 2 screens produce correct output at 80x24, verified by structural assertions and golden image comparison. DECCOLM screens (03-06) show wrapped content at current width (expected -- no resize per design decision). Menu 2 screens at 97x33 and 120x40 also pass where applicable. Target: 11/15 screens pass fully, 4 partial passes (DECCOLM screens have correct side effects but wrapped visual output).
