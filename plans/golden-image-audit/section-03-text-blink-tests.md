---
section: "03"
title: "Text Blink Multi-Frame Verification"
status: not-started
reviewed: false
goal: "Text blink tests capture multiple frames showing opacity changes over time, proving animation works"
inspired_by:
  - "Existing cursor opacity tests (oriterm/src/gpu/visual_regression/cursor_opacity_tests.rs)"
  - "Section 05 cursor blink multi-frame pattern (plans/vttest-conformance/section-05-fade-blink.md)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Multi-Frame Text Blink Test"
    status: not-started
  - id: "03.2"
    title: "Replace Single-Frame Tests"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Text Blink Multi-Frame Verification

**Status:** Not Started
**Goal:** Text blink GPU tests capture multiple frames at different `text_blink_opacity` values and verify that BLINK cells change brightness across frames while non-BLINK cells remain constant.

**Context:** The current `text_blink_visible`, `text_blink_hidden`, and `text_blink_half` tests each capture a single static frame at a fixed opacity value. They prove that the alpha pipeline works at three specific values, but they do NOT prove that blinking actually happens — a static renderer with opacity=1.0 would pass `text_blink_visible` just fine. Multi-frame capture is needed to prove opacity changes over time.

**Reference implementations:**
- **Existing cursor opacity tests** `cursor_opacity_tests.rs`: Uses `render_to_pixels_with_opacity()` at three opacity levels. This pattern captures frames at different opacity values, but it sets opacity explicitly rather than advancing a timer. The text blink tests should follow a similar approach — render the same FrameInput at multiple `text_blink_opacity` values and compare pixel brightness across frames.

**Depends on:** None (text blink rendering from 05B already works).

---

## 03.1 Multi-Frame Text Blink Test

**File(s):** `oriterm/src/gpu/visual_regression/text_blink_tests.rs`

Replace the three single-frame tests with a single multi-frame test that proves animation:

- [ ] Write `text_blink_multi_frame` test that:
  1. Creates a FrameInput with one BLINK cell (col 0) and one non-BLINK cell (col 5), both showing 'A'
  2. Renders 3 frames at `text_blink_opacity` = 1.0, 0.5, 0.0
  3. For each frame, extracts the center pixel of both cells
  4. Asserts:
     - **Frame 1 (1.0):** BLINK cell brightness ~= non-BLINK cell brightness (both visible)
     - **Frame 2 (0.5):** BLINK cell brightness < non-BLINK cell brightness (BLINK dimmed)
     - **Frame 3 (0.0):** BLINK cell brightness ~= background (BLINK invisible), non-BLINK still visible
     - **Non-BLINK cell brightness is constant across all 3 frames** (proves only BLINK cells are affected)
  5. Golden image comparison for each frame (3 reference PNGs, replacing the old ones)
- [ ] **Visually inspect** all 3 golden images after generation

---

## 03.2 Replace Single-Frame Tests

**File(s):** `oriterm/src/gpu/visual_regression/text_blink_tests.rs`, `oriterm/tests/references/`

- [ ] Remove the old `text_blink_visible`, `text_blink_hidden`, `text_blink_half` test functions
- [ ] Delete the old golden images: `text_blink_visible.png`, `text_blink_hidden.png`, `text_blink_half.png`
- [ ] Verify the new `text_blink_multi_frame` test generates and passes against new golden images
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] Multi-frame test captures 3 frames at opacity 1.0, 0.5, 0.0
- [ ] Test asserts BLINK cell brightness changes across frames
- [ ] Test asserts non-BLINK cell brightness is constant across frames
- [ ] Old single-frame tests and golden images removed
- [ ] New golden images visually verified
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** The text blink test suite proves that BLINK cells animate (change opacity across frames) while non-BLINK cells remain unaffected. No single-frame tests remain.
