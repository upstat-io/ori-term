# Section 42: Expose / Overview Mode -- Verification Results

**Verified:** 2026-03-29
**Status:** CONFIRMED NOT STARTED
**Reviewed:** false (unreviewed gate)

---

## 1. Code Search: Is Any Preliminary Code Present?

**No expose mode code exists.** Exhaustive search results:

- `expose|overview|thumbnail|mission.?control` across all `*.rs` files: only hits for:
  - Generic "expose" in doc comments (e.g., "exposes" as a verb)
  - `oriterm_ui/src/compositor/delegate.rs:18` -- "expose mode thumbnails" (forward reference in doc comment, no code)
  - `oriterm_ui/src/animation/delegate.rs:22` -- "expose mode (remove thumbnail after exit animation)" (forward reference)
  - `oriterm/src/gpu/render_target/mod.rs:6` -- "Used for tab previews, headless test rendering, thumbnails" (forward reference)
  - `oriterm/src/widgets/terminal_preview/mod.rs` -- a **scaffold widget** for tab hover preview (see below)
- No `oriterm/src/app/expose.rs` or `oriterm/src/app/expose/` directory (glob returned empty).
- No `ExposeMode`, `ExposeTile`, `ExposePhase`, `ThumbnailCache` types anywhere.

**Verdict:** Truly not started. Some infrastructure mentions expose in forward-looking doc comments, but no expose-specific code exists.

---

## 2. TODOs/FIXMEs Related to This Section's Domain

None. No TODOs reference expose mode, overview, or thumbnails.

---

## 3. Infrastructure That Partially Covers This Section

### 3a. TerminalPreviewWidget (PARTIAL SCAFFOLD)

**Location:** `oriterm/src/widgets/terminal_preview/mod.rs`

A scaffold widget for tab hover previews that partially overlaps with expose thumbnails:

- 320x200 default dimensions (matches the plan's thumbnail size exactly)
- 0.25 scale factor
- `WidgetId`, `Widget` trait implementation
- Currently draws a placeholder rounded rectangle (no actual terminal content)
- Marked `#[allow(dead_code)]` with reason "scaffold -- wired in tab hover preview section"

This widget is for tab hover previews, not expose mode, but the thumbnail concept and dimensions are identical. Expose mode would need many of these simultaneously, each backed by an offscreen texture.

### 3b. RenderTarget (READY)

**Location:** `oriterm/src/gpu/render_target/mod.rs`

Offscreen render target infrastructure is complete:

- `GpuState::create_render_target(width, height)` -- creates textures with `RENDER_ATTACHMENT | COPY_SRC`
- `GpuState::read_render_target()` -- pixel readback for testing
- Same render format as surfaces (pipelines work identically)
- `strip_row_padding()` for GPU alignment handling

This is exactly what the plan's `ThumbnailCache` would wrap.

### 3c. RenderTargetPool (READY)

**Location:** `oriterm/src/gpu/compositor/render_target_pool/mod.rs`

Pooled render target allocation with power-of-two bucketing:

- `RenderTargetPool::acquire(device, width, height, format)` -- allocates or reuses
- `RenderTargetPool::release(id)` -- returns to pool
- `RenderTargetPool::view(id)` -- gets texture view for render pass
- `RenderTargetPool::texture(id)` -- gets raw texture
- Power-of-two bucketing for reuse (minimum 256)
- `RENDER_ATTACHMENT | TEXTURE_BINDING` usage (can render into AND sample from)

This pool is more sophisticated than what the plan describes (which uses a simple `HashMap<PaneId, RenderTarget>`). The pool pattern is better -- expose mode should use it.

### 3d. Image Pipeline (READY)

**Location:** `oriterm/src/gpu/pipeline/image.rs`

A textured quad GPU pipeline already exists:

- `create_image_pipeline()` -- WGSL vertex/fragment shaders for textured quads
- `create_image_texture_bind_group_layout()` -- texture + sampler binding
- `IMAGE_INSTANCE_STRIDE` -- per-instance data stride
- Wired into `Pipelines` struct at `oriterm/src/gpu/pipelines.rs`
- Used by `oriterm/src/gpu/window_renderer/render.rs:710` (image render pass)

This is the "ImagePipeline" the plan proposes to create -- it already exists. Expose mode can use it for compositing thumbnail textures into the grid layout.

### 3e. Compositor Layer System (READY -- Section 43 Complete)

**Location:** `oriterm/src/gpu/compositor/`

The compositor layer system provides the infrastructure for full-frame modal overlays:

- Layer tree with ordered compositing
- Layer animator for transitions
- Pooled render targets
- The compositor can handle expose mode as a full-viewport layer

### 3f. Pane Enumeration (PARTIAL)

**Location:** `oriterm_mux/src/in_process/` and `oriterm/src/session/`

- `InProcessMux` provides pane CRUD and registry
- `SessionRegistry` tracks tabs, windows, layouts
- Pane enumeration is available via `PaneRegistry`

However, the plan's cross-window pane access ("enumerate all panes from `InProcessMux`") assumes Section 31 and 32 are complete. Section 31 (In-Process Mux + Multi-Pane Rendering) is "Not Started" per the index, and Section 32 (Tab & Window Management) is also "Not Started."

---

## 4. Gap Analysis

### Plan Strengths
- Well-designed state machine (`ExposePhase` enum with animation phases)
- Pure layout function (`compute_expose_grid()`) -- testable, deterministic
- Staggered thumbnail update strategy (burst on entry, round-robin after)
- Performance targets specified (entry < 100ms, per-frame < 4ms, filter < 2ms)
- Type-to-filter with double-Escape semantics
- Character label hints for O(1) selection

### Plan Gaps and Issues

**G1: ImagePipeline Already Exists.**
The plan proposes creating `oriterm/src/gpu/pipeline/image_pipeline.rs` with a new textured-quad pipeline. This pipeline already exists at `oriterm/src/gpu/pipeline/image.rs` (created for image protocol rendering). The plan should specify reusing the existing pipeline rather than creating a new one.

**G2: RenderTargetPool Not Mentioned.**
The plan describes a `ThumbnailCache` with `HashMap<PaneId, RenderTarget>`. The existing `RenderTargetPool` in the compositor module is a better pattern (power-of-two bucketing, reuse). The plan should reference it.

**G3: Dependencies Are Heavily Unmet.**
The plan lists Section 31 (In-Process Mux), Section 32 (Tab & Window Management), Section 07 (2D UI Framework), and Section 05 (GPU Pipeline) as prerequisites. Status:
- Section 31: Not Started (roadmap), but `InProcessMux` code exists
- Section 32: Not Started (roadmap), but session/tab/window management code exists
- Section 07: In Progress
- Section 05: Complete

The actual code for pane enumeration and tab/window management exists, but the roadmap sections aren't formally complete.

**G4: Full-Frame Rendering Replacement Underspecified.**
The plan says "The entire viewport is replaced by the thumbnail grid" but doesn't specify how this interacts with the rendering pipeline. Currently, `draw_frame()` renders terminal content. In expose mode, the entire frame is thumbnails + labels. Options:
- Short-circuit `draw_frame()` to call a separate `draw_expose_frame()`
- Use the compositor layer system to render expose as a full-viewport overlay layer
- The plan should specify which approach

**G5: Multi-Window Expose.**
The plan mentions "If pane is in a different window: raise that window, switch tab, focus pane." This requires cross-window focus management. On Windows, this means calling `SetForegroundWindow` (which has restrictions in Win32). On Linux/macOS, raising another window programmatically has different platform-specific mechanisms. The plan doesn't address platform considerations for cross-window switching.

**G6: Scrolling Not Addressed for Many Panes.**
The plan mentions "Many panes -> smaller thumbnails (cap at minimum size, enable scrolling if needed)" but the scrolling implementation is not specified. If there are 50+ panes and thumbnails are capped at 160x100, they won't fit in a viewport. The plan needs a scroll state and scroll input handling for the expose grid.

**G7: Memory Budget.**
Each thumbnail at 320x200 RGBA is ~256KB. With 50 panes, that's ~12.8MB of GPU memory just for thumbnails. The plan doesn't specify a memory cap or eviction strategy for thumbnail textures (it mentions LRU eviction on pane close, but not a memory cap).

---

## 5. Dependency Status

| Dependency | Roadmap Status | Actual Code Status |
|---|---|---|
| Section 31 (In-Process Mux) | Not Started | InProcessMux exists, pane CRUD works |
| Section 32 (Tab & Window Management) | Not Started | SessionRegistry, Tab, Window exist |
| Section 07 (2D UI Framework) | In Progress | Widget system, layout engine, text rendering working |
| Section 05 (GPU Pipeline) | Complete | RenderTarget, ImagePipeline, RenderTargetPool all exist |
| Section 43 (Compositor) | Complete | Layer system, render target pool, animation available |
