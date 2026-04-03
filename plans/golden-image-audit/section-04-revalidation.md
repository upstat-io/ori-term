---
section: "04"
title: "Golden Image Revalidation"
status: not-started
reviewed: false
goal: "Every affected golden image re-rendered, visually inspected, and verified correct"
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Re-render Affected Golden Images"
    status: not-started
  - id: "04.2"
    title: "Investigate inverse_video.png"
    status: not-started
  - id: "04.3"
    title: "Visual Verification Sweep"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Golden Image Revalidation

**Status:** Not Started
**Goal:** All golden images affected by Sections 01-02 fixes are re-rendered and visually verified correct. The `inverse_video.png` test is investigated and fixed if needed.

**Context:** After DECSCNM and VT102 scroll region fixes land, the affected golden images will no longer match (tests will fail). This section re-renders them with `ORITERM_UPDATE_GOLDEN=1`, visually inspects every re-rendered image by reading the PNG, and verifies correctness.

**Depends on:** Section 01 (DECSCNM fix), Section 02 (VT102 scroll region fix).

---

## 04.1 Re-render Affected Golden Images

**File(s):** `oriterm/tests/references/`

After Sections 01 and 02 are complete:

- [ ] Run `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests vttest_golden_menu2` to re-render all menu 2 golden images
- [ ] Run `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests vttest_golden_menu8` to re-render all VT102 golden images
- [ ] Run `./test-all.sh` to verify all tests pass with the new golden images

---

## 04.2 Investigate inverse_video.png

**File(s):** `oriterm/src/gpu/visual_regression/core_tests.rs` (or wherever this test lives)

The `inverse_video.png` golden image shows "Normal video mode" and "Inverse video mode" text, but the inverse line doesn't appear to have swapped colors. SGR 7 per-cell inverse works in vttest rendition patterns (02_13), so this is likely a test setup issue.

- [ ] Find the test that generates `inverse_video.png` — read the test code to see how the grid is set up
- [ ] Check if the test actually sets `CellFlags::INVERSE` on the "Inverse video mode" cells
- [ ] If the test is wrong (doesn't set flags), fix the test setup and re-render
- [ ] If the test is right (flags are set), the rendering has a bug — investigate and fix
- [ ] **Visually inspect** the corrected `inverse_video.png` — verify the inverse line has swapped fg/bg
- [ ] Re-render with `ORITERM_UPDATE_GOLDEN=1` and verify

---

## 04.3 Visual Verification Sweep

Read every re-rendered golden image to confirm visual correctness:

- [ ] Read all `vttest_*_02_03.png` (3 sizes) — verify white/light background
- [ ] Read all `vttest_*_02_04.png` (3 sizes) — verify white/light background
- [ ] Read all `vttest_*_02_14.png` (3 sizes) — verify white/light background with SGR rendition pattern
- [ ] Read `vttest_80x24_08_vt102_08.png` — verify top line shows A's (accordion with scroll region)
- [ ] Read `vttest_80x24_08_vt102_09.png` — verify top line shows A's (IL/DL result)
- [ ] Read `vttest_80x24_08_vt102_10.png` — verify top line starts with A, ends with B (insert mode)
- [ ] Read `vttest_80x24_08_vt102_11.png` — verify top line shows "AB" (delete char)
- [ ] Read `vttest_80x24_08_vt102_12.png` — verify top line shows A's (DCH stagger)
- [ ] Read `inverse_video.png` — verify inverse line has visible fg/bg swap
- [ ] Confirm no other golden images were inadvertently changed

---

## 04.R Third Party Review Findings

- None.

---

## 04.N Completion Checklist

- [ ] All 9 DECSCNM golden images show light background (visually verified)
- [ ] All 5 VT102 golden images (08-12) show correct top-line content (visually verified)
- [ ] `inverse_video.png` shows visible inverse video effect (visually verified)
- [ ] No other golden images inadvertently changed
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Every golden image that was broken by the audit findings has been re-rendered, visually inspected, and confirmed correct. The test suite catches rendering bugs rather than rubber-stamping broken output.
