---
plan: "golden-image-audit"
title: "Golden Image Audit & Test Methodology Fix"
status: not-started
references:
  - "oriterm/src/gpu/visual_regression/"
  - "oriterm/tests/references/"
  - "~/projects/reference_repos/console_repos/wezterm/term/src/terminalstate/mod.rs"
  - "~/projects/reference_repos/console_repos/alacritty/alacritty_terminal/src/term/mod.rs"
---

# Golden Image Audit & Test Methodology Fix

## Mission

Fix the broken test methodology that rubber-stamps incorrect rendering. 15 golden reference images validate broken behavior as "correct" (DECSCNM not rendering, VT102 scroll region bugs, untested blink animation). Implement the missing rendering features, replace the broken golden images, and add multi-frame blink verification so that GPU visual regression tests actually catch rendering bugs.

## Architecture

```
VTE Parser (crates/vte)
  │  CSI ? 5 h/l → NamedPrivateMode::ReverseVideo  [MISSING — add]
  ▼
Term Handler (oriterm_core/src/term/handler/modes.rs)
  │  apply_decset/decrst → TermMode::REVERSE_VIDEO  [MISSING — add]
  ▼
RenderableContent.mode (already carries TermMode)
  │
  ▼
GPU Extract (oriterm/src/gpu/extract/)
  │  snapshot_palette() → swap fg/bg when REVERSE_VIDEO  [MISSING — add]
  ▼
GPU Prepare (oriterm/src/gpu/prepare/)
  │  resolve_cell_colors() → account for DECSCNM  [MISSING — add]
  │  set_clear_color() → use fg when reversed  [MISSING — add]
  ▼
Golden Images (oriterm/tests/references/)
  │  12 DECSCNM images to re-render
  │  5 VT102 images to re-render after scroll fix (screens 08-12)
  │  3 text_blink images to replace with multi-frame tests
  ▼
compare_with_reference() — validates correctness
```

## Design Principles

1. **Fix the emulation, not the tests.** When a golden image shows wrong output, the bug is in the terminal emulation or GPU renderer — never adjust test expectations to match broken behavior. Fix the rendering, then regenerate the golden image.

2. **Tests must prove behavior, not freeze state.** A single-frame golden image of "blinking text at opacity 1.0" proves nothing about blinking. Multi-frame capture must show opacity changing over time.

3. **Visual inspection is mandatory for golden images.** Every golden image generated or regenerated MUST be visually inspected by reading the PNG. "Test passes" is not sufficient — the reference itself could be wrong.

## Section Dependency Graph

```
Section 01 (DECSCNM Rendering)
  │
  └──► Section 04 (Revalidation — re-render DECSCNM golden images)

Section 02 (VT102 Scroll Region Fix)
  │
  └──► Section 04 (Revalidation — re-render VT102 golden images)

Section 03 (Text Blink Multi-Frame) ◄── independent

Section 04 (Golden Image Revalidation) ◄── depends on 01 + 02
```

- Sections 01, 02, 03 are independent of each other.
- Section 04 depends on 01 and 02 (fixes must land before re-rendering).

## Implementation Sequence

```
Phase 1 - Rendering Fixes (parallel)
  ├─ 01: DECSCNM reverse video (VTE → TermMode → handler → renderer)
  └─ 02: VT102 IL/DL scroll region boundary fix
  Gate: vttest light-background screens show white bg; VT102 08-12 show correct top-line content

Phase 2 - Test Methodology Fix
  └─ 03: Text blink multi-frame verification
  Gate: Multi-frame captures prove opacity changes over time

Phase 3 - Revalidation
  └─ 04: Re-render ALL affected golden images, visual inspection
  Gate: Every golden image visually verified correct; ./test-all.sh green
```

**Why this order:**
- Phases 1 rendering fixes must land before Phase 3 regenerates golden images.
- Phase 2 (text blink tests) is independent and can run in parallel with Phase 1.
- Phase 3 is the final sweep that regenerates all affected golden images and visually verifies them.

## Known Bugs (Found During Audit)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| DECSCNM not rendering (12 images) | Mode 5 not recognized in VTE parser; no TermMode flag; no renderer logic | Section 01 | Not Started |
| VT102 IL/DL wrong with scroll regions (5 screens: 08-12) | Off-by-one or boundary error in `grid/scroll/mod.rs` IL/DL when DECSTBM active | Section 02 | Not Started |
| inverse_video.png may be wrong | SGR 7 works in vttest but test setup may not apply it correctly | Section 04 | Not Started |
| Text blink tests are single-frame | No multi-frame capture to prove opacity animation | Section 03 | Not Started |

## Metrics (Current State)

| Area | Broken Images | Total Images |
|------|--------------|-------------|
| DECSCNM (vttest 02_03, 02_04, 02_14) | 9 (3 resolutions x 3 screens) | — |
| VT102 scroll region (08_vt102_08-12) | 5 | — |
| Text blink (methodology, not rendering) | 3 | — |
| inverse_video.png (needs investigation) | 1 | — |
| **Total broken** | **17-18** | **131** |

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 DECSCNM Rendering | ~100 | Medium | — |
| 02 VT102 Scroll Region | ~100 | High | — |
| 03 Text Blink Multi-Frame | ~150 | Medium | — |
| 04 Golden Image Revalidation | ~20 | Low | 01, 02 |
| **Total new** | **~370** | | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | DECSCNM Reverse Video Rendering | `section-01-decscnm.md` | Complete |
| 02 | VT102 Insert/Delete Line with Scroll Regions | `section-02-vt102-scroll.md` | Not Started |
| 03 | Text Blink Multi-Frame Verification | `section-03-text-blink-tests.md` | Not Started |
| 04 | Golden Image Revalidation | `section-04-revalidation.md` | Not Started |
