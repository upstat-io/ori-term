---
section: 24
title: Visual Polish
status: in-progress
tier: 6
goal: Cursor blinking, hide-while-typing, minimum contrast, HiDPI, vector icons, background images, gradients, backdrop effects, scrollable menus
sections:
  - id: "24.1"
    title: Cursor Blinking
    status: complete
  - id: "24.2"
    title: Hide Cursor While Typing
    status: not-started
  - id: "24.3"
    title: Minimum Contrast
    status: not-started
  - id: "24.4"
    title: HiDPI & Display Scaling
    status: in-progress
  - id: "24.5"
    title: Vector Icon Pipeline (tiny_skia)
    status: not-started
  - id: "24.6"
    title: Background Images
    status: not-started
  - id: "24.7"
    title: Background Gradients
    status: not-started
  - id: "24.8"
    title: Window Backdrop Effects
    status: in-progress
  - id: "24.9"
    title: Scrollable Menus
    status: not-started
  - id: "24.10"
    title: Section Completion
    status: not-started
---

# Section 24: Visual Polish

**Status:** In Progress (24.1 cursor blink, 24.4 HiDPI, 24.8 backdrop effects are partially implemented)
**Goal:** Small visual features that collectively create a polished, modern feel. Each is low-to-medium effort but highly visible. These are the details people notice in the first 5 minutes. Missing cursor blink, broken HiDPI, or unreadable colors are dealbreakers.

**Crate:** `oriterm` (app layer + GPU rendering in `oriterm/src/gpu/`), `oriterm_ui` (widgets)
**Dependencies:** `image` (for background images -- currently build/dev only, needs runtime dep), `tiny-skia` (already in Cargo.toml), `window-vibrancy` (already in Cargo.toml), existing wgpu pipeline

**Internal dependencies between subsections:**
- 24.1 (Cursor Blinking) -- standalone
- 24.2 (Hide Cursor) -- standalone
- 24.3 (Minimum Contrast) -- standalone, but the uniform buffer change here affects 24.6/24.7 if they also add uniforms
- 24.4 (HiDPI) -- standalone, but 24.5 (icons) depends on DPI scale factor from this subsection
- 24.5 (Vector Icons) -- depends on 24.4 for scale factor; complete before 24.6/24.7 to avoid redoing icon rendering
- 24.6 (Background Images) -- depends on 24.3 uniform buffer layout (coordinate buffer layout if done first)
- 24.7 (Background Gradients) -- depends on 24.6 (shared render pass insertion point)
- 24.8 (Backdrop Effects) -- standalone (compositor-level via `window-vibrancy`/DWM, independent of GPU render passes)
- 24.9 (Scrollable Menus) -- standalone (`oriterm_ui` only, no GPU changes)

**Recommended implementation order:** 24.1 → 24.2 → 24.9 → 24.3 → 24.4 → 24.5 → 24.6 → 24.7 → 24.8

---

## 24.1 Cursor Blinking

Toggle cursor visibility on a timer. **Partially implemented**: `CursorBlink` state machine exists (`oriterm/src/app/cursor_blink/mod.rs`) with toggle, reset, and `next_toggle()`. The `about_to_wait` handler drives blink via `ControlFlow::WaitUntil`. `cursor_blink_visible` is threaded through the prepare pipeline. Remaining work: focus handling (unfocused hollow cursor), mouse click reset, and PTY cursor-move reset.

**File:** `oriterm/src/app/cursor_blink/mod.rs` (blink state -- exists), `oriterm/src/app/event_loop.rs` (`about_to_wait` blink timer -- exists), `oriterm/src/app/redraw/mod.rs` (cursor blink visible flag -- exists), `oriterm/src/gpu/prepare/mod.rs` (cursor emission gating -- exists)

- [x] Blink state tracking:
  - [x] `CursorBlink` struct with `visible`, `phase_start`, `interval` fields on `App`
  - [x] Blink interval: 530ms on / 530ms off (configurable via `cursor_blink_interval_ms`)
  - [x] `update()` toggles visibility when interval elapses
- [x] DECSCUSR blinking style detection:
  - [x] DECSCUSR values 1 (blinking block), 3 (blinking underline), 5 (blinking bar) enable blink
  - [x] Even values (2, 4, 6) = steady -- no blink
  - [x] Default (0) = implementation-defined -- follow config
  - [x] `TermMode::CURSOR_BLINKING` flag set/cleared by DECSCUSR handler; `blinking_active` cached on `App`
- [x] Reset blink to visible on keypress:
  - [x] `cursor_blink.reset()` called from keyboard input handler (resets deadline to now + interval)
- [x] Reset blink to visible on PTY cursor movement:
  - [x] Compare `last_cursor_pos` between frames in `handle_redraw()` and `handle_redraw_multi_pane()`; reset blink only on actual position change (not on every PTY byte)
- [x] Reset blink to visible on mouse click in grid:
  - [x] `cursor_blink.reset()` called from `handle_mouse_input()` on any grid press (after overlays/chrome/dividers return early)
- [x] Timer implementation using winit:
  - [x] `ControlFlow::WaitUntil(cursor_blink.next_toggle())` in `about_to_wait` when `blinking_active`
  - [x] `about_to_wait` calls `cursor_blink.update()`, marks dirty, sets next deadline
  - [x] When no blink needed: falls through to default `ControlFlow::Wait`
- [x] Renderer integration:
  - [x] `cursor_blink_visible` passed to `WindowRenderer::prepare()` and `prepare_frame_shaped_into()`
  - [x] `prepare/mod.rs` skips cursor emission when `!cursor_blink_visible`
- [x] Focus handling:
  - [x] Window loses focus (`WindowEvent::Focused(false)` in `event_loop.rs`): set `blinking_active = false` and call `cursor_blink.reset()` to freeze cursor visible
  - [x] Window gains focus (`WindowEvent::Focused(true)` in `event_loop.rs`): re-evaluate `blinking_active` from pane's `TermMode::CURSOR_BLINKING` and call `cursor_blink.reset()`
  - [x] Unfocused window renders cursor as hollow block (outline only). `window_os_focused` on `App` tracks OS focus; `window_focused: bool` on `FrameInput` propagated to both shaped and unshaped prepare paths
- [x] Config: `terminal.cursor_blink` (default: true) -- exists as `TerminalConfig::cursor_blink`
- [x] Config: `terminal.cursor_blink_interval_ms` (default: 530) -- exists as `TerminalConfig::cursor_blink_interval_ms`

**Tests:**
- [x] Blink state toggles after interval elapses (`update_after_interval_toggles`)
- [x] Keypress resets blink to visible (`reset_makes_visible`)
- [x] Even DECSCUSR values disable blinking (`decscusr_fires_cursor_blinking_change_event`)
- [x] Odd DECSCUSR values enable blinking (oriterm_core handler tests)
- [x] Focus loss sets `blinking_active = false` and freezes cursor visible
- [x] Mouse click in grid resets blink to visible
- [x] Unfocused window renders hollow block cursor (`unfocused_window_renders_hollow_cursor`, `unfocused_window_bar_cursor_becomes_hollow`, `focused_window_renders_block_cursor`)

---

## 24.2 Hide Cursor While Typing

Mouse cursor disappears when typing, reappears on mouse move.

**File:** `oriterm/src/app/mod.rs` (state), `oriterm/src/app/keyboard_input/mod.rs` (keypress hiding), `oriterm/src/app/event_loop.rs` (`CursorMoved` restore)

- [ ] Hide mouse cursor on keypress:
  - [ ] Track `mouse_cursor_hidden: bool` on `App`
  - [ ] On `KeyboardInput` with `ElementState::Pressed`: call `self.focused_ctx()?.window.window().set_cursor_visible(false)` and set `mouse_cursor_hidden = true`
  - [ ] Skip modifier-only keys (`NamedKey::Shift`, `NamedKey::Control`, `NamedKey::Alt`, `NamedKey::Super`) -- only hide on character-producing or action key presses
  - [ ] Only hide if mouse is over the grid area (not tab bar or resize border)
- [ ] Restore mouse cursor on mouse move:
  - [ ] On `CursorMoved` event: if `mouse_cursor_hidden`, call `window.set_cursor_visible(true)` and set `mouse_cursor_hidden = false`
  - [ ] Skip the `set_cursor_visible(true)` call when `!mouse_cursor_hidden` (avoid redundant winit calls)
- [ ] Suppress hiding during mouse reporting mode:
  - [ ] Check `TermMode::ANY_MOUSE` (composite of `MOUSE_REPORT_CLICK | MOUSE_DRAG | MOUSE_MOTION | MOUSE_X10`)
  - [ ] If any mouse mode is active, do not hide the cursor
- [ ] Config: add `hide_mouse_when_typing: bool` (default: true) to `BehaviorConfig` in `oriterm/src/config/behavior.rs`
  - [ ] Gate all hide/show logic on `self.config.behavior.hide_mouse_when_typing`

**Tests:** Extract decision logic into a pure function (`should_hide_cursor(...)`) testable without a winit `Window`. Test in `oriterm/src/app/tests.rs` or a `cursor_hide` submodule.
- [ ] Keypress with `hide_mouse_when_typing = true` returns `should_hide = true`
- [ ] Mouse move with `mouse_cursor_hidden = true` resets to `false`
- [ ] `ANY_MOUSE` mode active prevents hiding even when config is true
- [ ] `hide_mouse_when_typing = false` disables the feature entirely
- [ ] Modifier-only keypress (Shift, Ctrl, Alt, Super) does not trigger hiding

---

## 24.3 Minimum Contrast

**COMPLEXITY WARNING:** This subsection modifies the shared uniform buffer and 7 WGSL shader files in lockstep, and introduces WCAG math in WGSL. Build the Rust reference implementation and validate it thoroughly before touching any shaders. Test edge cases (HIDDEN cells, reverse video, bold-bright) in Rust first.

Ensure text is always readable regardless of color scheme. WCAG 2.0 contrast enforcement in the GPU shader.

**File:** `oriterm/src/gpu/contrast/mod.rs` (new -- Rust reference impl for WCAG luminance/contrast), `oriterm/src/gpu/contrast/tests.rs` (new -- unit tests), `oriterm/src/gpu/shaders/fg.wgsl` + `oriterm/src/gpu/shaders/subpixel_fg.wgsl` (WGSL shaders -- contrast enforcement), `oriterm/src/config/color_config.rs` (config -- field already exists), `oriterm/src/gpu/bind_groups/mod.rs` (uniform buffer write)

**Reference:** Ghostty's minimum contrast feature (see `ghostty/src/renderer/shaders/`), iTerm2's minimum contrast slider

- [x] Config: `colors.minimum_contrast` (range 1.0 disabled to 21.0 maximum, default 1.0) -- already exists in `ColorConfig`
  - [x] `effective_minimum_contrast()` clamps value -- already implemented
- [ ] **Step 1: Rust reference implementation** -- create `oriterm/src/gpu/contrast/mod.rs` with pure functions:
  - [ ] `luminance(r: f32, g: f32, b: f32) -> f32` -- ITU-R BT.709 relative luminance from linear RGB
  - [ ] `contrast_ratio(l1: f32, l2: f32) -> f32` -- WCAG formula `(L1 + 0.05) / (L2 + 0.05)`
  - [ ] `contrasted_color(min_contrast: f32, fg: [f32; 4], bg: [f32; 4]) -> [f32; 4]` -- adjusts fg toward white or black to meet ratio. Binary search for minimum alpha mix. Tries white first (dark backgrounds), then black (light backgrounds). Picks whichever achieves better contrast
  - [ ] Add `pub(crate) mod contrast;` to `gpu/mod.rs`
  - [ ] Write and pass all unit tests (Step 3 below) before proceeding to shaders
- [ ] **Step 2: Uniform buffer update** -- repurpose `_pad.x` for `min_contrast`:
  - [ ] Rename `UniformBuffer::write_screen_size()` to `write_uniforms()`, add `min_contrast: f32` parameter. Write `min_contrast` at bytes 8--11 (the current zero-padded `_pad.x` slot). Update ALL callers (grep for `write_screen_size`)
  - [ ] Rename `_pad: vec2<f32>` to `extra: vec2<f32>` in the `Uniform` struct across all 7 shaders that use the shared layout: `fg.wgsl`, `bg.wgsl`, `subpixel_fg.wgsl`, `color_fg.wgsl`, `ui_rect.wgsl`, `image.wgsl`, `composite.wgsl`. (`colr_solid.wgsl` and `colr_gradient.wgsl` use separate uniform structs and are not affected.) Buffer size stays 16 bytes
- [ ] **Step 3: WGSL shader port** -- port the validated Rust functions into `fg.wgsl` and `subpixel_fg.wgsl`:
  - [ ] Add `luminance()`, `contrast_ratio()`, `contrasted_color()` as WGSL functions
  - [ ] Apply contrast in vertex shader (`vs_main`), not fragment shader -- `bg_color` is only available as a per-instance attribute, not in fragment stage. Per-vertex is sufficient since each cell is one quad with uniform fg/bg:
    ```wgsl
    out.fg_color = contrasted_color(uniforms.extra.x, input.fg_color, input.bg_color);
    ```
  - [ ] Only apply in `fg.wgsl` and `subpixel_fg.wgsl` (text shaders). `bg.wgsl`, `ui_rect.wgsl`, `image.wgsl` do not render text. `color_fg.wgsl` (emoji/color glyphs): skip contrast -- adjusting bitmap colors would distort them
- [ ] Hot-reload: `minimum_contrast` value read from `config.colors.effective_minimum_contrast()` each frame, so config changes apply immediately

**Edge cases:**
- [ ] `minimum_contrast = 1.0` (default, disabled): shader short-circuits -- no luminance computation, pass fg through unchanged. Cost: one branch per vertex, zero overhead when disabled
- [ ] HIDDEN cells (SGR 8): do not reveal. HIDDEN cells already have `fg == bg` (set by `resolve_cell_colors()`). The shader's `contrasted_color()` must detect `fg == bg` and skip adjustment. Do NOT use `fg_color.a = 0.0` as a signal (it would break premultiplied alpha blending for all text)
- [ ] Reverse video cells (SGR 7): contrast uses the already-swapped fg/bg (no special handling needed)
- [ ] Bold/dim flags: contrast applied after bold-bright and dim adjustments (`resolve_cell_colors` output is what the shader sees, no special handling needed)

**Tests:** (in `oriterm/src/gpu/contrast/tests.rs`)
- [ ] White on black at `minimum_contrast = 1.0` passes through unchanged
- [ ] Dark gray (#333) on black at `minimum_contrast = 4.5` adjusts fg to a lighter color
- [ ] Light gray (#ccc) on white at `minimum_contrast = 4.5` adjusts fg to a darker color
- [ ] ITU-R BT.709 luminance: verify correct relative luminance for pure red, green, blue, white, black
- [ ] `contrast_ratio(white, black)` approximately equals 21.0; `contrast_ratio(#777, #000)` approximately equals 4.0
- [ ] `contrasted_color` picks white adjustment for dark backgrounds, black adjustment for light backgrounds
- [ ] `fg == bg` (HIDDEN cell) returns fg unchanged regardless of `min_contrast` value
- [ ] Config `effective_minimum_contrast()` clamps NaN to 1.0, values outside [1.0, 21.0] to nearest bound (partially tested in `oriterm/src/config/tests.rs`)
- [ ] Uniform buffer bytes 8--11 contain `min_contrast` after `write_uniforms()` call

---

## 24.4 HiDPI & Display Scaling

Render correctly on high-DPI displays and handle multi-monitor DPI transitions.

**File:** `oriterm/src/window/mod.rs` (per-window `ScaleFactor` -- exists), `oriterm/src/app/mod.rs` (`handle_dpi_change` -- exists), `oriterm/src/app/event_loop.rs` (`ScaleFactorChanged` handler -- exists), `oriterm/src/gpu/pipeline/mod.rs` (sRGB surface format -- exists)

- [x] Track `scale_factor: ScaleFactor` per-window on `TermWindow` (not `App`):
  - [x] Initial value from `window.scale_factor()`, updated via `update_scale_factor()`
- [x] Handle `ScaleFactorChanged` event:
  - [x] `ctx.window.update_scale_factor()` returns true if changed
  - [x] `handle_dpi_change()` re-rasterizes fonts at `config.font.size * (DEFAULT_DPI * scale)`
  - [x] Atlas clear + re-render via `renderer.set_font_size()`
  - [x] Updates hinting and subpixel mode for new scale factor
  - [x] Marks all grid lines dirty via `mux.mark_all_dirty()`
- [ ] Font size scaling (remaining):
  - [ ] Zoom operations (`increase_font_size`, `decrease_font_size`) account for scale factor
  - [ ] `reset_font_size()` resets to `config.font.size * scale_factor`
- [ ] Multi-monitor DPI handling (remaining):
  - [ ] Verify that winit fires `ScaleFactorChanged` when dragging between monitors with different DPI -- the existing handler should handle this. Do not add scale-factor polling during drag (violates event flow discipline). If winit does not fire the event, file a winit issue upstream
- [x] sRGB-correct rendering pipeline:
  - [x] GPU pipeline uses sRGB surface format for gamma-correct blending
  - [x] Luminance-based alpha correction option (`AlphaBlending::LinearCorrected`)
  - [x] Config: `colors.alpha_blending` (`"linear"` | `"linear_corrected"`, default: `linear_corrected`) -- exists in `ColorConfig`

**Tests:**
- [ ] `handle_dpi_change()` triggers font re-rasterization at the new scale (unit test)
- [ ] Grid dimensions (columns, rows) recalculated after DPI change
- [ ] Dragging window between monitors with different DPI transitions without visual artifacts

---

## 24.5 Vector Icon Pipeline (tiny_skia)

**COMPLEXITY WARNING:** This is the largest subsection. Implement in strict phases: (1) icon data structures + rasterization, (2) atlas integration + cache, (3) widget integration (one widget at a time), (4) cleanup of old `push_line()` icon code. Do not attempt all phases at once.

Replace jagged geometric-primitive icons with properly anti-aliased vector path rasterization. Currently, diagonal lines (close X, chevron) are decomposed into pixel-stepping rectangles in `gpu/draw_list_convert/mod.rs`, producing visible staircase artifacts. This section introduces `tiny_skia` to rasterize icon paths into bitmaps at the exact DPI, cached in the glyph atlas alongside font glyphs.

**File:** `oriterm_ui/src/icons/mod.rs` (new -- icon path definitions), `oriterm/src/gpu/icon_rasterizer/mod.rs` (new -- tiny_skia rasterization), `oriterm/src/gpu/icon_rasterizer/cache.rs` (new -- `IconCache`), `oriterm_ui/src/widgets/tab_bar/widget/draw.rs` (consume icon textures -- currently uses `push_line()`), `oriterm_ui/src/widgets/window_chrome/controls.rs` (consume icon textures -- currently uses `push_line()`)

**Reference:** WezTerm `wezterm-gui/src/customglyph.rs` (`Poly`/`PolyCommand` system with `to_skia()` bridge), Chromium `components/vector_icons/` (.icon format to Skia paths)

**Dependency:** `tiny-skia` crate -- already in `oriterm/Cargo.toml` (used by COLRv1 rasterization)

- [ ] Icon path data structures:
  - [ ] `PathCommand` enum: `MoveTo(f32, f32)`, `LineTo(f32, f32)`, `CubicTo(f32, f32, f32, f32, f32, f32)`, `Close`
  - [ ] `IconPath` struct: `commands: &'static [PathCommand]`, `style: IconStyle`
  - [ ] `IconStyle` enum: `Stroke(f32)` (line width), `Fill`
  - [ ] All coordinates normalized 0.0 to 1.0 (scaled to target pixel size at rasterization time)
- [ ] Static icon definitions (`&'static` slices, zero runtime allocation):
  - [ ] `ICON_CLOSE`: two diagonal lines forming × (tab close button)
  - [ ] `ICON_PLUS`: two perpendicular lines forming + (new tab button)
  - [ ] `ICON_CHEVRON_DOWN`: two lines forming ▾ (dropdown button)
  - [ ] `ICON_MINIMIZE`: single horizontal line ─ (window control)
  - [ ] `ICON_MAXIMIZE`: square outline □ (window control)
  - [ ] `ICON_RESTORE`: two overlapping rectangles ⧉ (window control, maximized state)
  - [ ] `ICON_WINDOW_CLOSE`: two diagonal lines × (window close, slightly different proportions than tab close)
- [ ] tiny_skia rasterization:
  - [ ] `rasterize_icon(icon: &IconPath, size_px: u32, color: Color) -> Vec<u8>` (RGBA8 bitmap)
  - [ ] Build `tiny_skia::Path` from `PathCommand` sequence (scale 0.0--1.0 coords to `size_px`)
  - [ ] For `Stroke`: use `tiny_skia::Stroke` with round caps and round joins
  - [ ] For `Fill`: use `tiny_skia::FillRule::Winding`
  - [ ] Anti-aliasing enabled (tiny_skia default -- analytic AA, no MSAA needed)
  - [ ] Return RGBA8 pixmap data ready for atlas upload
- [ ] Atlas integration:
  - [ ] Icon bitmaps cached in the existing glyph atlas (grayscale pages, same as text)
  - [ ] Cache key: `(IconId, size_px, color_hash)` -- re-rasterize on DPI change or theme change
  - [ ] `IconCache` struct: `HashMap<(IconId, u32), AtlasRegion>` -- maps icon+size to atlas UV coords
  - [ ] On DPI scale change: invalidate icon cache, re-rasterize at new physical size
  - [ ] On theme color change: invalidate icon cache (icon color changed)
- [ ] Widget integration -- tab bar:
  - [ ] `draw.rs`: replace `push_line()` calls for close ×, + button, and chevron with `push_image()` referencing cached atlas region
  - [ ] Icon size derived from tab bar constants (e.g., `CLOSE_BUTTON_WIDTH * scale_factor`)
  - [ ] Hover color change: cache both normal and hover-color variants, or tint in shader
- [ ] Widget integration -- window chrome:
  - [ ] `controls.rs`: replace `push_line()` calls for minimize, maximize, restore, close with `push_image()` referencing cached atlas regions
  - [ ] Maximize/restore: swap icon based on `is_maximized` state
  - [ ] Close button hover: white icon on red background (cache white variant)
- [ ] Draw list integration:
  - [ ] Add `push_icon(rect, atlas_page, uv)` convenience method to `DrawList` -- emits a `DrawCommand::Image` with the icon's atlas region (no new enum variant needed; `DrawCommand::Image` already has `rect`, `texture_id`, `uv`)
  - [ ] In `oriterm/src/gpu/draw_list_convert/mod.rs`: implement the `DrawCommand::Image` conversion (currently logged as "deferred no-op") -- emit a textured quad using the atlas bind group
  - [ ] Make `IconCache` accessible during draw list conversion: either pass through conversion context, store on `WindowRenderer`, or provide a pre-resolved icon atlas lookup via `DrawCtx` so widgets resolve `(IconId, size_px)` to `(atlas_page, uv)` before emitting commands
- [ ] Module registration:
  - [ ] Create `oriterm_ui/src/icons/mod.rs` -- icon path definitions (`PathCommand`, `IconPath`, `IconStyle`, static icon data). Add `#[cfg(test)] mod tests;` at bottom
  - [ ] Create `oriterm_ui/src/icons/tests.rs` -- tests for icon path data (e.g., each icon has at least one `MoveTo` and one `Close`)
  - [ ] Add `pub mod icons;` to `oriterm_ui/src/lib.rs`
  - [ ] Create `oriterm/src/gpu/icon_rasterizer/mod.rs` -- `rasterize_icon()` function + re-exports. Add `#[cfg(test)] mod tests;` at bottom
  - [ ] Create `oriterm/src/gpu/icon_rasterizer/cache.rs` -- `IconCache` struct (HashMap-based atlas region cache, invalidation on DPI/theme change). Keep under 500 lines
  - [ ] Create `oriterm/src/gpu/icon_rasterizer/tests.rs` -- tests for rasterization + cache
  - [ ] Add `mod icon_rasterizer;` to `oriterm/src/gpu/mod.rs`
  - [ ] Wire `IconCache` into `WindowRenderer` -- initialized on construction, invalidated on DPI/theme change
- [ ] Remove old icon line-stepping code:
  - [ ] Remove icon-specific `push_line()` calls from `draw.rs` and `controls.rs`
  - [ ] Keep the general `push_line()` / `convert_line()` infrastructure (still used for menu checkmarks, separators, etc.)

**Tests:**
- [ ] Rasterize close icon at 16px, 24px, 32px -- output is non-empty RGBA8 with correct dimensions (`size_px * size_px * 4` bytes)
- [ ] Rasterize at different sizes produces different pixel data (not a byte-for-byte duplicate)
- [ ] Icon cache returns same `AtlasRegion` for same `(IconId, size_px)` key (cache hit)
- [ ] DPI change invalidates cache; subsequent lookup re-rasterizes and returns new region
- [ ] Rasterized close icon at 2.0x scale has non-zero alpha along the diagonal (no staircase gap)

---

## 24.6 Background Images

**COMPLEXITY WARNING:** New GPU pipeline + texture management. The `record_draw_passes()` function already has a `#[expect(clippy::too_many_lines)]` annotation -- adding another pipeline makes it longer. Consider extracting per-tier draw helpers if any logical block exceeds 50 lines.

Display a background image behind the terminal grid.

**File:** `oriterm/src/gpu/window_renderer/render.rs` (render pass), `oriterm/src/gpu/shaders/bg_image.wgsl` (new shader), `oriterm/src/config/mod.rs` (config -- `WindowConfig`), `oriterm/src/gpu/bg_image/mod.rs` (new -- texture loading + GPU upload), `oriterm/src/gpu/bg_image/tests.rs` (new -- position mode UV computation tests)

- [ ] Config options:
  ```toml
  [window]
  background_image = "/path/to/image.png"
  background_image_opacity = 0.1
  background_image_position = "center"  # center | stretch | tile | fill
  ```
- [ ] Image loading:
  - [ ] Load at startup and on config reload (hot-reload)
  - [ ] Decode PNG/JPEG/BMP via `image` crate. Currently `build-dependencies` and `dev-dependencies` only in `oriterm/Cargo.toml` -- must add `image` to `[dependencies]` with features (`png`, `jpeg`, `bmp`). The `build-dependencies` entry (used for icon embedding at build time) is separate and stays as-is
  - [ ] Convert to RGBA8 texture for wgpu
  - [ ] Handle errors gracefully (missing file, corrupt image, unsupported format) -- log warning, continue without background image
  - [ ] Validate image dimensions: reject images larger than GPU `max_texture_dimension_2d` (typically 8192 or 16384); log error and skip
- [ ] GPU rendering:
  - [ ] Create a wgpu texture from the decoded image (RGBA8Unorm format, `TEXTURE_BINDING` usage)
  - [ ] Create a bind group for the background image texture + sampler (reuse atlas sampler or create dedicated one)
  - [ ] Add a new render pass **before** cell backgrounds in `record_draw_passes()` in `oriterm/src/gpu/window_renderer/render.rs`:
    - [ ] Insert between `LoadOp::Clear` and the terminal tier backgrounds draw call
    - [ ] Full-screen quad with image texture
    - [ ] Apply `background_image_opacity` as alpha multiplier in the shader
  - [ ] New shader: `oriterm/src/gpu/shaders/bg_image.wgsl` -- samples texture, applies opacity uniform, outputs premultiplied alpha
  - [ ] New pipeline: add `bg_image_pipeline` to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Cell backgrounds blend over the image (existing bg pipeline uses `src*1 + dst*(1-srcA)` blend -- already works)
  - [ ] Position/scale image according to `background_image_position` -- compute UV coordinates on CPU, pass as instance data or uniform
- [ ] Position modes:
  - [ ] `center`: original size, centered, crop if larger than window
  - [ ] `stretch`: scale to fill window, may distort aspect ratio
  - [ ] `fill`: scale to fill, maintaining aspect ratio, crop excess
  - [ ] `tile`: repeat at original size
- [ ] Handle window resize: recompute UV coordinates on resize (no re-decode needed)
- [ ] Memory: keep decoded texture in GPU memory only. Drop the `image::DynamicImage` after GPU upload
- [ ] Config placement: `background_image` fields go on `oriterm/src/config/mod.rs::WindowConfig` (app-layer config). The `oriterm_ui::window::WindowConfig` (window creation struct) does NOT get these fields -- background rendering is a GPU concern
- [ ] Module registration:
  - [ ] Create `oriterm/src/gpu/bg_image/mod.rs` -- image loading, GPU texture creation, position mode UV computation. Add `#[cfg(test)] mod tests;` at bottom
  - [ ] Create `oriterm/src/gpu/bg_image/tests.rs` -- UV coordinate computation tests for each position mode
  - [ ] Add `pub(crate) mod bg_image;` to `oriterm/src/gpu/mod.rs`
  - [ ] Add `bg_image_pipeline` to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Add `image` to `[dependencies]` in `oriterm/Cargo.toml` (with features `png`, `jpeg`)

**Tests:**
- [ ] Image loads from valid path, returns error for missing path
- [ ] Position mode center computes correct UV coordinates
- [ ] Position mode fill maintains aspect ratio
- [ ] Opacity multiplier applied correctly in shader
- [ ] Config reload swaps background image without restart

---

## 24.7 Background Gradients

GPU-rendered gradient backgrounds as an alternative to solid colors or images.

**File:** `oriterm/src/gpu/window_renderer/render.rs` (render pass), `oriterm/src/gpu/shaders/bg_gradient.wgsl` (new shader), `oriterm_ui/src/draw/gradient.rs` (gradient data structures -- exists), `oriterm/src/gpu/pipelines.rs` (add `bg_gradient_pipeline`)

**Reference:** WezTerm `background` config (gradient presets + custom)

- [ ] Config:
  ```toml
  [window]
  background_gradient = "none"  # "none", "linear", "radial"
  gradient_colors = ["#1e1e2e", "#313244"]  # start and end colors
  gradient_angle = 180  # degrees, for linear gradient (CSS convention: 0 = bottom-to-top, 180 = top-to-bottom)
  gradient_opacity = 1.0  # 0.0-1.0, blended with background color
  ```
- [ ] Linear gradient:
  - [ ] Two-stop gradient from color A to color B
  - [ ] Angle configurable: 0 deg = bottom-to-top, 90 deg = left-to-right, 180 deg = top-to-bottom (CSS convention, matching existing `oriterm_ui::draw::gradient::Gradient::angle` field)
  - [ ] WGSL shader: interpolate colors based on UV coordinates rotated by angle
- [ ] Radial gradient:
  - [ ] Center-to-edge gradient
  - [ ] Color A at center, color B at edges
  - [ ] WGSL shader: distance from center to lerp between colors
- [ ] Multi-stop gradients (stretch goal):
  - [ ] `gradient_colors = ["#1e1e2e", "#313244", "#45475a"]` -- 3+ stops
  - [ ] Even distribution across gradient length
- [ ] Rendering:
  - [ ] New shader: `oriterm/src/gpu/shaders/bg_gradient.wgsl` -- takes gradient parameters as uniforms (2 colors, angle), outputs interpolated color per fragment
  - [ ] New pipeline: add `bg_gradient_pipeline` to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Full-screen quad before cell backgrounds in `record_draw_passes()` (same insertion point as background image from 24.6)
  - [ ] Draw order in `record_draw_passes()`: clear, then gradient, then background image, then terminal cell backgrounds, then text, then cursors, then chrome, then overlays
  - [ ] If both gradient and image configured: gradient renders first, image blends on top with alpha
  - [ ] Cell backgrounds blend on top of gradient (existing blend mode handles this)
  - [ ] Gradient parameters passed via a dedicated uniform buffer or packed into the existing uniform buffer (evaluate 16-byte alignment cost vs. dedicated bind group)
- [ ] Interaction with transparency:
  - [ ] Gradient respects `window.opacity` -- blended with compositor-provided background
  - [ ] `gradient_opacity` controls gradient's own alpha (independent of window opacity)
- [ ] Hot-reload: gradient config changes apply immediately
- [ ] **Tests:**
  - [ ] Linear gradient: pixel at top differs from pixel at bottom
  - [ ] Angle rotation: 90 deg gradient varies horizontally, not vertically
  - [ ] Gradient opacity: alpha applied correctly
  - [ ] Config "none": no gradient rendered

---

## 24.8 Window Backdrop Effects

Platform-specific compositor effects: Acrylic/Mica on Windows, blur on macOS/Linux. **Partially implemented**: basic acrylic/vibrancy/blur already works via `transparency.rs` using the `window-vibrancy` crate. `WindowConfig` already has `opacity: f32` and `blur: bool`. This subsection adds fine-grained backdrop type selection beyond the existing boolean toggle.

**File:** `oriterm/src/gpu/transparency.rs` (existing -- already implements acrylic/vibrancy/blur), `oriterm/src/app/init/mod.rs` (window creation), `oriterm/src/config/mod.rs` (`WindowConfig`)

**Reference:** WezTerm `win32_system_backdrop`, Ghostty `background-blur-radius`, `window-vibrancy` crate (already a dependency)

- [ ] Config (extend existing `WindowConfig`):
  ```toml
  [window]
  backdrop = "none"  # "none", "blur", "acrylic", "mica", "auto"
  # Existing fields: opacity = 1.0, blur = true
  ```
- [x] Windows backdrop effects (Win32) -- partially implemented:
  - [x] `acrylic` -- `window_vibrancy::apply_acrylic()` with tint color (exists in `transparency.rs`)
  - [ ] `mica` -- `DWM_SYSTEMBACKDROP_TYPE::DWMSBT_MAINWINDOW` (Windows 11 only)
  - [ ] `auto` -- Mica on Windows 11, Acrylic on Windows 10
  - [x] Requires `window.opacity < 1.0` to see the effect (guarded in `apply_transparency()`)
  - [x] Uses `window-vibrancy` crate (already a dependency)
- [x] macOS backdrop effects -- partially implemented:
  - [x] `blur` -- `window_vibrancy::apply_vibrancy()` with `UnderWindowBackground` material (exists)
  - [ ] Material selection config (`.hudWindow` or `.sidebar`)
- [x] Linux backdrop effects -- implemented:
  - [x] `blur` -- `window.set_blur(true)` via winit (exists in `transparency.rs`)
  - [ ] Log a warning when compositor does not support blur (best-effort detection)
- [ ] Config mapping: the `backdrop` enum replaces the boolean `blur` field in `oriterm/src/config/mod.rs::WindowConfig`:
  - [ ] `backdrop = "none"` -- `apply_transparency(_, opacity, false, _)`
  - [ ] `backdrop = "blur"` -- `apply_transparency(_, opacity, true, _)` (current behavior)
  - [ ] `backdrop = "acrylic"` -- new code path: call `window_vibrancy::apply_acrylic()` directly
  - [ ] `backdrop = "mica"` -- new code path: Windows 11 DWM API
  - [ ] `backdrop = "auto"` -- platform detection logic
  - [ ] Deprecate `blur: bool` config field (keep for backwards compatibility; `backdrop` takes priority when both present)
- [ ] Interaction with other features:
  - [ ] Backdrop visible only when `window.opacity < 1.0`
  - [ ] Background gradient renders on top of backdrop effect
  - [ ] Background image renders on top of backdrop effect
  - [ ] Cell backgrounds render on top of all of the above
- [ ] Error handling (graceful fallback):
  - [ ] Mica on Windows 10: log warning, fall back to Acrylic
  - [ ] Acrylic on Windows without DWM composition: log warning, fall back to `none`
  - [ ] Linux without compositor: `window.set_blur(true)` may silently fail -- detect and log
- [ ] **Tests:**
  - [ ] Config parsing: all backdrop variants deserialize correctly
  - [ ] "none" disables backdrop
  - [ ] "auto" selects platform-appropriate effect
  - [ ] Backwards compatibility: `blur = true` without `backdrop` field still enables blur
  - [ ] `backdrop` field overrides `blur` field when both are present

---

## 24.9 Scrollable Menus

Add max-height constraint and scroll support to `MenuWidget` so long menus (e.g., 50+ theme entries) do not overflow the window.

**COMPLEXITY WARNING:** `menu/mod.rs` is already 499 lines. Adding scroll support directly will exceed the 500-line limit. Extract drawing into a submodule first, then add scroll logic.

**File:** `oriterm_ui/src/widgets/menu/mod.rs` (`MenuWidget` + `MenuStyle` -- exist, 499 lines), `oriterm_ui/src/widgets/menu/draw.rs` (new -- extracted drawing logic), `oriterm_ui/src/widgets/menu/scroll.rs` (new -- scroll state + scrollbar drawing), `oriterm_ui/src/widgets/scroll/mod.rs` (existing scroll widget for reference)

- [ ] **Prerequisite: split `menu/mod.rs`** to make room for scroll code:
  - [ ] Extract drawing logic into `oriterm_ui/src/widgets/menu/draw.rs` (the `draw()` method body and helper functions like separator drawing, check-mark drawing)
  - [ ] Keep widget struct, entry types, style, layout, and event handling in `mod.rs`
  - [ ] Verify `mod.rs` is well under 500 lines after split
- [ ] `max_height: Option<f32>` on `MenuStyle` (default: None = unlimited)
- [ ] `scroll_offset: f32` field on `MenuWidget` -- vertical scroll position (0.0 = top)
- [ ] When content height exceeds `max_height`: clip entries and show vertical scrollbar
  - [ ] In `layout()`: return `LayoutBox` with height capped to `max_height` when set and content exceeds it
  - [ ] In `draw()`: emit `DrawCommand::PushClip(bounds)` before entry rendering and `DrawCommand::PopClip` after (integrates with existing clipping in `oriterm/src/gpu/draw_list_convert/clip.rs`)
  - [ ] Offset all entry Y positions by `-scroll_offset` during draw
- [ ] Scrollbar: thin track on right edge, thumb sized proportionally to visible/total ratio
  - [ ] Style values from `ScrollbarStyle` (reuse from `oriterm_ui/src/widgets/scroll/mod.rs`) or add `scrollbar_width`, `scrollbar_color` to `MenuStyle`
  - [ ] Only visible when `total_height() > max_height`
- [ ] Mouse wheel scrolls menu content:
  - [ ] Handle `MouseEventKind::Scroll(delta)` in `handle_mouse()` (currently only `Move`, `Down`, `Up` are handled)
  - [ ] Clamp `scroll_offset` to `[0.0, total_height - max_height]`
- [ ] Keyboard navigation auto-scrolls to keep hovered item visible:
  - [ ] After `navigate()`: compute the Y position of the hovered entry and adjust `scroll_offset` to keep it within the visible window
  - [ ] `PageUp`/`PageDown` keys: scroll by visible height
  - [ ] `Home`/`End` keys: jump to first/last clickable entry
- [ ] Scroll position resets to top when menu opens
  - [ ] Reset `scroll_offset = 0.0` in `MenuWidget::new()` and when entries are replaced

**Tests:**
- [ ] Menu with 5 entries and no `max_height`: no clipping, no scrollbar, full height
- [ ] Menu with 50 entries and `max_height = 300.0`: layout height is 300.0, not `50 * item_height`
- [ ] Mouse wheel scroll adjusts `scroll_offset`, clamped to valid range
- [ ] `scroll_offset` cannot go negative or past `total_height - max_height`
- [ ] Keyboard navigate to entry beyond visible area auto-scrolls to reveal it
- [ ] `Home` key scrolls to top and hovers first clickable entry
- [ ] `End` key scrolls to bottom and hovers last clickable entry
- [ ] Scroll position resets on menu rebuild (new entries)

---

## 24.10 Section Completion

- [ ] All 24.1--24.9 items complete
- [ ] Cursor blinks at configured rate for blinking DECSCUSR styles
- [ ] Cursor blink resets on keypress, mouse click, and PTY cursor movement
- [ ] Unfocused windows show steady hollow cursor (no blink)
- [ ] Mouse cursor hides when typing, reappears on move
- [ ] Mouse hiding respects mouse reporting mode (does not hide when app uses mouse)
- [ ] Minimum contrast enforces readable text (WCAG 2.0 in shader)
- [ ] `minimum_contrast = 1.0` (default) adds zero per-vertex overhead (shader short-circuits)
- [ ] HiDPI displays render crisp text at correct scale
- [ ] Moving between monitors with different DPI works
- [ ] Vector icons (close ×, +, chevron, minimize, maximize, restore, window close) render with smooth anti-aliasing at all DPI scales
- [ ] No jagged staircase artifacts on diagonal lines in any icon
- [ ] Background images render behind terminal content
- [ ] Background gradients render behind terminal content
- [ ] Backdrop effects (blur/acrylic/mica) apply on supported platforms
- [ ] Scrollable menus handle 50+ entries without window overflow
- [ ] All features configurable and hot-reloadable
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` pass
- [ ] No new `#[allow(clippy)]` without `reason`

**Exit Criteria:** Terminal feels visually polished at first launch -- cursor blinks, text is readable, HiDPI is crisp, icons are smooth, scrolling works, and all features are configurable and hot-reloadable.
