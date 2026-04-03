---
section: "03"
title: "Text Blink Cross-Frame Consistency Assertion"
status: not-started
reviewed: true
goal: "Add a cross-frame assertion proving non-BLINK cell brightness is constant while BLINK cells change"
inspired_by:
  - "Existing text blink tests (oriterm/src/gpu/visual_regression/text_blink_tests.rs)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Add Cross-Frame Consistency Test"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Text Blink Cross-Frame Consistency Assertion

**Status:** Not Started
**Goal:** Add a single cross-frame test that renders at 3 opacity levels in one test function and asserts that non-BLINK cell brightness is constant across frames while BLINK cell brightness changes.

**Context:** The existing `text_blink_visible`, `text_blink_hidden`, and `text_blink_half` tests already render at three `text_blink_opacity` values (1.0, 0.5, 0.0) and assert pixel brightness properties per frame. Each proves behavioral correctness at its opacity level. However, since they run as independent tests, no assertion compares the non-BLINK cell across frames to prove it stays constant. A cross-frame test fills this narrow gap.

**Reference implementations:**
- **Existing text blink tests** `text_blink_tests.rs`: `blink_input(cell, opacity)` builds a `FrameInput` with `text_blink_opacity` set directly, rendered via `render_to_pixels()`. `cell_pixel(pixels, width, col, cell_w, cell_h)` extracts `[u8; 4]` RGBA at the center of a cell column. Constants `BLINK_COL = 0` and `NORMAL_COL = 5` identify the two test cells.
- **Multi-render precedent**: `core_tests.rs::cursor_shapes` renders 4 frames in one test via a loop over `render_to_pixels()`. Calling `render_to_pixels()` multiple times with the same `(GpuState, GpuPipelines, &mut WindowRenderer)` is proven safe.

**Depends on:** None (text blink rendering already works; existing tests pass).

---

## 03.1 Add Cross-Frame Consistency Test

**File:** `oriterm/src/gpu/visual_regression/text_blink_tests.rs`

Add one test that renders 3 frames in one function to make cross-frame assertions:

- [ ] Write `text_blink_cross_frame_consistency` test function that:
  1. No new imports needed -- uses `headless_env`, `render_to_pixels` from `super::`, plus `blink_input` and `cell_pixel` from this module.
  2. Calls `headless_env()` once with the standard early-return guard: `let Some((gpu, pipelines, mut renderer)) = headless_env() else { eprintln!("skipped: no GPU adapter available"); return; };`
  3. Gets cell metrics: `let cell = renderer.cell_metrics();` (`CellMetrics` is `Copy`).
  4. Renders 3 frames via separate `blink_input(cell, opacity)` + `render_to_pixels()` calls:
     ```rust
     let input_1_0 = blink_input(cell, 1.0);
     let pixels_1_0 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_1_0);
     let input_0_5 = blink_input(cell, 0.5);
     let pixels_0_5 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_0_5);
     let input_0_0 = blink_input(cell, 0.0);
     let pixels_0_0 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_0_0);
     ```
     Uses `render_to_pixels()` (not `render_to_pixels_with_opacity()` which controls cursor opacity).
  5. Extracts non-BLINK cell pixel from each frame via `cell_pixel(&pixels, input.viewport.width, NORMAL_COL, cell.width, cell.height)`.
  6. Asserts **non-BLINK cell RGB is constant across all 3 frames**: per-channel absolute difference sum `< 5` between each pair (3 comparisons). The prepare pipeline applies `fg_dim * text_blink_opacity` only when `is_blink`, so non-BLINK pixels should be identical -- tolerance is a safety margin for GPU driver variance. **This is the assertion no existing test makes.** Each `assert!` includes actual pixel values in the failure message.
  7. Extracts BLINK cell pixel at `BLINK_COL` from each frame, computes brightness as `r as u32 + g as u32 + b as u32`. Asserts **BLINK brightness at 1.0 > 0.5 > 0.0** (strict `>`). Each `assert!` includes actual brightness values.
  8. No `compare_with_reference()` call -- purely behavioral, no new golden images.
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green

**Keep existing tests:** `text_blink_visible`, `text_blink_hidden`, `text_blink_half` remain as-is (golden image regression + per-frame behavioral assertions).

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] `text_blink_cross_frame_consistency` test added to `text_blink_tests.rs`
- [ ] Renders 3 frames at opacity 1.0, 0.5, 0.0 via `render_to_pixels()`
- [ ] Uses existing helpers only (`blink_input`, `cell_pixel`, `BLINK_COL`, `NORMAL_COL`)
- [ ] Non-BLINK RGB constant across all 3 pairs (sum-of-channels diff < 5)
- [ ] BLINK brightness monotonically decreasing (1.0 > 0.5 > 0.0, strict `>`)
- [ ] All `assert!` messages include actual values
- [ ] No new golden images; existing 3 tests and images unchanged
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Text blink test suite proves both per-frame correctness (existing tests) AND cross-frame consistency (new test): BLINK cells change brightness while non-BLINK cells remain constant across opacity values.
