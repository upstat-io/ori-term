---
section: 54
title: "Render Capture & Texture Pipeline"
status: not-started
reviewed: true
last_verified: "2026-04-05"
tier: 5
goal: "Shared GPU infrastructure for persistent texture management with incremental sub-region updates and frame capture/downscale — first consumer is the minimap (Section 55), future consumers are expose thumbnails (Section 42) and tab hover previews (Section 16)"
success_criteria:
  - "PersistentTextureCache manages (PaneId, TexturePurpose)-keyed GPU textures with sub-region writes via queue.write_texture()"
  - "Global GPU memory budget enforced across all panes — eviction triggers when budget exceeded"
  - "FrameCapture downscales a rendered frame to a smaller RenderTargetPool target with linear filtering"
  - "Content cache texture updated with TEXTURE_BINDING flag when frame capture is active"
  - "All tests pass: cargo test -p oriterm -- capture, ./test-all.sh green"
  - "No allocation in the per-frame update path (reuse staging buffers)"
inspired_by:
  - "VS Code minimap incremental ImageData updates (minimap.ts)"
  - "Game engine render-to-texture + tile cache patterns (Cyberpunk 2077 minimap rewrite lessons)"
  - "ori_term ImageTextureCache LRU eviction pattern (oriterm/src/gpu/image_render/mod.rs)"
  - "ori_term RenderTargetPool power-of-two bucketing (oriterm/src/gpu/compositor/render_target_pool/mod.rs)"
depends_on: ["05", "43"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "54.1"
    title: "PersistentTextureCache"
    status: not-started
  - id: "54.2"
    title: "Frame Capture Pipeline"
    status: not-started
  - id: "54.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "54.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 54: Render Capture & Texture Pipeline

**Status:** Not Started
**Goal:** Provide shared GPU infrastructure for persistent texture management and frame capture/downscale that multiple features consume. The minimap (Section 55) is the first consumer — it needs a persistent GPU texture updated incrementally via sub-region writes. Expose mode (Section 42) and tab hover previews (Section 16) need frame capture — downscaling a rendered frame to thumbnail resolution. Both share a global GPU memory budget.

**Success Criteria:**

- [ ] `PersistentTextureCache` stores pane-keyed GPU textures, supports `write_region()` for sub-region RGBA uploads — satisfies mission criterion for minimap texture management
- [ ] Global GPU memory budget (configurable, default 64 MiB across all panes) with LRU eviction — satisfies mission criterion for bounded GPU memory
- [ ] `FrameCapture` renders a source texture to a smaller target with linear-filtered sampling — satisfies mission criterion for expose/tab thumbnails
- [ ] Content cache texture includes `TEXTURE_BINDING` usage flag when frame capture is active — satisfies mission criterion for source texture sampling
- [ ] Zero per-frame allocation in the update path (staging buffers reused via `.clear()` + capacity retention)
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `./test-all.sh` green

**Context:** The GPU pipeline already has two texture management patterns: `ImageTextureCache` (per-image, LRU eviction, GPU memory limit) and `RenderTargetPool` (transient, power-of-two bucketed, reusable). Neither fits the minimap use case — a long-lived texture that accumulates content over time with surgical sub-region updates. Expose and tab previews need a different primitive: downscale a full rendered frame to a smaller target. Both need a shared GPU memory budget to prevent VRAM blowup with many panes.

**Reference implementations:**
- **ori_term** `oriterm/src/gpu/image_render/mod.rs`: `ImageTextureCache` — LRU eviction with `gpu_memory_used` tracking and `frame_counter`-based aging. Pattern for global memory budgeting.
- **ori_term** `oriterm/src/gpu/compositor/render_target_pool/mod.rs`: `RenderTargetPool` — power-of-two bucketed transient targets with `RENDER_ATTACHMENT | TEXTURE_BINDING`. Pattern for pooled texture reuse.
- **VS Code** `minimap.ts`: Incremental `ImageData` updates — only changed lines redrawn. Double-buffered staging with pre-filled background.
- **wgpu** `queue.write_texture()`: Sub-region uploads with `origin` and `size` parameters for surgical partial texture updates.

**Depends on:** Section 05 (GPU pipeline — `GpuState`, `Device`, `Queue`, render targets). Section 43 (Compositor — `RenderTargetPool`, `CompositionPass` shader patterns).

---

## 54.1 PersistentTextureCache

**File(s):** `oriterm/src/gpu/texture_cache/mod.rs` (new module), `oriterm/src/gpu/texture_cache/tests.rs`

A persistent GPU texture cache for content that accumulates over time and is updated incrementally. Unlike `RenderTargetPool` (transient, per-frame) or `ImageTextureCache` (per-image, immutable after upload), this manages long-lived textures where sub-regions change independently.

The minimap is the first consumer: one texture per pane, updated row-by-row as terminal content changes. Future consumers (expose, tab previews) may store captured frame thumbnails.

- [ ] Write failing tests first: `texture_cache_create_entry`, `texture_cache_write_region`, `texture_cache_eviction`, `texture_cache_global_budget`, `texture_cache_reuse_after_release`

- [ ] Define `TextureCacheEntry` — a single persistent GPU texture:
  ```rust
  pub(crate) struct TextureCacheEntry {
      texture: wgpu::Texture,
      view: wgpu::TextureView,
      bind_group: Option<wgpu::BindGroup>,  // Lazily created on first bind_group() call
      width: u32,
      height: u32,
      size_bytes: usize,
      last_frame: u64,
  }
  ```

- [ ] Define `TextureCacheKey` — richer key supporting multiple textures per pane:
  ```rust
  /// Distinguishes different texture purposes for the same pane.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub(crate) enum TexturePurpose {
      Minimap,
      Thumbnail,
  }

  /// Cache key: pane identity + texture purpose.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub(crate) struct TextureCacheKey {
      pub pane_id: PaneId,
      pub purpose: TexturePurpose,
  }
  ```
  Import `PaneId` from `oriterm_mux::id::PaneId` (the `oriterm` crate depends on `oriterm_mux`).
  A pane may need multiple textures at different sizes (minimap, expose thumbnail, tab preview). Using bare `PaneId` as the key would force a single texture per pane. The compound key allows each purpose to have its own entry with independent dimensions and update cadence.

- [ ] Define `PersistentTextureCache` — keyed cache with global budget:
  ```rust
  pub(crate) struct PersistentTextureCache {
      entries: HashMap<TextureCacheKey, TextureCacheEntry>,
      staging_buffer: Vec<u8>,  // Reused across writes (clear + retain capacity)
      gpu_memory_used: usize,
      gpu_memory_limit: usize,   // Default 64 MiB
      frame_counter: u64,
      sampler: wgpu::Sampler,    // Linear filtering, clamp-to-edge
  }
  ```

- [ ] Implement `PersistentTextureCache::new(device, memory_limit)` — creates sampler with `FilterMode::Linear`, `AddressMode::ClampToEdge`. Follow `ImageTextureCache::new()` pattern from `image_render/mod.rs`.

- [ ] Implement `ensure_entry(device, key: TextureCacheKey, width, height)` — creates or resizes a texture entry. Texture usage: `TEXTURE_BINDING | COPY_DST` (sampled during composition, written via `queue.write_texture()`). Format: `TextureFormat::Rgba8UnormSrgb`. If entry exists at different size, destroys old and creates new (full invalidation on resize).

- [ ] Implement `write_region(queue, key: TextureCacheKey, x, y, width, height, data: &[u8])` — surgical sub-region write via `queue.write_texture()` with:
  ```rust
  queue.write_texture(
      wgpu::ImageCopyTexture {
          texture: &entry.texture,
          mip_level: 0,
          origin: wgpu::Origin3d { x, y, z: 0 },
          aspect: wgpu::TextureAspect::All,
      },
      data,
      wgpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: Some(width * 4),
          rows_per_image: Some(height),
      },
      wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
  );
  ```
  Uses `staging_buffer` for data preparation (`.clear()` + fill, no allocation after warmup). Updates `last_frame` on the entry.

- [ ] Implement `bind_group(device, layout, key: TextureCacheKey) -> Option<&BindGroup>` — returns bind group for composition pass sampling. Lazily creates bind group on first request (cached on entry).

- [ ] Implement `evict_if_needed()` — when `gpu_memory_used > gpu_memory_limit`, evict least-recently-used entries (lowest `last_frame`) until under budget. Follow `ImageTextureCache::evict_lru()` pattern.

- [ ] Implement `release(key: TextureCacheKey)` — removes entry, reclaims memory. Called when pane closes or feature disabled.

- [ ] Implement `release_pane(pane_id: PaneId)` — removes ALL entries for a pane (all purposes), reclaims memory. Called when pane closes.

- [ ] Implement `advance_frame()` — increments `frame_counter`. Called once per frame.

- [ ] Source file `oriterm/src/gpu/texture_cache/mod.rs` must stay under 500 lines. Tests in sibling `tests.rs` with `#[cfg(test)] mod tests;` at bottom of mod.rs. No `mod tests { }` wrapper inside `tests.rs` — the file IS the module.

- [ ] Register module: add `pub(crate) mod texture_cache;` to `oriterm/src/gpu/mod.rs` (currently 79 lines — ample headroom). Insert alphabetically after `transparency` line.

- [ ] **Test matrix:**
  - Create entry, write full region, verify dimensions
  - Write sub-region at various offsets (0,0), (64,128), (width-1, height-1)
  - Eviction: exceed budget, verify oldest entry evicted
  - Resize: ensure_entry at new dimensions destroys old texture
  - Release by key: verify memory reclaimed for that entry only
  - Release by pane: verify all entries for a pane reclaimed (minimap + thumbnail)
  - Global budget: multiple panes, verify total stays under limit
  - Multiple purposes per pane: same PaneId with Minimap and Thumbnail keys coexist independently
  - **Semantic pin:** `write_region` with non-zero origin writes to correct sub-region (not origin 0,0)

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- texture_cache`

---

## 54.2 Frame Capture Pipeline

**File(s):** `oriterm/src/gpu/capture/mod.rs` (new module), `oriterm/src/gpu/capture/tests.rs`

A GPU-side frame capture and downscale pipeline. Renders a source texture at reduced resolution to a smaller target texture using linear-filtered sampling. Used by expose mode (Section 42) for live pane thumbnails and tab bar (Section 16) for hover previews.

This is a render pass, not a compute shader — simpler, and linear filtering is built into the texture sampler.

**Critical texture usage constraint:** The existing content cache texture (`ensure_content_cache` in `window_renderer/render.rs`) is created with `RENDER_ATTACHMENT | COPY_SRC` — it does NOT have `TEXTURE_BINDING`. The `FrameCapture` pipeline needs to **sample** from its source texture via `textureSample()`, which requires `TEXTURE_BINDING` usage. The content cache cannot be sampled directly. Solutions:
1. Add `TEXTURE_BINDING` to the content cache usage flags when frame capture is enabled (cheapest — just one extra flag). This is the recommended approach.
2. Copy to an intermediate texture with `TEXTURE_BINDING` before sampling (wasteful extra copy).
3. Require callers to provide a source with the correct flags (pushes the problem to consumers).

Option 1 is correct: when `FrameCapture` is active, `ensure_content_cache` must include `TEXTURE_BINDING` in its usage flags. Add this as a step below.

- [ ] Write failing tests first: `capture_downscale_produces_correct_size`, `capture_linear_filtering`, `capture_reuses_target`

- [ ] Define `FrameCapture` pipeline:
  ```rust
  pub(crate) struct FrameCapture {
      pipeline: wgpu::RenderPipeline,
      bind_group_layout: wgpu::BindGroupLayout,
      sampler: wgpu::Sampler,
  }
  ```

- [ ] Implement `FrameCapture::new(device, target_format)` — creates:
  - WGSL shader: fullscreen triangle-strip vertex shader + texture sample fragment shader (standard downscale pattern)
  - Bind group layout: sampler (binding 0) + source texture (binding 1)
  - Render pipeline with the shader, targeting `target_format`, primitive topology `TriangleStrip`
  - Sampler: `FilterMode::Linear` for both mag and min (linear downscale)

- [ ] Write WGSL shader `oriterm/src/gpu/shaders/capture.wgsl`:
  ```wgsl
  // Fullscreen triangle-strip shader for texture downscale capture.
  // 4 vertices (TriangleStrip), no vertex buffer.
  // Matches the existing codebase convention (composite.wgsl, colr_solid.wgsl).

  @group(0) @binding(0) var tex_sampler: sampler;
  @group(0) @binding(1) var source: texture_2d<f32>;

  struct VertexOutput {
      @builtin(position) position: vec4<f32>,
      @location(0) uv: vec2<f32>,
  }

  @vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
      // TriangleStrip corners: 0=TL, 1=TR, 2=BL, 3=BR.
      let uv = vec2<f32>(f32(vi & 1u), f32((vi >> 1u) & 1u));
      let pos = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
      var out: VertexOutput;
      out.position = vec4<f32>(pos, 0.0, 1.0);
      out.uv = uv;
      return out;
  }

  @fragment fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
      return textureSample(source, tex_sampler, input.uv);
  }
  ```
  Uses a triangle strip (4 vertices, draw call: `draw(4, 1, 0, 0)`) matching the existing codebase convention. UVs are interpolated from the vertex shader (0..1 range maps source texture corners to output corners). No uniform buffer needed — the render pass viewport controls output dimensions, and the UV-based sampling handles the downscale mapping. The linear sampler produces the filtered result.

- [ ] Implement `capture(encoder, device, source_view, target_view, target_width, target_height)`:
  - Creates bind group for source texture + sampler
  - Begins render pass targeting `target_view` with `LoadOp::Clear` (transparent)
  - Sets viewport to `(0, 0, target_width, target_height)`
  - Draws fullscreen triangle strip (4 vertices via `draw(4, 1, 0, 0)`, no vertex buffer) — matches existing codebase convention
  - The GPU's linear sampler handles downscale filtering automatically — UVs map 0..1 across the source texture, the viewport controls output size

- [ ] Update `ensure_content_cache()` in `oriterm/src/gpu/window_renderer/render.rs` (currently 441 lines) to add `TEXTURE_BINDING` to the content cache texture usage flags. The change is at line ~410, in the `device.create_texture()` call — change:
  ```rust
  // Current (line ~410):
  usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
  // Updated:
  usage: wgpu::TextureUsages::RENDER_ATTACHMENT
       | wgpu::TextureUsages::COPY_SRC
       | wgpu::TextureUsages::TEXTURE_BINDING,
  ```
  This allows the content cache to be sampled by `FrameCapture`'s downscale pass. The extra usage flag has negligible cost on all GPU backends. This is a one-line change that does not materially affect the file's line count (441 → ~442).

- [ ] Integrate with `RenderTargetPool` — capture targets are acquired from the existing pool:
  ```rust
  let target_id = pool.acquire(device, thumb_width, thumb_height, format);
  capture.capture(encoder, device, source_view, pool.view(target_id), thumb_width, thumb_height);
  // ... use target for composition ...
  pool.release(target_id);
  ```

- [ ] Source file under 500 lines. Tests in sibling `tests.rs` with `#[cfg(test)] mod tests;` at bottom of mod.rs. No `mod tests { }` wrapper inside `tests.rs`.

- [ ] Register module: add `pub(crate) mod capture;` to `oriterm/src/gpu/mod.rs`. Insert alphabetically after `bind_groups` line.

- [ ] **Test matrix:**
  - Downscale 1920x1080 → 320x200: verify output texture dimensions
  - Downscale 128x128 → 64x64: verify linear filtering produces averaged colors
  - Identity capture (same size): verify pixel-perfect passthrough
  - Multiple captures per frame: verify no state leakage between captures
  - Content cache `TEXTURE_BINDING` flag: write a test that creates content cache and verifies `usage` includes `TEXTURE_BINDING` (regression pin for the flag change)
  - **Semantic pin:** downscale a 2x2 checkerboard (black/white) to 1x1 → should produce gray (linear blend), not black or white

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- capture`
- [ ] Verify TEXTURE_BINDING test passes: `timeout 150 cargo test -p oriterm -- content_cache`

---

## 54.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 54.N Completion Checklist

- [ ] `PersistentTextureCache` supports create/write_region/evict/release with global memory budget
- [ ] `FrameCapture` downscales source texture to smaller target via linear-filtered render pass
- [ ] Content cache texture includes `TEXTURE_BINDING` usage flag (verified by test)
- [ ] WGSL shader `capture.wgsl` renders fullscreen triangle with texture sampling
- [ ] All staging buffers reused (zero per-frame allocation after warmup)
- [ ] Global GPU memory budget enforced across panes (configurable limit, LRU eviction)
- [ ] `timeout 150 cargo test -p oriterm -- texture_cache` passes
- [ ] `timeout 150 cargo test -p oriterm -- capture` passes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] Source files under 500 lines (texture_cache/mod.rs, capture/mod.rs)
- [ ] Tests in sibling `tests.rs` files per test-organization.md
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] **Plan sync** — update plan metadata to reflect this section's completion:
  - [ ] This section's frontmatter `status` → `complete`, subsection statuses updated
  - [ ] `00-overview.md` Quick Reference table status updated
  - [ ] `index.md` section status updated
  - [ ] Next section's `depends_on` verified
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed — MUST run AFTER `/tpr-review` is clean

**Exit Criteria:** `PersistentTextureCache` manages pane-keyed GPU textures with sub-region `queue.write_texture()` updates, enforces a global memory budget via LRU eviction. `FrameCapture` downscales any source texture to a smaller target via a linear-filtered render pass. Both are exercised by unit tests with headless GPU. `./test-all.sh` green with 0 regressions.
