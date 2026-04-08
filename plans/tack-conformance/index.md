---
reroute: true
name: "Tack Conformance"
full_name: "Tack Conformance: Automated Terminfo Capability Validation Suite"
status: active
order: 1
---

# Tack Conformance Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Shared PtySession Infrastructure
**File:** `section-01-shared-pty-session.md` | **Status:** Complete

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
**File:** `section-02-terminfo-provisioning.md` | **Status:** Complete

```
terminfo, termcap, TERM, TERMINFO, TERMINFO_DIRS, xterm-256color
ori_term.info, ori_term-direct, ori_term+common, tic, infocmp
extra/, terminfo source, hand-authored, no host inheritance
TerminfoEnv, TerminfoVariant, compile, compile_with_variant
apply_env, env_pairs SSOT, include_str!, tempfile::TempDir
tic_available, infocmp_available, runtime gate, skip discipline
alacritty.info, wezterm.terminfo, ghostty.zig
am, bce, ccc, km, mir, msgr, xenl, colors, pairs
setaf, setab, sgr, cup, csr, smcup, rmcup, smkx, rmkx
acsc, smacs, rmacs, rep, BD, BE, PS, PE, kxIN, kxOUT, XF
Tc, Ms, Ss, Se, Smulx, Setulc, Sync, hs, dsl, tsl, fsl
kf1-kf63, BUG-07-008, child-process integrity test
```

---

### Section 03: Tack Smoke Test
**File:** `section-03-tack-smoke-test.md` | **Status:** Complete

```
tack, smoke test, menu navigation, PTY spawn
tack_available, tic_available, runtime skip
main menu, "Enter choice number", "tack [n] >"
basic information, terminal capabilities
wait_for_child_exit, bounded poll, try_wait, GetExitCodeProcess
ExitStatus, exit code surfacing, reader EOF, hot-spin mitigation
grid_text fidelity scope, snapshot canary
PATH override skip verification, no temporary scaffolding
snapshot stage before flake loop, git-add ordering
BUG-07-004 adjacent (Windows ConPTY child-lifecycle, not size)
Drop guard prerequisite, panic-on-timeout cleanup
mission tracing, Section 04 handoff, ScenarioSpec prerequisite
03.4 skip+compile, 03.5 exit+cleanup, 03.T TPR checkpoint
platform-gated diagnostics (strace Linux, lsof macOS, Get-Process Windows)
Section 04 hard handoff: ScenarioRunner::run_at must call quit_tack(5) — strict superset of wait_for_child_exit(2_000)
```

---

### Section 04: Scenario Catalog Framework
**File:** `section-04-scenario-framework.md` | **Status:** Complete

```
ScenarioSpec, MenuStep, TackNavigator, ScenarioRunner, ScenarioOutcome
semantic ID, screen_id, menu_path, ready_anchor, quit_path, parser
or_wait_for, MenuStep::new, snapshot_name, golden_name
LiveSession, LiveSession::finish, LiveSession::golden_name, M5 cleanup contract
PtySession::wait_for_with_context, PtySession::wait_for_any, PtySession::send_raw, PtySession::quit_tack
poll_until canonical bounded-poll helper, LEAK:algorithmic-duplication fix
bounded-poll invariant per-consumer pin (wait_for_with_context, wait_for_any, wait_for_child_exit_inner)
failing-test-first TDD discipline, debug+release parity
session/sync module, session/teardown module, 500-line split, BLOAT prevention
pre-existing-anchor guard (C1), state-aware quit (C2), exit-status assertion (C3)
catch_unwind antipattern banned, wait_for_any non-panicking primitive (M4b)
tokenized parser helpers, grid_has_token, grid_line_starts_with, grid_find_field
tack_framework::scenarios::*, single source of truth for catalog
tack_modes_am, modes screen, parse_modes_screen
insta, snapshot naming, assert_snapshot, grid_text
per-scenario parser, test assertions, Section 03 handoff reconciliation
cross-section consumer re-review gate for 05/06/07, Section 07 depends_on extends to 06
```

---

### Section 05: Tack Scenarios: Test Menu
**File:** `section-05-test-menu-scenarios.md` | **Status:** Complete

```
tack/test, modes, glitches, ACS, graphic rendition, color, cursor movement
pad_timing, send_strings, labels, function_key_test (stub), edit_terminfo (stub)
am, os, rmam, smam, bw, xenl, tabs
bel, flash, civis, cvvis, cnorm, sgr
colors, pairs, setf, setb, scp, op, ncv, bce
clear, home, cr, nel, cub1, cup, vpa, hpa
oriterm_core/tests/tack/, text snapshots, insta
PhaseSpec, ScenarioRunner::run_phase, run_phase_at, phase_anchor
BEGIN_TESTING_INVENTORY, BeginTestingKey, BeginTestingStatus
tack_begin_testing_inventory discovery test, drift gate
tack_version_supported, TACK_PINNED_MAJOR/MINOR, version gate, loud-skip diagnostic
cap_coverage_matrix, parse_declared_caps, CapCoverageContribution
section_05.rs, section_06.rs, section_08.rs, owner-partitioned exemptions (Pivot 5)
expand_kf_caps, expand_modified_key_caps, stale-exemption negative pin
tack_modes_phase_am/bce/bw/km/mir/msgr/xenl, unique screen_id per phase
grid_has_token, grid_has_paren_token, grid_find_field (M3 fix consumers)
mission criterion traceability table, cap-coverage contribution target
05.5b cross-section sync (06/07/08 contract changes)
unverified_menu_key, unverified_anchor runtime sentinels (Pivot 3)
phase-capture timeout panic includes setup_anchor + trigger + step count
Implementation Milestones M1 (foundation) / M2 (catalog) (Pivot 1)
poll_until reuse mandate (algorithmic-DRY skeleton, runner/phase.rs)
runner/mod.rs split into stable.rs + phase.rs (BLOAT prevention)
parser/tokens.rs sibling-tests restructure (Broken Window fix)
```

---

### Section 06: Tack Scenarios: Tools Menu
**File:** `section-06-tools-menu-scenarios.md` | **Status:** Not Started

```
tack/tools, ANSI status reports, SGR modes, character sets
DA, DSR, primary device attributes, cursor position
SGR 0-79, bold, dim, underline, reverse, blink
G0, G1, GL, GR, character set banks, ACS
ENQ/ACK, u8, u9, OSC 10, OSC 11, OSC queries
scan_codes (stub), decompile_terminfo (stub)
oriterm_core/tests/tack/, text snapshots
cap_coverage extension contract from Section 05.5
PhaseSpec consumer for scrolling tools-menu screens (e.g. SGR sweep)
tack_version_supported gate inherited via ScenarioRunner::available()
TOOLS_MENU_INVENTORY drift gate (parallel to Section 05's BEGIN_TESTING_INVENTORY)
covered_caps tools-menu extension: u6/u7/u8/u9, Cr/Cs, Ms, Smulx/Setulc/Sync,
BD/BE/PS/PE, AX/XT, hs/dsl/fsl/tsl, Se/Ss, XF/kxIN/kxOUT, Tc, RGB
```

---

### Section 07: GPU Golden Images
**File:** `section-07-gpu-golden-images.md` | **Status:** Not Started

```
GPU, golden images, visual regression, render_to_pixels
headless_env, compare_with_reference, PIXEL_TOLERANCE
tack color, tack SGR, tack character sets, tack modes
FrameInput, frame_input, assert_golden
oriterm/src/gpu/visual_regression/tack/
oriterm/tests/references/tack_*.png
6 goldens: color x3 + graphic_rendition + character_sets + modes
LiveSession::finish M5 cleanup contract
LiveSession::golden_name SSOT (no rebuilt format strings)
no run_phase_with_session_at — modes golden uses TACK_MODES_AM (os cap)
tack_version_supported gate inherited via ScenarioRunner::available()
```

---

### Section 08: Keyboard/Function Key Tests
**File:** `section-08-keyboard-tests.md` | **Status:** Not Started

```
keyboard, function keys, smkx, rmkx, key encoding
kf1-kf12 (unmodified), kf13-kf24 (shift), kf25-kf36 (ctrl)
kf37-kf48 (ctrl+shift), kf49-kf60 (alt), kf61-kf63 (alt+shift)
kcub1, kcud1, kcuf1, kcuu1 (cursor, app + normal mode)
kbs, khome, kend, kpp, knp, kdch1, kich1 (editing)
oriterm, key_encoding, KeyEncoder, in-crate sibling test
infocmp_query, decode_terminfo_string, CapMapping
oriterm/src/key_encoding/terminfo_xcheck.rs (preferred)
cap_coverage extension contract from Section 05.5
expand_kf_caps + expand_modified_key_caps SSOT helpers
covered_caps keyboard extension: kf1-kf63 + cursor + editing + modified-key family
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
