---
section: "02"
title: "Status Bar Widget"
status: in-progress
reviewed: true
goal: "Create a StatusBarWidget in oriterm_ui following all widget conventions, rendering a 22px bar with shell name, pane count, grid size, encoding, and term type — matching mockup exactly, with golden tests."
inspired_by:
  - "mockups/main-window-brutal.html (.status-bar CSS, lines 329-349)"
  - "oriterm_ui/src/widgets/tab_bar/ (widget with theme-derived colors pattern)"
  - "oriterm_ui/src/widgets/label/mod.rs (simple display widget pattern)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "StatusBarWidget Struct & Data Model"
    status: complete
  - id: "02.2"
    title: "Layout & Paint"
    status: complete
  - id: "02.3"
    title: "Theme Integration"
    status: complete
  - id: "02.4"
    title: "WidgetTestHarness Tests"
    status: complete
  - id: "02.5"
    title: "Golden Tests"
    status: in-progress
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Status Bar Widget

**Status:** Not Started
**Goal:** A new `StatusBarWidget` exists in `oriterm_ui/src/widgets/status_bar/`, implements the `Widget` trait, renders a 22px bar matching the mockup's status bar section, has full WidgetTestHarness coverage, and golden tests proving the visual output.

**Context:** The mockup shows a status bar at the bottom of the window with terminal metadata. No status bar widget currently exists in the codebase. This is a completely new widget that must follow all widget conventions established by the UI framework: `Widget` trait impl, `from_theme()` styling, `Sense::none()` (display-only), sibling `tests.rs` file, and WidgetTestHarness coverage.

**Reference implementations:**
- **mockup** `mockups/main-window-brutal.html:329-349`: `.status-bar` CSS — 22px height, `bg-surface` bg, 2px `border` top, 11px font, `text-faint` color, 16px gap between items
- **label widget** `oriterm_ui/src/widgets/label/mod.rs`: Simple display widget pattern (135 lines, implements Widget, uses TextStyle for rendering)
- **tab bar colors** `oriterm_ui/src/widgets/tab_bar/colors.rs`: Theme-derived color struct pattern

**Depends on:** None (independent).

---

## 02.1 StatusBarWidget Struct & Data Model

**File(s):** `oriterm_ui/src/widgets/status_bar/mod.rs` (NEW)

Create the widget struct and data model. The status bar displays terminal metadata items.

```html
<!-- From mockup -->
<div class="status-bar">
    <span class="status-item"><span class="status-accent">zsh</span></span>
    <span class="status-item">3 panes</span>
    <span class="status-item">120&times;30</span>
    <span class="status-separator"></span>
    <span class="status-item">UTF-8</span>
    <span class="status-item"><span class="status-accent">xterm-256color</span></span>
</div>
```

- [x] Create directory `oriterm_ui/src/widgets/status_bar/`
- [x] Create `oriterm_ui/src/widgets/status_bar/mod.rs` with `//!` module doc
- [x] Add `pub mod status_bar;` to `oriterm_ui/src/widgets/mod.rs` (between `sidebar_nav` and `slider` alphabetically, around line 38). The existing pattern is `pub mod name;` with no re-exports — consumers use `oriterm_ui::widgets::status_bar::StatusBarWidget` directly.
- [x] Define `StatusBarData`:
  ```rust
  /// Terminal metadata displayed in the status bar.
  pub struct StatusBarData {
      /// Shell or process name (e.g., "zsh", "bash"). Displayed in accent color.
      pub shell_name: String,
      /// Number of visible panes (e.g., "3 panes", "1 pane").
      pub pane_count: String,
      /// Grid dimensions (e.g., "120\u{00d7}30").
      pub grid_size: String,
      /// Character encoding (e.g., "UTF-8").
      pub encoding: String,
      /// Terminal type (e.g., "xterm-256color"). Displayed in accent color.
      pub term_type: String,
  }
  ```
- [x] Define `StatusBarColors`:
  ```rust
  /// Colors for status bar rendering, derived from UiTheme.
  #[derive(Debug, Clone, Copy, PartialEq)]
  pub struct StatusBarColors {
      pub bg: Color,
      pub border: Color,
      pub text: Color,
      pub accent: Color,
  }

  impl StatusBarColors {
      pub fn from_theme(theme: &UiTheme) -> Self {
          Self {
              bg: theme.bg_primary,       // --bg-surface (#16161c)
              border: theme.border,        // --border (#2a2a36)
              text: theme.fg_faint,        // --text-faint (#8c8ca0)
              accent: theme.accent,        // --accent (#6d9be0)
          }
      }
  }
  ```
- [x] Define `StatusBarWidget`:
  ```rust
  pub struct StatusBarWidget {
      id: WidgetId,
      data: StatusBarData,
      colors: StatusBarColors,
      window_width: f32,
  }
  ```
- [x] Constructor: `StatusBarWidget::new(window_width: f32, theme: &UiTheme) -> Self`
- [x] Setters: `set_data(&mut self, data: StatusBarData)`, `set_window_width(&mut self, width: f32)`, `apply_theme(&mut self, theme: &UiTheme)`
- [x] Implement `Default` for `StatusBarData` (all empty strings) so the widget can be constructed before terminal data is available. The `set_data()` method updates it once real data arrives.
- [x] Export `STATUS_BAR_HEIGHT` as `pub const` (not just file-scoped `const`) so Section 04.2 can import it in `chrome/mod.rs` for layout calculations. Alternatively, expose it via `StatusBarWidget::height() -> f32` as a method. Prefer the const export for consistency with `TAB_BAR_HEIGHT`.
- [x] **File size guard**: `mod.rs` will contain `StatusBarWidget`, `StatusBarData`, `StatusBarColors`, constants, `Widget` impl, and setters. Estimate ~250 lines. Well under the 500-line limit. If it grows, split colors into `colors.rs` submodule (same pattern as tab_bar).

**Validation:** Module compiles. `StatusBarWidget` can be constructed.

---

## 02.2 Layout & Paint

**File(s):** `oriterm_ui/src/widgets/status_bar/mod.rs`

Implement the Widget trait with proper layout and paint.

**Layout constants (from mockup CSS):**

```css
.status-bar {
    height: 22px;
    padding: 0 10px;
    font-size: 11px;
    gap: 16px;
}
```

- [x] Add constants:
  ```rust
  const STATUS_BAR_HEIGHT: f32 = 22.0;
  const STATUS_BAR_PADDING_X: f32 = 10.0;
  const STATUS_BAR_BORDER_TOP: f32 = 2.0;
  const STATUS_BAR_GAP: f32 = 16.0;
  const STATUS_BAR_FONT_SIZE: f32 = 11.0;
  ```
- [x] Implement `Widget` trait (follow `LabelWidget` pattern from `label/mod.rs`):
  - `id()`: return `self.id`
  - `is_focusable()`: `false` (override default which checks `sense().has_focus()` — Sense::none() already returns false, but explicit override is clearer)
  - `sense()`: `Sense::none()` (display-only, no interaction)
  - `layout()`: `fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox { LayoutBox::leaf(self.window_width, STATUS_BAR_HEIGHT).with_widget_id(self.id) }` — note `with_widget_id` is needed for the framework to track this widget's bounds
  - `paint()`:
    1. Draw background: `Rect::new(ctx.bounds.x(), ctx.bounds.y(), ctx.bounds.width(), STATUS_BAR_HEIGHT)` with `self.colors.bg` color. Use `ctx.bounds` not hardcoded (0,y) — the framework positions the widget via layout.
    2. Draw top border: `Rect::new(ctx.bounds.x(), ctx.bounds.y(), ctx.bounds.width(), STATUS_BAR_BORDER_TOP)` with `self.colors.border` color
    3. Draw left-aligned items starting at `x = ctx.bounds.x() + STATUS_BAR_PADDING_X`, `y_text = ctx.bounds.y() + (STATUS_BAR_HEIGHT - text_height) / 2.0`:
       - Shell name: accent color, 11px font
       - Pane count: text color, 11px font (x += previous_item.width + STATUS_BAR_GAP)
       - Grid size: text color, 11px font (x += previous_item.width + STATUS_BAR_GAP)
    4. Draw right-aligned items working inward from `x = ctx.bounds.x() + ctx.bounds.width() - STATUS_BAR_PADDING_X`:
       - Term type (rightmost): accent color, 11px font — position at `x - shaped.width`, then `x -= shaped.width + STATUS_BAR_GAP`
       - Encoding: text color, 11px font — position at `x - shaped.width`
    5. All text vertically centered: `y = ctx.bounds.y() + (STATUS_BAR_HEIGHT - shaped.height) / 2.0`
- [x] Handle empty data gracefully (empty strings skip rendering for that item, don't add gap)
- [x] The mockup shows a `<span class="status-separator"></span>` between the left group (shell, panes, grid) and right group (encoding, term). This is a visual spacer, not a drawn element — the gap between the two groups is implicit because the left group is left-aligned and the right group is right-aligned. No explicit separator drawing needed, but verify the gap is visually correct in the golden test.
- [x] Use `TextStyle::new(STATUS_BAR_FONT_SIZE, color)` for all text. Shape via `ctx.measurer.shape(text, &style, f32::INFINITY)` which returns a `ShapedText` with `.width` and `.height` fields. Draw via `ctx.scene.push_text(Point::new(x, y), shaped, color)`. This matches the `LabelWidget::paint()` pattern exactly (label/mod.rs lines 122-131).
- [x] The `\u{00d7}` (multiplication sign, times) character in grid size renders correctly via the font pipeline — IBM Plex Mono (the embedded UI font) includes this character. Verify by checking the `status_bar_full_data_96dpi` golden test.

**Validation:** StatusBarWidget paints all items at correct positions with correct colors.

---

## 02.3 Theme Integration

**File(s):** `oriterm_ui/src/widgets/status_bar/mod.rs`

Ensure the status bar properly integrates with the theme system.

- [x] `StatusBarColors::from_theme()` uses only `UiTheme` fields (no hardcoded colors)
- [x] All public types (`StatusBarWidget`, `StatusBarData`, `StatusBarColors`) are `pub` in `mod.rs` — the `pub mod status_bar;` in `widgets/mod.rs` (from 02.1) makes them accessible as `oriterm_ui::widgets::status_bar::StatusBarWidget`. No additional re-export needed (consistent with how `LabelWidget`, `TabBarWidget` etc. are exported).
- [x] The widget does NOT need controllers (no hover, click, focus) — it's pure display. Return empty from `controllers()` (the default impl returns empty, so no override needed).

**Validation:** `cargo test -p oriterm_ui` compiles. StatusBarWidget is accessible from `oriterm_ui::widgets::status_bar::StatusBarWidget`.

---

## 02.4 WidgetTestHarness Tests

**File(s):** `oriterm_ui/src/widgets/status_bar/tests.rs` (NEW)

Write headless widget tests using the WidgetTestHarness.

- [x] Create `oriterm_ui/src/widgets/status_bar/tests.rs`
- [x] Add `#[cfg(test)] mod tests;` at the bottom of `mod.rs`
- [x] **Test: `status_bar_layout_fixed_height`** — Verify layout returns 22px height. Note: `WidgetTestHarness::new()` accepts `impl Widget + 'static`. The harness does layout automatically. Use `render()` to get a `Scene`, and verify widget bounds via the scene or by querying the widget directly.
  ```rust
  let widget = StatusBarWidget::new(800.0, &UiTheme::dark());
  let mut h = WidgetTestHarness::new(widget);
  let scene = h.render();
  // StatusBarWidget.layout() returns LayoutBox::leaf(window_width, 22.0)
  // The harness should position the widget — verify via scene non-emptiness.
  assert!(!scene.is_empty());
  ```
  Alternatively, test the Widget::layout directly:
  ```rust
  let widget = StatusBarWidget::new(800.0, &UiTheme::dark());
  let layout = Widget::layout(&widget, &LayoutCtx { measurer: &MockMeasurer::new(), theme: &UiTheme::dark() });
  assert_eq!(layout.preferred_height(), STATUS_BAR_HEIGHT);
  ```
- [x] **Test: `status_bar_not_focusable`** — `assert!(!widget.is_focusable())`
- [x] **Test: `status_bar_sense_none`** — `assert_eq!(widget.sense(), Sense::none())`
- [x] **Test: `status_bar_data_update`** — Set data, verify widget accepts it (no panic, `widget.set_data(data)` succeeds)
- [x] **Test: `status_bar_theme_colors`** — Verify `StatusBarColors::from_theme(&UiTheme::dark())` produces expected colors: `bg == UiTheme::dark().bg_primary`, `border == UiTheme::dark().border`, `text == UiTheme::dark().fg_faint`, `accent == UiTheme::dark().accent`. Also test `from_theme(&UiTheme::light())` for coverage.
- [x] **Test: `status_bar_renders_scene`** — Call `render()`, verify scene is non-empty. Must set data first (otherwise all items are empty strings):
  ```rust
  let mut widget = StatusBarWidget::new(800.0, &UiTheme::dark());
  widget.set_data(StatusBarData { shell_name: "zsh".into(), pane_count: "1 pane".into(), grid_size: "80\u{00d7}24".into(), encoding: "UTF-8".into(), term_type: "xterm-256color".into() });
  let mut h = WidgetTestHarness::new(widget);
  let scene = h.render();
  assert!(!scene.is_empty(), "status bar should produce draw commands");
  ```

- [x] **Test: `status_bar_empty_data_no_crash`** — Construct with default (empty) `StatusBarData`, call `render()`. Must not panic, scene should have at least the background and border quads.
- [x] **Test: `status_bar_window_width_update`** — Call `set_window_width(1200.0)`, then `render()`. Verify the widget does not panic and accepts the new width.
- [x] **Test: `status_bar_default_data_is_all_empty`** — Verify `StatusBarData::default()` has all fields as empty strings.

**Validation:** All 9+ tests pass with `cargo test -p oriterm_ui`.

---

## 02.5 Golden Tests

**File(s):** `oriterm/src/gpu/visual_regression/status_bar.rs` (NEW), `oriterm/src/gpu/visual_regression/mod.rs`

Write golden tests rendering the status bar through the real GPU pipeline.

- [x] Create `oriterm/src/gpu/visual_regression/status_bar.rs` module with `#![cfg(all(test, feature = "gpu-tests"))]` at the top (same gate as all other golden tests)
- [x] Add `mod status_bar;` to `visual_regression/mod.rs`
- [x] **Test: `status_bar_full_data_96dpi`** — Status bar with all items populated:
  - Shell: "zsh", Panes: "3 panes", Grid: "120\u{00d7}30", Encoding: "UTF-8", Term: "xterm-256color"
  - Render at 800x22px, 96 DPI
  - Verifies: background color, top border, accent items, text items, gap spacing
- [x] **Test: `status_bar_single_pane_96dpi`** — Status bar with "1 pane" (singular)
  - Edge case: different text length, verify layout still works
- [x] **Test: `status_bar_empty_items_96dpi`** — Status bar with some empty fields
  - Verifies graceful handling of missing data (no spurious gaps or artifacts)
- [x] Reuse `headless_dialog_env()` and `render_dialog_to_pixels()` from `super::dialog_helpers`. These provide a UI-only renderer (IBM Plex Mono, no terminal font) at 96 DPI. Build the render helper:
  ```rust
  use super::dialog_helpers::{headless_dialog_env, render_dialog_to_pixels};

  const WIDTH: u32 = 800;
  const HEIGHT: u32 = 22;

  fn render_status_bar(
      gpu: &GpuState,
      pipelines: &GpuPipelines,
      renderer: &mut WindowRenderer,
      data: StatusBarData,
  ) -> Vec<u8> {
      let theme = UiTheme::dark();
      let mut widget = StatusBarWidget::new(WIDTH as f32, &theme);
      widget.set_data(data);

      let text_cache = TextShapeCache::new();
      let measurer = CachedTextMeasurer::new(renderer.ui_measurer(1.0), &text_cache, 1.0);
      let icons = renderer.resolved_icons();

      let mut scene = Scene::new();
      let bounds = Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
      let mut ctx = DrawCtx {
          scene: &mut scene,
          theme: &theme,
          measurer: &measurer,
          icons: Some(icons),
          bounds,
          now: Instant::now(),
          interaction: None,
          widget_id: None,
          frame_requests: None,
      };
      widget.paint(&mut ctx);
      render_dialog_to_pixels(gpu, pipelines, renderer, &scene, WIDTH, HEIGHT, 1.0)
  }
  ```
  This follows the same pattern as `tab_bar_icons.rs::render_tab_bar()` but uses the dialog pipeline (UI-only, no terminal font needed).
- [x] Run `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests -- status_bar` to generate references
- [x] Visually inspect generated PNGs against mockup
- [ ] Commit reference PNGs
- [ ] `/tpr-review` checkpoint

**Validation:** All `status_bar_*` golden tests pass. Reference PNGs visually match the mockup's status bar.

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 02.N Completion Checklist

- [x] `oriterm_ui/src/widgets/status_bar/mod.rs` exists with `StatusBarWidget`, `StatusBarData`, `StatusBarColors`
- [x] Widget trait fully implemented: `id`, `is_focusable` (false), `sense` (none), `layout` (22px), `paint` (all items)
- [x] `StatusBarColors::from_theme()` uses only `UiTheme` fields
- [x] All items render at correct positions: left-aligned (shell, panes, grid), right-aligned (encoding, term)
- [x] Accent items (shell, term) use `theme.accent` color
- [x] Normal items (panes, grid, encoding) use `theme.fg_faint` color
- [x] Top border: 2px, `theme.border` color
- [x] Font size: 11px for all items
- [x] 9+ WidgetTestHarness tests pass (11 tests)
- [ ] 3 golden tests pass with committed reference PNGs
- [x] `cargo test -p oriterm_ui` green
- [x] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** `StatusBarWidget` renders identically to the `.status-bar` section of `mockups/main-window-brutal.html` at 96 DPI. Widget follows all `oriterm_ui` conventions. Golden tests pass.
