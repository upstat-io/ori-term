---
section: "06"
title: Verification
status: complete
goal: "All UI polish changes verified: visual correctness, animation behavior, no regressions, build clean"
depends_on: ["01", "02", "03", "04", "05"]
sections:
  - id: "06.1"
    title: "Test Matrix"
    status: complete
  - id: "06.2"
    title: "Visual Regression"
    status: complete
  - id: "06.3"
    title: "Performance Validation"
    status: complete
  - id: "06.4"
    title: "Completion Checklist"
    status: complete
---

# Section 06: Verification

**Status:** Not Started
**Goal:** All UI polish changes work together correctly, animations feel smooth, no regressions in existing functionality, and the build is clean.

**Depends on:** All previous sections (01–05).

---

## 06.1 Test Matrix

- [x] **GPU Scissor Rects:**
  - `PushClip`/`PopClip` produce `ClipSegment`s — covered (`draw_list_convert/tests.rs`)
  - Nested clips intersect correctly — covered
  - Scale factor applied to clip rects — covered
  - Multi-writer clip segments (rects + glyphs) — covered
  - Unbalanced `PopClip` handled — covered
  - Scroll widget clips work correctly — visual verification + unit test
- [x] **Tab Clipping:**
  - With 20+ tabs open, no title text or close button from one tab overlaps into adjacent tab space — visual verification
  - With 3–5 tabs (normal width), tab appearance is unchanged from pre-clipping behavior — visual verification
  - During tab reorder slide, clipped content moves with the tab (no static clip artifact) — visual verification
- [x] **Color Animation:**
  - `Lerp for Color` — covered (`color/tests.rs`)
  - Tab hover fade in/out — visual verification
  - Close button opacity fade — visual verification
  - Active tab shows no hover animation — visual verification
  - Vec resize on `set_tabs()` — covered (`tab_bar/tests.rs`)
- [x] **Tab Lifecycle:**
  - Tab open width animation — visual verification
  - Tab close width animation — visual verification
  - Dynamic slide duration — covered (`slide/tests.rs`)
  - Variable-width `tab_x()` — covered (`tab_bar/tests.rs`)
  - Binary search `tab_index_at()` — covered (`tab_bar/tests.rs`)
  - `closing_complete()` — covered (`tab_bar/tests.rs`)
- [x] **Drag Elevation:**
  - Drop shadow visible during drag — visual verification
  - No backing rect artifact — visual verification
  - Shadow visible in light and dark themes — visual verification

---

## 06.2 Visual Regression

Verify visual output matches expectations at key states.

- [x] **Single tab**: Tab bar renders correctly with 1 tab (no animations needed)
- [x] **Many tabs**: 20+ tabs, all narrow — clipping works, no bleed-through
- [x] **Hover transition**: Move cursor across tabs — smooth color fades, no instant jumps
- [x] **Tab close**: Close a middle tab — width shrinks, neighbors slide, smooth
- [x] **Tab open**: Open new tab — width expands from zero, content fades in
- [x] **Tab drag**: Drag a tab — shadow visible, no backing rect, smooth reorder
- [x] **Tab drag + narrow tabs**: Drag a tab when tabs are narrow — clips hold, no bleed
- [x] **DPI scaling**: Test at 1x, 1.5x, 2x scale — scissor rects scale correctly
- [x] **Window resize**: Resize window while tabs visible — no clip artifacts
- [x] **Scroll widget**: If a scroll widget is visible (future feature), content clips at scroll bounds — verify no regression from Section 01 changes
- [x] **Rapid close**: Close 5 tabs rapidly — close animations don't pile up or corrupt state, Vecs stay in sync
- [x] **Rapid open**: Open 5 tabs rapidly — open animations overlap correctly (each tab animates independently)
- [x] **Close during hover**: Close a hovered tab — hover animation and close animation don't conflict
- [x] **Tab width lock + animation**: Hover-lock active during close — multipliers respect lock priority

---

## 06.3 Performance Validation

Tab bar rendering is NOT a hot path (redraws only on user interaction, not per-frame), but animations do request continued redraws. Verify they don't cause excessive GPU work.

- [x] **Animation frame budget**: During hover transition, total frame time (CPU + GPU) stays under 8ms — well within the 16.6ms budget for 60 FPS
- [x] **Animation convergence**: All animations complete and stop requesting redraws within their declared duration + 1 frame
- [x] **Idle overhead**: When no animations are running, zero additional work vs. current implementation
- [x] **Scissor rect overhead**: Adding clip segments to the render pass adds negligible overhead (scissor is a hardware-native operation)

---

## 06.4 Completion Checklist

- [x] **File size audit**: No source file (excluding `tests.rs`) exceeds 500 lines. Key files to check:
  - `draw_list_convert/mod.rs` (was 425 — clip infrastructure extracted to `clip.rs`)
  - `widget/mod.rs` (was 468 — animation state extracted to `animation.rs`)
  - `widget/draw.rs` (was 480 — `draw_dragged_tab_overlay` extracted if needed)
  - `helpers.rs` (was 300 — `record_draw_clipped` added)
  - `prepared_frame/mod.rs` (was 211 — `TierClips` fields added)
- [x] All unit tests pass: `./test-all.sh`
- [x] No clippy warnings: `./clippy-all.sh`
- [x] Cross-compilation succeeds: `./build-all.sh`
- [x] Tab show-through bug is fixed (20+ tabs, no bleed)
- [x] Hover animations feel smooth and responsive
- [x] Tab open/close animations feel natural
- [x] Dragged tab has visible elevation (shadow)
- [x] No visual regression at 1x, 1.5x, 2x DPI
- [x] Animations stop requesting redraws after completion

**Exit Criteria:** All three build scripts pass with zero warnings. Visual inspection at multiple DPI scales confirms: tabs clip correctly, hover animates smoothly, open/close transitions feel Chrome-grade, and dragged tabs show elevation. No frame drops during animation.
