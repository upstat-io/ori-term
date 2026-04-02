---
plan: "vttest-conformance"
title: "VTTest Conformance: Exhaustive Implementation Plan"
status: not-started
references:
  - "oriterm_core/tests/vttest.rs"
  - "oriterm/src/gpu/visual_regression/vttest.rs"
  - "~/projects/reference_repos/console_repos/wezterm/wezterm-gui/src/colorease.rs"
---

# VTTest Conformance: Exhaustive Implementation Plan

## Mission

Pass 90% of vttest screens across menus 1-3, 5-6, 8 at all terminal sizes (80x24, 97x33, 120x40), backed by automated structural assertions and GPU-rendered golden image tests. Menu 4 (double-size characters: DECDHL/DECDWL) and menu 7 (VT52 mode) are excluded from the pass rate — these features are not implemented. Implement WezTerm-style smooth fade cursor blink with easing, passing opacity through the existing instance alpha pipeline, verified by multi-frame capture tests. Make oriterm the most conformant terminal emulator against vttest — surpassing Alacritty, Ghostty, and matching or exceeding WezTerm on implemented features.

## Architecture

```
vttest (real PTY)
  │
  ▼
VTE Parser (vte crate) ──► Term<PtyResponder> (oriterm_core)
  │                              │
  │  DA/DSR responses ◄──────────┘
  │
  ▼
Grid State (cells, cursor, scroll region, modes)
  │
  ├──► Text Grid Assertions (oriterm_core/tests/vttest.rs)
  │      • Structural: border fills terminal, origin mode matches normal
  │      • Snapshots: insta text snapshots at 3 sizes
  │
  └──► GPU Golden Images (oriterm/src/gpu/visual_regression/vttest.rs)
         • Full pipeline: Term → RenderableContent → FrameInput → GPU → pixels
         • Per-pixel fuzzy comparison against reference PNGs
         • Captures: colors, bold, underline, reverse, line drawing, blink

Cursor Blink Pipeline:
  CursorBlink (oriterm_ui) ──► ColorEase (new) ──► build_cursor() alpha param ──► Instance Buffer
     epoch + interval           easing curves        opacity (0.0-1.0)           push_cursor(alpha)
```

## Design Principles

1. **Fix the emulation, not the tests.** When vttest fails, the bug is in oriterm's VTE handler — never adjust test expectations to match broken behavior. The test infrastructure captures current state (including bugs); fixing the handler makes tests pass.

2. **Structural assertions over visual inspection.** Golden images are for catching unexpected regressions. Structural assertions (border fills terminal, screens match across modes) are the load-bearing tests — they fail automatically without human review.

3. **Incremental conformance.** Each section targets specific vttest menus and failure modes. Fixes land one at a time with tests proving each fix. No big-bang "fix everything" section.

## Section Dependency Graph

```
Section 01 (Terminal Size Reporting)
  │
  ├──► Section 02 (Origin Mode & Scroll Regions)
  │      │
  │      ├──► Section 03 (Screen Features & DECCOLM)
  │      │
  │      └──► Section 04 (Character Sets & VT102)
  │
  Section 05 (Fade Blink) ◄── independent, no VTE dependency
  │
  Section 06 (Test Automation Expansion) ◄── depends on 01, 02, 03, 04
  │
  └──► Section 07 (Verification & Metrics)
```

- Section 01 is the foundation — terminal size reporting blocks everything at non-80-column sizes.
- Sections 02-04 are the VTE fix sections — each targets specific vttest menus.
- Section 05 (blink) is fully independent of VTE fixes — it uses the existing headless GPU pipeline for multi-frame capture, not vttest infrastructure.
- Section 06 expands test coverage to all menus; depends on Sections 01-04 so tests capture correct behavior.
- Section 07 is the final verification pass.

**Cross-section interactions:**
- **Section 01 + 02**: Terminal size fix unblocks origin mode testing at non-80-column sizes.
- **Section 05**: Self-contained. Blink tests use the existing `headless_env()` GPU pipeline and `FrameInput::test_grid()` — no vttest or PTY involvement.

## Implementation Sequence

```
Phase 1 - Foundation
  └─ 01: Fix terminal size reporting (CSI 18t, DA responses, PTY size)
  Gate: vttest_border_fills_* passes at all 3 sizes

Phase 2 - Core VTE Fixes
  ├─ 02: Origin mode, scroll regions, DECALN interactions
  ├─ 03: Screen features (DECCOLM reflow, wrap, tab stops, 132-col mode)
  └─ 04: Character sets, VT102 features (ICH/DCH/IL/DL)
  Gate: 90% of menus 1-3, 8 pass at 80x24 (menu 4 DECDHL/DECDWL excluded — not implemented)

Phase 3 - Cursor Blink
  └─ 05: WezTerm-style fade blink (ColorEase, instance alpha, multi-frame tests)
  Gate: Blink fade visually smooth, multi-frame capture shows opacity ramp

Phase 4 - Test Expansion
  └─ 06: Automate menus 5-8, add structural assertions for each
  Gate: All 8 menus automated with assertions + golden images

Phase 5 - Verification
  └─ 07: Full conformance audit, metrics, cleanup
  Gate: 90% pass rate across menus 1-3, 5-6, 8 at all 3 sizes (excl. menu 4 DECDHL, menu 7 VT52)
```

**Why this order:**
- Phase 1 unblocks all non-80-column testing (currently broken).
- Phase 2 fixes the specific VTE bugs vttest exposes.
- Phase 3 is fully independent of VTE fixes — can be interleaved or done in any order relative to Phase 2.
- Phase 4 requires phases 1-2 so tests capture correct behavior.

**Known failing tests (expected until plan completion):**
- **`vttest_border_fills_97x33`** — border stops at col 80. Root cause: Phase 1 (terminal size reporting).
- **`vttest_origin_mode_matches_normal_80x24`** — origin mode cursor positioning. Root cause: Phase 2.

## Metrics (Current State)

| Crate | Production LOC | Test LOC | Total |
|-------|---------------|----------|-------|
| `oriterm_core` (term handler) | ~2,800 | ~5,400 | ~8,200 |
| `oriterm_core` (grid) | ~1,600 | ~1,200 | ~2,800 |
| `oriterm` (gpu/visual_regression) | ~1,800 | (included) | ~1,800 |
| `oriterm_ui` (cursor_blink) | ~90 | ~143 | ~233 |
| **vttest tests (new)** | — | ~900 | ~900 |

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 Terminal Size Reporting | ~100 | Medium | — |
| 02 Origin Mode & Scroll Regions | ~200 | High | 01 |
| 03 Screen Features & DECCOLM | ~300 | High | 02 |
| 04 Character Sets & VT102 | ~200 | Medium | 02 |
| 05 Fade Blink | ~400 | High | — |
| 06 Test Automation Expansion | ~600 | Medium | 01, 02, 03, 04 |
| 07 Verification | ~100 | Low | All |
| **Total new** | **~1,900** | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| Border doesn't fill terminal at non-80-col sizes | DA1 response (`\x1b[?6;4c`) doesn't indicate VT220+ class (needs `62` prefix), so vttest falls back to 80x24 | Section 01 | Not Started |
| Origin mode (01_02) garbled at all sizes | `goto_origin_aware` cursor offset incorrect with scroll regions | Section 02 | Not Started |
| DECCOLM ignored (132-col mode) | `ColumnMode` stub logs debug and does nothing | Section 03 | Not Started |
| No cursor fade blink | `CursorBlink` is abrupt on/off binary toggle, no easing curve | Section 05 | Not Started |

## Cross-Cutting Risks

1. **DA1 response change (Section 01) affects all menus.** Changing from `\x1b[?6;4c` to `\x1b[?62;6;4c` (VT220 class) may cause vttest to enable VT200+ features that were previously inactive. This could surface NEW failures in menus 2-8 that were not visible when vttest treated oriterm as a VT100. Run all menus after the DA1 fix, before proceeding to sections 02-04.

2. **Section 05 touches 15+ files across 2 crates.** The bool-to-f32 migration (`cursor_blink_visible` -> `cursor_opacity`) is mechanically simple but touches the hottest code paths (prepare pipeline, redraw logic, event loop scheduling). A single missed call site causes a type error (good — compiler catches it), but a semantic error (wrong threshold, wrong opacity math) produces subtle visual bugs. The `/tpr-review` checkpoint in 05.2 is critical.

3. **VtTestSession duplication.** The `VtTestSession` + `PtyResponder` types are defined independently in both `oriterm_core/tests/vttest.rs` and `oriterm/src/gpu/visual_regression/vttest.rs`. When adding menu navigation in Section 06, the duplication must be maintained in both files. If either diverges (e.g., different `drain()` timing), tests will behave differently. Consider adding a comment cross-referencing the two implementations.

4. **vttest timing sensitivity.** vttest uses sleep-based synchronization. The `drain()` loop's `thread::sleep(Duration::from_millis(200))` may be insufficient on slow CI machines. If flaky tests appear, increase to 500ms with a `VTTEST_DRAIN_MS` env override.

5. **DSR cursor position in DECOM mode (Sections 02 + 06).** The current DSR 6 handler at `status.rs:86` always reports absolute screen coordinates. Per DEC spec, when DECOM is active, DSR 6 should report the cursor position relative to the scroll region origin. vttest menu 6 may test this. If the Section 02 origin mode fixes don't also fix DSR 6, menu 6 structural assertions will fail. The fix belongs in Section 02 alongside the other DECOM fixes.

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Terminal Size Reporting | `section-01-terminal-size.md` | Not Started |
| 02 | Origin Mode & Scroll Regions | `section-02-origin-mode.md` | Not Started |
| 03 | Screen Features & DECCOLM | `section-03-screen-features.md` | Not Started |
| 04 | Character Sets & VT102 | `section-04-charsets-vt102.md` | Not Started |
| 05 | Fade Blink | `section-05-fade-blink.md` | Not Started |
| 06 | Test Automation Expansion | `section-06-test-expansion.md` | Not Started |
| 07 | Verification & Metrics | `section-07-verification.md` | Not Started |
