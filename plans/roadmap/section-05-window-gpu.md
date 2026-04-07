---
section: 5
title: Window + GPU Rendering
status: in-progress
reviewed: true
last_verified: "2026-04-06"
tier: 2
goal: Open a frameless window, initialize wgpu, render the terminal grid with a proper staged render pipeline — first visual milestone
third_party_review:
  status: none
  updated: null
sections:
  - id: "5.1"
    title: Render Pipeline Architecture
    status: complete
  - id: "5.2"
    title: winit Window Creation
    status: complete
  - id: "5.3"
    title: wgpu GpuState + Offscreen Render Targets
    status: complete
  - id: "5.4"
    title: WGSL Shaders + GPU Pipelines
    status: complete
  - id: "5.5"
    title: Uniform Buffer + Bind Groups
    status: complete
  - id: "5.6"
    title: Font Discovery + Rasterization
    status: complete
  - id: "5.7"
    title: Glyph Atlas
    status: complete
  - id: "5.8"
    title: "Extract Phase (CPU)"
    status: complete
  - id: "5.9"
    title: "Prepare Phase (CPU)"
    status: complete
  - id: "5.10"
    title: "Render Phase (GPU)"
    status: complete
  - id: "5.11"
    title: App Struct + Event Loop
    status: complete
  - id: "5.12"
    title: Basic Input + Cursor
    status: complete
  - id: "5.13"
    title: Render Pipeline Testing
    status: complete
  - id: "5.14"
    title: "Integration: Working Terminal"
    status: complete
  - id: "5.16"
    title: "GPU Device Lost Recovery — Core Engine"
    status: not-started
  - id: "5.17"
    title: "Recovery Correctness & Infrastructure"
    status: not-started
  - id: "5.18"
    title: "Recovery Integrations & Deferred Contracts"
    status: not-started
  - id: "5.15"
    title: Section Completion
    status: in-progress
---

# Section 05: Window + GPU Rendering

**Status:** In progress (5.1–5.14 complete; 5.16/5.17/5.18 not started — reviewed and ready to implement 2026-04-06; 5.15 gates the whole section)
**Goal:** The first visual milestone. Open a native frameless window, initialize wgpu (Vulkan/DX12 on Windows, Vulkan on Linux, Metal on macOS), and render the terminal grid through a **proper staged render pipeline** — not scattered GPU code. Every frame flows through: Extract -> Prepare -> Render. The CPU-side phases are pure functions, fully unit-testable without a GPU.

**Crate:** `oriterm` (binary)
**Dependencies:** `oriterm_core`, `winit`, `wgpu`, `swash`, `rustybuzz`, `window-vibrancy`, `dwrote` (Windows)
**Reference:** `_old/src/gpu/` (what NOT to do — scattered rendering with no pipeline), Bevy's staged render architecture, wgpu test suite patterns.

**Anti-pattern from prototype:** The old codebase had `render_tab_bar()`, `render_grid()`, `render_overlay()`, `render_settings()` as independent functions that each built their own instance buffers, managed their own state, and submitted their own draw calls. No shared pipeline, no separation between CPU and GPU work, no testability. This section builds it right.

> **Verification (2026-03-29):** Sub-items 5.1–5.14 PASS. 2084 tests passing, 0 ignored. Three-layer test strategy fully implemented. All four performance invariants enforced. Phase separation verified (zero `use wgpu` in extract/prepare). Implementation goes significantly beyond plan (incremental rendering, multi-atlas, subpixel rendering, image rendering, compositor, builtin glyphs, pane cache). 5.15 (Section Completion gate for the whole section) is held open until 5.16 (Core GPU Recovery), 5.17 (Recovery Correctness & Infrastructure), and 5.18 (Recovery Integrations & Deferred Contracts) all land. Hygiene notes: `gpu/atlas/mod.rs` was at 579 lines, since split to 457 (resolved 2026-03-31). `gpu/window_renderer/helpers.rs:549` is currently over the 500-line limit and is tracked as a 5.16 fix-along-the-way item (see 5.16.14).

---

## 5.1 Render Pipeline Architecture (verified 2026-03-29)

The organizing principle for all rendering. Every frame flows through these phases in order. No phase reaches back into a previous phase. No phase touches the GPU until the Render phase.

**File:** `oriterm/src/gpu/pipeline_stages.rs` (types + documentation)

### The Three Phases

```
┌─────────┐      ┌─────────┐      ┌──────────┐
│ EXTRACT  │ ──→  │ PREPARE │ ──→  │  RENDER  │
│  (CPU)   │      │  (CPU)  │      │  (GPU)   │
│          │      │         │      │          │
│ Lock     │      │ Build   │      │ Upload   │
│ Snapshot │      │ DrawList│      │ Draw     │
│ Unlock   │      │ Instance│      │ Present  │
│          │      │ Buffers │      │          │
└─────────┘      └─────────┘      └──────────┘
  testable         testable        integration
  (unit test)      (unit test)     (headless GPU)
```

- [x] **Phase 1: Extract** — Lock terminal state, snapshot to `FrameInput`, unlock.
  - [x] Input: `&FairMutex<Term<EventProxy>>`, widget state, cursor state
  - [x] Output: `FrameInput` (owned, no references to locked state)
  - [x] Duration: microseconds. Lock is released before any other work.
  - [x] **Pure data copy.** No GPU types, no rendering logic.

- [x] **Phase 2: Prepare** — Convert `FrameInput` into GPU-ready instance buffers.
  - [x] Input: `&FrameInput`, `&FontCollection`, `&GlyphAtlas` (for UV lookups)
  - [x] Output: `PreparedFrame` containing `InstanceWriter` buffers (bg + fg + overlay)
  - [x] **Pure CPU computation.** Produces `Vec<u8>` byte buffers — no wgpu types, no device, no queue.
  - [x] This is where cell → pixel position math, glyph lookup, color resolution, cursor building all happen.
  - [x] **Fully unit-testable**: given a `FrameInput`, assert the exact bytes in the instance buffers.

- [x] **Phase 3: Render** — Upload buffers to GPU, execute draw calls, present.
  - [x] Input: `&PreparedFrame`, `&GpuState`, target `&wgpu::TextureView` (surface OR offscreen)
  - [x] Output: pixels on screen (or in offscreen texture)
  - [x] This phase is thin — just GPU plumbing. All logic is in Prepare.
  - [x] Accepts any `TextureView` as target (not hardcoded to surface). Enables: tab previews, headless testing, thumbnails.

### Key Data Types

- [x] `FrameInput` — everything needed to build a frame, no references
  - [x] `cells: Vec<RenderableCell>` — visible cells (via `content: RenderableContent`)
  - [x] `cursor: Option<RenderableCursor>` — cursor state (via `content.cursor`)
  - [x] `viewport: (u32, u32)` — viewport size in pixels (via `ViewportSize` newtype)
  - [x] `cell_size: (f32, f32)` — cell dimensions (via `CellMetrics` newtype, includes baseline)
  - [x] `baseline: f32` — font baseline (inside `CellMetrics`)
  - [x] `palette: FramePalette` — resolved colors for this frame
  - [x] `selection: Option<SelectionRange>` — active selection bounds (placeholder type)
  - [x] `search_matches: Vec<SearchMatch>` — highlighted search results (placeholder type)
  - [x] No `Arc`, no `Mutex`, no references — pure owned data.

- [x] `PreparedFrame` — GPU-ready output of the Prepare phase
  - [x] `bg_instances: InstanceWriter` — background quad instances (field: `backgrounds`)
  - [x] `fg_instances: InstanceWriter` — foreground glyph instances (field: `glyphs`)
  - [x] `overlay_instances: InstanceWriter` — overlay instances (field: `cursors`)
  - [x] `viewport: (u32, u32)` — for uniform buffer update (sourced from FrameInput at render time)
  - [x] `clear_color: [f32; 4]` — background clear color (`[f64; 4]` to match wgpu clear API)
  - [x] No wgpu types. Just bytes.

### Pipeline Rules (enforced by type system)

- [x] Extract returns owned `FrameInput` — cannot hold locks across phases
- [x] Prepare takes `&FrameInput`, returns owned `PreparedFrame` — pure function
- [x] Render takes `&PreparedFrame` + GPU resources — the only phase that touches wgpu
- [x] No function crosses phase boundaries (no "prepare and also render" functions)

---

## 5.2 winit Window Creation (verified 2026-03-29)

**File:** `oriterm/src/window/mod.rs` (318 lines, under 500-line limit)

- [x] `TermWindow` struct (Chrome `WindowTreeHost` pattern — pure window wrapper, NO tabs/content)
  - [x] Fields:
    - `window: Arc<winit::window::Window>` — the winit window (Arc for wgpu surface)
    - `surface: wgpu::Surface<'static>` — wgpu rendering surface
    - `surface_config: wgpu::SurfaceConfiguration` — surface format, size, present mode
    - `size_px: (u32, u32)` — window size in physical pixels
    - `scale_factor: ScaleFactor` — DPI scale factor (oriterm_ui newtype, clamped)
    - `is_maximized: bool` — window maximized state
  - [x] `TermWindow::new(event_loop, config: &WindowConfig, gpu: &GpuState) -> Result<Self>`
    - [x] Window attributes: frameless (`decorations: false`), transparent, title "oriterm" (via `oriterm_ui::window::create_window`)
    - [x] Initial size: 1024×768 (from `WindowConfig::default()`)
    - [x] Create wgpu surface from window (via `GpuState::create_surface`)
    - [x] Configure surface: format, alpha mode (pre-multiplied for transparency)
    - [x] Store dimensions and scale factor
  - [x] `TermWindow::resize_surface(&mut self, width, height, gpu: &GpuState)`
    - [x] Update surface config with new size (min 1×1)
    - [x] `self.surface.configure(&gpu.device, &self.surface_config)`
  - [x] `TermWindow::request_redraw(&self)` — `self.window.request_redraw()`
  - [x] `TermWindow::scale_factor(&self) -> ScaleFactor`
  - [x] `TermWindow::size_px(&self) -> (u32, u32)`
  - [x] `TermWindow::update_scale_factor(&mut self, f64) -> bool` — DPI change handling
  - [x] `TermWindow::set_visible(&self, bool)` — show after first frame
  - [x] `TermWindow::has_surface_area(&self) -> bool` — skip render when minimized
  - [x] `TermWindow::window_id(&self) -> WindowId` — event routing
  - [x] `WindowCreateError` enum — `Window` + `Surface` variants with `Display`/`Error`/`From`
- [x] Window vibrancy (platform-specific):
  - [x] Windows: `window_vibrancy::apply_acrylic()` for translucent background (via `gpu::transparency`)
  - [x] Linux/macOS: compositor-dependent (via `gpu::transparency`, see Section 03)
  - [x] Fallback: opaque dark background if vibrancy not available (opacity >= 1.0 short-circuits)
- [x] Forward-looking IME setup (no-op until Section 8.3 wires handlers):
  - [x] `window.set_ime_allowed(true)` — enable IME input
  - [x] `window.set_ime_purpose(ImePurpose::Terminal)` — hint for IME engine

---

## 5.3 wgpu GpuState + Offscreen Render Targets (verified 2026-03-29)

**File:** `oriterm/src/gpu/state/mod.rs` (376 lines) + `gpu/state/helpers.rs` + `gpu/render_target/mod.rs` (229 lines). 19 unit tests.

- [x] `GpuState` struct
  - [x] Fields:
    - `instance: wgpu::Instance` — wgpu instance (Vulkan/DX12 on Windows, Vulkan on Linux, Metal on macOS)
    - `adapter: wgpu::Adapter` — selected GPU adapter (dropped after init, device/queue independent)
    - `device: wgpu::Device` — logical device
    - `queue: wgpu::Queue` — command queue
    - `surface_format: wgpu::TextureFormat` — negotiated format (plus `render_format` sRGB variant)
  - [x] `GpuState::new() -> Result<Self>`
    - [x] Create instance with Vulkan + DX12 + Metal backends (wgpu auto-selects best available)
    - [x] Request adapter (high performance preference)
    - [x] Request device with reasonable limits
    - [x] Determine surface format from adapter capabilities
  - [x] `GpuState::new_headless() -> Result<Self>`
    - [x] Same as `new()` but with `compatible_surface: None`
    - [x] Used for testing — no window or surface required
    - [x] Falls back to software rasterizer if no GPU available
  - [x] `GpuState::configure_surface(&self, surface: &wgpu::Surface, width: u32, height: u32) -> wgpu::SurfaceConfiguration`
    - [x] Select present mode: `Mailbox` preferred (low latency), `Fifo` fallback
    - [x] Alpha mode: `PreMultiplied` for transparency, `Opaque` fallback
    - [x] Return configuration
  - [x] Offscreen render targets:
    - [x] `create_render_target(width: u32, height: u32) -> RenderTarget`
    - [x] `RenderTarget` struct: `texture: wgpu::Texture`, `view: wgpu::TextureView`
    - [x] Same format as surface (`render_format`) so pipelines are reusable
    - [x] Used for: tab previews, headless test rendering, thumbnails
    - [x] `read_render_target(target: &RenderTarget) -> Vec<u8>` — read pixels back to CPU
      - [x] `buffer.slice(..).map_async(MapMode::Read, ...)` + `device.poll(PollType::wait_indefinitely())`
      - [x] Returns RGBA bytes — used by visual regression tests and thumbnail generation

---

## 5.4 WGSL Shaders + GPU Pipelines (verified 2026-03-29)

**File:** `oriterm/src/gpu/shaders/bg.wgsl`, `oriterm/src/gpu/shaders/fg.wgsl`, plus `subpixel_fg.wgsl`, `color_fg.wgsl`, `ui_rect.wgsl`, `image.wgsl`, `composite.wgsl` (beyond plan). `gpu/pipeline/mod.rs` + `gpu/pipelines.rs`.

### Shaders

- [x] Background vertex shader:
  - [x] Input: instance data (pos, size, uv, fg_color, bg_color, kind)
  - [x] Output: screen-space quad with color
  - [x] Generate 4 vertices from instance (position + size → quad corners via TriangleStrip)
  - [x] Pass bg_color to fragment shader
- [x] Background fragment shader:
  - [x] Solid fill with bg_color
- [x] Foreground vertex shader:
  - [x] Input: instance data (pos, size, uv, fg_color, bg_color, kind)
  - [x] Output: screen-space quad with UV coordinates
- [x] Foreground fragment shader:
  - [x] Sample glyph alpha from atlas texture (R8Unorm)
  - [x] Output: fg_color with sampled alpha (pre-multiplied alpha blending)
- [x] Uniform buffer struct (shared by both shaders):
  - [x] `screen_size: vec2<f32>` — viewport dimensions in pixels (16B with padding)
  - [x] Used to convert pixel coordinates to NDC (-1..1)

### Pipelines

- [x] `create_bg_pipeline(gpu: &GpuState, uniform_layout: &BindGroupLayout) -> RenderPipeline`
  - [x] Vertex shader: bg vertex
  - [x] Fragment shader: bg fragment
  - [x] Instance buffer layout: stride 80 bytes
  - [x] Blend state: premultiplied alpha (for transparent windows)
  - [x] Target format: `gpu.render_format()`
- [x] `create_fg_pipeline(gpu: &GpuState, uniform_layout: &BindGroupLayout, atlas_layout: &BindGroupLayout) -> RenderPipeline`
  - [x] Vertex shader: fg vertex
  - [x] Fragment shader: fg fragment
  - [x] Same instance buffer layout
  - [x] Blend state: premultiplied alpha
  - [x] Two bind groups: uniforms + atlas texture
  - [x] Target format: `gpu.render_format()`

### Instance Buffer Layout

```
Offset  Size  Field           Type
0       8     pos             vec2<f32>
8       8     size            vec2<f32>
16      16    uv              vec4<f32>
32      16    fg_color        vec4<f32>
48      16    bg_color        vec4<f32>
64      4     kind            u32
68      12    _pad            3 × u32
Total:  80 bytes per instance
```

- [x] Vertex pulling: no vertex buffer, use `@builtin(vertex_index)` to generate 4 vertices per instance (TriangleStrip)

---

## 5.5 Uniform Buffer + Bind Groups (verified 2026-03-29)

**File:** `oriterm/src/gpu/bind_groups/mod.rs` (190+ lines, 10 tests)

- [x] Uniform buffer:
  - [x] Create `wgpu::Buffer` with `BufferUsages::UNIFORM | COPY_DST`
  - [x] Size: 16 bytes (`vec2<f32> screen_size` + `vec2<f32> _pad`)
  - [x] Updated on resize: `UniformBuffer::write_screen_size(&queue, width, height)`
- [x] Uniform bind group layout:
  - [x] Binding 0: uniform buffer, vertex visibility (created in 5.4 pipeline.rs)
- [x] Atlas bind group layout:
  - [x] Binding 0: texture view (atlas page), fragment visibility (created in 5.4 pipeline.rs)
  - [x] Binding 1: sampler (linear filtering), fragment visibility (created in 5.4 pipeline.rs)
- [x] Create bind groups from layouts + resources
  - [x] `UniformBuffer::new()` — buffer + bind group from uniform layout
  - [x] `AtlasBindGroup::new()` — sampler + bind group from atlas layout + texture view
  - [x] `AtlasBindGroup::rebuild()` — recreate bind group when atlas texture grows
  - [x] `create_placeholder_atlas_texture()` — 1x1 `R8Unorm` white pixel for pre-atlas bootstrapping

---

## 5.6 Font Discovery + Rasterization (verified 2026-03-29)

**Files:** `oriterm/src/font/mod.rs`, `oriterm/src/font/collection/mod.rs`, `oriterm/src/font/collection/face.rs`, `oriterm/src/font/collection/tests.rs`. ~72 tests (32 + 40).

**Deviations from original plan:**
- Glyph-ID-based cache key (`RasterKey { glyph_id, face_idx, size_q6 }`) instead of char-based `GlyphKey`
- Separate resolve/rasterize: `resolve(char, style) -> ResolvedGlyph`, `rasterize(RasterKey) -> RasterizedGlyph`
- Subpixel rendering support via `GlyphFormat` enum (Alpha, SubpixelRgb, SubpixelBgr, Color)
- Synthetic bold/italic flags (`SyntheticFlags`) instead of silent fallback
- f32 metrics throughout (no integer truncation)
- `Arc<Vec<u8>>` for font bytes (shared with rustybuzz in Section 6)

- [x] Font discovery integration (via `discovery::discover_fonts()`):
  - [x] Platform discovery → load font bytes from system paths or embedded fallback
  - [x] `FontSet::load(family, weight) -> Result<Self, FontError>`
- [x] `FontData` struct: `data: Arc<Vec<u8>>`, `index: u32`
- [x] `FontSet` struct — 4 style variants + fallback chain:
  - [x] `regular`, `bold`, `italic`, `bold_italic`: `Option<FontData>`
  - [x] `fallbacks: Vec<FontData>` — fallback fonts for missing glyphs
- [x] `FontCollection` struct:
  - [x] Fields: `primary: [Option<FaceData>; 4]`, `fallbacks`, `size_px: f32`, `cell_width: f32`, `cell_height: f32`, `baseline: f32`, `glyph_cache`, `scale_context`
  - [x] `FontCollection::new(font_set, size_pt, dpi, format, weight) -> Result<Self, FontError>`
  - [x] `rasterize(&mut self, key: RasterKey) -> Option<&RasterizedGlyph>` — cache check → face lookup → swash render → store
  - [x] `resolve(&self, ch, style) -> ResolvedGlyph` — style substitution with synthetic flags
  - [x] `cell_metrics(&self) -> CellMetrics` — produces GPU-ready `CellMetrics`
  - [x] `find_face_for_char(&self, ch, style) -> ResolvedGlyph`
  - [x] Pre-cache ASCII (0x20–0x7E) at construction time
- [x] Shared types in `font/mod.rs`: `GlyphFormat`, `GlyphStyle`, `RasterKey`, `SyntheticFlags`, `ResolvedGlyph`, `FontError`
- [x] `RasterizedGlyph`: `width: u32`, `height: u32`, `bearing_x/y: i32`, `advance: f32`, `format: GlyphFormat`, `bitmap: Vec<u8>`
- [x] Internal `FaceData` + helpers: `validate_font()`, `font_ref()`, `has_glyph()`, `glyph_id()`, `rasterize_from_face()`, `compute_metrics()`, `cap_height_px()`, `size_key()`
- [x] 28 unit tests (embedded-only + system discovery)

---

## 5.7 Glyph Atlas (verified 2026-03-29)

Texture atlas for glyph bitmaps. Shelf-packing on 1024x1024 texture pages.

**File:** `oriterm/src/gpu/atlas/mod.rs` (579 lines -- HYGIENE NOTE: exceeds 500-line limit by 79 lines, should be split). 43 tests.

**Deviations from original plan:**
- Directory module (`atlas/mod.rs` + `atlas/tests.rs`) per test-organization rules.
- `new(device: &Device)` instead of `new(gpu: &GpuState)` — takes `Device` directly, matching bind_groups pattern.
- `insert` returns `Option<AtlasEntry>` (not bare `AtlasEntry`) — `None` for zero-size glyphs.
- `lookup` takes `RasterKey` by value (8 bytes, `Copy`) per clippy `trivially_copy_pass_by_ref`.
- Cache key is `RasterKey` (glyph-ID-based) rather than plan's generic `GlyphKey`.
- Pre-cache ASCII is orchestrated by caller (GpuRenderer, Section 5.10) since atlas doesn't own a FontCollection.
- Best-fit shelf selection minimizes wasted vertical space (vs naive first-fit).
- 1px padding between glyphs to prevent texture filtering artifacts.

- [x] `GlyphAtlas` struct
  - [x] Fields: `pages: Vec<wgpu::Texture>`, `page_views`, `shelves`, `cache: HashMap<RasterKey, AtlasEntry>`, `page_size: u32`
  - [x] `Shelf` struct: `y: u32`, `height: u32`, `x_cursor: u32`
  - [x] `GlyphAtlas::new(device: &Device) -> Self` — create first 1024×1024 R8Unorm page
  - [x] `insert(&mut self, key, glyph, device, queue) -> Option<AtlasEntry>` — shelf-pack + upload
  - [x] `lookup(&self, key) -> Option<&AtlasEntry>`
- [x] `AtlasEntry`: `page: u32`, `uv_x/y/w/h: f32`, `width/height: u32`, `bearing_x/y: i32`
- [x] Pre-cache ASCII (0x20–0x7E) at creation time
- [x] 25 unit tests (9 packing logic + 16 GPU integration)

---

## 5.8 Extract Phase (CPU) (verified 2026-03-29)

Lock terminal state, copy to owned snapshot, release lock immediately. No GPU types.

**File:** `oriterm/src/gpu/extract/from_snapshot/mod.rs` (211 lines). 21 extract tests + 24 frame_input tests = 45 total.

**Deviations from original plan:**
- Signature uses `ViewportSize` and `CellMetrics` newtypes instead of raw tuples.
- `CursorState` parameter omitted (blink logic is part of Section 5.12). Cursor visibility is already resolved by `Term::renderable_content()` via `TermMode::SHOW_CURSOR`.
- Generic over `T: EventListener` (not concrete `EventProxy`) for testability with `VoidListener`.
- Added `extract_frame_into` for buffer reuse (hot-path variant matching `renderable_content_into` pattern).

- [x] `extract_frame(terminal: &FairMutex<Term<T>>, viewport: ViewportSize, cell_size: CellMetrics) -> FrameInput`
  - [x] `let term = terminal.lock();` — fair lock
  - [x] Copy visible cells to `Vec<RenderableCell>` (via `Term::renderable_content()`)
  - [x] Copy cursor position/shape/visibility
  - [x] Copy active selection bounds (if any — `None` placeholder)
  - [x] Copy palette colors needed for this frame (`FramePalette` from `Palette`)
  - [x] `drop(term);` — release lock immediately
  - [x] Total lock hold time: microseconds
  - [x] Return `FrameInput` (fully owned, no references)
- [x] `extract_frame_into` — reuse variant that fills `&mut FrameInput` in place
- [x] `log::trace!` timing around lock acquire/release for profiling
- [x] **Rule**: after `extract_frame` returns, the terminal lock is NEVER touched again during this frame

### Testability

- [x] `FrameInput` can be constructed manually in tests (no terminal or lock needed)
- [x] `FrameInput` implements `Debug` for snapshot testing
- [x] Factory helpers: `FrameInput::test_grid(cols: usize, rows: usize, text: &str)` — build a test frame from a string
- [x] 14 extract tests + 5 test_grid tests (19 total new tests)

---

## 5.9 Prepare Phase (CPU) (verified 2026-03-29)

Convert `FrameInput` into GPU-ready instance buffers. **Pure CPU, no wgpu types, fully unit-testable.**

**File:** `oriterm/src/gpu/prepare/mod.rs` (485 lines) + `prepare/emit.rs` + `prepare/decorations.rs` + `prepare/shaped_frame.rs` + `prepare/dirty_skip/`. 135 tests.

- [x] `InstanceWriter` struct — reusable CPU-side byte buffer
  - [x] Fields: `buf: Vec<u8>`, `count: usize`, `stride: usize` (80)
  - [x] `new(stride)`, `clear()`, `push(data: &[u8])`, `count()`, `as_bytes()`, `into_buffer()`
  - [x] Grows but never shrinks — reused across frames

- [x] `prepare_frame(input: &FrameInput, atlas: &dyn AtlasLookup) -> PreparedFrame`
  - [x] `AtlasLookup` — trait that maps `(char, GlyphStyle) → AtlasEntry` (no GPU types)
    - [x] Production: backed by `FontCollection::resolve` + `GlyphAtlas::lookup` (Section 5.10)
    - [x] Tests: backed by `HashMap<(char, GlyphStyle), AtlasEntry>` — no GPU needed
  - [x] Create `PreparedFrame::with_capacity(cols, rows, palette.background, 1.0)`
  - [x] For each cell in `input.content.cells`:
    - [x] Skip `WIDE_CHAR_SPACER` cells (primary wide char handles both columns)
    - [x] Compute pixel position: `(col * cell_width, row * cell_height)`
    - [x] Build 80-byte bg instance: position, size, bg_color
    - [x] Push to `backgrounds` (wide chars get 2× cell_width)
    - [x] If cell has a visible character (not space):
      - [x] Convert `CellFlags` → `GlyphStyle` via `glyph_style()` helper
      - [x] Look up glyph UV in `atlas` by `(char, GlyphStyle)`
      - [x] Build 80-byte fg instance: position + bearing offset, glyph size, UV, fg_color
      - [x] Push to `glyphs`
  - [x] Build cursor instance(s) via `build_cursor()`:
    - [x] `Block` → full cell rect
    - [x] `Bar` → 2px vertical line at left edge
    - [x] `Underline` → 2px horizontal line at bottom
    - [x] `HollowBlock` → 4 thin outline rects (top, bottom, left, right)
    - [x] `Hidden` → no instances
    - [x] Only emitted when `cursor.visible` is true
  - [x] Selection highlight is a no-op (SelectionRange = (), selection = None until Section 9)
  - [x] Return `PreparedFrame` with populated instance writers + clear color

### Testability

- [x] `prepare_frame` is a pure function: same `FrameInput` + same `AtlasLookup` = identical `PreparedFrame`
- [x] Instance buffer contents are deterministic — snapshot-testable
- [x] No wgpu, no device, no queue — runs in `cargo test` without GPU
- [x] Test helpers:
  - [x] `assert_counts(prepared, bg, fg, cursor)` — verify instance counts
  - [x] `decode_instance(bytes: &[u8]) -> DecodedInstance` — parse 80-byte instance for assertions
  - [x] `DecodedInstance` has `pos`, `size`, `uv`, `fg_color`, `bg_color`, `kind` fields
- [x] 27 unit tests: instance correctness, counts, colors, positions, bearings, cursor shapes, determinism, glyph styles

---

## 5.10 Render Phase (GPU) (verified 2026-03-29)

Upload prepared buffers to GPU, execute draw calls, present. This phase is thin — all logic is in Prepare.

**File:** `oriterm/src/gpu/window_renderer/mod.rs` + `gpu/window_renderer/render.rs` + `gpu/window_renderer/helpers.rs` + `gpu/window_renderer/multi_pane.rs` (evolved from plan's `gpu/renderer/mod.rs`)

**Deviations from original plan:**
- Directory module (`renderer/mod.rs` + `renderer/tests.rs`) per test-organization rules.
- Single render pass with 3 draw calls (bg, fg, cursor) instead of 2 separate passes. Cursors use the bg pipeline as solid-fill rects.
- `push_cursor` changed to write color to `bg_color` field (matching bg shader) instead of `fg_color`.
- `PreparedFrame` gained `viewport: ViewportSize` field for uniform buffer update.
- `RendererAtlas` bridge struct implements `AtlasLookup` for the Prepare phase.
- `ensure_glyphs_cached` pre-pass rasterizes + inserts missing glyphs before prepare.
- `draw(0..4, ...)` (TriangleStrip) instead of plan's `draw(0..6, ...)`.

- [x] `GpuRenderer` struct
  - [x] Fields:
    - `bg_pipeline: wgpu::RenderPipeline`
    - `fg_pipeline: wgpu::RenderPipeline`
    - `uniform_buffer: UniformBuffer`
    - `atlas_bind_group: AtlasBindGroup`
    - `atlas_layout: wgpu::BindGroupLayout` — for atlas bind group rebuild
    - `atlas: GlyphAtlas`
    - `font_collection: FontCollection`
    - `bg_buffer: Option<wgpu::Buffer>` — GPU-side, grows as needed
    - `fg_buffer: Option<wgpu::Buffer>` — GPU-side, grows as needed
    - `cursor_buffer: Option<wgpu::Buffer>` — GPU-side, grows as needed
  - [x] `GpuRenderer::new(gpu: &GpuState, font_collection: FontCollection) -> Self`
    - [x] Create pipelines, uniform buffer, bind groups, atlas
    - [x] Pre-cache ASCII glyphs in atlas

- [x] `render_frame(&mut self, prepared: &PreparedFrame, gpu: &GpuState, target: &wgpu::TextureView)`
  - [x] **Note: accepts any `TextureView`** — not coupled to a surface
  - [x] Update uniform buffer with viewport size from `PreparedFrame::viewport`
  - [x] Ensure GPU buffers are large enough (grow if needed, never shrink)
  - [x] Upload instance data for backgrounds, glyphs, and cursors
  - [x] Create command encoder
  - [x] **Single render pass with 3 draw calls:**
    - [x] Draw 1: Backgrounds — `Clear` with `prepared.clear_color`, bg_pipeline
    - [x] Draw 2: Glyphs — fg_pipeline with atlas texture bind group
    - [x] Draw 3: Cursors — bg_pipeline (solid-fill, color in `bg_color`)
  - [x] `gpu.queue.submit([encoder.finish()])`

- [x] `render_to_surface(&mut self, prepared: &PreparedFrame, gpu: &GpuState, surface: &wgpu::Surface) -> Result<(), SurfaceError>`
  - [x] Acquire surface texture: `surface.get_current_texture()`
  - [x] Create view from surface texture (with sRGB render format)
  - [x] Call `render_frame(prepared, gpu, &view)`
  - [x] `output.present()`
  - [x] Handle surface errors: `Lost`/`Outdated` → caller reconfigures, `OutOfMemory`/`Timeout`/`Other` → propagated

- [x] GPU buffer management:
  - [x] `ensure_buffer(device, slot: &mut Option<Buffer>, data: &[u8]) -> Option<&Buffer>`
  - [x] If existing buffer is large enough, reuse it
  - [x] Otherwise, create new buffer (round up to power of 2, min 256)
  - [x] Prevents per-frame GPU buffer allocation

---

## 5.11 App Struct + Event Loop (verified 2026-03-29)

The main application struct. Implements winit's `ApplicationHandler`. Orchestrates the pipeline phases.

**File:** `oriterm/src/app/mod.rs` + `app/event_loop.rs` + `app/constructors.rs` + `app/init/`. 8 event loop tests + 10 cursor blink tests.

- [x] `App` struct
  - [x] Fields:
    - `gpu: Option<GpuState>` — initialized on `Resumed` event
    - `renderer: Option<GpuRenderer>` — initialized after GPU + fonts
    - `window: Option<TermWindow>` — created on `Resumed`
    - `tab: Option<Tab>` — single tab (multi-tab deferred to Section 15)
    - `event_proxy: EventLoopProxy<TermEvent>` — for creating EventProxy instances
    - `dirty: bool` — coalesced redraw flag
    - `first_frame: bool` — show window after first render
    - `window_config: WindowConfig` — cached window config
  - [x] Max ~10 fields. Additional state goes in dedicated sub-structs.
- [x] `impl ApplicationHandler<TermEvent> for App`
  - [x] `fn resumed(...)` — init GPU, window, fonts, renderer, first tab
  - [x] `fn window_event(...)`:
    - [x] `CloseRequested` → save pipeline cache, exit
    - [x] `Resized(new_size)` → resize surface + PTY
    - [x] `RedrawRequested` → **run the 3-phase pipeline:**
      1. `let frame = extract_frame(&tab.terminal, viewport, cell);`
      2. `let prepared = renderer.prepare(&frame, gpu);`
      3. `renderer.render_to_surface(&prepared, gpu, window.surface());`
    - [x] `KeyboardInput` → forward `event.text` to PTY (basic, expanded in Section 8)
    - [x] `ScaleFactorChanged` → update scale factor, mark dirty
  - [x] `fn user_event(...)` — handle terminal events (wakeup, title, bell, child exit, pty write)
  - [x] `fn about_to_wait(...)` — coalesce: if dirty, request_redraw, clear dirty
- [x] `TermWindow::from_window()` — wrap existing window with GPU surface (avoids double window creation)
- [x] `main.rs` rewritten: SIGCHLD init → build event loop → create App → `run_app`
- [x] Removed dead_code annotations from gpu, window, font, tab modules (items now consumed by App)

**Deviation:** Single `Option<Tab>` instead of `HashMap<TabId, Tab>` + `active_tab` — multi-tab is Section 15. No `frame_input_scratch` — the `extract_frame_into` optimization can be added when profiling shows it's needed. Event loop + event batching live in `app/mod.rs` (no separate `event_loop.rs` file needed — the impl is compact).

- [x] Event batching:
  - [x] Collect `dirty` flag during event processing
  - [x] In `about_to_wait`: if dirty, `request_redraw()`, clear dirty
  - [x] Prevents per-keystroke renders when typing fast

---

## 5.12 Basic Input + Cursor (verified 2026-03-29)

Minimal keyboard handling + cursor rendering. Just enough to type and see output.

- [x] `WindowEvent::KeyboardInput` handler:
  - [x] Extract `event.text` (logical text from keypress)
  - [x] Send to active tab: `tab.write_input(text.as_bytes())`
  - [x] Handle Enter (`\r`), Backspace (`\x7f`), Ctrl+C (`\x03`), Ctrl+D (`\x04`)
  - [x] Ignore modifier-only presses, function keys (expanded in Section 08)
- [x] Cursor rendering (handled in Prepare phase, 5.9):
  - [x] Block, Bar, Underline, HollowBlock shapes
  - [x] Blink: 530ms on, 530ms off (standard xterm timing)
  - [x] Reset blink on keypress
  - [x] Respect `TermMode::SHOW_CURSOR`

**Deviation:** Keyboard input was mostly implemented in 5.11 (basic `event.text` forwarding). 5.12 added cursor blink state machine (`app/cursor_blink/mod.rs`) with `CursorBlink` struct tracking 530ms on/off phases. Blink is application-level state — the terminal declares `TermMode::CURSOR_BLINKING`, the App drives the timer via `ControlFlow::WaitUntil`. Blink visibility is applied between Extract and Prepare phases. Enter/Backspace/Ctrl+C/Ctrl+D work automatically through winit's `event.text` field — no special-casing needed.

---

## 5.13 Render Pipeline Testing (verified 2026-03-29)

Testing strategy for the render pipeline. Three layers of tests, from fast/cheap to slow/thorough.

**Files:** `oriterm/src/gpu/prepare/tests.rs` (Layer 1, 135 tests), `oriterm/src/gpu/pipeline_tests.rs` (Layer 2, 15 tests), `oriterm/src/gpu/visual_regression/mod.rs` (Layer 3, `gpu-tests` feature gated, 33 reference PNGs)

**Deviations from original plan:**
- Layer 1 tests live in `prepare/tests.rs` (existing from 5.9) rather than a new `gpu/tests/` directory — follows sibling-tests pattern.
- Layer 2 tests live in `gpu/pipeline_tests.rs` (full-pipeline integration) rather than `gpu/tests/`.
- Layer 3 tests live in `gpu/visual_regression.rs` with references in `oriterm/tests/references/`.
- Cursor shapes generate separate reference PNGs per shape (4 images) instead of one composite.
- Selection overlay test deferred: `SelectionRange = ()` placeholder (arrives in Section 9).

### Layer 1: Unit Tests — Prepare Phase (no GPU, runs in `cargo test`)

These test the CPU-side rendering logic. Fast, deterministic, run everywhere.

- [x] **Instance buffer correctness:**
  - [x] Given a `FrameInput` with known cells, verify the exact bytes in `PreparedFrame`
  - [x] Test: single character 'A' at (0,0) → verify bg instance has correct position/size/color, fg instance has correct UV/position
  - [x] Test: empty cell (space) → bg instance only, no fg instance
  - [x] Test: wide character (CJK) → one bg instance spanning 2 cells, one fg instance
  - [x] Test: cursor at (5, 3) → verify cursor instance position matches cell position

- [x] **Instance count tests:**
  - [x] 80×24 grid with all spaces → 1920 bg instances, 0 fg instances
  - [x] 80×24 grid with all 'A' → 1920 bg + 1920 fg instances
  - [x] Grid with selection → instance counts unchanged (selection is color inversion, not overlay)

- [x] **Color resolution tests:**
  - [x] Default fg/bg → correct palette colors in instance bytes
  - [x] Bold text → bold color variant
  - [x] Inverse video → fg/bg swapped in instance
  - [x] 256-color and truecolor → correct RGB in instance bytes

- [x] **Layout tests:**
  - [x] Cell positions are pixel-perfect: cell (c, r) → position (c * cell_width, r * cell_height)
  - [x] Glyph bearing offsets applied correctly
  - [x] Viewport bounds respected (no instances outside viewport)

- [x] **Determinism test:**
  - [x] Same `FrameInput` → identical `PreparedFrame` bytes (bitwise equal)
  - [x] Run twice, compare — catches any hidden state or randomness

### Layer 2: Integration Tests — Headless GPU (needs GPU adapter, no window)

These test the full pipeline including GPU submission. Slower, but still automated.

- [x] **Headless rendering setup:**
  - [x] `GpuState::new_headless()` — creates adapter with `compatible_surface: None`
  - [x] Create offscreen `RenderTarget` (e.g. 640×480)
  - [x] Full pipeline: extract → prepare → render to offscreen target → read back pixels

- [x] **Pixel readback tests:**
  - [x] Render a single colored cell → verify the pixel region has the expected color
  - [x] Render white text on black background → verify non-zero alpha in glyph region
  - [x] Render cursor → verify cursor pixels are present at expected position

- [x] **Pipeline smoke tests:**
  - [x] Pipeline creation does not error
  - [x] GPU adapter is found
  - [x] Offscreen render target creates successfully
  - [x] A frame renders without GPU errors or validation warnings
  - [x] `wgpu` validation layer enabled in tests to catch API misuse

### Layer 3: Visual Regression Tests (optional, CI-friendly)

Compare rendered output against reference images. Catches subtle rendering regressions.

- [x] **Reference image workflow:**
  - [x] Render known terminal content to PNG via headless pipeline
  - [x] Compare against checked-in reference PNGs in `tests/references/`
  - [x] Fuzzy comparison: allow per-pixel tolerance (±2 per channel) for anti-aliasing differences
  - [x] On failure: save actual output + diff image for inspection
- [x] **Test scenarios:**
  - [x] `tests/references/basic_grid.png` — 80×24 grid with ASCII text
  - [x] `tests/references/colors_16.png` — 16 ANSI colors
  - [x] `tests/references/cursor_shapes.png` — all cursor shapes (4 separate PNGs)
  - [x] `tests/references/bold_italic.png` — styled text
- [x] **CI considerations:**
  - [x] Headless GPU tests require a GPU adapter in CI (or software rasterizer like lavapipe/llvmpipe)
  - [x] Mark as `#[ignore]` by default, run with `cargo test -- --ignored` in GPU-enabled CI
  - [x] Non-GPU unit tests (Layer 1) always run in all CI environments

---

## 5.14 Integration: Working Terminal (verified 2026-03-29)

The "it works" milestone. Everything comes together.

- [x] Launch sequence:
  - [x] `main.rs` creates `winit::EventLoop` with `TermEvent` user events
  - [x] Creates `App` struct
  - [x] `event_loop.run_app(&mut app)` — enters the event loop
  - [x] On `Resumed`: GPU init, window, fonts, renderer, first tab
- [x] Verify visually:
  - [x] Window opens (frameless, transparent/vibrancy)
  - [x] Terminal grid renders with monospace font
  - [x] Shell prompt appears
  - [x] Type `echo hello` → see "hello" in output
  - [x] Colors work: `ls --color` shows colored output
  - [x] Cursor is visible and blinks
  - [x] Window resize works (grid re-renders at new size)
  - [x] Scroll: output that exceeds screen scrolls correctly
- [x] Verify pipeline discipline:
  - [x] `log::trace!` timing shows: Extract < 100μs, Prepare < 1ms, Render < 2ms
  - [x] Terminal lock is never held during Prepare or Render phases
  - [x] No wgpu types appear in Extract or Prepare phase code
  - [x] Frame builds are deterministic (same input → same instance buffer bytes)
- [x] Verify threading:
  - [x] PTY reader thread processes output without blocking renderer
  - [x] No visible stutter when output is flowing

---

## 5.16 GPU Device Lost Recovery — Core Engine

<!-- WezTerm audit: #7519 (Windows sleep/resume D3D11 crash), #7703 (crash on macOS sleep in fullscreen). -->

**Mission (the user-experienced promise):** When a laptop user closes the lid, when a
discrete GPU is power-gated, when a display driver crashes, or when the OS forces a TDR —
**oriterm does not crash**, all PTY processes keep running, scrollback is preserved
byte-for-byte, selections and search state are preserved, and the user's next keystroke
either renders normally (recovery succeeded) or hits a clear "GPU unavailable, press
Ctrl+R to retry" indication (recovery exhausted). At no point does the terminal lose state
or steal CPU in a recovery spin loop.

This mission has three load-bearing invariants. Every sub-item below traces back to one
of them and the test matrix in 5.16.13 verifies each one explicitly:

- **I1 — No crash on device loss.** Every code path that touches the GPU must consult
  `App::gpu_health` before submitting; no `unwrap()` on a stale wgpu handle survives loss.
- **I2 — Terminal state preservation.** PTY processes (`oriterm_mux::Pane`), scrollback
  rows (`oriterm_core::Grid`), per-pane selections (`App::pane_selections`), per-pane
  mark cursors (`App::mark_cursors`), search state, hyperlinks, image bytes
  (`oriterm_core::image::Image`), config, font collection, font byte cache, fallback map,
  IME state, clipboard, scrollback positions, and dirty-tracker contents are **all**
  CPU-side and survive recovery. Recovery may NOT touch any of them. Detailed invariants
  and the canonical-snapshot stress test live in **5.17**.
- **I3 — Bounded cost, no spin.** Recovery is single-flight, exponential backoff, total
  budget capped, and `Unavailable` state requires a manual user trigger to retry.

**Section split rationale:** Section 5.16 is the *core recovery engine*: detection,
state machine, teardown, recreation, first frame, backoff, minimal UX, logging, core
tests, and the section completion gate. Section 5.17 handles *correctness infrastructure*
— the recovery-module scaffold, terminal-state preservation invariants (I2 enforcement),
the canonical-snapshot stress test, and exhaustive enum/test-table coverage. Section 5.18
handles *cross-section integrations and deferred contracts* — PIN tests for later sections,
daemon-mode integration, manual destructive matrix, and items requiring user approval.
This split lets `/continue-roadmap` checkpoint at three sane gates instead of one giant
monolithic section.

**Source:** WezTerm #7519, #7703 — after system sleep/resume the GPU device is lost. WezTerm
crashed with APPCRASH in `d3d11.dll` and EGL errors on macOS. Alacritty addressed the same class
of bug with context-robustness in commit `cd884c9` (Jan 2025). This affects every laptop user
who closes the lid, every desktop user who installs a GPU driver update mid-session, and every
user whose discrete GPU is power-gated by the OS.

**Problem (verified against the codebase 2026-04-06):**

1. `oriterm/src/gpu/window_renderer/error.rs::SurfaceError` collapses wgpu's `Lost` AND
   `Outdated` variants into a single `Lost` arm, then folds `Other` into a generic `Other`.
   This loses the distinction the recovery state machine needs: `Outdated` is a benign
   reconfigure, `Lost` is a real swapchain teardown, `Other` may carry a device-lost detail.
2. `oriterm/src/app/redraw/post_render.rs::finish_render` "recovers" from `SurfaceError::Lost`
   by calling `ctx.window.resize_surface()` + `apply_pending_surface_resize()`. That re-enters
   `wgpu::Surface::configure()` against the **same** `wgpu::Device`. If the device itself is
   lost, every subsequent submit silently fails and the terminal hangs forever.
3. `GpuState` (`oriterm/src/gpu/state/mod.rs`) is constructed once in `App::try_init`
   (`oriterm/src/app/init/mod.rs:59`) and never re-created. There is no `recreate()`,
   no `set_device_lost_callback`, no health flag.
4. `GpuPipelines` (`oriterm/src/gpu/pipelines.rs`) holds 6 `RenderPipeline`s and 3
   `BindGroupLayout`s — all of them tied to the original `wgpu::Device`. Per-window
   `WindowRenderer` (`oriterm/src/gpu/window_renderer/mod.rs`) holds 13 instance `Buffer`s,
   3 `GlyphAtlas` textures + bind groups, an `ImageTextureCache`, a `content_cache` texture,
   and a `UniformBuffer` — all device-bound. After device loss every one of these handles
   becomes invalid and must be dropped before any new device is created.

**Architectural reframing:** This is **not** "surface loss handling." It is a *global GPU
epoch reset* that must be coordinated at `App` level across **all** windows
(`App::windows: HashMap<WindowId, WindowContext>` and `App::dialogs: HashMap<WindowId,
DialogWindowContext>`). Section 5.16 introduces an explicit recovery state machine with
deterministic teardown order, multi-window adapter validation, bounded retries, and a
terminal "renderer unavailable" state with manual retry.

### wgpu 28.0.0 API ground-truth (verified against `Cargo.lock` — wgpu 28.0.0, winit 0.30.13)

- `Device::poll(PollType) -> Result<PollStatus, PollError>` where
  `PollError ∈ { Timeout, WrongSubmissionIndex }`. **`Device::poll` does NOT signal device
  loss** — the original 5.16 plan item was wrong.
- The device-loss hook is `Device::set_device_lost_callback(impl Fn(DeviceLostReason, String)
  + Send + 'static)` where `DeviceLostReason ∈ { Unknown, Destroyed }`. The callback fires
  exactly once per device — must be re-registered after every `recreate()`.
- `Surface::get_current_texture() -> Result<SurfaceTexture, SurfaceError>` where
  `SurfaceError ∈ { Timeout, Outdated, Lost, OutOfMemory, Other }`. `Outdated` and `Lost`
  must be handled differently: `Outdated` reconfigures, `Lost` triggers full epoch reset.
  `Other`'s detail comes through error scopes / device-lost callback per the wgpu doc.
- **`Surface::configure()` is load-bearing — three panic/error modes that the recovery
  path MUST guard against** (each enforced by an explicit `debug_assert!` in 5.16.3):
  1. Panics if any old `SurfaceTexture` from that surface is still alive. Recovery MUST
     drop every in-flight `SurfaceTexture`, encoder, and frame-local handle before
     reconfiguring.
  2. Rejects zero width or zero height — minimized windows must clamp to `(1, 1)`
     (already done in `gpu/state/helpers.rs::build_surface_config`, verified).
  3. Validates that no in-flight `Queue::submit` is racing the reconfigure. Recovery
     pauses render with the App-level gate (5.16.2) before any teardown so no submit can
     race; the gate is the hard render pause / epoch barrier.
- `RequestAdapterOptions { compatible_surface: Option<&Surface> }` accepts only **one**
  surface. With multi-window we must call `Adapter::is_surface_supported(&surface)` for
  each remaining window's surface after picking the adapter.
- `wgpu::util::pipeline_cache_key(&AdapterInfo)` derives the per-driver cache filename;
  this is the **primary** identifier for the cached file because it captures driver UUID
  + device UUID (i.e., it detects "wrong device reuse" — the catastrophic case of loading
  a cache built against a now-discrete GPU into a now-integrated GPU). 5.16.5 keys the
  quarantine sidecar by `pipeline_cache_key`, with CRC as a secondary integrity check
  for byte-level corruption (e.g., truncated writes from a power-loss).
- `device.create_pipeline_cache(...)` already runs with `fallback: true` in
  `oriterm/src/gpu/state/pipeline_cache.rs:39` — corrupt or driver-mismatched files are
  silently ignored. We add **quarantine on repeated failure** to prevent infinite retry
  loops if a cache file consistently triggers a crash inside the driver.
- **`winit::event::WindowEvent::Occluded` is NOT a recovery trigger.** Per current winit
  docs, `Occluded` is unsupported on Windows, Wayland, and Android — only macOS and X11
  emit it reliably. Building any recovery decision around `Occluded` would silently
  break on the most common deployment targets. This section deliberately does **not**
  consume `Occluded` for any purpose.
- **`winit::Window::request_user_attention(Some(UserAttentionType::Critical))`** is the
  cross-platform "blink the taskbar / bounce the dock / urgent notification" hook that
  5.16.10 uses for the minimal Unavailable UX. No new dependency required.

### Existing primitives we can reuse (verified)

- `GpuState::create_surface(&Arc<Window>)` (`gpu/state/mod.rs:196`) already builds and
  configures a surface from cached format/alpha/present-mode state. Recovery calls this
  per window after recreating the device.
- `WindowRenderer::clear_and_recache(gpu)` (private, `gpu/window_renderer/font_config.rs:176`)
  already drops every atlas, clears `empty_keys`, clears `icon_cache`, resets the shaping
  scratch frame, and pre-caches ASCII for the active subpixel/mono atlas. **This is the
  exact primitive we need for atlas reset** — it must become `pub(crate)` so the recovery
  path can call it via a stable name (`reset_gpu_resources`).
- `FontCollection::glyph_cache: HashMap<RasterKey, RasterizedGlyph>` (`font/collection/mod.rs:103`)
  retains CPU bitmaps after upload (verified — Codex's blind-spot claim that bitmaps were not
  retained is **false** for our codebase). After device recreation, lazy rerasterization on
  next frame is sufficient and is the chosen approach (option (b) in the blind-spot check),
  but eager re-upload from this cache is also an available optimization if measurements show
  a visible blank flash on the first post-recovery frame.
- `ImageTextureCache::ensure_uploaded` (`gpu/image_render/mod.rs:75`) takes image bytes from
  `input.content.image_data` per frame (`window_renderer/frame_prep.rs:159`). The CPU bytes
  live in `oriterm_core::image::Image` and survive device loss. Image textures rebuild on the
  first post-recovery extract.

### Required work

#### 5.16.1 Detection — distinguish surface vs device loss

- [ ] Split `oriterm/src/gpu/window_renderer/error.rs::SurfaceError` into a wider enum that
      preserves wgpu's actual variants:
      `Outdated` (benign reconfigure), `Lost` (device-or-swapchain epoch reset),
      `Timeout` (pass through, retry next frame), `OutOfMemory` (terminal — see 5.16.10),
      `Other(String)` (carry the wgpu detail string for logging). Update the conversion in
      `WindowRenderer::render_to_surface` (`gpu/window_renderer/render.rs:207`).
- [ ] Add a global `DeviceHealth` enum on `App` (or on a new `GpuEpoch` type owned by `App`):
      `Healthy { epoch: u64 }`, `Recovering { epoch: u64, attempt: u8, since: Instant }`,
      `Unavailable { last_error: String, since: Instant }`. The `epoch` counter monotonically
      increments on every successful recreate so other state can detect stale epochs without
      pointer comparison.
      WHERE: new file `oriterm/src/gpu/recovery/mod.rs`, owned by `App` as `gpu_health: GpuHealth`.
- [ ] Register a `device_lost_callback` immediately after every successful
      `Adapter::request_device(...)` in `oriterm/src/gpu/state/helpers.rs::request_device`.
      The callback signals recovery via `EventSender::send(TermEvent::GpuDeviceLost { reason,
      message })`. Add the new `TermEvent` variant in `oriterm/src/event.rs` and wire its
      handler in `oriterm/src/app/event_loop.rs`.
- [ ] In `WindowRenderer::render_to_surface` map `wgpu::SurfaceError::Outdated` to the new
      `SurfaceError::Outdated` arm (currently collapsed into `Lost`). Update
      `app/redraw/post_render.rs::finish_render`: `Outdated` triggers
      `resize_surface + apply_pending_surface_resize` (the existing path); `Lost` and any
      device-lost callback signal trigger the full recovery state machine in 5.16.2.
- [ ] Treat `Other` from `get_current_texture` as **soft device loss**: log the detail
      string from wgpu, then escalate to the recovery state machine. Real-world WSLg / DX12
      driver bugs surface here when a device-lost detail is reported via error scopes
      instead of `SurfaceError::Lost`.
- [ ] **Wrap submit failure detection too.** `wgpu::Queue::submit` does not return an
      error in wgpu 28, but a device-lost callback may have fired *between* submit and
      present. After every `output.present()`, check whether the device-lost callback fired
      since the last frame (compare a `Cell<u64>` epoch against `App::gpu_health.epoch()`)
      — if so, that frame's pixels are garbage and the next frame will see the recovery
      gate. Drop the `SurfaceTexture` *before* updating the gate so `Surface::configure`
      in 5.16.6 does not panic.
- [ ] **Synthetic loss for testing**: introduce `App::trigger_test_device_loss(reason)`
      gated behind `#[cfg(any(test, feature = "gpu-tests"))]` that posts the same
      `TermEvent::GpuDeviceLost` the callback would. Required by 5.16.13 to exercise the
      full state machine without a real GPU crash.

#### 5.16.2 Recovery state machine — App-level, serialized across windows

- [ ] **Render-gating must be exhaustive.** The gate lives at the *single canonical
      dispatch point* `App::render_dirty_windows` in `oriterm/src/app/render_dispatch.rs`
      (verified — already the only place that walks `windows` + `dialogs` and calls
      `handle_redraw` / `render_dialog`). Before either loop, consult `App::gpu_health`:
      when `Recovering` or `Unavailable`, **return early** without clearing dirty flags
      and without invoking any `WindowRenderer` method. Windows stay dirty so the next
      successful frame after recovery is a full repaint. Enumerated guarded entry points
      that the gate must cover (audited from the codebase 2026-04-06):
  - [ ] `App::handle_redraw` (single-window terminal redraw)
  - [ ] `App::render_dialog` (dialog windows)
  - [ ] Multi-pane redraw (`oriterm/src/app/redraw/multi_pane/`) — gates via the same
        single-source dispatcher; do NOT add a duplicate guard inside the multi-pane
        helper (algorithmic-DRY: one canonical gate, not two).
  - [ ] Chrome scene refresh (`oriterm/src/app/redraw/chrome.rs`) — pure CPU but it
        writes into `WindowContext.chrome_scene` which is consumed by render. Allowed to
        run during recovery (no GPU touch); the gate fires *after* prepare, before submit.
  - [ ] Debug overlay (`debug_overlay.rs`) — same: CPU only, allowed during recovery.
  - [ ] Search bar redraw (`search_bar.rs`) — CPU only, allowed.
  - [ ] Pre-edit overlay (`preedit.rs`) — CPU only, allowed.
- [ ] **Add a `RenderOutcome` enum** to make the gate testable without GPU:
      `RenderOutcome { Submitted, GatedRecovering, GatedUnavailable, Skipped }`. Pure
      tests assert that calling the dispatcher with an injected `gpu_health` returns the
      expected outcome. SSOT: this enum lives in `oriterm/src/gpu/recovery/outcome.rs`
      next to `GpuHealth`.
- [ ] Recovery is **single-flight**: only one recovery attempt may be running across the
      entire `App` at any time. Subsequent `GpuDeviceLost` notifications while
      `Recovering` are coalesced — they don't restart the attempt.
- [ ] Ordering invariant for `App::recover_gpu()` (must be enforced by the function body
      and asserted with `debug_assert!` at each step):
      1. Set `gpu_health = Recovering { attempt }` and broadcast a "block render" flag.
      2. For every window in `self.windows` and `self.dialogs`: drop any in-flight
         `wgpu::SurfaceTexture` (none should exist outside `render_to_surface`, but assert).
         Drop the window's `wgpu::Surface<'static>` field on `TermWindow`
         (`oriterm/src/window/mod.rs:49`).
      3. For every window's `WindowContext.renderer: Option<WindowRenderer>`, take
         `Option::take()` to drop all per-window GPU state (atlases, bind groups, instance
         buffers, content cache, image cache, uniform buffer). Hold the bare
         `FontCollection` and `Option<UiFontSizes>` aside on the stack so they survive.
      4. Drop `self.pipelines: Option<GpuPipelines>` (every `RenderPipeline` and
         `BindGroupLayout`).
      5. Drop `self.gpu: Option<GpuState>` (`Device`, `Queue`, `Instance`, pipeline cache).
      6. Construct a new `GpuState` (5.16.4), recreate per-window surfaces (5.16.6), rebuild
         `GpuPipelines` (5.16.7), rebuild every `WindowRenderer` (5.16.8).
      7. Bump `epoch`, set `gpu_health = Healthy { epoch }`, queue a redraw on every window.
- [ ] **Do NOT** call `Device::poll(PollType::Wait)` on the lost device — `wgpu::PollError`
      cannot signal loss and a `Wait` against a destroyed device blocks indefinitely on some
      backends. Drop instead.
- [ ] **Gate every wakeup source.** Audit of timers/animators that pull a frame
      (verified against the codebase 2026-04-06) — every one must be quiesced while
      `gpu_health != Healthy`:
  - [ ] **Cursor blink** (`App::cursor_blink`, `App::blink_wakeup_gen`) — the wakeup
        thread must check `gpu_health.is_healthy()` before posting `MuxWakeup`. Failed
        check → exit silently (the recovery state-change wake-up will resume blinking).
  - [ ] **Text blink** (`App::text_blink`, SGR 5/6) — same gate as cursor blink.
  - [ ] **Tab slide animation** (`WindowContext.tab_slide`) — already CPU-only, but
        `request_redraw()` must consult the gate. Animation *progress* may continue (CPU
        time updates) so the visible state matches reality after recovery.
  - [ ] **Layer animator** (`oriterm_ui::compositor::LayerAnimator` via
        `WindowRoot::layer_animator`) — already CPU-only, but the wakeup that drives
        `is_any_animating()` → `WaitUntil` must respect the gate.
  - [ ] **Render scheduler** (`oriterm_ui::animation::RenderScheduler` on `WindowRoot`)
        — same: CPU-only state survives, but the wakeup is gated.
  - [ ] **Visual state animator** for hover/press transitions — same.
  - [ ] **Auto-scroll timer** (mark-mode and selection-drag scroll) — gated.
  - [ ] **Cursor hover hold timer** (URL hover, tooltip) — gated.
  - [ ] **Mux pump** (`App::mux_pump`) — **NOT gated**: PTY output must continue to be
        absorbed into `Term`/`Grid` so terminal state stays current. Only the *render
        wakeup* the pump would post is suppressed; the snapshot still flows to the dirty
        flag and the dirty flag is consumed on the first post-recovery frame.
- [ ] **Pure helper for the gate decision.** Add a free function
      `oriterm::gpu::recovery::should_post_wakeup(gpu_health: &GpuHealth, source:
      WakeupSource) -> bool` so tests can assert each (state, source) combination
      exhaustively. `WakeupSource` is an enum mirroring the bullets above (single source
      of truth — adding a new wakeup source forces an exhaustive match arm here).
- [ ] **Pane render-cache invalidation:** while `Recovering`, do NOT call
      `pane_cache.invalidate_all()` per pane on every notification. The teardown in
      5.16.3 will drop the cache wholesale. Repeatedly invalidating during recovery is
      wasted CPU.

#### 5.16.3 Resource teardown — ordered drop list

The teardown must drop in **reverse construction order** so that nothing references a
dropped wgpu handle. The recovery code lives in `App::recover_gpu` and uses helper methods
named after the resource family. Each helper is `&mut self` and is responsible for
preserving the CPU-side state that survives loss.

**Verified state inventory** (audited from `App` / `WindowContext` /
`DialogWindowContext` / `WindowRenderer` / `GpuState` 2026-04-06):

- [ ] `App::drop_per_frame_state()` — clear `App::scratch_dirty_windows`,
      `App::scratch_pane_sels`, `App::scratch_pane_mcs`, `App::notification_buf`. None of
      these own GPU handles but they may reference content from the pre-recovery frame.
      `pending_destroy`, `pending_focus_out`, `torn_off_pending`, and
      `pending_dropdown_id` are explicitly **kept** — they describe pending app-level
      actions that must still execute after recovery.
- [ ] `App::drop_per_window_gpu_state()` — for each `WindowContext`:
  - [ ] Drop `pane_cache: PaneRenderCache`. Verified: `PaneRenderCache` itself stores
        only `PreparedFrame` (CPU instance data), no direct wgpu handles, so it can be
        cleared in place. **However** the prepared frames reference `AtlasEntry`
        coordinates that point into the about-to-be-dropped glyph atlas — those entries
        are stale across recovery. Use `pane_cache = PaneRenderCache::new()` to wipe.
  - [ ] Drop `frame: Option<FrameInput>` (`take()`). `FrameInput` is CPU but contains
        snapshot references that are stale across recovery.
  - [ ] Drop `chrome_scene: Scene`. Scene is pure CPU but holds shaped glyph IDs keyed
        to the old atlas. `chrome_scene = Scene::new()` (drops the old vec capacity to
        a fresh state — a one-shot cost, not a hot path).
  - [ ] Clear `text_cache: TextShapeCache`. Pure CPU but glyph IDs key to old atlas.
  - [ ] Clear `tab_slide: TabSlideState` to its idle pose so the first post-recovery
        frame is not mid-animation against stale layout.
  - [ ] Clear `cached_dividers: Option<Vec<DividerLayout>>` — recomputed on first
        post-recovery layout pass.
  - [ ] Clear `tab_bar_phys_rect`, `status_bar_phys_rect` — recomputed on first
        post-recovery layout pass.
  - [ ] Reset `last_rendered_pane: Option<PaneId>` to `None` so the post-recovery frame
        triggers `force_full_refresh` (single-pane path tab-switch detector).
  - [ ] Reset `prev_text_blink_opacity` to `1.0` so blink-detection doesn't think the
        first frame is a text-blink-only delta.
  - [ ] Set `ui_stale = true` so the next frame is treated as content-changed.
  - [ ] **Preserve untouched** (CPU-only state, terminal-state preservation invariant
        I2): `tab_bar`, `status_bar`, `terminal_grid` widget (visual config),
        `hovered_url`, `url_cache`, `divider_drag`, `floating_drag`, `tab_drag`,
        `context_menu`, `last_drag_area_press`, `last_tab_press`, `search_bar_buf`,
        `debug_overlay_buf`, the entire `WindowRoot` (interaction, focus, overlays,
        scheduler, layer tree, layer animator — all pure CPU per
        `.claude/rules/crate-boundaries.md`). Verified by reading `WindowRoot` field set:
        none of `widget`, `layout`, `viewport`, `interaction`, `focus`, `overlays`,
        `keymap`, `key_contexts`, `last_keymap_handled`, `layer_tree`, `layer_animator`,
        `frame_requests`, `scheduler`, `invalidation`, `damage`, `dirty`,
        `urgent_redraw`, or `pending_actions` reference wgpu types.
  - [ ] **Mark dirty** at the end: `ctx.root.invalidation_mut().invalidate_all();
        ctx.root.damage_mut().reset(); ctx.root.mark_dirty();` so layout + paint run
        cleanly on the first post-recovery frame.
  - [ ] Take `renderer.take()` last (drops `WindowRenderer`: 13 instance buffers, 3
        atlases + bind groups, image texture cache, content cache texture, uniform
        buffer, icon cache, image instance buffer). Atlases are dropped here, **not**
        cleared in place via `clear_and_recache` — clear-in-place still references the
        old device.
- [ ] `App::drop_per_dialog_gpu_state()` — symmetrical for `DialogWindowContext`. Also
      preserves `WindowRoot` (dialog interaction state survives — a half-typed entry in
      the Settings dialog stays put across recovery).
- [ ] `App::drop_window_surfaces()` — for each window, take the `Surface` field on
      `TermWindow` (introduce `TermWindow::take_surface() -> Option<wgpu::Surface<'static>>`
      since the field is currently private — see `oriterm/src/window/mod.rs:49` and the
      verified absence of any take/set helper). The corresponding setter
      `TermWindow::set_surface(surface, config)` is added in 5.16.6.
- [ ] `App::drop_pipelines()` — `self.pipelines.take()` (drops `GpuPipelines`).
- [ ] `App::drop_gpu_state()` — `self.gpu.take()` (drops `GpuState`, including
      `pipeline_cache: Option<PipelineCache>` and the `wgpu::Instance`).
- [ ] **Drop ordering pin**: a debug-only invariant test asserts the order
      `per_frame → per_window → per_dialog → window_surfaces → pipelines → gpu_state`.
      Reordering causes use-after-free of layouts/devices in some wgpu backends and is a
      latent crash; pin the order with a test to catch refactor regressions.
- [ ] **Explicit `debug_assert!` invariants** — required by `.claude/rules/impl-hygiene.md`
      "Invariant Explicitness". Each is enforced AT the call site, not via prose:
  - [ ] **Before `App::drop_window_surfaces()` runs**: `debug_assert!(self.in_flight_surface_textures.is_empty(), "Surface::configure panics if any old SurfaceTexture from that surface is still alive")`. The recovery gate (5.16.2) ensures no render is in flight when we get here, but the assert documents and enforces the contract.
  - [ ] **Before `Surface::configure(...)` runs in 5.16.6**: `debug_assert!(width > 0 && height > 0, "Surface::configure rejects zero dimensions; minimized windows must clamp to (1,1)")`. Verified at `gpu/state/helpers.rs:165` — `build_surface_config` already clamps, but the assert protects against future refactors.
  - [ ] **Before `App::drop_pipelines()` runs**: `debug_assert!(self.windows.values().all(|ctx| ctx.renderer.is_none()), "All WindowRenderers must be dropped before pipelines (bind groups reference layouts)")`.
  - [ ] **Before `App::drop_gpu_state()` runs**: `debug_assert!(self.pipelines.is_none(), "GpuPipelines must be dropped before GpuState (pipelines reference device)")`.
  - [ ] **Before `Queue::submit` resumes after recovery**: `debug_assert!(matches!(self.gpu_health, GpuHealth::Healthy { .. }), "render gate failure — submit attempted while gpu_health != Healthy")`. This is the I1 invariant.
- [ ] **CPU-side state explicitly preserved across teardown** (the I2 invariant —
      mirrored in the test in 5.16.13):
  - `App::session: SessionRegistry` (tabs, windows, layouts)
  - `App::mux: Box<dyn MuxBackend>` (PTY threads, panes — **never touched**)
  - `App::pane_selections`, `App::mark_cursors`
  - `App::clipboard`, `App::config`, `App::bindings`, `App::_config_monitor`
  - `App::ime`, `App::modifiers`, `App::ui_theme`
  - `App::cursor_blink`, `App::text_blink`, `App::blinking_active`,
    `App::blink_wakeup_gen`, `App::next_blink_gen`, `App::last_cursor_pos`
  - `App::mouse: MouseState`, `App::mouse_cursor_hidden`
  - `App::window_manager`, `App::focused_window_id`, `App::active_window`
  - `App::pending_destroy`, `App::pending_focus_out`, `App::torn_off_pending`,
    `App::pending_dropdown_id`
  - `App::font_set`, `App::user_fallback_map` (the cached `FontSet` and the user
    fallback index map — survive so a new window opened mid-recovery uses the same
    fonts)
  - **Per-window**: the surviving `FontCollection` and `Option<UiFontSizes>` extracted
    from each `WindowRenderer` *before* it is dropped — these are reused by 5.16.8
    without any disk reads.

Each helper returns the bare CPU-side state needed for rebuild (per-window
`(FontCollection, Option<UiFontSizes>, AtlasFiltering, bool /* subpixel_positioning */,
HintingMode, GlyphFormat)` tuples) and consumes only what it must. The rebuild path in
5.16.8 re-applies each setting to the freshly constructed `WindowRenderer`.

#### 5.16.4 Device + adapter recreation — multi-window-aware

- [ ] Add `GpuState::recreate(&[&Arc<Window>], transparent: bool, backend: GpuBackend)
      -> Result<Self, GpuRecoveryError>` in `oriterm/src/gpu/state/mod.rs`. It mirrors
      `GpuState::new` but takes a slice of all live windows so the new adapter can be
      validated against every surface, not just the first.
- [ ] Inside `recreate`: build a new `wgpu::Instance` (cannot reuse the old one — a stale
      instance after device loss is undefined behaviour on some backends), create a probe
      surface from the **first** window for `pick_adapter`, then call
      `adapter.is_surface_supported(...)` for every remaining window's freshly-created
      surface. Reject the adapter and try the next backend if any window's surface is
      unsupported.
- [ ] After `request_device` succeeds, immediately call
      `device.set_device_lost_callback({ let proxy = self.event_proxy.clone(); move |reason,
      msg| proxy.send(TermEvent::GpuDeviceLost { reason, message: msg }) })`. The callback
      moves a clone of `EventSender` so it survives the device.
- [ ] On total failure (every backend rejected), return `GpuRecoveryError::NoAdapter` so
      the state machine can transition to `Unavailable`.
- [ ] Re-negotiate adapter-derived caps fresh from the new adapter+surface: `surface_format`,
      `render_format`, `surface_alpha_mode`, `present_mode`, `supports_view_formats`,
      `dual_source_blending`, `uses_dcomp`. Do **not** assume the new adapter has the same
      caps as the old one — a discrete GPU power-down may force fallback to integrated.
- [ ] If `dual_source_blending` flips between attempts, the subpixel pipeline shape changes
      and `GpuPipelines` must be rebuilt to match (which 5.16.7 does anyway, but log it).
- [ ] **Capability-change handling matrix.** When the new adapter exposes a *different*
      set of capabilities than the old one, the rebuild must adapt rather than crash.
      Verified caps that may flip between attempts (read directly from `GpuState` fields):
  - `surface_format` / `render_format` — may flip from BGRA8 to RGBA8 (WSLg, host
    driver swap). The renderer reads format at frame time so just pick up the new value.
  - `surface_alpha_mode` — Vulkan fallback may force `Opaque` even though we wanted
    transparency. Recovery must call `clear_compositor_surface_flag` per 5.16.6 and
    log a downgrade warning so the user knows transparency is lost until restart.
  - `present_mode` — Mailbox → Fifo downgrade on the fallback adapter. Latency
    increases by one refresh, no other consequence.
  - `supports_view_formats` — flips affect cache-blit eligibility. The renderer's
    `gpu.can_cache_blit()` is consulted per-frame so the change is picked up
    automatically; the first post-recovery frame still uses single-pass per 5.16.9.
  - `dual_source_blending` — flipping changes the subpixel pipeline shape; rebuild
    handles this by reconstructing `GpuPipelines` against the new device.
  - `uses_dcomp` — DComp → non-DComp downgrade requires `clear_compositor_surface_flag`
    on every window that was created with `WS_EX_NOREDIRECTIONBITMAP`.
- [ ] **DPI re-query.** During the lid-closed period the user may have moved the laptop
      to a new external monitor with a different scale factor. Before constructing each
      new `WindowRenderer`, call `window.window().scale_factor()` and compare against
      the renderer's last known DPI. If different, call `font_collection.set_size(...)`
      with the new physical DPI on the surviving `FontCollection` *before* handing it to
      `WindowRenderer::new`. Without this the first post-recovery frame renders glyphs
      at the wrong size.
- [ ] **Window size re-query.** Same reason: `inner_size()` may differ from
      `surface_config` after a docking change. Always re-query both physical size and
      scale factor at the start of recreation, never trust the cached value on
      `TermWindow`.

#### 5.16.5 Pipeline cache lifecycle — quarantine bad files

- [ ] **`pipeline_cache_key` is the primary identifier.** Recompute from the **new**
      `AdapterInfo` via `wgpu::util::pipeline_cache_key(&adapter_info)` after every
      adapter probe. The result captures driver UUID + device UUID — the only safe way
      to detect "wrong device reuse" (e.g., loading a discrete-GPU cache into the
      now-active integrated GPU after a power-state flip). The cache filename **is** the
      `pipeline_cache_key` string, no transformation. Verified at
      `oriterm/src/gpu/state/pipeline_cache.rs:23` — already used as the filename today
      via `cache_dir.join(cache_key)`.
- [ ] **CRC is a secondary integrity check, not a primary key.** Wrap the cache bytes
      with a 16-byte header `[ b"oriterm\0" (8) || u32 version || u32 crc32 ]` and
      validate the CRC before passing to `create_pipeline_cache`. A header mismatch or
      CRC failure treats the file as quarantine-eligible immediately (catches truncated
      writes from a power-loss-during-save). The CRC alone is **not** sufficient because
      a byte-perfect cache from the wrong adapter still has a valid CRC.
- [ ] **Sidecar attempt counter for quarantine.** In
      `oriterm/src/gpu/state/pipeline_cache.rs::load_pipeline_cache`, before reading the
      file, look for a sidecar `<cache_path>.attempt` file containing a small counter.
      Increment it before load, persist, then proceed. After successful pipeline creation
      in `GpuPipelines::new`, delete the sidecar (cache survived). If the counter reaches
      2 on entry, **rename the cache file to `<cache_path>.quarantine.<timestamp>`** and
      proceed without a cache. This breaks the infinite-recover-then-crash loop seen on
      bad driver updates.
- [ ] Persist the **new** cache (different filename if adapter changed because
      `pipeline_cache_key` changed) on the next call to
      `GpuState::save_pipeline_cache_async`. Old caches are left on disk untouched —
      they remain valid for the *original* adapter if the user later switches back.
- [ ] **Cross-platform `cfg`**: pipeline cache is currently Vulkan-only behind
      `wgpu::Features::PIPELINE_CACHE` (verified at `pipeline_cache.rs:24`). The
      quarantine code path runs unconditionally on all three platforms but no-ops when
      the feature is unavailable (DX12 + Metal). Verified all three target_os branches
      have a `cache_dir()` implementation (`pipeline_cache.rs:82` for windows,
      `:97` for non-windows) — no platform left behind.

#### 5.16.6 Surface re-creation per window

- [ ] After `GpuState::recreate` succeeds, walk every `WindowContext` and `DialogWindowContext`
      and call `gpu.create_surface(&window_arc)` to produce a fresh `wgpu::Surface` and
      `wgpu::SurfaceConfiguration`. Store back into the `TermWindow` via a new
      `TermWindow::set_surface(surface, config)` method.
- [ ] On Windows DComp paths: if the new adapter does **not** support DComp but the window
      was created with `WS_EX_NOREDIRECTIONBITMAP`, call
      `oriterm_ui::window::clear_compositor_surface_flag(&window_arc)` (this is already
      used by `init/mod.rs:75` for the first-init fallback). Otherwise the new surface
      cannot present and the window stays invisible after recovery.
- [ ] For dialog windows that share the App `GpuState` but render UI-only, repeat the
      surface rebuild and rebuild the dialog's renderer (UI-only mode). Dialog widget state
      survives — only GPU handles need refreshing.
- [ ] Reconfigure each new surface to the window's current `inner_size()`, not a stale
      cached size. A user resizing the window during recovery must produce a correct
      first-frame size.
- [ ] **Minimized windows still get a surface.** A minimized window has `inner_size()`
      `(0,0)` which `build_surface_config` clamps to `(1,1)` (verified at
      `gpu/state/helpers.rs:165`). Recovery still creates the surface so that when the
      user un-minimizes, the window is immediately renderable. The `TermWindow::has_surface_area`
      predicate keeps the actual render call gated until the size becomes non-zero.
- [ ] **Hidden / not-yet-shown windows.** A window that exists in `App::windows` but has
      `set_visible(false)` (e.g., torn-off-tab destination still being assembled) gets a
      surface; the visibility decision is independent of GPU state.
- [ ] **Per-surface verification.** For each newly created surface, run a single
      lightweight `clear_surface(...)` (already used by `init/mod.rs:235` for the
      first-paint flash prevention) to confirm the new device can actually present to it.
      Failure on this step demotes the *single window* (mark its renderer as
      `Unavailable`-overlay) without dragging the rest of the App down. This handles the
      pathological case where one of N windows landed on a display the new adapter
      cannot drive.

#### 5.16.7 Pipeline + bind group rebuild

- [ ] `GpuPipelines::new(&new_gpu)` builds a new `GpuPipelines`. No new code needed —
      assert that this is the *only* call site outside `App::try_init`.
- [ ] All bind group **layouts** are re-created here, so all bind groups (per-window
      atlas + uniform + image-texture bind groups) must be rebuilt against the new layouts
      in 5.16.8 — old bind groups reference dropped layouts.

#### 5.16.8 Per-window WindowRenderer rebuild

- [ ] For each window, call `WindowRenderer::new(&new_gpu, &new_pipelines, font_collection,
      ui_font_sizes)` exactly as `App::try_init` does, passing the **same** `FontCollection`
      and `UiFontSizes` that survived the teardown. No font re-discovery, no font re-load
      from disk — those are expensive (~50ms each) and unchanged across device loss.
- [ ] Re-apply per-window rendering settings that `try_init` applies after construction:
      `set_subpixel_positioning`, `set_atlas_filtering`, `set_hinting_and_format`,
      `set_font_size` (no, only if DPI changed during recovery — track DPI to compare),
      and the `set_atlas_filtering` call that takes `&pipelines.atlas_layout`.
- [ ] **Glyph atlases start empty.** ASCII pre-cache runs as part of `WindowRenderer::new`
      via `create_atlases`, so the first post-recovery frame has the same warm baseline as
      a fresh launch. Other glyphs lazy-rasterize on first miss using the surviving
      `FontCollection.glyph_cache` CPU bitmaps when present (cache hit) or fresh
      rasterization when absent.
- [ ] Reset every atlas generation counter (`atlas_generation`, `subpixel_atlas_generation`,
      `color_atlas_generation`) — they are tied to the old texture and must restart at the
      new atlas's generation, otherwise `rebuild_stale_atlas_bind_groups` may skip a needed
      rebuild on the first post-recovery frame.
- [ ] Image texture cache (`ImageTextureCache`) starts empty. Inline images (Sixel/iTerm2)
      re-upload on the next extract via `frame_prep.rs::ensure_uploaded` from the surviving
      `oriterm_core::image::Image` CPU bytes. Frame counter resets to 0 — LRU starts fresh.
- [ ] Per-window `pane_cache` and `text_cache` (`WindowContext.pane_cache`, `text_cache`)
      are invalidated. `chrome_scene` is cleared. `root.invalidation_mut().invalidate_all()`
      and `root.mark_dirty()` are called so the next frame is a full repaint.
- [ ] **Re-mark all grid lines dirty** for every active pane via
      `mux.mark_all_dirty(pane_id)` (verified at `app/mod.rs:362` — already used by
      `handle_dpi_change` for the same reason). Without this, the snapshot already-clean
      flag would suppress re-extract until the user types something. The mux is the
      canonical owner of dirty tracking, so the call routes through `MuxBackend` not
      directly to the pane.
- [ ] **Re-trigger UI font prewarm** on the new atlases. Reuse the existing
      `WindowRenderer::new` path which already calls `helpers::prewarm_ui_font_sizes`
      via `create_atlases`. Verify by reading `gpu/window_renderer/mod.rs:158` — the
      prewarm runs unconditionally inside `new`.
- [ ] **Re-inject terminal fallback emoji into UI font collection** — `WindowRenderer::new`
      already does this at `mod.rs:147` via `font_collection.fallback_font_data()` →
      `sizes.inject_fallbacks(&emoji_data)`. Verify it runs on the surviving
      `FontCollection` (it does — the surviving collection still owns the fallback data).
- [ ] **Mark window dirty + force full content re-render** by setting
      `ctx.invalidate_font_caches()` (verified at `app/window_context.rs:158` — the
      same hook used after font config changes). This sets `last_rendered_pane = None`
      and `frame = None` which forces the redraw path to treat the next frame as a
      tab-switch (full content re-extract, no `swap_renderable_content` reuse).
- [ ] **Re-apply hover state** by re-running URL detection on the visible region of
      every active pane (URL hover state is on `WindowContext` but URL targeting depends
      on cell metrics — if DPI changed, hover hit-test needs a refresh). Cheap: just
      call `ctx.url_cache.invalidate()` and let lazy detection re-run on next mouse-move.

#### 5.16.9 Resume strategy — first frame after recovery

- [ ] After `gpu_health` flips to `Healthy`, queue a synthetic `RedrawRequested` for every
      window via `EventSender::send(TermEvent::ForceRedrawAll)` (introduce the variant if
      it does not yet exist; route it through `mark_all_windows_dirty`).
- [ ] The first post-recovery frame uses the **single-pass** render path
      (`render_single_pass`) regardless of `gpu.can_cache_blit()`, because `content_cache`
      is freshly empty and `render_cached`'s "copy cache to surface" path would copy stale
      zeros. After one successful frame, fall back to the configured cached path.
      WHERE: add a one-shot `force_single_pass: bool` field on `WindowRenderer` cleared by
      `render_to_surface` after first successful frame.
- [ ] Any pending GPU error overlay (5.16.10) is dismissed once `gpu_health == Healthy`.
- [ ] **What the user sees during recovery (frame-by-frame contract):**
  1. **Frame N (loss detected):** the in-flight `SurfaceTexture` is dropped, no
     `output.present()` runs. The window's last presented frame stays on the OS
     compositor — Windows DWM, Wayland, and Quartz all keep the prior buffer. So the
     user sees a *frozen* terminal, not a black flash.
  2. **Frames N+1..N+K (during `Recovering`):** all render entry points return
     `RenderOutcome::GatedRecovering`. The OS compositor continues to display the
     frozen image. Cursor blink stops (gated). On Windows DWM may dim or fade the
     window after ~5s of "not responding" — that's a winit/DWM behavior we cannot
     suppress, but it is preferable to a crash.
  3. **First frame after `Healthy`:** uses `force_single_pass=true` (next bullet) and
     repaints the entire window from the surviving CPU state. The user sees the same
     terminal contents (scrollback, selection, cursor at the same row/col, hovered URL
     still highlighted) but with empty atlases lazily refilling — typically zero
     visible difference because ASCII pre-cache covers >95% of normal terminal text.
  4. **First frame +1 onwards:** the configured cached path resumes;
     `force_single_pass` is cleared.
- [ ] **Force a single-pass render on the first post-recovery frame.** Add a one-shot
      `force_single_pass: bool` field on `WindowRenderer` cleared by `render_to_surface`
      after the first successful frame. (Already in the previous bullet; pinning the
      mechanism for clarity.)
- [ ] **Eager glyph re-upload by default** to eliminate the worst-case "lazy fill" hitch
      on the first 1-2 post-recovery frames. Implementation:
      `WindowRenderer::reupload_cached_glyphs(&FontCollection)` walks the surviving
      `FontCollection.glyph_cache` (verified at `font/collection/mod.rs:103`) and
      writes each entry into the appropriate atlas in a single batched upload pass
      after `WindowRenderer::new` returns. The first-frame benchmark test in 5.16.13
      asserts the eager upload completes in **< 5ms** for a typical 1000-glyph cache;
      if any individual run exceeds the budget, the test fails (forcing a profile +
      fix, not a config-flag deferral).
- [ ] **No "frame queue" replay.** We do not buffer extracted frames during recovery and
      replay them after. The terminal state is the truth; the next post-recovery frame
      reads the *current* truth from the surviving snapshots. Replaying stale frames
      would be visually wrong (cursor ghost, late blinks) and a complexity trap.

#### 5.16.10 Backoff + retry + minimal Unavailable UX

- [ ] Backoff schedule (concrete): attempt 1 immediate, attempts 2..=8 at 100ms, 250ms,
      500ms, 1s, 2s, 5s, 5s, 5s after the previous attempt. After 8 failures **or** more
      than 30s total in `Recovering`, transition to `Unavailable`.
- [ ] Backoff timing is computed by a pure function `next_attempt_at(attempt: u8, now:
      Instant) -> Instant` so tests can drive it from an injected clock — never wall-clock
      sleeps in tests.
- [ ] After 30s of continuous `Healthy`, the attempt counter resets to 0 — a single bad
      driver moment doesn't poison future retries.
- [ ] **`OutOfMemory` is terminal — never retried.** OOM means no more VRAM is available;
      retrying immediately re-OOMs. Transition straight to `Unavailable` with a distinct
      message ("Out of GPU memory — close other GPU apps and restart").
- [ ] Permanent adapter loss (the recreate adapter loop returns `NoAdapter` — laptop
      external GPU unplugged, virtual machine GPU disabled): also terminal, `Unavailable`
      with "GPU unavailable. Reconnect a display adapter and restart".
- [ ] **Minimal Unavailable UX — no new dependencies.** When `gpu_health == Unavailable`,
      the user-facing signals are:
  - [ ] **Window title change** for every window in `App::windows` and `App::dialogs`:
        `winit::Window::set_title("ori_term — GPU unavailable, press Ctrl+R to retry")`.
        Restored to the previous title on transition back to `Healthy`. The previous
        title is captured in `App::pre_unavailable_titles: HashMap<WindowId, String>` so
        the restore is exact (a window may have a custom title from `OSC 0`/`OSC 2`).
  - [ ] **Cross-platform user-attention signal** via
        `winit::Window::request_user_attention(Some(UserAttentionType::Critical))` on
        the focused window. Cross-platform: Windows flashes the taskbar, macOS bounces
        the dock icon, Wayland sends an urgent xdg-activation token, X11 sets
        `_NET_WM_STATE_DEMANDS_ATTENTION`. **One-shot** — fired exactly once when
        entering `Unavailable`, not re-fired on every retry attempt (avoid attention
        spam). Implementation: a `bool` flag `unavailable_attention_fired` cleared on
        return to `Healthy`.
  - [ ] **Structured `log::error!`** captured by 5.16.12 with the human-readable reason
        ("GPU driver crashed", "Out of GPU memory", "GPU adapter disconnected") so
        users running with `RUST_LOG=info` see the explanation in stderr.
  - [ ] **No softbuffer, no CPU fallback overlay, no new deps in 5.16.** The OS
        compositor continues to display the last-presented buffer (Windows DWM, Wayland,
        and Quartz all retain it), so the user still sees a frozen but readable terminal.
        The richer in-window overlay (centered card with retry button) is **deferred to
        5.18 as an optional enhancement** behind a `requires-user-approval` gate.
- [ ] Manual retry triggers: `Ctrl+R` (primary), focus regained on a window in
      `Unavailable` state, the app coming out of `Suspended` (winit resume event). Each
      manual retry calls `recover_gpu()` and resets the attempt counter to 0.
- [ ] **Manual retry keybinding** lives in `oriterm/src/keybindings/defaults.rs` as a
      new `Action::GpuRecover` mapped to `Ctrl+R` (chosen because F5 conflicts with
      bash `forward-search-history` and many shells, but Ctrl+R *also* conflicts with
      `reverse-search-history` — see open decision in 5.18). The key handler
      short-circuits the normal terminal key path when `gpu_health == Unavailable` so
      the user cannot accidentally send the keystroke to the shell.
- [ ] **Manual retry from any window.** A multi-window user with one window minimized
      and one focused must be able to retry from the focused one and have *all* windows
      recover. `recover_gpu()` is App-level (not per-window) so the trigger source
      doesn't matter; document this explicitly in the help text.
- [ ] **Unavailable → Healthy transition.** When manual retry succeeds, restore every
      window title from `pre_unavailable_titles`, clear
      `unavailable_attention_fired`, queue a redraw on every window. The frozen
      compositor buffer is replaced by the first post-recovery frame (5.16.9).
- [ ] **`App::dismiss_unavailable_state()` helper** — single canonical exit point so
      both manual retry and successful auto-recovery route through the same code (SSOT
      for the Unavailable→Healthy transition).

#### 5.16.11 Cross-platform semantics

- [ ] **Windows DX12**: device removal arrives via `DXGI_ERROR_DEVICE_REMOVED` /
      `DXGI_ERROR_DEVICE_HUNG` / `DXGI_ERROR_DRIVER_INTERNAL_ERROR`. wgpu surfaces these
      as `SurfaceError::Lost` and via the device-lost callback. Sleep/resume reliably
      triggers this on laptops. DComp swapchains are extra-fragile during display mode
      changes (resolution, lid close) — handle them through the same path.
- [ ] **Windows Vulkan**: fewer device-loss reports but `VK_ERROR_DEVICE_LOST` still
      occurs after long suspends. Vulkan pipeline cache **must** be rebuilt (5.16.5) — a
      pipeline cache from before the driver update will trigger immediate re-loss.
- [ ] **Linux Vulkan**: `VK_ERROR_DEVICE_LOST` on AMDGPU after suspend. NVIDIA proprietary
      driver TDR after long compute timeouts (rare in a terminal but possible if a
      malicious shader is somehow loaded). The same path applies.
- [ ] **macOS Metal**: there is **no** explicit "device lost" in Metal. Lost devices
      manifest as silent rendering corruption or `MTLCommandBufferError.Internal`. We
      cannot proactively detect this — the user will see broken pixels and press F5. The
      manual-retry path in 5.16.10 is the recovery for macOS. Document this in the
      overlay text.
- [ ] **WSLg / virtualised GPUs**: surface loss is common after the host display sleeps.
      The cached surface format must be re-queried because the host surface format may
      change (e.g. WSLg switching between BGRA8 and RGBA8 after host driver swap).

#### 5.16.12 Logging + telemetry

- [ ] Structured `log::error!` on entry to `recover_gpu()`:
      `gpu_health::recover trigger=<surface_lost|callback|other> reason=<reason> attempt=<n>
       windows=<count> backend=<old> adapter=<old_name>`.
- [ ] Structured `log::info!` on successful exit:
      `gpu_health::recover ok backend=<new> adapter=<new_name> elapsed_ms=<n> epoch=<n>`.
- [ ] Structured `log::warn!` on each backoff sleep:
      `gpu_health::recover backoff attempt=<n> next_in_ms=<n>`.
- [ ] Structured `log::error!` on transition to `Unavailable`:
      `gpu_health::unavailable reason=<reason> total_attempts=<n> elapsed_ms=<n>`.
- [ ] Increment a `App::perf::recovery_count` counter exposed in the debug overlay
      (`Ctrl+Shift+F12`). Useful for diagnosing flaky drivers.

#### 5.16.13 Core test strategy — TDD discipline, organized by bug class

Test files and locations follow `.claude/rules/test-organization.md` (sibling `tests.rs`).
This sub-block is the **core engine** test set — only the tests needed to verify 5.16.1–
5.16.12 work. The exhaustive matrices, stress test, preservation invariants, and
cross-section PIN tests live in **5.17** and **5.18**.

**Test matrix dimensions** (every bug class below is a cross-product over at least two of
these axes — vague "add a test" items are forbidden per `.claude/rules/impl-hygiene.md`
"Invariant Explicitness"):

- **Exact failing case** — the concrete input that triggered the real-world bug. For
  5.16 this is "device loss during a laptop lid-close sleep/resume" (WezTerm #7519) and
  "device loss during macOS fullscreen sleep" (#7703). Both must be reachable via the
  synthetic `trigger_test_device_loss` path.
- **Edge cases** — empty state (no panes, no windows), single state (1 window 1 pane),
  boundary state (MAX attempts hit on the same frame as a new loss arrives), size-zero
  state (minimized window during loss).
- **Cross-type coverage** — every `DeviceLostReason` (`Unknown`, `Destroyed`) and every
  `SurfaceError` variant (`Timeout`, `Outdated`, `Lost`, `OutOfMemory`, `Other`) routes
  through the expected path. Adding a new variant breaks the exhaustive `match` in the
  dispatcher — a compile-time pin.
- **Cross-pattern coverage** — every `RenderEntryPoint` (single-window redraw, dialog,
  multi-pane redraw, chrome refresh, debug overlay, search bar, pre-edit overlay) routes
  through the gate exactly once, never bypasses it.
- **Semantic pin** — see the explicit list at the bottom of this sub-block. Every pin
  names the exact assertion that would fail if someone reverted the recovery code.
- **Debug AND release** — every test runs in both profiles. Injected clocks, no
  wall-clock sleeps.

**Bug-class organization:**
1. **Always-on pure tests** — no GPU, no platform deps, run on every CI build.
2. **`gpu-tests` feature** — real adapter, integration tests on llvmpipe/WARP.
3. **`manual-device-loss`** — destructive matrix in 5.18 (lid close, suspend/resume, TDR).

**Failing test first (TDD discipline — non-negotiable per CLAUDE.md):**

- [ ] `oriterm/src/gpu/recovery/tests.rs::recovery_state_machine_blocks_render_when_recovering`
      lands BEFORE any production code in 5.16.2. Construct a `GpuHealth::Recovering`,
      route a redraw through `App::render_dirty_windows`, assert the dispatch returns
      `RenderOutcome::GatedRecovering`. This test must fail compilation (new types) and
      then fail at runtime (wired but no gate) before 5.16.2 plumbs the gate. Test file
      lives at `oriterm/src/gpu/recovery/tests.rs` per `.claude/rules/test-organization.md`
      (sibling `tests.rs`, not inline `mod tests { }`).

**Always-on pure tests** (no GPU, no `cfg` gating):

- [ ] **State-machine transitions** (exhaustive): `Healthy → Recovering → Healthy`,
      `Healthy → Recovering → Unavailable`, `Unavailable → Healthy` (manual retry),
      `Recovering → Recovering` (coalesce), attempt counter increment, 30s healthy
      reset, OOM short-circuit to Unavailable, NoAdapter short-circuit to Unavailable.
      Compile-time exhaustiveness via `#[non_exhaustive]` + `match` covers any new
      variant automatically.
- [ ] **Backoff schedule pin** (semantic test): assert the exact sequence
      `[0, 100, 250, 500, 1000, 2000, 5000, 5000, 5000]` ms via a clock-injected pure
      test. Any schedule change must update the pin (intentional).
- [ ] **Pipeline cache quarantine test** (filesystem only, no GPU): write a corrupt
      cache file with a bad CRC into a `tempfile::tempdir`, call the new
      `load_pipeline_cache`, assert it returns `(None, ...)` and that the file was
      renamed with `.quarantine.<timestamp>.` prefix on the second call. Run the same
      test with a missing sidecar (counter reset after success). Run with a valid
      `pipeline_cache_key` filename + bad CRC AND with a wrong `pipeline_cache_key`
      filename to prove both keying paths trigger quarantine.
- [ ] **OOM is terminal** (pure): simulate `SurfaceError::OutOfMemory` once via the
      synthetic test path, assert `gpu_health == Unavailable` and the attempt counter
      did NOT increment past the OOM event.
- [ ] **Single-flight coalescing**: trigger a synthetic loss while already `Recovering`,
      assert the second event is coalesced (no second `recover_gpu` invocation, no
      double-counter increment).
- [ ] **Idle CPU during Recovering**: assert `compute_control_flow` returns
      `WaitUntil(retry_at)`, not `Poll`. Pin in `event_loop_helpers/tests.rs` so
      regression breaks the build (Performance Invariant 1 — zero idle CPU).
- [ ] **Test-only `trigger_test_device_loss` is gated**: a compile-fence test verifies
      the helper is `#[cfg(any(test, feature = "gpu-tests"))]` and not reachable from
      release builds.
- [ ] **`TermEvent` variants are exhaustive in dispatch**: a compile-time test asserts
      the new `GpuDeviceLost` / `ForceRedrawAll` / `ManualGpuRetry` variants land in the
      single canonical `match` arm in `event_loop.rs::user_event`. Adding a variant
      without updating dispatch fails to compile.

**`gpu-tests` feature tests** (real adapter — llvmpipe on Linux, WARP on Windows):

- [ ] **Device-lost callback wiring**: construct a headless `GpuState`, call
      `device.destroy()`, assert that the registered callback fires and that the
      resulting `TermEvent::GpuDeviceLost` lands in the test event channel. Verifies
      the *plumbing*, not the full recovery.
- [ ] **Full recovery integration — single window**:
      `oriterm/src/gpu/recovery/tests.rs::full_recovery_round_trip_single_window`. Build
      a headless `App`-like harness with one window, render a frame, invoke
      `App::recover_gpu()`, render another frame, assert pixel output matches a
      reference (visual regression style — lives next to
      `oriterm/src/gpu/visual_regression/`).
- [ ] **Full recovery integration — multi-window**:
      `full_recovery_round_trip_multi_window`. Build two windows, render both, recover,
      render both, assert both produce expected pixels and that
      `Adapter::is_surface_supported` was called for **every** surface, not just the
      first (validates the multi-window adapter check from 5.16.4).
- [ ] **Surface vs device loss distinction**: trigger a `SurfaceError::Outdated` —
      assert reconfigure-only path runs (no device recreation, no `recover_gpu` call).
      Trigger a `SurfaceError::Lost` — assert full `recover_gpu` runs. These exercise
      the 5.16.1 distinction between benign reconfigure and full epoch reset.
- [ ] **Resize during recovery**: render thread holds a `SurfaceTexture`, recovery
      triggers, recovery must wait for the in-flight present to complete (or block
      render) before dropping the surface. Assert no panic from `Surface::configure`
      "old SurfaceTexture alive" (verifies the 5.16.3 `debug_assert!` invariant).
- [ ] **Minimized window during recovery** (`gpu-tests`): start with a window at
      `(0, 0)` size, trigger loss, recover, assert recovery completed without panic
      (verifies the `Surface::configure` zero-dimension clamp from 5.16.6).
- [ ] **Cached render path after recovery** (`gpu-tests`): per CLAUDE.md "GPU Render
      Path Testing" — the production render path uses content caching
      (`render_cached`), and bugs in that path are invisible to `render_frame()`.
      After recovery completes, the first frame MUST go through
      `render_frame_cached()` (or the single-pass override path from 5.16.9) against a
      freshly-reconfigured surface whose dimensions may differ from the pre-loss
      viewport (common when DPI changed during recovery). Test: prepare a frame at
      the pre-loss `(w0, h0)`, trigger loss, recover with `(w1, h1)`, call
      `render_frame_cached(&gpu, &pipelines, w1, h1, true)`, assert no panic and
      pixel output is non-zero. Lives in `oriterm/src/gpu/visual_regression/resize_stress.rs`
      next to the existing cached-path resize tests — reuses `gpu.create_copy_dst_target()`.
- [ ] **Every-backend-rejected path** (`gpu-tests`): synthesize an adapter probe
      where every backend (`Vulkan`, `DX12`, `Metal`, `Gl`) returns `None` or fails
      `is_surface_supported`. Assert `GpuState::recreate` returns
      `GpuRecoveryError::NoAdapter` and the state machine transitions straight to
      `Unavailable` with a distinct message (verifies the 5.16.4 "every backend
      rejected" path). Use a `cfg(test)` adapter-probe shim that injects the synthetic
      rejection list — no real driver manipulation required.
- [ ] **Render epoch guard** (`gpu-tests`): bump `WindowRenderer::render_epoch` (5.18.2)
      between `prepare` and `render_to_surface`; assert the render call sees
      `prepared.epoch != renderer.render_epoch` and returns
      `RenderOutcome::Skipped` (stale work discarded). This prevents the pathological
      "prepare finished during recovery" race.
- [ ] **Eager glyph re-upload correctness** (`gpu-tests`): construct a
      `FontCollection` whose `glyph_cache` holds 50 known entries (plus ASCII),
      recreate the renderer via the recovery path, assert every key in the surviving
      CPU cache has a valid `AtlasEntry` in the new atlas before the next
      `render_to_surface` call. Complements the perf assertion in 5.17.4 (the perf
      pin asserts speed; this asserts correctness).

**Cross-platform smoke matrix** (CI):

- [ ] All always-on tests run on Linux, Windows, macOS CI runners.
- [ ] `gpu-tests`-gated tests run on Linux (llvmpipe) and Windows (WARP). macOS
      gpu-tests are deferred to 5.18 (Metal lacks an explicit device-lost path so
      gpu-tests are best-effort smoke only).
- [ ] **Debug AND release**: every recovery test must run in both debug and release
      profiles. Backoff timing tests use injected clocks to avoid wall-clock flakiness
      in CI.
- [ ] **No new clippy warnings, no `unwrap`, no `panic!` in the recovery path** —
      checked by `./clippy-all.sh` post-implementation.

**Semantic pins — the tests that would fail if you reverted 5.16** (per
`.claude/rules/impl-hygiene.md` "Semantic changes require semantic pins"). Each of
these is named because it encodes *new observable behavior* that did not exist before
5.16. Reverting any 5.16 code path must break the corresponding pin; if no pin breaks,
the pin is too weak and must be strengthened:

- [ ] **Render-gate pin**
      (`recovery::tests::recovery_state_machine_blocks_render_when_recovering`) — only
      passes if the gate in `App::render_dirty_windows` returns early when
      `gpu_health != Healthy`. Reverting the gate re-enters submit against a stale
      device → test triggers a panic in the mock `GpuDevice::submit` which panics on
      call. This is the primary I1 pin.
- [ ] **Backoff-schedule pin**
      (`recovery::tests::backoff_schedule_matches_pin_0_100_250_500_1000_2000_5000_5000_5000`)
      — asserts the exact ms sequence. Any "drive-by tweak" to the schedule must
      update the pin intentionally, forcing a review.
- [ ] **Drop-order pin**
      (`recovery::tests::teardown_drop_order_per_frame_then_window_then_dialog_then_surfaces_then_pipelines_then_gpu`)
      — an instrumented counter asserts the order. Reordering breaks the test even in
      debug. Pins the 5.16.3 invariant.
- [ ] **OOM-is-terminal pin**
      (`recovery::tests::oom_transitions_unavailable_without_retry`) — simulating
      `SurfaceError::OutOfMemory` transitions straight to `Unavailable`. Removing the
      OOM short-circuit in 5.16.10 would let it retry and re-OOM — the test asserts
      attempt counter stayed at its pre-OOM value.
- [ ] **Single-flight coalescing pin**
      (`recovery::tests::second_loss_during_recovery_is_coalesced`) — a second
      synthetic loss during `Recovering` must not increment the attempt counter or
      re-invoke `recover_gpu()`.
- [ ] **Idle CPU pin**
      (`event_loop_helpers::tests::control_flow_during_recovering_is_wait_until_backoff`)
      — performance invariant 1 (zero idle CPU). Reverting the gate would produce
      `Poll` instead of `WaitUntil` during recovery and the test breaks the build.
- [ ] **Zero-allocation post-recovery render pin** — after recovery completes and
      the first post-recovery frame runs through `render_frame_cached`, assert no
      new allocations on the hot path. The canonical allocation harnesses live at
      `oriterm_core/tests/alloc_regression.rs` (terminal-side invariants) and the
      per-render hooks live in `oriterm/src/gpu/recovery/tests.rs`
      (`first_post_recovery_frame_zero_alloc`) — the oriterm-side pin uses a
      counting allocator (e.g., `tikv-jemallocator` `stats` or a global wrapper) to
      bracket one call to `prepare()` + `render_frame_cached()` immediately after
      recovery and asserts delta == 0. Performance invariant 2 (zero allocations in
      hot render path) must survive recovery; the pin lives in `oriterm` because it
      depends on `WindowRenderer` which is an `oriterm` type.
- [ ] **Terminal-state preservation pin** — the `stress_50_losses_zero_drift` test in
      5.17.3 is the top-level I2 pin. Naming it here so the 5.16 implementer cannot
      forget that I2 is also their responsibility even though the test lives in 5.17.

#### 5.16.14 Section completion gate (5.16 only)

**Concrete success criteria** (every one is testable; no vague aspirations):

- [ ] All 5.16.1–5.16.13 boxes ticked (this section only — 5.17 and 5.18 have their
      own gates).
- [ ] `./fmt-all.sh` clean.
- [ ] `./clippy-all.sh` clean (no new warnings; recovery path uses no `#[allow]`).
- [ ] `./build-all.sh` succeeds for `x86_64-pc-windows-gnu` debug AND release.
- [ ] `./test-all.sh` passes (all existing 2084 tests + the new 5.16 recovery tests).
      Run with `timeout 150 ./test-all.sh` per CLAUDE.md mandatory test timeout.
- [ ] `cargo test -p oriterm --features gpu-tests recovery::` passes locally on Linux.
- [ ] **No hangs**: a watchdog test asserts `App::recover_gpu()` returns within
      **2 seconds** under normal conditions (synthetic; no real GPU init delays).
- [ ] **Unavailable threshold**: after 8 consecutive synthetic losses with each
      recovery attempt failing in the test harness, `gpu_health == Unavailable`
      arrives at exactly attempt 8 and no later than 30.0s from the first loss
      (synthetic clock).
- [ ] **First-frame latency**: the first post-recovery frame is rendered within
      **150ms** of `gpu_health` flipping to `Healthy` (synthetic clock test).
- [ ] **Performance invariants preserved** (CLAUDE.md "Performance Invariants" — all
      four must continue to pass after 5.16 lands):
  - [ ] Zero idle CPU beyond cursor blink (cursor blink GATED while Recovering — this
        section's contribution to the invariant).
  - [ ] Zero allocations in hot render path (recovery teardown path is cold; no new
        per-frame allocations introduced). Pinned by the 5.16.13 zero-alloc pin.
  - [ ] Stable RSS under sustained output (recovery is one-shot; no growing buffers).
  - [ ] Buffer shrink discipline preserved.
- [ ] **Plan sync (mandatory — every section-complete must do all five):**
  - [ ] Update this section's frontmatter `sections:` entry for `5.16` to
        `status: complete`.
  - [ ] Update `plans/roadmap/index.md` status table row for Section 5 (keep "In
        Progress" until 5.16, 5.17, AND 5.18 are all complete; flip to "Complete"
        only when all three sub-sections are done).
  - [ ] Update any notes in `plans/roadmap/00-overview.md` that reference Section 5
        state (the overview does not currently carry a per-section status column, so
        this step is a no-op unless a future revision adds one — verify before
        marking complete).
  - [ ] Run `./fmt-all.sh`, `./clippy-all.sh`, `./build-all.sh`, and
        `timeout 150 ./test-all.sh` per CLAUDE.md "After EVERY change".
  - [ ] Run `/tpr-review` via the Codex CLI skill and confirm a clean verdict before
        marking 5.16 complete (per CLAUDE.md tpr-review trigger: "proactively after
        completing ANY non-trivial work").

**Codebase hygiene fixes woven into 5.16** (Broken Window Policy — fix everything you
touch):

- [ ] **[BLOAT]** `oriterm/src/gpu/window_renderer/helpers.rs:549` — currently 549
      lines, exceeds the 500-line limit per `.claude/rules/code-hygiene.md` and
      CLAUDE.md "Module Organization". 5.16 must split this file before adding the
      `force_single_pass` field hookup or any other helper. Suggested split: extract
      `prewarm_ui_font_sizes` and related font-size pre-warm helpers into
      `helpers/prewarm.rs`, leaving `helpers.rs` for atlas/font-config plumbing.
- [ ] **[WASTE]** `oriterm/src/window/mod.rs:77` — `#[allow(dead_code, reason = "multi-window constructor for new windows")]` on `TermWindow::new`. Either wire it
      into the multi-window window-creation path being added in 5.16.6 (the recovery
      code path needs to construct surfaces — verify this constructor is on the
      surviving call path and remove the `dead_code` allow), or delete the constructor
      and use `from_window` exclusively. Do not leave dead code annotated.
- [ ] **[EXPOSURE]** `oriterm/src/gpu/window_renderer/error.rs::SurfaceError` collapses
      `Lost` and `Outdated` into a single arm. 5.16.1 already mandates the split, but
      track it as the explicit hygiene fix here too. The current collapse is the
      bug-injection point for the I1 invariant.

**References:**

- WezTerm issues #7519 (Windows D3D11 sleep/resume crash), #7703 (macOS sleep fullscreen).
- Alacritty commit `cd884c9` (Jan 2025) — context-robustness for GPU resets.
- wgpu 28.0.0 `src/api/device.rs:582` — `set_device_lost_callback` signature.
- wgpu-types 28.0.0 `src/lib.rs:564` — `DeviceLostReason` variants.
- wgpu 28.0.0 `src/api/surface_texture.rs:58` — full `SurfaceError` variant set.
- wgpu 28.0.0 `src/util/mod.rs:145` — `pipeline_cache_key`.
- winit 0.30.13 `Window::request_user_attention` — cross-platform user-attention hook.
- Existing oriterm primitives:
  `oriterm/src/gpu/window_renderer/font_config.rs:176` (`clear_and_recache`),
  `oriterm/src/gpu/state/mod.rs:196` (`create_surface`),
  `oriterm/src/gpu/state/pipeline_cache.rs:39` (`fallback: true` already in place),
  `oriterm/src/font/collection/mod.rs:103` (`glyph_cache` CPU-retained bitmaps).

**Priority:** High — affects every laptop user who closes the lid, every desktop user
who installs a driver mid-session, and every WSLg user when the host display sleeps.

---

## 5.17 Recovery Correctness & Infrastructure

**Mission:** Build the structural foundation that makes 5.16's core engine safe and
verifiable. This section owns the `oriterm/src/gpu/recovery/` module layout, the
exhaustive tests for terminal-state preservation invariant **I2** ("the user loses no
terminal state"), the canonical-snapshot stress test, and the exhaustive enum/test-table
coverage that turns "I think we cover all cases" into "the compiler proves we cover all
cases".

**Why split from 5.16:** the core engine in 5.16 is the *runtime path* (detect, tear
down, recreate, recover). 5.17 is the *correctness scaffolding* (enum exhaustiveness,
preservation tests, stress tests). 5.16 can land first and be functionally correct;
5.17 hardens it against future regressions and adds the I2 enforcement layer.

**Dependencies:** 5.16 must be complete before 5.17 begins. 5.17 reuses the
`oriterm/src/gpu/recovery/` module scaffolding and adds tests and invariants on top.

#### 5.17.1 Recovery module scaffolding

Verified directory: `oriterm/src/gpu/` exists today; the new `recovery/` subdirectory
must follow `.claude/rules/test-organization.md` (sibling `tests.rs`) and the 500-line
file limit per file from `.claude/rules/code-hygiene.md`.

- [ ] **`oriterm/src/gpu/recovery/` module** with submodules:
  - [ ] `mod.rs` — re-exports + module docs (`//!` docs explaining the recovery
        contract). Acts as **dispatch hub only** per `.claude/rules/impl-hygiene.md`
        "Module Roles" — no logic bodies, just `mod` declarations and `pub use`.
  - [ ] `health.rs` — `GpuHealth` enum + transitions + epoch counter. **Single source
        of truth for recovery state.** Constructed only via `GpuHealth::new()`,
        mutated only through methods (no public field access).
  - [ ] `outcome.rs` — `RenderOutcome` enum (`Submitted`, `GatedRecovering`,
        `GatedUnavailable`, `Skipped`). Used by 5.16.2 render gate.
  - [ ] `wakeup.rs` — `WakeupSource` enum + `should_post_wakeup` exhaustive predicate.
        Adding a new wakeup source is a compile error if the predicate isn't updated.
  - [ ] `backoff.rs` — `BackoffSchedule` constant slice + `next_attempt_at(attempt: u8,
        now: Instant) -> Instant` pure function. No I/O, no clock reads — caller
        injects `now`.
  - [ ] `errors.rs` — `GpuRecoveryError` enum (`NoAdapter`, `OutOfMemory`,
        `SurfaceRecreate`, `PipelineRebuild`, `RendererRebuild`, `Cascading`).
        `#[derive(Debug)]` + `Display` + `Error` impls. No `unwrap`, no `panic!` —
        full error chain via `source()`.
  - [ ] `tests.rs` — sibling test file per `.claude/rules/test-organization.md`. NO
        inline `#[cfg(test)] mod tests { ... }`. Imports parent items via `super::`.
        Hosts the always-on pure tests for the state machine, backoff, and wakeup
        predicate.
- [ ] **Crate-boundary verification**: every file in `oriterm/src/gpu/recovery/` lives
      in the `oriterm` binary crate per `.claude/rules/crate-boundaries.md` — recovery
      depends on `wgpu::*` types so it cannot live in `oriterm_ui`. The pure-state
      types (`GpuHealth`, `RenderOutcome`, `WakeupSource`, `BackoffSchedule`,
      `GpuRecoveryError`) are pure CPU and could theoretically live in `oriterm_ui`,
      but they're tightly coupled to `App` and `WindowRenderer` so they stay in
      `oriterm` to keep coupling local. Document this rationale in `mod.rs`.
- [ ] **500-line file limit** (CLAUDE.md "Module Organization"): each file in the new
      module must stay under 500 lines. If `health.rs` approaches the limit during
      implementation, split into `health/state.rs` + `health/transitions.rs` + a
      directory module — proactively, not reactively.
- [ ] **`TermEvent` extension** in `oriterm/src/event.rs` (verified — currently 58
      lines, well under 500): new variants `GpuDeviceLost { reason: DeviceLostReason,
      message: String }`, `ForceRedrawAll`, `ManualGpuRetry`. Update the existing
      `match` arms in `event_loop.rs::user_event` (verified single dispatch site).
- [ ] **`App` field additions**: `gpu_health: GpuHealth`,
      `recovery_started_at: Option<Instant>`, `last_recovered_at: Option<Instant>`,
      `recovery_attempt: u8`, `pre_unavailable_titles: HashMap<WindowId, String>`,
      `unavailable_attention_fired: bool`. All non-Option types initialize to
      `Healthy { epoch: 0 }` / `0` / `false` / empty so existing tests don't need
      updates.
- [ ] **`TermWindow::take_surface() -> Option<wgpu::Surface<'static>>`** + paired
      **`TermWindow::set_surface(surface, config)`**. Both methods take `&mut self`
      and update `surface_stale = false` on set. Currently the `surface` field is
      private at `oriterm/src/window/mod.rs:49` with no take/set helper — verified.
- [ ] **`WindowRenderer::force_single_pass: bool`** field, set by 5.16.9, cleared by
      `render_to_surface` after one successful frame. Add a `pub(crate)` getter for
      tests.
- [ ] **`GpuState::recreate(...)` constructor** per 5.16.4. Update the
      `app/init/mod.rs:59` call site comment to point at `recreate` for the recovery
      path so future readers understand the dual-construction surface.
- [ ] **Make `WindowRenderer::clear_and_recache` `pub(crate)`** under the new name
      `reset_gpu_resources` so the recovery path can call it. Verify with
      `cargo clippy -p oriterm` that no test relies on the old private signature.
      *(Note: full recovery in 5.16.3 drops the renderer and constructs a fresh one,
      so `reset_gpu_resources` is only needed for the in-place "warm reset" used by
      synthetic test paths and the future incremental recovery optimization.)*

#### 5.17.2 Terminal-state preservation invariants (I2 enforcement)

This sub-block exists because mission invariant I2 ("the user loses no terminal state")
is the load-bearing user-visible promise. Every check below MUST be enforced by a test
in 5.17.4 — they are not aspirational. Tests live at
`oriterm/src/gpu/recovery/tests.rs` (sibling `tests.rs` per
`.claude/rules/test-organization.md`).

- [ ] **PTY processes are never touched.** The recovery path is forbidden from calling
      any `MuxBackend` method. A test asserts that `App::recover_gpu()` does not
      reference `self.mux` *at all*. Verified mechanism: `oriterm_mux::Pane` IO
      threads run in their own threads; the main thread only reads snapshots.
      Recovery does not block the main thread long enough to drop a snapshot, and even
      if it did, the IO thread's circular snapshot buffer would catch up on the next
      swap.
- [ ] **Selection state survives.** Test: drag-select a region, trigger loss, recover,
      assert `App::pane_selections.get(pane_id) == before`.
- [ ] **Mark cursor state survives.** Symmetric: enter mark mode, position cursor,
      trigger loss, recover, assert `App::mark_cursors.get(pane_id) == before`.
- [ ] **Search state survives.** If a search is active, the search query string,
      direction, and current match position must persist. Search state lives on
      `WindowContext`/`WindowRoot` (CPU) so it survives by construction; the test
      pins this so a future refactor that moves search state into a `wgpu::Buffer`
      would fail loudly.
- [ ] **Hyperlinks survive.** Hyperlink IDs in `Cell` are CPU; the URL detection cache
      is cleared but the *underlying cells* still carry their `Hyperlink` reference
      so re-detection produces the same hits.
- [ ] **Inline images survive.** Image bytes live in `oriterm_core::image::Image`
      (CPU, verified). Recovery clears the GPU image texture cache; first
      post-recovery frame re-uploads. Test inserts an image, triggers loss, recovers,
      asserts the image pixel-matches a reference.
- [ ] **Cursor position survives.** Cursor row/column lives on `Term` in the IO
      thread; survives by construction. Test asserts `last_cursor_pos` matches before
      and after.
- [ ] **Clipboard contents survive.** Clipboard is OS-owned for cross-app copies;
      internal `Clipboard` state is CPU. Test pins.
- [ ] **Config + bindings survive.** No-op test (pure CPU) — pinned for refactor
      safety.
- [ ] **IME composition survives.** A user mid-CJK-composition during device loss
      must not lose the in-progress preedit string. `App::ime: ImeState` is CPU.
      Test pins.
- [ ] **Tab + window topology survives.** `SessionRegistry` is CPU. Tabs, splits,
      floating-pane positions all survive. Test creates a 2x2 split with a floating
      pane, triggers loss, recovers, asserts the topology snapshot is identical.
- [ ] **Pane focus survives.** `active_window` and `focused_window_id` are CPU.
- [ ] **Drag-in-progress is canceled cleanly.** A divider/floating/tab drag in
      progress when loss arrives is *canceled* (drag state cleared), not silently
      completed. Reason: the drag's hit-test was computed against pre-loss layout;
      completing it against post-recovery layout (which may differ if DPI changed)
      would land the target somewhere unexpected. Test: start a divider drag, trigger
      loss, recover, assert `WindowContext.divider_drag.is_none()` and that the
      divider position is unchanged.
- [ ] **Mouse-cursor-hidden state survives.** `App::mouse_cursor_hidden` survives so
      the typing-auto-hide doesn't reset on every recovery.

#### 5.17.3 Canonical-snapshot stress test (50-loss zero-drift)

The original 5.16 plan included a "byte-identical scrollback after 50 losses" test.
Codex flagged this as too weak: it can pass while attribute flags, wide-char
continuations, hyperlink anchors, or selection state silently drift. Replace with a
**canonical terminal-state snapshot** that captures every CPU-side terminal field that
must be invariant across recovery.

- [ ] **`CanonicalTerminalSnapshot` newtype** in
      `oriterm/src/gpu/recovery/tests.rs` (test-only) capturing:
  - [ ] `cells: Vec<Cell>` — every visible + scrollback cell, with all `CellFlags`
        (BOLD, ITALIC, UNDERLINE, STRIKEOUT, INVERSE, HIDDEN, BLINK, DIM, WIDE_CHAR,
        WIDE_CHAR_SPACER, WRAPLINE).
  - [ ] `wide_char_continuations: Vec<(Line, Column)>` — explicit positions of every
        wide-character continuation cell so a refactor that confuses width-1 vs
        width-2 cells fails the test.
  - [ ] `cursor: (Line, Column, CursorStyle, bool /* visible */)`.
  - [ ] `viewport: (u32, u32)` — cell grid dimensions.
  - [ ] `selection: Option<SelectionRange>` for every pane.
  - [ ] `hyperlinks: HashMap<HyperlinkId, Hyperlink>` — every active hyperlink anchor.
  - [ ] `scroll_offset: usize` — display_offset from the bottom.
  - [ ] `semantic_zones: Vec<SemanticZone>` — shell prompt / output / command marks
        if available.
  - [ ] `active_modes: ModeFlags` — alternate screen, bracketed paste, mouse mode,
        cursor key mode, application keypad.
  - [ ] `palette_mods: Vec<(u8, Rgb)>` — any OSC 4 dynamic palette overrides.
- [ ] **`stress_50_losses_zero_drift` test** in
      `oriterm/src/gpu/recovery/tests.rs`:
  1. Spawn a pane, write 10000 lines of mixed content (CJK, emoji ZWJ sequences,
     SGR attributes, OSC 8 hyperlinks, alternate screen toggle, selection drag).
  2. Capture `CanonicalTerminalSnapshot::from(&app)`.
  3. Loop 50 times: `app.trigger_test_device_loss(); app.recover_gpu(); render_frame();`
     with random 0-500ms gaps (synthetic clock).
  4. Capture `CanonicalTerminalSnapshot::from(&app)` again.
  5. `assert_eq!(snapshot_before, snapshot_after);` — single equality on the full
     canonical snapshot, NOT just byte-equal scrollback.
- [ ] **Eq impl for `CanonicalTerminalSnapshot`** is `derive(PartialEq, Eq)` only —
      no custom logic that could mask drift. Adding a new terminal-state field forces
      a new struct field which the derive picks up automatically (compile-time
      enforcement of "I2 covers everything").
- [ ] **Test runs in BOTH debug AND release** (CLAUDE.md "Debug AND release: every
      recovery test must run in both"). Lives in
      `oriterm/src/gpu/recovery/tests.rs` and is part of the always-on suite (not
      gpu-tests gated — the synthetic loss path doesn't need a real adapter).

#### 5.17.4 Exhaustive enum and test-table coverage

Compile-time exhaustiveness via Rust's `match` is the strongest enforcement mechanism
per `.claude/rules/impl-hygiene.md` "Enforcement mechanisms". Where the language
provides exhaustiveness for free, use it. Where it doesn't (test tables), iterate the
canonical enum at test-time.

- [ ] **`WakeupSource` exhaustiveness**: for each `WakeupSource` enum variant (cursor
      blink, text blink, tab slide, layer animator, render scheduler, visual state
      animator, auto-scroll, hover hold, mux pump), construct `should_post_wakeup`
      with each `GpuHealth` state and assert the expected boolean. Adding a new
      wakeup source forces a new exhaustive arm — guaranteed by exhaustive `match`
      in the predicate body (not just at test time).
- [ ] **`RenderOutcome` exhaustiveness**: a test iterates every `RenderOutcome`
      variant and asserts the dispatch logic in `App::render_dirty_windows` returns
      it for the corresponding `GpuHealth` state. Use `#[non_exhaustive]` on the enum
      so external dispatch is forced through the helper.
- [ ] **`GpuRecoveryError` exhaustiveness**: a test verifies every `GpuRecoveryError`
      variant has a non-empty `Display` impl AND a corresponding `log::error!` arm
      in 5.16.12's structured logging. The test iterates the enum variants via a
      const slice (compile-time enforcement: adding a variant breaks the slice).
- [ ] **Render entry-point gating exhaustiveness** (pure test): construct a
      `GpuHealth::Recovering` and a mock dispatcher; assert `render_dirty_windows`,
      `handle_redraw`, `render_dialog`, and the multi-pane redraw all return
      `RenderOutcome::GatedRecovering` without touching any GPU type. Use a
      `MockGpu` stub that panics on any method call so the test fails loudly if the
      gate leaks.
- [ ] **Drop ordering invariant test**: a debug-only test asserts the drop order
      `per_frame → per_window → per_dialog → window_surfaces → pipelines → gpu_state`
      via a counter on each helper. Reordering breaks the test.
- [ ] **Capability-change recovery** (`#[cfg(feature = "gpu-tests")]`): construct two
      `GpuState`s with different `dual_source_blending` settings (use `Features`
      toggle on the device descriptor). Verify the rebuilt `GpuPipelines` matches the
      new capability and the subpixel pipeline shape is correct.
- [ ] **DPI change during recovery**: synthesize a scale-factor change between the
      teardown and rebuild steps, assert the new `WindowRenderer` has the new
      physical DPI on its `FontCollection`.
- [ ] **Manual retry from Unavailable**: trigger 8 consecutive losses, assert
      `Unavailable`, fire `TermEvent::ManualGpuRetry`, assert the recovery attempt
      counter resets and `Recovering` is entered.
- [ ] **Ctrl+R keybinding in Unavailable state**: construct an `App` in `Unavailable`,
      send Ctrl+R via the keymap dispatch, assert `recover_gpu()` is invoked AND
      that the key did NOT reach the terminal pane (no PTY write).
- [ ] **Frame contract verification** — the user-visible "what does the user see"
      contract from 5.16.9: render frame, trigger loss mid-frame, assert no
      `output.present()` runs after loss; fully recover; render next frame; assert
      it uses `force_single_pass=true` exactly once.
- [ ] **OOM does NOT reset attempt counter**: synthesize OOM at attempt 3, assert
      transition straight to `Unavailable` (skips the remaining attempts).
- [ ] **Drag canceled on loss**: start a divider drag, trigger loss, recover, assert
      `WindowContext.divider_drag.is_none()`.
- [ ] **Mux pump unaffected by recovery**: spawn a pane that emits 1MB of output per
      second, trigger loss, recover, assert the output was fully consumed by `Term`
      (snapshot row count matches expected).
- [ ] **Eager glyph re-upload performance**: construct a `FontCollection` with 1000
      cached glyphs, run `WindowRenderer::reupload_cached_glyphs`, assert wall time
      < 5ms in release builds (debug builds skip the assertion).

#### 5.17.5 Cascading-loss-in-recovery handling

If the *recovery itself* triggers a second device loss inside `GpuPipelines::new` or
`WindowRenderer::new`, that strongly implies the pipeline cache file is poisoned.

- [ ] Catch the cascading loss and transition straight to the quarantine path in
      5.16.5 (rename to `<pipeline_cache_key>.quarantine.<timestamp>`).
- [ ] Retry the *current* recovery attempt **once** with no cache. If that also
      fails, increment the attempt counter and go through normal backoff.
- [ ] Test (in `oriterm/src/gpu/recovery/tests.rs`): synthesize a second loss inside
      the test harness's `GpuPipelines::new` mock, assert the pipeline cache is
      quarantined and one retry without cache is attempted. The test uses a fake
      `PipelineCache` adapter that panics on the first call and succeeds on the
      second.
- [ ] Pipeline cache fail-fast must NOT retry indefinitely. After the cache-less
      retry also fails, the cascading-loss path joins normal backoff.

#### 5.17.6 Section completion gate

- [ ] All 5.17.1–5.17.5 boxes ticked.
- [ ] `./fmt-all.sh` clean.
- [ ] `./clippy-all.sh` clean (no new warnings; tests use no `#[allow]`).
- [ ] `./build-all.sh` succeeds for `x86_64-pc-windows-gnu` debug AND release.
- [ ] `./test-all.sh` passes with `timeout 150 ./test-all.sh`. The
      `stress_50_losses_zero_drift` test must complete within the 150s budget on the
      slowest CI runner.
- [ ] `cargo test -p oriterm recovery::` passes locally on Linux.
- [ ] `cargo test -p oriterm --features gpu-tests recovery::` passes locally.
- [ ] **All terminal-state preservation tests in 5.17.2 pass.**
- [ ] **`stress_50_losses_zero_drift` passes** on the canonical snapshot, not just
      byte-equal scrollback.
- [ ] **Exhaustiveness** verified by compilation: adding a new `WakeupSource`,
      `RenderOutcome`, or `GpuRecoveryError` variant without updating consumers
      fails to compile.
- [ ] **Plan sync (mandatory):**
  - [ ] Update this section's frontmatter `sections:` entry for `5.17` to
        `status: complete`.
  - [ ] Update `plans/roadmap/index.md` status table row for Section 5 (remains "In
        Progress" until 5.18 also lands).
  - [ ] Run `./fmt-all.sh`, `./clippy-all.sh`, `./build-all.sh`, and
        `timeout 150 ./test-all.sh`.
  - [ ] Run `/tpr-review` and confirm clean before marking 5.17 complete.

---

## 5.18 Recovery Integrations & Deferred Contracts

**Mission:** Cross-section integration audit, daemon-mode integration, the manual
destructive test matrix, the optional enhanced Unavailable UX (softbuffer), and items
requiring user approval. **None of these block 5.16 or 5.17 from landing.** Each item
is either:

1. A **PIN test** that asserts an invariant later sections must preserve (lands now,
   protected by `#[cfg(feature = "future-section-N")]` or `#[ignore]` until that section
   exists).
2. A **deferred follow-up** that lands when its target section lands, marked with
   `<!-- depends-on:N -->`.
3. A **user-approval gate** that needs explicit sign-off before any code lands.

**Why split from 5.16/5.17:** 5.16 and 5.17 must be implementable today against the
current codebase. Cross-section contracts referencing Sections 6, 7, 12, 17, 19, 23, 32,
34, 39, 43, 50 are mostly later tiers — folding them into 5.16 would block Tier 2 on
unlanded Tier 4-7 work. Codex's verdict: hard blockers stay in 5.16/5.17, PIN tests
land here, follow-up work lands when the target section lands.

**Dependencies:** 5.16 + 5.17 must be complete before 5.18 begins. Some 5.18 items
remain `not-started` indefinitely — they wait for their target section to land.

#### 5.18.1 Cross-section integration audit (PIN tests + forward contracts)

Device loss touches every system that holds a wgpu handle, plus several that *don't*
but could regress to holding one in future sections. Each item below is classified as:

- **PIN test**: lands in `oriterm/src/gpu/recovery/tests.rs` now, may use `#[ignore]`
  or feature gating until the depended-on section exists.
- **Follow-up**: lands when the depended-on section's plan is implemented; the
  obligation is recorded in that section's plan file with a `relates-to:5.18` comment.

<!-- depends-on:6 -->
- [ ] **Section 6 (Font Pipeline) — PIN test**: `FontCollection.glyph_cache` retains
      CPU bitmaps across recovery; verified at `font/collection/mod.rs:103`. PIN test
      asserts that `FontCollection::glyph_cache.is_empty() == false` before AND after
      recovery (i.e., bitmaps are NOT cleared). If a future Section 6 refactor moves
      glyph storage to a GPU-only structure, this test fails loudly. **Status:**
      lands now (always-on).

<!-- depends-on:7 -->
- [ ] **Section 7 (2D UI Framework) — PIN test**: `WindowRoot` and all its members
      are pure CPU per `.claude/rules/crate-boundaries.md`. PIN test uses
      `static_assertions` (or a manual compile-time check) to assert `WindowRoot`
      contains no `wgpu::*` type. **Status:** lands now (always-on).
- [ ] **Section 7 — Forward contract**: future widgets that want a "GPU shader
      effect" must keep the *state* in CPU and re-create the shader resources via
      the recovery path — they cannot own a `wgpu::Device` directly. Document this
      in `oriterm_ui/src/widgets/README` (or its mod docs) when widgets land.
      **Status:** follow-up.

<!-- depends-on:12 -->
- [ ] **Section 12 (Resize & Reflow) — Follow-up**: an in-progress resize when loss
      arrives is handled like any other resize: the new size lands on
      `TermWindow.surface_config` via `resize_surface`. Recovery uses the *current*
      (post-resize) size for the new surface. If the resize is mid-drag
      (`divider_drag` or window-edge resize), the drag state is canceled per 5.17.2.
      Reflow itself runs on the IO thread (verified `oriterm_mux/src/pane/io_thread/`)
      so it is unaffected by GPU loss. **Status:** Section 12 already landed; verify
      this contract holds in the existing reflow tests. Add a regression case if
      missing.

<!-- depends-on:17 -->
- [ ] **Section 17 (Drag & Drop) — Follow-up**: an in-progress drag-and-drop (file
      drop, text drop, URL drop) when loss arrives is *not* canceled — DnD is owned
      by the OS compositor (winit forwards events) and completes via the OS event
      queue. Section 17 must add: start a file drag (mock the winit event), trigger
      loss, complete the drop, assert the drop reaches the terminal. **Status:**
      lands when Section 17 lands.

<!-- depends-on:19 -->
- [ ] **Section 19 (Event Routing) — DRIFT prevention**: 5.17.1 introduces
      `TermEvent::GpuDeviceLost`, `TermEvent::ForceRedrawAll`, and
      `TermEvent::ManualGpuRetry` variants. Section 19's event-routing tests must
      include these in the exhaustive variant match. **Status:** the variants land
      with 5.17.1; the exhaustive match is enforced at compile time via Rust's
      pattern exhaustiveness (no test needed for that), but Section 19's documentation
      tests should reference the new variants.

<!-- depends-on:23 -->
- [ ] **Section 23 (Damage Tracking) — Follow-up**: damage tracker state
      (`DamageSet`, `DamageTracker`) is CPU and survives. Recovery resets it via
      `damage_mut().reset()` per 5.16.3, forcing a full repaint on the first
      post-recovery frame. Section 23 tests must include a "damage survives reset"
      test to pin this contract. **Status:** lands when Section 23 lands.

<!-- depends-on:32 -->
- [ ] **Section 32 (Tab/Window Management) — Forward contract**: multi-window
      recovery is the single-flight path in 5.16.2. Dialogs (Settings, Confirmation,
      About) recover via `drop_per_dialog_gpu_state()` per 5.16.3. Section 32 must
      not introduce a side-channel that creates `wgpu::Surface` outside
      `App::recover_gpu()`'s view — every surface goes through
      `GpuState::create_surface` which is the single construction site (SSOT).
      **Status:** lands when Section 32 lands.

<!-- depends-on:39 -->
- [ ] **Section 39 (Image Protocols) — PIN test + follow-up**: image bytes are
      retained in `oriterm_core::image::Image` (verified). Recovery drops the GPU
      `ImageTextureCache` and re-uploads on first frame. **PIN test now**: insert a
      mock image, trigger loss, recover, assert the image bytes survive on the CPU
      side. **Follow-up**: Section 39's image insertion tests must include a recovery
      round-trip case once Section 39's full image-protocol pipeline lands.

<!-- depends-on:43 -->
- [ ] **Section 43 (Compositor Layer System) — Forward contract**: `LayerTree` and
      `LayerAnimator` are pure CPU (verified — no `wgpu::*` types in
      `oriterm_ui/src/compositor/`). When Section 43 introduces GPU-backed layers,
      those layer textures must register with a layer-recovery hook called by
      `App::recover_gpu()` after `GpuPipelines::new` and before `WindowRenderer::new`.
      Pin this contract here so the future Section 43 plan inherits the obligation.
      **Status:** lands when Section 43 lands.

<!-- depends-on:50 -->
- [ ] **Section 50 (Runtime Efficiency) — PIN test**: ControlFlow during recovery is
      `WaitUntil(retry_at)` where `retry_at` is the next backoff point. **Recovery
      must not busy-loop.** Verified path: `compute_control_flow` reads `gpu_health`
      and returns the appropriate `WaitUntil`. This is already pinned in 5.16.13's
      core test set; 5.18 adds the cross-section assertion that Section 50's
      `compute_control_flow` test suite includes the recovery branch. **Status:**
      lands now (always-on, pinned in `event_loop_helpers/tests.rs`).

#### 5.18.2 Daemon mode integration (Section 34)

<!-- depends-on:34 -->

In daemon mode (Section 34), the daemon owns PTYs and the client owns the GPU. Device
loss is *client-side only* — the daemon keeps running and continues mutating terminal
state via the IPC stream. This sub-block enumerates the explicit contracts the daemon
recovery path must honor.

- [ ] **Mux/IPC updates continue mutating terminal state during recovery.** The
      client-side `MuxBackend` (daemon variant) talks to the daemon over `oriterm_ipc`
      which is CPU/IO and unaffected by GPU loss. Recovery does NOT pause the IPC
      stream — pane snapshots continue arriving from the daemon and continue updating
      the client's `Term`/`Grid` state. Only the *render* path is gated, not the
      *state mutation* path.
- [ ] **Recovery redraws from latest model snapshot.** When recovery completes, the
      first post-recovery frame reads the *latest* snapshot from the IPC stream, not
      a stale snapshot captured before loss. This is automatic because the IO thread
      keeps writing to `SnapshotDoubleBuffer`; the main thread just consumes the
      latest on the next frame.
- [ ] **Render-generation / epoch guard prevents stale render work from committing
      post-recovery.** Add a `render_epoch: u64` field on `WindowRenderer`. Bumped
      on every successful `App::recover_gpu()`. Any in-flight prepare/extract work
      from before the loss carries the old epoch; the post-recovery render path
      checks `prepared.epoch == renderer.render_epoch` before submitting and
      discards stale work. This prevents the pathological case where prepare started
      before loss, finished during recovery, and tries to commit against a renderer
      that no longer exists.
- [ ] **Section 34's reconnection tests must include a "GPU loss does not drop daemon
      connection" case.** A multi-stage test: connect to daemon, render some output,
      trigger client-side device loss, recover, send a keystroke to the daemon, assert
      the daemon received the keystroke (the IPC connection survived). **Status:**
      PIN test lands now using a mock daemon backend; full integration test lands
      when Section 34 lands.

#### 5.18.3 Image protocol re-upload (Section 39 contract)

<!-- depends-on:39 -->

- [ ] **First frame after recovery re-uploads inline images** from
      `oriterm_core::image::Image` CPU bytes via `ImageTextureCache::ensure_uploaded`
      (verified at `gpu/image_render/mod.rs:75`). **Contract**: every image visible
      in the first post-recovery frame must render correctly without a one-frame
      blank flash. **Test (PIN, lands now)**: insert a mock image, render, trigger
      loss, recover, render, assert the image is present in the post-recovery frame's
      instance buffer. **Follow-up**: when Section 39's full Sixel/iTerm2 pipeline
      lands, add an end-to-end pixel test.

#### 5.18.4 Compositor layer recovery hook (Section 43 contract)

<!-- depends-on:43 -->

- [ ] **`App::recover_gpu()` calls a layer-recovery hook** between `GpuPipelines::new`
      and `WindowRenderer::new`. The hook is a pub trait method on a future
      `LayerSystem` trait that Section 43 will implement. Today there is no
      `LayerSystem`, so this item is purely a forward contract. Mark Section 43's
      plan with `relates-to:5.18` so the implementer doesn't forget. **Status:**
      lands when Section 43 lands.

#### 5.18.5 Optional enhanced Unavailable UX via softbuffer

<!-- requires-user-approval: RESOLVED 2026-04-06 — DECLINED -->

5.16.10 ships the **minimal** Unavailable UX (window title change + structured log +
one-shot `request_user_attention`). This sub-block is the **optional enhancement** that
draws an in-window overlay via the `softbuffer` crate. It is **NOT required** to
satisfy the mission "no crash, recover seamlessly, preserve state" — the OS compositor
already retains the last-presented buffer so the user sees a frozen-but-readable
terminal during recovery.

**Resolution (2026-04-06): DECLINED.** The user reviewed this sub-block during the
5.16 plan review (see 5.18.8) and chose not to add `softbuffer` to the project. 5.16's
minimal Unavailable UX (window title text + native attention request + structured log)
is the final state for permanent GPU failure. This sub-block stays `not-started`
indefinitely as the "declined + closed" state. Revisit only if user feedback shows
the minimal UX is too thin in practice.

- [x] **softbuffer dependency declined** — 2026-04-06.
- [x] **`gpu-fallback` feature flag declined** — 2026-04-06 (paired decision).
- [x] **No `oriterm/src/gpu/recovery/cpu_overlay/` module** — declined.
- [x] **5.18.5 closed** as `not-started` permanently. 5.18.9 plan-sync notes will
      reflect "declined" rather than "complete".

#### 5.18.6 Manual destructive test matrix

These tests CANNOT be automated — they require physical hardware actions or destructive
driver-level operations. Document instructions for human testers; do not block CI on
them. Run manually before each 5.16+ release.

- [ ] **Windows: D3D12 device removal via Device Manager.** Launch on Windows, force
      device removal via `dxcap.exe --recover` or by toggling Device Manager
      "Disable" / "Enable" on the GPU. Verify successful recovery and continued
      typing in the terminal. Verify scrollback survives by `Ctrl+Home`-scrolling
      before the trigger and confirming the same content is at the top after
      recovery.
- [ ] **Windows: lid close on a laptop.** Close the lid for ≥30s, reopen, verify
      recovery (or graceful `Unavailable` with manual Ctrl+R retry).
- [ ] **Linux: host suspend/resume.** Launch on Linux, suspend the host (`systemctl
      suspend`), resume, observe successful recovery (or graceful `Unavailable` with
      manual retry).
- [ ] **macOS: laptop sleep/wake.** Launch on macOS, sleep the laptop, wake. Because
      Metal lacks an explicit device-lost path (5.16.11), this test verifies the
      *manual retry* path: corrupt output is detected by the user, Ctrl+R is
      pressed, recovery succeeds.
- [ ] **Plug-pull test (laptop with discrete + integrated GPU).** App running on
      discrete, switch GPU at runtime via NVIDIA control panel ("force integrated").
      Observe recovery onto integrated GPU. Verify capability re-negotiation
      (`dual_source_blending` may flip).
- [ ] **WSLg suspend/resume.** WSL host display sleeps, observe recovery on host
      wake.
- [ ] **GPU TDR simulation (Windows).** Use `nvidia-smi --gpu-reset` (admin) or the
      Microsoft TDR registry tweak to force a GPU timeout. Verify recovery.
- [ ] **External display unplug.** Run on a window placed on an external monitor,
      unplug the external monitor, verify the window migrates back to the laptop
      display and recovery succeeds.

#### 5.18.7 Cross-platform CI smoke matrix (separate from manual destructive)

- [ ] **Linux CI**: all always-on tests + `gpu-tests`-gated tests on llvmpipe.
- [ ] **Windows CI**: all always-on tests + `gpu-tests`-gated tests on WARP.
- [ ] **macOS CI**: all always-on tests; gpu-tests are best-effort smoke (Metal lacks
      device-lost so the integration tests are functionally identical to a normal
      render path).
- [ ] **Boot test on each platform**: launch oriterm, render one frame, exit cleanly
      — proves the recovery infrastructure additions did not break the cold-start
      path.
- [ ] **Recover-once smoke on Linux+Windows**: boot, trigger one synthetic loss,
      recover, render again, exit cleanly — proves the full recovery path works on
      real adapters in CI.

#### 5.18.8 Items requiring user approval before implementation

These items need explicit user sign-off because they introduce a dependency, a feature
flag, or a public API change. Resolve these via `AskUserQuestion` before starting the
corresponding sub-block. **None block 5.16 or 5.17 implementation.**

- [x] **`softbuffer = "0.4"` dependency** (gates 5.18.5).
      **Resolved 2026-04-06: DECLINED.** 5.16's minimal Unavailable UX (window title
      change + structured log + `request_user_attention(Critical)`) is sufficient.
      5.18.5 stays `not-started` indefinitely as the "declined + closed" state. Can
      be revisited if user feedback shows the minimal UX is too thin.
- [x] **`gpu-fallback` feature flag** (gates 5.18.5).
      **Resolved 2026-04-06: DECLINED** (paired with softbuffer decision).
- [x] **`gpu-tests` feature flag** (gates the `gpu-tests` test set in 5.16.13 and
      5.17.4).
      **Resolved 2026-04-06: APPROVED — already exists** in `oriterm/Cargo.toml`
      `[features]` block. No new flag needed; reuse the existing one.
- [x] **Manual GPU retry keybinding** (gates 5.16.10).
      **Resolved 2026-04-06: F5 only.** F5 is conventional reload across browsers/IDEs,
      currently unbound in `oriterm/src/keybindings/defaults.rs`, and does not
      conflict with shell keybindings. `Ctrl+R` is rejected because it conflicts
      with bash `reverse-search-history`.
- [x] **Default attempt count (8) and total time cap (30s)** (gates 5.16.10).
      **Resolved 2026-04-06: APPROVED.** Schedule
      `[100, 250, 500, 1000, 2000, 5000, 5000, 5000]` ms = ~14s of backoff plus
      recovery work. Reset to attempt 0 after 30s of clean operation.
- [x] **`TermEvent` variant additions** (`GpuDeviceLost`, `ForceRedrawAll`,
      `ManualGpuRetry`) (gates 5.17.1).
      **Resolved 2026-04-06: APPROVED.** Mechanical infrastructure for dispatching
      the `set_device_lost_callback` payload and the F5 retry into the event loop.
- [x] **`TermWindow::take_surface` / `set_surface` methods** as new API on
      `TermWindow` (gates 5.17.1).
      **Resolved 2026-04-06: APPROVED.** Required to drop a window's
      `wgpu::Surface<'static>` during teardown and reattach a fresh one after
      `GpuState::recreate()`. Surface lifecycle change is necessary; alternative
      (recreating the entire `TermWindow`) would discard non-recreatable per-window
      state.
- [x] **`CLAUDE.md` "Performance Invariants" addition** (gates 5.16.14): add a fifth
      bullet "**Bounded recovery cost.** Device loss triggers single-flight recovery
      with exponential backoff capped at 8 attempts / 30s. While Recovering, idle
      CPU stays at the cursor-blink baseline (i.e., zero — blink is gated)."
      **Resolved 2026-04-06: APPROVED** with the wording above. Add to `CLAUDE.md`
      `## Performance Invariants` block as the fifth bullet during 5.16.14
      implementation.

#### 5.18.9 Section completion gate

5.18 may remain `not-started` for an extended period — many of its items wait on
later sections. The gate is satisfied incrementally as each sub-block lands.

- [ ] All 5.18.1 PIN tests that can land today are landed (Section 6 PIN, Section 7
      PIN, Section 39 PIN, Section 50 PIN). Follow-up items remain `not-started`
      until their target section lands.
- [ ] 5.18.2 daemon-mode integration: PIN tests landed, full integration deferred to
      Section 34 implementation.
- [ ] 5.18.5 softbuffer overlay: status reflects user decision (approved + landed,
      or declined + closed).
- [ ] 5.18.6 manual destructive matrix: documented and run at least once on each
      platform before declaring 5.18 complete.
- [ ] 5.18.7 CI smoke matrix: all required CI workflows updated.
- [ ] 5.18.8 user approvals: each item resolved before its dependent sub-block lands.
- [ ] **Plan sync (mandatory):**
  - [ ] Update this section's frontmatter `sections:` entry for `5.18` to
        `status: complete` once the landable items are complete (follow-up items
        that wait on later sections stay tracked here but do not block the status
        flip — they carry `<!-- depends-on:N -->` markers and move to their target
        section's plan when that section lands).
  - [ ] Update `plans/roadmap/index.md` status table row for Section 5: once 5.16,
        5.17, and 5.18 are all `complete` AND 5.15 (whole-section gate) is green,
        flip Section 5 from "In Progress" to "Complete".
  - [ ] Run `./fmt-all.sh`, `./clippy-all.sh`, `./build-all.sh`, and
        `timeout 150 ./test-all.sh`.
  - [ ] Run `/tpr-review` and confirm clean before marking 5.18 complete.

**Note:** 5.18 may legitimately stay in `in-progress` for several roadmap iterations as
later sections land their pieces. That is by design — 5.18 is the canonical home for
recovery contracts that can't be fully implemented until later sections exist.

---

## 5.15 Section Completion (whole-section gate)

This is the **whole-section gate** for Section 5. It exits the section only after all
sub-sections (5.1–5.14, 5.16, 5.17, 5.18) are complete. The 5.1–5.14 portion was
verified complete on 2026-03-29 and remains green; the gate is now blocked on the
recovery sub-sections.

**5.1–5.14 completion (verified 2026-03-29):**

- [x] All 5.1–5.14 items complete (one 5.13 item blocked-by:9 — selection overlay) (verified 2026-03-29)
- [x] **Pipeline architecture:**
  - [x] Extract → Prepare → Render phases are cleanly separated
  - [x] No function crosses phase boundaries
  - [x] Prepare phase has zero wgpu imports
  - [x] Render phase accepts any `TextureView` (surface or offscreen)
- [x] **Testing:**
  - [x] Prepare phase unit tests pass (instance buffer correctness, counts, colors, determinism)
  - [x] Headless GPU integration tests pass (pipeline creation, offscreen render, pixel readback)
  - [x] Visual regression test infrastructure exists (even if initial reference set is small)
- [x] **Functional:**
  - [x] Binary launches, window appears, terminal grid renders <!-- unblocks:3.8 -->
  - [x] Shell is functional: can type commands and see output
  - [x] Colors render correctly
  - [x] Cursor visible and blinks
  - [x] Resize works
  - [x] No visible rendering artifacts
- [x] **Build:**
  - [x] `cargo build -p oriterm --target x86_64-pc-windows-gnu --release` succeeds
  - [x] `cargo clippy -p oriterm --target x86_64-pc-windows-gnu` — no warnings
  - [x] `cargo test -p oriterm` — all prepare-phase unit tests pass (400 tests, 4 ignored) (verified 2026-03-29: now 2084 tests, 0 ignored — expanded by subsequent sections)
- [x] No mouse selection, no search, no config, no tabs — just one terminal in one window

**Recovery sub-sections (added 2026-04-06):**

- [ ] **5.16** GPU Device Lost Recovery — Core Engine: status `complete`, all 5.16.1–
      5.16.14 items ticked, all core tests passing.
- [ ] **5.17** Recovery Correctness & Infrastructure: status `complete`, all 5.17.1–
      5.17.6 items ticked, canonical-snapshot stress test passing in both debug and
      release.
- [ ] **5.18** Recovery Integrations & Deferred Contracts: status `complete` (note:
      may legitimately remain in-progress until later sections land their PIN-test
      counterparts; the whole-section gate accepts 5.18 as "complete for the items
      that can land today", with deferred items tracked individually).
- [ ] **Performance invariants** (CLAUDE.md "Performance Invariants" — fifth bullet
      added by 5.18.8): "**Bounded recovery cost.** Device loss triggers single-flight
      recovery with exponential backoff capped at 8 attempts / 30s. While Recovering,
      idle CPU stays at the cursor-blink baseline (i.e., zero — blink is gated)."
- [ ] **Cross-section pin updates** (5.18.1): each cross-section contract has been
      written into its target section's plan file with a `relates-to:5.18` comment so
      Section 6/7/12/17/19/23/32/34/39/43/50 implementers cannot miss the contract.
- [ ] **`./build-all.sh`, `./clippy-all.sh`, `./test-all.sh`** all clean after 5.16+
      lands (CLAUDE.md "After EVERY change" — tests run with `timeout 150 ./test-all.sh`).

**Exit Criteria:** A working, visually correct terminal emulator with a clean, tested
render pipeline AND a verified GPU device-loss recovery path. The pipeline architecture
(Extract -> Prepare -> Render) is the foundation that all future rendering builds on.
The Prepare phase is independently testable. Offscreen rendering works for tab previews
and headless testing. Device loss does not crash the terminal; recovery preserves all
terminal state and enforces a bounded retry budget.

### Hygiene Notes (2026-03-29)

- [x] `gpu/atlas/mod.rs` split: extracted growth/eviction methods into `atlas/growth.rs` (mod.rs now 457 lines, well under 500-line limit). Done 2026-03-31.

### Positive Deviations (2026-03-29)

Implementation goes significantly beyond the plan:
- Incremental rendering (`dirty_skip` module): only regenerates dirty rows.
- Multi-atlas support: separate mono, subpixel, and color atlases.
- Subpixel rendering: full LCD subpixel pipeline with per-channel blending.
- Image rendering: inline image support (Sixel/iTerm2 protocol).
- Compositor: multi-layer composition with render target pooling.
- Builtin glyphs: box drawing, Braille, powerline, block elements, decorations.
- Pane cache: per-pane render caching for multi-pane layouts.
- Draw list conversion: scene-graph to instance buffer conversion with clipping.
