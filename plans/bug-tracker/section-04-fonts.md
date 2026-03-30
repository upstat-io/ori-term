---
section: "04"
title: "Fonts"
status: in-progress
third_party_review:
  status: findings
  updated: 2026-03-29
sections:
  - id: "04.1"
    title: "Active Bugs"
    status: in-progress
  - id: "04.R"
    title: "Third Party Review Findings"
    status: in-progress
---

# Section 04: Fonts

Font discovery, collection, shaping, rasterization, COLRv1, emoji fallback.

## 04.1 Active Bugs

- [ ] `[BUG-04-002][high]` **Tab bar text blurry after DPI change (window dragged between monitors)** — found by manual.
  Repro: Drag oriterm window from a 1.25x DPI monitor to a 1.0x DPI monitor. Tab bar text (and likely status bar text) appears blurry/upscaled.
  Subsystem: `oriterm/src/app/mod.rs` (`handle_dpi_change`), `oriterm/src/gpu/window_renderer/` (UI font atlas)
  Found: 2026-03-29 | Source: manual
  Analysis: `handle_dpi_change` calls `renderer.set_font_size()` (terminal font) and `ctx.text_cache.clear()`, but the UI font (`UiFontSizes`) may not be re-rasterized at the new physical DPI. The UI font glyphs cached in the GPU atlas at the old scale factor would render blurry when composited at the new scale.

- [x] `[BUG-04-001][high]` **Color emoji (COLRv1) bitmaps clipped on bottom and right edges** — found by manual. **FIXED 2026-03-28.**
  Fix: Replaced swash's COLR renderer with our own COLRv1 compositor (`colr_v1/compose/`) that uses the correct COLR clip box for canvas sizing. Three compositor bugs were fixed: (1) two-circle radial gradients implemented pixel-by-pixel via quadratic solve (previously approximated as point-focal), (2) sweep gradient angle normalization to [0°, 360°) fixing atan2 discontinuity, (3) double premultiplication removed from pixel write path. Golden test updated.

---

## 04.R Third Party Review Findings

- [ ] `[TPR-04-006][high]` `oriterm/src/app/init/mod.rs:110`, `oriterm/src/gpu/window_renderer/font_config.rs:106`, `oriterm/src/app/mod.rs:301` — UI text is initialized and rebuilt as grayscale/no-hinting, but every DPI-change path immediately overwrites `UiFontSizes` with the terminal font's hinting/subpixel mode via `set_hinting_and_format()`. That means tab bar, status bar, dialogs, and overlays change rasterization policy after a monitor move, which matches `[BUG-04-002]` and can introduce blur/color-fringing that was not present at startup.

- [ ] `[TPR-04-007][high]` `oriterm/src/config/font_config.rs:81`, `oriterm/src/gpu/window_renderer/helpers.rs:184`, `oriterm/src/gpu/scene_convert/text.rs:62`, `oriterm/src/gpu/prepare/emit.rs:74`, `oriterm/src/font/collection/rasterize.rs:42`, `oriterm/src/font/collection/face.rs:227` — `font.subpixel_positioning` is parsed and tested, but never consumed by the renderer. Grid glyphs and UI glyphs always quantize to quarter-pixel X phases and always pass that fractional offset through to rasterization, so there is currently no way to request fully snapped positioning for maximum crispness on hinted/1x displays.

- [ ] `[TPR-04-008][medium]` `oriterm/src/gpu/atlas/mod.rs:45`, `oriterm/src/gpu/atlas/mod.rs:302`, `oriterm/src/gpu/atlas/mod.rs:413`, `oriterm/src/gpu/atlas/mod.rs:569`, `oriterm/src/gpu/atlas/texture.rs:63`, `oriterm/src/gpu/bind_groups/mod.rs:82` — Atlas packing reserves a 1px gutter, but uploads only the glyph body, never clears the gutter, and page clear/eviction only resets metadata. Because glyph sampling uses a linear sampler, reused regions can interpolate against stale texels in that uncleared padding, producing edge bleed under atlas churn.

- [ ] `[TPR-04-009][medium]` `oriterm/src/font/discovery/mod.rs:348`, `oriterm/src/font/discovery/linux.rs:109`, `oriterm/src/font/discovery/macos.rs:104`, `oriterm/src/font/discovery/families.rs:23`, `oriterm/src/font/collection/resolve.rs:48` — Fallback selection is a static filename-ordered chain, not a codepoint-aware or locale-aware fallback resolver. This is materially weaker than the reference implementations for emoji and CJK selection, especially on macOS where Chromium/Ghostty/WezTerm explicitly defer to CoreText per-codepoint fallback for Han locale correctness.

- [ ] `[TPR-04-010][medium]` `oriterm/src/gpu/prepare/emit.rs:92,138,173,248`, `oriterm/src/gpu/prepare/mod.rs:355` — Grid text Y positions are not rounded to integer pixels. Every grid glyph Y is computed as `oy + row as f32 * ch` with no rounding, so fractional origin + fractional cell heights produce sub-pixel Y coordinates. The bilinear atlas sampler then interpolates vertically, softening glyph edges. UI text already fixes this: `scene_convert/text.rs:51` rounds `base_y` to integers with a comment explaining the artifact. The grid path should apply the same rounding. Most visible on non-integer scale factors (1.25x, 1.5x) where `ch` is fractional.
  Evidence: UI text path has explicit `.round()` on base_y with comment about bilinear interpolation artifact. Grid path omits it.
  Impact: Glyph vertical softening on fractional-DPI displays. At 1x/2x scale this is rarely visible (cell heights tend to be integers), but at 1.25x/1.5x every row has fractional Y.

- [ ] `[TPR-04-011][low]` `oriterm/src/gpu/bind_groups/mod.rs:90-91` — Atlas sampler uses `FilterMode::Linear` for all three atlas types (mono, subpixel, color). Ghostty uses `FilterMode::Nearest` for its font atlas, which gives pixel-perfect glyph rendering when quad positions align exactly to texel boundaries. Linear filtering provides tolerance for minor positioning errors but softens glyphs whenever UV coordinates don't land exactly on texel centers. With the subpixel X binning and proper Y rounding (see TPR-04-010), Nearest could produce crisper results. However, Alacritty and Zed also use Linear — this is a valid design trade-off, not a clear defect.
  Evidence: Ghostty uses `filter::nearest` in Metal shaders and `.min_filter = .nearest` in OpenGL path. Alacritty and Zed use Linear.
  Impact: Slight glyph softening vs pixel-perfect crispness. Low severity because Linear is a defensible choice and switching requires exact texel alignment.

- [x] `[TPR-04-004][medium]` `oriterm/src/font/collection/mod.rs:293` — **FIXED 2026-03-29.** Eviction ordering fixed: cache now clears *before* inserting the new glyph, so the threshold-crossing glyph survives and `glyph_cache.get(&key)` succeeds.

- [x] `[TPR-04-005][low]` `oriterm/src/font/collection/mod.rs:1` — **FIXED 2026-03-29.** Extracted configuration setters (`set_size`, `set_hinting`, `set_format`, `set_features`, `set_fallback_meta`, codepoint mapping) into `collection/config.rs` (142 lines). `mod.rs` now 417 lines.

- [x] `[TPR-04-001][high]` `oriterm/src/font/collection/colr_v1/compose/mod.rs` — **FIXED 2026-03-29.** Affine transforms now applied to radial radii via `transform_radius_scale()` (geometric-mean of singular values) and to sweep angles via `transform_rotation_degrees()`. Both helpers added as `pub(super)` free functions in `compose/mod.rs`. Point-focal radial path in `brush.rs` also fixed. Golden reference updated.

- [x] `[TPR-04-002][medium]` `oriterm/src/font/collection/colr_v1/compose/tests.rs` — **FIXED 2026-03-29.** Added 10 unit tests: `transform_radius_scale` (identity, uniform 2x, rotation, non-uniform), `transform_rotation_degrees` (identity, 90°, 45°), `solve_radial_t` (concentric, no-solution, linear), `fill_sweep_direct` (full-circle coverage), `fill_radial_direct` (midrange pixel, outside-pad).

- [x] `[TPR-04-003][low]` `oriterm/src/font/collection/colr_v1/compose/mod.rs` — **FIXED 2026-03-29.** Extracted `SkiaPen` + `glyph_path` into `compose/pen.rs` (107 lines). `compose/mod.rs` now 429 lines, well under the 500-line limit.
