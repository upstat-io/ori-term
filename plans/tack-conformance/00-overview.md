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

- [x] `crates/oriterm_test_support` crate exists with shared `PtySession` infrastructure
- [x] `PtySession` has explicit `impl Drop` that kills+reaps the child process (fixes pre-existing zombie-leak bug in current `VtTestSession`)
- [x] vttest text tests (`oriterm_core/tests/vttest/`) migrated to use shared `PtySession` — all 198 existing snapshots unchanged
- [x] vttest GPU golden tests (`oriterm/src/gpu/visual_regression/vttest/`) migrated to use shared `PtySession` — all golden images unchanged
- [x] VtTestSession duplication eliminated (LEAK fixed)
- [x] `vttest_available()` defined in exactly ONE location (shared crate) — no scattered knowledge
- [x] `extra/ori_term.info` terminfo source exists as a hand-authored, fully-pinned entry: a private `ori_term+common` base fragment plus two user-facing entries (`ori_term` and `ori_term-direct`) that consume only the private fragment via `use=ori_term+common,`. The capability vocabulary is derived from xterm-256color conventions, but there is NO `use=xterm-256color,` inheritance — every cap is declared explicitly so host terminfo drift never silently changes what ori_term claims. (Matches Alacritty's `alacritty+common` pattern.)
- [x] `tic` compiles `ori_term.info` successfully; tests use pinned `TERM=ori_term` + BOTH `TERMINFO` and `TERMINFO_DIRS` pointing at the compiled entry (some ncurses consumers honor only one of the two — set both). Verified end-to-end by `child_process_with_apply_env_reads_pinned_terminfo`, which spawns `infocmp` with the env triple set and asserts `infocmp`'s reconstruction-source header points inside `env.terminfo_dir()` — proving env-precedence steered the child to OUR compiled entry, not any system-installed `ori_term`. The test is immune to future packaging releases that might install `ori_term` system-wide.
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
                         │    compile() -> temp dir        │
                         │    apply_env(&mut cmd)          │
                         │      sets TERM/TERMINFO/DIRS    │
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
                                            Section 05
                                           (Test Menu)
                                       — owns cap_coverage_matrix —
                                                  ↓
                                    ┌─────────────┼─────────────┐
                                    ↓             ↓             ↓
                              Section 06    Section 07    Section 08
                              (Tools Menu)  (GPU Golden)  (Keyboard)
                                    └─────────────┼─────────────┘
                                                  ↓
                                            Section 09
                                          (Verification)
```

- Sections 01-04 are sequential: each gates the next.
- **Section 05 now gates Sections 06/07/08** (post Agent-2 review). Section 05 introduces (a) the `PhaseSpec` framework extension, (b) the `tack_version_supported()` gate, (c) the `BEGIN_TESTING_INVENTORY` discovery pattern, and (d) the `cap_coverage_matrix` SSOT enforcement. Sections 06 and 08 must extend `covered_caps()` with their cap lists and remove matching `EXEMPT_CAPS` entries — the matrix test fires loudly on stale exemptions. Section 07 inherits the version gate and the open architectural decision (default: NO `run_phase_with_session_at`, modes GPU golden uses stable-screen `TACK_MODES_AM`).
- Section 06 (tools menu) and Section 08 (keyboard) can be worked in parallel after Section 05 lands.
- Section 07 (GPU goldens) can be worked in parallel after Sections 05/06 land (its const ScenarioSpec values come from the `tack_framework::scenarios::*` SSOT module Sections 05/06 populate).
- Section 09 requires all prior sections.

<!-- reviewed: cohesion fix — Sections 06/07/08 now formally depend on Section 05 because Section 05 introduces the cross-section contracts (PhaseSpec, version gate, cap_coverage_matrix, expand_kf_caps/expand_modified_key_caps SSOT helpers) that those sections consume. The previous "independent after 04" claim was true for the first draft but no longer accurate. -->

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

Phase 3 - Test Catalog Foundation [CRITICAL PATH]
  └─ 05: Test menu scenarios + framework extensions
        - PhaseSpec / ScenarioRunner::run_phase (mid-flow capture)
        - tack_version_supported gate
        - BEGIN_TESTING_INVENTORY discovery + drift gate
        - cap_coverage_matrix SSOT enforcement
        - test menu scenarios (modes, ACS/SGR, color, cursor)
  Gate: catalog framework extensions land; cap_coverage_matrix passes
        with deferral exemptions for Sections 06/08; all 05 scenarios green

Phase 4 - Test Catalog Expansion (parallel after Phase 3)
  └─ 06: Tools menu scenarios + cap_coverage extension (consumer of 05)
  └─ 07: GPU golden images for visual subset (consumer of 05)
  └─ 08: Keyboard/function key tests + cap_coverage extension (consumer of 05)
  Gate: cap_coverage_matrix passes with all deferral exemptions REMOVED
        (every cap in extra/ori_term.info is now in covered_caps())

Phase 5 - Verification
  └─ 09: Full test matrix, cross-platform, regression checks
```
<!-- reviewed: cohesion fix — Phase 3/4 split reflects the post-Agent-2 dependency reality where Sections 06/07/08 consume Section 05's framework extensions and cap_coverage SSOT. -->

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
| 05 Test Menu Scenarios | ~1,100 (discovery + phase-capture extension + version gate + 18 scenarios + 3 stubs + owner-partitioned cap-coverage matrix [Pivot 5 of Agent 3] + cross-section sync + traceability table + per-section CapCoverageContribution files + expand_kf/expand_modified_key helpers + stale-exemption negative pin + runtime sentinel forcing-function helpers [Pivot 3] + Implementation Milestones M1/M2 [Pivot 1] + algorithmic-DRY poll_until reuse note) | High | 04 |
| 06 Tools Menu Scenarios | ~480 (7 scenarios + 2 stubs + cap_coverage extension subsection that moves tools-menu caps from EXEMPT_CAPS into covered_caps) | Medium | 04, 05 |
| 07 GPU Golden Images | ~240 (6 goldens; modes golden uses TACK_MODES_AM stable-screen, NOT phase capture) | Low | 01, 02, 04, 05 |
| 08 Keyboard Tests | ~360 (kf1-kf63 + cursor + editing + modified-key family + cap_coverage extension subsection) | Medium | 01, 02, 05 |
| 09 Verification | ~100 | Low | All |
| **Total new** | **~3,030** | | |
<!-- reviewed: accuracy/feasibility fix — Section 05 grew by ~350 lines after Agent 1 of /review-plan added the discovery, phase-capture, version-gate, and cap-coverage matrix subsections. -->
<!-- reviewed: cohesion fix — Section 05 grew an additional ~100 lines after Agent 2 added: mission-criterion traceability table, cross-section sync subsection (05.5b) updating Sections 06/07/08 contracts, expanded EXEMPT_CAPS with cross-section deferral notes for ~60 caps, expand_kf_caps + expand_modified_key_caps helpers, stale-exemption negative pin in cap_coverage_matrix test, compile_error! forcing-function gates replacing the broken byte-literal placeholders in 05.2/05.3/05.4, loud-skip diagnostic in tack_version_supported, expanded phase-capture timeout panic. Sections 06 / 07 / 08 grew by ~60/0/40 lines for the cross-section sync content (cap_coverage extension checklists + frontmatter re_review_reason updates). Section 06 now also depends on 05 (cap_coverage extension contract) and Section 08 now depends on 05 (same). -->
<!-- reviewed: executability/hygiene fix (Agent 3) — Section 05 grew an additional ~150 lines after Agent 3 of /review-plan applied the Codex midpoint pivots: Pivot 1 added Implementation Milestones M1/M2 with explicit completion gates so the section is implementable without cognitive overload, Pivot 3 replaced compile_error! forcing-functions with runtime unverified_menu_key()/unverified_anchor() sentinels (the compile_error! design broke `cargo check` for the entire oriterm_test_support crate while 05.0 was in flight, blocking concurrent impl-hygiene work — Codex flagged this as too hostile in a multi-agent flow), Pivot 5 refactored the cap-coverage matrix to owner-partitioned CapCoverageContribution per-section files instead of a single flat EXEMPT_CAPS junk drawer (each section's exemptions live in their own owned file), Agent 3 also wove in: parser/tokens.rs sibling-tests-restructure as a Broken Window fix in 05.0.b, the runner/mod.rs split into runner/{mod, stable, phase} BEFORE adding run_phase to stay under 500 lines, the algorithmic-DRY poll_until reuse mandate in run_phase_at, the poll_until visibility widening from pub(super) to pub(crate), TDD ordering checklist items, debug+release parity gates, cross-platform compile gate weaved into M1, file-size projection for cap_coverage/* files. Pivot 2 refined Section 07's depends_on (07 only needs 04 + 05 cap_coverage CONTRACT, not body — captured in new depends_on_contract frontmatter field). Pivot 4 validated PhaseSpec design against the codebase (no action needed). -->

| **Total deleted** | **~480** | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| VtTestSession duplicated (LEAK) | No shared test-support crate | Section 01 | Not Started |
| `vttest/mod.rs` 581 lines (BLOAT) | Mixed concerns: PTY + GPU rendering | Section 01 | Not Started |
| `VtTestSession::_child` never killed or reaped on Drop — every test leaks a zombie vttest child (std::process::Child does NOT kill on drop) | Missing `impl Drop` that calls `child.kill()` + `child.wait()` | Section 01 (new `impl Drop` on PtySession) | Not Started |
| `vttest_available()` defined twice: `oriterm_core/tests/vttest/session.rs:232` and `oriterm/src/gpu/visual_regression/vttest/mod.rs:297` (scattered knowledge) | No shared test-support crate | Section 01 (delete both, re-export from shared crate) | Not Started |
| TERM hardcoded as scattered constant | No canonical terminfo provisioning | Section 02 | Complete |
| `oriterm::key_encoding` is `pub(crate)` — not reachable from an integration test target | Over-restricted visibility on a stable, well-tested module | Section 08 adopts the PREFERRED in-crate sibling test approach at `oriterm/src/key_encoding/terminfo_xcheck.rs` so NO visibility change is required. The `pub(crate)` scope stays as-is. | Not Started |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Shared PtySession Infrastructure | `section-01-shared-pty-session.md` | Complete |
| 02 | Terminfo Provisioning | `section-02-terminfo-provisioning.md` | Complete |
| 03 | Tack Smoke Test | `section-03-tack-smoke-test.md` | Complete |
| 04 | Scenario Catalog Framework | `section-04-scenario-framework.md` | Complete |
| 05 | Tack Scenarios: Test Menu | `section-05-test-menu-scenarios.md` | In Progress (M1 complete) |
| 06 | Tack Scenarios: Tools Menu | `section-06-tools-menu-scenarios.md` | Not Started |
| 07 | GPU Golden Images | `section-07-gpu-golden-images.md` | Not Started |
| 08 | Keyboard/Function Key Tests | `section-08-keyboard-tests.md` | Not Started |
| 09 | Verification | `section-09-verification.md` | Not Started |
