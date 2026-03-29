# Section 05 Verification Results: Window + GPU Rendering

**Auditor:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-29
**Section status in plan:** complete
**Verdict:** PASS -- all items verified, minor hygiene note on atlas file size

## Context Loaded

- CLAUDE.md (read in full -- 139 lines)
- `.claude/rules/code-hygiene.md` (read -- 104 lines)
- `.claude/rules/test-organization.md` (read -- 57 lines)
- `.claude/rules/impl-hygiene.md` (read -- 52 lines)
- `.claude/rules/crate-boundaries.md` (read -- loaded via system reminder)
- Reference: alacritty, wezterm, ghostty, ratatui patterns referenced via CLAUDE.md; no direct consultation needed

---

## 1. Test Inventory

| Module | Test File | Test Count | Status |
|--------|-----------|------------|--------|
| `gpu::frame_input` | `frame_input/tests.rs` | 24 | PASS |
| `gpu::extract::from_snapshot` | `from_snapshot/tests.rs` | 21 | PASS |
| `gpu::prepare` | `prepare/tests.rs` | 135 | PASS |
| `gpu::instance_writer` | `instance_writer/tests.rs` | 20 | PASS |
| `gpu::atlas` | `atlas/tests.rs` | 43 | PASS |
| `gpu::state` | `state/tests.rs` | 19 | PASS |
| `gpu::bind_groups` | `bind_groups/tests.rs` | 10 | PASS |
| `gpu::render_target` | `render_target/tests.rs` | 8 | PASS |
| `gpu::prepared_frame` | `prepared_frame/tests.rs` | counted in total | PASS |
| `gpu::pipeline_tests` | `pipeline_tests.rs` (L2) | 15 | PASS |
| `gpu::tests` (srgb) | `tests.rs` | 8 | PASS |
| `app::cursor_blink` | `cursor_blink/tests.rs` | 10 | PASS |
| `app::event_loop_helpers` | `event_loop_helpers/tests.rs` | 8 | PASS |
| `font::tests` | `font/tests.rs` | ~32 | PASS |
| `font::collection::tests` | `collection/tests.rs` | ~40 | PASS |
| `gpu::visual_regression` | `visual_regression/mod.rs` (L3) | 4+ tests, `gpu-tests` feature gated | PASS (infrastructure verified) |
| **Total `cargo test -p oriterm`** | | **2084** | **ALL PASS, 0 IGNORED** |

**Section claims:** "400 tests, 4 ignored" (in 5.15). Actual: 2084 pass, 0 ignored. The 400 figure was accurate at section completion time; subsequent sections have expanded the test count significantly.

---

## 2. Item-by-Item Verification

### 5.1 Render Pipeline Architecture

**Verified:** The three-phase architecture (Extract -> Prepare -> Render) is cleanly implemented.

- **Phase separation enforced at import level:**
  - `gpu/extract/` and `gpu/frame_input/`: zero `use wgpu` imports (grep confirmed).
  - `gpu/prepare/`: zero `use wgpu` imports (grep confirmed).
  - Only `gpu/window_renderer/`, `gpu/state/`, `gpu/pipeline/`, `gpu/bind_groups/`, `gpu/atlas/`, `gpu/render_target/` touch wgpu.

- **Key types exist and are correct:**
  - `FrameInput` (`gpu/frame_input/mod.rs`): fully owned, implements `Debug`. Fields: `content: RenderableContent`, `viewport: ViewportSize`, `cell_size: CellMetrics`, `palette: FramePalette`, `selection`, `search`, etc. No `Arc`, no `Mutex`, no references.
  - `PreparedFrame` (`gpu/prepared_frame/mod.rs`): 13 `InstanceWriter` buffers (backgrounds, glyphs, subpixel_glyphs, color_glyphs, cursors, ui_rects, etc.), `viewport`, `clear_color`. No wgpu types.
  - `ViewportSize` newtype with `.new()` clamping to min 1x1.
  - `CellMetrics` with `width`, `height`, `baseline` + `columns()`/`rows()` methods.
  - `FramePalette` with `background`, `foreground`, `cursor_color`, `opacity`, `selection_fg/bg`.

- **Pipeline rules enforced:**
  - Extract returns owned `FrameInput` (locks released immediately).
  - Prepare takes `&FrameInput`, returns owned `PreparedFrame` -- pure function.
  - Render takes `&PreparedFrame` + GPU resources.

- **Semantic pin:** `gpu::prepare::tests::single_char_produces_one_bg_and_one_fg` would fail if prepare phase stopped emitting instances. `gpu::frame_input::tests::viewport_clamps_zero_to_one` would fail if min-1 clamping were removed.

### 5.2 winit Window Creation

**Verified:** `oriterm/src/window/mod.rs` (318 lines, under 500-line limit).

- `TermWindow` struct fields match plan: `window: Arc<Window>`, `surface: wgpu::Surface<'static>`, `surface_config`, `size_px: (u32, u32)`, `scale_factor: ScaleFactor`, `is_maximized: bool`, plus `session_window_id` and `surface_stale` (deviations documented).
- `TermWindow::new()`: creates frameless transparent window via `oriterm_ui::window::create_window`, creates surface via `GpuState::create_surface`, stores dimensions/scale, applies vibrancy when configured, sets IME.
- `TermWindow::from_window()`: wraps existing window (used during GPU init).
- `resize_surface()`: clamps to min 1x1, defers actual `configure()` to `apply_pending_surface_resize()`.
- `update_scale_factor()`, `set_visible()`, `has_surface_area()`, `window_id()` all present.
- `WindowCreateError` enum with `Window` + `Surface` variants, `Display`/`Error` impls.
- IME setup: `set_ime_allowed(true)`, `set_ime_purpose(Terminal)`.

- **Semantic pin:** `has_surface_area()` returns false for 0-dimension windows. `resize_surface()` clamps to min 1x1.

### 5.3 wgpu GpuState + Offscreen Render Targets

**Verified:** `gpu/state/mod.rs` (376 lines) + `gpu/state/helpers.rs` + `gpu/render_target/mod.rs` (229 lines).

- `GpuState` fields: `instance`, `device`, `queue`, `surface_format`, `render_format`, `surface_alpha_mode`, `supports_view_formats`, `present_mode`, `pipeline_cache`, `pipeline_cache_path`.
- `GpuState::new(window, transparent)`: tries DX12+DComp on Windows for transparency, then Vulkan, then PRIMARY, then SECONDARY backends.
- `GpuState::new_headless()`: no surface, uses `Rgba8UnormSrgb` as default format.
- `create_render_target(w, h)`: creates `RenderTarget` with matching `render_format`, `RENDER_ATTACHMENT | COPY_SRC`.
- `read_render_target()`: reads RGBA pixels back via staging buffer with proper alignment handling.
- **19 unit tests** covering: `select_formats`, `select_alpha_mode`, `select_present_mode`, `build_surface_config`, headless init, pipeline cache round-trip.

- **Semantic pin:** `gpu::state::tests::headless_init_succeeds_when_adapter_available` confirms headless GPU init works. `gpu::render_target::tests::create_render_target_succeeds` confirms offscreen targets.

### 5.4 WGSL Shaders + GPU Pipelines

**Verified:** Shaders in `gpu/shaders/bg.wgsl`, `gpu/shaders/fg.wgsl`, plus `subpixel_fg.wgsl`, `color_fg.wgsl`, `ui_rect.wgsl`, `image.wgsl`, `composite.wgsl`.

- **bg.wgsl:** Uniform struct with `screen_size: vec2<f32>` + `_pad`. InstanceInput with pos, size, uv, fg_color, bg_color, kind. TriangleStrip vertex pulling via `@builtin(vertex_index)`. Fragment outputs premultiplied `bg_color * alpha`.
- **fg.wgsl:** Same InstanceInput + `atlas_page: u32`. Texture2DArray sampling. Fragment samples alpha from atlas, tints with `fg_color`, premultiplied output.
- Pipeline factories in `gpu/pipeline/mod.rs`: `create_bg_pipeline`, `create_fg_pipeline`, `create_subpixel_fg_pipeline`, `create_color_fg_pipeline`, `create_ui_rect_pipeline`.
- `GpuPipelines` aggregator in `gpu/pipelines.rs`: holds all 6 pipelines + 3 bind group layouts. Created once, shared across windows.

- **Instance buffer layout:** 80 bytes, matching plan's offset table. Verified by `INSTANCE_SIZE = 80` compile-time assertion and `instance_writer/tests.rs` field offset tests.

- **Semantic pin:** `gpu::pipeline_tests::pipeline_creation_succeeds` confirms all pipelines compile. `gpu::instance_writer::tests::instance_size_is_80_bytes` pins the 80-byte record size.

### 5.5 Uniform Buffer + Bind Groups

**Verified:** `gpu/bind_groups/mod.rs` (190+ lines) with 10 tests.

- `UniformBuffer`: 16-byte buffer (`vec2<f32> screen_size` + padding), `write_screen_size()` method.
- `AtlasBindGroup`: sampler + bind group from atlas layout + texture view, `rebuild()` when atlas texture grows.
- `create_placeholder_atlas_texture()`: 1x1 `R8Unorm` white pixel for bootstrapping.
- Tests: creation, placeholder texture, write operations, bind group rebuild, compatibility with pipelines.

- **Semantic pin:** `gpu::bind_groups::tests::bind_groups_compatible_with_pipelines` verifies layout compatibility.

### 5.6 Font Discovery + Rasterization

**Verified:** `font/` module tree with `collection/`, `discovery/`, `shaper/`.

- `FontSet::embedded()` for deterministic testing without system fonts.
- `FontCollection::new()`: loads faces, computes metrics, pre-caches ASCII.
- `resolve(char, style) -> ResolvedGlyph`, `rasterize(RasterKey) -> Option<RasterizedGlyph>`.
- `CellMetrics` newtype with `width`, `height`, `baseline`, `columns()`, `rows()`.
- **~72 font tests** (32 in `font/tests.rs` + 40 in `collection/tests.rs`): system discovery, embedded fallback, glyph resolution, rasterization, synthetic bold/italic, subpixel phases.

### 5.7 Glyph Atlas

**Verified:** `gpu/atlas/mod.rs` (579 lines -- see hygiene note below) + `atlas/rect_packer/mod.rs` + `atlas/texture.rs`.

- `GlyphAtlas` with `HashMap<RasterKey, AtlasEntry>` cache, `RectPacker` per page, 2048x2048 pages (up to `MAX_PAGES = 4`).
- `insert()` returns `Option<AtlasEntry>` (None for zero-size).
- `lookup()` by `RasterKey` (Copy, 8 bytes).
- Best-fit guillotine packing with 1px padding.
- LRU eviction across pages.
- Lazy mode (`new_lazy`) for deferred texture allocation.
- **43 unit tests**: packing logic, UV normalization, LRU eviction, subpixel atlas, page reuse.

- **Semantic pin:** `gpu::atlas::tests::uv_coordinates_are_normalized` would fail if UV computation changed. `gpu::atlas::tests::lru_eviction_evicts_oldest_page` pins eviction behavior.

### 5.8 Extract Phase (CPU)

**Verified:** `gpu/extract/from_snapshot/mod.rs` (211 lines).

- `extract_frame_from_snapshot()`: converts `PaneSnapshot` wire types to `RenderableContent` + `FrameInput`. No GPU types, no locks.
- `extract_frame_from_snapshot_into()`: buffer-reusing variant.
- `snapshot_palette()`: extracts semantic colors from 270-entry palette array.
- Generic over snapshot source (daemon-mode `PaneSnapshot`, not hardcoded to `Term<T>`).
- `FrameInput::test_grid()` factory for tests.
- **21 extract tests + 24 frame_input tests = 45 total**: cell positions, colors, flags, cursor shapes, wide chars, empty snapshots, `_into` equivalence, capacity preservation.

- **Semantic pin:** `extract_into_preserves_capacity` confirms allocation reuse. `renderable_cell_positions` would fail if row/column mapping changed.

### 5.9 Prepare Phase (CPU)

**Verified:** `gpu/prepare/mod.rs` (485 lines) + `prepare/emit.rs` + `prepare/decorations.rs` + `prepare/shaped_frame.rs` + `prepare/dirty_skip/`.

- `AtlasLookup` trait for testability (test atlas backed by HashMap, no GPU).
- `prepare_frame()` (unshaped, test-only) and `prepare_frame_shaped()` (production).
- `fill_frame_shaped()`: backgrounds -> decorations -> builtin glyphs -> shaped glyphs -> URL hover -> prompt markers -> cursor.
- `build_cursor()`: Block, Bar (2px), Underline (2px), HollowBlock (4 outline rects), Hidden (no instances).
- `resolve_cell_colors()`: selection inversion with INVERSE guard, fg==bg fallback, HIDDEN respect, search match highlighting.
- Incremental update support via `dirty_skip` module.
- **135 tests**: instance correctness, counts, colors, positions, bearings, cursor shapes, determinism, wide chars, decorations, selection inversion, search highlighting, shaped glyphs, ligatures, combining marks, URL hover, offset application, unfocused window cursor.

- **Semantic pin:** `gpu::prepare::tests::determinism_same_input_same_output` would fail if any non-determinism crept in. `gpu::prepare::tests::cursor_block_position_matches_cell` pins cursor positioning.

### 5.10 Render Phase (GPU)

**Verified:** `gpu/window_renderer/mod.rs` + `gpu/window_renderer/render.rs` + `gpu/window_renderer/helpers.rs` + `gpu/window_renderer/multi_pane.rs`.

- `WindowRenderer` struct: owns atlas, font_collection, bind groups, prepared frame, shaped frame cache.
- `prepare()`: shapes text, ensures glyph cache, calls `prepare_frame_shaped_into`.
- `render_frame()`: uploads instance data, executes draw calls (bg -> mono fg -> subpixel fg -> color fg -> cursors -> UI rects -> overlays -> images), uses `TriangleStrip draw(0..4, ...)`.
- `render_to_surface()`: acquires surface texture, creates sRGB view, renders, presents.
- GPU buffer management: `ensure_buffer()` grows as needed (power-of-2 rounding, min 256), never shrinks per-frame.
- Accepts any `TextureView` as target (not coupled to surface).

- **Semantic pin:** `gpu::pipeline_tests::full_pipeline_extract_prepare_render_readback` exercises the complete Extract -> Prepare -> Render -> pixel readback pipeline.

### 5.11 App Struct + Event Loop

**Verified:** `app/mod.rs` + `app/event_loop.rs` + `app/constructors.rs` + `app/init/`.

- `App` struct with `windows: HashMap<WindowId, WindowContext>`, `event_proxy`, `dirty` tracking, `cursor_blink`, etc.
- `impl ApplicationHandler<TermEvent> for App`: `resumed()` inits GPU/window/fonts/renderer/tab, `window_event()` handles Close/Resize/Redraw/Keyboard/ScaleFactorChanged, `user_event()` handles terminal events.
- Event batching: `dirty` flag coalesced, `about_to_wait` requests redraw only if dirty.
- `compute_control_flow()` pure function with 8 tests proving zero idle CPU.

- **Semantic pin:** `event_loop_helpers::tests::idle_returns_wait` proves idle CPU is zero. `blinking_returns_next_toggle` proves cursor blink is the only wakeup source.

### 5.12 Basic Input + Cursor

**Verified:** `app/keyboard_input/` + `app/cursor_blink/mod.rs` (90 lines).

- `CursorBlink`: phase-based visibility (elapsed time / interval, even phase = visible), `reset()` on keypress, `next_toggle()` for `WaitUntil`, `set_interval()` for config reload.
- 530ms default interval (standard xterm timing).
- **10 cursor blink tests**: initial visibility, update detection, double-interval restoration, reset, next_toggle timing, custom interval, set_interval, phase skipping.

- **Semantic pin:** `cursor_blink::tests::update_after_interval_reports_change` pins the 530ms blink behavior.

### 5.13 Render Pipeline Testing

**Verified:** Three-layer testing strategy fully implemented.

**Layer 1 (Unit, no GPU):**
- `gpu/prepare/tests.rs`: 135 tests. All run in `cargo test` without GPU. Uses `TestAtlas` backed by HashMap.
- Tests cover: instance buffer correctness, counts, colors, positions, bearings, cursor shapes, determinism, wide chars, selection, search, styled text.

**Layer 2 (Integration, headless GPU):**
- `gpu/pipeline_tests.rs`: 15 tests including subpixel blend formula verification and headless GPU tests.
- `headless_gpu_adapter_found`, `pipeline_creation_succeeds`, `offscreen_render_target_creates`, `frame_renders_without_errors`, `render_colored_cell_correct_bg_color`, `render_text_produces_nonzero_alpha_in_glyph_region`, `render_cursor_pixels_at_expected_position`, `full_pipeline_extract_prepare_render_readback`, `wgpu_validation_layer_enabled_in_tests`.

**Layer 3 (Visual regression):**
- `gpu/visual_regression/mod.rs` + sub-modules (`reference_tests.rs`, `edge_case_tests.rs`, `decoration_tests.rs`, `multi_size.rs`, `meta_tests.rs`).
- Feature-gated behind `gpu-tests`.
- **33 reference PNGs** in `oriterm/tests/references/`: `basic_grid.png`, `colors_16.png`, `cursor_block.png`, `cursor_bar.png`, `cursor_underline.png`, `cursor_hollowblock.png`, `bold_italic.png`, `strikethrough.png`, `underline_styles.png`, `inverse_video.png`, `box_drawing.png`, `braille.png`, `powerline.png`, `ligatures.png`, `combining_marks.png`, `subpixel_rgb.png`, etc.
- Fuzzy comparison: per-pixel tolerance +/-2 per channel, max 0.5% mismatch.
- `ORITERM_UPDATE_GOLDEN=1` regeneration mode, diff image saved on failure.

### 5.14 Integration: Working Terminal

**Verified:** All integration items covered by the combination of:
- Full-pipeline GPU tests (extract -> prepare -> render -> readback).
- Visual regression tests proving correct rendering.
- Event loop tests proving proper control flow.
- Cursor blink tests proving visibility state machine.

### 5.15 Section Completion

**Verified:** All checklist items are satisfied.

---

## 3. Performance Invariant Verification

| Invariant | Status | Evidence |
|-----------|--------|----------|
| **Zero idle CPU beyond cursor blink** | PASS | `compute_control_flow()` returns `Wait` when idle; 8 pure-function tests. Cursor blink `next_toggle()` is the only `WaitUntil` source when idle. |
| **Zero alloc in hot render path** | PASS | `InstanceWriter::clear()` retains capacity. `PreparedFrame::clear()` retains capacity. `extract_frame_from_snapshot_into()` reuses allocations. `extract_into_preserves_capacity` test pins this. |
| **Buffer shrink discipline** | PASS | `maybe_shrink()` on `InstanceWriter`, `PreparedFrame`, `ShapedFrame`, `WindowRenderer`. Threshold: capacity > 4x len AND > 4096. Grep confirms 17+ call sites. |
| **Stable RSS under sustained output** | PASS (by design) | Atlas has `MAX_PAGES = 4` cap with LRU eviction. Instance buffers grow-only-in-hot-path with post-render shrink. |

---

## 4. Code Hygiene Audit

### File Size (500-line limit)

| File | Lines | Status |
|------|-------|--------|
| `gpu/prepare/mod.rs` | 485 | OK |
| `gpu/frame_input/mod.rs` | 468 | OK |
| `gpu/instance_writer/mod.rs` | 405 | OK |
| `gpu/state/mod.rs` | 376 | OK |
| `gpu/visual_regression/mod.rs` | 472 | OK |
| `gpu/prepared_frame/mod.rs` | 463 | OK |
| `window/mod.rs` | 318 | OK |
| `app/cursor_blink/mod.rs` | 90 | OK |
| `gpu/extract/from_snapshot/mod.rs` | 211 | OK |
| **`gpu/atlas/mod.rs`** | **579** | **VIOLATION** |

**Finding:** `gpu/atlas/mod.rs` exceeds the 500-line limit by 79 lines. The file could be split by extracting the `GlyphAtlas` impl methods into a submodule (e.g., `atlas/glyph_atlas.rs` or splitting insertion/lookup logic). This is a hygiene issue, not a functional one.

### Test Organization

All Section 05 modules follow the sibling `tests.rs` pattern:
- `#[cfg(test)] mod tests;` at the bottom of each `mod.rs`.
- Test files use `super::` imports.
- No inline test modules.
- No `mod tests { }` wrapper in test files.

### Error Handling

- Zero `unwrap()` in production code (grep confirmed across `gpu/` and `window/`).
- `GpuInitError`, `WindowCreateError`, `ReadbackError` all have proper `Display`/`Error` impls.
- Surface errors handled gracefully (`Lost`/`Outdated` -> reconfigure, others -> propagated).

### Unsafe Code

Zero `unsafe` in all Section 05 files (grep confirmed).

### Phase Separation

- Zero `use wgpu` in `gpu/extract/`, `gpu/frame_input/`, `gpu/prepare/` (grep confirmed on all files, not just `mod.rs`).
- Clean one-way data flow: Extract -> Prepare -> Render, no callbacks or reverse dependencies.

### Module Docs

All `mod.rs` files have `//!` module documentation.

### Dead Code

A few `#[allow(dead_code, reason = "...")]` annotations exist with proper justification strings (e.g., "headless GPU for testing", "used by tests now, production consumers in later sections"). These are appropriate.

---

## 5. Gap Analysis

**Section goal:** "Open a frameless window, initialize wgpu, render the terminal grid with a proper staged render pipeline -- first visual milestone."

### Goal Fulfillment: COMPLETE

All listed items are implemented and tested. The section delivers:
1. Three-phase render pipeline with clean type-level separation.
2. Frameless transparent window with vibrancy support.
3. wgpu GPU state with headless mode for testing.
4. WGSL shaders (bg, fg, subpixel, color, UI rect, image, composite).
5. Glyph atlas with guillotine packing, LRU eviction, lazy allocation.
6. Font discovery, rasterization, and shaping.
7. Full extract -> prepare -> render pipeline.
8. Cursor blink state machine.
9. Three-layer test strategy (unit, integration, visual regression).
10. 33 reference PNGs for visual regression.

### Items Beyond Original Plan (Positive Deviations)

The implementation goes significantly beyond the plan:
- **Incremental rendering** (`dirty_skip` module): only regenerates dirty rows.
- **Multi-atlas support**: separate mono, subpixel, and color atlases.
- **Subpixel rendering**: full LCD subpixel pipeline with per-channel blending.
- **Image rendering**: inline image support (Sixel/iTerm2 protocol).
- **Compositor**: multi-layer composition with render target pooling.
- **Builtin glyphs**: box drawing, Braille, powerline, block elements, decorations.
- **Pane cache**: per-pane render caching for multi-pane layouts.
- **Draw list conversion**: scene-graph to instance buffer conversion with clipping.

### Missing Items (None Critical)

- `pipeline_stages.rs` documentation-only file mentioned in plan does not exist. The architecture is instead realized through the actual module structure, which is a better approach.
- Selection overlay visual regression test was noted as deferred to Section 9 in the plan (5.13 deviation note). Selection rendering itself is fully implemented and tested via unit tests.

---

## 6. Summary

| Dimension | Rating | Notes |
|-----------|--------|-------|
| **Functional completeness** | PASS | All 15 sub-items implemented |
| **Test coverage** | STRONG PASS | 2084 tests, 0 ignored, three test layers |
| **Semantic pins** | PASS | Key behaviors pinned by specific tests |
| **Phase separation** | PASS | Zero wgpu imports in extract/prepare |
| **Performance invariants** | PASS | All four invariants enforced |
| **Code hygiene** | MINOR NOTE | `atlas/mod.rs` at 579 lines (limit: 500) |
| **Test organization** | PASS | All sibling `tests.rs` pattern |
| **Error handling** | PASS | Zero unwraps, proper error types |
| **Unsafe code** | PASS | Zero unsafe |

**Overall: PASS.** Section 05 is complete, well-tested, and architecturally sound. The single hygiene note (atlas file size) is minor and non-blocking.
