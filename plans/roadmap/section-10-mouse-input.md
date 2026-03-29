---
section: 10
title: Mouse Input & Reporting
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 3
goal: Mouse reporting for terminal apps + mouse selection state machine
sections:
  - id: "10.1"
    title: Mouse Selection State Machine
    status: complete
  - id: "10.2"
    title: Mouse Reporting
    status: complete
  - id: "10.3"
    title: Section Completion
    status: complete
---

# Section 10: Mouse Input & Reporting

**Status:** Complete
**Goal:** Implement the mouse input layer: a state machine for tracking selection gestures, and mouse event reporting to the PTY for terminal applications that request it (vim, tmux, htop, etc.). Mouse reporting supports all three encoding formats (X10 normal, UTF-8, SGR) and all tracking modes.

**Crate:** `oriterm` (binary)
**Dependencies:** `winit` (mouse events), `oriterm_core` (TermMode, Grid)
**Reference:** `_old/src/app/mouse_report.rs`, `_old/src/app/mouse_selection.rs`, `_old/src/app/input_mouse.rs`

**Prerequisite:** Section 07 complete (Selection model and rendering). Section 03 complete (PTY send channel). Section 02 complete (TermMode flags for mouse mode detection).

---

## 10.1 Mouse Selection State Machine (verified 2026-03-29)

Centralized state machine for tracking mouse gesture state. Coordinates between selection creation (Section 08) and mouse reporting (10.2), ensuring clean separation of concerns.

**File:** `oriterm/src/app/mouse_selection/mod.rs`

**Implementation note:** The existing architecture (free functions + `MouseState` + `Tab`-owned selection) is cleaner than the `SelectionAction`/`SelectionState` enum described in the original spec. All functionality is covered.

- [x] `MouseState` struct (tracks left_down, touchdown, drag_active, click_detector, cursor_pos, last_reported_cell) (verified 2026-03-29)
- [x] `handle_press` — click detection, shift-extend, word/line boundary computation (verified 2026-03-29)
- [x] `handle_drag` — threshold check, endpoint update with mode-aware snapping (verified 2026-03-29)
- [x] `handle_release` — clears drag state (verified 2026-03-29)
- [x] `pixel_to_cell` / `pixel_to_side` — coordinate conversion (verified 2026-03-29)
- [x] `classify_press` — pure logic for determining selection action (verified 2026-03-29)
- [x] `redirect_spacer` — wide char spacer handling (verified 2026-03-29)
- [x] `handle_auto_scroll` — viewport scrolling when dragging outside grid (verified 2026-03-29)
- [x] Comprehensive tests in `mouse_selection/tests.rs` (verified 2026-03-29 -- 57 tests, all pass)

---

## 10.2 Mouse Reporting (verified 2026-03-29)

Encode mouse events and send to PTY when terminal applications request mouse tracking. Supports all three encoding formats and all tracking modes.

**Files:**
- `oriterm/src/app/mouse_report/mod.rs` — encoding functions + `impl App` dispatch
- `oriterm/src/app/mouse_report/encode.rs` — encoding implementation (extracted submodule)
- `oriterm/src/app/mouse_report/tests.rs` — 100 encoding + dispatch tests
- `oriterm_core/src/term/mode/mod.rs` — `ALTERNATE_SCROLL` flag added
- `oriterm_core/src/term/handler/modes.rs` — DECSET/DECRST wired for AlternateScroll
- `oriterm_core/src/term/handler/helpers.rs` — mode flag mapping wired

- [x] **Mouse tracking modes** (checked via TermMode flags): (verified 2026-03-29)
  - [x] `MOUSE_REPORT_CLICK` (DECSET 1000) — report button press/release only
  - [x] `MOUSE_DRAG` (DECSET 1002) — report press/release + drag motion (button held)
  - [x] `MOUSE_MOTION` (DECSET 1003) — report all motion (even without button)
  - [x] No flag set: mouse events are local-only (selection, no PTY reporting)
- [x] **Mouse encoding modes** (checked via TermMode flags): (verified 2026-03-29)
  - [x] `MOUSE_SGR` (DECSET 1006) — preferred: `ESC[<code;col;row M/m`
  - [x] `MOUSE_UTF8` (DECSET 1005) — coordinates UTF-8 encoded
  - [x] Default (X10 normal) — `ESC[M cb cx cy` (coordinates limited to 222)
- [x] **Button encoding**: 0=left, 1=middle, 2=right, 3=release(normal), 64=scroll up, 65=scroll down, +32=motion (verified 2026-03-29)
- [x] **Modifier bits**: +4 Shift, +8 Alt, +16 Ctrl (verified 2026-03-29)
- [x] **SGR encoding**: `\x1b[<{code};{col+1};{row+1}{M|m}` — stack-allocated, no coord limit (verified 2026-03-29 -- matches Alacritty + WezTerm byte-for-byte)
- [x] **UTF-8 encoding**: `\x1b[M` + UTF-8 values, custom 2-byte for coords >= 95 (verified 2026-03-29 -- matches Alacritty + WezTerm)
- [x] **Normal (X10) encoding**: `\x1b[M` + 3 bytes, coords clamped to 222 (verified 2026-03-29 -- matches Alacritty + WezTerm)
- [x] **URXVT encoding** (DECSET 1015): `\x1b[{32+code};{col+1};{line+1}M` — legacy, included for completeness (verified 2026-03-29)
- [x] **X10 mode** (DECSET 9): press-only, no modifiers, no motion (verified 2026-03-29)
- [x] **Mouse mode priority over selection**: when ANY_MOUSE active, events go to PTY (verified 2026-03-29)
- [x] **Shift bypasses mouse reporting**: Shift+click always does local selection (verified 2026-03-29)
- [x] **Motion deduplication**: `last_reported_cell` on MouseState, only report on cell change (verified 2026-03-29)
- [x] **Alternate scroll mode** (DECSET 1007): (verified 2026-03-29)
  - [x] `ALTERNATE_SCROLL` TermMode flag (default on, matching xterm)
  - [x] Alt screen + ALTERNATE_SCROLL: scroll wheel → `\x1bOA`/`\x1bOB` (SS3 arrow keys)
- [x] **Mouse event dispatch**: (verified 2026-03-29)
  - [x] `should_report_mouse()` — checks ANY_MOUSE + !Shift
  - [x] `report_mouse_button()` — encode + write to PTY
  - [x] `report_mouse_motion()` — motion dedup + encode
  - [x] `handle_mouse_wheel()` — 3-tier: report → alt scroll → viewport scroll
  - [x] `handle_mouse_input()` — left/middle/right button dispatch
- [x] **Tests** (100 tests in `mouse_report/tests.rs`): (verified 2026-03-29 -- significantly expanded from original 31; all pass)
  - [x] SGR encoding (9+ tests): left/middle/right, release, coords, modifiers, scroll, motion, large coords, extreme coords, full round-trip (verified 2026-03-29)
  - [x] Normal encoding (8+ tests): correct format, coord clamping, release code, max coord boundary, modifier release (verified 2026-03-29)
  - [x] UTF-8 encoding (9+ tests): small coords, boundary single/two-byte, multi-byte, out-of-range, max coord, symmetry (verified 2026-03-29)
  - [x] URXVT encoding (8 tests): origin, large coords, scroll, priority vs UTF-8, priority vs SGR, modifiers, release (verified 2026-03-29)
  - [x] X10 mode (10 tests): press encodes, release suppressed, strips modifiers, all buttons, out-of-range, motion suppressed (verified 2026-03-29)
  - [x] button_code (6 tests): all buttons + motion offset (verified 2026-03-29)
  - [x] apply_modifiers (5+ tests): none, shift, alt, ctrl, combined, exhaustive 8x4 matrix (verified 2026-03-29)
  - [x] Dispatch (6+ tests): SGR/UTF-8/Normal selection, SGR priority, release codes, boundary dispatch (verified 2026-03-29)
  - [x] Mutual exclusion (10 tests): tracking mode clear, encoding mode clear, DECRST behavior, RIS clear (verified 2026-03-29)

---

## 10.3 Section Completion (verified 2026-03-29)

- [x] All 10.1-10.2 items complete (verified 2026-03-29)
- [x] `./test-all.sh` — all tests pass (verified 2026-03-29 -- ~185 mouse-related tests across oriterm + oriterm_core)
- [x] `./clippy-all.sh` — no warnings (verified 2026-03-29)
- [x] Mouse selection state machine handles all gesture types (single/double/triple click, drag, release) (verified 2026-03-29)
- [x] Drag threshold prevents accidental selection (verified 2026-03-29)
- [x] Mouse reporting sends correct sequences for all four encoding formats (SGR, UTF-8, URXVT, X10 Normal) (verified 2026-03-29 -- cross-referenced against Alacritty + WezTerm)
- [x] All tracking modes work: click-only, drag, all-motion (verified 2026-03-29)
- [x] Modifier bits correct in mouse reports (Shift, Alt, Ctrl) (verified 2026-03-29)
- [x] Scroll wheel events reported correctly (verified 2026-03-29)
- [x] Shift bypasses mouse reporting for local selection (verified 2026-03-29)
- [x] Motion events deduplicated (only report on cell change) (verified 2026-03-29)
- [x] Alternate scroll mode converts scroll to arrow keys in alt screen (verified 2026-03-29)
- [x] Mouse mode and selection mode coexist correctly (mutual exclusion with Shift override) (verified 2026-03-29)
- [x] Tracking modes mutually exclusive via `ANY_MOUSE` clear on DECSET (verified 2026-03-29)
- [x] Encoding modes mutually exclusive via `ANY_MOUSE_ENCODING` clear on DECSET (verified 2026-03-29)
- [x] Zero-allocation encoding via stack-allocated `MouseReportBuf` (verified 2026-03-29)

**Exit Criteria:** Mouse reporting works correctly for all terminal applications that use it. vim, tmux, htop, and other mouse-aware apps receive correct mouse events. Selection and reporting coexist cleanly with Shift-override convention.
