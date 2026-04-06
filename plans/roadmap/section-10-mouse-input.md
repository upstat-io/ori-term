---
section: 10
title: Mouse Input & Reporting
status: in-progress
reviewed: false
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
  - id: "10.4"
    title: "Horizontal Scroll Support"
    status: not-started
  - id: "10.5"
    title: "TUI Scroll Magnitude Forwarding"
    status: not-started
  - id: "10.6"
    title: "Cancel Inertial Scroll on Screen Switch"
    status: not-started
  - id: "10.7"
    title: "Click-Through on Unfocused Window"
    status: not-started
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

## 10.4 Horizontal Scroll Support

<!-- WezTerm audit: #7665 (horizontal scroll wheel buttons 6/7 not recognized on X11) -->

**Source:** WezTerm #7665 — Horizontal scroll wheel (X11 buttons 6/7, trackpad horizontal swipe) not recognized. The hardware works in Chrome, VSCode, and Kitty, but WezTerm ignores it.

**Problem:** `parse_wheel_delta()` in `oriterm/src/app/mouse_report/mod.rs` discards x-axis scroll data entirely. The widget framework (`ScrollWidget`) already supports horizontal scrolling via `scroll_by_x()`, but the application input handler only processes `y` delta.

**Required work:**

- [ ] Extend `parse_wheel_delta()` to return both x and y deltas (currently returns `Option<(usize, bool)>` for vertical only)
- [ ] Forward horizontal scroll to TUI apps via mouse protocol: X11 buttons 6 (WheelLeft) and 7 (WheelRight) in SGR mouse encoding
- [ ] Wire horizontal scroll to UI framework for widgets that support it (ScrollWidget with `ScrollDirection::Horizontal` or `Both`)
- [ ] Handle winit `MouseScrollDelta::LineDelta(x, _)` where x != 0 (currently only `(_, y)` is used)
- [ ] Test: generate horizontal scroll event, verify SGR mouse report with button 66/67 encoding

**Priority:** Medium — affects users with trackpads and horizontal scroll wheels (Logitech MX Master, etc.).

---

## 10.5 TUI Scroll Magnitude Forwarding

<!-- WezTerm audit: #7645 (scrolling in TUIs sluggish with precision pointing devices) -->

**Source:** WezTerm #7645 — When a TUI app (tmux, vim, emacs) has mouse reporting enabled, scroll gestures from trackpads are collapsed to a single line regardless of swipe speed. Fast trackpad swipes should generate multiple scroll events proportional to the gesture magnitude.

**Problem:** When mouse reporting is active, ori_term converts the scroll delta to a line count but then sends only ONE mouse scroll event to the PTY. A fast trackpad swipe producing delta=5.0 lines should send 5 scroll events, not 1.

**Required work:**

- [ ] When mouse reporting is active: send N scroll events where N = `delta.abs().ceil() as usize` (clamped to a reasonable max like 20)
- [ ] Each event is a separate SGR mouse report with the scroll button code
- [ ] Works for both `PixelDelta` (trackpad → divide by cell height → N events) and `LineDelta` (mouse wheel → already in lines)
- [ ] Configurable scroll multiplier for mouse-reporting apps (e.g., `tui_scroll_multiplier = 1.0`)
- [ ] Test: with mouse reporting enabled, simulate trackpad swipe with delta=5 lines → verify 5 SGR scroll events sent to PTY

**Priority:** Medium — affects all trackpad users running tmux/vim/neovim.

**Reference:** Kitty scroll multiplier, Ghostty scroll sensitivity config.

---

## 10.6 Cancel Inertial Scroll on Screen Switch

<!-- Ghostty audit: #3845 (cancel inertial scroll when changing between primary/alt screen) -->

**Source:** Ghostty #3845 — When a user is scrolling with a trackpad (inertial/momentum scroll) and an application switches between primary and alt screen (e.g., opening vim, less, or hitting Ctrl+C in a TUI), the remaining momentum scroll events continue to arrive and get forwarded to the new screen. This causes unexpected scrolling in the newly-switched screen.

**Required work:**

- [ ] Detect screen switch events (mode 1049/1047 set/reset) in the VTE handler
- [ ] On screen switch: set a "cancel inertial scroll" flag
- [ ] In the scroll handler: when flag is set, discard incoming scroll events until a brief cooldown (200-300ms) expires or a new deliberate scroll gesture starts
- [ ] Only apply to `PixelDelta` (trackpad) events, not `LineDelta` (mouse wheel) — mouse wheels don't have inertia
- [ ] Test: simulate rapid scroll events, trigger alt screen switch, verify scroll events after switch are discarded

**Priority:** Low — affects trackpad users switching between terminal apps and shell.

---

## 10.7 Click-Through on Unfocused Window

<!-- Alacritty audit (closed): #2929 (allow first click on unfocused window to pass through) -->

**Source:** Alacritty #2929 — On macOS (and optionally other platforms), the first click on an unfocused terminal window should pass through to the terminal instead of just focusing the window. This matches native macOS app behavior and avoids a "wasted click" when switching to the terminal to click a URL or position the cursor.

**Required work:**

- [ ] Config option: `click_through_unfocused = true` (default: true on macOS, false on other platforms — matching platform conventions)
- [ ] When enabled: on window focus event triggered by mouse click, forward the click event to the terminal after focusing
- [ ] When disabled: first click only focuses the window, no click event forwarded
- [ ] Must interact correctly with: URL click (Ctrl+click), selection start, mouse reporting
- [ ] Platform-specific: macOS `acceptsFirstMouse` returns YES; Windows/Linux handle via focus event + synthetic click
- [ ] Test: unfocused window, click on URL → verify window focuses AND URL opens in one click

**Priority:** Low — UX polish, especially important on macOS where this is the expected behavior.

**Reference:** Alacritty `acceptsFirstMouse`, macOS HIG (click-through behavior).

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
