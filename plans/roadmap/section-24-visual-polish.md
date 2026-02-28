---
section: 24
title: Visual Polish
status: not-started
tier: 6
goal: Cursor blinking, hide-while-typing, minimum contrast, HiDPI, background images, gradients, backdrop effects
sections:
  - id: "24.1"
    title: Cursor Blinking
    status: not-started
  - id: "24.2"
    title: Hide Cursor While Typing
    status: not-started
  - id: "24.3"
    title: Minimum Contrast
    status: not-started
  - id: "24.4"
    title: HiDPI & Display Scaling
    status: not-started
  - id: "24.6"
    title: Background Images
    status: not-started
  - id: "24.7"
    title: Background Gradients
    status: not-started
  - id: "24.8"
    title: Window Backdrop Effects
    status: not-started
  - id: "24.9"
    title: Section Completion
    status: not-started
---

# Section 24: Visual Polish

**Status:** Not Started
**Goal:** Small visual features that collectively create a polished, modern feel. Each is low-to-medium effort but highly visible. These are the details people notice in the first 5 minutes. Missing cursor blink, broken HiDPI, or unreadable colors are dealbreakers.

**Crate:** `oriterm` (rendering + app layer)
**Dependencies:** `image` (for background images), existing wgpu pipeline

---

## 24.1 Cursor Blinking

Toggle cursor visibility on a timer. Cursor shapes (Block/Underline/Beam) render correctly but no animation logic exists -- cursor is always visible.

**File:** `oriterm/src/app.rs` (blink state), `oriterm/src/gpu/renderer.rs` (render skip)

**Reference:** `_old/src/app.rs` (cursor rendering), `_old/src/gpu/renderer.rs` (FrameParams)

- [ ] Blink state tracking:
  - [ ] Add `cursor_visible: bool` and `cursor_blink_deadline: Instant` to `App`
  - [ ] Blink interval: 530ms on / 530ms off (configurable)
  - [ ] Toggle `cursor_visible` when deadline elapses
- [ ] Only blink when DECSCUSR sets a blinking style:
  - [ ] DECSCUSR values 1 (blinking block), 3 (blinking underline), 5 (blinking bar)
  - [ ] Even values (2, 4, 6) = steady -- no blink
  - [ ] Default (0) = implementation-defined -- follow config
  - [ ] Store `cursor_blinking: bool` per tab alongside `cursor_shape`
- [ ] Reset blink to visible state on:
  - [ ] Any keypress (reset deadline to now + interval)
  - [ ] PTY output that moves cursor
  - [ ] Mouse click in grid area
- [ ] Timer implementation using winit:
  - [ ] Use `ControlFlow::WaitUntil(next_blink_deadline)` when cursor is blinking
  - [ ] In `about_to_wait()` handler: check if deadline elapsed, toggle visibility, request redraw, set next deadline
  - [ ] When no blink needed: revert to `ControlFlow::Wait`
- [ ] Renderer integration:
  - [ ] `FrameParams` gains `cursor_visible: bool`
  - [ ] `build_grid_instances()` skips cursor rendering when `!cursor_visible`
- [ ] Focus handling:
  - [ ] When window loses focus: show steady cursor (no blink), or hide cursor
  - [ ] When window gains focus: restart blink timer
  - [ ] Unfocused window: render cursor as hollow block (outline only)
- [ ] Config: `terminal.cursor_blink = true | false` (default: true)
- [ ] Config: `terminal.cursor_blink_interval = 530` (ms)

**Tests:**
- [ ] Blink state toggles after interval elapses
- [ ] Keypress resets blink to visible
- [ ] Even DECSCUSR values disable blinking
- [ ] Odd DECSCUSR values enable blinking
- [ ] Focus loss stops blinking, shows steady cursor

---

## 24.2 Hide Cursor While Typing

Mouse cursor disappears when typing, reappears on mouse move.

**File:** `oriterm/src/app.rs` (input handling)

- [ ] On `KeyboardInput` event (non-modifier keys): `window.set_cursor_visible(false)`
  - [ ] Track `mouse_cursor_hidden: bool` on `App`
  - [ ] Only hide if mouse is over the grid area (not tab bar or resize border)
- [ ] On `CursorMoved` event: `window.set_cursor_visible(true)`
  - [ ] Reset `mouse_cursor_hidden = false`
- [ ] Don't hide during mouse reporting mode (application is using the mouse):
  - [ ] Check `TermMode::MOUSE_REPORT | MOUSE_MOTION | MOUSE_ALL`
  - [ ] If any mouse mode is on, don't hide
- [ ] Config: `behavior.hide_mouse_when_typing = true | false` (default: true)

**Tests:**
- [ ] Keypress hides mouse cursor
- [ ] Mouse move restores mouse cursor
- [ ] Mouse reporting mode prevents hiding
- [ ] Config false disables the feature entirely

---

## 24.3 Minimum Contrast

Ensure text is always readable regardless of color scheme. WCAG 2.0 contrast enforcement in the GPU shader.

**File:** `oriterm/src/gpu/pipeline.rs` (WGSL shader), `oriterm/src/config.rs` (config), `oriterm/src/gpu/renderer.rs` (uniform)

**Reference:** Ghostty's minimum contrast feature, iTerm2's minimum contrast slider

- [ ] Config: `colors.minimum_contrast` (range 1.0 disabled to 21.0 maximum, default 1.0)
  - [ ] `effective_minimum_contrast()` clamps value
- [ ] WCAG 2.0 implementation in WGSL shader:
  - [ ] `luminance()`: ITU-R BT.709 relative luminance from linear RGB
  - [ ] `contrast_ratio()`: WCAG formula `(L1 + 0.05) / (L2 + 0.05)`
  - [ ] `contrasted_color()`: adjusts fg toward white or black to meet ratio
    - [ ] Binary search for minimum alpha mix that achieves target ratio
    - [ ] Tries white first (for dark backgrounds), then black (for light)
    - [ ] Picks whichever achieves better contrast
- [ ] Per-vertex enforcement: contrast applied in vertex shader
  ```wgsl
  out.fg_color = contrasted_color(uniforms.min_contrast, input.fg_color, input.bg_color);
  ```
- [ ] Uniform buffer passes `min_contrast` from config to shader
- [ ] Hot-reload: changing `minimum_contrast` in config takes effect immediately

**Tests:**
- [ ] White on black at minimum_contrast 1.0 passes through unchanged
- [ ] Dark gray on black at minimum_contrast 4.5 adjusts fg to lighter color
- [ ] Light gray on white at minimum_contrast 4.5 adjusts fg to darker color

---

## 24.4 HiDPI & Display Scaling

Render correctly on high-DPI displays and handle multi-monitor DPI transitions.

**File:** `oriterm/src/app.rs` (DPI tracking, WindowDrag), `oriterm/src/gpu/pipeline.rs` (sRGB)

- [ ] Track `scale_factor: f64` on `App` struct
  - [ ] Initial value: 1.0, updated on first window creation
- [ ] Handle `ScaleFactorChanged` event:
  - [ ] Update `self.scale_factor`
  - [ ] Re-rasterize fonts at `config.font.size * scale_factor`
  - [ ] Trigger font set rebuild (atlas clear + re-render)
  - [ ] Recalculate grid layout with new cell dimensions
- [ ] Font size scaling:
  - [ ] Font rasterized at `font_size * scale_factor`
  - [ ] Zoom operations account for scale factor
  - [ ] `reset_font_size()` resets to `config.font.size * scale_factor`
- [ ] Multi-monitor DPI handling:
  - [ ] Manual window drag replaces native `drag_window()` to prevent `WM_DPICHANGED` oscillation at per-monitor DPI boundaries
  - [ ] `WindowDrag` struct tracks screen-space cursor and window positions
  - [ ] Periodic scale factor check during drag
- [ ] sRGB-correct rendering pipeline:
  - [ ] GPU pipeline uses sRGB surface format for gamma-correct blending
  - [ ] Luminance-based alpha correction option (`AlphaBlending::LinearCorrected`)
  - [ ] Config: `colors.alpha_blending = "linear" | "linear_corrected"` (default: linear_corrected)

**Tests:**
- [ ] Scale factor change triggers font re-rasterization
- [ ] Grid dimensions recalculated after DPI change
- [ ] Window drag across monitors with different DPI works without oscillation

---

## 24.6 Background Images

Display a background image behind the terminal grid.

**File:** `oriterm/src/gpu/renderer.rs` (render pass), `oriterm/src/gpu/pipeline.rs` (shader), `oriterm/src/config.rs` (config)

- [ ] Config options:
  ```toml
  [window]
  background_image = "/path/to/image.png"
  background_image_opacity = 0.1
  background_image_position = "center"  # center | stretch | tile | fill
  ```
- [ ] Image loading:
  - [ ] Load at startup and on config reload (hot-reload)
  - [ ] Decode PNG/JPEG/BMP via `image` crate (add dependency)
  - [ ] Convert to RGBA8 texture for wgpu
  - [ ] Handle errors gracefully (missing file, corrupt image)
- [ ] GPU rendering:
  - [ ] Create a wgpu texture from the decoded image
  - [ ] Add a new render pass before cell backgrounds:
    - [ ] Full-screen quad with image texture
    - [ ] Apply `background_image_opacity` as alpha multiplier
  - [ ] Cell backgrounds blend over the image
  - [ ] Position/scale image according to `background_image_position`
- [ ] Position modes:
  - [ ] `center`: original size, centered, crop if larger than window
  - [ ] `stretch`: scale to fill window, may distort aspect ratio
  - [ ] `fill`: scale to fill, maintaining aspect ratio, crop excess
  - [ ] `tile`: repeat at original size
- [ ] Handle window resize: rescale/reposition image
- [ ] Memory: keep decoded texture in GPU memory, not system RAM

**Tests:**
- [ ] Image loads from valid path, returns error for missing path
- [ ] Position mode center computes correct UV coordinates
- [ ] Position mode fill maintains aspect ratio
- [ ] Opacity multiplier applied correctly in shader
- [ ] Config reload swaps background image without restart

---

## 24.7 Background Gradients

GPU-rendered gradient backgrounds as an alternative to solid colors or images.

**File:** `oriterm/src/gpu/renderer.rs` (render pass), `oriterm/src/gpu/pipeline.rs` (gradient shader)

**Reference:** WezTerm `background` config (gradient presets + custom)

- [ ] Config:
  ```toml
  [window]
  background_gradient = "none"  # "none", "linear", "radial"
  gradient_colors = ["#1e1e2e", "#313244"]  # start and end colors
  gradient_angle = 180  # degrees, for linear gradient (0 = top-to-bottom)
  gradient_opacity = 1.0  # 0.0-1.0, blended with background color
  ```
- [ ] Linear gradient:
  - [ ] Two-stop gradient from color A to color B
  - [ ] Angle configurable: 0° = top→bottom, 90° = left→right, etc.
  - [ ] WGSL shader: interpolate colors based on UV coordinates rotated by angle
- [ ] Radial gradient:
  - [ ] Center-to-edge gradient
  - [ ] Color A at center, color B at edges
  - [ ] WGSL shader: distance from center → lerp between colors
- [ ] Multi-stop gradients (stretch goal):
  - [ ] `gradient_colors = ["#1e1e2e", "#313244", "#45475a"]` — 3+ stops
  - [ ] Even distribution across gradient length
- [ ] Rendering:
  - [ ] Full-screen quad before cell backgrounds (same pass as background image)
  - [ ] If both gradient and image specified: gradient first, image on top with alpha
  - [ ] Cell backgrounds blend on top of gradient
- [ ] Interaction with transparency:
  - [ ] Gradient respects `window.opacity` — blended with compositor-provided background
  - [ ] `gradient_opacity` controls gradient's own alpha (independent of window opacity)
- [ ] Hot-reload: gradient config changes apply immediately
- [ ] **Tests:**
  - [ ] Linear gradient: pixel at top differs from pixel at bottom
  - [ ] Angle rotation: 90° gradient varies horizontally, not vertically
  - [ ] Gradient opacity: alpha applied correctly
  - [ ] Config "none": no gradient rendered

---

## 24.8 Window Backdrop Effects

Platform-specific compositor effects: Acrylic/Mica on Windows, blur on macOS/Linux.

**File:** `oriterm/src/app.rs` (window creation), `oriterm/src/config.rs` (config)

**Reference:** WezTerm `win32_system_backdrop`, Ghostty `background-blur-radius`, `window-vibrancy` crate

- [ ] Config:
  ```toml
  [window]
  backdrop = "none"  # "none", "blur", "acrylic", "mica", "auto"
  ```
- [ ] Windows backdrop effects (Win32):
  - [ ] `blur` — `DWM_SYSTEMBACKDROP_TYPE::DWMSBT_TRANSIENTWINDOW` (standard blur)
  - [ ] `acrylic` — `DWM_SYSTEMBACKDROP_TYPE::DWMSBT_TRANSIENTWINDOW` with tint color
  - [ ] `mica` — `DWM_SYSTEMBACKDROP_TYPE::DWMSBT_MAINWINDOW` (Windows 11 only)
  - [ ] `auto` — Mica on Windows 11, Acrylic on Windows 10
  - [ ] Requires `window.opacity < 1.0` to see the effect through the window
  - [ ] Uses `window-vibrancy` crate (already a dependency)
- [ ] macOS backdrop effects:
  - [ ] `blur` — `NSVisualEffectView` with `NSVisualEffectBlendingMode::behindWindow`
  - [ ] Material selection: `.hudWindow` or `.sidebar` for tasteful blur
  - [ ] `window-vibrancy` crate handles the NSVisualEffectView setup
- [ ] Linux backdrop effects:
  - [ ] KDE: `_KDE_NET_WM_BLUR_BEHIND_REGION` X11 property
  - [ ] GNOME/Wayland: limited support — compositor-dependent
  - [ ] `blur`: best-effort, log warning if unsupported
- [ ] Interaction with other features:
  - [ ] Backdrop visible only when `window.opacity < 1.0`
  - [ ] Background gradient renders on top of backdrop effect
  - [ ] Background image renders on top of backdrop effect
  - [ ] Cell backgrounds render on top of all of the above
- [ ] **Tests:**
  - [ ] Config parsing: all backdrop variants
  - [ ] "none" disables backdrop
  - [ ] "auto" selects platform-appropriate effect

---

## 24.9 Section Completion

- [ ] All 24.1-24.8 items complete
- [ ] Cursor blinks at configured rate for blinking styles
- [ ] Cursor blink resets on keypress
- [ ] Mouse cursor hides when typing, reappears on move
- [ ] Minimum contrast enforces readable text (WCAG 2.0 in shader)
- [ ] HiDPI displays render crisp text at correct scale
- [ ] Moving between monitors with different DPI works
- [ ] Smooth scrolling feels natural with mouse wheel
- [ ] Background images render behind terminal content
- [ ] All features configurable and hot-reloadable

**Exit Criteria:** Terminal feels visually polished at first launch -- cursor blinks, text is readable, HiDPI works, scrolling is smooth.
