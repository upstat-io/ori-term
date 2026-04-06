# Tack Conformance Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Shared PtySession Infrastructure
**File:** `section-01-shared-pty-session.md` | **Status:** Not Started

```
PtySession, PtyResponseCollector, oriterm_test_support, test-support crate
VtTestSession, session.rs, vttest, duplication, LEAK, dedup
portable-pty, PtySize, CommandBuilder, native_pty_system
EventListener, Event::PtyWrite, PtyResponder, drain, drain_blocking
wait, wait_for, send, grid_text, grid_chars, tool_available
dev-dependency, workspace member, crates/
```

---

### Section 02: Terminfo Provisioning
**File:** `section-02-terminfo-provisioning.md` | **Status:** Not Started

```
terminfo, termcap, TERM, TERMINFO_DIRS, xterm-256color
ori_term.info, tic, infocmp, extra/, terminfo source
TerminfoEnv, compile, temp directory, OnceLock
alacritty.info, wezterm.terminfo, ghostty.zig
am, bce, km, mir, msgr, xenl, colors, pairs
setaf, setab, sgr, cup, csr, smcup, rmcup
```

---

### Section 03: Tack Smoke Test
**File:** `section-03-tack-smoke-test.md` | **Status:** Not Started

```
tack, smoke test, menu navigation, PTY spawn
tack_available, tic_available, runtime skip
main menu, "Enter choice number", "tack [n] >"
basic information, terminal capabilities
```

---

### Section 04: Scenario Catalog Framework
**File:** `section-04-scenario-framework.md` | **Status:** Not Started

```
ScenarioSpec, TackNavigator, scenario catalog
semantic ID, menu_path, ready_anchor, screen_parser
tack_modes_am, tack_color_setf, tack_cursor_cup
insta, snapshot naming, assert_snapshot, grid_text
per-scenario parser, test assertions, Done marker
```

---

### Section 05: Tack Scenarios: Test Menu
**File:** `section-05-test-menu-scenarios.md` | **Status:** Not Started

```
tack/test, modes, glitches, ACS, graphic rendition, color, cursor movement
am, os, rmam, smam, bw, xenl, tabs
bel, flash, civis, cvvis, cnorm, sgr
colors, pairs, setf, setb, scp, op, ncv, bce
clear, home, cr, nel, cub1, cup, vpa, hpa
oriterm_core/tests/tack/, text snapshots, insta
```

---

### Section 06: Tack Scenarios: Tools Menu
**File:** `section-06-tools-menu-scenarios.md` | **Status:** Not Started

```
tack/tools, ANSI status reports, SGR modes, character sets
DA, DSR, primary device attributes, cursor position
SGR 0-79, bold, dim, underline, reverse, blink
G0, G1, GL, GR, character set banks, ACS
oriterm_core/tests/tack/, text snapshots
```

---

### Section 07: GPU Golden Images
**File:** `section-07-gpu-golden-images.md` | **Status:** Not Started

```
GPU, golden images, visual regression, render_to_pixels
headless_env, compare_with_reference, PIXEL_TOLERANCE
tack color, tack SGR, tack character sets
FrameInput, frame_input, assert_golden
oriterm/src/gpu/visual_regression/tack/
oriterm/tests/references/tack_*.png
```

---

### Section 08: Keyboard/Function Key Tests
**File:** `section-08-keyboard-tests.md` | **Status:** Not Started

```
keyboard, function keys, smkx, rmkx, key encoding
kf1-kf63, kcub1, kcud1, kcuf1, kcuu1
oriterm, key_encoding, KeyEncoder
tack/test/fkey, ENQ/ACK, u8, u9
```

---

### Section 09: Verification
**File:** `section-09-verification.md` | **Status:** Not Started

```
verification, test matrix, cross-platform
build-all.sh, clippy-all.sh, test-all.sh
Windows, macOS, Linux, runtime skip
tack_available, tic_available, infocmp
performance, regression, zero idle CPU
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Shared PtySession Infrastructure | `section-01-shared-pty-session.md` |
| 02 | Terminfo Provisioning | `section-02-terminfo-provisioning.md` |
| 03 | Tack Smoke Test | `section-03-tack-smoke-test.md` |
| 04 | Scenario Catalog Framework | `section-04-scenario-framework.md` |
| 05 | Tack Scenarios: Test Menu | `section-05-test-menu-scenarios.md` |
| 06 | Tack Scenarios: Tools Menu | `section-06-tools-menu-scenarios.md` |
| 07 | GPU Golden Images | `section-07-gpu-golden-images.md` |
| 08 | Keyboard/Function Key Tests | `section-08-keyboard-tests.md` |
| 09 | Verification | `section-09-verification.md` |
