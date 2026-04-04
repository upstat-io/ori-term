---
section: "04"
title: "Golden Image Revalidation"
status: complete
reviewed: true
goal: "Every affected golden image re-rendered, visually inspected, and verified correct"
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Re-render Affected Golden Images"
    status: complete
  - id: "04.2"
    title: "Investigate inverse_video.png"
    status: complete
  - id: "04.3"
    title: "Visual Verification Sweep"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: complete
---

# Section 04: Golden Image Revalidation

<!-- reviewed: architecture fix -->
<!-- OBE analysis: All three subsections (04.1, 04.2, 04.3) were completed as
     side effects of Sections 01 and 02. Section 01 re-rendered and visually
     verified all 9 DECSCNM golden images (01.4). Section 02 / DECCOLM fix
     (commit 6937781a) re-rendered VT102 golden images. The inverse_video test
     was investigated and found correct. Visual verification of all images
     performed during this review pass on 2026-04-02. -->

**Status:** Complete (OBE — all work completed during Sections 01-02)
**Goal:** All golden images affected by Sections 01-02 fixes are re-rendered and visually verified correct. The `inverse_video.png` test is investigated and confirmed correct.

**Context:** This section was originally planned as a post-fix revalidation sweep. In practice, Sections 01 and 02 each re-rendered and visually verified their own golden images as part of their completion checklists. The inverse_video test was investigated and found to be correct. All work is OBE (overtaken by events).

**Depends on:** Section 01 (DECSCNM fix), Section 02 (VT102 scroll region fix) — both complete.

---

## 04.1 Re-render Affected Golden Images

<!-- reviewed: architecture fix — OBE, completed during Sections 01-02 -->

**File(s):** `oriterm/tests/references/`

All golden images were re-rendered during Sections 01 and 02:

- [x] DECSCNM golden images (02_03, 02_04, 02_14 at 3 sizes = 9 images) re-rendered in Section 01 (commit `9b37ca07`), then again in DECCOLM fix (commit `6937781a`). Checked off in Section 01.4.
- [x] VT102 golden images (08-12) re-rendered in DECCOLM fix (commit `6937781a`). Section 02 verified all 14 screens render correctly.
- [x] All vttest golden tests pass (8/8): menu1 3 sizes, menu2 3 sizes, menu3, menu8. `./test-all.sh` green.

---

## 04.2 Investigate inverse_video.png

<!-- reviewed: architecture fix — investigated, found correct, no fix needed -->

**File(s):** `oriterm/src/gpu/visual_regression/decoration_tests.rs` (line 236)

Investigation result: the test is correct, no fix needed.

- [x] Test found at `decoration_tests.rs:236` (not `core_tests.rs` as originally guessed)
- [x] Test does NOT use `CellFlags::INVERSE` — it directly swaps `cell.fg` and `cell.bg` via `std::mem::swap()`. This is architecturally correct because `FrameInput::test_grid()` bypasses the extract phase where `CellFlags::INVERSE` would normally be resolved to swapped colors. The comment in the test documents this: "The extract phase resolves INVERSE to swapped colors; since test_grid bypasses extract, we swap the colors directly."
- [x] Golden image visually inspected: row 0 shows light text on dark background (normal), row 1 shows dark text on light background (inverse). The swap is clearly visible. Default fg is (211, 215, 207) light gray, default bg is (30, 30, 46) dark blue-gray — after swap, row 1 has a light gray background.
- [x] Test passes: `cargo test -p oriterm --features gpu-tests inverse_video` — ok.
- [x] Original plan concern ("the inverse line doesn't appear to have swapped colors") was a false alarm — the contrast difference is subtle at small display sizes but the colors ARE correctly swapped.

---

## 04.3 Visual Verification Sweep

<!-- reviewed: architecture fix — all images visually verified during this review -->

Every golden image referenced in the plan was visually inspected:

- [x] `vttest_80x24_02_03.png`, `vttest_97x33_02_03.png`, `vttest_120x40_02_03.png` — light background with "132 column mode" text confirmed at all 3 sizes
- [x] `vttest_80x24_02_04.png`, `vttest_97x33_02_04.png`, `vttest_120x40_02_04.png` — light background with "80 column mode" text confirmed at all 3 sizes
- [x] `vttest_80x24_02_14.png`, `vttest_97x33_02_14.png`, `vttest_120x40_02_14.png` — light background with SGR rendition pattern (vanilla, underline, blink, negative, etc.) confirmed at all 3 sizes
- [x] `vttest_80x24_08_vt102_08.png` — top line shows A's (accordion with scroll region)
- [x] `vttest_80x24_08_vt102_09.png` — top line shows A's (IL/DL result)
- [x] `vttest_80x24_08_vt102_10.png` — top line starts with A, ends with B (insert mode)
- [x] `vttest_80x24_08_vt102_11.png` — top line shows "AB" (delete char)
- [x] `vttest_80x24_08_vt102_12.png` — top line shows A's with stagger pattern (DCH)
- [x] `inverse_video.png` — row 0 normal (light-on-dark), row 1 inverse (dark-on-light)
- [x] Working tree clean (`git status` shows no changes) — no other golden images were inadvertently changed

---

## 04.R Third Party Review Findings

- None.

---

## 04.N Completion Checklist

- [x] All 9 DECSCNM golden images show light background (visually verified)
- [x] All 5 VT102 golden images (08-12) show correct top-line content (visually verified)
- [x] `inverse_video.png` shows visible inverse video effect (visually verified — no fix needed)
- [x] No other golden images inadvertently changed (clean working tree)
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Every golden image that was broken by the audit findings has been re-rendered, visually inspected, and confirmed correct. The test suite catches rendering bugs rather than rubber-stamping broken output.
