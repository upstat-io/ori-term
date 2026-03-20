---
section: "05"
title: "GPU-Side Scroll"
status: not-started
reviewed: true
goal: "Scroll is a GPU texture blit — scrollable content renders to an offscreen texture, and scrolling offsets the texture without repainting content. Only newly revealed content strips are painted."
inspired_by:
  - "Flutter RepaintBoundary + compositing layers (separate GPU surfaces per scroll region)"
  - "Chromium cc/layers (tiled compositing with damage-based update)"
  - "macOS Core Animation (layer-backed scroll views with GPU compositing)"
depends_on: ["01", "04"]
sections:
  - id: "05.1"
    title: "Offscreen Render Target"
    status: not-started
  - id: "05.2"
    title: "Content-to-Texture Pipeline"
    status: not-started
  - id: "05.3"
    title: "Scroll as Texture Blit"
    status: not-started
  - id: "05.4"
    title: "Strip Painting"
    status: not-started
  - id: "05.5"
    title: "Tests"
    status: not-started
  - id: "05.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: GPU-Side Scroll

**Status:** Not Started
**Goal:** Scrolling a long page produces <2ms frame time by compositing a pre-rendered content texture at a new offset, rather than repainting all visible content. Only the newly revealed strip (the content that scrolled into view) is painted fresh.

**Context:** Even with viewport culling (Section 01) and retained scene (Section 04), scrolling still requires repainting all visible widgets at their new positions. For a page with 6 visible scheme cards, that's 6 card paints + 6 text measurements + 6 GPU instance sets per scroll frame. The ideal is: scroll = move the texture, paint only the 1-2 newly visible cards.

This is how every major GUI framework handles scroll: render content to a backing texture larger than the viewport, then composite that texture at the scroll offset. When new content enters the viewport, paint just the new strip and update the texture.

**Reference implementations:**
- **Flutter** `rendering/layer.dart`: `OffsetLayer` composites child layers at an offset. `RepaintBoundary` creates a separate compositing layer.
- **Chromium** `cc/layers/picture_layer_impl.cc`: Tiled rendering with per-tile damage tracking. Scroll updates tile offsets and paints new tiles.
- **macOS** `NSScrollView`: Layer-backed views get their own GPU surface. `scrollPoint:` changes the layer's bounds origin without repainting.

**Depends on:** Section 01 (viewport culling to paint only visible content into the texture), Section 04 (retained scene for strip painting).

**Crate boundary note:** All GPU types (`ScrollTexture`, composite pipeline, bind groups) belong in the `oriterm` crate per crate-boundaries.md. They cannot be tested headlessly. Strategy: extract pure logic (strip range computation, ring buffer offset math, texture size calculation) into standalone functions in `oriterm_ui` (testable headlessly), and keep GPU resource management in `oriterm` (tested via `#[ignore]` GPU integration tests or visual regression tests).

---

## 05.1 Offscreen Render Target

**File(s):** `oriterm/src/gpu/scroll_composite/texture.rs` (new module)

Create a wgpu texture for scroll content rendering.

- [ ] Create a `ScrollTexture` struct:
  ```rust
  pub struct ScrollTexture {
      texture: wgpu::Texture,
      view: wgpu::TextureView,
      width: u32,   // physical pixels
      height: u32,  // content height (may be larger than viewport)
      format: wgpu::TextureFormat,
  }
  ```

- [ ] Allocate the texture at content height (or viewport height + margin for pre-rendering):
  - Option A: Full content height texture (simple, wasteful for very long content)
  - Option B: Viewport + 2x margin (ring buffer style, repaint when margin is consumed)
  - **Recommended: Option B** — viewport height × 3 (1 screen above + viewport + 1 screen below). Repaint margin strips as user scrolls.

- [ ] Handle texture resize when content height changes (page switch, window resize)

- [ ] **Cross-platform texture format:** The texture format must match the surface's preferred format (`surface.get_capabilities(adapter).formats[0]`). Different platforms prefer different formats: macOS uses `Bgra8UnormSrgb`, Windows/Linux typically use `Bgra8Unorm`. Use `surface_config.format` as the scroll texture format.

- [ ] **DPI/scale factor:** Texture dimensions must be in PHYSICAL pixels (`logical_size * scale_factor`). When DPI changes (e.g., dragging window between monitors), the texture must be recreated at the new size. Wire into `DialogWindowContext::resize_surface()` which already handles DPI changes.

- [ ] **Texture lifecycle:** The `ScrollTexture` must be destroyed and recreated on page switch (content height changes completely). Invalidate on any `full_invalidation` event. Consider making `ScrollTexture` an `Option<ScrollTexture>` — only allocate when scroll content is active, drop when switching to a non-scrollable page.

---

## 05.2 Content-to-Texture Pipeline

**File(s):** `oriterm/src/gpu/window_renderer/` (scene_append.rs, render.rs), `oriterm/src/app/dialog_rendering.rs`

Render scroll content to the offscreen texture instead of the main surface.

- [ ] Create a wgpu `RenderPass` targeting the `ScrollTexture`:
  ```rust
  fn render_scroll_content(
      &mut self,
      encoder: &mut wgpu::CommandEncoder,
      scroll_texture: &ScrollTexture,
      scene: &Scene,
      visible_range: Range<f32>,  // y range to render
  ) { ... }
  ```

- [ ] The Scene primitives for scroll content are converted to GPU instances as usual (via `convert_scene`), but rendered to the scroll texture instead of the main surface

- [ ] The scroll texture contains ALL visible content at its natural position (no scroll offset applied — the offset is applied at composite time)

---

## 05.3 Scroll as Texture Blit

**File(s):** `oriterm/src/gpu/scroll_composite/mod.rs` (pipeline + render pass), `oriterm/src/gpu/shaders/composite_scroll.wgsl` (new shader)

**WARNING: High complexity.** A new GPU composite shader requires: a WGSL shader file, a new `wgpu::RenderPipeline` + `PipelineLayout`, a new `BindGroupLayout` + `BindGroup` for the scroll texture + sampler + uniforms, a `wgpu::Sampler`, a uniform buffer for `scroll_offset`/`content_height`, integration into `WindowRenderer` render pass ordering, and pipeline caching. Split into `scroll_composite/mod.rs` (pipeline + render pass) and `scroll_composite/texture.rs` (ScrollTexture lifecycle) to stay under the 500-line limit per file.

When the user scrolls, instead of repainting:
1. Offset the scroll texture's sampling UV coordinates by the scroll delta
2. Composite the scroll texture onto the main surface at the viewport position
3. Render chrome, footer, and scrollbar on top (these don't scroll)

- [ ] Create a compositing render pass that samples the scroll texture:
  ```wgsl
  @fragment
  fn composite_scroll(in: VertexOutput) -> @location(0) vec4<f32> {
      let scroll_uv = in.uv + vec2<f32>(0.0, scroll_offset / content_height);
      return textureSample(scroll_texture, scroll_sampler, scroll_uv);
  }
  ```

- [ ] The composite pass draws a single quad (the viewport-sized scroll area) with the scroll texture

- [ ] Frame composition order:
  1. Clear main surface
  2. Draw chrome (title bar)
  3. Draw scroll content via texture composite (single quad)
  4. Draw scrollbar on top
  5. Draw footer on top
  6. Draw overlays (dropdowns) on top

---

## 05.4 Strip Painting

When the user scrolls, new content enters the viewport. Paint only that strip.

- [ ] Track the "rendered range" — the y-range of content currently in the scroll texture
- [ ] On scroll, compute the "revealed strip" — content that entered the viewport but isn't in the texture:
  ```rust
  let revealed = if scroll_delta > 0.0 {
      // Scrolled down — new content at bottom
      rendered_range.end..new_rendered_range.end
  } else {
      // Scrolled up — new content at top
      new_rendered_range.start..rendered_range.start
  };
  ```

- [ ] Paint only widgets within the revealed strip (using viewport culling from Section 01)
- [ ] Update the scroll texture with the new strip (partial texture update via `queue.write_texture()` or render to a subregion)

- [ ] Handle large scroll jumps (scroll > margin): fall back to full content repaint into texture

---

## 05.4b Overlay Rendering Interaction

**File(s):** `oriterm/src/app/dialog_rendering.rs` (`render_dialog_overlays`)

Overlay popups (dropdown lists) are rendered AFTER the main content via `render_dialog_overlays()`. Each overlay clears the scene, paints, and appends with its own opacity. Overlays must be composited AFTER the scroll texture blit, not into it.

- [ ] Frame composition order must account for overlays:
  1. Clear main surface
  2. Draw chrome (title bar) — NOT scrolled
  3. Draw scroll content via texture composite (single quad)
  4. Draw scrollbar on top
  5. Draw footer on top — NOT scrolled
  6. Draw each overlay on top — NOT scrolled, NOT in scroll texture
- [ ] Overlays use the existing `append_overlay_scene_with_text()` path unchanged
- [ ] Scroll texture is invalidated only for scroll content, not overlay changes

---

## 05.5 Tests

**File(s):** Pure logic tests in `oriterm_ui` (strip range, texture size), GPU integration tests in `oriterm/src/gpu/scroll_composite/tests.rs` (`#[ignore]` -- require GPU).

- [ ] Test (pure, oriterm_ui): `strip_range_scroll_down` — scroll down by 50px, verify revealed strip is `(old_bottom..old_bottom+50)`
- [ ] Test (pure, oriterm_ui): `strip_range_scroll_up` — scroll up by 50px, verify revealed strip is `(old_top-50..old_top)`
- [ ] Test (pure, oriterm_ui): `large_scroll_triggers_full_repaint` — scroll by 3x viewport, verify `needs_full_repaint() == true`
- [ ] Test (pure, oriterm_ui): `texture_size_from_viewport` — verify texture dimensions = viewport * 3 in physical pixels
- [ ] Test (GPU, #[ignore]): `scroll_texture_created_at_correct_size` — verify wgpu texture dimensions match content
- [ ] Test (GPU, #[ignore]): `scroll_blit_produces_correct_viewport` — composite at offset 0, verify first page visible
- [ ] Test (GPU, #[ignore]): `strip_paint_renders_only_new_content` — scroll by 1 card height, verify only 1-2 cards painted

---

## 05.6 Completion Checklist

- [ ] New `oriterm/src/gpu/scroll_composite/` module directory (mod.rs + texture.rs, each < 500 lines)
- [ ] Pure logic functions (strip range, texture size) in `oriterm_ui` (headlessly testable)
- [ ] ScrollTexture struct with wgpu texture management
- [ ] Content renders to offscreen texture
- [ ] Scroll changes texture sampling offset (no content repaint)
- [ ] Strip painting renders only newly revealed content
- [ ] Large scroll jumps fall back to full repaint
- [ ] Texture resized on content height change
- [ ] Texture format matches surface format (cross-platform: Bgra8UnormSrgb on macOS, Bgra8Unorm on Windows/Linux)
- [ ] Texture dimensions in physical pixels (`logical * scale_factor`)
- [ ] Texture recreated on DPI change
- [ ] Texture invalidated on page switch (content height changes completely)
- [ ] Overlay rendering composited AFTER scroll texture blit (not into scroll texture)
- [ ] Chrome/footer rendered independently (not scrolled)
- [ ] No regressions — `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green
- [ ] Frame time during scroll <2ms (measured)

**Exit Criteria:** Continuous scrolling on the Colors page with 12 scheme cards produces <2ms frame time. Most frames are texture blits (no paint calls). Strip painting occurs only when new content enters the viewport.
