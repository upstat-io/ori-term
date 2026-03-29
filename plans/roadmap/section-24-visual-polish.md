---
section: 24
title: Visual Polish
status: in-progress
reviewed: false
last_verified: "2026-03-29"
tier: 6
goal: Cursor blinking, hide-while-typing, minimum contrast, HiDPI, vector icons, background images, gradients, backdrop effects, scrollable menus
sections:
  - id: "24.1"
    title: Cursor Blinking
    status: complete
  - id: "24.2"
    title: Hide Cursor While Typing
    status: complete
  - id: "24.3"
    title: Minimum Contrast
    status: not-started
  - id: "24.4"
    title: HiDPI & Display Scaling
    status: in-progress
  - id: "24.5"
    title: Vector Icon Pipeline (tiny_skia)
    status: complete
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
    status: in-progress
  - id: "24.10"
    title: Section Completion
    status: not-started
---

# Section 24: Visual Polish

**Status:** In Progress (24.1, 24.2, 24.5 complete; 24.4 HiDPI, 24.8 backdrop, 24.9 scrollable menus partially implemented)
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

**Recommended implementation order:** 24.1 -> 24.2 -> 24.9 -> 24.3 -> 24.4 -> 24.5 -> 24.6 -> 24.7 -> 24.8

---

## 24.1 Cursor Blinking

Toggle cursor visibility on a timer. **Complete**: `CursorBlink` state machine exists (`oriterm/src/app/cursor_blink/mod.rs`) with `is_visible()`, `reset()`, and `next_toggle()`. The `about_to_wait` handler drives blink via `ControlFlow::WaitUntil`. `cursor_blink_visible` is threaded through the prepare pipeline. Focus handling (unfocused hollow cursor), mouse click reset, and PTY cursor-move reset are all implemented.

**File:** `oriterm/src/app/cursor_blink/mod.rs` (blink state -- exists), `oriterm/src/app/event_loop.rs` (`about_to_wait` blink timer -- exists), `oriterm/src/app/redraw/mod.rs` (cursor blink visible flag -- exists), `oriterm/src/gpu/prepare/mod.rs` (cursor emission gating -- exists)

- [x] Blink state tracking: (verified 2026-03-29, 13 tests pass)
  - [x] `CursorBlink` struct with `last_visible`, `epoch`, `interval` fields on `App`
  - [x] Blink interval: 530ms on / 530ms off (configurable via `cursor_blink_interval_ms`)
  - [x] `update()` checks `is_visible()` (pure function of elapsed time since epoch), caches the result, and returns `true` if visibility changed
- [x] DECSCUSR blinking style detection: (verified 2026-03-29)
  - [x] DECSCUSR values 1 (blinking block), 3 (blinking underline), 5 (blinking bar) enable blink
  - [x] Even values (2, 4, 6) = steady -- no blink
  - [x] Default (0) = implementation-defined -- follow config
  - [x] `TermMode::CURSOR_BLINKING` flag set/cleared by DECSCUSR handler; `blinking_active` cached on `App`
- [x] Reset blink to visible on keypress:
  - [x] `cursor_blink.reset()` called from keyboard input handler (resets `epoch` to `Instant::now()`, so the phase-0 visible window restarts)
- [x] Reset blink to visible on PTY cursor movement:
  - [x] Compare `last_cursor_pos` between frames in `handle_redraw()` and `handle_redraw_multi_pane()`; reset blink only on actual position change (not on every PTY byte)
- [x] Reset blink to visible on mouse click in grid:
  - [x] `cursor_blink.reset()` called from `handle_mouse_input()` on any grid press (after overlays/chrome/dividers return early)
- [x] Timer implementation using winit: (verified 2026-03-29)
  - [x] `ControlFlow::WaitUntil(cursor_blink.next_toggle())` in `about_to_wait` when `blinking_active`
  - [x] `about_to_wait` calls `cursor_blink.update()`, marks dirty, sets next deadline
  - [x] When no blink needed: falls through to default `ControlFlow::Wait`
- [x] Renderer integration: (verified 2026-03-29)
  - [x] `cursor_blink_visible` passed to `WindowRenderer::prepare()` and `prepare_frame_shaped_into()`
  - [x] `prepare/mod.rs` skips cursor emission when `!cursor_blink_visible`
- [x] Focus handling: (verified 2026-03-29)
  - [x] Window loses focus (`WindowEvent::Focused(false)` in `event_loop.rs`): set `blinking_active = false` and call `cursor_blink.reset()` to freeze cursor visible
  - [x] Window gains focus (`WindowEvent::Focused(true)` in `event_loop.rs`): re-evaluate `blinking_active` from pane's `TermMode::CURSOR_BLINKING` and call `cursor_blink.reset()`
  - [x] Unfocused window renders cursor as hollow block (outline only). `focused_window_id` on `App` tracks which window has OS focus; `window_focused: bool` on `FrameInput` is set from `ctx.window.window().has_focus()` and propagated to the shaped prepare path
- [x] Config: `terminal.cursor_blink` (default: true) -- exists as `TerminalConfig::cursor_blink`
- [x] Config: `terminal.cursor_blink_interval_ms` (default: 530) -- exists as `TerminalConfig::cursor_blink_interval_ms`

**Tests:**
- [x] Blink state reports change after interval elapses (`update_after_interval_reports_change`)
- [x] Keypress resets blink to visible (`reset_makes_visible`)
- [x] Even DECSCUSR values disable blinking (`decscusr_fires_cursor_blinking_change_event`)
- [x] Odd DECSCUSR values enable blinking (oriterm_core handler tests)
- [x] Focus loss sets `blinking_active = false` and freezes cursor visible (verified in `event_loop.rs` Focused handler; no dedicated unit test -- integration behavior)
- [x] Mouse click in grid resets blink to visible (verified in `mouse_input.rs` grid click path; no dedicated unit test -- integration behavior)
- [x] Unfocused window renders hollow block cursor (`unfocused_window_renders_hollow_cursor`, `unfocused_window_bar_cursor_becomes_hollow`, `focused_window_renders_block_cursor`)

---

## 24.2 Hide Cursor While Typing

Mouse cursor disappears when typing, reappears on mouse move.

**File:** `oriterm/src/app/cursor_hide/mod.rs` (pure decision logic), `oriterm/src/app/mod.rs` (state + `restore_mouse_cursor`), `oriterm/src/app/keyboard_input/mod.rs` (keypress hiding), `oriterm/src/app/event_loop.rs` (`CursorMoved`/`CursorLeft`/`Focused` restore), `oriterm/src/config/behavior.rs` (config)

- [x] Hide mouse cursor on keypress: (verified 2026-03-29, 7 tests pass)
  - [x] Track `mouse_cursor_hidden: bool` on `App`
  - [x] Pure decision function `should_hide_cursor(HideContext)` in `app/cursor_hide/mod.rs`
  - [x] Called from `encode_key_to_pty()` — hides via `window.set_cursor_visible(false)`
  - [x] Skip modifier-only keys (`NamedKey::Shift`, `NamedKey::Control`, `NamedKey::Alt`, `NamedKey::Super`, `Hyper`, `Meta`)
  - [x] Skip when IME composition active (`ime.should_suppress_key()`)
- [x] Restore mouse cursor on mouse move: (verified 2026-03-29)
  - [x] `restore_mouse_cursor()` helper on `App` — only calls `set_cursor_visible(true)` when `mouse_cursor_hidden` is true
  - [x] Called on `CursorMoved` event in `event_loop.rs`
  - [x] Called on `CursorLeft` event to avoid sticky hidden state
- [x] Suppress hiding during mouse reporting mode: (verified 2026-03-29)
  - [x] Check `TermMode::ANY_MOUSE` inline at the call site (already have `mode` from `pane_mode()`)
  - [x] Passed as `mouse_reporting` field in `HideContext`
- [x] Restore cursor on window focus loss: (verified 2026-03-29)
  - [x] `restore_mouse_cursor()` called on `WindowEvent::Focused(false)`
- [x] Config: `hide_mouse_when_typing: bool` (default: true) on `BehaviorConfig`
  - [x] All hide/show logic gated on `self.config.behavior.hide_mouse_when_typing`
  - [x] Added to `Default for BehaviorConfig`

**Tests:** Pure function `should_hide_cursor()` tested in `app/cursor_hide/tests.rs`.
- [x] Keypress with `hide_mouse_when_typing = true` returns `should_hide = true`
- [x] Already-hidden cursor skips redundant hide
- [x] `ANY_MOUSE` mode active prevents hiding even when config is true
- [x] `hide_mouse_when_typing = false` disables the feature entirely
- [x] Modifier-only keypress (Shift, Ctrl, Alt, Super) does not trigger hiding
- [x] IME composition does not hide cursor
- [x] Named action keys (Enter, Space, Backspace) trigger hiding

---

## 24.3 Minimum Contrast

**COMPLEXITY WARNING:** This subsection modifies the shared uniform buffer and 7 WGSL shader files in lockstep, and introduces WCAG math in WGSL. Build the Rust reference implementation and validate it thoroughly before touching any shaders. Test edge cases (HIDDEN cells, reverse video, bold-bright) in Rust first.

Ensure text is always readable regardless of color scheme. WCAG 2.0 contrast enforcement in the GPU shader.

**File:** `oriterm/src/gpu/contrast/mod.rs` (new -- Rust reference impl for WCAG luminance/contrast), `oriterm/src/gpu/contrast/tests.rs` (new -- unit tests), `oriterm/src/gpu/shaders/fg.wgsl` + `oriterm/src/gpu/shaders/subpixel_fg.wgsl` (WGSL shaders -- contrast enforcement), `oriterm/src/config/color_config.rs` (config -- field already exists), `oriterm/src/gpu/bind_groups/mod.rs` (uniform buffer write)

**FILE SIZE WARNING:** `oriterm/src/gpu/pipeline/mod.rs` is at 500 lines (the hard limit). The 24.3 uniform buffer change (repurposing `_pad.x`) does not change the buffer layout or require pipeline modifications -- it only changes the data written via `write_uniforms()`. However, sections 24.6 and 24.7 add NEW pipelines whose creation functions must go in new submodules (e.g., `pipeline/bg_image.rs`, `pipeline/bg_gradient.rs`), not in `pipeline/mod.rs`. Extract existing pipeline code into submodules (e.g., `pipeline/fg.rs`, `pipeline/bg.rs`) before 24.6 if `mod.rs` needs modifications beyond adding `mod` declarations.

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
  - [ ] Rename `UniformBuffer::write_screen_size()` to `write_uniforms()`, add `min_contrast: f32` parameter. Write `min_contrast` at bytes 8--11 (the current zero-padded `_pad.x` slot). Update all callers:
    - `oriterm/src/gpu/window_renderer/render.rs` line 27 (`render_frame()`)
    - `oriterm/src/gpu/bind_groups/tests.rs` lines 21-45 (test functions `write_screen_size_does_not_panic`, `write_screen_size_zero_dimensions`)
    - Update doc comments on `UniformBuffer` struct (line 23 of `bind_groups/mod.rs`)
  - [ ] Rename `_pad: vec2<f32>` to `extra: vec2<f32>` in 7 shaders that use the `Uniform` struct: `fg.wgsl`, `bg.wgsl`, `subpixel_fg.wgsl`, `color_fg.wgsl`, `ui_rect.wgsl`, `image.wgsl`, and `composite.wgsl` (which names its struct `ScreenUniform` but shares the same memory layout at group 0). (`colr_solid.wgsl` and `colr_gradient.wgsl` use separate uniform structs and are not affected.) Buffer size stays 16 bytes
- [ ] **Step 3: WGSL shader port** -- port the validated Rust functions into `fg.wgsl` and `subpixel_fg.wgsl`:
  - [ ] Add `luminance()`, `contrast_ratio()`, `contrasted_color()` as WGSL functions
  - [ ] **sRGB-to-linear conversion**: the `luminance()` function in WGSL expects linear RGB. Colors in the instance buffer are already in linear space (the prepare phase converts via `srgb_to_linear()` in `gpu/mod.rs`). The `*Srgb` surface format handles the final linear-to-sRGB conversion on output. No additional sRGB conversion is needed in the shader
  - [ ] Apply contrast adjustment in `vs_main` (vertex shader) for both `fg.wgsl` and `subpixel_fg.wgsl`. In `fg.wgsl`, `bg_color` is a per-instance attribute in `vs_main` but is NOT passed to the fragment stage, so contrast must be applied per-vertex. In `subpixel_fg.wgsl`, `bg_color` IS passed through to `fs_main` (for per-channel compositing), but vertex stage is preferred for consistency:
    ```wgsl
    out.fg_color = contrasted_color(uniforms.extra.x, input.fg_color, input.bg_color);
    ```
  - [ ] Only apply in `fg.wgsl` and `subpixel_fg.wgsl` (text shaders). `bg.wgsl`, `ui_rect.wgsl`, `image.wgsl` do not render text. `color_fg.wgsl` (emoji/color glyphs): skip contrast -- adjusting bitmap colors would distort them
  - [ ] **UI text side effect**: UI text rendered via `draw_list_convert/mod.rs` uses the same `fg.wgsl` and `subpixel_fg.wgsl` shaders. The `min_contrast` uniform will apply to UI text (tab labels, menu items) as well as terminal cells. This is acceptable (Ghostty does the same) but should be documented as intentional behavior
- [ ] Hot-reload: `minimum_contrast` value read from `config.colors.effective_minimum_contrast()` each frame, so config changes apply immediately
- [ ] Threading `min_contrast` to `write_uniforms()`: add `min_contrast: f32` field to `PreparedFrame` in `oriterm/src/gpu/prepared_frame/mod.rs`, set during `prepare()` from the config. This keeps `render_frame()` pure (reads from `self.prepared`, no config access during render)

**Edge cases:**
- [ ] `minimum_contrast = 1.0` (default, disabled): shader short-circuits -- no luminance computation, pass fg through unchanged. Cost: one branch per vertex, zero overhead when disabled
- [ ] HIDDEN cells (SGR 8): do not reveal. SGR 8 sets `CellFlags::HIDDEN` on the cell but does NOT set `fg = bg`. The fg and bg colors remain independently resolved by `resolve_fg()`/`resolve_bg()` in `oriterm_core/src/term/renderable/mod.rs` and `apply_inverse()`. The shader cannot rely on `fg == bg` to detect HIDDEN cells. **Solution**: the prepare phase (`resolve_cell_colors()` in `oriterm/src/gpu/prepare/mod.rs`) must detect `CellFlags::HIDDEN` and explicitly set `fg = bg` before writing to the instance buffer. This makes the `fg == bg` signal reliable for the shader's `contrasted_color()` to detect and skip. Do NOT use `fg_color.a = 0.0` as a signal (it would break premultiplied alpha blending for all text)
- [ ] **Prepare phase change for HIDDEN**: in `resolve_cell_colors()` (and the unshaped path in `unshaped.rs` which also calls `resolve_cell_colors()`), add an early return after the base color resolution: `if cell.flags.contains(CellFlags::HIDDEN) { return (bg, bg); }`. Place after selection handling (so selected HIDDEN cells remain hidden) to ensure the shader sees `fg == bg` for all HIDDEN cells
- [ ] Reverse video cells (SGR 7): contrast uses the already-swapped fg/bg (no special handling needed -- `apply_inverse()` runs in the renderable layer before colors reach the prepare phase)
- [ ] Bold/dim flags: contrast applied after bold-bright resolution (handled at the terminal renderable layer in `oriterm_core/src/term/renderable/mod.rs::resolve_fg()`) and dim adjustments. The shader sees final resolved colors; no special handling needed

**Tests:** (in `oriterm/src/gpu/contrast/tests.rs`)
- [ ] White on black at `minimum_contrast = 1.0` passes through unchanged
- [ ] Dark gray (#333) on black at `minimum_contrast = 4.5` adjusts fg to a lighter color
- [ ] Light gray (#ccc) on white at `minimum_contrast = 4.5` adjusts fg to a darker color
- [ ] ITU-R BT.709 luminance: verify correct relative luminance for pure red, green, blue, white, black
- [ ] `contrast_ratio(white, black)` approximately equals 21.0; `contrast_ratio(#777, #000)` approximately equals 4.0
- [ ] `contrasted_color` picks white adjustment for dark backgrounds, black adjustment for light backgrounds
- [ ] `fg == bg` (HIDDEN cell) returns fg unchanged regardless of `min_contrast` value
- [ ] Config `effective_minimum_contrast()` clamps NaN to 1.0, values outside [1.0, 21.0] to nearest bound (tested in `oriterm/src/config/tests.rs`: `minimum_contrast_nan_defaults_to_one`, `minimum_contrast_inf_clamped_to_twenty_one`, `minimum_contrast_clamped`)
- [ ] Uniform buffer bytes 8--11 contain `min_contrast` after `write_uniforms()` call

**Tests:** (in `oriterm/src/gpu/prepare/tests.rs` -- HIDDEN cell prepare phase)
- [ ] HIDDEN cell (SGR 8) with distinct fg/bg produces `fg == bg` in the instance buffer (prepare phase sets `fg = bg`)
- [ ] HIDDEN cell under selection still produces `fg == bg` (not revealed by selection)
- [ ] Non-HIDDEN cell with same fg/bg is NOT treated as hidden (contrast still applies)

---

## 24.4 HiDPI & Display Scaling

Render correctly on high-DPI displays and handle multi-monitor DPI transitions.

**File:** `oriterm/src/window/mod.rs` (per-window `ScaleFactor` -- exists), `oriterm/src/app/mod.rs` (`handle_dpi_change` -- exists), `oriterm/src/app/event_loop.rs` (`ScaleFactorChanged` handler -- exists), `oriterm/src/gpu/state/mod.rs` (sRGB surface format via `select_formats()` + `render_format` -- exists)

- [x] Track `scale_factor: ScaleFactor` per-window on `TermWindow` (not `App`):
  - [x] Initial value from `window.scale_factor()`, updated via `update_scale_factor()`
- [x] Handle `ScaleFactorChanged` event:
  - [x] `ctx.window.update_scale_factor()` returns true if changed
  - [x] `handle_dpi_change()` re-rasterizes fonts at `config.font.size * (DEFAULT_DPI * scale)`
  - [x] Atlas clear + re-render via `renderer.set_font_size()`
  - [x] Updates hinting and subpixel mode for new scale factor
  - [x] Marks all grid lines dirty via `mux.mark_all_dirty()`
- [ ] Font size zoom operations (not yet implemented):
  - [ ] Add `increase_font_size()`, `decrease_font_size()`, `reset_font_size()` methods to `App` or `WindowContext` that call `renderer.set_font_size()` with the adjusted size
  - [ ] Bind to keybindings: `Ctrl+=` (increase), `Ctrl+-` (decrease), `Ctrl+0` (reset). Register in keybinding dispatch table
  - [ ] Zoom operations must account for scale factor: the font size passed to the renderer is `logical_size * (DEFAULT_DPI * scale_factor)`, so zoom increments apply to the logical size and then re-multiply
  - [ ] `reset_font_size()` resets to `config.font.size * scale_factor`
  - [ ] After zoom: recalculate grid dimensions (columns/rows), notify mux of resize, mark all dirty
- [ ] Multi-monitor DPI transitions:
  - [ ] Confirm that winit fires `ScaleFactorChanged` when dragging between monitors with different DPI -- the existing handler should handle this correctly. If winit does not fire the event on a particular platform, file a winit issue upstream. Do not add scale-factor polling during drag
- [ ] **BUG:** First Aero Snap after launch shrinks text — if the user opens the app and Aero Snaps without manually resizing first, text renders smaller. A single manual resize before snapping prevents it. Suggests initial DPI/scale factor isn't fully applied until the first resize event forces recalculation. Discovered during chrome plan verification (2026-03-10).
- [ ] **BUG:** Settings dialog inherits parent window DPI — dragging a settings dialog to a monitor with different DPI keeps the parent's scale factor instead of responding to its own `ScaleFactorChanged` event. The dialog's `handle_dpi_change` path may not be wired up or `ScaleFactorChanged` may not fire for child windows. Discovered during chrome plan verification (2026-03-10).
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

**File:** `oriterm_ui/src/icons/mod.rs` (new -- icon path definitions), `oriterm/src/gpu/icon_rasterizer/mod.rs` (new -- tiny_skia rasterization), `oriterm/src/gpu/icon_rasterizer/cache.rs` (new -- `IconCache`), `oriterm_ui/src/widgets/tab_bar/widget/draw.rs` (consume icon textures -- currently uses `push_line()`), `oriterm_ui/src/widgets/window_chrome/controls.rs` (consume icon textures -- currently uses `push_line()` for minimize/close and `push_rect()` for maximize/restore)

**Reference:** WezTerm `wezterm-gui/src/customglyph.rs` (`Poly`/`PolyCommand` system with `to_skia()` bridge), Chromium `components/vector_icons/` (.icon format to Skia paths)

**Dependency:** `tiny-skia` crate -- already in `oriterm/Cargo.toml` (used by COLRv1 rasterization)

- [x] **Phase 1: Library crate types** (`oriterm_ui` -- must be implemented before binary crate code): (verified 2026-03-29, 9 icon tests pass)
  - [x] Create `oriterm_ui/src/icons/mod.rs` -- icon path definitions. Add `#[cfg(test)] mod tests;` at bottom
  - [x] Add `pub mod icons;` to `oriterm_ui/src/lib.rs`
  - [x] `PathCommand` enum: `MoveTo(f32, f32)`, `LineTo(f32, f32)`, `CubicTo(f32, f32, f32, f32, f32, f32)`, `Close`
  - [x] `IconPath` struct: `commands: &'static [PathCommand]`, `style: IconStyle`
  - [x] `IconStyle` enum: `Stroke(f32)` (line width), `Fill`
  - [x] `IconId` enum: `Close`, `Plus`, `ChevronDown`, `Minimize`, `Maximize`, `Restore`, `WindowClose` -- newtype for type-safe cache key (not a bare `u32`)
  - [x] All coordinates normalized 0.0 to 1.0 (scaled to target pixel size at rasterization time)
  - [x] Create `oriterm_ui/src/icons/tests.rs` -- validate each icon has at least one `MoveTo` and one `Close` command
- [x] **Phase 1b: Static icon definitions** (`&'static` slices, zero runtime allocation):
  - [x] `ICON_CLOSE`: two diagonal lines forming x (tab close button)
  - [x] `ICON_PLUS`: two perpendicular lines forming + (new tab button)
  - [x] `ICON_CHEVRON_DOWN`: two lines forming down-pointing chevron (dropdown button)
  - [x] `ICON_MINIMIZE`: single horizontal line (window control)
  - [x] `ICON_MAXIMIZE`: square outline (window control)
  - [x] `ICON_RESTORE`: two overlapping rectangles (window control, maximized state)
  - [x] `ICON_WINDOW_CLOSE`: two diagonal lines x (window close, slightly different proportions than tab close)
- [x] **Phase 2: Rasterization + cache** (`oriterm` binary crate -- depends on Phase 1): (verified 2026-03-29, 9 rasterizer tests pass)
  - [x] Create `oriterm/src/gpu/icon_rasterizer/mod.rs` -- `rasterize_icon()` function + re-exports. Add `#[cfg(test)] mod tests;` at bottom
  - [x] Create `oriterm/src/gpu/icon_rasterizer/cache.rs` -- `IconCache` struct (HashMap-based atlas region cache, invalidation on DPI change). Keep under 500 lines
  - [x] Create `oriterm/src/gpu/icon_rasterizer/tests.rs` -- tests for rasterization + cache
  - [x] Add `mod icon_rasterizer;` to `oriterm/src/gpu/mod.rs`
  - [x] `rasterize_icon(icon: &IconPath, size_px: u32) -> Vec<u8>` -- rasterize as alpha-only (single-channel). Render white-on-transparent in tiny_skia, extract the alpha channel as R8 data for the mono glyph atlas. Output size: `size_px * size_px` bytes
  - [x] Build `tiny_skia::Path` from `PathCommand` sequence (scale 0.0--1.0 coords to `size_px`)
  - [x] For `Stroke`: use `tiny_skia::Stroke` with round caps and round joins
  - [x] For `Fill`: use `tiny_skia::FillRule::Winding`
  - [x] Anti-aliasing enabled (tiny_skia default -- analytic AA, no MSAA needed)
- [x] **Phase 2b: Atlas integration**:
  - [x] Icon bitmaps cached in the existing glyph atlas as R8Unorm grayscale pages (same as mono text glyphs), rendered through the `fg.wgsl` pipeline. This allows tinting icons to any color at draw time (hover states, theme changes) without re-rasterizing
  - [x] Cache key: `(IconId, size_px)` -- re-rasterize on DPI change only (color is applied by shader, not baked in)
  - [x] `IconCache` struct: `HashMap<(IconId, u32), AtlasRegion>` -- maps icon+size to atlas UV coords
  - [x] On DPI scale change: invalidate icon cache, re-rasterize at new physical size
  - [x] Wire `IconCache` into `WindowRenderer` -- initialized on construction, invalidated on DPI change
- [x] **Phase 3: Draw list integration**: (verified 2026-03-29)
  - [x] Add a `DrawCommand::Icon { rect: Rect, atlas_page: u32, uv: [f32; 4] }` variant that routes through the existing glyph pipeline. Do NOT reuse `DrawCommand::Image` -- that variant implies per-image texture bind groups (routed through `record_image_draws()`), while icons live in the shared glyph atlas
  - [x] Add `push_icon(rect, atlas_page, uv)` convenience method to `DrawList` -- emits a `DrawCommand::Icon`
  - [x] In `oriterm/src/gpu/draw_list_convert/mod.rs`: handle `DrawCommand::Icon { .. }` by emitting a glyph instance via `push_glyph()` on the mono writer with the atlas page and UV from the icon cache
  - [x] **FILE SIZE NOTE:** `draw_list_convert/mod.rs` is at 444 lines. Adding `DrawCommand::Icon` handling (~10 lines) stays within the 500-line limit
  - [x] Icons must be resolved to `(atlas_page, uv)` BEFORE draw list emission (in the widget's `draw()` method). Pass `&ResolvedIcons` through `DrawCtx` so widgets can look up atlas regions at draw time
- [x] **Phase 4: Widget integration** (one widget at a time -- depends on Phases 1-3): (verified 2026-03-29, tab bar + window chrome + dropdown all using push_icon)
  - [x] Widget integration -- tab bar:
    - [x] `draw.rs`: replace `push_line()` calls for close x, + button, and chevron with `push_icon()` referencing cached atlas region (with push_line fallback when icons not resolved)
    - [x] `drag_draw.rs`: replace `push_line()` calls for close x on dragged tab with `push_icon()` (with fallback)
    - [x] Icon size derived from tab bar constants (e.g., `CLOSE_BUTTON_WIDTH - 2*CLOSE_ICON_INSET`)
    - [x] Hover color change: shader tints icon to hover color via `fg_color` attribute (no re-rasterization needed)
  - [x] Widget integration -- window chrome:
    - [x] `controls.rs`: replace `push_line()` calls for minimize and close, and `push_rect()` calls for maximize and restore, with `push_icon()` referencing cached atlas regions (with fallback)
    - [x] Maximize/restore: swap icon based on `is_maximized` state (existing logic, icon lookup by `IconId::Maximize` vs `IconId::Restore`)
    - [x] Close button hover: white icon on red background (shader tints to white via `fg_color`)
  - [x] Widget integration -- dropdown:
    - [x] `oriterm_ui/src/widgets/dropdown/mod.rs`: replace `push_line()` calls for chevron indicator with `push_icon()` referencing `ICON_CHEVRON_DOWN` atlas region (with fallback)
    - [x] Chevron uses the same icon definition but may need a smaller size variant (4px arrow_half vs tab bar's `CHEVRON_HALF_W`)
- [x] **Phase 5: Cleanup** (after all widgets migrated and `ResolvedIcons` wired into `WindowRenderer`): (verified 2026-03-29, no push_line fallback branches remain for icons)
  - [x] Remove icon-specific `push_line()` fallback branches from `draw.rs`, `drag_draw.rs`, `controls.rs`, and `dropdown/mod.rs`
  - [x] Keep the general `push_line()` / `convert_line()` infrastructure (still used for menu checkmarks, menu separators, tab separators, checkbox checkmarks, dialog separators, and `separator/mod.rs`) and `push_rect()` (still used everywhere for non-icon rectangles)

**Tests:**
- [x] Rasterize close icon at 16px, 24px, 32px -- output is non-empty with correct dimensions (`size_px * size_px` bytes for alpha-only R8 data)
- [x] Rasterize at different sizes produces different pixel data (not a byte-for-byte duplicate)
- [ ] Icon cache returns same `AtlasRegion` for same `(IconId, size_px)` key (cache hit) -- requires GPU test harness
- [ ] DPI change invalidates cache; subsequent lookup re-rasterizes and returns new region -- requires GPU test harness
- [x] Rasterized close icon at 2.0x scale has non-zero alpha along the diagonal (no staircase gap)

---

## 24.6 Background Images

**COMPLEXITY WARNING:** New GPU pipeline + texture management. The `record_draw_passes()` function already has a `#[expect(clippy::too_many_lines)]` annotation -- adding another pipeline makes it longer. Consider extracting per-tier draw helpers if any logical block exceeds 50 lines.

**FILE SIZE WARNING (UPDATED 2026-03-29):** `render.rs` is now at 735 lines -- ALREADY exceeds the 500-line limit. This is a pre-existing hygiene violation that MUST be addressed before 24.6 work begins. Extract existing render pass helper functions into a sibling file within `window_renderer/` (e.g., `window_renderer/render_passes.rs`) BEFORE adding the background image pass. Note: `render.rs` is a plain file inside the `window_renderer/` directory module -- extraction creates sibling files. Similarly, `pipeline/mod.rs` is at exactly 500 lines -- new pipeline creation functions MUST go in a new submodule (e.g., `pipeline/bg_image.rs`).

**CONFIG WARNING:** `config/mod.rs` is at 331 lines. Sections 24.6, 24.7, and 24.8 collectively add ~40 lines of config structs/fields/impls to `WindowConfig`. Consider extracting `WindowConfig` and its impls into a `config/window.rs` submodule if the file approaches 400 lines after 24.6.

Display a background image behind the terminal grid.

**File:** `oriterm/src/gpu/window_renderer/render.rs` (render pass -- extract helpers first), `oriterm/src/gpu/shaders/bg_image.wgsl` (new shader), `oriterm/src/config/mod.rs` (config -- `WindowConfig`), `oriterm/src/gpu/bg_image/mod.rs` (new -- texture loading + GPU upload), `oriterm/src/gpu/bg_image/tests.rs` (new -- position mode UV computation tests)

- [ ] Config options:
  ```toml
  [window]
  background_image = "/path/to/image.png"
  background_image_opacity = 0.1
  background_image_position = "center"  # center | stretch | tile | fill
  ```
- [ ] Config fields on `WindowConfig` in `oriterm/src/config/mod.rs`:
  - [ ] Add `background_image: Option<String>` (default: `None`)
  - [ ] Add `background_image_opacity: f32` (default: `0.1`)
  - [ ] Add `background_image_position: BackgroundImagePosition` (default: `Center`)
  - [ ] Create `BackgroundImagePosition` enum: `Center`, `Stretch`, `Fill`, `Tile` with `#[serde(rename_all = "lowercase")]`
  - [ ] Add `effective_background_image_opacity()` clamped to [0.0, 1.0] (matching the pattern of other `effective_*` methods)
  - [ ] Update `Default for WindowConfig` to include new fields
  - [ ] Config placement note: these fields go on app-layer `WindowConfig` only. The `oriterm_ui::window::WindowConfig` (window creation struct) does NOT get these fields -- background rendering is a GPU concern
- [ ] Image loading:
  - [ ] Load at startup and on config reload (hot-reload)
  - [ ] Decode PNG/JPEG/BMP via `image` crate. Currently `build-dependencies` and `dev-dependencies` only in `oriterm/Cargo.toml` -- add `image` to `[dependencies]` with features (`png`, `jpeg`, `bmp`). The existing `build-dependencies` entry (for icon embedding at build time) is separate and stays as-is
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
    - [ ] Texture handle and opacity stored on `PreparedFrame` during prepare phase (rendering discipline: no config access during render, no state mutation)
  - [ ] New shader: `oriterm/src/gpu/shaders/bg_image.wgsl` -- samples texture, applies opacity uniform, outputs premultiplied alpha
  - [ ] New pipeline: add `bg_image_pipeline` to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Cell backgrounds blend over the image (existing bg pipeline uses `src*1 + dst*(1-srcA)` blend -- already works)
  - [ ] Position/scale image according to `background_image_position` -- compute UV coordinates on CPU, pass as instance data or uniform
- [ ] Position modes:
  - [ ] `center`: original size, centered, crop if larger than window. UV computed from window/image size ratio
  - [ ] `stretch`: scale to fill window, may distort aspect ratio. UV = 0..1 on both axes
  - [ ] `fill`: scale to fill, maintaining aspect ratio, crop excess. UV computed to crop the shorter axis
  - [ ] `tile`: repeat at original size. Requires `AddressMode::Repeat` on the sampler (the atlas sampler uses `ClampToEdge`, so `tile` mode needs a dedicated sampler)
- [ ] Handle window resize: recompute UV coordinates on resize (no re-decode needed)
- [ ] Memory: keep decoded texture in GPU memory only. Drop the `image::DynamicImage` after GPU upload
- [ ] Module registration:
  - [ ] Create `oriterm/src/gpu/bg_image/mod.rs` -- image loading, GPU texture creation, position mode UV computation. Add `#[cfg(test)] mod tests;` at bottom
  - [ ] Create `oriterm/src/gpu/bg_image/tests.rs` -- UV coordinate computation tests for each position mode
  - [ ] Add `pub(crate) mod bg_image;` to `oriterm/src/gpu/mod.rs`
  - [ ] Add `bg_image_pipeline` to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Add pipeline creation function `create_bg_image_pipeline()` in a new `oriterm/src/gpu/pipeline/bg_image.rs` submodule (NOT in `pipeline/mod.rs` which is at 500 lines). Add `mod bg_image;` and re-export from `pipeline/mod.rs`
  - [ ] Add `image` to `[dependencies]` in `oriterm/Cargo.toml` (with features `png`, `jpeg`). The existing `build-dependencies` and `dev-dependencies` entries for `image` are separate and stay as-is

**Tests:**
- [ ] Image loads from valid path, returns error for missing path
- [ ] Corrupt/truncated image file returns error gracefully (no panic)
- [ ] Image exceeding `max_texture_dimension_2d` is rejected with log warning
- [ ] Position mode `center` computes correct UV coordinates for window larger than image and image larger than window
- [ ] Position mode `stretch` produces UV = 0..1 on both axes regardless of aspect ratio
- [ ] Position mode `fill` maintains aspect ratio (UV covers window, may exceed 0..1 on one axis)
- [ ] Position mode `tile` uses `AddressMode::Repeat` sampler
- [ ] Opacity multiplier applied correctly in shader
- [ ] Config reload swaps background image without restart
- [ ] Config change from image path to `None` removes the background image draw call

---

## 24.7 Background Gradients

GPU-rendered gradient backgrounds as an alternative to solid colors or images.

**DEPENDENCY:** This section depends on the `render.rs` extraction done in 24.6 (extracting render pass helpers to stay under 500 lines). If 24.6's render pass extraction is not done, this section will exceed the file size limit.

**File:** `oriterm/src/gpu/window_renderer/render.rs` (render pass -- must be extracted per 24.6), `oriterm/src/gpu/shaders/bg_gradient.wgsl` (new shader), `oriterm_ui/src/draw/gradient.rs` (gradient data structures -- exists), `oriterm/src/gpu/pipelines.rs` (add `bg_gradient_pipeline`)

**Reference:** WezTerm `background` config (gradient presets + custom)

- [ ] Config (add to `WindowConfig` in `oriterm/src/config/mod.rs`):
  ```toml
  [window]
  background_gradient = "none"  # "none", "linear", "radial"
  gradient_colors = ["#1e1e2e", "#313244"]  # start and end colors
  gradient_angle = 180  # degrees, for linear gradient (CSS convention: 0 = bottom-to-top, 180 = top-to-bottom)
  gradient_opacity = 1.0  # 0.0-1.0, blended with background color
  ```
  - [ ] Create `GradientType` enum: `None`, `Linear`, `Radial` with `#[serde(rename_all = "lowercase")]`
  - [ ] Add fields to `WindowConfig`: `background_gradient: GradientType` (default `None`), `gradient_colors: Vec<String>` (default empty), `gradient_angle: f32` (default `180.0`), `gradient_opacity: f32` (default `1.0`)
  - [ ] Update `Default for WindowConfig`
  - [ ] Add `effective_gradient_opacity()` clamped to [0.0, 1.0]
  - [ ] Validate `gradient_colors` has at least 2 entries when `background_gradient != None`; log warning and fall back to `None` if fewer
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
  - [ ] Draw order in `record_draw_passes()`: clear -> gradient -> background image -> terminal cell backgrounds -> text -> cursors -> chrome -> overlays
  - [ ] If both gradient and image configured: gradient renders first, image blends on top with alpha
  - [ ] Cell backgrounds blend on top of gradient (existing blend mode handles this)
  - [ ] Gradient parameters passed via a dedicated uniform buffer or packed into the existing uniform buffer (evaluate 16-byte alignment cost vs. dedicated bind group)
- [ ] Interaction with transparency:
  - [ ] Gradient respects `window.opacity` -- blended with compositor-provided background
  - [ ] `gradient_opacity` controls gradient's own alpha (independent of window opacity)
- [ ] Hot-reload: gradient config changes apply immediately
- [ ] Module registration:
  - [ ] Add pipeline creation function `create_bg_gradient_pipeline()` in a new `oriterm/src/gpu/pipeline/bg_gradient.rs` submodule (NOT in `pipeline/mod.rs` which is at 500 lines). Add `mod bg_gradient;` and re-export from `pipeline/mod.rs`
  - [ ] Add `bg_gradient_pipeline: RenderPipeline` field to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Wire pipeline creation in `GpuPipelines::new()`
  - [ ] Create gradient uniform buffer (or extend existing uniform buffer) -- must fit 2 colors (8 floats), angle (1 float), opacity (1 float). Evaluate: separate bind group at group 1 is cleaner than extending the 16-byte screen uniform buffer

**Tests:**
- [ ] Linear gradient: pixel at top differs from pixel at bottom (for 180 deg angle)
- [ ] Angle rotation: 90 deg gradient varies horizontally, not vertically
- [ ] Gradient opacity: alpha channel reflects `gradient_opacity` value
- [ ] Config `background_gradient = "none"`: no gradient rendered
- [ ] `gradient_colors` with < 2 entries falls back to no gradient with log warning
- [ ] Config hot-reload: changing gradient type applies without restart

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
- [ ] Create `Backdrop` enum: `None`, `Blur`, `Acrylic`, `Mica`, `Auto` with `#[serde(rename_all = "lowercase")]` and `Default = Auto`
- [ ] Add `backdrop: Backdrop` field to `WindowConfig`
- [ ] Update `Default for WindowConfig` to include `backdrop: Backdrop::Auto`
- [ ] Deprecate `blur: bool` config field (keep for backwards compatibility; `backdrop` takes priority when both present)
- [ ] Refactor `transparency.rs::apply_transparency()` to accept `Backdrop` enum instead of `bool blur`. Update caller in `oriterm/src/app/init/mod.rs`
- [x] Windows backdrop effects (Win32) -- partially implemented:
  - [x] `acrylic` -- `window_vibrancy::apply_acrylic()` with tint color (exists in `transparency.rs`)
  - [ ] `mica` -- `DWM_SYSTEMBACKDROP_TYPE::DWMSBT_MAINWINDOW` (Windows 11 only). Requires `windows-sys` feature `Win32_Graphics_Dwm` (already in Cargo.toml)
  - [ ] `auto` -- Mica on Windows 11, Acrylic on Windows 10
  - [x] Requires `window.opacity < 1.0` to see the effect (guarded in `apply_transparency()`)
  - [x] Uses `window-vibrancy` crate (already a dependency)
- [x] macOS backdrop effects -- partially implemented:
  - [x] `blur` -- `window_vibrancy::apply_vibrancy()` with `UnderWindowBackground` material (exists)
  - [ ] Material selection config (`.hudWindow` or `.sidebar`)
- [x] Linux backdrop effects -- implemented:
  - [x] `blur` -- `window.set_blur(true)` via winit (exists in `transparency.rs`)
  - [ ] Log a warning when compositor does not support blur (best-effort detection)
- [ ] Config mapping to `apply_transparency()`:
  - [ ] `backdrop = "none"` -- `apply_transparency(_, opacity, false, _)`
  - [ ] `backdrop = "blur"` -- `apply_transparency(_, opacity, true, _)` (current behavior)
  - [ ] `backdrop = "acrylic"` -- new code path: call `window_vibrancy::apply_acrylic()` directly
  - [ ] `backdrop = "mica"` -- new code path: Windows 11 DWM API
  - [ ] `backdrop = "auto"` -- platform detection logic
- [ ] Interaction with other features:
  - [ ] Backdrop visible only when `window.opacity < 1.0`
  - [ ] Background gradient renders on top of backdrop effect
  - [ ] Background image renders on top of backdrop effect
  - [ ] Cell backgrounds render on top of all of the above
- [ ] Error handling (graceful fallback):
  - [ ] Mica on Windows 10: log warning, fall back to Acrylic
  - [ ] Acrylic on Windows without DWM composition: log warning, fall back to `none`
  - [ ] Linux without compositor: `window.set_blur(true)` may silently fail -- detect and log

**Tests:**
- [ ] Config parsing: all `Backdrop` variants deserialize correctly from TOML
- [ ] `backdrop = "none"` disables backdrop
- [ ] `backdrop = "auto"` selects platform-appropriate effect
- [ ] Backwards compatibility: `blur = true` without `backdrop` field still enables blur
- [ ] `backdrop` field overrides `blur` field when both are present

---

## 24.9 Scrollable Menus

Add max-height constraint and scroll support to `MenuWidget` so long menus (e.g., 50+ theme entries) do not overflow the window.

**COMPLEXITY WARNING:** `menu/mod.rs` was at 499 lines. The drawing split was done (as `widget_impl.rs` instead of `draw.rs`) and scroll logic was added.

**File:** `oriterm_ui/src/widgets/menu/mod.rs` (`MenuWidget` + `MenuStyle` -- 376 lines post-split), `oriterm_ui/src/widgets/menu/widget_impl.rs` (drawing + Widget impl -- 336 lines), `oriterm_ui/src/widgets/scroll/mod.rs` (existing scroll widget for reference)

- [x] **Prerequisite: split `menu/mod.rs`** to make room for scroll code: (verified 2026-03-29, split done as `widget_impl.rs` instead of planned `draw.rs` -- same effect, mod.rs now 376 lines)
  - [x] Extract drawing logic into `oriterm_ui/src/widgets/menu/widget_impl.rs` (verified 2026-03-29)
  - [x] Widget struct, entry types, style, layout, and event handling remain in `mod.rs` (verified 2026-03-29)
  - [x] `mod.rs` well under 500 lines after split (376 lines) (verified 2026-03-29)
- [x] `max_height: Option<f32>` on `MenuStyle` (default: None = unlimited) (verified 2026-03-29)
- [x] `scroll_offset: f32` field on `MenuWidget` -- vertical scroll position (0.0 = top) (verified 2026-03-29)
- [x] When content height exceeds `max_height`: clip entries and show vertical scrollbar (verified 2026-03-29)
  - [x] `visible_height()` clamped by `max_height` (verified 2026-03-29)
  - [x] In `draw()`: `push_clip()` / `pop_clip()` when scrollable (verified 2026-03-29, widget_impl.rs lines 55-63, 68-71)
  - [x] Offset all entry Y positions by `-scroll_offset` during draw (verified 2026-03-29)
- [x] Scrollbar: thin track on right edge, thumb sized proportionally to visible/total ratio (verified 2026-03-29)
  - [x] `SCROLLBAR_WIDTH` and `SCROLLBAR_MIN_THUMB` constants (verified 2026-03-29)
  - [x] `draw_scrollbar()` with proportional thumb (verified 2026-03-29, widget_impl.rs lines 307-334)
  - [x] Only visible when `is_scrollable()` (verified 2026-03-29)
- [x] Mouse wheel scrolls menu content: (verified 2026-03-29)
  - [x] Handle `ScrollDelta::Pixels` and `ScrollDelta::Lines` (verified 2026-03-29, widget_impl.rs lines 99-110)
  - [x] `scroll_by(delta)` with clamping (verified 2026-03-29)
- [x] Keyboard navigation auto-scrolls to keep hovered item visible: (verified 2026-03-29)
  - [x] `ensure_visible(index)` for keyboard navigation auto-scroll (verified 2026-03-29, mod.rs line 249)
  - [x] ArrowDown/ArrowUp call `ensure_visible()` after `navigate()` (verified 2026-03-29, widget_impl.rs lines 135-152)
  - [ ] `PageUp`/`PageDown` keys in `handle_key()` -- NOT IMPLEMENTED (only ArrowDown, ArrowUp, Enter, Space, Escape handled)
  - [ ] `Home`/`End` keys in `handle_key()` -- NOT IMPLEMENTED
- [x] Scroll position resets to top when menu opens: (verified 2026-03-29, `MenuWidget::new()` starts at `scroll_offset: 0.0`)
- [x] `entry_at_y()` accounts for scroll offset (verified 2026-03-29, mod.rs line 274)
- [ ] **ZERO scroll-related tests** -- all 8 planned test items are unaddressed despite substantial implementation (gap identified 2026-03-29)

**Tests:** (all UNIMPLEMENTED as of 2026-03-29 -- 0 of 8 tests exist despite substantial scroll implementation)
- [ ] Menu with 5 entries and no `max_height`: no clipping, no scrollbar, full height
- [ ] Menu with 50 entries and `max_height = 300.0`: layout height is 300.0, not `50 * item_height`
- [ ] Mouse wheel scroll adjusts `scroll_offset`, clamped to valid range
- [ ] `scroll_offset` cannot go negative or past `total_height - max_height`
- [ ] Keyboard navigate to entry beyond visible area auto-scrolls to reveal it
- [ ] `Home` key scrolls to top and hovers first clickable entry (blocked: Home key not implemented)
- [ ] `End` key scrolls to bottom and hovers last clickable entry (blocked: End key not implemented)
- [ ] Scroll position resets on menu rebuild (new entries)

---

## 24.10 Section Completion

- [ ] All 24.1--24.9 items complete
- [x] Cursor blinks at configured rate for blinking DECSCUSR styles (verified 2026-03-29)
- [x] Cursor blink resets on keypress, mouse click, and PTY cursor movement (verified 2026-03-29)
- [x] Unfocused windows show steady hollow cursor (no blink) (verified 2026-03-29)
- [x] Mouse cursor hides when typing, reappears on move (verified 2026-03-29)
- [x] Mouse hiding respects mouse reporting mode (does not hide when app uses mouse) (verified 2026-03-29)
- [ ] Minimum contrast enforces readable text (WCAG 2.0 in shader)
- [ ] `minimum_contrast = 1.0` (default) adds zero per-vertex overhead (shader short-circuits)
- [ ] HIDDEN cells (SGR 8) are not revealed by minimum contrast at any setting
- [ ] HiDPI displays render crisp text at correct scale
- [ ] Moving between monitors with different DPI works
- [ ] Font zoom (Ctrl+=/Ctrl+-/Ctrl+0) works correctly at all DPI scales
- [x] Vector icons (close x, +, chevron, minimize, maximize, restore, window close) render with smooth anti-aliasing at all DPI scales (verified 2026-03-29)
- [x] No jagged staircase artifacts on diagonal lines in any icon (verified 2026-03-29)
- [x] Dropdown chevron icons render with smooth anti-aliasing (same as tab bar) (verified 2026-03-29)
- [ ] Background images render behind terminal content
- [ ] Background gradients render behind terminal content
- [ ] Backdrop effects (blur/acrylic/mica) apply on supported platforms
- [ ] Scrollable menus handle 50+ entries without window overflow
- [ ] All features configurable and hot-reloadable
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` pass
- [ ] No new `#[allow(clippy)]` without `reason`

**Exit Criteria:** Terminal feels visually polished at first launch -- cursor blinks, text is readable, HiDPI is crisp, icons are smooth, scrolling works, and all features are configurable and hot-reloadable.

**Verification Notes (2026-03-29):**
- **HYGIENE-001:** `render.rs` at 735 lines exceeds 500-line limit. Must be split before 24.6/24.7 work.
- **HYGIENE-002:** `pipeline/mod.rs` at exactly 500 lines. New pipelines must go in submodules.
- **GAP-001:** Zero scroll tests for menu widget (24.9) despite substantial implementation. 8 planned test items unaddressed.
- **GAP-002:** Zero tests for `handle_dpi_change()` (24.4). Font re-rasterization logic untested.
- **GAP-003:** Zero tests for backdrop/transparency (24.8). Low severity (thin platform dispatch).
- **STALE-001:** 24.9 was marked not-started but is substantially implemented. Updated to in-progress.
