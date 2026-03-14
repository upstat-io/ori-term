---
section: "04"
title: "Scroll as Viewport Transform"
status: complete
goal: "Scrolling a ScrollWidget changes a viewport offset — not a content redraw. Child content is retained and only redrawn when the child's own state changes."
inspired_by:
  - "Chromium cc::ScrollNode (cc/trees/scroll_node.h) — scroll offset as transform"
  - "Flutter PhysicalShapeLayer clip + offset composition"
depends_on: ["03"]
sections:
  - id: "04.1"
    title: "Scroll Offset as Transform"
    status: complete
  - id: "04.2"
    title: "DrawList Transform Command"
    status: complete
  - id: "04.3"
    title: "GPU Converter Support"
    status: complete
  - id: "04.4"
    title: "Completion Checklist"
    status: complete
reviewed: true
---

# Section 04: Scroll as Viewport Transform

**Status:** Not Started
**Goal:** When `ScrollWidget` receives a scroll event (wheel, keyboard, scrollbar drag) and the child content has not changed, the framework applies a viewport offset transform to the existing cached scene rather than calling `child.draw()` again. Child content is only rebuilt when its own state changes (hover, text change, etc.).

**Context:** Today `ScrollWidget::draw()` (scroll/mod.rs:241-270) calls `self.child.draw()` on every frame, passing shifted `child_bounds` to account for `scroll_offset`. The child re-traverses its entire widget tree, re-shapes all text, and re-emits all draw commands. For the Settings dialog (which is wrapped in a ScrollWidget), this means scrolling triggers full content rebuild — all labels, buttons, dropdowns, sections, form rows.

With Section 03's scene retention, the child's draw commands are cached. But `ScrollWidget::draw()` still calls `child.draw()` because the `bounds` change on every scroll, which invalidates the cache (bounds mismatch). The fix is to separate the scroll transform from the content bounds: the child always draws at its natural (unscrolled) position, and the scroll offset is applied as a post-draw transform via a `PushTranslate`/`PopTranslate` draw command pair.

**Reference implementations:**
- **Chromium** `cc/trees/scroll_node.h`: Scroll offset is a property on the scroll node in the property tree. Content layers are positioned in scroll-parent space. The compositor applies the scroll offset as a transform during composition.
- **Flutter** `flow/layers/transform_layer.cc`: Scroll containers wrap their child layer list in a transform that shifts by the scroll offset.

**Depends on:** Section 03 (scene retention gives us cached child draw commands that can be replayed under a transform).

---

## 04.1 Scroll Offset as Transform

**File(s):** `oriterm_ui/src/widgets/scroll/mod.rs`

**File size note:** `scroll/mod.rs` is currently 443 lines. The `draw()` changes are structural (replacing bounds-shifting with translate commands), not additive, so the file should stay within the limit. Monitor during implementation.

Modify `ScrollWidget::draw()` to emit a transform instead of shifting child bounds.

- [x] Change draw strategy:
  ```rust
  // Before (scroll/mod.rs:241-270):
  // Note: push_clip(ctx.bounds) already exists before this.
  let child_bounds = Rect::new(
      ctx.bounds.x() - self.scroll_offset_x,
      ctx.bounds.y() - self.scroll_offset,
      content_w, content_h,
  );
  self.child.draw(&mut child_ctx);
  // pop_clip() already exists after this.

  // After:
  ctx.draw_list.push_clip(ctx.bounds);
  ctx.draw_list.push_translate(-self.scroll_offset_x, -self.scroll_offset);
  // Child draws at its natural (unscrolled) position — ctx.bounds unshifted.
  let child_bounds = Rect::new(ctx.bounds.x(), ctx.bounds.y(), content_w, content_h);
  self.child.draw(&mut child_ctx);
  ctx.draw_list.pop_translate();
  ctx.draw_list.pop_clip();
  ```

- [x] With this change, `child_bounds` no longer changes on scroll. The child's `SceneNode` cache key (bounds) stays stable across scroll events. Only the translate transform changes, which is a single draw command swap — not a full subtree rebuild.

- [x] `ScrollWidget` scroll events should produce `DirtyKind::Paint` for the scroll widget itself (to update the transform + scrollbar), but NOT propagate dirtiness to children. The child's scene node remains valid.

- [x] **Layout cache interaction:** `ScrollWidget::child_natural_size()` (scroll/mod.rs:174) caches the child layout keyed by `viewport: Rect`. Under the old approach, `draw()` computed `child_bounds` with the scroll offset baked in, but `child_natural_size()` was called with `ctx.bounds` (the viewport). Under the new approach, `draw()` uses `ctx.bounds` (unscrolled) for child bounds — same as the layout call. This is correct: the layout cache key matches the draw bounds. No change needed to `child_natural_size()`.

- [x] **Scrollbar draw:** The scrollbar (drawn by `self.draw_scrollbar()` at scroll/mod.rs:269) must still update on scroll — the thumb position changes. The scroll widget's own `SceneNode` is invalidated on scroll (it wraps the translate + scrollbar), but the child's node stays valid. The scroll widget's `draw()` must NOT cache the entire output as one node — it must cache the child separately from the scrollbar so only the scrollbar redraws on scroll.

---

## 04.2 DrawList Transform Command

**File(s):** `oriterm_ui/src/draw/draw_list.rs`

Add translate transform commands to the DrawList, analogous to PushClip/PopClip.

- [x] Add `DrawCommand::PushTranslate` and `DrawCommand::PopTranslate` variants:
  ```rust
  /// Push a 2D translation transform onto the transform stack.
  PushTranslate { dx: f32, dy: f32 },
  /// Pop the most recent translation transform.
  PopTranslate,
  ```

- [x] Add `push_translate()` and `pop_translate()` methods to `DrawList`, with stack depth tracking (same pattern as clip stack).

- [x] Transform stacks compose: nested scroll widgets produce nested translates. The GPU converter applies cumulative transforms.

---

## 04.3 GPU Converter Support

**File(s):** `oriterm/src/gpu/draw_list_convert/mod.rs` (the `convert_draw_list` function), `oriterm/src/gpu/window_renderer/draw_list.rs` (calls into it)

**File size prerequisite:** `draw_list_convert/mod.rs` is **488 lines** -- just 12 lines under the 500-line hard limit. Adding `PushTranslate`/`PopTranslate` match arms and transform stack management will exceed it. **Before implementing 04.3, extract one of these into a submodule:**
- `draw_list_convert/text.rs` — `convert_text()` + `emit_text_glyph()` (~100 lines)
- `draw_list_convert/shapes.rs` — `convert_rect()` + `convert_line()` + helpers (~140 lines)

Recommended: extract `text.rs` since text conversion is self-contained. Add `mod text;` to `draw_list_convert/mod.rs` and `use text::{convert_text, convert_icon};` to keep the public API unchanged.

The GPU draw list converter (`convert_draw_list()` in `draw_list_convert/mod.rs`, called by `append_ui_draw_list_with_text` in `window_renderer/draw_list.rs`) must handle the new transform commands.

- [x] When `PushTranslate { dx, dy }` is encountered, push `(dx, dy)` onto a transform stack.
- [x] All subsequent rect positions, text positions, icon positions, and line positions are offset by the cumulative transform.
- [x] `PopTranslate` pops the stack.
- [x] **Clip-translate interaction:** The scroll widget's own clip rect (the viewport bounds) is NOT affected by the translate — it defines the visible window and stays fixed. However, any `PushClip` commands emitted by child widgets inside the scroll container (e.g. a nested container with `clip_children: true`) ARE affected by the cumulative translate — they are in content-space, not viewport-space. The GPU converter must apply the active translate to child clip rects but NOT to the scroll widget's own viewport clip.
  - Implementation: `PushClip` commands emitted AFTER a `PushTranslate` get their rect offset by the cumulative translate. The scroll widget emits `PushClip(viewport)` BEFORE `PushTranslate`, so the viewport clip is not translated.
  - This matches CSS behavior: the scroll container's `overflow: hidden` clip is in viewport space, but children's `overflow: hidden` clips are in content space.

---

## 04.4 Completion Checklist

- [x] `PushTranslate`/`PopTranslate` draw commands are defined and handled by the GPU converter
- [x] `ScrollWidget::draw()` uses translate instead of bounds shifting
- [x] Scrolling the Settings dialog does NOT call `Widget::draw()` on any child widget (verified by draw counter)
- [x] Scrollbar renders correctly during scroll (updates position via its own draw, not child rebuild)
- [x] Nested scroll widgets compose transforms correctly
- [x] **Mouse coordinate translation (critical):** `ScrollWidget::handle_mouse()` (scroll/mod.rs:272-346) translates mouse event coordinates by shifting `child_bounds` by the scroll offset. This is the event-space coordinate transform, separate from the draw-space transform. The draw path uses `PushTranslate`; the event path continues to use bounds-shifting. These two systems must agree:
  - `draw()`: child draws at `(bounds.x, bounds.y)`, translated by `PushTranslate(-offset_x, -offset)`
  - `handle_mouse()`: child receives events with `bounds = (bounds.x - offset_x, bounds.y - offset, content_w, content_h)`
  - Both produce the same mapping from screen-space mouse position to child-local position. The draw transform shifts rendered output; the event bounds shift hit-test space. Both must apply the same offset.
  - Test: click at a screen-space position that maps to a specific child widget. Verify the same child receives the event under both the old bounds-shifting draw and the new translate draw.
- [x] `DrawList` output produces identical GPU output whether using translate or bounds shifting (behavioral equivalence test)
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Scrolling the Settings dialog 100px calls zero `Widget::draw()` methods on content widgets. The GPU receives an updated translate transform and replayed cached draw commands. Frame time for scroll is <1ms (no shaping, no layout, no draw traversal).
