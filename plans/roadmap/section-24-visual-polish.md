---
section: 24
title: Visual Polish
status: in-progress
reviewed: true
third_party_review:
  status: none
  updated: null
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
    status: not-started
  - id: "24.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "24.10"
    title: Section Completion
    status: not-started
---

# Section 24: Visual Polish

**Status:** In Progress (24.1 cursor blink complete; 24.4 HiDPI and 24.8 backdrop effects partially implemented)
**Goal:** Small visual features that collectively create a polished, modern feel. Each is low-to-medium effort but highly visible. These are the details people notice in the first 5 minutes. Missing cursor blink, broken HiDPI, or unreadable colors are dealbreakers.

**Crate:** `oriterm` (app layer + GPU rendering in `oriterm/src/gpu/`), `oriterm_ui` (widgets)
**Dependencies:** `image` (for background images -- currently build/dev only, needs runtime dep), `tiny-skia` (already in Cargo.toml), `window-vibrancy` (already in Cargo.toml), existing wgpu pipeline

**Internal dependencies between subsections:**
- 24.1 (Cursor Blinking) -- standalone
- 24.2 (Hide Cursor) -- standalone
- 24.3 (Minimum Contrast) -- standalone, but the uniform buffer change here affects 24.6/24.7 if they also add uniforms
- 24.4 (HiDPI) -- standalone, but 24.5 (icons) depends on DPI scale factor from this subsection
- 24.5 (Vector Icons) -- depends on 24.4 for scale factor; complete before 24.6/24.7 to avoid redoing icon rendering
- 24.6 (Background Images) -- standalone (new pipeline + `record_cached_content_passes()` insertion); if 24.3 is done first the uniform buffer is already updated
- 24.7 (Background Gradients) -- depends on 24.6 (same draw order slot in `record_cached_content_passes()`; gradient goes before the background image draw)
- 24.8 (Backdrop Effects) -- standalone (compositor-level via `window-vibrancy`/DWM, independent of GPU render passes)
- 24.9 (Scrollable Menus) -- standalone (`oriterm_ui` only, no GPU changes)

**Recommended implementation order:** 24.1 -> 24.2 -> 24.9 -> 24.3 -> 24.4 -> 24.5 -> 24.6 -> 24.7 -> 24.8

---

## 24.1 Cursor Blinking

Toggle cursor visibility on a timer. **Complete**: `CursorBlink` state machine exists (`oriterm_ui/src/animation/cursor_blink/mod.rs`) with `is_visible()`, `reset()`, and `next_toggle()`. The `about_to_wait` handler drives blink via `ControlFlow::WaitUntil`. `cursor_blink_visible` is threaded through the prepare pipeline. Focus handling (unfocused hollow cursor), mouse click reset, and PTY cursor-move reset are all implemented.

**File:** `oriterm_ui/src/animation/cursor_blink/mod.rs` (blink state -- exists), `oriterm/src/app/event_loop.rs` (`about_to_wait` blink timer -- exists), `oriterm/src/app/redraw/mod.rs` (cursor blink visible flag -- exists), `oriterm/src/gpu/prepare/mod.rs` (cursor emission gating -- exists)

- [x] Blink state tracking:
  - [x] `CursorBlink` struct with `last_visible`, `epoch`, `interval` fields on `App`
  - [x] Blink interval: 530ms on / 530ms off (configurable via `cursor_blink_interval_ms`)
  - [x] `update()` checks `is_visible()` (pure function of elapsed time since epoch), caches the result, and returns `true` if visibility changed
- [x] DECSCUSR blinking style detection:
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

**File:** `oriterm_ui/src/interaction/cursor_hide/mod.rs` (pure decision logic + `HideContext`/`should_hide_cursor()` -- exists), `oriterm/src/app/mod.rs` (state + `restore_mouse_cursor`), `oriterm/src/app/keyboard_input/mod.rs` (keypress hiding -- calls `oriterm_ui::interaction::cursor_hide::should_hide_cursor`), `oriterm/src/app/event_loop.rs` (`CursorMoved`/`CursorLeft`/`Focused` restore), `oriterm/src/config/behavior.rs` (config)

- [x] Hide mouse cursor on keypress:
  - [x] Track `mouse_cursor_hidden: bool` on `App`
  - [x] Pure decision function `should_hide_cursor(HideContext)` in `oriterm_ui/src/interaction/cursor_hide/mod.rs`
  - [x] Called from `encode_key_to_pty()` — hides via `window.set_cursor_visible(false)`
  - [x] Skip modifier-only keys (`NamedKey::Shift`, `NamedKey::Control`, `NamedKey::Alt`, `NamedKey::Super`, `Hyper`, `Meta`)
  - [x] Skip when IME composition active (`ime.should_suppress_key()`)
- [x] Restore mouse cursor on mouse move:
  - [x] `restore_mouse_cursor()` helper on `App` — only calls `set_cursor_visible(true)` when `mouse_cursor_hidden` is true
  - [x] Called on `CursorMoved` event in `event_loop.rs`
  - [x] Called on `CursorLeft` event to avoid sticky hidden state
- [x] Suppress hiding during mouse reporting mode:
  - [x] Check `TermMode::ANY_MOUSE` inline at the call site (already have `mode` from `pane_mode()`)
  - [x] Passed as `mouse_reporting` field in `HideContext`
- [x] Restore cursor on window focus loss:
  - [x] `restore_mouse_cursor()` called on `WindowEvent::Focused(false)`
- [x] Config: `hide_mouse_when_typing: bool` (default: true) on `BehaviorConfig`
  - [x] All hide/show logic gated on `self.config.behavior.hide_mouse_when_typing`
  - [x] Added to `Default for BehaviorConfig`

**Tests:** Pure function `should_hide_cursor()` tested in `oriterm_ui/src/interaction/cursor_hide/tests.rs`.
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

**FILE SIZE WARNING:** `oriterm/src/gpu/pipeline/mod.rs` is at 391 lines (as of last audit). The 24.3 uniform buffer change (repurposing `_pad.x`) does not change the buffer layout or require pipeline modifications -- it only changes the data written via `write_uniforms()`. Sections 24.6 and 24.7 add NEW pipelines whose creation functions must go in new submodules (e.g., `pipeline/bg_image.rs`, `pipeline/bg_gradient.rs`), not in `pipeline/mod.rs`. The existing `pipeline/image.rs` and `pipeline/ui_rect.rs` submodules already exist -- follow their pattern for new pipeline submodules.

**FILE SIZE WARNING:** `oriterm/src/gpu/prepare/mod.rs` is at 487 lines (as of last audit). The HIDDEN cell fix in Step 2 adds approximately 5 lines, leaving the file at ~492 lines — just under the 500-line limit. Do NOT add any other code to `prepare/mod.rs` during 24.3. If the file reaches 500 lines before the HIDDEN fix is applied, split `resolve_cell_colors()` and its helpers into a new `prepare/color_resolve.rs` submodule first.

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
    - `oriterm/src/gpu/window_renderer/render.rs`: two call sites exist — `render_frame()` at line ~41 and a second call at line ~117 (both write screen size before draw calls). Both must be updated to `write_uniforms()`.
    - `oriterm/src/gpu/bind_groups/tests.rs`: rename test functions `write_screen_size_does_not_panic` → `write_uniforms_does_not_panic` and `write_screen_size_zero_dimensions` → `write_uniforms_zero_dimensions`. Also update `uniform_buffer_creation_succeeds` and `uniform_bind_group_accessor_returns_valid_ref` if they call `write_screen_size` anywhere (they do not, but the module doc comment references the old name and must be updated). Update function body calls from `write_screen_size` to `write_uniforms(queue, w, h, min_contrast)`.
    - Update doc comments on `UniformBuffer` struct and module-level doc in `bind_groups/mod.rs`
  - [ ] Rename `_pad: vec2<f32>` to `extra: vec2<f32>` in 7 shaders that use the main screen `Uniform` struct: `fg.wgsl`, `bg.wgsl`, `subpixel_fg.wgsl`, `color_fg.wgsl`, `ui_rect.wgsl`, `image.wgsl`, and `composite.wgsl` (which names its struct `ScreenUniform` but has the same `_pad: vec2<f32>` field). `colr_solid.wgsl` and `colr_gradient.wgsl` do NOT use the group-0 screen uniform — they are render-to-texture shaders with their own bind group layouts (`FillUniforms`/`GradientUniforms`) and are not affected. Buffer size stays 16 bytes.
- [ ] **Step 3: WGSL shader port** -- port the validated Rust functions into `fg.wgsl` and `subpixel_fg.wgsl`:
  - [ ] Add `luminance()`, `contrast_ratio()`, `contrasted_color()` as WGSL functions
  - [ ] **sRGB-to-linear conversion**: the `luminance()` function in WGSL expects linear RGB. Colors in the instance buffer are already in linear space (the prepare phase converts via `srgb_to_linear()` in `gpu/mod.rs`). The `*Srgb` surface format handles the final linear-to-sRGB conversion on output. No additional sRGB conversion is needed in the shader
  - [ ] Apply contrast adjustment in `vs_main` (vertex shader) for both `fg.wgsl` and `subpixel_fg.wgsl`. In `fg.wgsl`, `bg_color` is a per-instance attribute in `vs_main` but is NOT passed to the fragment stage, so contrast must be applied per-vertex. In `subpixel_fg.wgsl`, `bg_color` IS passed through to `fs_main` (for per-channel compositing), but vertex stage is preferred for consistency:
    ```wgsl
    out.fg_color = contrasted_color(uniforms.extra.x, input.fg_color, input.bg_color);
    ```
  - [ ] Only apply in `fg.wgsl` and `subpixel_fg.wgsl` (text shaders). `bg.wgsl`, `ui_rect.wgsl`, `image.wgsl` do not render text. `color_fg.wgsl` (emoji/color glyphs): skip contrast -- adjusting bitmap colors would distort them
  - [ ] **UI text side effect**: UI text rendered via `oriterm/src/gpu/scene_convert/mod.rs` (the actual draw-list-to-GPU conversion module — `draw_list_convert` does not exist) uses the same `fg.wgsl` and `subpixel_fg.wgsl` shaders. The `min_contrast` uniform will apply to UI text (tab labels, menu items) as well as terminal cells. This is acceptable (Ghostty does the same) but should be documented as intentional behavior.
- [ ] Hot-reload: `minimum_contrast` value read from `config.colors.effective_minimum_contrast()` each frame, so config changes apply immediately
- [ ] Threading `min_contrast` to `write_uniforms()`: add `min_contrast: f32` field to `PreparedFrame` in `oriterm/src/gpu/prepared_frame/mod.rs`, set during `prepare()` from the config. This keeps `render_frame()` pure (reads from `self.prepared`, no config access during render). `PreparedFrame` currently has no `min_contrast` field — initialize it to `1.0` (disabled) in both `PreparedFrame::new()` and `PreparedFrame::with_capacity()`.

**Edge cases:**
- [ ] `minimum_contrast = 1.0` (default, disabled): shader short-circuits -- no luminance computation, pass fg through unchanged. Cost: one branch per vertex, zero overhead when disabled
- [ ] HIDDEN cells (SGR 8): do not reveal. SGR 8 sets `CellFlags::HIDDEN` on the cell but does NOT set `fg = bg`. The fg and bg colors remain independently resolved by `resolve_fg()`/`resolve_bg()` in `oriterm_core/src/term/renderable/mod.rs` and `apply_inverse()`. The shader cannot rely on `fg == bg` to detect HIDDEN cells. **Solution**: the prepare phase (`resolve_cell_colors()` in `oriterm/src/gpu/prepare/mod.rs`) must detect `CellFlags::HIDDEN` and explicitly set `fg = bg` before writing to the instance buffer. This makes the `fg == bg` signal reliable for the shader's `contrasted_color()` to detect and skip. Do NOT use `fg_color.a = 0.0` as a signal (it would break premultiplied alpha blending for all text)
- [ ] **Prepare phase change for HIDDEN**: in `resolve_cell_colors()` in `oriterm/src/gpu/prepare/mod.rs`, add an early return before the final `return (cell.fg, cell.bg)` at line 160: `if cell.flags.contains(CellFlags::HIDDEN) { return (bg, bg); }`. This must come after selection handling (so selected HIDDEN cells remain hidden). Because the fix is in the shared function itself, it automatically covers all three call sites: `prepare/mod.rs:357` (shaped path), `prepare/unshaped.rs:95` (test path), and `prepare/dirty_skip/mod.rs:377` (incremental path). No changes needed at any call site.
- [ ] Reverse video cells (SGR 7): contrast uses the already-swapped fg/bg (no special handling needed -- `apply_inverse()` runs in the renderable layer before colors reach the prepare phase)
- [ ] Bold/dim flags: contrast applied after bold-bright resolution (handled at the terminal renderable layer in `oriterm_core/src/term/renderable/mod.rs::resolve_fg()`) and dim adjustments. The shader sees final resolved colors; no special handling needed

**Tests:** (in `oriterm/src/gpu/contrast/tests.rs`)
- [ ] `minimum_contrast_disabled_passes_through`: white on black at `minimum_contrast = 1.0` returns fg unchanged (short-circuit)
- [ ] `contrast_boost_dark_bg`: dark gray (#333) on black at `minimum_contrast = 4.5` adjusts fg to a lighter color (result luminance ratio >= 4.5)
- [ ] `contrast_boost_light_bg`: light gray (#ccc) on white at `minimum_contrast = 4.5` adjusts fg to a darker color (result luminance ratio >= 4.5)
- [ ] `luminance_bt709_pure_colors`: correct relative luminance for pure red (0.2126), green (0.7152), blue (0.0722), white (1.0), black (0.0)
- [ ] `contrast_ratio_extremes`: `contrast_ratio(1.0, 0.0)` approximately 21.0; `contrast_ratio_approx_4` tests a known pair near 4.0
- [ ] `contrasted_color_chooses_white_for_dark_bg`: picks white direction for backgrounds with luminance < 0.5
- [ ] `contrasted_color_chooses_black_for_light_bg`: picks black direction for backgrounds with luminance >= 0.5
- [ ] `contrasted_color_fg_eq_bg_returns_unchanged`: `fg == bg` (HIDDEN cell signal) passes fg through unchanged regardless of `min_contrast` value -- do NOT boost contrast when fg equals bg, as that would reveal hidden text
- [ ] `contrasted_color_already_meets_ratio`: fg that already meets `min_contrast` vs bg is returned unchanged (no unnecessary adjustment)
- [ ] `contrasted_color_max_ratio_21`: at `min_contrast = 21.0` (maximum), output is either pure black or pure white (only valid extremes achieve 21:1)
- [ ] Config `effective_minimum_contrast()` clamps NaN to 1.0, values outside [1.0, 21.0] to nearest bound (tested in `oriterm/src/config/tests.rs`: `minimum_contrast_nan_defaults_to_one`, `minimum_contrast_inf_clamped_to_twenty_one`, `minimum_contrast_clamped`)
- [ ] `write_uniforms_encodes_min_contrast`: extract byte-packing logic from `write_uniforms()` into a private `pack_uniform_bytes(width, height, min_contrast) -> [u8; 16]` helper, then test that helper directly (pure function, no GPU needed). Assert bytes[8..12] equal `4.5f32.to_le_bytes()` when `min_contrast = 4.5`. The existing GPU tests in `bind_groups/tests.rs` use `GpuState::new_headless()` — add a complementary `write_uniforms_does_not_panic` GPU test following that same pattern.


**Tests:** (in `oriterm/src/gpu/prepare/tests.rs` -- HIDDEN cell prepare phase)
- [ ] `hidden_cell_sets_fg_eq_bg`: HIDDEN cell (SGR 8) with distinct fg/bg produces `fg == bg` in resolved colors (prepare phase sets `fg = bg` so shader does not reveal text)
- [ ] `hidden_cell_under_selection_stays_hidden`: HIDDEN cell under selection still produces `fg == bg` (selection does not reveal hidden text)
- [ ] `non_hidden_cell_same_fg_bg_gets_contrast`: non-HIDDEN cell with same fg/bg is NOT treated as hidden (contrast adjustment still applies, so the shader can boost it)
- [ ] `hidden_cell_no_reveal_at_max_contrast`: HIDDEN cell with `min_contrast = 21.0` still produces `fg == bg` (contrast module's `fg==bg` guard prevents any boost)


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
  - [ ] **Font zoom implementation** -- font zoom state is per-window (different windows can have different zoom levels, matching Alacritty and WezTerm behavior). Store `logical_font_size: f32` on `TermWindow` (NOT on `App`). Initialize from `config.font.size`. `App` dispatches zoom actions to the focused window via the existing `execute_action()` in `action_dispatch.rs`.
  - [ ] Add `increase_font_size()`, `decrease_font_size()`, `reset_font_size()` methods to `TermWindow` that adjust `self.logical_font_size` and call `renderer.set_font_size(self.logical_font_size * DEFAULT_DPI * self.scale_factor.get(), gpu)`
  - [ ] Zoom increment: 1pt per step. Minimum: 4pt (prevent invisible text). Maximum: 72pt (prevent runaway zoom). Clamp in the zoom methods.
  - [ ] **Wire the existing stub in `action_dispatch.rs`**: `Action::ZoomIn`, `Action::ZoomOut`, and `Action::ZoomReset` already exist in `oriterm/src/keybindings/mod.rs` and are already bound (`Ctrl+=`/`Ctrl++`/`Ctrl+-`/`Ctrl+0` in `keybindings/defaults.rs`). They currently match a stub at line 238 of `action_dispatch.rs` that logs "not yet implemented". Replace that stub with real dispatch to `increase_font_size()`, `decrease_font_size()`, and `reset_font_size()` on the focused `TermWindow`. Do NOT create new action variants or new keybindings.
  - [ ] Zoom operations must account for scale factor: the font size passed to the renderer is `logical_size * (DEFAULT_DPI * scale_factor)`, so zoom increments apply to the logical size and then re-multiply
  - [ ] `reset_font_size()` resets `self.logical_font_size` to `config.font.size`
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
- [ ] `handle_dpi_change_calls_set_font_size`: `handle_dpi_change()` calls `renderer.set_font_size()` with `config.font.size * (DEFAULT_DPI * new_scale)`. Pure unit test using a mock renderer or by inspecting `renderer.current_font_size()` after the call.
- [ ] `handle_dpi_change_marks_all_dirty`: after `handle_dpi_change()`, `mux.mark_all_dirty()` has been called (verified via MuxBackend mock or by inspecting dirty state)
- [ ] Grid dimensions (columns, rows) recalculated after DPI change -- verified by checking that `TermWindow::grid_size()` returns updated values proportional to the new scale
- [ ] `increase_font_size_zoom_applies_to_logical_size`: calling `increase_font_size()` with a default 1pt increment adjusts `logical_font_size` and re-multiplies by `scale_factor * DEFAULT_DPI` before calling `set_font_size()`
- [ ] `decrease_font_size_clamps_to_minimum`: calling `decrease_font_size()` below the minimum font size (e.g. 4pt) clamps to the minimum rather than going negative or zero
- [ ] `increase_font_size_clamps_to_maximum`: calling `increase_font_size()` above the maximum font size (e.g. 72pt) clamps to the maximum
- [ ] `reset_font_size_restores_config_value`: calling `reset_font_size()` restores `logical_font_size` to `config.font.size * scale_factor`
- [ ] `zoom_keybinding_wired_to_action`: `Ctrl+=` maps to `Action::ZoomIn`, `Ctrl+-` maps to `Action::ZoomOut`, `Ctrl+0` maps to `Action::ZoomReset` in `default_bindings()` in `oriterm/src/keybindings/defaults.rs` (already asserted -- add a test in `oriterm/src/keybindings/tests.rs` if one doesn't exist, checking `action_for_key(Key::Character("="), ctrl_mods)`)
- [ ] `zoom_action_stub_removed`: the stub at `Action::ZoomIn | Action::ZoomOut | Action::ZoomReset` in `action_dispatch.rs` no longer logs "not yet implemented" -- it calls the real methods
- [ ] Dragging window between monitors with different DPI transitions without visual artifacts (integration behavior -- note in tests that this requires manual verification; no automated test possible without multi-monitor setup)


---

## 24.5 Vector Icon Pipeline (tiny_skia)

**COMPLEXITY WARNING:** This is the largest subsection. Implement in strict phases: (1) icon data structures + rasterization, (2) atlas integration + cache, (3) widget integration (one widget at a time), (4) cleanup of old `push_line()` icon code. Do not attempt all phases at once.

Replace jagged geometric-primitive icons with properly anti-aliased vector path rasterization. This section introduces `tiny_skia` to rasterize icon paths into bitmaps at the exact DPI, cached in the glyph atlas alongside font glyphs. The conversion from scene primitives to GPU instances is handled by `oriterm/src/gpu/scene_convert/` (not `draw_list_convert`, which does not exist).

**File:** `oriterm_ui/src/icons/mod.rs` (icon path definitions -- exists), `oriterm/src/gpu/icon_rasterizer/mod.rs` (rasterization -- exists), `oriterm/src/gpu/icon_rasterizer/cache.rs` (`IconCache` -- exists), `oriterm_ui/src/widgets/tab_bar/widget/draw.rs` (consuming icons via `draw_icon()` helper), `oriterm_ui/src/widgets/window_chrome/controls.rs` (consuming icons via `ctx.icons`).

**Reference:** WezTerm `wezterm-gui/src/customglyph.rs` (`Poly`/`PolyCommand` system with `to_skia()` bridge), Chromium `components/vector_icons/` (.icon format to Skia paths)

**Dependency:** `tiny-skia` crate -- already in `oriterm/Cargo.toml` (used by COLRv1 rasterization)

- [x] **Phase 1: Library crate types** (`oriterm_ui` -- must be implemented before binary crate code):
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
- [x] **Phase 2: Rasterization + cache** (`oriterm` binary crate -- depends on Phase 1):
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
  - [x] `IconCache` struct: `HashMap<CacheKey, AtlasEntry>` where `CacheKey { id: IconId, size_px: u32 }` -- maps icon+size to atlas UV coords. (`AtlasEntry` not `AtlasRegion` -- the actual type from `gpu::atlas`.)
  - [x] On DPI scale change: invalidate icon cache, re-rasterize at new physical size
  - [x] Wire `IconCache` into `WindowRenderer` -- initialized on construction, invalidated on DPI change
- [x] **Phase 3: Draw list integration**:
  - [x] `IconPrimitive { bounds: Rect, atlas_page: u32, uv: [f32; 4], color: Color }` is the draw primitive type in `oriterm_ui/src/draw/scene/primitives.rs`. Widgets emit icons via `ctx.scene.push_icon()` which appends to `Scene::icons`. The scene-to-GPU conversion path (`scene_convert/mod.rs`) converts `IconPrimitive` via `convert_icon()`.
  - [x] `push_icon(rect, atlas_page, uv, color)` is the `DrawCtx` convenience method — emits an `IconPrimitive` to the scene
  - [x] In `oriterm/src/gpu/scene_convert/mod.rs` (the actual draw-list-to-GPU conversion module): handle `IconPrimitive` by emitting a glyph instance via the mono writer with the atlas page and UV from the icon cache. The `DrawCommand::Icon` variant lives in `oriterm_ui`'s `Scene`/`IconPrimitive` type, and `scene_convert/mod.rs` converts it via `convert_icon()` (in `scene_convert/text.rs`).
  - [x] **FILE SIZE NOTE:** `scene_convert/mod.rs` is at 367 lines. The icon conversion path is already implemented via `convert_icon()`. No further changes needed here for 24.5
  - [x] Icons are resolved to `(atlas_page, uv)` by looking up `ctx.icons` (of type `Option<&ResolvedIcons>`) in the widget's `draw()` method. `DrawCtx` already carries `pub icons: Option<&'a ResolvedIcons>` (verified in `oriterm_ui/src/widgets/contexts.rs`). If `ctx.icons` is `None`, skip icon drawing gracefully.
- [x] **Phase 4: Widget integration** (one widget at a time -- depends on Phases 1-3):
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
    - [x] `oriterm_ui/src/widgets/dropdown/mod.rs`: uses `push_icon()` referencing `IconId::DropdownArrow` (NOT `IconId::ChevronDown` — the dropdown uses a distinct `DropdownArrow` icon definition, not the tab bar chevron). Both `ChevronDown` and `DropdownArrow` are separate entries in `IconId`.
- [x] **Phase 5: Cleanup** (after all widgets migrated and `ResolvedIcons` wired into `WindowRenderer`):
  - [x] Remove icon-specific `push_line()` fallback branches from `draw.rs`, `drag_draw.rs`, `controls.rs`, and `dropdown/mod.rs`
  - [x] Keep the general `push_line()` / `convert_line()` infrastructure (still used for menu checkmarks, menu separators, tab separators, checkbox checkmarks, dialog separators, and `separator/mod.rs`) and `push_rect()` (still used everywhere for non-icon rectangles)

**Tests:**
- [x] Rasterize close icon at 16px, 24px, 32px -- output is non-empty with correct dimensions (`size_px * size_px` bytes for alpha-only R8 data)
- [x] Rasterize at different sizes produces different pixel data (not a byte-for-byte duplicate)
- [ ] Icon cache returns same `AtlasEntry` for same `(IconId, size_px)` key (cache hit). `IconCache::get_or_insert()` requires a live `GlyphAtlas`, `Device`, and `Queue`, so a full GPU test harness is needed for an end-to-end test. As a cheaper alternative: test that `IconCache::len()` returns 1 after two `get_or_insert()` calls with the same key using a mock/stub atlas.
- [ ] `IconCache::clear()` discards all entries; subsequent lookup requires re-rasterization. Test via `clear()` + `len() == 0`. The DPI-change path calls `clear()` on the `IconCache` in `WindowRenderer` and also clears the `GlyphAtlas`.
- [x] Rasterized close icon at 2.0x scale has non-zero alpha along the diagonal (no staircase gap)

---

## 24.6 Background Images

**COMPLEXITY WARNING:** New GPU pipeline + texture management.

**Render insertion point — critical:** Background image draw calls must be inserted at the top of `record_cached_content_passes()` in `oriterm/src/gpu/window_renderer/render_helpers.rs` (396 lines), before the `bg_pipeline` draw. This function encodes all terminal draw calls into a single shared `RenderPass` — adding a draw call here is consistent with the existing pattern. Do NOT add a separate render pass in `render_frame()` for backgrounds (a second pass on the same target would require `LoadOp::Load` and prevents the content cache optimization). `render_helpers.rs` is at 396 lines -- adding background image draw logic here (~20 lines) approaches the 450-line caution threshold; extract `record_bg_image_draw()` as a private helper in the same file rather than expanding `record_cached_content_passes()` body.

**FILE SIZE WARNING:** `render_helpers.rs` is at 396 lines (as of last audit). The background image draw call addition will bring it near 420 lines. `pipeline/mod.rs` is at 391 lines -- new pipeline creation functions MUST go in new submodules (e.g., `pipeline/bg_image.rs`) so `mod.rs` does not exceed the 500-line limit.

**CONFIG WARNING:** `config/mod.rs` is at 391 lines (as of last audit). Sections 24.6, 24.7, and 24.8 collectively add ~50 lines of config structs/fields/impls to `WindowConfig`. Extract `WindowConfig` and its impls into a `config/window.rs` submodule before 24.6 if the file is at or above 400 lines at implementation time.

Display a background image behind the terminal grid.

**File:** `oriterm/src/gpu/window_renderer/render_helpers.rs` (draw insertion point -- `record_cached_content_passes()`), `oriterm/src/gpu/shaders/bg_image.wgsl` (new shader), `oriterm/src/config/mod.rs` (config -- `WindowConfig`), `oriterm/src/gpu/bg_image/mod.rs` (new -- texture loading + GPU upload), `oriterm/src/gpu/bg_image/tests.rs` (new -- position mode UV computation tests)

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
  - [ ] Decode PNG/JPEG/BMP via `image` crate. Currently in `build-dependencies` and `dev-dependencies` only (`png` feature only) in `oriterm/Cargo.toml` -- add `image` to `[dependencies]` with `features = ["png", "jpeg", "bmp"]` and `default-features = false`. The existing `build-dependencies` entry (for icon embedding at build time) stays as-is with only the `png` feature.
  - [ ] Convert to RGBA8 texture for wgpu
  - [ ] Handle errors gracefully (missing file, corrupt image, unsupported format) -- log warning, continue without background image
  - [ ] Validate image dimensions: reject images larger than GPU `max_texture_dimension_2d` (typically 8192 or 16384); log error and skip
- [ ] GPU rendering:
  - [ ] Create a wgpu texture from the decoded image (RGBA8Unorm format, `TEXTURE_BINDING` usage)
  - [ ] Create a bind group for the background image texture + sampler (reuse atlas sampler or create dedicated one)
  - [ ] Add background image draw call at the top of `record_cached_content_passes()` in `oriterm/src/gpu/window_renderer/render_helpers.rs`, before the existing `bg_pipeline` draw. Use `record_bg_image_draw()` private helper to keep `record_cached_content_passes()` readable:
    - [ ] Insert before the `// Terminal tier: backgrounds.` comment
    - [ ] Full-screen quad with image texture, encoded as a single `record_draw()` call with the `bg_image_pipeline`
    - [ ] Apply `background_image_opacity` as alpha multiplier in the shader
    - [ ] Texture handle, bind group, and opacity stored on `PreparedFrame` during prepare phase (rendering discipline: no config access during render, no state mutation)
  - [ ] New shader: `oriterm/src/gpu/shaders/bg_image.wgsl` -- samples texture, applies opacity uniform, outputs premultiplied alpha
  - [ ] New pipeline: add `bg_image_pipeline: RenderPipeline` field to `GpuPipelines` struct in `oriterm/src/gpu/pipelines.rs`. Wire `create_bg_image_pipeline()` in `GpuPipelines::new()`. The module-level doc comment in `pipelines.rs` says "five render pipelines" but the struct currently has SIX (`bg`, `fg`, `subpixel_fg`, `color_fg`, `ui_rect`, `image`) — update the doc comment to reflect the new count.
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
  - [ ] Add `bg_image_pipeline: RenderPipeline` field to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Add pipeline creation function `create_bg_image_pipeline()` in a new `oriterm/src/gpu/pipeline/bg_image.rs` submodule. The existing `pipeline/image.rs` and `pipeline/ui_rect.rs` files demonstrate the expected pattern (each is ~141 lines). Add `pub(super) mod bg_image;` and re-export `create_bg_image_pipeline` from `pipeline/mod.rs`.
  - [ ] Add `image` to `[dependencies]` in `oriterm/Cargo.toml` with `features = ["png", "jpeg", "bmp"]` and `default-features = false` (matches the image loading step above). The existing `build-dependencies` and `dev-dependencies` entries stay as-is with only `png`. Do not duplicate -- this single `[dependencies]` addition covers both the loading step and the module.

**Tests:** (in `oriterm/src/gpu/bg_image/tests.rs` -- pure UV computation, no GPU needed)
- [ ] `image_load_valid_path_succeeds`: image loads from a valid test path, returns `Ok` with non-empty pixel data
- [ ] `image_load_missing_path_returns_error`: missing file returns `Err`, no panic
- [ ] `image_load_corrupt_file_returns_error`: a truncated/corrupt file returns `Err`, no panic
- [ ] `image_exceeds_max_dimension_rejected`: image with width or height > `max_texture_dimension_2d` is rejected (returns `Err`), not uploaded to GPU
- [ ] `position_center_window_larger_than_image_uv`: window 800x600, image 200x150 → UV covers image in center (non-zero margin on all sides); computed UV start and end tested against expected values
- [ ] `position_center_image_larger_than_window_uv`: window 200x150, image 800x600 → UV clips to window viewport (start/end outside 0..1 on both axes)
- [ ] `position_stretch_uv_always_full`: any window/image ratio → UV = [0.0, 0.0, 1.0, 1.0] (distorts aspect ratio)
- [ ] `position_fill_maintains_aspect_ratio`: window 800x600 (4:3), image 400x400 (1:1) → UV on wider axis covers beyond 0..1; aspect ratio maintained
- [ ] `position_fill_no_letterboxing`: with `fill` mode, the entire window is covered -- no uncovered pixels at any corner
- [ ] `position_tile_uses_repeat_address_mode`: tile mode sampler descriptor has `AddressMode::Repeat` on both U and V axes
- [ ] `effective_background_image_opacity_clamped`: opacity -0.5 clamps to 0.0; opacity 1.5 clamps to 1.0; NaN defaults to 0.1 (the default)
- [ ] `background_image_none_produces_no_draw_call`: `PreparedFrame` with `background_image = None` renders without the bg_image pipeline draw call being issued
- [ ] Config reload: changing `background_image` path at runtime applies without restart (integration behavior -- note that automated test requires config-reload infrastructure; may be manual-only at this stage)


---

## 24.7 Background Gradients

GPU-rendered gradient backgrounds as an alternative to solid colors or images.

**DEPENDENCY:** This section depends on 24.6 for the draw-order insertion point: gradient draws go into `record_cached_content_passes()` in `render_helpers.rs`, before the background image draw (draw order: gradient → image → cell backgrounds → text). If 24.6 is not complete, the insertion location is undefined.

**File:** `oriterm/src/gpu/window_renderer/render_helpers.rs` (draw insertion point -- `record_cached_content_passes()`, before 24.6's bg_image draw), `oriterm/src/gpu/shaders/bg_gradient.wgsl` (new shader), `oriterm_ui/src/draw/gradient.rs` (gradient data structures -- exists), `oriterm/src/gpu/pipelines.rs` (add `bg_gradient_pipeline`)

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
  - [ ] New pipeline: add `bg_gradient_pipeline: RenderPipeline` field to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`. Wire `create_bg_gradient_pipeline()` in `GpuPipelines::new()`.
  - [ ] Full-screen quad added at the top of `record_cached_content_passes()` in `render_helpers.rs`, before the 24.6 background image draw (if present), via `record_gradient_draw()` private helper
  - [ ] Draw order in `record_cached_content_passes()`: gradient -> background image -> terminal cell backgrounds -> text -> chrome (cursors and overlays are recorded in separate passes in `render.rs` after content)
  - [ ] If both gradient and image configured: gradient renders first, image blends on top with alpha
  - [ ] Cell backgrounds blend on top of gradient (existing blend mode handles this)
  - [ ] Gradient parameters passed via a dedicated uniform buffer or packed into the existing uniform buffer (evaluate 16-byte alignment cost vs. dedicated bind group)
- [ ] Interaction with transparency:
  - [ ] Gradient respects `window.opacity` -- blended with compositor-provided background
  - [ ] `gradient_opacity` controls gradient's own alpha (independent of window opacity)
- [ ] Hot-reload: gradient config changes apply immediately
- [ ] Module registration:
  - [ ] Add pipeline creation function `create_bg_gradient_pipeline()` in a new `oriterm/src/gpu/pipeline/bg_gradient.rs` submodule. Follow the same pattern as `pipeline/image.rs` and `pipeline/ui_rect.rs`. Add `pub(super) mod bg_gradient;` and re-export from `pipeline/mod.rs`.
  - [ ] Add `bg_gradient_pipeline: RenderPipeline` field to `GpuPipelines` in `oriterm/src/gpu/pipelines.rs`
  - [ ] Wire pipeline creation in `GpuPipelines::new()`
  - [ ] Create gradient uniform buffer (or extend existing uniform buffer) -- must fit 2 colors (8 floats), angle (1 float), opacity (1 float). Evaluate: separate bind group at group 1 is cleaner than extending the 16-byte screen uniform buffer

**Tests:**
- [ ] `linear_gradient_180deg_varies_vertically`: 180 deg angle → interpolated color at top of window equals `color_a`, bottom equals `color_b` (pure uniform computation, no GPU needed)
- [ ] `linear_gradient_0deg_varies_vertically_reversed`: 0 deg angle → top equals `color_b`, bottom equals `color_a` (inverse of 180 deg)
- [ ] `linear_gradient_90deg_varies_horizontally`: 90 deg angle → left side equals `color_a`, right side equals `color_b`; pixel at center-top and center-bottom are identical
- [ ] `radial_gradient_center_is_color_a`: center pixel of window equals `color_a` for radial mode
- [ ] `radial_gradient_edge_is_color_b`: corner pixels of window equal `color_b` for radial mode
- [ ] `effective_gradient_opacity_clamped`: opacity -0.1 clamps to 0.0; opacity 1.5 clamps to 1.0; NaN defaults to 1.0
- [ ] `gradient_none_skips_draw_call`: `PreparedFrame` with `background_gradient = None` skips the gradient draw call entirely
- [ ] `gradient_colors_fewer_than_two_falls_back`: `gradient_colors = ["#1e1e2e"]` → gradient falls back to `None` (no render); a warning is logged (test via a test-friendly log capture or by asserting `PreparedFrame` has no gradient pipeline scheduled)
- [ ] `gradient_colors_empty_falls_back`: empty `gradient_colors` → same fallback behavior as single-entry
- [ ] Config parsing: all `GradientType` variants (`"none"`, `"linear"`, `"radial"`) deserialize correctly from TOML strings
- [ ] Config hot-reload: changing gradient type applies without restart (integration behavior -- note manual verification required; may lack automated test infrastructure at this stage)


---

## 24.8 Window Backdrop Effects

Platform-specific compositor effects: Acrylic/Mica on Windows, blur on macOS/Linux. **Partially implemented**: basic acrylic/vibrancy/blur already works via `transparency.rs` using the `window-vibrancy` crate. `WindowConfig` already has `opacity: f32` and `blur: bool`. This subsection adds fine-grained backdrop type selection beyond the existing boolean toggle.

**File:** `oriterm/src/gpu/transparency.rs` (existing -- already implements acrylic/vibrancy/blur), `oriterm/src/window/mod.rs` (window creation -- three `apply_transparency()` call sites at lines ~91, ~128, ~272), `oriterm/src/config/mod.rs` (`WindowConfig`)

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
- [ ] Refactor `transparency.rs::apply_transparency()` to accept `Backdrop` enum instead of `bool blur`. Update all three callers in `oriterm/src/window/mod.rs`:
  - [ ] `TermWindow::new()` at line ~91: replace `apply_transparency(&window, config.opacity, true, DEFAULT_BLUR_TINT)` guard with `Backdrop`-based call
  - [ ] `TermWindow::from_window()` at line ~128: same update as above
  - [ ] `TermWindow::set_transparency()` method at line ~272 (also accepts `blur: bool`): update signature to `set_transparency(opacity: f32, backdrop: Backdrop)` and update internal call. Update both callers: `app/event_loop.rs` (passes `self.config.window.blur`) and `app/config_reload/mod.rs` (passes `blur`) — both must now read `config.window.backdrop` and pass a `Backdrop` value
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
- [ ] Config mapping to `apply_transparency()` (after refactor to `Backdrop` enum):
  - [ ] `backdrop = "none"` -- `apply_transparency(window, opacity, Backdrop::None, bg)`
  - [ ] `backdrop = "blur"` -- `apply_transparency(window, opacity, Backdrop::Blur, bg)` (current `blur=true` behavior, now explicit)
  - [ ] `backdrop = "acrylic"` -- `apply_transparency(window, opacity, Backdrop::Acrylic, bg)` routes to `window_vibrancy::apply_acrylic()` inside `transparency.rs`
  - [ ] `backdrop = "mica"` -- `apply_transparency(window, opacity, Backdrop::Mica, bg)` routes to Windows 11 DWM API inside `transparency.rs`
  - [ ] `backdrop = "auto"` -- `apply_transparency(window, opacity, Backdrop::Auto, bg)` selects platform-appropriate effect inside `transparency.rs`
- [ ] Interaction with other features:
  - [ ] Backdrop visible only when `window.opacity < 1.0`
  - [ ] Background gradient renders on top of backdrop effect
  - [ ] Background image renders on top of backdrop effect
  - [ ] Cell backgrounds render on top of all of the above
- [ ] Error handling (graceful fallback):
  - [ ] Mica on Windows 10: log warning, fall back to Acrylic
  - [ ] Acrylic on Windows without DWM composition: log warning, fall back to `none`
  - [ ] Linux without compositor: `window.set_blur(true)` may silently fail -- detect and log

**Tests:** (in `oriterm/src/config/tests.rs` -- pure serde/config tests, no GPU needed)
- [ ] `backdrop_variants_deserialize`: all `Backdrop` variants (`"none"`, `"blur"`, `"acrylic"`, `"mica"`, `"auto"`) deserialize correctly from TOML string values
- [ ] `backdrop_unknown_variant_deserializes_to_error`: an unrecognized string like `"frosted"` returns a serde error (not a panic or silent fallback)
- [ ] `backdrop_default_is_auto`: `WindowConfig::default().backdrop == Backdrop::Auto`
- [ ] `backdrop_none_disables_effect`: resolved `apply_transparency()` call with `Backdrop::None` does not call any `window_vibrancy` API (unit test by mocking the apply call or inspecting the resolved `Backdrop` from config)
- [ ] `backdrop_compat_blur_true_no_backdrop_field`: TOML with `blur = true` and no `backdrop` field → resolved backdrop equals `Backdrop::Blur` (backwards compatibility mapping)
- [ ] `backdrop_compat_blur_false_no_backdrop_field`: TOML with `blur = false` and no `backdrop` field → resolved backdrop equals `Backdrop::None`
- [ ] `backdrop_overrides_blur_when_both_present`: TOML with `blur = true` AND `backdrop = "mica"` → resolved backdrop equals `Backdrop::Mica` (`backdrop` field wins)
- [ ] `backdrop_auto_resolves_platform_appropriate`: `Backdrop::Auto` resolves to `Mica` on Windows 11, `Acrylic` on Windows 10, `Blur` on macOS/Linux -- test the resolution function with platform mocking or by calling `resolve_backdrop(Backdrop::Auto, platform)` where `platform` is injected
- [ ] `mica_falls_back_to_acrylic_on_windows_10`: `apply_transparency()` with `Backdrop::Mica` on a simulated Windows 10 context logs a warning and falls back to Acrylic (test the fallback path in `transparency.rs` logic, not the OS call)


---

## 24.9 Scrollable Menus

Add max-height constraint and scroll support to `MenuWidget` so long menus (e.g., 50+ theme entries) do not overflow the window.


**STATUS NOTE:** The core scroll infrastructure is already substantially implemented. The module has been split (`menu/mod.rs` 444 lines + `menu/widget_impl.rs` 454 lines + `menu/tests.rs` 780 lines). `max_height`, `scroll_offset`, `scroll_by()`, `ensure_visible()`, `entry_at_y()`, `total_height()`, `visible_height()`, `max_scroll()`, mouse-wheel scroll, scrollbar drag, and keyboard arrow auto-scroll are all present. What remains is PageUp/PageDown/Home/End keyboard bindings and missing test coverage.

**File:** `oriterm_ui/src/widgets/menu/mod.rs` (444 lines -- scroll helpers, layout, struct), `oriterm_ui/src/widgets/menu/widget_impl.rs` (454 lines -- event handling, draw, scrollbar), `oriterm_ui/src/widgets/menu/tests.rs` (780 lines -- existing scroll tests), `oriterm_ui/src/action/keymap_action/mod.rs` (actions), `oriterm_ui/src/action/keymap/mod.rs` (bindings)

**Already implemented (do not re-implement):**
- [x] `max_height: Option<f32>` on `MenuStyle` with `scrollbar: ScrollbarStyle`
- [x] `scroll_offset: f32` and `scrollbar_state: MenuScrollbarState` on `MenuWidget`
- [x] `total_height()`, `visible_height()`, `max_scroll()`, `is_scrollable()`, `scroll_by()` helpers
- [x] `entry_top_y(index)` computes Y offset of entry in content coordinates
- [x] `ensure_visible(index)` auto-scrolls to keep hovered entry in view (called from `navigate_keyboard()`)
- [x] `entry_at_y(y)` hit-tests accounting for `scroll_offset`
- [x] Mouse-wheel scroll: `InputEvent::Scroll { delta }` handled in `widget_impl.rs`, converts `Lines` and `Pixels` deltas via `scroll_by()`, clamped via `max_scroll()`
- [x] Scrollbar drag and track-click handled via `ScrubController` and `handle_drag_start/update/end()`
- [x] Layout clips height to `max_height` when set
- [x] Scroll position resets to 0.0 in `MenuWidget::new()`

**Remaining work:**

- [ ] **PageUp/PageDown/Home/End keyboard navigation** -- these keys are not yet bound for the Menu context:
  - [ ] Add `NavigatePageUp`, `NavigatePageDown`, `NavigateFirst`, `NavigateLast` action variants to the `actions!(widget, [...])` macro in `oriterm_ui/src/action/keymap_action/mod.rs`. The existing set is `Activate, NavigateUp, NavigateDown, Confirm, Dismiss, FocusNext, FocusPrev, IncrementValue, DecrementValue, ValueToMin, ValueToMax`
  - [ ] Bind `PageUp` → `NavigatePageUp`, `PageDown` → `NavigatePageDown`, `Home` → `NavigateFirst`, `End` → `NavigateLast` for `"Menu"` and `"Dropdown"` contexts in `push_list_bindings()` in `oriterm_ui/src/action/keymap/mod.rs`
  - [ ] Handle `NavigatePageUp` in `MenuWidget::handle_keymap_action()` in `widget_impl.rs`: scroll up by `visible_height()` via `scroll_by(-self.visible_height())`, then update `hovered` via `self.entry_at_y(self.style.padding_y)` to select the entry now at the top of the visible area. If `entry_at_y` returns `None` (all entries are non-clickable), leave `self.hovered` as `None`.
  - [ ] Handle `NavigatePageDown` in `handle_keymap_action()`: scroll down by `visible_height()` via `scroll_by(self.visible_height())`, then update `hovered` via `self.entry_at_y(self.style.padding_y)` to select the entry now at the top of the visible area. If `entry_at_y` returns `None`, leave `self.hovered` as `None`.
  - [ ] Handle `NavigateFirst` in `handle_keymap_action()`: set `self.scroll_offset = 0.0`, then find the first clickable entry with `self.entries.iter().position(MenuEntry::is_clickable)` and set `self.hovered = Some(idx)`
  - [ ] Handle `NavigateLast` in `handle_keymap_action()`: set `self.scroll_offset = self.max_scroll()`, then find the last clickable entry with `self.entries.iter().rposition(MenuEntry::is_clickable)` and set `self.hovered = Some(idx)`
  - [ ] Note: `ValueToMin`/`ValueToMax` are already bound to `Home`/`End` for `"Slider"` — the new `NavigateFirst`/`NavigateLast` are scoped to `"Menu"` and `"Dropdown"` so they will not conflict
  - [ ] **FILE SIZE NOTE:** `widget_impl.rs` is at 454 lines. Adding 4 new match arms to `handle_keymap_action()` adds ~20 lines → ~474 lines total. Safe under 500. The `handle_keymap_action()` function body will reach ~48 lines with 4 new arms — at the 50-line limit. If the arms grow longer, extract a `handle_page_navigation()` helper before hitting 50 lines.

**Tests (in `menu/tests.rs`):**
- [ ] `no_max_height_visible_equals_total`: menu with 5 entries and no `max_height` → `visible_height()` equals `total_height()` (no clipping)
- [ ] `max_height_clips_layout_height`: menu with 50 entries and `max_height = 300.0` → `layout()` returns height 300.0, not `50 * item_height`
- [x] Mouse wheel scroll adjusts `scroll_offset`, clamped to valid range (existing `scroll_wheel_over_scrollbar_keeps_hover_clear` test covers partial behavior; add direct test of `scroll_offset` after wheel event)
- [ ] `scroll_by_clamps_negative`: `scroll_by(-1000.0)` leaves `scroll_offset == 0.0`
- [ ] `scroll_by_clamps_above_max`: `scroll_by(1000.0)` leaves `scroll_offset == max_scroll()`
- [x] Keyboard navigate to entry beyond visible area auto-scrolls to reveal it (`navigate_keyboard` calls `ensure_visible` -- covered by existing tests for `scrollbar_thumb_drag_updates_offset` and scroll behavior)
- [ ] `navigate_first_scrolls_to_top_and_hovers_first_clickable`: `NavigateFirst` → `scroll_offset == 0.0` and `hovered == Some(index_of_first_clickable_entry)`
- [ ] `navigate_last_scrolls_to_bottom_and_hovers_last_clickable`: `NavigateLast` → `scroll_offset == max_scroll()` and `hovered == Some(index_of_last_clickable_entry)`
- [ ] `navigate_page_down_scrolls_by_visible_height`: `NavigatePageDown` → `scroll_offset` increases by `visible_height()` (clamped to `max_scroll()`), and `hovered` is updated to an entry in the new visible area
- [ ] `navigate_page_up_scrolls_by_visible_height`: `NavigatePageUp` → `scroll_offset` decreases by `visible_height()` (clamped to 0.0), and `hovered` is updated to an entry in the new visible area
- [ ] `navigate_first_on_all_separators_menu_does_not_panic`: menu with only separator entries → `NavigateFirst` leaves `hovered == None` and does not panic (no clickable entry found via `iter().position(is_clickable)`)
- [ ] `navigate_last_on_all_separators_menu_does_not_panic`: menu with only separator entries → `NavigateLast` leaves `hovered == None` and does not panic
- [ ] `navigate_first_skips_leading_separators`: menu with 2 separators then clickable entries → `NavigateFirst` sets `hovered` to the first clickable entry (index 2), not the first entry (index 0)
- [ ] `navigate_last_skips_trailing_separators`: menu with clickable entries then 2 trailing separators → `NavigateLast` sets `hovered` to the last clickable entry, not the last entry


---

## 24.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

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
- [ ] HIDDEN cells (SGR 8) are not revealed by minimum contrast at any setting
- [ ] HiDPI displays render crisp text at correct scale
- [ ] Moving between monitors with different DPI works
- [ ] Font zoom (Ctrl+=/Ctrl+-/Ctrl+0) works correctly at all DPI scales
- [ ] Vector icons (close x, +, chevron, minimize, maximize, restore, window close) render with smooth anti-aliasing at all DPI scales
- [ ] No jagged staircase artifacts on diagonal lines in any icon
- [ ] Dropdown chevron icons render with smooth anti-aliasing (same as tab bar)
- [ ] Background images render behind terminal content
- [ ] Background gradients render behind terminal content
- [ ] Backdrop effects (blur/acrylic/mica) apply on supported platforms
- [ ] Scrollable menus handle 50+ entries without window overflow
- [ ] All features configurable and hot-reloadable
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` pass
- [ ] No new `#[allow(clippy)]` without `reason`

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** Terminal feels visually polished at first launch -- cursor blinks, text is readable, HiDPI is crisp, icons are smooth, scrolling works, and all features are configurable and hot-reloadable.
