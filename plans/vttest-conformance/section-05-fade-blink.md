---
section: "05"
title: "Fade Blink"
status: not-started
reviewed: true
goal: "Cursor blink uses smooth fade animation via instance alpha pipeline, verified by multi-frame capture tests"
inspired_by:
  - "WezTerm ColorEase (wezterm-gui/src/colorease.rs)"
  - "WezTerm cursor_blink uniform (wezterm-gui/src/termwindow/render/draw.rs:233)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "ColorEase Type"
    status: not-started
  - id: "05.2"
    title: "Instance Alpha Integration"
    status: not-started
  - id: "05.3"
    title: "Wire Into Render Pipeline"
    status: not-started
  - id: "05.4"
    title: "Multi-Frame Capture Tests"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Fade Blink

**Status:** Not Started
**Goal:** Cursor blink smoothly fades in and out using eased opacity through the existing instance alpha pipeline, matching WezTerm's visual quality. Verified by multi-frame capture tests that assert the opacity ramp.

**Context:** The current `CursorBlink` at `oriterm_ui/src/animation/cursor_blink/mod.rs` is a pure on/off toggle — the cursor is either fully visible or fully hidden. WezTerm uses `ColorEase` with configurable easing curves for smooth fade. oriterm's cursor is rendered as bg-pipeline instances via `build_cursor()` -> `push_cursor(rect, color, alpha)` at `prepare/emit.rs`. The `alpha` parameter already exists and is supported by the bg shader's premultiplied alpha blending. The implementation replaces the binary `cursor_blink_visible: bool` parameter with a continuous `cursor_opacity: f32` computed by `ColorEase`.

**Reference implementations:**
- **WezTerm** `wezterm-gui/src/colorease.rs`: `ColorEase` type with `intensity_continuous()` returning `f32` opacity and next update `Instant`. Separate `in_duration`/`out_duration` and `in_function`/`out_function` easing curves.
- **WezTerm** `wezterm-gui/src/termwindow/render/draw.rs:233`: cursor blink state converted to `ColorEaseUniform`, passed to shader via uniform buffer.

**Depends on:** None (self-contained -- uses existing headless GPU test pipeline).

---

## 05.1 ColorEase Type

**File(s):** `oriterm_ui/src/animation/cursor_blink/mod.rs` (replace `CursorBlink` with `ColorEase`)

Create a `ColorEase` struct that computes continuous opacity from elapsed time using easing functions.

```rust
pub struct ColorEase {
    /// Duration of the fade-in phase (visible plateau + fade-out follows).
    in_duration: Duration,
    /// Duration of the fade-out phase.
    out_duration: Duration,
    /// Easing function for fade-in.
    in_ease: EasingFunction,
    /// Easing function for fade-out.
    out_ease: EasingFunction,
    /// Cycle start time (reset on keypress).
    epoch: Instant,
}

pub enum EasingFunction {
    Linear,
    EaseInOut,  // Cubic: 3t^2 - 2t^3 (smoothstep)
}
```

Key methods:
```rust
/// Returns opacity in [0.0, 1.0].
pub fn intensity(&self) -> f32

/// Returns the Instant of the next visual change.
/// During fade transitions: ~16ms (animation frame rate).
/// During plateaus (fully on/off): end of the plateau.
pub fn next_change(&self) -> Instant
```

The blink cycle is: **visible plateau** (opacity=1.0 for `in_duration`) -> **fade out** (opacity 1.0->0.0 using `out_ease` over a short transition, e.g., 200ms) -> **hidden plateau** (opacity=0.0 for `out_duration`) -> **fade in** (opacity 0.0->1.0 using `in_ease` over ~200ms) -> repeat. The transition duration is a fraction of each phase, not the full phase.

- [ ] Rename `CursorBlink` to `ColorEase` (or keep the name and evolve the API — the name matters less than the API)
- [ ] Add `EasingFunction` enum: `Linear`, `EaseInOut` (smoothstep: `3t^2 - 2t^3`)
- [ ] Add configurable fade-in/fade-out easing functions (default: both `EaseInOut`)
- [ ] Replace `is_visible() -> bool` with `intensity() -> f32` returning opacity in [0.0, 1.0]
- [ ] Keep `reset()` to restart cycle on keypress (returns to full opacity)
- [ ] Replace `next_toggle() -> Instant` with `next_change() -> Instant` that returns the next time the opacity will change visually. During fade transitions, this is the next animation frame (~16ms at 60fps). During the fully-on or fully-off plateau, this is the end of the plateau (same as current `next_toggle`).
- [ ] Default: 530ms in_duration + 530ms out_duration (matching current CursorBlink's 530ms interval), EaseInOut easing, with ~200ms fade transitions
- [ ] Keep `update() -> bool` but change semantics: returns true when opacity changed enough to warrant a redraw (threshold: > 0.01 change)
- [ ] Keep `set_interval()` for config reload
- [ ] Unit tests: verify opacity ramps from 1.0 -> 0.0 -> 1.0 over one cycle (use epoch backdating as in existing tests at `cursor_blink/tests.rs:22`)
- [ ] Unit tests: verify `reset()` returns to full opacity
- [ ] Unit tests: verify `next_change()` returns ~16ms during fade, ~530ms during plateau
- [ ] Unit tests: verify EaseInOut produces correct values at t=0 (0.0), t=0.5 (0.5), t=1.0 (1.0)
- [ ] Unit tests: verify `set_interval()` works for config reload (both in_duration and out_duration update)
- [ ] Define the transition fraction as a named constant: e.g., `const FADE_FRACTION: f32 = 0.38` means the fade occupies 38% of each phase duration (~200ms out of 530ms). Document the visual rationale.
- [ ] Unit test: `intensity_monotonic_during_fade_out` — sample intensity at 10 evenly-spaced points during the fade-out transition, verify each sample is <= the previous (monotonically decreasing)
- [ ] Unit test: `intensity_monotonic_during_fade_in` — same for fade-in (monotonically increasing)
- [ ] Unit test: `intensity_plateau_stable` — during the visible plateau (between reset and fade-out start), intensity is exactly 1.0; during the hidden plateau, intensity is exactly 0.0

---

## 05.2 Instance Alpha Integration

**File(s):** `oriterm/src/gpu/prepare/emit.rs:166` (`build_cursor`), `oriterm/src/gpu/instance_writer/mod.rs:279` (`push_cursor`), `oriterm/src/gpu/prepare/mod.rs` (all `cursor_blink_visible` call sites)

**Risk note:** This subsection changes a `bool` to `f32` across 10+ call sites in the GPU prepare pipeline. The compiler catches type mismatches, but semantic risks remain: (1) the block cursor color inversion logic at `resolve_cell_colors()` depends on whether the cursor is "visible" -- with opacity this becomes a threshold check; (2) the `HollowBlock` cursor shape emits 4 outline rectangles that all need the same alpha; (3) the multi-pane path at `redraw/multi_pane/mod.rs:284` combines per-pane focus with blink visibility -- needs careful translation to f32 math.

The cursor is rendered as instances in the bg pipeline. `build_cursor()` at `emit.rs:166` calls `push_cursor(rect, color, 1.0)` — the third argument is already an `alpha: f32`. No shader changes are needed. The fix is to:
1. Change `cursor_blink_visible: bool` to `cursor_opacity: f32` throughout the prepare pipeline.
2. Pass the opacity through to `build_cursor()` instead of hard-coded `1.0`.

- [ ] Change `build_cursor()` signature at `emit.rs:166`: add `opacity: f32` parameter (10th param), pass to all `push_cursor()` calls instead of hard-coded `1.0`. Update the `#[expect(clippy::too_many_arguments)]` reason string. All 4 cursor shapes (Block, Bar, Underline, HollowBlock's 4 outline rects) must pass the same opacity.
- [ ] Change all `cursor_blink_visible: bool` parameters to `cursor_opacity: f32` in:
  - `resolve_cell_colors()` at `prepare/mod.rs:115`
  - `prepare_frame_shaped_into()` at `prepare/mod.rs:208`
  - `fill_frame_shaped()` at `prepare/mod.rs:294`
  - `fill_frame_incremental()` at `prepare/dirty_skip/mod.rs:258`
  - `fill_frame()` at `prepare/unshaped.rs:66` (line 71 is the `cursor_blink_visible` param)
  - `update_cursor_only()` at `prepare/mod.rs:249`
  - `WindowRenderer::prepare()` at `window_renderer/frame_prep.rs:54`
  - `WindowRenderer::prepare_pane_into()` at `window_renderer/multi_pane.rs:52` (line 57 is the `cursor_blink_visible` param)
- [ ] Update `resolve_cell_colors()`: use `cursor_opacity > 0.5` instead of `cursor_blink_visible` for the `is_block_cursor_cell` check -- at opacity <= 0.5, the cursor is fading out and text should revert to normal colors for readability
- [ ] Update `render_to_pixels_with_origin()` at `visual_regression/mod.rs:132`: the call `renderer.prepare(input, gpu, pipelines, origin, true, true)` passes `true` for cursor_blink_visible — change to `1.0_f32` when the type becomes `f32`. This propagates to all existing visual regression tests
- [ ] Update all 4 `build_cursor()` call sites to pass the new opacity parameter:
  - `prepare/mod.rs:267` (in `update_cursor_only`)
  - `prepare/mod.rs:474` (in `fill_frame_shaped`)
  - `prepare/unshaped.rs:179` (in `fill_frame`)
  - `prepare/dirty_skip/mod.rs:483` (in `fill_frame_incremental`)
- [ ] Update the per-pane cursor visibility at `app/redraw/multi_pane/mod.rs:284`: change `let pane_cursor_visible = cursor_blink_visible && layout.is_focused` to `let pane_cursor_opacity = if layout.is_focused { cursor_opacity } else { 0.0 }`
- [ ] Verify cursor renders at full opacity (1.0) when blink is disabled
- [ ] Verify cursor is invisible (0.0) at the nadir of the blink cycle
- [ ] Verify intermediate opacities (0.5) produce visually smooth semi-transparent cursor via the existing bg shader alpha blending
- [ ] **WARNING: Two `multi_pane` files.** `app/redraw/multi_pane/mod.rs:284` computes `pane_cursor_visible` (bool AND) -- this is the app-layer gate and MUST change to f32. `gpu/window_renderer/multi_pane.rs:285` has `push_cursor(..., 1.0)` for pane borders -- these are NOT cursor blink instances and must NOT be affected by opacity. Only change the app-layer computation.
- [ ] `/tpr-review` checkpoint

---

## 05.3 Wire Into Render Pipeline

**File(s):** `oriterm/src/app/mod.rs:166`, `oriterm/src/app/redraw/mod.rs:245-264`, `oriterm/src/app/redraw/multi_pane/mod.rs:94`, `oriterm/src/app/event_loop.rs:398+486`, `oriterm/src/app/constructors.rs:128`

Replace the binary `CursorBlink` with the continuous-opacity `ColorEase`.

- [ ] Replace `cursor_blink: CursorBlink` field at `app/mod.rs:166` with `cursor_blink: ColorEase`
- [ ] Update constructor at `constructors.rs:128`: create `ColorEase` with configurable cycle duration and EaseInOut easing
- [ ] Update `redraw/mod.rs:245-246`: replace the boolean computation `!blinking_now || !self.blinking_active || self.cursor_blink.is_visible()` with a f32 computation: if `!blinking_now || !self.blinking_active` then `1.0_f32`, else `self.cursor_blink.intensity()`. Pass opacity (f32) to `prepare()`
- [ ] Update `redraw/multi_pane/mod.rs:94`: same pattern -- replace `!self.blinking_active || self.cursor_blink.is_visible()` with f32 opacity (1.0 when not blinking, else `self.cursor_blink.intensity()`). Note: line 284 further gates per-pane cursor visibility based on `layout.is_focused` -- update to multiply opacity by 0.0 when unfocused
- [ ] Update `event_loop.rs:398`: `cursor_blink.update()` returns bool for dirty check -- `ColorEase` should compare prev opacity vs current opacity, mark dirty if changed
- [ ] Update `event_loop.rs:486`: rename `next_toggle` to `next_blink` (or similar), change from `cursor_blink.next_toggle()` to `cursor_blink.next_change()`. The `ControlFlowInput.next_toggle` field at `event_loop_helpers/mod.rs:270` also needs renaming. During fade transitions `next_change()` returns ~16ms (animation frame rate); during plateaus it returns ~530ms (same as old `next_toggle`). The existing `compute_control_flow()` logic at line 307-311 does not need structural changes -- `WaitUntil(next_toggle)` naturally adapts
- [ ] Update all `cursor_blink.reset()` call sites (9 total: `keyboard_input/action_dispatch.rs:100`, `keyboard_input/mod.rs:286`, `keyboard_input/ime.rs:144`, `mouse_input.rs:344`, `event_loop.rs:170`, `redraw/mod.rs:500`, `redraw/mod.rs:505`, `redraw/multi_pane/mod.rs:273`, `redraw/multi_pane/mod.rs:553`) -- `ColorEase` must support the same reset semantic (restart cycle from full visible)
- [ ] Update `keyboard_input/mod.rs:325`: `cursor_hidden_by_blink: self.blinking_active && !self.cursor_blink.is_visible()` -- change to use opacity threshold, e.g., `self.blinking_active && self.cursor_blink.intensity() < 0.01` (cursor is "hidden" when opacity is near zero)
- [ ] Update `config_reload/mod.rs:279`: `set_interval()` on `ColorEase`
- [ ] Ensure cursor is always visible when `CURSOR_BLINKING` mode is off (opacity = 1.0)
- [ ] Ensure cursor shows as hollow block when window is unfocused (no blink, opacity = 1.0)
- [ ] Verify idle CPU: during fade transitions, wakeups at animation_fps rate; when fully on or fully off (the two plateaus), wakeups only at phase boundary (same as current CursorBlink). No continuous polling during steady state.
- [ ] Update `ControlFlowInput.next_toggle` field name at `event_loop_helpers/mod.rs:270` and all its usages in `event_loop_helpers/tests.rs` (6 references: lines 14, 56, 60, 82, 106, 122) to match the new naming (e.g., `next_blink_change`)
- [ ] Verify performance invariant: `compute_control_flow()` tests still pass -- WaitUntil scheduling must be correct
- [ ] Add unit test: `compute_control_flow_fade_blink_wakeup` -- when blinking_active=true and next_blink_change is 16ms in the future (fade transition), verify WaitUntil is 16ms, not 530ms
- [ ] Add unit test: `compute_control_flow_plateau_blink_wakeup` -- when blinking_active=true and next_blink_change is 530ms in the future (plateau), verify WaitUntil is 530ms
- [ ] Verify `./test-all.sh` passes after ALL changes in this section -- this section touches 15+ files across 2 crates

---

## 05.4 Multi-Frame Capture Tests

**File(s):** `oriterm/src/gpu/visual_regression/mod.rs` (new test alongside existing `cursor_shapes` test)

Test the blink animation by rendering multiple frames with different cursor opacity values and verifying the cursor pixel alpha changes.

The approach: render `FrameInput::test_grid()` with cursor visible, calling `build_cursor()` with different opacity values. Read back pixels at the cursor position and verify brightness/alpha.

- [ ] Create a test helper that renders a frame with a specific cursor opacity (pass opacity directly to the prepare pipeline, not through `ColorEase` -- this tests the GPU path in isolation)
- [ ] Capture frame with opacity=1.0 (cursor fully visible)
- [ ] Capture frame with opacity=0.5 (cursor semi-transparent)
- [ ] Capture frame with opacity=0.0 (cursor invisible)
- [ ] Assert cursor pixel alpha at opacity=1.0 is fully opaque
- [ ] Assert cursor pixel alpha at opacity=0.0 matches the background (no cursor visible)
- [ ] Assert cursor pixel alpha at opacity=0.5 is intermediate between the two
- [ ] Golden image for each opacity level (visual verification of fade quality)
- [ ] Separately test `ColorEase` unit tests (in `oriterm_ui/src/animation/cursor_blink/tests.rs`): verify opacity ramps from 1.0 -> 0.0 -> 1.0 over one cycle, verify `reset()` returns to full opacity, verify `next_change()` scheduling
- [ ] Test function: `cursor_opacity_full` -- render with opacity=1.0, read cursor pixel, assert RGBA alpha channel is 255 (fully opaque)
- [ ] Test function: `cursor_opacity_zero` -- render with opacity=0.0, read cursor pixel, assert it matches the background color (no cursor visible)
- [ ] Test function: `cursor_opacity_half` -- render with opacity=0.5, read cursor pixel, assert alpha is intermediate (roughly 128, with tolerance for premultiplied alpha blending)
- [ ] Update existing tests in `cursor_blink/tests.rs` to use new API: `visible_at_start`, `hidden_after_interval`, `visible_after_two_intervals`, `reset_makes_visible`, `update_returns_true_on_toggle`, `update_returns_false_when_same`, `next_toggle_advances` (now `next_change_advances`), `set_interval_changes_frequency`. These tests change from bool assertions to f32 opacity assertions.

---

## 05.R Third Party Review Findings

- None.

---

## 05.N Completion Checklist

- [ ] `ColorEase` produces smooth opacity curve (unit tests pass)
- [ ] `build_cursor()` passes opacity through to `push_cursor()` alpha parameter
- [ ] All `cursor_blink_visible: bool` parameters converted to `cursor_opacity: f32`
- [ ] Cursor fades smoothly in/out at 60fps (visual verification)
- [ ] Multi-frame capture test passes (3 opacity levels rendered and compared)
- [ ] No regression: cursor visible when blink disabled, hollow when unfocused
- [ ] Idle CPU unchanged: `WaitUntil` scheduling via `next_change()`, no polling
- [ ] `compute_control_flow()` tests still pass
- [ ] Event loop scheduling uses `ColorEase::next_change()` correctly
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Cursor blink produces a visually smooth fade matching WezTerm's quality. Multi-frame capture test verifies opacity ramps from 1.0 → 0.0 → 1.0 across one blink cycle. No increase in idle CPU beyond the blink timer.
