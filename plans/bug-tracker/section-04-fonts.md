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

- [x] `[BUG-04-001][high]` **Color emoji (COLRv1) bitmaps clipped on bottom and right edges** — found by manual. **FIXED 2026-03-28.**
  Fix: Replaced swash's COLR renderer with our own COLRv1 compositor (`colr_v1/compose/`) that uses the correct COLR clip box for canvas sizing. Three compositor bugs were fixed: (1) two-circle radial gradients implemented pixel-by-pixel via quadratic solve (previously approximated as point-focal), (2) sweep gradient angle normalization to [0°, 360°) fixing atan2 discontinuity, (3) double premultiplication removed from pixel write path. Golden test updated.
