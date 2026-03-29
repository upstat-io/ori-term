---
section: "04"
title: "Fonts"
status: in-progress
third_party_review:
  status: resolved
  updated: 2026-03-29
sections:
  - id: "04.1"
    title: "Active Bugs"
    status: in-progress
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
---

# Section 04: Fonts

Font discovery, collection, shaping, rasterization, COLRv1, emoji fallback.

## 04.1 Active Bugs

- [x] `[BUG-04-001][high]` **Color emoji (COLRv1) bitmaps clipped on bottom and right edges** — found by manual. **FIXED 2026-03-28.**
  Fix: Replaced swash's COLR renderer with our own COLRv1 compositor (`colr_v1/compose/`) that uses the correct COLR clip box for canvas sizing. Three compositor bugs were fixed: (1) two-circle radial gradients implemented pixel-by-pixel via quadratic solve (previously approximated as point-focal), (2) sweep gradient angle normalization to [0°, 360°) fixing atan2 discontinuity, (3) double premultiplication removed from pixel write path. Golden test updated.

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-004][medium]` `oriterm/src/font/collection/mod.rs:293` — **FIXED 2026-03-29.** Eviction ordering fixed: cache now clears *before* inserting the new glyph, so the threshold-crossing glyph survives and `glyph_cache.get(&key)` succeeds.

- [x] `[TPR-04-005][low]` `oriterm/src/font/collection/mod.rs:1` — **FIXED 2026-03-29.** Extracted configuration setters (`set_size`, `set_hinting`, `set_format`, `set_features`, `set_fallback_meta`, codepoint mapping) into `collection/config.rs` (142 lines). `mod.rs` now 417 lines.

- [x] `[TPR-04-001][high]` `oriterm/src/font/collection/colr_v1/compose/mod.rs` — **FIXED 2026-03-29.** Affine transforms now applied to radial radii via `transform_radius_scale()` (geometric-mean of singular values) and to sweep angles via `transform_rotation_degrees()`. Both helpers added as `pub(super)` free functions in `compose/mod.rs`. Point-focal radial path in `brush.rs` also fixed. Golden reference updated.

- [x] `[TPR-04-002][medium]` `oriterm/src/font/collection/colr_v1/compose/tests.rs` — **FIXED 2026-03-29.** Added 10 unit tests: `transform_radius_scale` (identity, uniform 2x, rotation, non-uniform), `transform_rotation_degrees` (identity, 90°, 45°), `solve_radial_t` (concentric, no-solution, linear), `fill_sweep_direct` (full-circle coverage), `fill_radial_direct` (midrange pixel, outside-pad).

- [x] `[TPR-04-003][low]` `oriterm/src/font/collection/colr_v1/compose/mod.rs` — **FIXED 2026-03-29.** Extracted `SkiaPen` + `glyph_path` into `compose/pen.rs` (107 lines). `compose/mod.rs` now 429 lines, well under the 500-line limit.
