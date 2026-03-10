---
section: "07"
title: "GPU Multi-Window Rendering"
status: complete
goal: "Per-window GPU rendering with shared device/pipelines and a lighter pipeline for dialog windows"
inspired_by:
  - "Chromium Layer compositor per WindowTreeHost (ui/aura/window_tree_host.h)"
  - "ori_term existing shared GpuState + per-window WindowRenderer pattern"
depends_on: ["01"]
sections:
  - id: "07.1"
    title: "GPU Resource Sharing Audit"
    status: complete
  - id: "07.2"
    title: "Dialog Window Renderer"
    status: complete
  - id: "07.3"
    title: "Multi-Window Frame Scheduling"
    status: complete
  - id: "07.4"
    title: "Completion Checklist"
    status: complete
---

# Section 07: GPU Multi-Window Rendering

**Status:** Not Started
**Goal:** Dialog windows have their own GPU rendering pipeline — lighter than terminal windows (no grid cell rendering, no terminal atlas). All windows share the GPU device, queue, and compiled pipelines. Frame scheduling handles multiple dirty windows without dropped frames.

**Context:** ori_term already has a good foundation for multi-window GPU rendering. `GpuState` (device + queue) and `GpuPipelines` (shader pipelines, bind group layouts) are shared. Each `WindowContext` owns a `WindowRenderer` with per-window atlases, instance buffers, and bind groups. The `about_to_wait` handler already iterates dirty windows and renders each independently.

The gap is that `WindowRenderer` is terminal-focused: it has grid instance buffers, terminal font atlases, cursor buffers, etc. Dialog windows only need UI rendering (rectangles, text, buttons, form controls). Creating a full `WindowRenderer` for each dialog wastes GPU memory.

**Reference implementations:**
- **Chromium** `ui/aura/window_tree_host.h`: Each root has its own compositor. The compositor manages layers (textures) and composites them into the window's surface.
- **ori_term**: Existing `WindowRenderer` in `oriterm/src/gpu/window_renderer/mod.rs` is the per-window renderer.

**Depends on:** Section 01 (WindowManager tracks window kinds for renderer selection).

---

## 07.1 GPU Resource Sharing Audit

**File(s):** `oriterm/src/gpu/mod.rs`, `oriterm/src/gpu/state/mod.rs`

Verify and document the current sharing model. No code changes expected — this is validation.

- [ ] Confirm `GpuState` (device + queue) is created once and shared across all windows
- [ ] Confirm `GpuPipelines` (shader modules, pipeline layouts, bind group layouts) are shared
- [ ] Confirm `WindowRenderer::new()` can be called multiple times with the shared state
- [ ] Document the GPU resource ownership model:
  ```
  Shared (1 per app):        Per window (1 per OS window):
  ├── wgpu::Device           ├── wgpu::Surface
  ├── wgpu::Queue            ├── SurfaceConfiguration
  ├── GpuPipelines           ├── WindowRenderer
  │   ├── BindGroupLayouts   │   ├── UniformBuffer + BindGroup
  │   ├── RenderPipelines    │   ├── GlyphAtlas (mono)
  │   └── ShaderModules      │   ├── GlyphAtlas (subpixel)
  └──                        │   ├── GlyphAtlas (color/emoji)
                             │   ├── Instance buffers (bg, fg, overlay, ui, cursor, image)
                             │   ├── FontCollection (terminal mono)
                             │   ├── ui_font_collection (proportional sans-serif, optional)
                             │   ├── IconCache
                             │   ├── ImageTextureCache
                             │   └── PreparedFrame (reusable scratch)
                             └──
  ```

- [ ] Identify which WindowRenderer components are unnecessary for dialog windows:
  - NOT needed: terminal grid buffers (`bg_buffer`, `fg_buffer`, `subpixel_fg_buffer`, `color_fg_buffer`, `cursor_buffer`)
  - NOT needed: terminal font collection (`font_collection` with mono shaping, hinting, ShapingScratch)
  - NOT needed: image rendering (`ImageTextureCache`, `image_instance_buffer`, `image_instance_data`)
  - NOT needed: grid-specific PreparedFrame fields (backgrounds, glyphs, cursors)
  - NEEDED: UI instance buffers (`ui_rect_buffer`, `ui_fg_buffer`, `ui_subpixel_fg_buffer`, `ui_color_fg_buffer`)
  - NEEDED: overlay instance buffers (`overlay_rect_buffer`, etc.) — if using OverlayManager inside dialog
  - NEEDED: `ui_font_collection` (proportional sans-serif for form labels/text)
  - NEEDED: `GlyphAtlas` (mono) for UI text rendering, `GlyphAtlas` (color) for emoji in UI
  - NEEDED: `UniformBuffer`, atlas bind groups, `IconCache`
  - Note: `PaneRenderCache` lives on `WindowContext` (not `WindowRenderer`), so it is not relevant here

---

## 07.2 Dialog Window Renderer

**File(s):** If approach (a): `oriterm/src/gpu/window_renderer/ui_only.rs` (new, for UiOnly constructor and render path). If approach (b): `oriterm/src/gpu/dialog_renderer/mod.rs` (new).

**WARNING: `window_renderer/mod.rs` is 459 lines.** Adding `RendererMode` + conditional buffer creation risks exceeding 500 lines. For approach (a), put `new_ui_only()` and `render_ui_only()` in a new `window_renderer/ui_only.rs` submodule.

Two approaches. Evaluate which is better:

**(a) Parameterized WindowRenderer** (recommended if WindowRenderer isn't too bloated):
- Add a `RendererMode` enum: `Terminal` (full) vs. `UiOnly` (dialog)
- In `UiOnly` mode, skip creating terminal-specific buffers
- Pro: one type, shared draw code
- Con: conditional logic in renderer

**(b) Separate DialogRenderer type**:
- New type with only UI rendering capabilities
- Pro: clean separation, no wasted memory
- Con: duplicated draw infrastructure

- [ ] Evaluate WindowRenderer size — how much memory do the terminal-specific buffers waste?
  - If < 1MB per dialog window: use approach (a), not worth the complexity
  - If > 5MB per dialog window: use approach (b)

- [ ] Implement chosen approach
  ```rust
  // Approach (a): Parameterized mode
  pub enum RendererMode {
      /// Full terminal renderer with grid, cursor, etc.
      Terminal,
      /// UI-only renderer for dialogs (rects, text, icons).
      UiOnly,
  }

  impl WindowRenderer {
      pub fn new(mode: RendererMode, gpu: &GpuState, pipelines: &GpuPipelines,
                 /* ... */) -> Self {
          match mode {
              RendererMode::Terminal => {
                  // Create all buffers including grid
              }
              RendererMode::UiOnly => {
                  // Skip grid buffers, cursor buffer, terminal atlas
                  // Only create UI rect/text buffers
              }
          }
      }
  }
  ```

- [ ] Add `WindowRenderer::new_ui_only(gpu, pipelines, ui_font_collection)` constructor (separate constructor is cleaner than making `font_collection` `Option` -- avoids Option proliferation in existing code)
  - Skips: grid instance buffers (bg, fg, subpixel_fg, color_fg, cursor), terminal FontCollection + ShapingScratch, ImageTextureCache, image instance buffer
  - Creates: UI instance buffers, UniformBuffer, GlyphAtlas (mono + color), IconCache, ui_font_collection
  - All render methods that touch grid buffers must check `self.mode` and skip when `UiOnly`

- [ ] Add `render_ui_only()` method to WindowRenderer that runs only the UI render pass (the existing render pipeline has bg/fg/subpixel/color/overlay/ui passes -- UiOnly mode only needs the ui_pass and possibly overlay_pass)
  - Or: parametrize the existing `render()` method to skip grid passes based on `self.mode`

- [ ] Dialog render pass: draw chrome, draw dialog content, present
  ```rust
  impl DialogWindowContext {
      pub fn render(&mut self, gpu: &GpuState, pipelines: &GpuPipelines) {
          let renderer = self.renderer.as_mut().unwrap();
          let frame = self.surface.get_current_texture().ok()?;
          let view = frame.texture.create_view(&Default::default());

          // Build draw list from dialog content
          let mut draw_list = DrawList::new();
          self.chrome.draw(&mut draw_list, viewport);
          self.content.draw(&mut draw_list, content_area);

          // Convert draw list to GPU instances
          renderer.prepare_ui_instances(&draw_list);

          // Render pass
          renderer.render_ui_only(&view, gpu, pipelines);

          frame.present();
      }
  }
  ```

- [ ] UI font for dialogs: share the `ui_font_collection` across windows (or create per-window)
  - If font collection is cheap (just font data + shaper), create per-window
  - If expensive (large atlas), share via `Arc<FontCollection>`

- [ ] **Atlas sharing clarification**: Glyph atlases (GPU textures) are per-window and cannot be shared -- they are bound to a specific `wgpu::BindGroup` and may contain DPI-specific rasterizations. Font sets (discovery metadata) are cloned from `self.ui_font_set` per window (cheap clone, existing pattern).

- [ ] Evaluate whether `PreparedFrame` is needed for UiOnly mode -- if not, skip allocation (it is a significant per-window allocation)

---

## 07.3 Multi-Window Frame Scheduling

**File(s):** `oriterm/src/app/event_loop.rs`, `oriterm/src/app/redraw/mod.rs`

Ensure multiple windows can render in the same frame without issues.

- [ ] Verify that `about_to_wait` handles both terminal and dialog dirty windows
  ```rust
  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
      // Phase 1: Render dirty terminal windows (existing)
      // Phase 2: Render dirty dialog windows (new)
      // Both use the same GPU device/queue — wgpu handles this fine
      // as long as we don't submit conflicting commands
  }
  ```

- [ ] Verify wgpu can handle multiple surfaces presenting in the same frame
  - Each surface has its own swap chain
  - Multiple `queue.submit()` calls in one frame are fine
  - Confirm no surface contention

- [ ] Handle DPI changes for dialog windows
  - Dialog on a different monitor than parent may have different DPI
  - `ScaleFactorChanged` event triggers atlas rebuild for the dialog's renderer
  - Font sizes recalculated per-window DPI

- [ ] Dialog window resize handling
  - Settings dialog: resizable (within min/max bounds)
  - Confirmation dialog: fixed size
  - Surface configuration update on resize (same as main window)

---

## 07.4 Completion Checklist

- [ ] GPU sharing model documented and verified
- [ ] Dialog windows create lighter-weight renderers (no grid buffers)
- [ ] Dialog render pass works: chrome + content → GPU → present
- [ ] Multiple windows render in same frame without artifacts
- [ ] DPI handling correct for dialog windows (including multi-monitor)
- [ ] Dialog resize triggers surface reconfiguration
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green

**Exit Criteria:** A settings dialog window renders correctly with custom chrome, form controls, and smooth text. GPU memory usage for a dialog is significantly less than a full terminal window. Opening 5 dialog windows doesn't cause frame drops in the main terminal window.
