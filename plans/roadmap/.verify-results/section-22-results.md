# Section 22: Terminal Modes -- Verification Results

**Verified by:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-29
**Status:** MOSTLY COMPLETE -- one missing implementation (DECALN), one missing test (sync output)

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (full)
- `.claude/rules/code-hygiene.md`, `.claude/rules/impl-hygiene.md`, `.claude/rules/test-organization.md` (3 rules files -- 4th is `crate-boundaries.md`, loaded via system reminder)
- `plans/roadmap/section-22-terminal-modes.md` (full)

## Reference Repos Cross-Referenced

- **Alacritty** (`~/projects/reference_repos/console_repos/alacritty/alacritty_terminal/src/term/mod.rs`)
- **WezTerm** (`~/projects/reference_repos/console_repos/wezterm/term/src/terminalstate/mod.rs`)

---

## 22.1 Mouse Reporting Modes -- VERIFIED

### Implementation

**Files:** `oriterm/src/app/mouse_report/mod.rs` (310 lines), `oriterm/src/app/mouse_report/encode.rs` (267 lines)

**Mode flags** (`oriterm_core/src/term/mode/mod.rs:69-84`):
- `MOUSE_X10` (bit 26) -- mode 9
- `MOUSE_REPORT_CLICK` (bit 3) -- mode 1000
- `MOUSE_DRAG` (bit 4) -- mode 1002
- `MOUSE_MOTION` (bit 5) -- mode 1003
- `ANY_MOUSE` composite = X10 | REPORT_CLICK | DRAG | MOTION

**Mouse encoding flags** (`oriterm_core/src/term/mode/mod.rs:30,25,68`):
- `MOUSE_SGR` (bit 6) -- mode 1006
- `MOUSE_UTF8` (bit 7) -- mode 1005
- `MOUSE_URXVT` (bit 25) -- mode 1015
- `ANY_MOUSE_ENCODING` composite = SGR | UTF8 | URXVT

**Mutual exclusion** (`oriterm_core/src/term/handler/modes.rs:31-62`): On DECSET, `self.mode.remove(TermMode::ANY_MOUSE)` before insert. Same for encodings: `self.mode.remove(TermMode::ANY_MOUSE_ENCODING)` before insert. This is cleaner than Alacritty, which does pairwise removal (SGR removes UTF8 only, UTF8 removes SGR only) because Alacritty does not support URXVT at all.

**Cross-reference:**
- **Alacritty** (`term/mod.rs:1953-1966`): uses `MOUSE_MODE` composite (REPORT_CLICK | DRAG | MOTION) to remove before insert. Does NOT include X10 mode 9 in `MOUSE_MODE`. Does NOT support URXVT encoding (mode 1015). ori_term includes X10 in `ANY_MOUSE` -- correct since X10 is mutually exclusive with other tracking modes.
- **WezTerm** (`terminalstate/mod.rs:1759-1802`): uses separate boolean fields (`mouse_tracking`, `button_event_mouse`, `any_event_mouse`), does NOT enforce mutual exclusion between them. ori_term's bitflag approach with composite removal is a better design.

**Button encoding** (`encode.rs:77-91`): Left=0, Middle=1, Right=2, None=3, ScrollUp=64, ScrollDown=65. Motion adds 32. Matches xterm spec.

**Modifier encoding** (`encode.rs:96-108`): Shift=+4, Alt=+8, Ctrl=+16. Matches xterm spec and Alacritty (`input/mod.rs:551-561`).

**Shift-bypass** (`mouse_report/mod.rs:31-33`): `!self.modifiers.shift_key() && mode.intersects(TermMode::ANY_MOUSE)`. Matches Alacritty (`input/mod.rs:619`).

**Motion dedup** (`mouse_report/mod.rs:98-99`): `self.mouse.last_reported_cell() == Some((col, line))` check before sending. Correct.

**Alternate scroll** (`mouse_report/mod.rs:181-192`): Sends `\x1bOA` / `\x1bOB` when `ALT_SCREEN | ALTERNATE_SCROLL`. Uses application cursor keys format (ESC O A/B). Matches Alacritty (`input/mod.rs:799+`).

**Encoding priority** (`encode.rs:250-264`): SGR > URXVT > UTF8 > Normal. Correct per xterm documentation. Alacritty only has SGR > Normal (with UTF8 variant in the Normal path).

**X10 mode** (`encode.rs:238-248`): Strips modifiers, suppresses releases. Correct per xterm spec.

### Tests

**101 tests** in `oriterm/src/app/mouse_report/tests.rs`, all passing.

Key coverage:
- Button codes for all buttons including None and scroll (8 tests)
- Modifier encoding: shift, alt, ctrl, combined (5 tests)
- SGR encoding: origin, coordinates, release (lowercase m), large coords, modifiers, motion (12+ tests)
- Normal (X10) encoding: left click, out-of-range drops, max encodable coord, release code 3 (8+ tests)
- UTF-8 encoding: single byte, boundary 94/95, multi-byte, out-of-range (8+ tests)
- URXVT encoding: click, large coords, scroll, priority vs UTF-8 and SGR, modifiers (8+ tests)
- X10 mode: press encodes, release suppressed, motion suppressed, no modifiers, all buttons, out-of-range (10 tests)
- Encoding priority dispatch: SGR wins over UTF8, URXVT wins over UTF8, SGR wins over URXVT (3 tests)
- Motion always reports as 'M' (pressed), scroll release behavior (2 tests)
- Buffer overflow safety: SGR with 65535 coordinates fits in 32 bytes (1 test)

**Mutual exclusion tests** in `oriterm_core/src/term/handler/tests.rs`:
- `mouse_mode_1003_clears_1000_and_1002` (line 4394)
- `mouse_mode_1002_clears_1000_and_1003` (line 4408)
- `mouse_encoding_1006_clears_1005_and_1015` (line 4420)
- `mouse_encoding_1015_clears_1005_and_1006` (line 4434)
- `mouse_encoding_1005_clears_when_setting_1015` (line 4831)
- `decrst_mouse_tracking_does_not_reactivate_previous` (line 4682)
- `decrst_1000_preserves_active_1003` (line 4883)
- `decrst_1002_preserves_active_1003` (line 4899)
- `decrst_9_preserves_active_1000` (line 4910)
- `decrst_encoding_reverts_to_no_encoding` (line 4847)
- `ris_clears_all_mouse_modes` (line 4774)

**Verdict: PASS** -- Thorough implementation and testing. Exceeds Alacritty (which lacks URXVT and X10 mode 9).

---

## 22.2 Cursor Styles -- VERIFIED

### Implementation

**Files:** `oriterm_core/src/grid/cursor/mod.rs` (100 lines), `oriterm_core/src/term/handler/dcs.rs`

**CursorShape enum** (`cursor/mod.rs:15-22`): Block, Underline, Bar, HollowBlock, Hidden. Default is Block. Conversion from `vte::ansi::CursorShape` handles all 5 variants.

**DECSCUSR dispatch** (`handler/dcs.rs:18-28`): `dcs_set_cursor_style` handles `Some(style)` (set shape + blinking) and `None` (reset to default Block + blinking on). Fires `Event::CursorBlinkingChange`.

**Cross-reference:**
- **Alacritty** (`term/mod.rs:1987-1991`): Uses `self.cursor_style.get_or_insert()` pattern for blinking. ori_term's approach is simpler (direct field set + mode flag).

### Tests

**9 DECSCUSR tests** in handler/tests.rs, all passing:
- `decscusr_0_resets_to_default` (line 2984)
- `decscusr_1_sets_blinking_block` (line 2918)
- `decscusr_2_sets_steady_block` (line 2930)
- `decscusr_3_sets_blinking_underline` (line 2966)
- `decscusr_4_sets_steady_underline` (line 2975)
- `decscusr_5_sets_blinking_bar` (line 2942)
- `decscusr_6_sets_steady_bar` (line 2954)
- `decscusr_fires_cursor_blinking_change_event` (line 2996)
- `decscusr_set_same_shape_twice_is_idempotent` (line 3210)

**6 cursor unit tests** in `cursor/tests.rs`: default position, set_line/col, default shape, template default, clone preservation, shape variant distinctness.

**Verdict: PASS** -- All DECSCUSR values 0-6 tested. Save/restore cursor tested separately in grid navigation tests (5 tests).

---

## 22.3 Focus Events -- VERIFIED

### Implementation

**File:** `oriterm/src/app/event_loop_helpers/mod.rs:104-116`

`send_focus_event()` checks `TermMode::FOCUS_IN_OUT`, sends `\x1b[I` (focus in) or `\x1b[O` (focus out) to the active pane's PTY.

`flush_pending_focus_out()` suppresses false focus-out when focus moves to a child dialog.

Event loop integration in `oriterm/src/app/event_loop.rs:119-142`: `WindowEvent::Focused(true)` calls `send_focus_event(true)`, `Focused(false)` defers to `pending_focus_out`.

**Cross-reference:**
- **Alacritty** (`input/mod.rs`): Identical pattern -- gate on `FOCUS_IN_OUT` mode flag, send `\x1b[I` / `\x1b[O`.

### Tests

**3 focus tests** in `oriterm/src/app/tests.rs`:
- `focus_in_out_mode_bit_pattern` (line 179)
- `focus_in_out_not_set_by_default` (line 188)
- `focus_in_out_combined_with_other_modes` (line 195)

**1 handler test:** DECSET/DECRST for mode 1004 verified in `decset_decrst_flag_sync` (line 5131, covers `ReportFocusInOut` variant).

**Verdict: PASS** -- Mode flag correctly gated. Integration with winit's Focused event verified in event loop code.

---

## 22.4 Synchronized Output -- VERIFIED (minor gap)

### Implementation

Mode 2026 is handled as a flag in `TermMode::SYNC_UPDATE` (bit 14). The DECSET/DECRST handlers set/clear the flag (`modes.rs:84,149`). vte 0.15 handles BSU/ESU internally -- the flag is informational.

### Tests

**Gap:** The plan claims `[x] Verify that vte processes BSU/ESU sequences without error` but there is no explicit test for this. The `SyncUpdate` mode is only tested as part of the `decset_decrst_flag_sync` comprehensive test (line 5158) which verifies the flag can be set/cleared but does NOT verify vte's BSU/ESU buffering behavior.

**Verdict: PASS with MINOR GAP** -- Flag handling is correct. Missing explicit vte BSU/ESU integration test as claimed by the plan. Low severity since vte handles this internally and the flag mechanism is verified.

---

## 22.5 Hyperlinks -- VERIFIED

### Implementation

**OSC 8** (`handler/osc.rs:163-169`): Sets hyperlink on cursor template via `set_hyperlink()`.

**Cell storage** (`cell/mod.rs:59-65,217-236`): `CellExtra.hyperlink: Option<Hyperlink>`. `Hyperlink` struct has `uri` and `id`. Set/clear via `Cell::set_hyperlink()`. Extra is dropped when empty (zero overhead for normal cells).

**URL detection** (`oriterm/src/url_detect/mod.rs`, 413 lines): Regex-based detection of http/https/ftp/file URLs. Snapshot-based detection for production, grid-based for tests. Caches by logical line, invalidates on content change. Handles balanced parentheses (Wikipedia URLs), trailing punctuation trimming, wrapped URLs across multiple rows.

### Tests

**6 OSC 8 tests** in handler/tests.rs:
- `osc8_sets_hyperlink` (line 1751)
- `osc8_clear_hyperlink` (line 1780)
- `osc8_hyperlink_survives_sgr_reset` (line 2339)
- `osc8_hyperlink_written_to_cells` (line 2359)
- `osc8_with_id_parameter`, `osc8_uri_with_semicolons`

**2 cell hyperlink tests** in cell/tests.rs: `extra_created_for_hyperlink`, `hyperlink_display`.

**23 URL detection tests** in url_detect/tests.rs: Simple URL, multiple URLs, wrapped URL, scrollback, cache invalidation, balanced parens, nested parens, schemes (ftp, file), query strings, wide chars, three-row spanning, no false positives.

**Verdict: PASS** -- Comprehensive hyperlink support with both explicit (OSC 8) and implicit (regex) detection.

---

## 22.6 Comprehensive Mode Table -- VERIFIED (one missing implementation)

### Private Modes Implemented

All modes in the table are registered as `TermMode` flags and handled in `apply_decset`/`apply_decrst` (`handler/modes.rs`):

| Mode | Flag | DECSET line | DECRST line | Status |
|------|------|-------------|-------------|--------|
| 1 (DECCKM) | APP_CURSOR | 19 | 103 | OK |
| 6 (DECOM) | ORIGIN | 21-24 | 105-107 | OK |
| 7 (DECAWM) | LINE_WRAP | 24 | 108 | OK |
| 9 (X10 Mouse) | MOUSE_X10 | 31-34 | 115-117 | OK |
| 12 (ATT610) | CURSOR_BLINKING | 25-28 | 109-112 | OK |
| 25 (DECTCEM) | SHOW_CURSOR | 29 | 113 | OK |
| 45 (Reverse Wrap) | REVERSE_WRAP | 64-66 | 135-137 | OK |
| 47 (Alt Screen) | ALT_SCREEN | 67-70 | 138-141 | OK |
| 1000 (Normal Mouse) | MOUSE_REPORT_CLICK | 35-39 | 118-120 | OK |
| 1002 (Button Mouse) | MOUSE_DRAG | 40-44 | 121-123 | OK |
| 1003 (Any Mouse) | MOUSE_MOTION | 45-49 | 124-127 | OK |
| 1004 (Focus Events) | FOCUS_IN_OUT | 50 | 130 | OK |
| 1005 (UTF-8 Mouse) | MOUSE_UTF8 | 51-53 | 131 | OK |
| 1006 (SGR Mouse) | MOUSE_SGR | 54-57 | 132 | OK |
| 1007 (Alt Scroll) | ALTERNATE_SCROLL | 85-87 | 151-153 | OK |
| 1015 (URXVT Mouse) | MOUSE_URXVT | 58-62 | 133 | OK |
| 1042 (Urgency) | URGENCY_HINTS | 63 | 134 | OK |
| 1047 (Alt Screen Opt) | ALT_SCREEN | 73-76 | 138 | OK |
| 1048 (Save Cursor) | (grid op) | 77 | 143 | OK |
| 1049 (Swap+Restore) | ALT_SCREEN | 78-82 | 144-148 | OK |
| 2004 (Bracketed Paste) | BRACKETED_PASTE | 83 | 149 | OK |
| 2026 (Sync Output) | SYNC_UPDATE | 84 | 150 | OK |

### Standard Modes

- IRM (mode 4): `TermMode::INSERT` -- handler/mod.rs:282,292 -- OK
- LNM (mode 20): `TermMode::LINE_FEED_NEW_LINE` -- handler/mod.rs:283-285,293-295 -- OK

### Application Keypad

- DECKPAM (`ESC =`): handler/mod.rs:337-339 -- OK
- DECKPNM (`ESC >`): handler/mod.rs:340-342 -- OK

### DECALN -- MISSING IMPLEMENTATION

**FINDING:** The plan marks `[x] ESC # 8 (DECALN): fill entire screen with 'E' characters` but DECALN is **not implemented**. The VTE handler trait provides `fn decaln(&mut self) {}` with a default no-op (`crates/vte/src/ansi/handler.rs:226`), and ori_term does not override it. No `decaln` method exists anywhere in `oriterm_core/src`.

**Reference:** Alacritty implements DECALN (`term/mod.rs:1141-1153`): iterates all cells, sets each to `Cell::default()` with `c = 'E'`, then marks fully damaged.

**Impact:** Low. DECALN is primarily a diagnostic tool used during terminal testing. It is not used by normal applications (tmux, vim, htop, etc.). However, it is marked as complete in the plan when it is not.

### Mode Interactions

- Mouse mutual exclusion: VERIFIED (see 22.1)
- Encoding mutual exclusion: VERIFIED (see 22.1)
- Alt screen save/restore (1049): VERIFIED (`alt_screen_preserves_and_restores_cursor_position`, line 3876)
- Mode 47 no cursor save: VERIFIED (`mode_47_swaps_without_cursor_save`, line 4448)
- Mode 1047 clear alt on enter: VERIFIED (`mode_1047_clears_alt_on_enter`, line 4474)
- Mode 1048 save/restore standalone: VERIFIED (`mode_1048_saves_and_restores_cursor`, line 4499)
- Double-enter no-op: VERIFIED (`mode_47_double_enter_is_noop`, line 4749)
- Cross-mode enter/exit: VERIFIED (`mode_1049_enter_then_47_exit`, line 4792)

### XTSAVE/XTRESTORE

Implementation in `handler/modes.rs:167-194`: Uses `HashMap<u16, bool>` (single save per mode). Cleared on RIS (`handler/esc.rs:51`).

**Cross-reference:**
- **Alacritty**: Does NOT implement XTSAVE/XTRESTORE.
- **WezTerm** (`terminalstate/mod.rs:1909-1912`): Logs as unimplemented.
- ori_term is ahead of both reference repos here.

### Reverse Wraparound (Mode 45)

Implementation in `handler/helpers.rs:102-122`: Checks col 0, previous line exists, WRAP flag set. Correctly implemented.

### Tests

**35+ Section 22-specific tests** in handler/tests.rs covering:
- Mouse mode mutual exclusion (5 tests)
- Mouse encoding mutual exclusion (4 tests)
- DECRST targeted clear preserves other modes (3 tests)
- Legacy alt screen (modes 47, 1047, 1048) (4 tests)
- Reverse wraparound (3 tests + boundary at line 0)
- XTSAVE/XTRESTORE (5 tests: save/restore, no-save no-op, multiple modes, RIS clears, overwrite, unknown mode)
- Alt screen interactions (scroll region, double-enter, cross-mode enter/exit)
- Unknown mode handling (2 tests)
- Comprehensive flag sync (1 test covering all 20 NamedPrivateMode variants)
- RIS clears all mouse modes + saved modes (2 tests)
- DECRST encoding reverts (3 tests: SGR, UTF-8, URXVT)
- DECRPM reporting (3 tests)

**Verdict: PASS with ONE FINDING** -- DECALN (`ESC # 8`) is marked complete but not implemented.

---

## 22.7 Image Protocol -- VERIFIED (deferred)

Correctly deferred to Section 39. The plan notes this clearly.

**Verdict: PASS** -- Section 39 handles this.

---

## 22.8 Section Completion -- VERIFIED (with findings)

All completion checklist items are accurate except:
- DECALN is marked complete but is a no-op (see 22.6 finding)
- Sync output test is claimed but only the flag test exists (see 22.4 gap)

---

## Code Hygiene

### File Sizes (all under 500-line limit)
- `term/mode/mod.rs`: 134 lines
- `term/handler/modes.rs`: 195 lines
- `app/mouse_report/mod.rs`: 310 lines
- `app/mouse_report/encode.rs`: 267 lines
- `term/handler/helpers.rs`: 244 lines
- `grid/cursor/mod.rs`: 100 lines
- `url_detect/mod.rs`: 413 lines

### Test Organization
All test files follow the sibling `tests.rs` pattern with `#[cfg(test)] mod tests;` at the bottom of source files. No inline test modules found.

### Import Organization
Standard 3-group pattern (std, external, crate) used consistently across all files.

### Documentation
All public items have `///` doc comments. Module docs (`//!`) present on all files.

### No unwrap() in library code
Mouse encoding uses `Result` handling (returns 0 on overflow). No `unwrap()` in hot paths.

### Zero-allocation encoding
`MouseReportBuf` uses a stack-allocated `[u8; 32]` buffer with `std::io::Cursor` -- no heap allocation in the mouse encoding hot path.

---

## Test Summary

| Component | Tests | Status |
|-----------|-------|--------|
| TermMode flags | 14 | ALL PASS |
| Mouse encoding | 101 | ALL PASS |
| VTE handler (modes/mouse/alt) | 35+ (within 320 total) | ALL PASS |
| DECSCUSR cursor styles | 9 | ALL PASS |
| Cursor unit tests | 6 | ALL PASS |
| OSC 8 hyperlinks | 6 | ALL PASS |
| Cell hyperlink | 2 | ALL PASS |
| URL detection | 23 | ALL PASS |
| Focus event mode | 3 | ALL PASS |
| App tests (focus) | 20 | ALL PASS |

**Total Section 22-related tests: ~220+**

---

## Findings

### FINDING 1: DECALN Not Implemented (Severity: LOW)

**Plan claim:** `[x] ESC # 8 (DECALN): fill entire screen with 'E' characters`
**Reality:** The VTE handler trait's `fn decaln()` has a default no-op, and ori_term does not override it. No implementation exists in `oriterm_core`.
**Reference:** Alacritty implements this at `term/mod.rs:1141-1153`.
**Impact:** Low -- DECALN is a diagnostic tool for testing screen alignment. No normal TUI applications use it. However, it is incorrectly marked as complete.
**Fix:** Implement `fn decaln()` on `Term<T>` to iterate visible rows, set each cell to 'E', and mark dirty. Also add a test feeding `\x1b#8` and verifying all cells contain 'E'.

### FINDING 2: Missing Synchronized Output Test (Severity: TRIVIAL)

**Plan claim:** `[x] Verify that vte processes BSU/ESU sequences without error`
**Reality:** No test feeds BSU (`\x1b[?2026h`) / ESU (`\x1b[?2026l`) through the VTE processor and verifies behavior. The `SyncUpdate` flag is only tested via the comprehensive `decset_decrst_flag_sync` test.
**Impact:** Trivial -- vte handles this internally and the flag mechanism works. The test would be redundant but the plan claims it exists.

---

## Overall Verdict

**Section 22 is SUBSTANTIALLY COMPLETE.** The terminal mode system is well-architected, thoroughly tested, and exceeds both Alacritty and WezTerm in coverage (URXVT encoding, X10 mode, XTSAVE/XTRESTORE). The only substantive gap is the unimplemented DECALN screen alignment test, which is a low-severity diagnostic feature incorrectly marked as complete.
