---
plan: "teseq-conformance"
title: "Teseq Conformance: Human-Readable Escape Sequence Test Framework"
status: in-progress
references:
  - "plans/completed/vttest-conformance/"
  - "plans/completed/golden-image-audit/"
---

# Teseq Conformance: Human-Readable Escape Sequence Test Framework

## Mission

Build a teseq-powered escape sequence test framework for ori_term that uses GNU teseq/reseq as the **authoring and analysis layer** (human-readable scenario files) with **terminal state snapshots as the oracle** (grid, cursor, modes, events via insta). This framework complements the existing vttest integration (black-box PTY-based conformance) and handler unit tests (per-sequence byte-level validation) by enabling:

1. **Human-readable test authoring** — scenarios written in teseq's annotated format, describing multi-sequence interactions in plain text
2. **State-based golden comparison** — grid state, cursor position, mode flags, and captured events as the authoritative golden output (via insta snapshots)
3. **Outbound response validation** — DA/DSR/DECRQM response bytes validated via raw PtyWrite byte assertions (canonical), with optional teseq analysis for human-readable debug output
4. **Multi-sequence workflow coverage** — complex interactions (scroll region + origin mode, alt screen roundtrips, DECCOLM transitions) that individual handler unit tests don't cover

**What this does NOT replace:** Existing vttest integration (1,459 lines, 198 snapshots), handler unit tests (5,860 lines), or GPU golden image tests (160+ reference PNGs). It adds a new test surface focused on scenario-based, human-readable, multi-sequence interaction testing.

**Novel territory:** No reference terminal emulator (Alacritty, WezTerm, Ghostty) uses teseq for testing. Alacritty uses binary recording + JSON grid snapshots (46MB). WezTerm uses structural unit tests organized by C0/C1/CSI. Ghostty uses embedded tests + AFL++ fuzzing (26MB corpus). This plan pioneers human-readable scenario-based testing.

## Mission Success Criteria

- [ ] `TeseqHarness` exists in `oriterm_core/tests/teseq/` with loader, runner, assertions, and reseq subprocess adapter
- [ ] `RecordedEvent` enum provides structured event capture (not `Debug` format strings)
- [ ] Scenario sidecar TOML format supports terminal config (size, scrollback, modes), pre-feed sequences, and expected assertions
- [ ] `reseq` subprocess converts `.teseq` scenario files to raw bytes; graceful skip when `reseq` unavailable
- [ ] C0 control character scenarios cover: CR, LF, BS, TAB, BEL, FF, VT, SO, SI
- [ ] CSI cursor movement scenarios cover: CUP, CUU, CUD, CUF, CUB, VPA, HPA, CHA with edge cases
- [ ] CSI erase scenarios cover: ED (modes 0-3), EL (modes 0-2)
- [ ] Erase-with-attributes workflow validates erased cells inherit cursor template background (SGR + ED/EL cross-cutting concern, tested in workflows)
- [ ] CSI insert/delete scenarios cover: ICH, DCH, IL, DL with scroll region interactions
- [ ] Mode interaction scenarios cover: DECOM+DECSTBM, DECCOLM+DECAWM, alt screen (1049), IRM
- [ ] SGR scenarios cover: 16-color, 256-color, TrueColor, bold-as-bright, dim, inverse, underline styles
- [x] Report/response scenarios cover: DA1, DA2, DA3, DSR cursor position, DECRQM with raw PtyWrite byte assertions (teseq analysis as optional debug aid, not oracle)
- [ ] ESC sequence scenarios cover: DECSC/DECRC, RIS, character set designation (SCS G0/G1)
- [ ] OSC scenarios cover: title+icon (0), icon name (1), title (2), clipboard (52), color query (4/10/11)
- [ ] OSC 7 (CWD) tested at mux layer via `RawInterceptor`, not teseq harness (documented limitation: `Term<T>` does not implement `set_working_directory`)
- [ ] Workflow scenarios cover: scroll region + origin mode combo, alt screen enter/exit roundtrip, DECCOLM transition, DA handshake sequence
- [ ] All scenarios run at 80x24 minimum; cursor clamping, mode interaction, and workflow scenarios also at 97x33 and 120x40
- [ ] `timeout 150 cargo test -p oriterm_core --test teseq` passes with zero failures
- [ ] `timeout 150 ./test-all.sh` green — no regressions in existing test suites
- [ ] CI gracefully skips teseq tests when `reseq` is unavailable (Windows, macOS without GNU tools)

## Architecture

```
                     Authoring Layer                       Execution Layer
                  ┌─────────────────┐                 ┌──────────────────────┐
 .teseq file ───►│  reseq (subprocess) ───► raw bytes ──►│ vte::Processor      │
                  └─────────────────┘                 │    ↓                  │
 .toml sidecar ──► ScenarioSpec ─────────────────────►│ Term<RecordedListener>│
   (size, modes,                                      │    ↓                  │
    pre_feed,                                         │ Grid state           │
    assertions)                                       │ Cursor position      │
                                                      │ Mode flags           │
                                                      │ RecordedEvent vec    │
                                                      └──────────┬───────────┘
                                                                 │
                     Assertion Layer                              │
                  ┌──────────────────────────────────────────────┘
                  │
                  ├─► grid_text() ──► insta::assert_snapshot!()  [grid golden]
                  ├─► cursor_pos() ──► assert_eq!()              [cursor check]
                  ├─► events() ──► insta::assert_snapshot!()     [event golden]
                  └─► pty_writes() ──► assert_pty_writes()        [raw byte oracle]
                       (optional)  ──► teseq ──► debug output   [human-readable aid]
```

**Scenario phases** (per ScenarioSpec):
1. `pre_feed` — mode setup sequences fed before the scenario (e.g., enable Mode 40). Events from pre-feed are cleared before the scenario runs.
2. `feed` — the `.teseq` scenario content, compiled via `reseq` to raw bytes. PtyWrite events emitted during feed (DA/DSR responses) are captured in `RecordedEvent` for assertion.

Note: outbound response validation (Section 03) uses the PtyWrite events captured during the `feed` phase — no separate "expect_outbound" phase is needed. Multi-step query/response workflows simply include the queries in the `.teseq` file.

## Design Principles

1. **teseq is the authoring tool, not the oracle.** Terminal state (grid, cursor, modes, events) is the authoritative golden — not teseq output. teseq's role is (a) human-readable scenario authoring (input) and (b) optional debug analysis of outbound response bytes (output). The canonical assertion for outbound responses is raw PtyWrite bytes via `assert_pty_writes()`, never teseq-analyzed output. This avoids coupling test correctness to teseq's output format, which is version-fragile and human-oriented.

2. **Complement, don't duplicate.** The existing handler unit tests (5,860 lines) already cover individual escape sequences thoroughly. This framework targets what they miss: multi-sequence interactions, mode combinations, stateful workflows, and outbound response validation. A scenario that tests only a single sequence in isolation belongs in `handler/tests.rs`, not here.

3. **Graceful degradation.** `reseq` is a GNU tool (Perl-based) that may not be available on all CI platforms. All tests must gracefully skip when `reseq` is unavailable, using the same pattern as vttest: `if !reseq_available() { eprintln!("..."); return; }`.

4. **Teseq format rules.** Three line types in `.teseq` files: (a) `|text|` — literal text (delimiters stripped, trailing `.` = LF appended); (b) `. LABEL/^X` — C0 control characters; (c) `: Esc [ params` — escape/CSI/DCS introducer lines (spaces between tokens are stripped by reseq). OSC content (titles, URIs, clipboard data) MUST be on `|text|` lines, not inline on `: Esc` lines, because reseq strips spaces on control lines. Example: `|0;My Title|` not `0 ; My Title` on a `: Esc ]` line.

## Section Dependency Graph

```
Section 01 (TeseqHarness & Infrastructure)
  └─► Section 02 (Basic Scenario Suite)
       ├─► Section 03 (Reports & Response Validation)
       ├─► Section 04 (Mode Interaction Scenarios)
       └─► Section 05 (SGR & Color Scenarios)
            └─► Section 06 (Complex Workflow Scenarios) [depends on 01-05]
                 └─► Section 07 (Verification & CI Integration) [depends on all]
```

- Section 01 is the foundation — all other sections depend on it.
- Sections 02-05 are largely independent (can be worked in listed order, but no hard cross-dependencies).
- Section 03 (reports) was moved before modes/SGR per Codex recommendation: it validates the event model established in Section 01.
- Section 06 (workflows) depends on all scenario sections (02-05) as it combines patterns from each.
- Section 07 (verification) depends on everything.

**Cross-section interactions:**
- **Section 01 + Section 03**: RecordedEvent enum (Section 01) must capture PtyWrite payloads verbatim for Section 03's `assert_pty_writes()` canonical byte assertions. If Section 01 only captures event types (not payloads), Section 03 cannot validate DA/DSR response content. Raw PtyWrite bytes are the oracle; teseq analysis is a supplementary debug aid.

## Implementation Sequence

```
Phase 0 - Infrastructure
  └─ 01: TeseqHarness, ScenarioSpec, RecordedEvent, reseq adapter, sidecar TOML parser
  Gate: `timeout 150 cargo test -p oriterm_core --test teseq` compiles and runs with one smoke scenario

Phase 1 - Basic Coverage
  └─ 02: C0 controls + basic CSI (cursor, erase, insert/delete) — validates harness end-to-end
  Gate: 20+ scenarios pass covering C0 + basic CSI

Phase 2 - Specialized Coverage
  └─ 03: Reports & outbound response validation (DA, DSR, DECRQM)
  └─ 04: Mode interaction scenarios (DECOM+scroll, DECCOLM, alt screen, IRM)
  └─ 05: SGR & color scenarios (16/256/TrueColor, attributes)
  Gate: 60+ total scenarios covering all major protocol families

Phase 3 - Integration  [CRITICAL PATH]
  └─ 06: Complex multi-sequence workflow scenarios
  Gate: Workflow scenarios exercise 3+ sequences in combination at multiple sizes

Phase 4 - Verification
  └─ 07: Test matrix, coverage gap analysis, CI integration, documentation
  Gate: Full test matrix documented, CI green, `timeout 150 ./test-all.sh` passes
```

**Why this order:**
- Phase 0 is pure infrastructure — no behavioral changes, no test expectations.
- Phase 1 validates the harness end-to-end with simple scenarios before building complex ones.
- Phase 2 sections are independent; ordering by complexity (reports depend on event model, modes are complex interactions, SGR is mostly data-driven).
- Phase 3 is the critical path — workflow scenarios are the highest-value deliverable, exercising multi-sequence interactions that no other test surface covers.

**Known failing tests (expected until plan completion):**
- None. This plan adds new tests without modifying existing code. All existing test suites remain green throughout.

## Metrics (Current State)

| Crate | Production LOC | Test LOC | Total |
|-------|---------------|----------|-------|
| `oriterm_core` | ~15,000 | ~10,500 | ~25,500 |
| **New (teseq tests)** | **0** | **~2,750 est.** | **~2,750 est.** |

Existing escape sequence test coverage:
- Handler unit tests: 5,860 lines (individual sequences)
- vttest integration: 1,459 lines + 198 snapshots (black-box PTY)
- GPU visual regression: 160+ golden PNGs (rendering)
- **Gap**: multi-sequence interactions, outbound response validation, human-readable scenario authoring

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 TeseqHarness & Infrastructure | ~500 | High | — |
|   ↳ 01.1 ScenarioSpec & TOML parser | ~120 | Medium | — |
|   ↳ 01.2 RecordedEvent & listener | ~80 | Low | — |
|   ↳ 01.3 Reseq subprocess adapter | ~60 | Medium | — |
|   ↳ 01.4 TeseqHarness runner | ~140 | High | 01.1-01.3 |
|   ↳ 01.5 Assertion helpers & snapshots | ~100 | Medium | 01.4 |
| 02 Basic Scenario Suite | ~550 | Medium | 01 |
| 03 Reports & Response Validation | ~300 | Medium | 01, 02 |
| 04 Mode Interaction Scenarios | ~350 | High | 01, 02 |
| 05 SGR & Color Scenarios | ~300 | Medium | 01, 02 |
| 06 Complex Workflow Scenarios | ~400 | High | 01-05 |
| 07 Verification & CI Integration | ~250 | Low | 01-06 |
| **Total new** | **~2,750** | | |
| **Total deleted** | **~0** | | |

## Known Bugs (Pre-existing)

None discovered during research that directly affect this plan. Existing test infrastructure is stable and well-tested.

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | TeseqHarness & Infrastructure | `section-01-infrastructure.md` | Complete |
| 02 | Basic Scenario Suite | `section-02-basic-scenarios.md` | Complete |
| 03 | Reports & Response Validation | `section-03-reports.md` | Complete |
| 04 | Mode Interaction Scenarios | `section-04-mode-interactions.md` | Not Started |
| 05 | SGR & Color Scenarios | `section-05-sgr-colors.md` | Not Started |
| 06 | Complex Workflow Scenarios | `section-06-workflows.md` | Not Started |
| 07 | Verification & CI Integration | `section-07-verification.md` | Not Started |
