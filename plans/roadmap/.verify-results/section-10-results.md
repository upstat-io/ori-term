# Section 10: Mouse Input & Reporting -- Verification Results

**Verified by:** verify-roadmap agent
**Date:** 2026-03-29
**Section status in plan:** complete
**Verdict:** PASS

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (full)
- `.claude/rules/code-hygiene.md` (full)
- `.claude/rules/impl-hygiene.md` (full)
- `.claude/rules/test-organization.md` (full)
- `.claude/rules/crate-boundaries.md` (full, loaded via system-reminder)
- `plans/roadmap/section-10-mouse-input.md` (full)

## Files Audited

### Production Code
| File | Lines | Under 500? |
|------|-------|-----------|
| `oriterm/src/app/mouse_report/mod.rs` | 310 | Yes |
| `oriterm/src/app/mouse_report/encode.rs` | 267 | Yes |
| `oriterm/src/app/mouse_selection/mod.rs` | 471 | Yes |
| `oriterm/src/app/mouse_selection/helpers.rs` | 154 | Yes |
| `oriterm/src/app/mouse_input.rs` (dispatch) | read lines 300-419 | N/A (shared) |
| `oriterm/src/app/event_loop.rs` (wiring) | read lines 220-297 | N/A (shared) |
| `oriterm_core/src/term/mode/mod.rs` | 134 | Yes |
| `oriterm_core/src/term/handler/modes.rs` | 195 | Yes |
| `oriterm_core/src/term/handler/helpers.rs` (mode map) | lines 64-75 | N/A (shared) |

### Test Code
| File | Test Count |
|------|-----------|
| `oriterm/src/app/mouse_report/tests.rs` | 100 |
| `oriterm/src/app/mouse_selection/tests.rs` | 57 |
| `oriterm_core/src/term/mode/tests.rs` | 14 (mouse-related subset) |
| `oriterm_core/src/term/handler/tests.rs` | ~13 mouse-specific tests |
| `oriterm/src/app/cursor_hide/tests.rs` | 1 (mouse_reporting_prevents_hiding) |

**Total mouse-related tests:** ~185

### Reference Repos Cross-Referenced
- `~/projects/reference_repos/console_repos/alacritty/alacritty/src/input/mod.rs` (lines 490-828)
- `~/projects/reference_repos/console_repos/alacritty/alacritty_terminal/src/term/mod.rs` (DECSET/DECRST dispatch)
- `~/projects/reference_repos/console_repos/wezterm/term/src/terminalstate/mouse.rs` (full file, 364 lines)

---

## Protocol Verification

### 10.2.1 SGR Encoding (DECSET 1006)

**Format:** `\x1b[<{code};{col+1};{line+1}{M|m}`

| Property | ori_term (`encode.rs:115-129`) | Alacritty (`input/mod.rs:607-615`) | WezTerm (`mouse.rs:88-94, 155-161`) | Match? |
|----------|------|------|------|--------|
| Prefix | `\x1b[<` | `\x1b[<` | `\x1b[<` | YES |
| Coords | 1-indexed (`col+1`, `line+1`) | 1-indexed (`point.column+1`, `point.line+1`) | 1-indexed (`event.x+1`, `event.y+1`) | YES |
| Press suffix | `M` | `M` (ElementState::Pressed) | `M` | YES |
| Release suffix | `m` | `m` (ElementState::Released) | `m` | YES |
| Release button code | Preserves actual button code (0/1/2) | Preserves actual button code (`button + mods`) | Preserves actual button code (`release_button`) | YES |
| No coord limit | Confirmed (test `sgr_extreme_coordinates_fit_in_buffer` goes to 65535) | No limit | No limit | YES |

**Tests verified:** `sgr_left_click_origin`, `sgr_release_uses_lowercase_m`, `sgr_coordinates_are_1_indexed`, `sgr_large_coordinates`, `sgr_middle_release_preserves_button_code`, `sgr_right_release_preserves_button_code`, `sgr_all_modifiers_full_round_trip`, `sgr_motion_always_uppercase_m`, `sgr_extreme_coordinates_fit_in_buffer`. All produce byte-correct output.

### 10.2.2 Normal (X10) Encoding (default)

**Format:** `\x1b[M` + 3 bytes (button, col, line)

| Property | ori_term (`encode.rs:198-212`) | Alacritty (`input/mod.rs:573-605`) | WezTerm (`mouse.rs:33-40`) | Match? |
|----------|------|------|------|--------|
| Prefix | `\x1b[M` | `\x1b[M` | `\x1b[M` | YES |
| Button byte | `32 + code` | `32 + button` | `(32 + button) as u8` | YES |
| Coord bytes | `32 + 1 + pos` as u8 | `32 + 1 + pos` as u8 | `encode_coord()` with `32 + 1 + pos` | YES |
| Max coord | 222 (>222 drops) | 223 (>=223 drops) -- same boundary: 0-222 valid | 255 max byte - 33 = 222 | YES |
| Release code | 3 (+ modifiers) | 3 (+ mods, line 567) | `release_button = 3` (line 237) | YES |

**Tests verified:** `normal_left_click` (bytes 32,33,33), `normal_out_of_range_drops_event`, `normal_at_max_encodable_coord` (byte 255 at coord 222), `normal_just_past_max_drops` (223 drops), `normal_release_code_is_3`, `normal_release_with_shift_modifier` (byte 39 = 32+7), `dispatch_normal_boundary_at_max_coord_succeeds`, `dispatch_normal_boundary_one_past_max_drops`.

### 10.2.3 UTF-8 Encoding (DECSET 1005)

**Format:** `\x1b[M` + button byte + UTF-8 encoded col + UTF-8 encoded line

| Property | ori_term (`encode.rs:135-176`) | Alacritty (`input/mod.rs:585-603`) | WezTerm (`mouse.rs:8-31`) | Match? |
|----------|------|------|------|--------|
| Prefix | `\x1b[M` | `\x1b[M` | `\x1b[M` | YES |
| Button byte | `32 + code` (single byte) | `32 + button` | `(32 + button) as u8` | YES |
| Small coords (<95) | Single byte: `32+1+pos` | Single byte: `32+1+pos` (line 595) | `char::from_u32(val).encode_utf8()` | YES (same bytes) |
| Large coords (>=95) | 2-byte: `C0+val/64`, `80+(val&63)` | 2-byte: `C0+pos/64`, `80+(pos&63)` (line 587-588) | `char::from_u32(val).encode_utf8()` | YES (same bytes for valid UTF-8 range) |
| Max coord | 2014 (val 2047 = 0x7FF) | 2015 (>=2015 drops) -- same boundary: 0-2014 | val < 0x800 (same 2047 limit) | YES |
| Boundary (single to 2-byte) | pos=94 single, pos=95 two-byte | column >= 95 uses 2-byte (line 592) | N/A (uses std UTF-8) | YES |

**Tests verified:** `utf8_small_coords_single_byte`, `utf8_boundary_pos_94_single_byte` (val=127, single byte), `utf8_boundary_pos_95_two_bytes` (val=128, two bytes), `utf8_large_coords_multi_byte`, `utf8_out_of_range_returns_zero`, `utf8_max_button_code_with_all_modifiers_in_range`, `utf8_exact_max_coord_2015` (out of range), `utf8_one_below_max_coord_2014` (in range), `utf8_boundary_symmetry`.

### 10.2.4 URXVT Encoding (DECSET 1015)

**Format:** `\x1b[{32+code};{col+1};{line+1}M`

| Property | ori_term (`encode.rs:183-190`) | Alacritty | WezTerm | Match? |
|----------|------|------|------|--------|
| Prefix | `\x1b[` | N/A (Alacritty does not implement URXVT) | N/A | N/A |
| Button value | `32 + code` (decimal) | -- | -- | Standard per xterm docs |
| Coords | 1-indexed decimal | -- | -- | Standard per xterm docs |
| Suffix | Always `M` (no release distinction) | -- | -- | Standard per xterm docs |

Note: Neither Alacritty nor WezTerm implement URXVT (mode 1015) encoding output -- URXVT is considered legacy. ori_term includes it for completeness.

**Tests verified:** `urxvt_left_click_at_origin` (exact bytes `\x1b[32;1;1M`), `urxvt_right_click_at_large_coords`, `urxvt_scroll_up`, `urxvt_has_higher_priority_than_utf8`, `sgr_has_higher_priority_than_urxvt`, `urxvt_with_shift_modifier`, `urxvt_with_ctrl_modifier`, `urxvt_release_uses_m_suffix`.

### 10.2.5 Button Codes

| Button | ori_term | Alacritty | WezTerm | Match? |
|--------|---------|-----------|---------|--------|
| Left | 0 | 0 (`input/mod.rs:623`) | 0 (`mouse.rs:54`) | YES |
| Middle | 1 | 1 (implicit from code+mods) | 1 (`mouse.rs:55`) | YES |
| Right | 2 | 2 (`input/mod.rs:625`) | 2 (`mouse.rs:56`) | YES |
| None (release/buttonless) | 3 | 3 (line 567) | 3 (`mouse.rs:53`) | YES |
| ScrollUp | 64 | 64 (`input/mod.rs:761`) | 64 (`mouse.rs:57`) | YES |
| ScrollDown | 65 | 65 (`input/mod.rs:762`) | 65 (`mouse.rs:58`) | YES |
| Motion offset | +32 | +32 (lines 508-514: codes 32-35) | +32 (`mouse.rs:270`) | YES |

**Tests verified:** `button_code_left_press` through `button_code_scroll_down` (6 tests), `button_code_motion_adds_32`, `button_code_none_motion_is_35`, `all_modifier_combinations_all_buttons` (32-entry matrix).

### 10.2.6 Modifier Bits

| Modifier | ori_term | Alacritty | WezTerm | Match? |
|----------|---------|-----------|---------|--------|
| Shift | +4 | +4 (`input/mod.rs:554`) | +4 (`mouse.rs:64`) | YES |
| Alt | +8 | +8 (`input/mod.rs:557`) | +8 (`mouse.rs:67`) | YES |
| Ctrl | +16 | +16 (`input/mod.rs:560`) | +16 (`mouse.rs:70`) | YES |

**Tests verified:** `modifiers_none`, `modifiers_shift` (4), `modifiers_alt` (8), `modifiers_ctrl` (16), `modifiers_combined` (28), `all_modifier_combinations_all_buttons` (exhaustive 8x4 matrix).

### 10.2.7 X10 Mode (DECSET 9)

| Property | ori_term (`encode.rs:233-267`) | Standard |
|----------|------|---------|
| Press only (no release) | Yes (`x10 && !pressed` returns empty) | Correct |
| No modifier bits | Yes (stripped in `encode_mouse_event`) | Correct |
| Normal encoding format | Yes (falls through to `encode_normal`) | Correct |

**Tests verified:** `x10_mode_press_encodes_normally`, `x10_mode_release_is_suppressed`, `x10_mode_strips_modifiers`, `x10_mode_right_click_press`, `x10_mode_middle_click_press`, `x10_mode_scroll_up_press`, `x10_mode_scroll_down_press`, `x10_mode_out_of_range_drops_event`, `x10_mode_scroll_release_is_suppressed`, `x10_mode_motion_is_suppressed` (documents encoder behavior, motion filtered at App layer).

### 10.2.8 Encoding Priority

| Priority | ori_term (`encode.rs:250-264`) | Reference |
|----------|------|---------|
| 1. SGR (1006) | `if mode.contains(TermMode::MOUSE_SGR)` | SGR takes priority (standard) |
| 2. URXVT (1015) | `else if mode.contains(TermMode::MOUSE_URXVT)` | URXVT next |
| 3. UTF-8 (1005) | `else if mode.contains(TermMode::MOUSE_UTF8)` | UTF-8 next |
| 4. Normal (default) | `else` | Fallback to X10 normal |

**Tests verified:** `dispatch_sgr_when_sgr_mode`, `dispatch_utf8_when_utf8_mode`, `dispatch_normal_when_no_encoding_flags`, `dispatch_sgr_takes_priority_over_utf8`, `sgr_has_higher_priority_than_urxvt`, `urxvt_has_higher_priority_than_utf8`.

### 10.2.9 Tracking Mode DECSET Numbers

| Mode | DECSET | ori_term VTE mapping | ori_term TermMode flag | Alacritty |
|------|--------|---------------------|----------------------|-----------|
| X10 | 9 | `X10Mouse` (`vte/types.rs:179`) | `MOUSE_X10` | N/A (Alacritty skips X10) |
| Click | 1000 | `ReportMouseClicks` (`vte/types.rs:186`) | `MOUSE_REPORT_CLICK` | Same |
| Drag | 1002 | `ReportCellMouseMotion` (`vte/types.rs:187`) | `MOUSE_DRAG` | Same |
| All motion | 1003 | `ReportAllMouseMotion` (`vte/types.rs:188`) | `MOUSE_MOTION` | Same |
| UTF-8 | 1005 | `Utf8Mouse` (`vte/types.rs:190`) | `MOUSE_UTF8` | Same |
| SGR | 1006 | `SgrMouse` (`vte/types.rs:191`) | `MOUSE_SGR` | Same |
| Alternate scroll | 1007 | `AlternateScroll` (`vte/types.rs:192`) | `ALTERNATE_SCROLL` | Same |
| URXVT | 1015 | `UrxvtMouse` (`vte/types.rs:193`) | `MOUSE_URXVT` | N/A |

### 10.2.10 Mutual Exclusion on DECSET

Tracking modes are mutually exclusive: setting one clears all others via `self.mode.remove(TermMode::ANY_MOUSE)` then inserting the new flag (`modes.rs:31-48`). Encoding modes are mutually exclusive: `self.mode.remove(TermMode::ANY_MOUSE_ENCODING)` then insert (`modes.rs:52-61`).

**Tests verified in oriterm_core handler tests:**
- `mouse_mode_1003_clears_1000_and_1002`
- `mouse_mode_1002_clears_1000_and_1003`
- `mouse_encoding_1006_clears_1005_and_1015`
- `mouse_encoding_1015_clears_1005_and_1006`
- `mouse_encoding_1005_clears_when_setting_1015`
- `decrst_mouse_tracking_does_not_reactivate_previous` (no auto-reactivation on DECRST)
- `decrst_encoding_reverts_to_no_encoding` (DECRST clears to Normal)
- `decrst_1000_preserves_active_1003` (targeted clear)
- `decrst_9_preserves_active_1000` (targeted clear)
- `ris_clears_all_mouse_modes` (full reset)

### 10.2.11 Alternate Scroll (DECSET 1007)

| Property | ori_term (`mouse_report/mod.rs:180-193`) | Alacritty (`input/mod.rs:795-828`) |
|----------|------|------|
| Condition | `ALT_SCREEN \| ALTERNATE_SCROLL` | `ALT_SCREEN \| ALTERNATE_SCROLL` |
| Scroll up | `\x1bOA` (SS3 A) | `0x1b, 'O', 'A'` |
| Scroll down | `\x1bOB` (SS3 B) | `0x1b, 'O', 'B'` |
| Default state | On (`TermMode::default()` includes `ALTERNATE_SCROLL`) | On (same) |

**Tests verified:** `TermMode::default()` includes `ALTERNATE_SCROLL` (line 98 of `mode/mod.rs`), `decrst_mouse_tracking_does_not_reactivate_previous` tests 1007 implicitly.

---

## 10.1 Mouse Selection State Machine

### Implementation Audit

- **MouseState** (`mouse_selection/mod.rs:71-148`): tracks buttons via compact `ButtonsDown` bitfield, touchdown position, drag state, click detector, cursor position, last reported cell.
- **handle_press** (`mod.rs:223-288`): click detection, shift-extend, alt-block, word/line boundary snapping for multi-click. Pure logic extracted via `classify_press`.
- **handle_drag** (`mod.rs:416-459`): threshold check (max(cell_width/4, 2px)), mode-aware endpoint snapping, auto-scroll when outside grid.
- **handle_release** (`mod.rs:464-468`): clears drag state and touchdown.
- **pixel_to_cell** (`mod.rs:172-197`): bounds-checked, handles offset origins, returns None outside grid.
- **pixel_to_side** (`mod.rs:200-212`): left/right half detection via `rem_euclid`.
- **classify_press** (`mod.rs:332-390`): pure function, handles single/double/triple click, shift-extend, alt-block toggle.
- **helpers.rs** (`helpers.rs:1-154`): `compute_drag_endpoint` (word/line snap), `auto_scroll_delta`, `compute_auto_scroll_endpoint`.

### Test Coverage

57 tests in `mouse_selection/tests.rs` covering:
- pixel_to_cell: origin, mid-grid, last cell, negative coords, no bounds, offset origin, boundary edges, fractional widths
- pixel_to_side: left/right half, midpoint, second cell, offset origin, zero width
- MouseState: initial state, cursor tracking, dragging predicate, button tracking (L/M/R), multi-button, other button noop
- classify_press: double-click word, triple-click line, alt-block toggle, shift-extend, edge cases (missing bounds -> char fallback)
- Drag threshold: constant check, distance computation, button guard
- Motion deduplication: initial None, set/get, clear, persist across button changes
- Cross-module: SGR/Normal origin release exact output

---

## Hygiene Checks

### Code Hygiene
- All 4 source files have `//!` module docs.
- All `pub` items have `///` doc comments.
- No `unwrap()` in production code (zero matches across all 4 files).
- No `String`, `Vec`, `Box`, `format!` in encoding path -- zero-allocation `MouseReportBuf` stack buffer.
- No dead code, no commented-out code, no `println!` debugging.
- No `#[allow(clippy)]` without reason. One `#[expect(clippy::too_many_arguments)]` with justification in `handle_press` and `compute_drag_endpoint`.
- Import groups: std, external, crate -- correctly ordered.
- All files under 500-line limit (max: `mouse_selection/mod.rs` at 471).

### Test Organization
- Sibling `tests.rs` pattern followed for both `mouse_report/` and `mouse_selection/`.
- `#[cfg(test)] mod tests;` at bottom of each `mod.rs`.
- No inline test modules.
- Test files use `super::` imports.
- No `mod tests { }` wrapper in test files.

### Implementation Hygiene
- **Zero allocation in hot path:** `MouseReportBuf` is `[u8; 32]` on stack. Encoding uses `Cursor<&mut [u8]>`.
- **One-way data flow:** encode functions are pure (take event + mode, return bytes). App dispatch calls encode then writes to PTY.
- **No layer bleeding:** encoding module does not touch grid, PTY, or App state.
- **Event flow:** all mouse events route through `event_loop.rs` match arms to dedicated handlers.
- **Mutual exclusion on DECSET:** tracking modes clear `ANY_MOUSE` before inserting; encoding modes clear `ANY_MOUSE_ENCODING` before inserting.
- **Shift bypass:** `should_report_mouse` checks `!shift && ANY_MOUSE` -- tested in `handle_mouse_input` dispatch.
- **Motion deduplication:** `last_reported_cell` on MouseState, checked before encoding in `report_mouse_motion`.
- **Crate boundaries respected:** encoding logic in `oriterm` (binary crate), TermMode flags in `oriterm_core` (library). Clean boundary.

---

## Test Execution

```
cargo test -p oriterm -- mouse_report
  101 passed; 0 failed (includes 1 cursor_hide integration test)

cargo test -p oriterm -- mouse_selection
  57 passed; 0 failed

cargo test -p oriterm_core -- mouse
  12 passed; 0 failed
```

All 170 tests pass within timeout.

---

## Issues Found

**None.** All protocol bytes match reference implementations. All plan items are implemented and tested. Code hygiene, test organization, and implementation hygiene rules are all satisfied.

---

## Plan Accuracy

The section plan accurately describes the implementation. Minor deviation noted:
- Plan says "31 tests in `mouse_report/tests.rs`" but actual count is **100 tests**. The plan was written before subsequent test expansion (URXVT, X10, motion, multi-button, boundary, modifier matrix tests were added later). This is a positive deviation -- test coverage significantly exceeds the plan's description.
- Plan says file is `oriterm/src/app/mouse_report/mod.rs` for encoding. Encoding was actually extracted to a separate `encode.rs` submodule for cleanliness. Plan mentions this file in the "Files" section but the description could be more explicit.

**Plan status: CORRECT. Section is genuinely complete.**
