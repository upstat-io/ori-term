---
section: "04"
title: "Fonts"
status: in-progress
sections:
  - id: "04.1"
    title: "Active Bugs"
    status: in-progress
---

# Section 04: Fonts

Font discovery, collection, shaping, rasterization, COLRv1, emoji fallback.

## 04.1 Active Bugs

- [ ] `[BUG-04-001][high]` **Color emoji (COLRv1) bitmaps clipped on bottom and right edges** — found by manual.
  Repro: Run `printf '\e]1;😀 Test\a'` to set an emoji tab icon. The smiley face's chin and right edge are hard-cut with no anti-aliasing. All three test emoji (🐍🔥😀) show clipping. Visible in the golden test `tab_bar_emoji.png` and confirmed by the `emoji_not_clipped_in_rendered_output` clip detection test (snake bottom 77%, smiley bottom 62%).
  Subsystem: `oriterm/src/font/collection/face.rs` (rasterize_from_face), swash `Source::ColorOutline`
  Root cause: Swash determines the color glyph bitmap canvas size from the base glyph outline bounds, not the COLR clip box. COLR paint layers that extend beyond the outline bounds are clipped. The COLR clip box (from skrifa) IS larger and correct, but swash doesn't use it.
  Attempted fixes: (1) Expanding bitmap with transparent padding — adds empty space but missing content was never rendered. (2) Using our custom COLRv1 compositor — correct bounds but wrong colors (sweep gradient issues). (3) Combining swash colors with COLR clip box canvas — pixels in the expanded area are transparent (content never rendered by swash).
  Correct fix: Either fix swash upstream to use COLR clip box for canvas sizing, fix our COLRv1 compositor's sweep gradient color accuracy, or implement a hybrid approach that re-renders only the overflow regions.
  Found: 2026-03-28 | Source: manual
  Note: Affects both tab bar emoji icons AND terminal grid emoji. The `emoji_not_clipped_in_rendered_output` test exists with a 90% threshold (should be 50% when fixed). The golden test `tab_bar_emoji_golden` captures the current (clipped) state as the reference.
