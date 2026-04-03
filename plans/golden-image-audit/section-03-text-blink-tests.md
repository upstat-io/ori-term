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
**Goal:** Add a single cross-frame test that renders at 3 opacity levels in one test function and asserts that non-BLINK cell brightness is constant across frames while BLINK cell brightness changes. This is the one gap in the existing test suite.

<!-- reviewed: architecture fix — original plan mischaracterized existing tests as "single static frame" snapshots.
In reality, the three existing tests (text_blink_visible, text_blink_hidden, text_blink_half) already:
  - Render at three different text_blink_opacity values (1.0, 0.5, 0.0)
  - Assert behavioral pixel brightness properties (not just golden image comparison)
  - Verify BLINK cells are dimmer/invisible at lower opacity
  - Verify non-BLINK cells remain visible at opacity 0.0
The only missing assertion is CROSS-FRAME: that the same non-BLINK cell has identical
brightness across all three opacity values. Each existing test checks within its own frame
but no test compares across frames. This is a narrow gap, not a methodology failure. -->

**Context:** The existing `text_blink_visible`, `text_blink_hidden`, and `text_blink_half` tests at `oriterm/src/gpu/visual_regression/text_blink_tests.rs` already render at three different `text_blink_opacity` values (1.0, 0.5, 0.0) and assert pixel brightness properties per frame. They are NOT "single static frame" snapshots -- each already proves behavioral correctness at its opacity level. However, since they run as independent tests, no assertion compares the non-BLINK cell across frames to prove it stays constant. A cross-frame test fills this gap.

<!-- reviewed: architecture fix — removed incorrect reference to cursor_opacity_tests.rs.
render_to_pixels_with_opacity() controls CURSOR opacity (passed to renderer.prepare() as
the cursor_opacity parameter). Text blink uses input.text_blink_opacity, read by the
prepare pipeline (fill_frame_shaped, fill_frame_incremental, fill_frame at unshaped.rs).
The existing text blink tests correctly use render_to_pixels() and set text_blink_opacity
directly on FrameInput. The cursor_opacity_tests pattern does NOT apply here. -->

<!-- reviewed: accuracy verified against codebase 2026-04-02 -->
**Reference implementations:**
- **Existing text blink tests** `text_blink_tests.rs`: Uses `blink_input(cell, opacity)` to build a `FrameInput` with `text_blink_opacity` set directly, then renders via `render_to_pixels()`. The `cell_pixel(pixels, width, col, cell_w, cell_h)` helper extracts an `[u8; 4]` RGBA value at the center of a cell column. Constants `BLINK_COL = 0` and `NORMAL_COL = 5` identify the two test cells. All three tests share this infrastructure. The new cross-frame test reuses these exact helpers and constants.
- **Multi-render precedent**: `core_tests.rs::cursor_shapes` renders 4 frames in one test via a loop over `render_to_pixels()`. `multi_size.rs` does the same across font sizes. Calling `render_to_pixels()` multiple times with the same `(GpuState, GpuPipelines, &mut WindowRenderer)` is a proven, safe pattern.

**Depends on:** None (text blink rendering from 05B already works; existing tests already pass).

---

## 03.1 Add Cross-Frame Consistency Test

<!-- reviewed: architecture fix — changed from "replace three tests with one" to "add one test".
The three existing tests are well-structured, individually runnable, each with golden image
validation AND behavioral assertions. Deleting them loses test granularity for no gain.
The only gap is a cross-frame comparison — add it as a fourth test. -->

**File(s):** `oriterm/src/gpu/visual_regression/text_blink_tests.rs`

<!-- reviewed: accuracy verified against codebase 2026-04-02. All helpers, constants, return types,
     and assertion patterns confirmed by reading text_blink_tests.rs, mod.rs, and frame_input/mod.rs. -->
Add a single test that renders 3 frames in one test function to make cross-frame assertions:

<!-- reviewed: completeness/hygiene fix — added variable naming, fresh-input-per-frame note,
     CellMetrics is Copy clarification, assertion message requirements, and edge case notes -->
- [ ] Write `text_blink_cross_frame_consistency` test function in `text_blink_tests.rs` that:
  1. Uses the existing imports: `headless_env`, `render_to_pixels` from `super::`, plus `blink_input` and `cell_pixel` from this module. No new imports needed.
  2. Calls `headless_env()` once — returns `Option<(GpuState, GpuPipelines, WindowRenderer)>`. Use the standard early-return guard: `let Some((gpu, pipelines, mut renderer)) = headless_env() else { eprintln!("skipped: no GPU adapter available"); return; };`
  3. Gets cell metrics: `let cell = renderer.cell_metrics();` — `CellMetrics` is `Copy`, so `cell` can be passed to all 3 `blink_input()` calls without issues.
  4. Renders 3 frames using **3 separate calls** to `blink_input(cell, opacity)` (which creates a fresh `FrameInput` each time), then `render_to_pixels(&gpu, &pipelines, &mut renderer, &input)`. Use named variables for clarity:
     ```rust
     let input_1_0 = blink_input(cell, 1.0);
     let pixels_1_0 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_1_0);
     let input_0_5 = blink_input(cell, 0.5);
     let pixels_0_5 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_0_5);
     let input_0_0 = blink_input(cell, 0.0);
     let pixels_0_0 = render_to_pixels(&gpu, &pipelines, &mut renderer, &input_0_0);
     ```
     Each call returns `Vec<u8>` of RGBA pixel data. (NOT `render_to_pixels_with_opacity()` — that controls cursor opacity, not text blink.)
  5. For each frame, extracts the non-BLINK cell pixel via `cell_pixel(&pixels, input.viewport.width, NORMAL_COL, cell.width, cell.height)` — returns `[u8; 4]` (RGBA). All 3 inputs have the same viewport width (from `blink_input` with the same `cell`), so any `input_*.viewport.width` works.
  6. Asserts **non-BLINK cell RGB brightness is constant across all 3 frames**: compare `normal_1_0[0..3]` vs `normal_0_5[0..3]` vs `normal_0_0[0..3]`. Use per-channel absolute difference sum `< 5` between each pair (3 comparisons: 1.0 vs 0.5, 0.5 vs 0.0, 1.0 vs 0.0). Since `text_blink_opacity` does not affect non-BLINK cells (verified in prepare pipeline: `fg_dim * text_blink_opacity` only applies when `is_blink`), the pixels should be identical — the tolerance of 5 is a safety margin for GPU driver variance. **This is the assertion no existing test makes.** Each `assert!` must include a descriptive message showing the actual pixel values (matching existing test style).
  7. Also extracts the BLINK cell pixel at `BLINK_COL` from each frame and computes brightness as `r as u32 + g as u32 + b as u32` (matching existing pattern in `text_blink_half`). Asserts **BLINK brightness at 1.0 > BLINK brightness at 0.5 > BLINK brightness at 0.0** (strict monotonic decrease via `>`). This is a tighter cross-frame constraint than the per-frame assertions in the individual tests. Each `assert!` must include the actual brightness values in the failure message.
  8. No `compare_with_reference()` call — this is purely a behavioral assertion test, not a golden image test. No new golden images needed. The existing 3 golden images (`text_blink_visible.png`, `text_blink_hidden.png`, `text_blink_half.png`) remain validated by the existing 3 tests.

**Edge cases already covered by design:**
- **`headless_env()` returns `None`**: Step 2's early-return guard handles this (skips with message, does not fail). This matches all existing GPU tests.
- **RAPID_BLINK vs SLOW_BLINK**: Both SGR 5 and SGR 6 map to the same `CellFlags::BLINK` flag (`cell/mod.rs:23`). There is no separate rapid blink flag. The test covers both via the single flag.
- **`text_blink_opacity` vs `cursor_opacity`**: These are independent pipelines. `text_blink_opacity` modulates `fg_dim` for BLINK cells in the prepare pass. `cursor_opacity` (the param to `render_to_pixels_with_opacity`) controls cursor alpha. The test uses `render_to_pixels()` which passes `cursor_opacity: 1.0` — cursor is hidden anyway via `input.content.cursor.visible = false` in `blink_input`.
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green

**Keep existing tests:** `text_blink_visible`, `text_blink_hidden`, `text_blink_half` remain as-is. They provide golden image regression protection at each opacity level plus per-frame behavioral assertions. Do not delete them.

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

<!-- reviewed: completeness/hygiene fix — added assertion message requirement and pair-count check -->
- [ ] `text_blink_cross_frame_consistency` test added to `text_blink_tests.rs`
- [ ] Test calls `headless_env()` once and renders 3 frames via `render_to_pixels()` at opacity 1.0, 0.5, 0.0
- [ ] Test uses existing `blink_input()`, `cell_pixel()`, `BLINK_COL`, `NORMAL_COL` — no new helpers
- [ ] Each `blink_input()` call creates a fresh `FrameInput` (not mutating a shared one)
- [ ] Test asserts non-BLINK cell RGB brightness is constant across all 3 pairs (1.0 vs 0.5, 0.5 vs 0.0, 1.0 vs 0.0) — sum-of-channels diff < 5
- [ ] Test asserts BLINK cell brightness decreases monotonically (1.0 > 0.5 > 0.0) via strict `>`
- [ ] All `assert!` messages include actual pixel/brightness values (matching existing test style)
- [ ] No `compare_with_reference()` call — no new golden images
- [ ] Existing 3 tests (`text_blink_visible`, `text_blink_hidden`, `text_blink_half`) unchanged
- [ ] Existing 3 golden images unchanged
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** The text blink test suite proves both per-frame correctness (existing tests) AND cross-frame consistency (new test): BLINK cells change brightness while non-BLINK cells remain constant across opacity values.
