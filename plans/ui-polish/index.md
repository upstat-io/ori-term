---
reroute: true
name: "UI Polish"
full_name: "2D Framework Polish: Z-Index, Clipping, Animation Quality"
status: queued
order: 1
---

# UI Polish Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: GPU Scissor Rect Support
**File:** `section-01-gpu-scissor.md` | **Status:** Not Started

```
scissor, clip, PushClip, PopClip, clip_stack, set_scissor_rect
convert_draw_list, draw_list_convert, InstanceWriter, RenderPass
wgpu::RenderPass::set_scissor_rect, clip rect, clip region
ClipSegment, TierClips, record_draw_clipped, multi-writer
PreparedFrame, ui_clips, overlay_clips
draw_list_convert/clip.rs, DrawBindings
```

---

### Section 02: Tab Bar Clipping
**File:** `section-02-tab-clipping.md` | **Status:** Not Started

```
tab show-through, tab bleed, tab overlap, z-index, z-order
push_clip, pop_clip, draw_tab, tab_rect, clip_children
tab bar draw order, painter's algorithm, tab stacking
```

---

### Section 03: Color Lerp & Animated Hover
**File:** `section-03-color-animation.md` | **Status:** Not Started

```
Color lerp, Lerp for Color, AnimatedValue<f32>, AnimatedValue<Color>
hover transition, hover_progress, close_btn_opacity
tab hover bg, close button fade, smooth color, color interpolation
bell_phase, tab_hover_bg, inactive_bg, button_hover_bg
widget/animation.rs, set_hover_hit
```

---

### Section 04: Tab Open/Close Animations
**File:** `section-04-tab-lifecycle-anim.md` | **Status:** Not Started
**Note:** Soft dependency on Section 03 — the `widget/animation.rs` extraction in Section 03 must complete first to keep `widget/mod.rs` under 500 lines.

```
tab open animation, tab close animation, width animation
opacity fade, width expand, width shrink, tab_width, TabBarLayout
slide duration, dynamic duration, distance-proportional
SLIDE_DURATION, start_close_slide, start_reorder_slide
width_multipliers, closing_tabs, closing_complete
tab_positions, per_tab_widths, binary search, cumulative sum
TabBarLayout Copy removal, non-Copy layout
```

---

### Section 05: Dragged Tab Elevation
**File:** `section-05-drag-elevation.md` | **Status:** Not Started

```
drag overlay, backing rect hack, drop shadow, elevation
draw_dragged_tab_overlay, drag_visual, RectStyle::with_shadow
Shadow, tab drag, floating tab, opaque backing
widget/drag_draw.rs (if draw.rs exceeds 500 lines)
```

---

### Section 06: Verification
**File:** `section-06-verification.md` | **Status:** Not Started

```
visual regression, test matrix, animation test, performance
clippy, build, test-all, frame time, animation correctness
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | GPU Scissor Rect Support | `section-01-gpu-scissor.md` |
| 02 | Tab Bar Clipping | `section-02-tab-clipping.md` |
| 03 | Color Lerp & Animated Hover | `section-03-color-animation.md` |
| 04 | Tab Open/Close Animations | `section-04-tab-lifecycle-anim.md` |
| 05 | Dragged Tab Elevation | `section-05-drag-elevation.md` |
| 06 | Verification | `section-06-verification.md` |
