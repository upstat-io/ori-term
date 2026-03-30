---
section: "01"
title: "Bug Fixes"
status: not-started
reviewed: true
goal: "Fix three font rendering bugs: DPI change overwrites UI font settings, stale atlas gutter texels, and unrounded grid text Y positions."
inspired_by:
  - "Ghostty atlas zeroing (src/font/Atlas.zig:clear)"
  - "ori_term UI text Y rounding (gpu/scene_convert/text.rs:51)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "TPR-04-006: DPI Change UI Font Fix"
    status: not-started
  - id: "01.2"
    title: "TPR-04-008: Atlas Gutter Clearing"
    status: not-started
  - id: "01.3"
    title: "TPR-04-010: Grid Text Y Rounding"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Bug Fixes

**Status:** Not Started
**Goal:** After this section, DPI changes preserve UI font crispness, atlas gutter regions contain only zeros, and grid text Y positions are snapped to integer pixels.

**Context:** The TPR review identified three independent rendering bugs that cause blurriness or visual artifacts. All three are straightforward fixes with clear root causes. None require UI changes or new config fields.

**Reference implementations:**
- **Ghostty** `src/font/Atlas.zig:clear`: Zeroes atlas texture memory on clear, preventing stale texel sampling.
- **ori_term** `gpu/scene_convert/text.rs:51`: UI text already rounds `base_y` to integer pixels with a comment explaining the bilinear interpolation artifact — the grid path should do the same.

**Depends on:** Nothing.

---

## 01.1 TPR-04-006: DPI Change UI Font Fix

**File(s):** `oriterm/src/gpu/window_renderer/font_config.rs`

The UI font registry is intentionally initialized with `GlyphFormat::Alpha` and `HintingMode::None` (grayscale, no hinting — best for UI text at all DPIs). But `set_hinting_and_format()` at line 108-112 syncs the UI font registry to the *terminal* font's hinting/format settings. After a DPI change, the DPI handler calls this method with the terminal's resolved hinting/format, overwriting the UI font's intentional settings.

**Root cause:** Lines 108-112 of `font_config.rs`:
```rust
// Keep UI font registry in sync with the terminal font's rendering settings.
if let Some(sizes) = &mut self.ui_font_sizes {
    sizes.set_hinting(mode);
    sizes.set_format(format);
}
```

This comment is wrong — UI fonts should NOT be "in sync" with terminal font rendering settings. UI fonts always use Alpha/None.

- [ ] Remove lines 108-112 from `set_hinting_and_format()` in `gpu/window_renderer/font_config.rs`. The UI font registry's hinting/format are set at construction time (`GlyphFormat::Alpha`, `HintingMode::None`) and by `rebuild_ui_font_sizes()` during config reload (which also correctly hardcodes Alpha/None at `config_reload/font_config.rs:170-171`). DPI changes already correctly rebuild UI fonts via `set_font_size()` → `sizes.set_dpi(dpi)` (line 47-50). No path needs to change UI hinting/format during DPI change.

- [ ] Update the comment above `set_hinting_and_format()` to clarify it only affects terminal font rendering. Remove the misleading "Keep UI font registry in sync" comment.

- [ ] Verify the dialog window DPI handler at `app/dialog_context/event_handling/mod.rs:404` — it calls the same `set_hinting_and_format()`, so the fix propagates automatically.

### Tests (01.1)

Tests go in `gpu/window_renderer/tests.rs` (already exists). All require `GpuState::new_headless()` — early-return if unavailable.

- [ ] `test_set_hinting_and_format_preserves_ui_font_settings` — construct a `WindowRenderer` with UI font sizes (Alpha/None), call `set_hinting_and_format(HintingMode::Full, GlyphFormat::SubpixelRgb, gpu)`, assert UI font sizes registry still reports `HintingMode::None` and `GlyphFormat::Alpha`.
- [ ] `test_set_hinting_and_format_updates_terminal_font` — same setup, assert the terminal `FontCollection` now reports `HintingMode::Full` and `GlyphFormat::SubpixelRgb` (verifies the terminal path still works after the fix).
- [ ] `test_set_hinting_and_format_noop_when_unchanged` — call with the current values, verify no atlas clear happened (check that pre-cached glyphs survive — use atlas entry count before/after if available).

---

## 01.2 TPR-04-008: Atlas Gutter Clearing

**File(s):** `oriterm/src/gpu/atlas/mod.rs`, `oriterm/src/gpu/atlas/texture.rs`

The atlas reserves a 1px `GLYPH_PADDING` gutter between glyphs (line 46), but `upload_glyph()` writes only the glyph body. When `clear()` or `evict_page()` resets the packer without zeroing texture memory, newly packed glyphs inherit stale gutter pixels from previous occupants. With `FilterMode::Linear`, the bilinear sampler can interpolate these stale texels into glyph edges.

**Fix approach:** Zero the gutter when uploading each glyph. This is cheaper than zeroing entire atlas pages (which would require a `queue.write_texture` of megabytes) and handles both the clear and eviction cases since every new glyph gets clean padding.

- [ ] In `atlas/texture.rs`, modify `upload_glyph()` to also zero the padding region around the glyph. The allocator reserves `GLYPH_PADDING` (1px) on the right and bottom of each glyph via `find_space()` (which packs `w + GLYPH_PADDING` x `h + GLYPH_PADDING`). To prevent bilinear sampling from reading stale texels, upload zero strips for the right and bottom padding after uploading the glyph body:

  ```rust
  // Zero the 1px right strip: (x + width, y) with size (PADDING, height).
  // Zero the 1px bottom strip: (x, y + height) with size (width + PADDING, PADDING).
  ```

  The left and top edges are covered by the previous glyph's right/bottom padding (or page edge at x=0/y=0 which starts zeroed). This requires 2 additional `queue.write_texture` calls per glyph upload, each writing a tiny zero buffer.

- [ ] Store a reusable zero buffer on `GlyphAtlas` (e.g. `padding_zeros: Vec<u8>`) sized to `(PAGE_SIZE + GLYPH_PADDING) * bpp_max` once (where `bpp_max = 4` for RGBA). Slice into it for the strip uploads. This avoids per-glyph allocation. In practice glyphs are tiny, so the actual slices are very small.

- [ ] Alternative considered but rejected: building a padded buffer with the glyph centered would require copying the entire bitmap into a larger buffer (1 allocation + 1 memcpy per glyph). The 2-strip approach (right + bottom) is simpler and sufficient because the packer's padding is only on the right/bottom edges.

### Tests (01.2)

Tests go in `atlas/tests.rs` (already exists). All require `GpuState::new_headless()` — early-return if unavailable.

- [ ] `test_upload_glyph_zeros_right_padding` — upload a glyph, then read back the 1px strip at `(x + width, y)` to `(x + width + GLYPH_PADDING, y + height)`. All bytes must be zero.
- [ ] `test_upload_glyph_zeros_bottom_padding` — upload a glyph, then read back the 1px strip at `(x, y + height)` to `(x + width + GLYPH_PADDING, y + height + GLYPH_PADDING)`. All bytes must be zero.
- [ ] `test_stale_texels_cleared_after_atlas_reset` — insert a glyph filling most of a region, call `clear()` (resets packer but not texture memory), insert a smaller glyph into the same region. The old glyph's pixels in the new glyph's padding zone must be zeroed. (Note: requires texture readback.)
- [ ] `test_padding_zero_buffer_reused` — upload multiple glyphs of varying sizes, verify the `padding_zeros` buffer on `GlyphAtlas` is allocated once and sliced into (no per-glyph allocation).

**Warning:** Texture readback in wgpu requires `COPY_SRC` usage on the atlas texture (already present at `texture.rs:34`) and `map_async` on a staging buffer. If headless GPU does not support readback, these tests should early-return, not panic.

---

## 01.3 TPR-04-010: Grid Text Y Rounding

**File(s):** `oriterm/src/gpu/prepare/mod.rs`, `oriterm/src/gpu/prepare/dirty_skip/mod.rs`, `oriterm/src/gpu/prepare/emit.rs`

Grid text Y positions are computed as `oy + row as f32 * ch` without rounding. On non-integer scale factors (1.25x, 1.5x), `ch` is fractional, producing sub-pixel Y coordinates for every row. The bilinear atlas sampler then interpolates vertically, softening glyph edges. UI text already fixes this at `scene_convert/text.rs:51` by rounding `base_y`.

**Fix approach:** Round each row's Y position to an integer pixel. The per-row Y is `oy + row as f32 * ch`. On non-integer scale factors, `ch` is fractional, so this accumulates sub-pixel error across rows regardless of whether `oy` is rounded. The correct fix is `(oy + row as f32 * ch).round()` for every row Y computation in the grid path. This matches `scene_convert/text.rs:51` which rounds `base_y` for UI text.

- [ ] In `gpu/prepare/mod.rs`, in `fill_frame_shaped()`, change the per-row Y computation (line 346 and 355):
  ```rust
  // Line 346 (off-screen check):
  let row_y = (oy + row as f32 * ch).round();
  // Line 355 (cell Y):
  let y = (oy + row as f32 * ch).round();
  ```
  Both use the same `row` value, so the Y will be consistent within a row. The `oy` variable itself should NOT be rounded independently — rounding happens at the combined `oy + row * ch` level.

- [ ] In `gpu/prepare/dirty_skip/mod.rs`, apply the same `.round()` to the Y computations in `fill_frame_incremental()`:
  - Line 355 (off-screen check): `let row_y = (oy + row as f32 * ch).round();`
  - Line 375 (cell Y): `let y = (oy + row as f32 * ch).round();`

  `fill_frame_incremental` is a **separate function** from `fill_frame_shaped` — it copies cached instances for clean rows and runs its own cell loop for dirty rows. Cached instances from previous frames were generated with the same rounding (since `fill_frame_shaped` was already fixed), so clean-row copies remain consistent.

- [ ] In `gpu/prepare/emit.rs`, `draw_prompt_markers()` (line 138), `build_cursor()` (line 173), and `draw_url_hover_underline()` (line 248) all compute `oy + row as f32 * ch` independently. Apply `.round()` to the row base Y in each:
  - `draw_prompt_markers`: `let y = (oy + row as f32 * ch).round();` (line 138)
  - `build_cursor`: `let y = (oy + row as f32 * ch).round();` (line 173)
  
  - `draw_url_hover_underline`: `let y = (oy + line as f32 * ch).round() + underline_y_offset;` (line 248 — round only the row base, keep underline offset fractional for consistent positioning relative to the integer-aligned row)

- [ ] In `gpu/prepare/unshaped.rs` (test-only path), apply the same rounding at line 93:
  ```rust
  let y = (oy + cell.line as f32 * ch).round();
  ```

- [ ] In `update_cursor_only()` in `prepare/mod.rs` (line 267): `build_cursor` is called with `origin` (ox, oy). Since `build_cursor` will now round internally, this is handled. Verify consistency.

### Tests (01.3)

Tests go in `prepare/tests.rs` (already exists). These are pure CPU tests (no GPU needed) — `prepare_frame_shaped` works with a `TestAtlas`.

- [ ] `test_grid_y_positions_integer_at_fractional_scale` — with `oy = 56.3` and `ch = 18.75` (simulating 1.25x scale), run `prepare_frame_shaped` and verify all glyph instance Y positions satisfy `y == y.round()` for every row.
- [ ] `test_grid_y_positions_integer_at_integer_scale` — with `oy = 56.0` and `ch = 18.0` (1x scale), verify Y positions are already integer (regression guard: rounding must not introduce drift at integer scales).
- [ ] `test_cursor_y_position_integer_at_fractional_scale` — with fractional `oy` and `ch`, verify `build_cursor` output Y positions are integer-aligned.
- [ ] `test_prompt_marker_y_integer_at_fractional_scale` — with fractional `oy` and `ch`, verify `draw_prompt_markers` output Y positions are integer-aligned.
- [ ] `test_url_underline_y_base_integer_at_fractional_scale` — with fractional `oy` and `ch`, verify `draw_url_hover_underline` output Y positions have integer base (the `underline_y_offset` fractional component is allowed).
- [ ] `test_unshaped_y_positions_integer` — same fractional params through the unshaped path in `prepare/unshaped.rs`, verify all Y positions are integer.

**Note:** The `fill_frame_incremental` path in `dirty_skip/mod.rs` cannot be tested independently in the current test harness (it requires prior-frame state). The Y rounding is the same arithmetic as `fill_frame_shaped`, so the unit tests above provide confidence. The `dirty_skip/tests.rs` integration tests should be verified to exercise this path.

---

## 01.R Third Party Review Findings

- None.

---

## 01.N Completion Checklist

- [ ] TPR-04-006: `set_hinting_and_format()` no longer touches UI font registry hinting/format
- [ ] TPR-04-008: Atlas gutter texels are zeroed on every glyph upload
- [ ] TPR-04-010: Grid text Y positions are integer-aligned (fill_frame_shaped, fill_frame_incremental, emit.rs, unshaped.rs)
- [ ] 01.1 tests pass: 3 tests in `gpu/window_renderer/tests.rs`
- [ ] 01.2 tests pass: 4 tests in `gpu/atlas/tests.rs` (GPU-dependent, may skip)
- [ ] 01.3 tests pass: 6 tests in `gpu/prepare/tests.rs`
- [ ] `timeout 150 cargo test -p oriterm` green
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** DPI change preserves UI font crispness (Alpha/None not overwritten), atlas edges show no stale-texel artifacts under atlas churn, and grid glyph Y positions are integer-valued in prepared frame instances.
