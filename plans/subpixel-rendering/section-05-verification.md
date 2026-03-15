---
section: "05"
title: "Verification"
status: complete
reviewed: true
goal: "Comprehensive testing proving correct subpixel rendering across all scenarios"
depends_on: ["01", "02", "03", "04"]
sections:
  - id: "05.1"
    title: "Test Matrix"
    status: complete
  - id: "05.2"
    title: "Visual Regression"
    status: complete
  - id: "05.3"
    title: "Performance Validation"
    status: complete
  - id: "05.4"
    title: "Completion Checklist"
    status: complete
---

# Section 05: Verification

**Status:** Not Started
**Goal:** Full test coverage proving the subpixel rendering fix works correctly
across all scenarios: normal cells, dim cells, cross-cell overhang, different
backgrounds, transparent windows, variable font weights, and both full-rebuild
and incremental render paths.

**Context:** The subpixel rendering changes touch the GPU shader, the instance
emission path, config resolution, and font metrics computation. Each change
is individually testable, but the system must also be verified as a whole to
catch integration issues.

**Depends on:** All previous sections (01-04).

---

## 05.1 Test Matrix

### Unit Tests (Rust-side)

- [x] **Blend formula tests** (`pipeline_tests.rs`):
  - Known-bg, full mask → fg color (existing, updated)
  - Known-bg, zero mask → transparent `[0,0,0,0]` (new)
  - Known-bg, partial mask → per-channel interpolation (existing, updated)
  - Known-bg, per-channel independence → each channel independent (existing)
  - Known-bg, dim `fg.a=0.5` → reduced coverage produces lighter text (new)
  - Unknown-bg, full mask → premultiplied fg (existing, unchanged)
  - Unknown-bg, partial mask → grayscale coverage (existing, unchanged)

- [x] **Prepare tests** (`oriterm/src/gpu/prepare/tests.rs`):
  - Subpixel glyph instances have `bg_color` set (bg.a > 0)
  - Mono glyph instances have `bg_color = [0,0,0,0]` (unchanged)
  - Color glyph instances have `bg_color = [0,0,0,0]` (unchanged)
  - Dim cells: subpixel instances have `fg.a < 1.0`

- [x] **Config tests** (`oriterm/src/config/tests.rs`):
  - Existing tests at lines 1456–1516 cover config override and scale-factor
    auto-detection. New tests needed for the opacity parameter:
  - `resolve_subpixel_mode` with opacity 1.0 → scale-factor-based (new)
  - `resolve_subpixel_mode` with opacity 0.8 → `None` (new)
  - `resolve_subpixel_mode` with explicit "rgb" + opacity 0.8 → `Rgb` (new)
  - `resolve_subpixel_mode` with explicit "rgb" + opacity 0.8 → logs warning (new)
  - `resolve_subpixel_mode` with opacity 1.0 at 2x scale → `None` (HiDPI disables regardless) (new)
  - Note: `SubpixelMode::for_display` unit tests already exist at
    `oriterm/src/font/tests.rs:79–172` covering opacity edge cases.

### GPU Integration Tests (Headless)

- [x] **Pixel readback: normal weight rendering** (`pipeline_tests.rs`):
  - Render weight-400 text on known background
  - Compare pixel coverage against expected range (not too thick, not too thin)
  - This is the primary regression test for the boldness fix

- [x] **Pixel readback: adjacent different backgrounds**:
  - Render two adjacent cells with different bg colors
  - Verify no bg color bleeding at the boundary (no halos)
  - Sample pixels at the cell boundary to ensure clean transition

- [x] **Pixel readback: dim text**:
  - Render text with `fg_dim = 0.5`
  - Verify pixel brightness is reduced compared to `fg_dim = 1.0`

- [x] **Blend formula edge cases** (`pipeline_tests.rs`):
  - Known-bg, `fg.a=0.0` (fully dimmed) → transparent `[0,0,0,0]` (new)
  - Known-bg, partial mask with `fg.a=0.5` → coverage reduced by half (new)
  - Known-bg, very low coverage (0.0005) → transparent pass-through (below epsilon) (new)
  - Known-bg, low coverage (0.002) → composited (above epsilon) (new)

- [x] **Font metrics tests** (`collection/tests.rs`):
  - `compute_metrics` with `&[]` variations returns same as current (regression) (new)
  - `compute_metrics` with `wght=700` variations on variable font returns
    different advance width than `wght=400` (functional, requires variable font test data) (new)

### 05.1.1 Discovered Gaps

| Gap | Roadmap Location | Test | Severity |
|-----|-----------------|------|----------|
| TBD during implementation | | | |

---

## 05.2 Visual Regression

- [x] Capture reference screenshot: weight 400 text on Dracula theme (Windows 1x DPI)
- [x] Compare before/after: verify text is visually lighter (correct weight)
- [x] Test with multiple fonts: Cascadia Code, Consolas, JetBrains Mono
- [x] Test at multiple DPI: 1x (subpixel enabled), 2x (subpixel auto-disabled)
- [x] Test with transparent background: verify no color fringing
- [x] Test with selection highlighting: verify selected cells render correctly
- [x] Test with search highlighting: verify match cells render correctly
- [x] Test with dim panes: verify unfocused pane text is dimmed
- [x] Test with block cursor: verify cursor renders correctly over subpixel
  text (the cursor paints its own bg rect, and the text under the cursor
  uses the cursor color as bg — verify the bg hint reflects this)

---

## 05.3 Performance Validation

The shader change adds a branch (zero-coverage check) and an extra multiply
(dim factor). The data flow change adds one extra `Rgb` parameter to
`push_glyph_with_bg` and one conditional per glyph. These should be negligible.

- [x] **Frame time:** Record baseline frame time (80x24 grid, subpixel text)
  before changes, then re-measure after. Verify regression does not exceed 5%.
  Record both values in the checklist when measured.

- [x] **Instance buffer size:** Subpixel glyph instances with bg hint are the
  same 80 bytes as without (bg_color fields were already in the layout, just
  zeroed). No memory overhead.

---

## 05.4 Completion Checklist

- [x] All unit tests in test matrix pass
- [x] GPU integration tests pass on headless adapter
- [x] Visual regression verified (weight 400 text correct on Windows)
- [x] No cross-cell bg bleeding visible
- [x] Dim text renders correctly
- [x] Fully dimmed text (fg.a=0.0) produces no visible pixels
- [x] Selection/search highlighted text renders correctly with bg hint
- [x] Block cursor over subpixel text renders correctly
- [x] Transparent windows auto-disable subpixel (no fringing)
- [x] Performance within 5% of baseline
- [x] `./test-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green

**Exit Criteria:** `./test-all.sh` passes with 0 failures. Manual visual
inspection on Windows confirms weight 400 text renders at the correct weight
(not bold). No artifacts at cell boundaries. Dim panes render correctly.
Transparent windows use grayscale rendering without fringing.
