---
section: "01"
title: GPU Scissor Rect Support
status: not-started
goal: "PushClip/PopClip commands in DrawList produce GPU scissor rect changes during rendering"
inspired_by:
  - "Chrome compositor clip regions"
  - "wgpu::RenderPass::set_scissor_rect"
depends_on: []
sections:
  - id: "01.1"
    title: "Clip State in convert_draw_list"
    status: not-started
  - id: "01.2"
    title: "Scissor Rect Encoding"
    status: not-started
  - id: "01.3"
    title: "Tests"
    status: not-started
  - id: "01.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: GPU Scissor Rect Support

**Status:** Not Started
**Goal:** `PushClip`/`PopClip` draw commands produce actual GPU scissor rect changes during rendering. Currently these commands are no-ops (logged as `trace!` in `convert_draw_list`). After this section, any widget can clip its children by wrapping draw calls in `push_clip(rect)` / `pop_clip()`.

**Context:** The `DrawList` supports `PushClip { rect }` / `PopClip` commands (added in Section 07). The GPU converter in `oriterm/src/gpu/draw_list_convert/mod.rs` explicitly skips them:

```rust
DrawCommand::PushClip { .. } => {
    log::trace!("DrawCommand::PushClip deferred — not yet implemented");
}
DrawCommand::PopClip => {
    log::trace!("DrawCommand::PopClip deferred — not yet implemented");
}
```

There is even a test (`clip_commands_are_noop`) that verifies they are no-ops. This is the root cause of tab show-through — tabs draw content that overflows their bounds into adjacent tabs, and nothing clips it.

**Reference implementations:**
- **wgpu** `RenderPass::set_scissor_rect(x, y, w, h)`: Hardware scissor test — pixels outside the rect are discarded by the GPU. Zero CPU cost, zero fragment shader cost for discarded pixels.

**Depends on:** Nothing — this is pure infrastructure.

---

## 01.1 Clip State in convert_draw_list

**File(s):** `oriterm/src/gpu/draw_list_convert/mod.rs`

The challenge: `convert_draw_list` currently produces a flat stream of instance records that are uploaded to GPU buffers and drawn in a single `RenderPass`. Scissor rects require splitting the draw into segments — each segment has its own scissor rect.

**Approach — ClipSegment Side-Channel:**

Use a `ClipSegment` side-channel that records (instance_offset, scissor_rect) pairs alongside the instance buffer stream. During `record_draw_passes`, the render pass consults these segments to call `set_scissor_rect` at the right points and split draw calls into sub-ranges. This avoids restructuring the render pipeline while adding clip support.

- [ ] Add `ClipSegment` struct: `{ instance_offset: u32, rect: Option<[u32; 4]> }` — `None` means reset to full viewport
- [ ] Add `clip_out: &mut TierClips` output parameter to `convert_draw_list` (callers pass reusable storage; cleared at start of each call). The `TierClips` struct holds 4 `Vec<ClipSegment>` — one per writer (rects, mono, subpixel, color). When `text_ctx` is `None`, only the rects Vec is populated.
- [ ] Track a `clip_stack: &mut Vec<Rect>` parameter to `convert_draw_list` (caller provides reusable storage; cleared at start of each call — avoids per-frame allocation in the render path)
- [ ] On `PushClip { rect }`:
  - Scale rect by `scale` factor and convert to physical pixels
  - Intersect with current clip (top of stack, or viewport if empty) — clips nest
  - Push intersected rect onto stack
  - Emit a `ClipSegment` at current instance offset with the new scissor rect
- [ ] On `PopClip`:
  - Pop the stack
  - Emit a `ClipSegment` at current instance offset with the new top (or `None` for full viewport)
- [ ] Remove the `log::trace!` no-op handling for `PushClip`/`PopClip`

```rust
/// A clip state change at a specific point in the instance stream.
#[derive(Debug, Clone, Copy)]
pub struct ClipSegment {
    /// Instance index (not byte offset) where this clip takes effect.
    /// Corresponds to `InstanceWriter::len()` at the point the clip was emitted.
    pub instance_offset: u32,
    /// Scissor rect in physical pixels, or `None` for full viewport.
    pub rect: Option<[u32; 4]>,
}
```

---

## 01.2 Scissor Rect Encoding

**File(s):** `oriterm/src/gpu/window_renderer/render.rs`, `oriterm/src/gpu/prepared_frame/mod.rs`, `oriterm/src/gpu/window_renderer/draw_list.rs`, `oriterm/src/gpu/window_renderer/helpers.rs`

**File size warning:** `draw_list_convert/mod.rs` is 425 lines. Adding clip infrastructure will exceed the 500-line limit. Extract `ClipSegment`, `TierClips`, and clip-stack intersection logic into `draw_list_convert/clip.rs` before adding the `PushClip`/`PopClip` arms to the match.

The `record_draw_passes` method must consume `ClipSegment`s to set scissor rects during the render pass.

**The multi-writer problem:**

`convert_draw_list` writes to 4 writers simultaneously: `ui_writer` (rects), `mono_writer`, `subpixel_writer`, `color_writer` (text). A single `ClipSegment` tracks the rect writer offset, but the scissor must also apply to glyph draw calls. Two approaches:

**(a) Per-writer clip segments** (recommended): Return 4 parallel `Vec<ClipSegment>` — one per writer. Each records the instance offset in its own writer when a clip change occurs. All 4 have identical scissor rects at corresponding boundaries. `record_draw_passes` splits each of the 4 draw calls independently.

**(b) Unified clip with draw-call splitting**: Track only ui_rect offsets. Before each tier, set scissor rect and issue partial draw calls. Requires refactoring `record_draw` to accept instance ranges.

**Recommended path:** Option (a) — more code but cleanly composable.

- [ ] Add per-tier clip segment storage to `PreparedFrame`:
  ```rust
  /// Clip segments for the chrome tier (draws 6–9), one per writer.
  pub ui_clips: TierClips,
  /// Clip segments for the overlay tier (draws 10–13), one per writer.
  pub overlay_clips: TierClips,
  ```
- [ ] Define `TierClips` to hold 4 parallel clip segment vectors:
  ```rust
  /// Clip segments for all 4 writers in a tier.
  pub struct TierClips {
      pub rects: Vec<ClipSegment>,
      pub mono: Vec<ClipSegment>,
      pub subpixel: Vec<ClipSegment>,
      pub color: Vec<ClipSegment>,
  }
  ```
- [ ] Update `PreparedFrame::new()`, `clear()`, and `extend_from()` to handle the new fields
- [ ] Update `append_ui_draw_list_with_text` and `append_overlay_draw_list_with_text` in `draw_list.rs` to pass clip segment vectors and reusable `clip_stack` to `convert_draw_list` and store results in `PreparedFrame`
- [ ] Add reusable `clip_stack: Vec<Rect>` field to `WindowRenderer` (not per-call allocation)
- [ ] Modify `convert_draw_list` to accept `clip_segments: &mut TierClips`, `clip_stack: &mut Vec<Rect>` and emit segments into ALL active writers when a clip change occurs
- [ ] In `record_draw_passes`, for the chrome tier (draws 6–9) and overlay tier (draws 10–13):
  - Replace each `record_draw()` call with `record_draw_clipped()` that iterates the tier's clip segments for that writer
  - Between each segment boundary, call `pass.set_scissor_rect(x, y, w, h)` and issue `pass.draw(0..4, start..end)` for the instance range
  - After the tier, reset scissor to full viewport
- [ ] Refactor `record_draw` in `helpers.rs` to support instance sub-ranges: add `record_draw_range(pass, pipeline, bg, atlas, buffer, start, end)` or add optional `ClipSegment` slice parameter
- [ ] Consider a `DrawBindings` struct to reduce parameter count on `record_draw_clipped` (9 params violates "> 3 params → config struct" guideline)
- [ ] Terminal tier (draws 1–5) does NOT need clip support — grid cells are always aligned to the cell grid
- [ ] Ensure `set_scissor_rect` coordinates are clamped to surface dimensions (wgpu panics on out-of-bounds)
- [ ] Reset scissor rect to full viewport between tiers to prevent cross-tier clip leakage

```rust
// In record_draw_passes, for a clipped draw call:
fn record_draw_clipped<'a>(
    pass: &mut RenderPass<'a>,
    pipeline: &'a RenderPipeline,
    uniform_bg: &'a BindGroup,
    atlas_bg: Option<&'a BindGroup>,
    buffer: Option<&'a Buffer>,
    total_instances: u32,
    clips: &[ClipSegment],
    viewport_w: u32,
    viewport_h: u32,
) {
    if total_instances == 0 {
        return;
    }
    let Some(buf) = buffer else { return };
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, uniform_bg, &[]);
    if let Some(atlas) = atlas_bg {
        pass.set_bind_group(1, atlas, &[]);
    }
    pass.set_vertex_buffer(0, buf.slice(..));

    if clips.is_empty() {
        // No clips — draw everything.
        pass.draw(0..4, 0..total_instances);
        return;
    }

    let mut cursor = 0u32;
    for seg in clips {
        // Draw instances before this clip change.
        if seg.instance_offset > cursor {
            pass.draw(0..4, cursor..seg.instance_offset);
        }
        // Apply new scissor.
        if let Some(r) = seg.rect {
            pass.set_scissor_rect(r[0], r[1], r[2], r[3]);
        } else {
            pass.set_scissor_rect(0, 0, viewport_w, viewport_h);
        }
        cursor = seg.instance_offset;
    }
    // Draw remaining instances.
    if cursor < total_instances {
        pass.draw(0..4, cursor..total_instances);
    }
    // Reset scissor.
    pass.set_scissor_rect(0, 0, viewport_w, viewport_h);
}
```

---

## 01.3 Tests

**File(s):** `oriterm/src/gpu/draw_list_convert/tests.rs`

- [ ] Update `clip_commands_are_noop` test → rename to `clip_commands_produce_segments`
  - Verify that `PushClip` + rect + `PopClip` produces 2 `ClipSegment`s (enter + exit)
  - Verify the rect instance is still emitted (clip doesn't suppress content)
- [ ] Test nested clips: `PushClip(A)` → `PushClip(B)` → rect → `PopClip` → `PopClip`
  - Inner clip should be intersection of A and B
  - After inner pop, clip reverts to A
  - After outer pop, clip is `None` (full viewport)
- [ ] Test clip with scale factor: push clip in logical pixels, verify segment rect is in physical pixels
- [ ] Test empty clip (zero-area intersection): content inside should still be emitted (GPU discards via scissor, not CPU-side filtering)
- [ ] Test clip with text content: build a `DrawList` with `PushClip` → rect + text glyph → `PopClip`, pass through `convert_draw_list` with a `text_ctx`, verify `ClipSegment`s are emitted into all active writers (rects, mono, subpixel, color) at the correct instance offsets for each writer
- [ ] Test unbalanced clips: extra `PopClip` without matching `PushClip` must not panic (pop on empty stack → no-op or log warning)
- [ ] Test clip rect clamping: clip rect extending beyond surface dimensions (e.g. negative x, width exceeding viewport) must be clamped to `[0, 0, viewport_w, viewport_h]`
- [ ] Test scroll widget integration: `ScrollWidget` already emits `PushClip`/`PopClip` — verify its draw list produces valid `ClipSegment`s (existing scroll tests in `oriterm_ui/src/widgets/scroll/tests.rs` verify balanced clip commands; add a test that feeds the resulting draw list through `convert_draw_list` and checks segments)

---

## 01.4 Completion Checklist

- [ ] `ClipSegment` and `TierClips` extracted into `draw_list_convert/clip.rs` (keeps `mod.rs` under 500 lines)
- [ ] `clip_stack` is reusable storage (field on `WindowRenderer`), not per-call allocation
- [ ] `PushClip`/`PopClip` in `DrawList` produce `ClipSegment`s in `convert_draw_list`
- [ ] Clip segments emitted into all 4 writers (rects, mono, subpixel, color) at correct offsets
- [ ] `record_draw_passes` applies scissor rects from segments with per-writer draw splitting
- [ ] `record_draw_clipped` (or equivalent) issues `pass.draw(0..4, start..end)` sub-ranges
- [ ] Nested clips correctly intersect
- [ ] Scale factor correctly applied to clip rects
- [ ] Scissor rects clamped to surface dimensions
- [ ] Scissor rects reset between tiers (chrome → overlay) to prevent cross-tier leakage
- [ ] Unbalanced `PopClip` (empty stack) handled gracefully (no panic)
- [ ] Old no-op test updated to verify new behavior
- [ ] `append_ui_draw_list_with_text` and `append_overlay_draw_list_with_text` pass/store clip segments
- [ ] `PreparedFrame` updated with `TierClips` fields + `new()`, `clear()`, `extend_from()`
- [ ] `./clippy-all.sh` — no warnings
- [ ] `./test-all.sh` — all pass
- [ ] `./build-all.sh` — cross-compilation succeeds

**Exit Criteria:** A `DrawList` with `push_clip(rect)` → `push_rect(...)` → `pop_clip()` correctly clips the rect to the clip bounds when rendered. Verified by unit tests on `ClipSegment` output and by visual confirmation that tab content no longer bleeds through adjacent tabs (after Section 02 adds the clip calls).
