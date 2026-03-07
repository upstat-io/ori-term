---
section: "02"
title: Tab Bar Clipping
status: complete
goal: "Each tab's content is clipped to its tab rect — no show-through between adjacent tabs"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "Per-Tab Clip Rects"
    status: complete
  - id: "02.2"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Tab Bar Clipping

**Status:** Not Started
**Goal:** Each tab's draw content (background, title text, close button) is clipped to the tab's bounding rect. Adjacent tabs never bleed into each other. The active tab renders on top of inactive tabs with correct z-ordering via draw order.

**Context:** The tab bar's `draw_tab()` method at `oriterm_ui/src/widgets/tab_bar/widget/draw.rs:79` draws tab content (background rect, label text via `draw_tab_label`, close button via `draw_close_button`) without any clip rect. When tabs are narrow, title text or close button icons can overflow the tab bounds and overlap into adjacent tab space. This is the visible "show-through" bug.

**Depends on:** Section 01 (GPU scissor rect support — `PushClip`/`PopClip` must actually work).

---

## 02.1 Per-Tab Clip Rects

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`

**File size note:** `draw.rs` is 480 lines. Section 02 adds only ~4 lines (well within budget), but Sections 03 and 05 also modify this file. The first section to push past 500 must extract `draw_dragged_tab_overlay` into `widget/drag_draw.rs`.

Wrap each `draw_tab()` call in a clip rect matching the tab's bounds.

- [x] In `draw_tab()`, add `push_clip(tab_rect)` before drawing content and `pop_clip()` after:
  ```rust
  fn draw_tab(&self, ctx: &mut DrawCtx<'_>, index: usize, strip: &TabStrip) {
      let tab = &self.tabs[index];
      let x = self.layout.tab_x(index) + self.anim_offset(index);
      let tab_rect = Rect::new(x, strip.y, self.layout.tab_width, strip.h);

      ctx.draw_list.push_clip(tab_rect);

      // ... existing draw code (bg rect, label, close button) ...

      ctx.draw_list.pop_clip();
  }
  ```
- [x] Ensure `push_clip` is called BEFORE `push_layer` in `draw_tab()`, and `pop_clip` AFTER `pop_layer` — clip wraps the entire tab content including the layer boundary
- [x] Verify the dragged tab overlay (`draw_dragged_tab_overlay`) does NOT need a clip — it floats freely and should be allowed to extend beyond tab bounds for visual polish
- [x] Verify new-tab button and dropdown button don't need clips — they have fixed positions and don't overlap tabs

---

## 02.2 Completion Checklist

- [x] `draw_tab()` wraps content in `push_clip(tab_rect)` / `pop_clip()`
- [x] Clip wraps layer boundary: `push_clip` before `push_layer`, `pop_clip` after `pop_layer`
- [x] Tab title text is clipped at tab boundaries
- [x] Close button is clipped at tab boundaries
- [x] Active tab renders on top of inactive tabs (draw order unchanged)
- [x] Dragged tab overlay is not clipped (floats freely)
- [x] Clip rect includes `anim_offset` — tab clips move correctly during slide animations
- [x] No visual regression in tab bar appearance at normal widths
- [x] `./clippy-all.sh` — no warnings
- [x] `./test-all.sh` — all pass
- [x] `./build-all.sh` — cross-compilation succeeds

**Exit Criteria:** With 20+ tabs open (narrow widths), title text and close buttons are cleanly clipped at tab boundaries. No content from one tab bleeds into adjacent tabs. Visually matches Chrome's tab strip behavior.
