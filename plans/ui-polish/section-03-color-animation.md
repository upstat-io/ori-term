---
section: "03"
title: Color Lerp & Animated Hover
status: not-started
goal: "Tab bar hover bg and close button visibility animate smoothly instead of instant swaps"
inspired_by:
  - "Chrome tab hover transition (~150ms ease-out)"
  - "WindowControlButton hover animation (already uses AnimatedValue<f32>)"
depends_on: []
sections:
  - id: "03.1"
    title: "Lerp for Color"
    status: complete  # already exists in color/mod.rs:111
  - id: "03.2"
    title: "Animated Tab Hover"
    status: not-started
  - id: "03.3"
    title: "Animated Close Button Visibility"
    status: not-started
  - id: "03.4"
    title: "Tests"
    status: not-started
  - id: "03.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Color Lerp & Animated Hover

**Status:** Not Started
**Goal:** Tab bar hover and close-button visibility transitions animate smoothly. Hovering a tab fades the background color over ~100ms. The close button fades in/out on hover enter/leave.

**Context:** The `AnimatedValue<T>` primitive in `oriterm_ui/src/animation/mod.rs` handles smooth interpolation with interruption, requiring `T: Lerp`. `Color` already implements the `Lerp` trait (in `oriterm_ui/src/color/mod.rs:111`), so `AnimatedValue<Color>` can be constructed today. However, the tab bar does not use it — hover state is resolved as an instant color swap in `draw_tab()`. The window control buttons already use `AnimatedValue<f32>` for hover progress — the same pattern should apply to tab hover.

**Reference implementations:**
- **WindowControlButton** (`oriterm_ui/src/widgets/window_chrome/controls.rs`): Uses `AnimatedValue<f32>` with 100ms `EaseOut` for hover progress. This is the exact pattern to follow.
- **Chrome**: Tab hover transitions are ~150ms ease-out.

**Depends on:** Nothing — `Lerp` trait, `Lerp for Color`, and `AnimatedValue<T>` already exist.

---

## 03.1 Lerp for Color — ALREADY COMPLETE

**File(s):** `oriterm_ui/src/color/mod.rs`

`Lerp` for `Color` already exists at `oriterm_ui/src/color/mod.rs:111`. It implements per-channel linear interpolation directly on struct fields. `Color` is `Copy` (derives `Clone, Copy`). Tests already exist in `oriterm_ui/src/color/tests.rs` (lines 188-221) covering endpoints and midpoint.

- [x] `Lerp` impl for `Color` exists in `oriterm_ui/src/color/mod.rs` (not `animation/mod.rs` — the orphan rule requires impl-at-type-site)
- [x] `Color` is `Copy` (required by `Lerp` bound)
- [x] Tests cover `lerp(BLACK, WHITE, 0.0)`, `lerp(BLACK, WHITE, 1.0)`, `lerp(BLACK, WHITE, 0.5)`, and alpha interpolation

**No work needed for this sub-section.**

---

## 03.2 Animated Tab Hover

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/mod.rs`, `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`

**File size warning:** `widget/mod.rs` (468 lines) and `widget/draw.rs` (480 lines) are both near the 500-line limit. Before adding animation fields and drawing logic:
- **`mod.rs`**: Extract animation state and methods into `widget/animation.rs`.
- **`draw.rs`**: Verify line count after Section 02 changes; if over 480, extract `draw_dragged_tab_overlay` into its own submodule (following the `controls_draw.rs` pattern already established).

Add per-tab hover animation using `AnimatedValue<f32>` (hover progress 0.0→1.0, matching the WindowControlButton pattern).

- [ ] Add `hover_progress: Vec<AnimatedValue<f32>>` to `TabBarWidget` — one per tab, 100ms `EaseOut`
- [ ] Resize `hover_progress` in `set_tabs()` to match tab count — use `resize_with()` to grow (new entries at 0.0) and `truncate()` to shrink
- [ ] When `set_tabs()` is called, reset `hover_progress` to all-zero values (reorder and add/remove both invalidate index-based animation state)
- [ ] On `set_hover_hit()`: when hover enters tab `i`, call `hover_progress[i].set(1.0, now)`; when hover leaves, call `hover_progress[i].set(0.0, now)`
- [ ] Add `now: Instant` parameter to `set_hover_hit()` (or a new `set_hover_hit_animated(&mut self, hit: TabBarHit, now: Instant)` method) so animation start time can be recorded
- [ ] In `draw_tab()`, compute background color by lerping between `inactive_bg` and `tab_hover_bg` using `hover_progress[index].get(ctx.now)`:
  ```rust
  let hover_t = self.hover_progress.get(index)
      .map(|p| p.get(ctx.now))
      .unwrap_or(0.0);
  // NOTE: requires `use crate::animation::Lerp;` in scope for Color::lerp().
  let bg = if strip.active {
      self.colors.active_bg
  } else if strip.bell > 0.0 {
      self.colors.bell_pulse(strip.bell)
  } else {
      Color::lerp(self.colors.inactive_bg, self.colors.tab_hover_bg, hover_t)
  };
  ```
- [ ] Set `ctx.animations_running` when any `hover_progress` is animating to request continued redraws

---

## 03.3 Animated Close Button Visibility

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`

The close button currently appears/disappears instantly based on hover state. Animate its opacity.

- [ ] Add `close_btn_opacity: Vec<AnimatedValue<f32>>` to `TabBarWidget` — one per tab, 80ms `EaseOut`
- [ ] Resize in `set_tabs()` to match tab count (same reset-on-set_tabs strategy as `hover_progress`)
- [ ] On hover enter tab: `close_btn_opacity[i].set(1.0, now)`. On hover leave: `close_btn_opacity[i].set(0.0, now)`
- [ ] Active tab close button is always fully visible (opacity 1.0, no animation)
- [ ] In `draw_close_button()`, modulate the close icon color alpha by `close_btn_opacity[index].get(ctx.now)`:
  ```rust
  let opacity = if strip.active {
      1.0
  } else {
      self.close_btn_opacity.get(index)
          .map(|o| o.get(ctx.now))
          .unwrap_or(0.0)
  };
  if opacity < 0.01 { return; }  // Skip drawing invisible close button
  let fg = self.colors.close_fg.with_alpha(opacity);
  ```
- [ ] Set `ctx.animations_running` when any `close_btn_opacity` is animating

---

## 03.4 Tests

**File(s):** `oriterm_ui/src/widgets/tab_bar/tests.rs`

- [x] Test `Lerp for Color`: already covered in `oriterm_ui/src/color/tests.rs` (lines 188-221)
- [ ] Test `AnimatedValue<Color>` smoke test (infra validation — the tab bar uses `AnimatedValue<f32>` for hover progress, but `AnimatedValue<Color>` should work for future consumers): create, set target color, verify interpolation at 0%, 50%, 100% of duration
- [ ] Test hover progress: call `set_hover_hit()` on tab index, then query `hover_progress[i].get(now)` at `t=0` (should be 0.0), `t=50ms` (should be mid-transition), `t=100ms+` (should be 1.0); verify leaving hover starts reverse transition
- [ ] Test close button opacity: verify inactive tab has `close_btn_opacity` of 0.0 by default; after hover enter, opacity reaches 1.0 after 80ms; active tab always returns 1.0

---

## 03.5 Completion Checklist

- [x] `Lerp` impl for `Color` — already exists at `oriterm_ui/src/color/mod.rs:111`
- [ ] `AnimatedValue<Color>` works correctly (infra exists, needs first consumer)
- [ ] Tab hover bg animates smoothly (~100ms)
- [ ] Close button fades in on hover enter (~80ms)
- [ ] Close button fades out on hover leave (~80ms)
- [ ] Active tab close button is always visible (no animation)
- [ ] Animation flag set to request continued redraws
- [ ] `hover_progress` and `close_btn_opacity` Vecs resized correctly in `set_tabs()`
- [ ] Tab reorder (via drag) resets animation Vecs (no stale animation on wrong tab)
- [ ] `set_hover_hit()` accepts `Instant` parameter (or uses `Instant::now()` internally)
- [ ] No visual regression in tab bar appearance
- [ ] `./clippy-all.sh` — no warnings
- [ ] `./test-all.sh` — all pass
- [ ] `./build-all.sh` — cross-compilation succeeds

**Exit Criteria:** Hovering over an inactive tab produces a smooth ~100ms color fade. Moving the cursor away produces a smooth fade back. The close button fades in when hovering a tab and fades out when leaving, with no instant visibility jumps.
