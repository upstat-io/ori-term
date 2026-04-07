---
plan: "tack-conformance"
title: "Tack Conformance: Automated Terminfo Capability Validation Suite"
status: in-progress
references:
  - "plans/completed/vttest-conformance/"
  - "plans/completed/golden-image-audit/"
---

# Tack Conformance: Automated Terminfo Capability Validation Suite

## Mission

Automate tack's (Terminfo Action Checker, ncurses v1.08) test scenarios in ori_term's test suite, validating terminfo capability correctness alongside the existing vttest VT protocol conformance tests. Along the way: consolidate the duplicated VtTestSession infrastructure into a shared test-support crate, create a pinned ori_term terminfo entry, and build a structured scenario catalog framework for scalable terminal capability testing.

This is a **testing infrastructure plan** (side plan, not roadmap). It complements the vttest-conformance work: vttest validates VT protocol compliance, tack validates terminfo entry accuracy. Together they form a comprehensive terminal emulation validation suite.

**Novel territory:** No reference terminal emulator (Alacritty, WezTerm, Ghostty) has automated tack testing. This plan pioneers machine-driven tack validation using the same PTY-driven model proven by vttest integration.

## Mission Success Criteria

- [ ] `crates/oriterm_test_support` crate exists with shared `PtySession` infrastructure
- [ ] `PtySession` has explicit `impl Drop` that kills+reaps the child process (fixes pre-existing zombie-leak bug in current `VtTestSession`)
- [ ] vttest text tests (`oriterm_core/tests/vttest/`) migrated to use shared `PtySession` — all 198 existing snapshots unchanged
- [ ] vttest GPU golden tests (`oriterm/src/gpu/visual_regression/vttest/`) migrated to use shared `PtySession` — all golden images unchanged
- [ ] VtTestSession duplication eliminated (LEAK fixed)
- [ ] `vttest_available()` defined in exactly ONE location (shared crate) — no scattered knowledge
- [ ] `extra/ori_term.info` terminfo source exists, derived from xterm-256color with explicit capability declarations
- [ ] `tic` compiles `ori_term.info` successfully; tests use pinned `TERM=ori_term` + `TERMINFO_DIRS` pointing to compiled entry
- [ ] Tack test scenarios cover EVERY navigable begin-testing screen: modes/glitches, ACS, graphic rendition, color, cursor movement, pad timing, send strings, labels. Interactive-only screens (function key test, edit terminfo, output) have concrete in-code exclusion stubs.
- [ ] Tack tool scenarios cover EVERY automatable tools screen: ANSI status reports (DA/DSR), SGR modes, character sets, ENQ/ACK, OSC queries. Interactive/overlap tools (scan codes, decompile terminfo) have in-code stubs.
- [ ] Text snapshots (insta) exist for all navigable tack test screens at 80x24 (with size matrix for color/cursor)
- [ ] GPU golden images exist for curated visual tack test subset: color (3 sizes), graphic rendition, character sets, modes
- [ ] Keyboard/function key capability tests exist in `oriterm` crate exercising real key encoding pipeline for the FULL kf1-kf63 namespace (F1-F12, Shift, Ctrl, Ctrl+Shift, Alt, Alt+Shift) plus cursor keys (normal + application mode) plus editing keys
- [ ] All tests skip cleanly when tack/tic unavailable (cross-platform: compile everywhere, runtime skip)
- [ ] `./test-all.sh` green, `./build-all.sh` green, `./clippy-all.sh` green — no regressions

## Architecture

```
                          crates/oriterm_test_support
                         ┌──────────────────────────────┐
                         │  PtySession                   │
                         │    spawn(cmd, env) -> session  │
                         │    drain() / wait() / send()  │
                         │    grid_text() / grid_chars() │
                         │                               │
                         │  PtyResponseCollector          │
                         │    (captures Event::PtyWrite)  │
                         │                               │
                         │  TerminfoEnv                   │
                         │    compile(info) -> temp dir   │
                         │    env_vars() -> (TERM, DIRS)  │
                         │                               │
                         │  tool_available(name) -> bool  │
                         └───────────┬──────────┬────────┘
                  dev-dep ↓          ↓ dev-dep
         ┌────────────────┴──┐    ┌──┴────────────────────┐
         │   oriterm_core    │    │      oriterm           │
         │   tests/vttest/   │    │  gpu/visual_regression/│
         │   tests/tack/     │    │    vttest/ (golden)    │
         │   (text snapshots)│    │    tack/   (golden)    │
         └───────────────────┘    │  tests/ (keyboard)     │
                                  └────────────────────────┘
```

**Data flow for tack tests:**
```
tack binary (PTY) → PtySession → VTE parser → Term → grid state
                                                  ↓
                                          RenderableContent
                                        ↙              ↘
                    grid_text() → insta snapshot    GPU render → golden PNG
```

## Design Principles

### 1. Single Source of Truth for PTY Test Infrastructure

The VtTestSession duplication between `oriterm_core/tests/vttest/session.rs` and `oriterm/src/gpu/visual_regression/vttest/mod.rs` (95% identical code, 581-line file exceeding 500-line limit) is a LEAK finding. A shared `PtySession` in `crates/oriterm_test_support` eliminates this. The GPU-specific methods (`frame_input`, `assert_golden`) remain in `oriterm` as standalone functions that adapt `PtySession` — the shared crate stays GPU-free.

The same single-source rule applies to the **scenario catalog**: `ScenarioSpec` const values, per-scenario parsers, the `TackNavigator`, and the `ScenarioRunner` all live in `crates/oriterm_test_support::tack_framework`. Both `oriterm_core/tests/tack/` (text scenarios in Sections 05-06) and `oriterm/src/gpu/visual_regression/tack/` (GPU goldens in Section 07) consume the same const ScenarioSpec values via `use oriterm_test_support::tack_framework::scenarios::*`. Test wrapper `#[test] fn`s live near the test target (`oriterm_core/tests/tack/test_menu/*.rs`, `oriterm_core/tests/tack/tools_menu/*.rs`, `oriterm/src/gpu/visual_regression/tack/mod.rs`) — they're thin wrappers calling `ScenarioRunner::run(&scenarios::FOO)`.

### 2. Pinned Terminfo Entry

Without a controlled terminfo entry, tests validate the host's ncurses database, not ori_term's capabilities. The plan creates `extra/ori_term.info` (derived from xterm-256color, following Alacritty's `alacritty.info` and WezTerm's `wezterm.terminfo` patterns), compiled at test runtime via `tic` into a temp directory. `TERM=ori_term` and `TERMINFO_DIRS` are set per-session.

### 3. Structured Scenario Catalog

tack's interactive menu-driven design requires structured navigation scripts. Each scenario has a semantic ID (`tack_modes_am`, not `tack_01_02`), a menu path (`[n, x]` = begin testing → modes), a readiness anchor (prompt text), and per-scenario assertions. This prevents the fragile regex-over-whole-grid antipattern.

## Section Dependency Graph

```
Section 01 ──→ Section 02 ──→ Section 03 ──→ Section 04
(PtySession)   (Terminfo)     (Smoke Test)   (Scenario Framework)
                                                  ↓
                                    ┌─────────────┼─────────────┐
                                    ↓             ↓             ↓
                              Section 05    Section 06    Section 07
                              (Test Menu)   (Tools Menu)  (GPU Golden)
                                    ↓             ↓             ↓
                                    └─────────────┼─────────────┘
                                                  ↓
                                            Section 08
                                           (Keyboard)
                                                  ↓
                                            Section 09
                                          (Verification)
```

- Sections 01-04 are sequential: each gates the next.
- Sections 05, 06, 07 are independent after 04 and can be worked in any order.
- Section 08 depends on 01 (shared PtySession) and 02 (terminfo) but NOT on 03-07. It tests ori_term's key encoding pipeline directly (not through tack), using the pinned terminfo entry to validate key sequences match what the terminfo claims.
- Section 09 requires all prior sections.

**Cross-section interactions:**
- **Section 01 + existing vttest tests**: Migration must preserve all 198 existing snapshots and golden images. Zero behavioral change.
- **Section 02 + Sections 05-08**: All tack/keyboard tests depend on the pinned terminfo entry from Section 02.

## Implementation Sequence

```
Phase 0 - Infrastructure
  └─ 01: Shared PtySession crate (dedup LEAK, migrate vttest)
  Gate: all existing vttest tests pass unchanged

Phase 1 - Foundation
  └─ 02: Terminfo provisioning (ori_term.info, tic compilation)
  └─ 03: Tack smoke test (prove PTY + terminfo + navigation works)
  Gate: tack launches, navigates main menu, captures grid snapshot

Phase 2 - Scenario Framework
  └─ 04: Scenario catalog framework (ScenarioSpec, TackNavigator)
  Gate: one end-to-end scenario (e.g., tack_modes_am) passes with snapshot

Phase 3 - Test Coverage  [CRITICAL PATH]
  └─ 05: Test menu scenarios (modes, ACS/SGR, color, cursor)
  └─ 06: Tools menu scenarios (ANSI status, SGR, charsets)
  └─ 07: GPU golden images (visual subset)
  └─ 08: Keyboard/function key tests
  Gate: all tack categories have test coverage

Phase 4 - Verification
  └─ 09: Full test matrix, cross-platform, regression checks
```

**Why this order:**
- Phase 0 is a pure infrastructure refactor with zero behavioral change — safe foundation.
- Phase 1 establishes the terminfo contract that all subsequent tests depend on.
- Phase 2 builds the framework once, then Phases 3-4 scale it to all categories.
- GPU goldens (Section 07) come after text snapshots (05-06) because text is faster to iterate on.

## Metrics (Current State)

| Area | Files | Lines |
|------|-------|-------|
| vttest text tests (`oriterm_core/tests/vttest/`) | 13 .rs + 260 .snap | ~1,460 |
| vttest GPU golden tests (`oriterm/src/gpu/visual_regression/vttest/`) | 2 .rs | ~840 |
| GPU visual regression infra (`oriterm/src/gpu/visual_regression/mod.rs`) | 1 .rs | ~302 |
| Golden reference images (`oriterm/tests/references/`) | 137 PNGs | — |

**Duplication to eliminate:** ~240 lines of identical PtySession code between two files.

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 Shared PtySession | ~320 new (incl. Drop), ~480 deleted | Medium | — |
| 02 Terminfo Provisioning | ~200 | Medium | 01 |
| 03 Tack Smoke Test | ~100 | Low | 02 |
| 04 Scenario Framework | ~260 (incl. snapshot policy doc) | Medium | 03 |
| 05 Test Menu Scenarios | ~550 (18 scenarios + 3 stubs) | Medium | 04 |
| 06 Tools Menu Scenarios | ~420 (7 scenarios + 2 stubs) | Medium | 04 |
| 07 GPU Golden Images | ~240 (6 goldens) | Low | 01, 02, 04, 05 |
| 08 Keyboard Tests | ~320 (kf1-kf63) | Medium | 01, 02 |
| 09 Verification | ~100 | Low | All |
| **Total new** | **~2,510** | | |
| **Total deleted** | **~480** | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| VtTestSession duplicated (LEAK) | No shared test-support crate | Section 01 | Not Started |
| `vttest/mod.rs` 581 lines (BLOAT) | Mixed concerns: PTY + GPU rendering | Section 01 | Not Started |
| `VtTestSession::_child` never killed or reaped on Drop — every test leaks a zombie vttest child (std::process::Child does NOT kill on drop) | Missing `impl Drop` that calls `child.kill()` + `child.wait()` | Section 01 (new `impl Drop` on PtySession) | Not Started |
| `vttest_available()` defined twice: `oriterm_core/tests/vttest/session.rs:232` and `oriterm/src/gpu/visual_regression/vttest/mod.rs:297` (scattered knowledge) | No shared test-support crate | Section 01 (delete both, re-export from shared crate) | Not Started |
| TERM hardcoded as scattered constant | No canonical terminfo provisioning | Section 02 | Not Started |
| `oriterm::key_encoding` is `pub(crate)` — not reachable from an integration test target | Over-restricted visibility on a stable, well-tested module | Section 08 adopts the PREFERRED in-crate sibling test approach at `oriterm/src/key_encoding/terminfo_xcheck.rs` so NO visibility change is required. The `pub(crate)` scope stays as-is. | Not Started |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Shared PtySession Infrastructure | `section-01-shared-pty-session.md` | Not Started |
| 02 | Terminfo Provisioning | `section-02-terminfo-provisioning.md` | Not Started |
| 03 | Tack Smoke Test | `section-03-tack-smoke-test.md` | Not Started |
| 04 | Scenario Catalog Framework | `section-04-scenario-framework.md` | Not Started |
| 05 | Tack Scenarios: Test Menu | `section-05-test-menu-scenarios.md` | Not Started |
| 06 | Tack Scenarios: Tools Menu | `section-06-tools-menu-scenarios.md` | Not Started |
| 07 | GPU Golden Images | `section-07-gpu-golden-images.md` | Not Started |
| 08 | Keyboard/Function Key Tests | `section-08-keyboard-tests.md` | Not Started |
| 09 | Verification | `section-09-verification.md` | Not Started |
