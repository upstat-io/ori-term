---
section: "04"
title: "Production Wiring & Composed Tests"
status: not-started
reviewed: true
goal: "Wire the status bar into the production window pipeline, update grid origin offsets for the new tab bar height, update all affected golden references, and add composed golden tests showing the full main window chrome."
inspired_by:
  - "oriterm/src/app/redraw/ (single-pane and multi-pane render paths)"
  - "oriterm/src/gpu/visual_regression/settings_dialog.rs (composed golden test pattern)"
  - "oriterm/src/app/window_context.rs (WindowContext owns WindowRoot + renderer)"
depends_on: ["01", "02", "03"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Wire Status Bar into Window Pipeline"
    status: not-started
  - id: "04.2"
    title: "Update Grid Origin Offsets"
    status: not-started
  - id: "04.3"
    title: "Update Existing Golden References"
    status: not-started
  - id: "04.4"
    title: "Composed Golden Tests"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Production Wiring & Composed Tests

**Status:** Not Started
**Goal:** The status bar renders in the production window below the terminal content. The grid origin accounts for the new 36px tab bar height. All existing golden references are updated. Composed golden tests prove the full chrome renders correctly.

**Context:** Sections 01-03 implement individual components (tab bar, status bar, frame/panes). This section wires them together: the status bar must appear in the actual terminal window, the grid layout must account for the new tab bar height (36px down from 46px), and composed golden tests must capture the full window chrome (tab bar + terminal + status bar + borders).

**Reference implementations:**
- **redraw/mod.rs**: Main render entry point — calls tab bar, terminal grid, search bar
- **window_context**: Owns `WindowRoot`, `WindowRenderer`, `FrameInput` — constructs the render pipeline per frame
- **settings_dialog.rs**: Pattern for composed golden tests (`build_dialog_scene` + `render_dialog_to_pixels`)

**Depends on:** Sections 01, 02, 03 (all components must exist before wiring).

---

## 04.1 Wire Status Bar into Window Pipeline

**File(s):** `oriterm/src/app/window_context.rs`, `oriterm/src/app/redraw/`, `oriterm/src/app/mod.rs`

The status bar widget needs to be created, updated with terminal data, and rendered in the window pipeline.

- [ ] **Status bar ownership**: Store in `WindowContext` alongside `tab_bar: TabBarWidget`. The `WindowContext` struct is in `oriterm/src/app/window_context.rs` (line 31). The `TabBarWidget` is stored as `pub(super) tab_bar: TabBarWidget` (line 39). The `StatusBarWidget` follows the same pattern.
- [ ] Add `pub(super) status_bar: StatusBarWidget` field to `WindowContext` (after `terminal_grid` at line 40)
- [ ] Add `use oriterm_ui::widgets::status_bar::{StatusBarWidget, StatusBarData};` to `window_context.rs`
- [ ] Update `WindowContext::new()` (line 103) to accept a `StatusBarWidget` parameter and store it. The constructor currently takes `(window, tab_bar, terminal_grid, renderer)` — add `status_bar: StatusBarWidget` as the 4th parameter (before `renderer`).
- [ ] Update ALL call sites of `WindowContext::new()`:
  - `oriterm/src/app/init/mod.rs` — initial window creation
  - `oriterm/src/app/window_management.rs` — new window creation on tear-off
  - Search for other call sites with `WindowContext::new(`
  - At each call site, construct `StatusBarWidget::new(logical_w, &UiTheme::dark())` and pass it.
- [ ] **Update status bar data each frame**: Add a helper method `update_status_bar_data()` on `App` or inline in the redraw path. Data sources:
  - Shell name: from the focused pane's process name. The mux snapshot has `snapshot.title` — use that, or fall back to "shell". Note: process name detection may not be implemented yet — check `PaneSnapshot` fields. If not available, use `"zsh"` as a placeholder and add a TODO.
  - Pane count: count layouts from `compute_pane_layouts()`. For single-pane: "1 pane". For multi-pane: `format!("{} panes", layouts.len())`.
  - Grid size: from `frame.content_cols` and `frame.content_rows` (or `wl.cols` and `wl.rows` from `compute_window_layout`). Format as `"{cols}\u{00d7}{rows}"`.
  - Encoding: "UTF-8" (hardcoded — all modern terminals use UTF-8)
  - Term type: from `self.config.terminal.term_type` (if it exists) or "xterm-256color" as default
- [ ] Call `ctx.status_bar.set_data(data)` before painting in both redraw paths.
- [ ] **Render the status bar**: Add `draw_status_bar()` method to `impl App` in `draw_helpers.rs` (currently 211 lines — adding ~40 stays under 500). Follow the same pattern as `draw_tab_bar()`:
  ```rust
  pub(in crate::app::redraw) fn draw_status_bar(
      status_bar: &StatusBarWidget,
      renderer: &mut WindowRenderer,
      scene: &mut Scene,
      bounds: Rect,  // from wl.status_bar_rect converted to logical coords
      scale: f32,
      gpu: &GpuState,
      theme: &UiTheme,
      text_cache: &TextShapeCache,
  ) {
      let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), text_cache, scale);
      scene.clear();
      let mut ctx = DrawCtx {
          measurer: &measurer, scene, bounds,
          now: Instant::now(), theme, icons: None,
          interaction: None, widget_id: None, frame_requests: None,
      };
      status_bar.paint(&mut ctx);
      renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);
  }
  ```
  Note: `bounds` comes from `wl.status_bar_rect` (physical pixels from `compute_window_layout`) divided by scale factor to get logical coords. The scene is `ctx.chrome_scene` (reused for tab bar, search bar, status bar). Each component clears and re-appends -- GPU instances accumulate in the prepared frame.
- [ ] Call `draw_status_bar()` from `handle_redraw()` (redraw/mod.rs) and `handle_redraw_multi_pane()` (multi_pane/mod.rs), after tab bar drawing, before window border. Gate on `self.config.window.show_status_bar` (new config field).
- [ ] **Edge case**: When `TabBarPosition::Bottom` is configured, both the tab bar and status bar want to be at the bottom. In this case, hide the status bar (or position it above the tab bar). For the initial implementation, hide the status bar when `tab_bar_position == Bottom` — this matches the mockup which only shows `TabBarPosition::Top`.
- [ ] **Config option**: Add `show_status_bar: bool` field to `WindowConfig` (config/mod.rs line 187) with `#[serde(default = "default_true")]` and default `true`. Add `fn default_true() -> bool { true }` helper. Add tests:
  - **Test: `status_bar_default_true`** — Verify `WindowConfig::default().show_status_bar == true`.
  - **Test: `status_bar_toml_false`** — Parse `[window]\nshow_status_bar = false` and verify the field is `false`.
  - **Test: `status_bar_toml_missing_defaults_true`** — Parse `[window]` with no `show_status_bar` and verify the field is `true`.
- [ ] **Resize handling**: Update `handle_resize()` in `chrome/resize.rs` (line 185-188) — alongside `ctx.tab_bar.set_window_width(logical_w)`, add `ctx.status_bar.set_window_width(logical_w)`.
- [ ] **Theme updates**: In `apply_theme` handler (keyboard_input/overlay_dispatch.rs lines 132-137), alongside `ctx.tab_bar.apply_theme(...)`, add `ctx.status_bar.apply_theme(&self.ui_theme)`.
- [ ] **Init**: In `init/mod.rs` and `window_management.rs`, construct the status bar widget and pass to `WindowContext::new()`.

- [ ] **Search bar overlap**: The search bar overlay renders at the top of the terminal grid. It should NOT overlap the status bar at the bottom. Verify: the search bar's y position is computed from `chrome_height + grid_offset`, not from `viewport_h - search_bar_h`. No change needed if the search bar is top-anchored (which it is).
- [ ] **Status bar position must match layout**: The `draw_status_bar()` helper receives its bounds from `wl.status_bar_rect` (from `compute_window_layout()`). This single source of truth eliminates position drift between the layout engine and the draw helper. Verify in a golden test that no gap or overlap exists between the grid bottom and status bar top.
- [ ] **File size check**: `draw_helpers.rs` is currently 211 lines. Adding `draw_status_bar()` (~40-60 lines) brings it to ~270 lines. Well under 500-line limit.

**Validation:** Status bar appears at the bottom of the terminal window with correct data.

---

## 04.2 Update Grid Origin Offsets
**File(s):** `oriterm/src/app/chrome/mod.rs` (primary — `compute_window_layout()`), `oriterm/src/app/chrome/resize.rs`, `oriterm/src/app/window_management.rs`

The terminal grid position is computed by `compute_window_layout()` in `oriterm/src/app/chrome/mod.rs`. This function builds a `Column { TabBar(fixed), Grid(fill) }` layout descriptor and runs the flexbox solver. All grid origin and sizing flows through this single function. Changing tab bar height from 46px to 36px and adding a status bar requires modifying THIS function, not scattered offset calculations.

- [ ] **Tab bar height change is automatic**: The `compute_window_layout()` function (chrome/mod.rs line 121) takes `tab_bar_height: f32` as a parameter. Callers pass `ctx.tab_bar.metrics().height`. When Section 01 changes `TAB_BAR_HEIGHT` to 36.0, the layout automatically adjusts. Verified call sites:
  - `chrome/resize.rs` line 75: `compute_window_layout(viewport_w, viewport_h, &cell, scale, hidden, tb_h)`
  - `window_management.rs` line 41 and line 163: same pattern
  - `init/mod.rs` line 146: same pattern
  - All use `ctx.tab_bar.metrics().height` — no hardcoded `46.0` in production call sites. The only hardcoded `46.0` values are in `chrome/tests.rs` (test assertions — see below).
- [ ] **Update chrome/tests.rs**: The tests hardcode `46.0` as the tab bar height parameter (e.g., line 98: `compute_window_layout(1920, 1080, &cell, 1.0, false, 46.0)`). After Section 01 changes the default to 36.0:
  - Update ALL test calls that pass `46.0` to use `36.0` (or better: use a named constant `const TEST_TAB_BAR_HEIGHT: f32 = 36.0;`)
  - Lines affected: 98, 113, 128, 144, 157, 168, 169, 196, 211, 212
  - Also update `origin_integer_*` tests that compute `grid_origin_y(46.0, ...)` to use `36.0`
  - Also update the `origin_integer_for_all_common_dpi_scales` test (line 68) which uses `chrome_height = 46.0`
- [ ] **Add status bar to layout**: Modify `compute_window_layout()` to accept `status_bar_height: f32` and `border_inset: f32` parameters. Compute the inset viewport first, then build the 3-element flex layout inside it:
  ```rust
  let inset_px = (border_inset * scale).round();
  let viewport = Rect::new(
      inset_px, inset_px,
      viewport_w as f32 - 2.0 * inset_px,
      viewport_h as f32 - 2.0 * inset_px,
  );
  let status_bar_h_px = (status_bar_height * scale).round();
  let root = LayoutBox::flex(
      Direction::Column,
      vec![
          // Tab bar: fixed height.
          LayoutBox::leaf(0.0, tab_bar_h_px)
              .with_width(SizeSpec::Fill)
              .with_height(SizeSpec::Fixed(tab_bar_h_px)),
          // Terminal grid: fills remaining space.
          LayoutBox::leaf(0.0, 0.0)
              .with_width(SizeSpec::Fill)
              .with_height(SizeSpec::Fill),
          // Status bar: fixed height at bottom.
          LayoutBox::leaf(0.0, status_bar_h_px)
              .with_width(SizeSpec::Fill)
              .with_height(SizeSpec::Fixed(status_bar_h_px)),
      ],
  );
  let layout = compute_layout(&root, viewport);
  ```
  Grid rect is `layout.children[1].rect`. Tab bar rect is `layout.children[0].rect`. Status bar rect is `layout.children[2].rect`. All rects are in physical pixels, inset by the border width.
- [ ] **Update `#[expect(clippy::too_many_arguments)]` reason string** on `compute_window_layout()`: the function goes from 6 to 8 parameters. Update the `reason = "..."` string to: `"window layout: viewport size, cell metrics, scale, tab bar visibility + height, status bar height, border inset"`. If the parameter count grows further in the future, refactor into a `WindowLayoutInput` struct.
- [ ] Update ALL call sites of `compute_window_layout()` to pass the two new parameters (`status_bar_height`, `border_inset`):
  - `chrome/resize.rs` line 75: add `let sb_h = if self.config.window.show_status_bar { STATUS_BAR_HEIGHT } else { 0.0 };` and `let border_inset = if ctx.window.is_maximized() { 0.0 } else { 2.0 };`
  - `window_management.rs` lines 41, 163: same
  - `init/mod.rs` line 146: same
  - Import `STATUS_BAR_HEIGHT` from `oriterm_ui::widgets::status_bar` (or define as a constant in `chrome/mod.rs`)
- [ ] **Critical: border inset affects ALL content, not just the grid.** The `compute_window_layout` inset shifts the entire flex layout (tab bar + grid + status bar) inward. The tab bar flex element starts at `y = inset_px`, not y=0. Add `tab_bar_rect: Rect` and `status_bar_rect: Rect` to the `WindowLayout` return struct so callers know where to position the tab bar and status bar. The tab bar draw bounds must use `tab_bar_rect` instead of `(0, 0, logical_width, height)`:
  - Update `draw_tab_bar()` in `draw_helpers.rs` to accept a `tab_bar_bounds: Rect` parameter (computed from `wl.tab_bar_rect` converted to logical coords) instead of hardcoding `Rect::new(0.0, 0.0, logical_width, height)`.
  - Update `draw_status_bar()` similarly to accept bounds from `wl.status_bar_rect`.
  - Without this, the tab bar paints at y=0 and the accent bar (top 2px) is hidden by the window border. The mockup places all content INSIDE the border.
- [ ] `GRID_PADDING` (8.0, chrome/mod.rs line 93) remains as grid-internal padding, separate from the window border inset
- [ ] **Add new tests to chrome/tests.rs**:
  - **Test: `layout_with_status_bar`** — Call `compute_window_layout(1920, 1080, &cell, 1.0, false, 36.0, 22.0, 0.0)`. Verify grid rect height < grid rect height from `compute_window_layout(1920, 1080, &cell, 1.0, false, 36.0, 0.0, 0.0)`. Verify `grid_rect.y()` is unchanged (tab bar + padding). Verify rows are fewer.
  - **Test: `layout_with_border_inset`** — Call with `border_inset: 2.0`. Verify `grid_rect.x() >= inset_px + pad` and `grid_rect.y() >= inset_px + chrome_h + pad`. Verify grid dimensions are smaller than without inset. Verify `tab_bar_rect.x() == inset_px` and `tab_bar_rect.y() == inset_px` (tab bar is inset from window edge).
  - **Test: `layout_status_bar_hidden`** — Call with `status_bar_height: 0.0`. Verify result matches the old 2-element layout (backward compatible).
  - **Test: `layout_border_inset_zero_when_maximized`** — Call with `border_inset: 0.0`. Verify grid rect starts at (pad, chrome_h + pad), same as current behavior. This confirms maximized mode is backward compatible.
  - **Test: `layout_status_bar_plus_border_inset`** — Both active simultaneously. Verify grid gets fewer rows and is inset. Verify no overlap between grid bottom and status bar top.
  - **Test: `layout_status_bar_integer_origin`** — With `status_bar_height: 22.0` at fractional DPI (1.25x), verify status bar physical height rounds to integer pixels.
- [ ] **Update all existing chrome tests**: The function signature changes from 6 parameters to 8 (adding `status_bar_height: f32` and `border_inset: f32`). All existing tests must pass `0.0, 0.0` for the new parameters to preserve existing behavior. Update the tab bar height parameter from `46.0` to `36.0` simultaneously.
- [ ] Run `./test-all.sh` to verify no regressions. Run `cargo test -p oriterm --test architecture` specifically.

**Validation:** Terminal grid renders at the correct position with the new tab bar and status bar heights. No grid content is cut off or misaligned.

---

## 04.3 Update Existing Golden References

**File(s):** `oriterm/tests/references/*.png`

The tab bar height change (46px → 36px) will affect existing golden tests that include the tab bar. The status bar addition may affect composed tests.

- [ ] Identify all existing golden tests affected by the tab bar height change (note: `tab_bar_emoji.png` is already handled in Section 01.7):
  - Any test that renders with a tab bar origin offset
  - Any test that includes the tab bar in its rendered output
- [ ] For each affected test:
  - Run the test to see if it fails (pixel mismatch)
  - If it fails: regenerate with `ORITERM_UPDATE_GOLDEN=1`
  - Visually inspect the new reference to verify correctness
  - Commit the updated reference PNG
- [ ] **Do NOT regenerate references blindly** — inspect each one to verify the change is expected (height change, not a regression)
- [ ] Tests that render ONLY the grid (no chrome) should NOT be affected — verify these still pass unchanged

**Validation:** All existing golden tests pass with correct references. Only tab-bar-related references changed.

---

## 04.4 Composed Golden Tests

**File(s):** `oriterm/src/gpu/visual_regression/main_window.rs` (NEW), `oriterm/src/gpu/visual_regression/mod.rs`

Write golden tests that render the full main window chrome: tab bar + terminal grid + status bar + borders.

- [ ] Create `oriterm/src/gpu/visual_regression/main_window.rs` module with `#![cfg(all(test, feature = "gpu-tests"))]` at the top
- [ ] Add `mod main_window;` to `visual_regression/mod.rs`
- [ ] **Build a composed rendering helper**. This is the most complex golden test — it renders tab bar (UI font), terminal grid (terminal font), and status bar (UI font) into a single frame. Approach:
  - Use `headless_tab_bar_env()` pattern (from `tab_bar_icons.rs`) which loads BOTH terminal font (`FontCollection`) AND UI font (`UiFontSizes`). This gives a `WindowRenderer` that can render both chrome and grid content.
  - Compute layout via `compute_window_layout()` to get correct grid positioning.
  - Paint tab bar at y=0 via `build_scene` + `append_ui_scene_with_text` (same as production).
  - Paint grid content via `prepare()` + grid origin from layout (same as production `handle_redraw`).
  - Paint status bar at bottom via `build_scene` + `append_ui_scene_with_text`.
  - Optionally paint window border via `append_window_border()`.
  - Render frame and read back pixels.
  ```rust
  fn render_main_window(
      gpu: &GpuState,
      pipelines: &GpuPipelines,
      renderer: &mut WindowRenderer,
      tabs: Vec<TabEntry>,
      active_tab: usize,
      status_data: StatusBarData,
      grid_text: &str,
      width: u32,
      height: u32,
  ) -> Vec<u8>
  ```
- [ ] **Test: `main_window_single_pane_96dpi`** — Full window with 1 tab, single pane, status bar
  - Size: 800x600 at 96 DPI
  - Tab bar: 1 active tab ("zsh")
  - Grid: simple text content (use `FrameInput::test_grid` pattern from existing tests)
  - Status bar: "zsh | 1 pane | 80x24 | UTF-8 | xterm-256color"
  - Window border: 2px border_strong
  - Verifies: all chrome elements render at correct positions with correct styling
- [ ] **Test: `main_window_3tabs_96dpi`** — Full window with 3 tabs (1 active, 1 modified)
  - Tab bar: 3 tabs with mockup content
  - Status bar populated
  - Verifies: tab bar features (accent bar, separators, modified dot) in composed context
- [ ] **Test: `main_window_192dpi`** — Same as single_pane at 192 DPI
  - Catches DPI scaling regressions in the composed layout
  - All dimensions double: 72px tab bar, 44px status bar, 4px borders
- [ ] **Test: `main_window_no_status_bar_96dpi`** — Full window with `show_status_bar: false`. Verifies that the grid expands to fill the space where the status bar would be. Catches regressions where status bar space is reserved even when hidden.
- [ ] **Test: `main_window_hidden_tab_bar_96dpi`** — Full window with `tab_bar_position: Hidden`. Verifies grid starts at the top (y=0 + padding) with no tab bar. Status bar still at bottom. Catches regressions in the hidden tab bar + status bar combination.
- [ ] Run `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests -- main_window` to generate references
- [ ] Visually inspect against the HTML mockup (open both side by side)
- [ ] Commit reference PNGs
- [ ] `/tpr-review` checkpoint

**Validation:** All `main_window_*` golden tests pass. The composed output visually matches `mockups/main-window-brutal.html`.

---

## 04.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 04.N Completion Checklist

- [ ] Status bar renders in production window at correct position
- [ ] Status bar data populated from terminal state (shell, panes, grid size, encoding, term type)
- [ ] Grid origin updated for 36px tab bar height
- [ ] Grid available height accounts for status bar presence/absence
- [ ] Config option `window.show_status_bar` controls visibility
- [ ] All existing golden tests pass (updated references where needed)
- [ ] 5+ composed golden tests pass with committed reference PNGs (single pane, 3 tabs, 192dpi, no status bar, hidden tab bar)
- [ ] No regressions in grid rendering, terminal behavior, or input handling
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** The full main window (tab bar + terminal + status bar) renders correctly in production. Composed golden tests match the mockup. All existing tests pass.
