---
section: "05"
title: Dragged Tab Elevation
status: complete
goal: "Dragged tab has a drop shadow and no longer relies on an opaque backing rect hack"
inspired_by:
  - "Chrome dragged tab elevation with drop shadow"
  - "RectStyle::with_shadow (already supported by DrawList)"
depends_on: []
sections:
  - id: "05.1"
    title: "Drop Shadow on Dragged Tab"
    status: complete
  - id: "05.2"
    title: "Remove Backing Rect Hack"
    status: complete
  - id: "05.3"
    title: "Completion Checklist"
    status: complete
---

# Section 05: Dragged Tab Elevation

**Status:** Not Started
**Goal:** The dragged tab renders with a subtle drop shadow conveying elevation, and the opaque backing rect hack is removed. The dragged tab visually floats above the strip without needing to cover underlying content with a solid color fill.

**Context:** `draw_dragged_tab_overlay` at `oriterm_ui/src/widgets/tab_bar/widget/draw.rs:286` (backing rect at lines 297-300) draws an "opaque backing rect" to hide the tab content underneath:

```rust
// Opaque backing rect (hides underlying content from the fg pass).
let backing = Rect::new(visual_x, strip.y, w, strip.h);
ctx.draw_list.push_rect(backing, RectStyle::filled(self.colors.bar_bg));
```

This is a z-index workaround. The proper fix requires either: (a) drawing the dragged tab slot as empty in the normal pass, or (b) using the compositor layer system for true layer separation. Option (a) is simpler and sufficient.

**Reference implementations:**
- **Chrome**: Dragged tab has a drop shadow and renders above other tabs. The slot where the tab was is either empty or shows an insertion marker.
- **Shadow support**: `oriterm_ui/src/draw/shadow.rs` defines `Shadow { offset_x, offset_y, blur_radius, spread, color }` and `RectStyle::with_shadow()` already supports it. Shadow opacity is encoded in the `color` alpha channel, not a separate field.

**Depends on:** Nothing.

---

## 05.1 Drop Shadow on Dragged Tab

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`

**File size note:** `draw.rs` is 480 lines. If Sections 02/03 push it past 500, extract `draw_dragged_tab_overlay` into `widget/drag_draw.rs` before modifying it here.

Add a subtle drop shadow to the dragged tab overlay.

- [x] In `draw_dragged_tab_overlay`, replace the flat `RectStyle::filled(active_bg)` with one that includes a shadow:
  ```rust
  use crate::draw::Shadow;

  let shadow = Shadow {
      offset_x: 0.0,
      offset_y: 2.0,
      blur_radius: 8.0,
      spread: 0.0,
      color: Color::BLACK.with_alpha(0.25),
  };
  let style = RectStyle::filled(self.colors.active_bg)
      .with_per_corner_radius(ACTIVE_TAB_RADIUS, ACTIVE_TAB_RADIUS, 0.0, 0.0)
      .with_shadow(shadow);
  ```
- [x] Verify the shadow renders correctly via the existing shadow conversion in `convert_draw_list` (shadows emit an expanded shadow rect before the main rect)

---

## 05.2 Remove Backing Rect Hack

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`

The backing rect exists because the dragged tab's original slot still draws content underneath. Fix: skip drawing the dragged tab's original slot entirely.

The current `draw()` method already skips the dragged tab in the normal pass:
```rust
if i == self.active_index || self.is_dragged(i) {
    continue;
}
```

So the dragged slot is already empty in the inactive pass. The backing rect is only needed to cover the **bar background** behind the dragged tab overlay. With a shadow, the active background + shadow will naturally occlude the bar background.

- [x] Remove the opaque backing rect from `draw_dragged_tab_overlay`:
  ```rust
  // DELETE these lines:
  // let backing = Rect::new(visual_x, strip.y, w, strip.h);
  // ctx.draw_list.push_rect(backing, RectStyle::filled(self.colors.bar_bg));
  ```
- [x] Verify the dragged tab renders correctly over the bar background without the backing rect

- [x] Verify that bar_bg shows through at rounded corners — this is EXPECTED behavior (matches Chrome). If bar_bg and active_bg are too similar, increase shadow `blur_radius` or `color` alpha for more visual separation.
- [x] Test with both light and dark themes to confirm shadow visibility

---

## 05.3 Completion Checklist

- [x] Dragged tab has a drop shadow (`offset_y: 2.0`, `blur_radius: 8.0`, `color: BLACK.with_alpha(0.25)`)
- [x] Opaque backing rect removed from `draw_dragged_tab_overlay`
- [x] Dragged tab visually floats above the tab strip
- [x] Bar background showing through rounded corners is acceptable (matches Chrome behavior)
- [x] Shadow visible in both light and dark themes
- [x] Shadow does not clip at tab bar boundaries
- [x] Dragged tab overlay drawn outside any clip region (verified by draw() call order — step 7, after all clipped tabs)
- [x] `./clippy-all.sh` — no warnings
- [x] `./test-all.sh` — all pass
- [x] `./build-all.sh` — cross-compilation succeeds

**Exit Criteria:** Dragging a tab shows a subtle shadow underneath it, conveying elevation. The dragged tab no longer relies on a solid-color backing rect to hide underlying content. Visual polish matches Chrome's dragged tab appearance.
