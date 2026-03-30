---
section: "02"
title: "Advanced Font Rendering Settings"
status: not-started
reviewed: true
goal: "Expose hinting, subpixel AA, subpixel positioning, and atlas filtering as user-configurable settings in the Font page's Advanced section, with auto-detection defaults."
inspired_by:
  - "WezTerm freetype_load_target/freetype_load_flags DPI-based auto-detection (config/src/config.rs, wezterm-font/src/ftwrap.rs)"
  - "Ghostty FreetypeLoadFlags boolean toggles (src/config/Config.zig)"
  - "Zed SectionHeader('Advanced Settings') pattern (crates/settings_ui/src/page_data.rs)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Config Fields + Resolution Functions"
    status: not-started
  - id: "02.2"
    title: "Renderer Wiring"
    status: not-started
  - id: "02.3"
    title: "Settings UI: Font Page Advanced Section"
    status: not-started
  - id: "02.4"
    title: "Remove Rendering Page Subpixel Toggle"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Advanced Font Rendering Settings

**Status:** Not Started
**Goal:** After this section, the Font page has an Advanced section with 4 dropdowns (hinting, subpixel AA, subpixel positioning, atlas filtering). Each defaults to "Auto" with the detected value shown in parentheses. Changes persist to TOML config and take effect immediately.

**Context:** The TPR review found that `subpixel_positioning` is a dead config (TPR-04-007) — parsed from TOML but never consumed by the renderer. The atlas sampler's `FilterMode` is hardcoded to `Linear`. Users have no way to control rendering quality beyond the basic subpixel toggle on the Rendering page. Reference terminal emulators (WezTerm, Ghostty) expose these controls for power users.

**Reference implementations:**
- **WezTerm** `config/src/config.rs:274-335`: `freetype_load_target`, `freetype_load_flags`, `display_pixel_geometry` with DPI-based auto-detection fallback in `ftwrap.rs:57-96`.
- **Ghostty** `src/config/Config.zig:9561`: `FreetypeLoadFlags` packed struct with boolean enable/disable syntax.
- **Zed** `crates/settings_ui/src/page_data.rs`: `SectionHeader("Advanced Settings")` for grouping power-user options below main settings.

**Depends on:** Nothing (independent of Section 01, though both may touch nearby code).

**File-size hygiene warnings:**
- `window_renderer/mod.rs` is at **509 lines** — already over the 500-line limit. Adding 2 new fields (`subpixel_positioning`, `atlas_filtering`) plus initializers in both `new()` and `new_ui_only()` is NOT possible without first splitting. Before adding fields, extract the `CombinedAtlasLookup` struct and its `AtlasLookup` impl (lines 53-66, 14 lines) into a private submodule (e.g. `atlas_lookup.rs`) or move the struct definition and impl into `helpers.rs` (which is at 467 lines, has room). This creates headroom for the 2 new fields + 2 initializer lines in each constructor (~8 lines total). Setter methods MUST go in `font_config.rs` per the plan.
- `config_reload/mod.rs` is at **478 lines**. Adding ~20 lines of change detection + resolution calls approaches ~498. Safe but tight — no other additions in this section.
- `font/mod.rs` is at 493 lines. Do NOT add `AtlasFiltering` here (would exceed 500). Place it in `gpu/bind_groups/mod.rs` per the plan.

---

## 02.1 Config Fields + Resolution Functions

**File(s):** `oriterm/src/config/font_config.rs`, `oriterm/src/app/config_reload/font_config.rs`, `oriterm/src/font/mod.rs`

Wire the existing dead `subpixel_positioning` field and add a new `atlas_filtering` field. Add resolution functions with auto-detection fallbacks.

### 02.1.1 Add `AtlasFiltering` enum

- [ ] In `oriterm/src/gpu/bind_groups/mod.rs`, add an `AtlasFiltering` enum (this is a GPU sampling concern, not a font concern — unlike `HintingMode`/`SubpixelMode` which affect rasterization):
  ```rust
  /// Atlas texture sampling filter mode.
  ///
  /// Controls how the GPU samples glyph textures. `Linear` (bilinear
  /// interpolation) is forgiving of sub-texel positioning but slightly
  /// softens glyphs. `Nearest` (point sampling) gives pixel-perfect
  /// crispness but requires exact texel alignment.
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
  pub enum AtlasFiltering {
      /// Bilinear interpolation — slight softening, tolerant of positioning.
      #[default]
      Linear,
      /// Nearest-neighbor — pixel-perfect, requires exact alignment.
      Nearest,
  }

  impl AtlasFiltering {
      /// Auto-detect filtering mode from display scale factor.
      ///
      /// HiDPI (2x+) uses Nearest (enough pixels for perfect alignment).
      /// Non-HiDPI uses Linear (sub-texel tolerance helps at low resolution).
      pub fn from_scale_factor(scale_factor: f64) -> Self {
          if scale_factor >= 2.0 { Self::Nearest } else { Self::Linear }
      }

      /// Convert to the wgpu `FilterMode` for sampler creation.
      pub fn to_filter_mode(self) -> FilterMode {
          match self {
              Self::Linear => FilterMode::Linear,
              Self::Nearest => FilterMode::Nearest,
          }
      }
  }
  ```

  Note: `font/mod.rs` is at 493 lines — adding the enum there would exceed the 500-line limit. The GPU bind_groups module is the right home since `AtlasFiltering` only affects the GPU sampler, not glyph rasterization.

### 02.1.2 Add `atlas_filtering` config field

- [ ] In `oriterm/src/config/font_config.rs`, add `atlas_filtering: Option<String>` to `FontConfig` with `#[serde(default)]`. Valid values: `"linear"`, `"nearest"`, `None` (auto). Also add `atlas_filtering: None` to the `Default` impl (line 106). The `FontConfig` derives `PartialEq`, so the new field will automatically participate in equality checks (used by `per_page_dirty` and `apply_font_changes`).

### 02.1.2b Change `subpixel_positioning` from `bool` to `Option<bool>`

- [ ] In `oriterm/src/config/font_config.rs`, change `pub subpixel_positioning: bool` (line 81) to `pub subpixel_positioning: Option<bool>` and remove the `#[serde(default = "default_true")]` attribute (replace with `#[serde(default)]`). Update the `Default` impl to set `subpixel_positioning: None` instead of `true`.

- [ ] Remove the now-unused `default_true()` function at line 114 of `font_config.rs` (only `subpixel_positioning` used it; `config/mod.rs` has its own copy for `resize_increments`). The `dead_code = "deny"` lint would catch this.

- [ ] Audit all existing references to `config.font.subpixel_positioning`: the field is currently unused by the renderer (TPR-04-007). The only consumers are the config TOML deserialization and the `apply_font_changes()` change detection (which uses `pending.font != original.font`). The new resolution function in 02.1.3 will be the primary consumer.

### 02.1.3 Add resolution functions

- [ ] In `oriterm/src/app/config_reload/font_config.rs`, add two new resolution functions:

  ```rust
  /// Resolve subpixel positioning from config, falling back to auto-detection.
  ///
  /// `None` = auto (enabled). `Some(true)` = forced on. `Some(false)` = forced off.
  pub(crate) fn resolve_subpixel_positioning(config: &FontConfig, _scale_factor: f64) -> bool {
      match config.subpixel_positioning {
          Some(explicit) => explicit,
          None => true, // Auto: always enabled (quarter-pixel X binning).
      }
  }

  /// Resolve atlas filtering from config, falling back to auto-detection.
  pub(crate) fn resolve_atlas_filtering(
      config: &FontConfig,
      scale_factor: f64,
  ) -> crate::gpu::bind_groups::AtlasFiltering {
      use crate::gpu::bind_groups::AtlasFiltering;
      match config.atlas_filtering.as_deref() {
          Some("linear") => AtlasFiltering::Linear,
          Some("nearest") => AtlasFiltering::Nearest,
          Some(other) => {
              log::warn!("config: unknown atlas_filtering {other:?}, using auto-detection");
              AtlasFiltering::from_scale_factor(scale_factor)
          }
          None => AtlasFiltering::from_scale_factor(scale_factor),
      }
  }
  ```

  

- [ ] Export the new functions from `config_reload/mod.rs`:
  ```rust
  pub(crate) use font_config::{resolve_atlas_filtering, resolve_subpixel_positioning};
  ```

### Tests (02.1)

Resolution function tests belong in `config/tests.rs` (which already tests `resolve_hinting` and `resolve_subpixel_mode` at line 1414+). Config deserialization tests are already there too.

**Update existing tests** for the `subpixel_positioning` type change from `bool` to `Option<bool>`:
- [ ] `subpixel_positioning_defaults_to_true` (line 1311): rename to `subpixel_positioning_defaults_to_none_auto` and change `assert!(parsed.font.subpixel_positioning)` to `assert_eq!(parsed.font.subpixel_positioning, None)` (None = auto = enabled)
- [ ] `subpixel_positioning_false_from_toml` (line 1317): change `assert!(!parsed.font.subpixel_positioning)` to `assert_eq!(parsed.font.subpixel_positioning, Some(false))`
- [ ] `font_config_new_fields_roundtrip` (line 1394): change `cfg.font.subpixel_positioning = false` to `cfg.font.subpixel_positioning = Some(false)` and `assert!(!parsed.font.subpixel_positioning)` to `assert_eq!(parsed.font.subpixel_positioning, Some(false))`

**New tests:**
- [ ] `subpixel_positioning_true_from_toml` — TOML `subpixel_positioning = true` deserializes to `Some(true)`.
- [ ] `atlas_filtering_defaults_to_none` — empty TOML → `config.font.atlas_filtering == None`.
- [ ] `atlas_filtering_linear_from_toml` — TOML `atlas_filtering = "linear"` → `Some("linear")`.
- [ ] `atlas_filtering_nearest_from_toml` — TOML `atlas_filtering = "nearest"` → `Some("nearest")`.
- [ ] `atlas_filtering_roundtrip` — set `atlas_filtering = Some("nearest")`, serialize, deserialize, assert preserved.
- [ ] `resolve_subpixel_positioning_none_means_auto` — `config.subpixel_positioning = None` → `resolve_subpixel_positioning()` returns `true`.
- [ ] `resolve_subpixel_positioning_explicit_false` — `config.subpixel_positioning = Some(false)` → returns `false`.
- [ ] `resolve_subpixel_positioning_explicit_true` — `config.subpixel_positioning = Some(true)` → returns `true`.
- [ ] `resolve_atlas_filtering_none_low_dpi` — `None` config + scale 1.0 → `AtlasFiltering::Linear`.
- [ ] `resolve_atlas_filtering_none_high_dpi` — `None` config + scale 2.0 → `AtlasFiltering::Nearest`.
- [ ] `resolve_atlas_filtering_explicit_linear` — `Some("linear")` → `AtlasFiltering::Linear` regardless of scale.
- [ ] `resolve_atlas_filtering_explicit_nearest` — `Some("nearest")` → `AtlasFiltering::Nearest` regardless of scale.
- [ ] `resolve_atlas_filtering_invalid_falls_back` — `Some("invalid")` → auto-detection result + warning logged.

---

## 02.2 Renderer Wiring

**File(s):** `oriterm/src/gpu/bind_groups/mod.rs`, `oriterm/src/gpu/prepare/emit.rs`, `oriterm/src/gpu/window_renderer/font_config.rs`, `oriterm/src/app/config_reload/mod.rs`

Wire the resolved settings into the renderer so they actually take effect.

### 02.2.1 Atlas filtering → sampler

- [ ] Modify `AtlasBindGroup::new()` in `gpu/bind_groups/mod.rs` to accept a `FilterMode` parameter instead of hardcoding `FilterMode::Linear`:
  ```rust
  pub fn new(
      device: &Device,
      layout: &BindGroupLayout,
      view: &TextureView,
      filter: FilterMode,
  ) -> Self {
  ```
  Store the `FilterMode` on the struct so `rebuild()` can recreate the sampler with the same filter when atlas textures grow:
  ```rust
  pub struct AtlasBindGroup {
      bind_group: BindGroup,
      sampler: wgpu::Sampler,
      filter: FilterMode, // NEW: needed so rebuild() preserves the filter
  }
  ```

- [ ] Update `rebuild()` to recreate the sampler from `self.filter` instead of reusing `self.sampler`. This is necessary because `rebuild_stale_atlas_bind_groups()` (in `window_renderer/mod.rs:305`) calls `rebuild()` when atlas generations change. If `set_atlas_filtering()` replaced the bind group with a new sampler but then the atlas grew, `rebuild()` must produce a sampler matching the current `self.filter`, not the stale `self.sampler` from before the filter change. Always recreating from `self.filter` keeps the sampler and filter in sync unconditionally.

- [ ] Update all `AtlasBindGroup::new()` call sites to pass the resolved `FilterMode`. There are 10 total:
  - 3 in `WindowRenderer::new()` at `window_renderer/mod.rs:184-188`
  - 3 in `WindowRenderer::new_ui_only()` at `window_renderer/ui_only.rs:79-83`
  - 4 in test code at `bind_groups/tests.rs:101,114,133,153`

  For WindowRenderer, store `AtlasFiltering` on the struct (initialized from resolved config in `new()`, defaulting to `AtlasFiltering::Linear` in `new_ui_only()`). Convert via `filtering.to_filter_mode()` when creating bind groups.

- [ ] Add a `set_atlas_filtering()` method to `WindowRenderer` in `font_config.rs` that updates the stored filtering mode and recreates all three atlas bind groups with the new sampler. Also snapshot atlas generations to prevent `rebuild_stale_atlas_bind_groups()` from immediately re-rebuilding the bind groups we just created:
  ```rust
  pub fn set_atlas_filtering(&mut self, filtering: AtlasFiltering, device: &Device, layout: &BindGroupLayout) {
      let filter = filtering.to_filter_mode();
      self.atlas_bind_group = AtlasBindGroup::new(device, layout, self.atlas.view(), filter);
      self.subpixel_atlas_bind_group = AtlasBindGroup::new(device, layout, self.subpixel_atlas.view(), filter);
      self.color_atlas_bind_group = AtlasBindGroup::new(device, layout, self.color_atlas.view(), filter);
      // Snapshot generations so rebuild_stale_atlas_bind_groups() doesn't
      // immediately re-rebuild the bind groups we just created.
      self.atlas_generation = self.atlas.generation();
      self.subpixel_atlas_generation = self.subpixel_atlas.generation();
      self.color_atlas_generation = self.color_atlas.generation();
      self.atlas_filtering = filtering;
  }
  ```


### 02.2.2 Subpixel positioning → glyph emission

- [ ] **Pre-requisite: `frame_input/mod.rs` is at 508 lines (over the 500-line limit).** Before adding a field, extract the `cell_in_search_match()` free function (lines 490-505, 16 lines) into a private submodule (e.g. `frame_input/search_match.rs`). Re-export via `use search_match::cell_in_search_match;` in `mod.rs`. This reclaims ~16 lines, bringing `mod.rs` to ~492 — safe for the new field.
- [ ] Add a `subpixel_positioning: bool` field to `FrameInput` in `gpu/frame_input/mod.rs` (after the existing `fg_dim` field at line 375). Default to `true`. Update all 6 construction sites:
  - `gpu/extract/from_snapshot/mod.rs:37` — struct literal (add `subpixel_positioning: true`)
  - `gpu/extract/from_snapshot/mod.rs:157` — `_into()` field reset (add `out.subpixel_positioning = true;`)
  - `gpu/frame_input/mod.rs:460` — `test_grid()` helper (add `subpixel_positioning: true`)
  - `gpu/frame_input/tests.rs:100, 126, 151` — 3 test struct literals (add `subpixel_positioning: true`)

- [ ] Add a `subpixel_positioning: bool` field to the `GlyphEmitter` struct in `gpu/prepare/emit.rs`. When `false`, force `subpx` to 0 at line 74:
  ```rust
  let subpx = if self.subpixel_positioning { subpx_bin(sg.x_offset) } else { 0 };
  ```
  This ensures the `RasterKey` has `subpx_x: 0` (no subpixel phase) and the glyph is rasterized without fractional X offset.

- [ ] **Pre-requisite: `window_renderer/mod.rs` is at 509 lines (over the 500-line limit).** Before adding fields, extract the `CombinedAtlasLookup` struct and its `AtlasLookup for CombinedAtlasLookup` impl (lines 53-66) into `window_renderer/helpers.rs` (at 467 lines, has room). Re-export via `use helpers::CombinedAtlasLookup;` in `mod.rs`. This reclaims ~14 lines, bringing `mod.rs` to ~495.
- [ ] Add `subpixel_positioning: bool` and `atlas_filtering: AtlasFiltering` fields to `WindowRenderer` (in `window_renderer/mod.rs`). Initialize `subpixel_positioning: true` and `atlas_filtering: AtlasFiltering::Linear` in `WindowRenderer::new()` and `new_ui_only()`. This adds ~8 lines, bringing `mod.rs` to ~503 — setter methods MUST go in `font_config.rs` to keep `mod.rs` as close to 500 as possible.
- [ ] Add a `set_subpixel_positioning()` method to `WindowRenderer` in `font_config.rs`. This is a trivial setter (no GPU resources need recreation — the flag only affects glyph emission at prepare time):
  ```rust
  pub fn set_subpixel_positioning(&mut self, enabled: bool) {
      self.subpixel_positioning = enabled;
  }
  ```
  Also add a getter `pub fn subpixel_positioning(&self) -> bool` so the prepare path and raster key functions can read the value.
- [ ] In `app/init/mod.rs`, change `let renderer` (line 134) to `let mut renderer` and add the initial resolution immediately after:
  ```rust
  let mut renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, ui_sizes);
  let subpx_pos = config_reload::resolve_subpixel_positioning(&self.config.font, scale);
  renderer.set_subpixel_positioning(subpx_pos);
  let atlas_filter = config_reload::resolve_atlas_filtering(&self.config.font, scale);
  renderer.set_atlas_filtering(atlas_filter, &gpu.device, &pipelines.atlas_layout);
  ```
  This ensures the renderer has correct values from the first frame, matching the existing pattern where `set_hinting_and_format` is called during DPI handling. The variable `scale` (line 82, `f64`) is already in scope.
- [ ] In `app/window_management.rs`, in `create_window_renderer()`, restructure the return at line 264-269 to capture the renderer in a `let mut renderer = ...;` binding, apply resolved values, then return `Some(renderer)`. The function already resolves `hinting` and `format` at lines 214-221, so add `subpixel_positioning` and `atlas_filtering` resolution in the same block and apply after construction. Access `pipelines.atlas_layout` from the `pipelines` parameter already in scope. Note: `scale` is `f32` here (line 213), so use `f64::from(scale)` for resolution functions, matching the existing pattern at line 215/219.

- [ ] Thread the `subpixel_positioning` flag from `WindowRenderer` through `fill_frame_shaped()` → `GlyphEmitter`.
  **Chosen approach: Option B (FrameInput).** Add `subpixel_positioning: bool` to `FrameInput` (already has `fg_dim`, a rendering param). This avoids adding parameters to `fill_frame_shaped`, `fill_frame_incremental`, `prepare_frame_shaped_into`, and `prepare_frame_shaped` — no clippy `too_many_arguments` issues.
  - **Option A rejected:** Adding a parameter to `fill_frame_shaped` cascades to `fill_frame_incremental`, `prepare_frame_shaped_into`, `prepare_frame_shaped` (4 functions), and ALL their call sites — including ~19 `prepare_frame_shaped()` calls in tests, ~7 `fill_frame_shaped()` calls in tests, ~6 `prepare_frame_shaped_into()` calls in tests. That is a 32+ site update for a single bool. FrameInput is the right vehicle.

  FrameInput struct literal sites to update (6 total — all default to `subpixel_positioning: true`). These are the SAME sites as the 02.2.2 bullet above — listed here for the FrameInput approach context:
  - `gpu/extract/from_snapshot/mod.rs:37` — `extract_frame_from_snapshot()` struct literal (default `true`)
  - `gpu/extract/from_snapshot/mod.rs:157` — `extract_frame_from_snapshot_into()` field reset (add `out.subpixel_positioning = true;`)
  - `gpu/frame_input/mod.rs:460` — `test_grid()` helper (hardcode `true`; covers all test callers including `visual_regression/edge_case_tests.rs` which uses `test_grid()`)
  - `gpu/frame_input/tests.rs:100, 126, 151` — 3 test struct literals (hardcode `true`)

- [ ] Wire `subpixel_positioning` from `WindowRenderer` to `FrameInput` at extraction sites. After extraction, set `input.subpixel_positioning = renderer.subpixel_positioning()`. This avoids adding a parameter to `extract_frame_from_snapshot` (which has 12+ call sites):
  - `app/redraw/mod.rs:145-148` — single-pane extraction (set on the FrameInput after `extract_frame_from_snapshot` / `extract_frame_from_snapshot_into`)
  - `app/redraw/multi_pane/mod.rs:171-179` — multi-pane extraction (same pattern)

  **Note:** `redraw/mod.rs` (534 lines) and `redraw/multi_pane/mod.rs` (555 lines) are pre-existing violations of the 500-line limit. This plan adds only 1-2 lines to each (a single field assignment after extraction), which is not the right time to split them. Their file-size debt should be tracked separately.

- [ ] In `fill_frame_shaped()` (prepare/mod.rs), the `GlyphEmitter` is constructed at line 437 inside the per-cell loop. Add `subpixel_positioning: input.subpixel_positioning` to the construction (the `input: &FrameInput` parameter is in scope):
  ```rust
  GlyphEmitter {
      baseline,
      size_q6: shaped.size_q6(),
      hinted: shaped.hinted(),
      fg_dim,
      subpixel_positioning: input.subpixel_positioning, // NEW
      atlas,
      frame,
  }
  ```
  No parameter changes to `fill_frame_shaped`, `fill_frame_incremental`, `prepare_frame_shaped_into`, or `prepare_frame_shaped` — the flag comes from `FrameInput`.

- [ ] In `fill_frame_incremental()` (dirty_skip/mod.rs:258), read `input.subpixel_positioning` the same way and pass to `GlyphEmitter`.

- [ ] In `multi_pane.rs:99` (`fill_frame_shaped` call), no changes needed — `FrameInput` already carries the flag.

- [ ] Add `subpixel_positioning: bool` parameter to `grid_raster_keys()` in `helpers.rs:184`. When `false`, force `subpx_x: 0` in the produced `RasterKey` (line 196):
  ```rust
  subpx_x: if subpixel_positioning { crate::font::subpx_bin(glyph.x_offset) } else { 0 },
  ```
  Update all 2 callers of `grid_raster_keys()`:
  - `window_renderer/mod.rs:405` — `WindowRenderer::prepare()` Phase B, pass `self.subpixel_positioning`
  - `window_renderer/multi_pane.rs:70` — `prepare_pane()` Phase B, pass `self.subpixel_positioning`

- [ ] Add `subpixel_positioning: bool` parameter to `scene_raster_keys()` in `helpers.rs:206`. When `false`, force `subpx_x: 0` (line 232) and round `cursor_x` to integer pixels:
  ```rust
  let cursor_x = if subpixel_positioning { cursor_x } else { cursor_x.round() };
  subpx_x: if subpixel_positioning { crate::font::subpx_bin(cursor_x + glyph.x_offset) } else { 0 },
  ```
  Update the 1 caller of `scene_raster_keys()`:
  - `window_renderer/scene_append.rs:112` — `append_ui_scene_with_text()`, pass `self.subpixel_positioning`

- [ ] Add `subpixel_positioning: bool` field to `TextContext` in `gpu/scene_convert/mod.rs:32`. This threads the flag from the renderer to the UI text conversion path. Update `convert_text()` in `scene_convert/text.rs:62`:
  ```rust
  let subpx = if ctx.subpixel_positioning { subpx_bin(cursor_x + glyph.x_offset) } else { 0 };
  ```
  When `false`, also round `cursor_x` to integer pixels before the glyph loop to prevent fractional drift.

- [ ] Update `TextContext` construction in `scene_append.rs` (2 production sites at lines 32 and 75) to include `subpixel_positioning: self.subpixel_positioning`. Update all 19 test construction sites in `scene_convert/tests.rs` to include `subpixel_positioning: true` (preserving current test behavior).

- [ ] The rasterizer in `font/collection/rasterize.rs` uses `subpx_offset(key.subpx_x)` to position the fractional X offset during rasterization. When `subpx_x` is 0, `subpx_offset(0)` returns 0.0 — no rasterizer changes needed. Verify this.

### 02.2.3 Config reload wiring

- [ ] In `app/config_reload/mod.rs`, update the doc comment on `apply_font_changes()` (line 111) to include "subpixel positioning, atlas filtering" in the field list.

- [ ] In the same function, update the `font_changed` detection (lines 123-133) to also check the new fields:
  ```rust
  || new.font.subpixel_positioning != old.subpixel_positioning
  || new.font.atlas_filtering != old.atlas_filtering
  ```
  Without this, changing these settings via the dialog or TOML file will NOT trigger application. This piggybacks on the full font reload path, which is heavier than strictly necessary for these settings, but is acceptable for a cold path.

- [ ] In `apply_font_changes()`, after `let Some(gpu) = &self.gpu else { return };` (line 163), add `let Some(pipelines) = self.pipelines.as_ref() else { return };`. Both are immutable borrows on separate `App` fields — no borrow conflict with the mutable `self.windows` iteration below.

- [ ] In the `apply_font_changes()` per-window loop (lines 166-217), after `renderer.replace_font_collection(fc, gpu);` (line 215), resolve and apply the new settings:
  ```rust
  let subpx_pos = resolve_subpixel_positioning(&new.font, scale);
  renderer.set_subpixel_positioning(subpx_pos);
  let atlas_filter = resolve_atlas_filtering(&new.font, scale);
  renderer.set_atlas_filtering(atlas_filter, &gpu.device, &pipelines.atlas_layout);
  ```
  Note: `renderer.replace_font_collection()` already clears atlases and re-caches. The `set_subpixel_positioning()` call just stores the flag; the `set_atlas_filtering()` call recreates bind groups with the new sampler. Both are cheap operations that piggyback on the existing reload.

- [ ] In the DPI change handler `app/mod.rs:handle_dpi_change` (line 301): after `renderer.set_hinting_and_format(hinting, format, gpu)` at line 321, add atlas filtering re-resolution. Note: `self.pipelines` is borrowed immutably while `renderer` (from `self.windows`) is borrowed mutably — these are separate `App` fields, no borrow conflict:
  ```rust
  let atlas_filter = config_reload::resolve_atlas_filtering(&self.config.font, scale_factor);
  if let Some(pipelines) = &self.pipelines {
      renderer.set_atlas_filtering(atlas_filter, &gpu.device, &pipelines.atlas_layout);
  }
  ```

- [ ] In the dialog DPI handler `app/dialog_context/event_handling/mod.rs:handle_dialog_dpi_change` (line 381): same pattern after `renderer.set_hinting_and_format(...)` at line 404. The dialog handler is `impl App`, so `self.gpu`, `self.dialogs`, and `self.pipelines` are all accessible. Add `self.pipelines.as_ref()` for the atlas layout. The borrow pattern is: `self.gpu` (immutable), `self.dialogs[window_id].renderer` (mutable), `self.pipelines` (immutable) — all separate fields, no conflict.

### Tests (02.2)

**AtlasFiltering enum tests** go in `gpu/bind_groups/tests.rs` (pure unit tests, no GPU needed):

- [ ] `test_atlas_filtering_from_scale_factor_low_dpi` — `AtlasFiltering::from_scale_factor(1.0)` returns `Linear`.
- [ ] `test_atlas_filtering_from_scale_factor_high_dpi` — `AtlasFiltering::from_scale_factor(2.0)` returns `Nearest`.
- [ ] `test_atlas_filtering_from_scale_factor_boundary` — `AtlasFiltering::from_scale_factor(1.99)` returns `Linear` (below 2.0 threshold).
- [ ] `test_atlas_filtering_to_filter_mode_linear` — `AtlasFiltering::Linear.to_filter_mode()` returns `FilterMode::Linear`.
- [ ] `test_atlas_filtering_to_filter_mode_nearest` — `AtlasFiltering::Nearest.to_filter_mode()` returns `FilterMode::Nearest`.

**AtlasBindGroup tests** go in `gpu/bind_groups/tests.rs` (already exists, has 4 `AtlasBindGroup::new()` call sites that must be updated per 02.2.1). All require `GpuState::new_headless()`.

- [ ] `test_atlas_bind_group_new_with_linear_filter` — create with `FilterMode::Linear`, verify struct stores the filter mode.
- [ ] `test_atlas_bind_group_new_with_nearest_filter` — create with `FilterMode::Nearest`, verify struct stores the filter mode.
- [ ] `test_atlas_bind_group_rebuild_preserves_filter` — create with `Nearest`, rebuild with new texture view, verify filter is still `Nearest`.

**Subpixel positioning — grid path tests** go in `gpu/prepare/tests.rs`:

- [ ] `test_subpixel_positioning_disabled_forces_zero_subpx` — run glyph emission with `subpixel_positioning: false`, verify all `RasterKey.subpx_x` values in the prepared frame are 0.
- [ ] `test_subpixel_positioning_enabled_allows_nonzero_subpx` — same with `subpixel_positioning: true` and a glyph with fractional x_offset, verify `subpx_x` is non-zero.

**Subpixel positioning — raster key tests** go in `gpu/window_renderer/tests.rs`:

- [ ] `test_grid_raster_keys_disabled_subpx_all_zero` — call `grid_raster_keys(shaped, hinted, false)`, verify all keys have `subpx_x == 0`.
- [ ] `test_scene_raster_keys_disabled_subpx_all_zero` — call `scene_raster_keys(scene, hinted, scale, keys, false)`, verify all keys have `subpx_x == 0`.

**Subpixel positioning — UI text tests** go in `gpu/scene_convert/tests.rs`:

- [ ] `test_convert_text_disabled_subpx_forces_zero` — construct `TextContext` with `subpixel_positioning: false`, run `convert_text` with a multi-glyph text run, verify all emitted glyph instances have integer X positions (no fractional subpixel offset).

**Config change detection** — add to `app/settings_overlay/tests.rs`:

- [ ] `test_apply_font_changes_detects_subpixel_positioning_change` — modify `config.font.subpixel_positioning` from `None` to `Some(false)`, verify `font_changed` is true in the change detection logic. (This can be tested indirectly by verifying `per_page_dirty` marks font page as dirty.)
- [ ] `test_apply_font_changes_detects_atlas_filtering_change` — same for `atlas_filtering`.

- [ ] `/tpr-review` checkpoint — substantial new rendering code wired across multiple files.

---

## 02.3 Settings UI: Font Page Advanced Section

**File(s):** `oriterm/src/app/settings_overlay/form_builder/font.rs`, `oriterm/src/app/settings_overlay/form_builder/mod.rs`, `oriterm/src/app/settings_overlay/action_handler/mod.rs`

Add 4 dropdowns to a new "Advanced" section on the Font page.

### 02.3.1 Add SettingsIds for new controls

- [ ] In `form_builder/mod.rs`, add 4 new fields to `SettingsIds`:
  ```rust
  // Font page — Advanced section.
  pub hinting_dropdown: WidgetId,
  pub subpixel_aa_dropdown: WidgetId,
  pub subpixel_positioning_dropdown: WidgetId,
  pub atlas_filtering_dropdown: WidgetId,
  ```

- [ ] Add corresponding `WidgetId::placeholder()` entries in `SettingsIds::placeholder()`.

- [ ] Update the `settings_ids_all_distinct` test in `form_builder/tests.rs` (line 23): change the expected count from `26` to `29` (26 original - 1 removed `subpixel_toggle` + 4 new = 29). NOTE: `subpixel_toggle` removal happens in 02.4 — if implementing 02.3 first, use `30` temporarily then drop to `29` in 02.4.
- [ ] Update the `collect_ids()` helper in `form_builder/tests.rs` (line 196): add `set.insert(ids.hinting_dropdown.raw())`, `set.insert(ids.subpixel_aa_dropdown.raw())`, `set.insert(ids.subpixel_positioning_dropdown.raw())`, and `set.insert(ids.atlas_filtering_dropdown.raw())` in the Font section. In 02.4, remove `set.insert(ids.subpixel_toggle.raw())` from the Rendering section.

### 02.3.2 Build the Advanced section

- [ ] In `form_builder/font.rs`, add a `build_advanced_section(config, ids, theme, scale_factor, opacity)` function and call it from `build_page()` (added after `build_fallback_section()`). Each dropdown must be initialized with the correct selected index matching the current config value:
  - Hinting: `config.font.hinting.as_deref()` — `None` → 0 (Auto), `Some("full")` → 1, `Some("none")` → 2
  - Subpixel AA: `config.font.subpixel_mode.as_deref()` — `None` → 0 (Auto), `Some("rgb")` → 1, `Some("bgr")` → 2, `Some("none")` → 3
  - Subpixel pos: `config.font.subpixel_positioning` — `None` → 0 (Auto), `Some(true)` → 1, `Some(false)` → 2
  - Atlas filtering: `config.font.atlas_filtering.as_deref()` — `None` → 0 (Auto), `Some("linear")` → 1, `Some("nearest")` → 2

  Each dropdown has "Auto (detected)" as index 0, plus explicit options:

  **Hinting dropdown:**
  ```
  Items: ["Auto (Full)", "Full", "None"]   // or "Auto (None)" at 2x+ DPI
  ```
  The "Auto" label includes the current auto-detected value in parentheses. To compute this, call `HintingMode::from_scale_factor(scale)` at build time and format the label.

  
  **Subpixel AA dropdown:**
  ```
  Items: ["Auto (RGB)", "RGB", "BGR", "None (Grayscale)"]
  ```
  "Auto" label shows `SubpixelMode::for_display(scale, opacity)` result. "None" explicitly notes it falls back to grayscale rendering.

  **Subpixel positioning dropdown:**
  ```
  Items: ["Auto (Quarter-pixel)", "Quarter-pixel", "None"]
  ```

  **Atlas filtering dropdown:**
  ```
  Items: ["Auto (Linear)", "Linear", "Nearest"]
  ```
  "Auto" label shows `AtlasFiltering::from_scale_factor(scale)` result.

- [ ] The `build_page()` function needs access to the current scale factor and opacity to compute Auto labels. This requires a signature change cascade:

  1. Add `scale_factor: f64` and `opacity: f64` parameters to `build_settings_dialog()` in `form_builder/mod.rs`
  2. Pass them through to `font::build_page()` (and only font — other pages don't need them)
  
  3. Update all 14 call sites of `build_settings_dialog()` (found via grep):
     - `dialog_management.rs:402` — production dialog opening. `build_settings_content()` is `&self` on `App`, so use the first window's scale factor via `self.windows.values().next().map(|ctx| ctx.window.scale_factor().factor()).unwrap_or(1.0)` and `f64::from(self.config.window.effective_opacity())`
     - `dialog_context/content_actions.rs:130` — settings reset. Has `ctx.scale_factor.factor()` for the dialog window's DPI and `pending_config.window.effective_opacity()` for opacity
     - `test_support.rs:52` — visual regression test helper (use `1.0, 1.0`)
     - `action_handler/tests.rs:13` — 1 call via `default_ids()` (use `1.0, 1.0`)
     - `form_builder/tests.rs` — 10 call sites at lines 14, 21, 32, 40, 53, 67, 90, 109, 133, 152 (all use `1.0, 1.0`)
     Production call sites get real scale_factor + opacity. Test call sites pass `1.0` for both (at 1.0x, auto-detection gives Full hinting, RGB subpixel, Linear filtering — same as current defaults).
  4. In `font::build_page()`, use the scale factor to compute Auto labels: `HintingMode::from_scale_factor(scale)`, `SubpixelMode::for_display(scale, opacity)`, `AtlasFiltering::from_scale_factor(scale)`.

### 02.3.3 Wire action handler

- [ ] In `action_handler/mod.rs`, add a new `handle_font_advanced()` function called from `handle_settings_action()` (do NOT add to `handle_font()` — it is already 41 lines, and adding 4 match arms would push it past the 50-line function limit). Match the 4 new dropdown IDs:

  ```rust
  // Hinting: 0=Auto (config=None), 1=Full, 2=None
  WidgetAction::Selected { id, index } if *id == ids.hinting_dropdown => {
      config.font.hinting = match index {
          0 => None,           // Auto (scale-factor-based detection)
          1 => Some("full".to_owned()),
          _ => Some("none".to_owned()),
      };
      true
  }
  ```

  For subpixel_aa (maps to `config.font.subpixel_mode`):
  ```rust
  // 0=Auto (config=None), 1=RGB, 2=BGR, 3=None(Grayscale)
  WidgetAction::Selected { id, index } if *id == ids.subpixel_aa_dropdown => {
      config.font.subpixel_mode = match index {
          0 => None,
          1 => Some("rgb".to_owned()),
          2 => Some("bgr".to_owned()),
          _ => Some("none".to_owned()),
      };
      true
  }
  ```

  For atlas_filtering (maps to `config.font.atlas_filtering`):
  ```rust
  // 0=Auto (config=None), 1=Linear, 2=Nearest
  WidgetAction::Selected { id, index } if *id == ids.atlas_filtering_dropdown => {
      config.font.atlas_filtering = match index {
          0 => None,
          1 => Some("linear".to_owned()),
          _ => Some("nearest".to_owned()),
      };
      true
  }
  ```

  
  For subpixel_positioning (now `Option<bool>` per 02.1.2b): Auto=`None`, Quarter-pixel=`Some(true)`, None=`Some(false)`.
  ```rust
  WidgetAction::Selected { id, index } if *id == ids.subpixel_positioning_dropdown => {
      config.font.subpixel_positioning = match index {
          0 => None,              // Auto
          1 => Some(true),        // Quarter-pixel
          _ => Some(false),       // None
      };
      true
  }
  ```

- [ ] Wire `handle_font_advanced` into the `handle_settings_action()` dispatch chain at line 29: add `|| handle_font_advanced(action, ids, config)` after `handle_font(action, ids, config)`.

### Tests (02.3)

**Form builder tests** go in `form_builder/tests.rs`:

- [ ] Update `settings_ids_all_distinct` — update expected count (see 02.3.1).
- [ ] Update `collect_ids()` helper — add the 4 new dropdown IDs to the `HashSet` insertion list.
- [ ] `test_advanced_dropdowns_default_to_auto` — build dialog, verify all 4 Advanced dropdown IDs are non-placeholder and their initial selected index is 0 (Auto).

**Action handler tests** go in `action_handler/tests.rs`:

- [ ] `test_hinting_dropdown_auto_sets_none` — select index 0, assert `config.font.hinting == None`.
- [ ] `test_hinting_dropdown_full_sets_full` — select index 1, assert `config.font.hinting == Some("full")`.
- [ ] `test_hinting_dropdown_none_sets_none_str` — select index 2, assert `config.font.hinting == Some("none")`.
- [ ] `test_subpixel_aa_dropdown_auto_sets_none` — select index 0, assert `config.font.subpixel_mode == None`.
- [ ] `test_subpixel_aa_dropdown_rgb` — select index 1, assert `config.font.subpixel_mode == Some("rgb")`.
- [ ] `test_subpixel_aa_dropdown_bgr` — select index 2, assert `config.font.subpixel_mode == Some("bgr")`.
- [ ] `test_subpixel_aa_dropdown_none_grayscale` — select index 3, assert `config.font.subpixel_mode == Some("none")`.
- [ ] `test_subpixel_positioning_dropdown_auto` — select index 0, assert `config.font.subpixel_positioning == None`.
- [ ] `test_subpixel_positioning_dropdown_quarter_pixel` — select index 1, assert `config.font.subpixel_positioning == Some(true)`.
- [ ] `test_subpixel_positioning_dropdown_off` — select index 2, assert `config.font.subpixel_positioning == Some(false)`.
- [ ] `test_atlas_filtering_dropdown_auto` — select index 0, assert `config.font.atlas_filtering == None`.
- [ ] `test_atlas_filtering_dropdown_linear` — select index 1, assert `config.font.atlas_filtering == Some("linear")`.
- [ ] `test_atlas_filtering_dropdown_nearest` — select index 2, assert `config.font.atlas_filtering == Some("nearest")`.

**Note on test harness updates:** All tests in `action_handler/tests.rs` use `default_ids()` which calls `build_settings_dialog`. After the signature change in 02.3.2, `default_ids()` must pass `1.0, 1.0` for scale_factor and opacity.

- [ ] `/tpr-review` checkpoint — new UI controls and action wiring.

---

## 02.4 Remove Rendering Page Subpixel Toggle

**File(s):** `oriterm/src/app/settings_overlay/form_builder/rendering.rs`, `oriterm/src/app/settings_overlay/form_builder/mod.rs`, `oriterm/src/app/settings_overlay/action_handler/mod.rs`

The Rendering page's subpixel toggle is superseded by the richer "Subpixel AA" dropdown in the Font page's Advanced section.

- [ ] Remove `build_text_section()` from `rendering.rs` and its call from `build_page()`. The "Text" section header and subpixel toggle are both removed. Also remove the now-unused `use oriterm_ui::widgets::toggle::ToggleWidget;` import and update the module doc comment from "GPU backend and text rendering settings" to "GPU backend settings" (the `dead_code = "deny"` lint would catch the unused import).

- [ ] Remove `subpixel_toggle` from `SettingsIds` in `form_builder/mod.rs`. Also remove from `SettingsIds::placeholder()`.

- [ ] Remove the `WidgetAction::Toggled` match arm for `subpixel_toggle` from `handle_rendering()` in `action_handler/mod.rs`. Update the doc comment from "GPU backend, subpixel toggle" to "GPU backend".

- [ ] Remove the existing `subpixel_toggled_updates_config` test from `action_handler/tests.rs` (line 394) — it references the now-removed `ids.subpixel_toggle` field and will fail to compile.

- [ ] In `per_page_dirty()` in `settings_overlay/mod.rs`, remove `|| pending.font.subpixel_mode != original.font.subpixel_mode` from the Rendering page entry (index 7, line 56). Change the comment at line 54 from "GPU backend, subpixel mode" to "GPU backend". The Font page's `pending.font != original.font` comparison at line 40 already catches all font field changes including `subpixel_mode`.

### Tests (02.4)

Tests go in `settings_overlay/tests.rs`:

- [ ] `test_subpixel_mode_change_dirties_font_not_rendering` — change `config.font.subpixel_mode` from `None` to `Some("rgb")`, verify `per_page_dirty[2]` (Font) is `true` and `per_page_dirty[7]` (Rendering) is `false`.
- [ ] `test_gpu_backend_change_still_dirties_rendering` — change `config.rendering.gpu_backend`, verify `per_page_dirty[7]` (Rendering) is still `true` (only the `subpixel_mode` check was removed, not the GPU backend check).

Tests in `action_handler/tests.rs`:

- [ ] `test_subpixel_toggle_removed` — verify that `handle_settings_action` returns `false` for a `Toggled` action with a random `WidgetId::unique()` (since `subpixel_toggle` no longer exists, no `Toggled` handler in `handle_rendering()` should match). This is a compile-time regression guard: if anyone tries to re-add a `subpixel_toggle` field, the test name reminds them it was intentionally removed.

Tests in `form_builder/tests.rs`:

- [ ] Verify `settings_ids_all_distinct` count is updated to final value (26 - 1 removed + 4 new = 29 fixed controls).

---

## 02.R Third Party Review Findings

- None.

---

## 02.N Completion Checklist

- [ ] `window_renderer/mod.rs` reduced below 500 lines (CombinedAtlasLookup extracted) before adding fields
- [ ] `frame_input/mod.rs` reduced below 500 lines (`cell_in_search_match` extracted) before adding `subpixel_positioning` field
- [ ] `font.atlas_filtering` config field parses from TOML with `None`/`"linear"`/`"nearest"` values
- [ ] `font.subpixel_positioning` changed from `bool` to `Option<bool>` (None=auto, Some(true)=on, Some(false)=off)
- [ ] `subpixel_positioning` config is consumed by the renderer (no longer dead — TPR-04-007 resolved)
- [ ] Atlas sampler responds to `atlas_filtering` setting (verified: switch to Nearest, glyphs render crisper)
- [ ] `AtlasBindGroup` stores `FilterMode` and `rebuild()` recreates sampler from stored filter
- [ ] `set_atlas_filtering()` snapshots atlas generations to prevent redundant rebuild
- [ ] `apply_font_changes()` change detection includes `subpixel_positioning` and `atlas_filtering`
- [ ] Font page shows "Advanced" section with 4 dropdowns
- [ ] Each dropdown defaults to "Auto (detected-value)" at index 0
- [ ] `build_settings_dialog()` accepts and threads `scale_factor` + `opacity` for Auto labels
- [ ] `handle_font_advanced()` extracted as separate function (not added to `handle_font()`) and wired into dispatch chain
- [ ] Changing any dropdown persists to TOML config on Save
- [ ] Changing hinting/subpixel AA triggers font re-rasterization (atlas clear + re-cache)
- [ ] Changing atlas filtering triggers bind group rebuild (no atlas clear needed)
- [ ] Changing subpixel positioning affects both grid and UI text glyph emission
- [ ] `FrameInput.subpixel_positioning` field added and wired from renderer at extraction sites
- [ ] DPI change handlers re-resolve atlas filtering (scale factor dependency)
- [ ] Rendering page no longer shows subpixel toggle
- [ ] 02.1 tests: 13 new tests + 3 updated tests in `config/tests.rs`
- [ ] 02.2 tests: 5 AtlasFiltering enum unit tests + 3 AtlasBindGroup GPU tests in `bind_groups/tests.rs`, 2 subpixel tests in `prepare/tests.rs`, 2 raster key tests in `window_renderer/tests.rs`, 1 UI text test in `scene_convert/tests.rs`, 2 change detection tests in `settings_overlay/tests.rs`
- [ ] 02.3 tests: 1 form builder test + 13 action handler tests
- [ ] 02.4 tests: 2 in `settings_overlay/tests.rs`, 1 in `action_handler/tests.rs`, 1 count update in `form_builder/tests.rs`
- [ ] `timeout 150 cargo test -p oriterm` green
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** All 4 Advanced font settings are functional in the dialog, persist to config, and produce visible rendering changes. The dead `subpixel_positioning` config is wired. The redundant Rendering page subpixel toggle is removed.
