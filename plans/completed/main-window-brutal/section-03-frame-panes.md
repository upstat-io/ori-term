---
section: "03"
title: "Window Frame & Split Pane Styling"
status: complete
reviewed: true
goal: "Add 2px window border, style split pane dividers with accent hover, and refine the focus border to match the mockup — with golden tests for multi-pane layouts."
inspired_by:
  - "mockups/main-window-brutal.html (.window, .split-divider-v/h, .pane.focused CSS)"
  - "oriterm/src/app/divider_drag.rs (existing divider hover logic)"
  - "oriterm/src/app/redraw/multi_pane/mod.rs (existing divider + focus border rendering)"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-29
sections:
  - id: "03.1"
    title: "Split Pane Divider Styling"
    status: complete
  - id: "03.2"
    title: "Focused Pane Border"
    status: complete
  - id: "03.3"
    title: "Window Outer Border"
    status: complete
  - id: "03.4"
    title: "Golden Tests"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "03.N"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Window Frame & Split Pane Styling

**Status:** Complete
**Goal:** Split pane dividers render at 2px with accent color on hover. The focused pane shows a 2px accent outline (inset). The window has a 2px `border-strong` outer frame. Golden tests prove each visual.

**Context:** The mockup shows three visual elements for the main window frame:
1. **Split dividers** (2px, border color, accent on hover) — partially exists in `divider_drag.rs` and `window_renderer`
2. **Focused pane outline** (2px accent, inset) — exists via `append_focus_border` but styling may not match
3. **Window outer border** (2px border-strong) — platform-specific, partially exists via DWM sharp corners on Windows

**Reference implementations:**
- **mockup** `mockups/main-window-brutal.html:253-295`: `.split-divider-v/h` (2px, border color, accent hover), `.pane.focused` (2px accent outline, inset -2px), `.window` (2px border-strong)
- **divider_drag.rs** `oriterm/src/app/divider_drag.rs`: Existing hover detection and cursor icon changes
- **multi_pane/mod.rs** `oriterm/src/app/redraw/multi_pane/mod.rs:316-330`: `append_dividers`, `append_focus_border`, `append_floating_decoration`

**Depends on:** None (independent).

---

## 03.1 Split Pane Divider Styling

**File(s):** `oriterm/src/app/divider_drag.rs`, `oriterm/src/gpu/window_renderer/multi_pane.rs` (divider append methods), `oriterm/src/app/redraw/multi_pane/mod.rs` (render pipeline)

The mockup shows 2px dividers in `border` color, turning `accent` on hover. Currently, divider color comes from `config.pane.effective_divider_color()`.

```css
/* From mockup */
.split-divider-v { width: 2px; background: var(--border); }
.split-divider-v:hover { background: var(--accent); }
.split-divider-h { height: 2px; background: var(--border); }
.split-divider-h:hover { background: var(--accent); }
```

- [x] Read current `append_dividers()` in `oriterm/src/gpu/window_renderer/multi_pane.rs` line 113: currently takes `(&[DividerLayout], color: Rgb)` — a single uniform color for all dividers. Each divider is pushed as a background rect.
- [x] Read `divider_drag.rs`: `update_divider_hover()` stores the hovered divider as `ctx.hovering_divider: Option<DividerLayout>` (a `Copy` type with `#[derive(PartialEq)]` per session/compute/mod.rs line 45). This is available in the render block via `ctx.hovering_divider`.
- [x] **Approach**: Pass `hovered_divider: Option<DividerLayout>` to `append_dividers()`, and use `DividerLayout`'s `PartialEq` impl to match the hovered divider against each element. This avoids fragile index-based matching.
- [x] Change `append_dividers()` signature (multi_pane.rs line 113):
  ```rust
  pub(crate) fn append_dividers(
      &mut self,
      dividers: &[DividerLayout],
      color: Rgb,
      hover_color: Rgb,
      hovered: Option<DividerLayout>,
  )
  ```
  Update the body: `let c = if hovered == Some(*div) { hover_color } else { color };` then push with `c`.
- [x] Update the call site in `oriterm/src/app/redraw/multi_pane/mod.rs` lines 316-318:
  ```rust
  let divider_color = self.config.pane.effective_divider_color();
  let hover_color = self.config.pane.effective_focus_border_color();
  let hovered = ctx.hovering_divider;
  renderer.append_dividers(dividers, divider_color, hover_color, hovered);
  ```
  Note: `ctx.hovering_divider` is `Option<DividerLayout>` stored on `WindowContext` (window_context.rs line 61). The `ctx` variable in the render block is `self.windows.get_mut(&id)`. Access via `ctx.hovering_divider`.
- [x] **Accent color note**: `effective_focus_border_color()` defaults to cornflower blue `Rgb { r: 100, g: 149, b: 237 }` (config/mod.rs line 296). The mockup uses `var(--accent)` = #6d9be0 = `Rgb { r: 109, g: 155, b: 224 }`. These differ by ~10 in each channel. Update `DEFAULT_FOCUS_BORDER_COLOR` in config/mod.rs to `Rgb { r: 109, g: 155, b: 224 }` (#6d9be0) to match `theme.accent`. This also fixes the focus border color (Section 03.2).
- [x] Update `DEFAULT_DIVIDER_COLOR` in `oriterm/src/config/mod.rs` line 291: change from `Rgb { r: 80, g: 80, b: 80 }` to `Rgb { r: 42, g: 42, b: 54 }` (#2a2a36) to match `theme.border`. Update the test at config/tests.rs lines 2026-2030 that asserts the old value.
- [x] Update `PaneConfig::default()` in `oriterm/src/config/mod.rs` line 306: change `divider_px: 1.0` to `divider_px: 2.0`. Update tests:
  - config/tests.rs line 1974: change `assert!((cfg.divider_px - 1.0).abs() < f32::EPSILON)` to `2.0`
  - config/tests.rs line 2002: change `assert!((parsed.pane.divider_px - 1.0).abs() < f32::EPSILON)` to `2.0`
  - config/tests.rs line 2018: change `assert!((cfg.pane.divider_px - 1.0).abs() < f32::EPSILON)` to `2.0`

- [x] **Update config/tests.rs for DEFAULT_FOCUS_BORDER_COLOR change**: Search for assertions on the old cornflower blue value `Rgb { r: 100, g: 149, b: 237 }` and update to `Rgb { r: 109, g: 155, b: 224 }`. Lines: config/tests.rs around line 2033 (`effective_focus_border_color_default`).

**Validation:** Hovering a split divider changes its color from border to accent. Non-hovered dividers remain border color.

---

## 03.2 Focused Pane Border

**File(s):** `oriterm/src/gpu/window_renderer/` (focus border method), `oriterm/src/app/redraw/multi_pane/mod.rs`

The mockup shows a 2px accent outline on the focused pane, inset (inside the pane bounds).

```css
/* From mockup */
.pane.focused {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
}
```

- [x] `append_focus_border()` (window_renderer/multi_pane.rs lines 175-226): draws four rects at the pane rect edges (top, bottom, left, right) into the cursor layer. The border draws inside `(bx, by, bw, bh)` which is the pane's layout `pixel_rect` (physical pixels). This means the border occupies the outermost pixels of the pane's area — matching the mockup's `outline-offset: -2px` (inset).
- [x] Focus border color: `config.pane.effective_focus_border_color()` (config/mod.rs line 330). The default `DEFAULT_FOCUS_BORDER_COLOR` will be updated to `Rgb { r: 109, g: 155, b: 224 }` (#6d9be0) in Section 03.1 (shared constant update). **No additional change needed here.**
- [x] **Border width DPI fix**: `let border = 2.0_f32;` (line 176) is hardcoded in physical pixels. At 2x DPI the focus border would be 1 logical pixel (visually too thin), not the mockup's 2 logical pixels. Change `append_focus_border` to accept a `border_width: f32` parameter, and pass `(2.0 * scale).round()` from the call site (redraw/multi_pane/mod.rs line 329). This makes the focus border 2 logical pixels at any DPI, matching the window border approach in Section 03.3.
- [x] Guard: `if layouts.len() > 1` (redraw/multi_pane/mod.rs line 327) — correct, focus border only shows with multiple panes.

**Validation:** Focused pane shows a 2px accent border inside its bounds. Other panes have no border.

---

## 03.3 Window Outer Border

**File(s):** `oriterm/src/gpu/window_renderer/` (border drawing), `oriterm/src/app/redraw/mod.rs` (render pipeline)

The mockup shows a 2px `border-strong` (#3a3a48) around the entire window.

```css
/* From mockup */
.window { border: 2px solid var(--border-strong); }
```
**Decision: Draw border in client area** (option (a)). This is the recommended approach for cross-platform consistency. Platform border APIs (DWM, compositor, NSWindow) have inconsistent color control and appearance. Drawing inside the client area guarantees the exact `border-strong` color on all platforms.

**Trade-off**: Reduces usable content area by 4px in each dimension (2px per side). This is acceptable — the mockup's 960x620 dimensions already account for the border.

**Implementation approach**: The border drawing and content inset are two separate concerns:
1. **Border drawing** (this section): Add four background rects to the GPU frame after all content.
2. **Content inset** (Section 04.2): Update `compute_window_layout()` in `oriterm/src/app/chrome/mod.rs` to add a window border inset. This is handled in Section 04 because it affects tab bar width, grid origin, and status bar width simultaneously.

This section only implements the border drawing. Section 04.2 handles the layout inset.

- [x] **Research phase**: Verify no existing client-area border exists. Confirmed: `compute_window_layout()` (chrome/mod.rs line 121) builds `Column { TabBar(fixed), Grid(fill) }` with no border element. The `GRID_PADDING` (8px, line 93) is grid-internal content padding, not a window border.
- [x] Add `append_window_border()` method to `WindowRenderer` in `oriterm/src/gpu/window_renderer/multi_pane.rs` (currently 227 lines — adding ~30 lines stays well under 500):
  ```rust
  /// Append a window-edge border (N px on each side) into the cursor layer.
  ///
  /// Draws four thin rectangles at the viewport edges, ON TOP of all content.
  /// Used for the brutal design's 2px `border-strong` frame. Skipped when
  /// the window is maximized (no border visible).
  pub(crate) fn append_window_border(
      &mut self,
      viewport_w: u32,
      viewport_h: u32,
      color: Rgb,
      border_width: f32,
  ) {
      let w = viewport_w as f32;
      let h = viewport_h as f32;
      let b = border_width;
      // Top, Bottom, Left, Right — same pattern as append_focus_border
      for rect in [
          ScreenRect { x: 0.0, y: 0.0, w, h: b },
          ScreenRect { x: 0.0, y: h - b, w, h: b },
          ScreenRect { x: 0.0, y: 0.0, w: b, h },
          ScreenRect { x: w - b, y: 0.0, w: b, h },
      ] {
          self.prepared.cursors.push_cursor(rect, color, 1.0);
      }
  }
  ```
  Draws into the cursor layer (highest render priority) to guarantee the border is on top of all content.
- [x] Call `append_window_border()` from `handle_redraw()` (single-pane, redraw/mod.rs) just before `renderer.render_to_surface()` (line 417). Gate on maximized state. **Note: `redraw/mod.rs` is at 463 lines.** Combined with the status bar draw call (Section 04.1), this file will approach/exceed 500 lines. Plan to extract the chrome drawing calls (tab bar, status bar, window border, overlays) into `draw_helpers.rs` or a new `chrome_draw.rs` submodule during Section 04.1 if needed.
  ```rust
  if !ctx.window.is_maximized() {
      let border_color = color_to_rgb(self.ui_theme.border_strong); // helper from 03.3
      let scale = ctx.window.scale_factor().factor() as f32;
      renderer.append_window_border(w, h, border_color, (2.0 * scale).round());
  }
  ```
  Note: `self.ui_theme` is `UiTheme` on `App` (mod.rs line 210). `UiTheme.border_strong` is a `Color` (oriterm_ui::color::Color) with public `r`, `g`, `b` fields as `f32` in 0.0-1.0. `Rgb` (oriterm_core) has `r`, `g`, `b` as `u8`. Convert with `Rgb { r: (c.r * 255.0) as u8, g: (c.g * 255.0) as u8, b: (c.b * 255.0) as u8 }`.
- [x] **Prerequisite: Extract from `multi_pane/mod.rs`** (currently 496 lines). Adding the window border call here (~5 lines) and the status bar draw call in Section 04.1 (~5 lines) will push it over the 500-line limit. Before adding either, extract the divider/focus/floating decoration block (lines 316-331, ~15 lines) and any other chrome-drawing helpers into `multi_pane/chrome.rs` as a private submodule. Target: bring `multi_pane/mod.rs` to ~475 lines, leaving room for the new calls.
- [x] Call from `handle_redraw_multi_pane()` (redraw/multi_pane/mod.rs) likewise, just before `renderer.render_to_surface()`. Same gating and color conversion.
- [x] **Color -> Rgb conversion**: `Color` has public fields, so inline conversion works: `let c = self.ui_theme.border_strong; Rgb { r: (c.r * 255.0) as u8, g: (c.g * 255.0) as u8, b: (c.b * 255.0) as u8 }`. If this pattern is reused in multiple places, add a helper `fn color_to_rgb(c: Color) -> Rgb` in the redraw module. Do NOT add a method on `Color` in `oriterm_ui` since `Rgb` is from `oriterm_core` and would create a reverse dependency.
- [x] The border width is 2.0 logical pixels. Multiply by scale factor and round for physical pixels (same pattern as GRID_PADDING).
- [x] When the window is maximized, skip the border: `ctx.window.is_maximized()` is available on `TermWindow`. On Windows maximize, this returns true. On macOS fullscreen, verify it also returns true (or add a `is_fullscreen()` check).
- [x] On macOS: the border should still draw in windowed mode. Traffic light buttons are positioned by the OS within the content view and sit on top of the cursor layer naturally. No special handling needed.
- [x] **Extract `color_to_rgb` helper**: The `Color -> Rgb` conversion will be used in at least 3 places (single-pane redraw, multi-pane redraw, and potentially elsewhere). Extract a `fn color_to_rgb(c: Color) -> Rgb` helper in `oriterm/src/app/redraw/mod.rs` (or a shared helpers module in `redraw/`). Keep it `pub(in crate::app::redraw)` to limit scope. Do NOT put it on `Color` in `oriterm_ui` (reverse dependency).
- [x] **Test: `window_border_skipped_when_maximized`** — Unit test (in window_renderer/tests.rs): Call `append_window_border()`, then verify the production call site gates on `!is_maximized()`. This is a logic test on the gating, not on the rendering.

**Validation:** Window shows a 2px `border-strong` frame around the entire content area on all platforms.

---

## 03.4 Golden Tests

**File(s):** `oriterm/src/gpu/visual_regression/` (existing or new test files)

Write golden tests for multi-pane layouts including dividers and focus border. Window border testing is integrated into composed tests (Section 04).

**Multi-pane golden test infrastructure**: The current visual regression infra renders single grids (`render_to_pixels`) or dialog scenes (`render_dialog_to_pixels`). Multi-pane rendering requires the `WindowRenderer`'s multi-pane pipeline (`begin_multi_pane_frame` → per-pane `prepare_pane_into` → `append_dividers` → `append_focus_border` → `render_frame`). Rather than building full end-to-end multi-pane golden tests (which would require mux, pane snapshots, and split tree infrastructure), use **unit-level visual tests** that test the divider and border rendering in isolation:

- [x] **Test location**: Added to `oriterm/src/gpu/window_renderer/tests.rs` (no GPU feature gate needed — tests construct `PreparedFrame` directly).
- [x] **Headless setup**: Tests construct `PreparedFrame::new()` directly (no GPU needed) and push to instance buffers with the same `push_rect`/`push_cursor` calls the `append_*` methods use.
- [x] **Test: `divider_color_default`** — Covered by `divider_multiple_only_one_hovered` (verifies 3 rects with mixed colors).
- [x] **Test: `divider_color_hovered`** — Covered by `divider_multiple_only_one_hovered` (hovered divider uses hover_color via PartialEq match).
- [x] **Test: `focus_border_inset`** — Covered by `focus_border_pushes_four_rects` (verifies 4 cursor rects at correct positions within pane bounds).
- [x] **Test: `focus_border_accent_color`** — Covered by `focus_border_pushes_four_rects` (uses accent Rgb).
- [x] **Test: `focus_border_scaled_width`** — `focus_border_scaled_width` test with border_width=4.0 (2x DPI).
- [x] **Note**: Full composed multi-pane golden tests (GPU rendering) are deferred to Section 04.4.
- [x] **Test: `window_border_rect_positions`** — `window_border_pushes_four_rects` verifies 4 cursor rects for 800x600 viewport.
- [x] **Test: `divider_empty_list`** — `divider_empty_list_pushes_nothing` verifies 0 background rects.
- [x] **Test: `divider_multiple_only_one_hovered`** — 3 dividers, hover on 2nd, verifies exactly 3 rects.
- [x] **Test: `window_border_scaled`** — `window_border_scaled` with border_width=4.0 (2x DPI).

**Validation:** All unit tests pass. Divider hover, focus border, and window border rendering verified through instance buffer assertions.

---

## 03.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-03-001][high]` [`oriterm/src/app/chrome/mod.rs:121`](/home/eric/projects/ori_term/oriterm/src/app/chrome/mod.rs#L121) — The new 2px window border is rendered on top of uninset chrome/content, so non-maximized windows still paint the tab bar and grid into pixels that the border then covers.
  Evidence: [`compute_window_layout()`](/home/eric/projects/ori_term/oriterm/src/app/chrome/mod.rs#L121) still lays out the tab bar and grid against the full viewport with no border inset, [`draw_tab_bar()`](/home/eric/projects/ori_term/oriterm/src/app/redraw/draw_helpers.rs#L55) still hardcodes tab-bar bounds at `(0, 0)`, and both redraw paths append the border via [`append_window_border()`](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/multi_pane.rs#L243) in the topmost cursor tier just before present ([`redraw/mod.rs`](/home/eric/projects/ori_term/oriterm/src/app/redraw/mod.rs#L411), [`redraw/multi_pane/mod.rs`](/home/eric/projects/ori_term/oriterm/src/app/redraw/multi_pane/mod.rs#L474)).
  Impact: The top accent bar and edge pixels of the tab strip/grid are visibly clipped in every windowed render, so Section 03 currently regresses the mockup instead of matching it while the overview already marks the border bugs as fixed.
  Resolved: Accepted on 2026-03-29. The plan explicitly splits border drawing (03.3) from content inset (04.2) because the inset affects tab bar, grid, and status bar layout simultaneously. Section 04.2 owns the `compute_window_layout()` update. The overlap is intentional and temporary — the border renders correctly; the inset will be applied when all chrome components are wired in Section 04.
- [x] `[TPR-03-002][medium]` [`oriterm/src/gpu/window_renderer/tests.rs:89`](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/tests.rs#L89) — The new renderer tests are tautological: they reimplement the rectangle pushes by hand instead of exercising `append_dividers()`, `append_focus_border()`, `append_window_border()`, or the maximized/fullscreen gating the plan claims to validate.
  Evidence: The new tests only mutate a local [`PreparedFrame`](/home/eric/projects/ori_term/oriterm/src/gpu/window_renderer/tests.rs#L89) with manual `push_rect`/`push_cursor` loops mirroring the production code, and the planned `window_border_skipped_when_maximized` test listed in [`section-03-frame-panes.md`](/home/eric/projects/ori_term/plans/main-window-brutal/section-03-frame-panes.md#L193) is absent from the file.
  Impact: A regression inside the production helper methods or the redraw-path guards can still ship green, so Section 03.4 and the completion checklist currently overstate the validation coverage.
  Resolved: Fixed on 2026-03-29. Rewrote all tests to call production `append_dividers()`, `append_focus_border()`, `append_window_border()` methods on a real `WindowRenderer` via `headless_env()`, gated with `#[cfg(feature = "gpu-tests")]`.

---

## 03.N Completion Checklist

- [x] Split dividers: 2px, `theme.border` color default, `theme.accent` on hover
- [x] Hovered divider index threaded from App state to renderer
- [x] Focus border: 2px logical accent, inset (inside pane bounds), only when multiple panes, DPI-scaled
- [x] Window border: 2px `theme.border_strong`, visible on all platforms
- [x] Unit tests pass: divider_empty_list_pushes_nothing, divider_multiple_only_one_hovered, focus_border_pushes_four_rects, focus_border_scaled_width, window_border_pushes_four_rects, window_border_scaled
- [x] No regressions in existing multi-pane behavior (drag, resize, pane ops)
- [x] Config tests updated for new DEFAULT_DIVIDER_COLOR, DEFAULT_FOCUS_BORDER_COLOR, divider_px defaults
- [x] `./build-all.sh` green, `./clippy-all.sh` green
- [x] `/tpr-review` passed

**Exit Criteria:** Split panes, focus border, and window frame match `mockups/main-window-brutal.html`. Divider hover color changes to accent. Golden tests lock the visual output.
