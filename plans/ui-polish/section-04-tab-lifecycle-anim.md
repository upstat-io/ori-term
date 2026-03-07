---
section: "04"
title: Tab Open/Close Animations
status: complete
goal: "Tab open expands from zero width with opacity fade-in; tab close shrinks to zero with opacity fade-out; slide duration scales with distance"
inspired_by:
  - "Chrome tab open/close width animation"
  - "TabSlideState compositor transforms (already in place)"
depends_on: []  # soft dependency on 03 (file-size split), not a hard API dependency
sections:
  - id: "04.1"
    title: "Tab Width Animation State"
    status: complete
  - id: "04.2"
    title: "Tab Open Animation"
    status: complete
  - id: "04.3"
    title: "Tab Close Animation"
    status: complete
  - id: "04.4"
    title: "Dynamic Slide Duration"
    status: complete
  - id: "04.5"
    title: "Tests"
    status: complete
  - id: "04.6"
    title: "Completion Checklist"
    status: complete
---

# Section 04: Tab Open/Close Animations

**Status:** Not Started
**Goal:** When a tab opens, it expands from zero width to its target width over ~200ms with an opacity fade-in. When a tab closes, it shrinks to zero width over ~150ms with an opacity fade-out, and neighboring tabs slide to fill the gap. Slide animations scale their duration proportional to pixel distance.

**Context:** Currently, `TabBarLayout::compute()` recalculates all tab widths instantly when the tab count changes. There is no width transition — tabs appear and disappear at full size. The existing `TabSlideState` handles *position* sliding (neighboring tabs animate to fill gaps after close) but there is no *width* animation.

Chrome handles this by treating the closing tab as a special "phantom" slot that shrinks over time. The layout recalculates around this phantom width each frame until it reaches zero.

**Reference implementations:**
- **Chrome** `tab_strip_layout_helper.cc`: Animates tab width via `TabAnimation` objects. Each tab has an `ideal_bounds` computed from the animated width.
- **TabSlideState** (`oriterm_ui/src/widgets/tab_bar/slide/mod.rs`): Existing compositor-driven position animation. Uses `LayerAnimator::animate_transform()` with `AnimationParams { duration: SLIDE_DURATION (150ms), easing: EaseOut }` to interpolate `Transform2D::translate()` to `identity()`.

**Depends on:** Soft dependency on Section 03's `widget/animation.rs` file extraction (not an API dependency — Section 04 needs the file split to stay under 500 lines). `AnimatedValue<f32>` and `TabSlideState` already exist.

---

## 04.1 Tab Width Animation State

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/mod.rs`, `oriterm_ui/src/widgets/tab_bar/layout.rs`

**File size prerequisite:** Section 03's file-size mitigation (extracting animation state into `widget/animation.rs`) MUST complete before this section begins. Without that extraction, `widget/mod.rs` (468 lines + Section 03 additions) would far exceed 500 lines after Section 04 additions. **This creates a soft ordering dependency: Section 03's structural split before Section 04.**

Add per-tab animated width multiplier. The multiplier goes from 0.0 (collapsed) to 1.0 (full width). The layout uses `tab_width * multiplier` for each tab's allocated space.

- [x] Add `width_multipliers: Vec<AnimatedValue<f32>>` to `TabBarWidget` — one per tab, initialized to 1.0
- [x] Resize `width_multipliers` in `set_tabs()` to match tab count (new entries initialized to 1.0 — new tabs appear at full width by default; `animate_tab_open` is called separately to override)

**Layout API changes required:**

The current `TabBarLayout` assumes uniform tab widths: `tab_x(i) = left_margin + i * tab_width`. With per-tab width multipliers, tab positions become cumulative sums of varying widths. This requires one of:

**(a) Store per-tab positions in `TabBarLayout`** (recommended): Add a `tab_positions: Vec<f32>` field that holds the pre-computed X position of each tab. `tab_x(i)` becomes an index into this Vec. All derived methods (`tabs_end()`, `new_tab_x()`, `dropdown_x()`) read from the last position + last width. `tab_index_at()` uses binary search instead of division.

**(b) Keep cumulative computation in `draw()` only**: Don't change `TabBarLayout`. Compute positions inline during drawing based on multipliers. Downside: hit testing (`tab_index_at`) and interactive_rects() won't account for animation positions.

**Recommended path:** Option (a). Specific changes:

**BREAKING CHANGE:** `TabBarLayout` currently derives `Copy` (`#[derive(Debug, Clone, Copy, PartialEq)]`). Adding `Vec` fields removes `Copy`. The `derive(Copy)` must be removed, and all sites that copy `TabBarLayout` by value must switch to `Clone` or borrow. Audit all consumers before proceeding.

- [x] Remove `Copy` from `TabBarLayout`'s derive list — it can no longer be `Copy` with `Vec` fields
- [x] Audit and fix all `TabBarLayout` consumers that relied on implicit `Copy` semantics
- [x] Add `tab_positions: Vec<f32>` field to `TabBarLayout` — pre-computed cumulative X positions
- [x] Add `per_tab_widths: Vec<f32>` field to `TabBarLayout` — the effective width of each tab (`tab_width * multiplier`)
- [x] Change `TabBarLayout::compute()` to accept `width_multipliers: Option<&[f32]>` — when provided, compute per-tab positions as cumulative sums: `tab_x[0] = left_margin`, `tab_x[i+1] = tab_x[i] + tab_width * multipliers[i]`
- [x] Update `tab_x(index)` to read from `tab_positions[index]` instead of computing `left + index * width`
- [x] Update `tabs_end()` to use `tab_positions[last] + per_tab_widths[last]` (or `left_margin` if empty)
- [x] Update `tab_index_at(x)` to use binary search over `tab_positions` instead of division
- [x] Update `max_text_width()` to accept an index parameter or use minimum tab width
- [x] Verify `tab_width_lock` interaction: if lock is active, multipliers should be ignored (locked width takes precedence)
- [x] In `draw()`, compute current multipliers from `AnimatedValue::get(ctx.now)` and recompute layout with overrides
- [x] When multipliers are all 1.0, skip the override path (zero overhead when idle)

**Design decision — per-tab multiplier vs phantom slot:**

**(a) Per-tab multiplier** (recommended):
Each tab has a `width_multipliers[i]` value animated 0→1 (open) or 1→0 (close). The layout multiplies `tab_width * multiplier[i]` per tab. Simple, no phantom state.

**Why this is best:** Reuses existing `AnimatedValue<f32>`, no new state machine, composable with existing slide animations.

**Trade-off:** Layout must be recomputed each frame during animation (it's already cheap — pure arithmetic on N tabs). `TabBarLayout` grows from a fixed struct to holding per-tab Vecs.

**(b) Phantom slot** (Chrome's approach):
Track a "closing slot" with its own animated width. More complex state management.

**Downside:** Requires a separate concept of "phantom tabs" in the layout, complicating tab indexing.

**Recommended path:** Option (a).

---

## 04.2 Tab Open Animation

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/mod.rs`

When a tab is added, animate its width from 0 to full.

- [x] Add `animate_tab_open(&mut self, index: usize, now: Instant)` method:
  - Set `width_multipliers[index]` to a new `AnimatedValue<f32>` with initial value 0.0
  - Call `.set(1.0, now)` to start the animation (200ms, `EaseOut`)
- [x] Call `animate_tab_open` from the app layer when a new tab is created — calling protocol: `set_tabs(new_tabs)` (initializes new entries at 1.0) → `animate_tab_open(new_index, now)` (overrides to start from 0.0)
- [x] Audit all `set_tabs()` call sites in `oriterm/src/app/` to add `animate_tab_open` calls where appropriate (new tab creation, NOT on reorder or title update)
- [x] In `draw_tab()`, modulate the tab's content opacity by the multiplier (fading in as it expands):
  ```rust
  let width_t = self.width_multipliers.get(index)
      .map(|m| m.get(ctx.now))
      .unwrap_or(1.0);
  // Content opacity fades in faster than width expands
  let content_opacity = (width_t * 2.0).min(1.0);
  ```
- [x] Set `ctx.animations_running` when any width multiplier is animating

---

## 04.3 Tab Close Animation

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/mod.rs`

When a tab is closed, animate its width from full to zero, then remove it.

- [x] Add `animate_tab_close(&mut self, index: usize, now: Instant)` method:
  - Call `width_multipliers[index].set(0.0, now)` (150ms, `EaseOut`)
  - Mark the tab as "closing": add `closing_tabs: Vec<bool>` to `TabBarWidget` (parallel Vec, resized in `set_tabs()`)
- [x] During draw, skip interaction for closing tabs (no hover, no click): check `closing_tabs[i]` before processing hover/click
- [x] After the animation completes (multiplier reaches ~0.0), the app layer removes the tab from the data model by calling `set_tabs()` with the updated list
- [x] Add `closing_complete(&self, now: Instant) -> Option<usize>` method — returns the index of the first tab whose close animation has finished (`width_multipliers[i].get(now) < 0.01 && closing_tabs[i]`). The app layer polls this during redraw and calls `set_tabs()` to remove the finished tab.
- [x] Coordinate with `TabSlideState`: the position slide for neighboring tabs should start AFTER the tab is removed (sequential flow, not simultaneous)

**Co-implementation requirement with TabSlideState:**
The close slide (`start_close_slide`) currently runs AFTER the tab is already removed from the widget state. With width animation, the flow becomes:
1. User closes tab → `animate_tab_close(index, now)` starts width shrink
2. Width animation runs for 150ms (tab stays in widget, marked as closing)
3. Animation completes → app layer removes tab via `set_tabs()`
4. `start_close_slide()` runs for remaining neighbors (they slide to fill the gap left by the now-removed tab)

This is sequential, not simultaneous. The width shrink replaces the need for neighbors to slide during the shrink — the neighbors' positions naturally update as the closing tab's width decreases each frame (because layout is recomputed with multipliers each frame).

---

## 04.4 Dynamic Slide Duration

**File(s):** `oriterm_ui/src/widgets/tab_bar/slide/mod.rs`

Replace the fixed 150ms slide duration with distance-proportional timing.

- [x] Replace `const SLIDE_DURATION: Duration = Duration::from_millis(150)` with a function:
  ```rust
  /// Compute slide duration proportional to pixel distance.
  ///
  /// Base: 80ms. Scales up to 200ms for large distances (5+ tab widths).
  /// Clamped to [80ms, 200ms] range.
  fn slide_duration(distance_px: f32, tab_width: f32) -> Duration {
      let slots = (distance_px.abs() / tab_width).max(1.0);
      let ms = 80.0 + slots * 25.0;
      Duration::from_millis(ms.clamp(80.0, 200.0) as u64)
  }
  ```
- [x] Update `create_slide_layers` to accept duration parameter
- [x] Update `start_close_slide` and `start_reorder_slide` to compute duration from distance

---

## 04.5 Tests

**File(s):** `oriterm_ui/src/widgets/tab_bar/tests.rs`, `oriterm_ui/src/widgets/tab_bar/slide/tests.rs`

- [x] Test open width multiplier: call `animate_tab_open(index, now)`, verify `width_multipliers[index].get(now)` is 0.0 at `t=0`, mid-value at `t=100ms`, and 1.0 at `t=200ms+`
- [x] Test close width multiplier: call `animate_tab_close(index, now)`, verify `width_multipliers[index].get(now)` is 1.0 at `t=0`, mid-value at `t=75ms`, and ~0.0 at `t=150ms+`
- [x] Test layout with width overrides: call `compute()` with `width_multipliers: Some(&[1.0, 0.5, 1.0])`, verify `tab_x(1)` equals `left + tab_width` and `tab_x(2)` equals `left + tab_width + tab_width * 0.5` (cumulative sums, not `index * width`)
- [x] Test `slide_duration()`: 1 slot → 105ms, 3 slots → 155ms, 10 slots → capped at 200ms
- [x] Test `tab_x()` with multipliers: tab 0 at 0.5 multiplier → tab 1 starts at `left + tab_width * 0.5` (not `left + tab_width`)
- [x] Test `tab_index_at()` with multipliers: binary search correctly finds tab under cursor when widths are non-uniform
- [x] Test `tabs_end()` with multipliers: returns correct total width accounting for all per-tab widths
- [x] Test `closing_complete()`: returns `None` when animation in progress, `Some(index)` when finished
- [x] Test `set_tabs()` resizes `width_multipliers` and `closing_tabs` correctly
- [x] Test `tab_width_lock` interaction with multipliers: lock takes precedence, multipliers ignored

---

## 04.6 Completion Checklist

- [x] `Copy` derive removed from `TabBarLayout`; all consumers updated
- [x] `width_multipliers` tracks per-tab animated width
- [x] Tab open expands from 0→1 width over ~200ms
- [x] Tab close shrinks from 1→0 width over ~150ms
- [x] Content opacity fades with width animation
- [x] Layout recomputes with width overrides during animation
- [x] `TabBarLayout` stores `tab_positions: Vec<f32>` and `per_tab_widths: Vec<f32>`
- [x] `tab_x()`, `tabs_end()`, `new_tab_x()`, `dropdown_x()`, `tab_index_at()` updated for variable widths
- [x] `closing_tabs: Vec<bool>` tracks closing state per tab
- [x] `closing_complete()` returns index of finished close animations
- [x] `set_tabs()` resizes all parallel Vecs (`width_multipliers`, `closing_tabs`, `hover_progress`, `close_btn_opacity`)
- [x] `tab_width_lock` + multipliers interaction defined and tested
- [x] Slide duration proportional to distance (80–200ms range)
- [x] Zero overhead when no animations are running
- [x] `./clippy-all.sh` — no warnings
- [x] `./test-all.sh` — all pass
- [x] `./build-all.sh` — cross-compilation succeeds

**Exit Criteria:** Opening a new tab shows a smooth width expansion from zero with content fading in. Closing a tab shows a smooth width collapse with content fading out, followed by neighbors sliding to fill the gap. The slide speed feels proportional to the distance moved.
