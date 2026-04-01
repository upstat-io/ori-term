---
section: "04"
title: "Fonts"
status: in-progress
third_party_review:
  status: findings
  updated: 2026-03-31
sections:
  - id: "04.1"
    title: "Active Bugs"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: in-progress
---

# Section 04: Fonts

Font discovery, collection, shaping, rasterization, COLRv1, emoji fallback.

## 04.1 Active Bugs

- [x] `[BUG-04-002][high]` **Tab bar text blurry after DPI change (window dragged between monitors)** — found by manual.
  Resolved: OBE on 2026-03-30. Root cause was TPR-04-006 (`set_hinting_and_format()` overwriting UI font Alpha/None with terminal settings). Fixed in commit 1f31395.

- [x] `[BUG-04-001][high]` **Color emoji (COLRv1) bitmaps clipped on bottom and right edges** — found by manual. **FIXED 2026-03-28.**
  Fix: Replaced swash's COLR renderer with our own COLRv1 compositor (`colr_v1/compose/`) that uses the correct COLR clip box for canvas sizing. Three compositor bugs were fixed: (1) two-circle radial gradients implemented pixel-by-pixel via quadratic solve (previously approximated as point-focal), (2) sweep gradient angle normalization to [0°, 360°) fixing atan2 discontinuity, (3) double premultiplication removed from pixel write path. Golden test updated.

- [x] `[BUG-04-003][high]` **Ligatures no longer rendering — regression** — found by manual.
  Repro: Type `=>`, `->`, `!=`, `fi`, or any other ligature-supported sequence in the terminal with a ligature font (e.g. Fira Code, JetBrains Mono). Ligatures should combine into single glyphs but render as separate characters instead.
  Subsystem: `oriterm/src/font/shaper/mod.rs` (run segmentation + rustybuzz shaping), `oriterm/src/font/collection/mod.rs` (feature application)
  Found: 2026-03-29 | Source: manual — user reports ligatures stopped working (were working previously)
  Resolved: 2026-03-30 — Exhaustive pipeline audit confirmed no defect. Five new tests added proving the full ligature pipeline works correctly with the embedded JetBrains Mono font:
  (1) `ligature_arrow_calt_applies` — shaping `=>` produces calt-substituted glyph IDs different from individually-shaped `=` and `>`
  (2) `ligature_not_equal_calt_applies` — same for `!=`
  (3) `ligature_run_segmentation_groups_correctly` — `=` and `>` are correctly grouped into a single shaping run
  (4) `ligature_arrow_space_between_no_ligature` — column mapping is correct when space separates characters
  (5) `rasterize_ligature_glyph_id_produces_bitmap` — calt-substituted glyph IDs rasterize to valid bitmaps through swash
  The full pipeline (run segmentation → rustybuzz shaping with calt/liga features → column mapping → glyph rasterization) is verified end-to-end. Original report was likely caused by a font that lacks GSUB/calt tables, a config with `features = []`, or was already resolved by intervening changes.

- [ ] `[BUG-04-004][medium]` **Emoji in tab title vanishes after monitor transition** — found by manual.
  Repro: Set a tab title containing an emoji (e.g. via OSC 2). Drag the window between monitors with different DPI/scale factors. The emoji sometimes disappears from the tab bar while ASCII text remains.
  Subsystem: `oriterm/src/gpu/window_renderer/font_config.rs` (`clear_and_recache`), `oriterm/src/font/collection/mod.rs` (glyph cache), `oriterm/src/app/mod.rs` (`handle_dpi_change`)
  Found: 2026-03-31 | Source: manual — user report

---

## 04.R Third Party Review Findings

- [x] `[TPR-04-006][high]` `oriterm/src/gpu/window_renderer/font_config.rs:106` — UI text rasterization overwritten on DPI change.
  Resolved: Fixed in commit 1f31395. Removed the 4-line block that synced UI font hinting/format to terminal settings. UI fonts now always keep Alpha/None. Also removed dead `UiFontSizes::set_hinting/set_format` methods. 3 GPU-gated tests added. Fixed 2026-03-30.

- [x] `[TPR-04-007][high]` `oriterm/src/config/font_config.rs:81`, `oriterm/src/gpu/window_renderer/helpers.rs:184`, `oriterm/src/gpu/scene_convert/text.rs:62`, `oriterm/src/gpu/prepare/emit.rs:74`, `oriterm/src/font/collection/rasterize.rs:42`, `oriterm/src/font/collection/face.rs:227` — `font.subpixel_positioning` is parsed and tested, but never consumed by the renderer. Grid glyphs and UI glyphs always quantize to quarter-pixel X phases and always pass that fractional offset through to rasterization, so there is currently no way to request fully snapped positioning for maximum crispness on hinted/1x displays.
  Resolved: Fixed in font-rendering-quality plan Section 02 on 2026-03-31. `subpixel_positioning` now flows through `FrameInput`, `prepare/mod.rs`, `prepare/emit.rs`, `prepare/dirty_skip/mod.rs`, `scene_convert/text.rs`, and `window_renderer/helpers.rs`. When false, all subpx_bin calls return 0 (fully snapped). Exposed as "Subpixel positioning" dropdown in Settings > Font > Advanced.

- [x] `[TPR-04-008][medium]` `oriterm/src/gpu/atlas/texture.rs` — Atlas gutter texels never cleared.
  Resolved: Fixed in commit 1f31395. `upload_glyph()` now writes zero strips for right and bottom GLYPH_PADDING regions. Reusable zero buffer on GlyphAtlas. 4 texture readback tests added. Fixed 2026-03-30.

- [ ] `[TPR-04-009][medium]` `oriterm/src/font/discovery/mod.rs:348`, `oriterm/src/font/discovery/linux.rs:109`, `oriterm/src/font/discovery/macos.rs:104`, `oriterm/src/font/discovery/families.rs:23`, `oriterm/src/font/collection/resolve.rs:48` — Fallback selection is a static filename-ordered chain, not a codepoint-aware or locale-aware fallback resolver. This is materially weaker than the reference implementations for emoji and CJK selection, especially on macOS where Chromium/Ghostty/WezTerm explicitly defer to CoreText per-codepoint fallback for Han locale correctness.

- [x] `[TPR-04-010][medium]` Grid text Y positions not rounded to integer pixels.
  **Fixed 2026-03-30.** Added `.round()` to all grid row Y computations: `prepare/mod.rs` (main path), `prepare/dirty_skip/mod.rs` (dirty-skip path), `prepare/emit.rs` (prompt markers and cursor). Matches the UI text rounding in `scene_convert/text.rs:51`.

- [x] `[TPR-04-011][low]` `oriterm/src/gpu/bind_groups/mod.rs:90-91` — Atlas sampler uses `FilterMode::Linear` for all three atlas types (mono, subpixel, color). Ghostty uses `FilterMode::Nearest` for its font atlas, which gives pixel-perfect glyph rendering when quad positions align exactly to texel boundaries. Linear filtering provides tolerance for minor positioning errors but softens glyphs whenever UV coordinates don't land exactly on texel centers. With the subpixel X binning and proper Y rounding (see TPR-04-010), Nearest could produce crisper results. However, Alacritty and Zed also use Linear — this is a valid design trade-off, not a clear defect.
  Evidence: Ghostty uses `filter::nearest` in Metal shaders and `.min_filter = .nearest` in OpenGL path. Alacritty and Zed use Linear.
  Impact: Slight glyph softening vs pixel-perfect crispness. Low severity because Linear is a defensible choice and switching requires exact texel alignment.
  Resolved: Fixed in font-rendering-quality plan Section 02 on 2026-03-31. `AtlasFiltering` enum added to `bind_groups/mod.rs`. `AtlasBindGroup` stores the filter mode and `rebuild()` recreates the sampler accordingly. Exposed as "Atlas filtering" dropdown in Settings > Font > Advanced. Auto-detects based on scale factor (Nearest at 2x+, Linear below).

- [x] `[TPR-04-004][medium]` `oriterm/src/font/collection/mod.rs:293` — **FIXED 2026-03-29.** Eviction ordering fixed: cache now clears *before* inserting the new glyph, so the threshold-crossing glyph survives and `glyph_cache.get(&key)` succeeds.

- [x] `[TPR-04-005][low]` `oriterm/src/font/collection/mod.rs:1` — **FIXED 2026-03-29.** Extracted configuration setters (`set_size`, `set_hinting`, `set_format`, `set_features`, `set_fallback_meta`, codepoint mapping) into `collection/config.rs` (142 lines). `mod.rs` now 417 lines.

- [x] `[TPR-04-001][high]` `oriterm/src/font/collection/colr_v1/compose/mod.rs` — **FIXED 2026-03-29.** Affine transforms now applied to radial radii via `transform_radius_scale()` (geometric-mean of singular values) and to sweep angles via `transform_rotation_degrees()`. Both helpers added as `pub(super)` free functions in `compose/mod.rs`. Point-focal radial path in `brush.rs` also fixed. Golden reference updated.

- [x] `[TPR-04-002][medium]` `oriterm/src/font/collection/colr_v1/compose/tests.rs` — **FIXED 2026-03-29.** Added 10 unit tests: `transform_radius_scale` (identity, uniform 2x, rotation, non-uniform), `transform_rotation_degrees` (identity, 90°, 45°), `solve_radial_t` (concentric, no-solution, linear), `fill_sweep_direct` (full-circle coverage), `fill_radial_direct` (midrange pixel, outside-pad).

- [x] `[TPR-04-003][low]` `oriterm/src/font/collection/colr_v1/compose/mod.rs` — **FIXED 2026-03-29.** Extracted `SkiaPen` + `glyph_path` into `compose/pen.rs` (107 lines). `compose/mod.rs` now 429 lines, well under the 500-line limit.
