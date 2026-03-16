---
section: "07"
title: "Layout Extensions & Theme"
status: complete
goal: "Grid layout, RichText, and extended theme tokens support the settings mockup's visual requirements"
inspired_by:
  - "CSS Grid auto-fill (CSS Grid Layout)"
depends_on: []
reviewed: true
sections:
  - id: "07.1"
    title: "Grid Layout"
    status: complete
  - id: "07.2"
    title: "RichText"
    status: complete
  - id: "07.3"
    title: "Theme Extension"
    status: complete
  - id: "07.4"
    title: "Completion Checklist"
    status: complete
---

# Section 07: Layout Extensions & Theme

**Status:** Not Started
**Goal:** The layout engine supports grid layout for color swatch grids and scheme card grids.
RichText enables multi-color text spans for the font preview. Theme tokens cover all the
granular colors needed by the mockup (input backgrounds, card surfaces, faint text, accent
tints).

**Context:** The settings mockup requires two layout capabilities the current flex engine
doesn't have: (1) a grid layout for the color scheme cards and ANSI color swatches
(`grid-template-columns: repeat(auto-fill, minmax(240px, 1fr))`), and (2) multi-color text
on a single line for the font preview panel. The theme also needs 9 additional color tokens
beyond what `UiTheme` currently provides.

**Depends on:** Nothing (independent of event/animation work).

**Recommended implementation order:** 07.3 Theme first (smallest, no dependencies, purely
additive struct fields), then 07.1 Grid Layout (layout engine is a library crate — must
come before ContainerWidget integration in the binary crate), then 07.2 RichLabel (new
widget, depends on nothing but benefits from having the layout engine complete). All three
subsections are in `oriterm_ui` (a library crate), so there is no library-before-binary
ordering concern between them, but Theme is the least risky starting point.

---

## 07.1 Grid Layout

**File(s):** `oriterm_ui/src/layout/grid_solver.rs` (new file for solving logic),
`oriterm_ui/src/layout/layout_box.rs` (new `BoxContent::Grid` variant)

**File size budget:** `layout_box.rs` is 261 lines. Adding `GridColumns` enum (~10 lines),
`BoxContent::Grid` variant (~5 lines), `grid()` constructor (~25 lines), and two builder
methods (~20 lines) totals ~321 lines — safely under 500. `solver.rs` is 443 lines;
adding one 3-line match arm + one import = ~447 lines — safe. `theme/mod.rs` is 107 lines;
adding 9 fields + values in both constructors = ~143 lines — safe.

Add `BoxContent::Grid` variant to the layout engine.

- [x] Define `GridColumns` enum in `oriterm_ui/src/layout/layout_box.rs` (colocated with
  `BoxContent` since it's a field of the `Grid` variant):
  ```rust
  /// Column specification for grid layout.
  #[derive(Debug, Clone, Copy, PartialEq)]
  pub enum GridColumns {
      /// Fixed number of columns.
      Fixed(usize),
      /// Fill as many columns as fit, each at least `min_width` wide.
      /// Remaining space distributed equally (CSS `auto-fill` behavior).
      AutoFill { min_width: f32 },
  }
  ```
  **Required derives:** `Debug, Clone, Copy, PartialEq` — needed because `BoxContent` derives
  `Debug, Clone, PartialEq`, and `GridColumns` is a field of the `Grid` variant. `Copy` is
  natural since it's a small enum with no heap data.
- [x] Export `GridColumns` from `oriterm_ui/src/layout/mod.rs`: add to the `pub use layout_box::`
  line so it reads `pub use layout_box::{BoxContent, GridColumns, LayoutBox};`
- [x] Add `BoxContent::Grid` variant to `oriterm_ui/src/layout/layout_box.rs`:
  ```rust
  // Existing variants (for reference — do not duplicate):
  //   Leaf { intrinsic_width: f32, intrinsic_height: f32 },
  //   Flex { direction, align, justify, gap, children },
  // New variant:
  Grid { columns: GridColumns, row_gap: f32, column_gap: f32, children: Vec<LayoutBox> },
  ```
- [x] Add `mod grid_solver;` declaration to `oriterm_ui/src/layout/mod.rs` (private module —
  only `solver.rs` calls into it, not external consumers). Place it after `mod solver;`.
- [x] Make `solve()` in `solver.rs` `pub(super)` (currently private). This is required so
  `grid_solver::solve_grid()` can call it to recursively solve child boxes.
  `solve_flex()` calls `solve()` directly because both are in the same file (`solver.rs`).
  `grid_solver.rs` is a sibling module under `layout/` and cannot see private functions in
  `solver.rs` — hence the visibility change. Add `use super::grid_solver;` import to
  `solver.rs`.
- [x] Implement grid solving in `oriterm_ui/src/layout/grid_solver.rs` (**new file**, not
  inline in `solver.rs` which is already 443 lines).

  Note: `grid_solver.rs` is a plain file module (not a directory module) because it has
  no tests of its own — all grid layout tests go in the parent `layout/tests.rs`. Per
  test-organization.md, only modules with their own `#[cfg(test)] mod tests;` need to be
  directory modules.

  The function signature:
  ```rust
  pub(super) fn solve_grid(
      layout_box: &LayoutBox,
      columns: &GridColumns,
      row_gap: f32,
      column_gap: f32,
      children: &[LayoutBox],
      constraints: LayoutConstraints,
      pos_x: f32,
      pos_y: f32,
  ) -> LayoutNode
  ```
  It calls `super::solver::solve()` (made `pub(super)` above) for each child.

  Algorithm:
  1. Resolve column count:
     - `Fixed(n)`: use n
     - `AutoFill { min_width }`: `max(1, floor(available_width / (min_width + column_gap)))`.
       Edge case: if `available_width <= 0` or `min_width <= 0`, clamp to 1 column.
  2. Compute column width: `(available_width - (n-1) * column_gap) / n`
  3. Lay out children into grid cells: row-major order, wrap at column count
  4. Row height = max height of children in that row
  5. Total height = sum of row heights + (rows-1) * row_gap

  Each child gets a `LayoutConstraints` with `max_width = column_width` and
  `max_height = f32::INFINITY` (height determined by content). After solving all
  children, row heights are computed as `max(child_heights in row)`.
- [x] Add `BoxContent::Grid { .. }` match arm to the private `solve()` function in
  `solver.rs` (called by the public `compute_layout()` entry point). The match currently
  handles `Leaf` and `Flex`. The new arm delegates to `grid_solver::solve_grid()`.
- [x] Add a `LayoutBox::grid()` convenience constructor in `layout_box.rs`, mirroring
  the existing `LayoutBox::leaf()` and `LayoutBox::flex()` patterns. The constructor must
  initialize all `LayoutBox` fields including `sense`, `hit_test_behavior`, and `clip`
  (added by Section 02):
  ```rust
  pub fn grid(columns: GridColumns, children: Vec<Self>) -> Self {
      Self {
          width: SizeSpec::Hug,
          height: SizeSpec::Hug,
          padding: Insets::default(),
          margin: Insets::default(),
          min_width: 0.0,
          max_width: f32::INFINITY,
          min_height: 0.0,
          max_height: f32::INFINITY,
          content: BoxContent::Grid {
              columns,
              row_gap: 0.0,
              column_gap: 0.0,
              children,
          },
          sense: Sense::all(),
          hit_test_behavior: HitTestBehavior::default(),
          clip: false,
      }
  }
  ```
  Also add builder methods `with_row_gap()` and `with_column_gap()` that operate on
  `BoxContent::Grid` (same pattern as `with_gap()` for `BoxContent::Flex` — silently
  no-ops on non-Grid content, using `if let BoxContent::Grid { .. }` guard).
- [x] **Sync point — `layout/tests.rs` `walk_invariant()` helper** (line 879): The
  `walk_invariant()` function in `oriterm_ui/src/layout/tests.rs` uses
  `if let BoxContent::Flex { children, .. }` on line 879 to recurse into children. This
  must be updated to also handle `BoxContent::Grid { children, .. }` — otherwise the
  content_rect invariant test will silently skip grid children. Replace the `if let` with
  a match:
  ```rust
  let children = match &layout_box.content {
      BoxContent::Flex { children, .. } | BoxContent::Grid { children, .. } => children,
      BoxContent::Leaf { .. } => return,
  };
  ```
- [x] Add grid container support to `ContainerWidget`.

  **WARNING: `container/mod.rs` is already 484 lines.** Before adding any grid code,
  **extract** existing helper methods into a submodule (e.g., `container/layout_build.rs`
  for `build_layout_box()`, `get_or_compute_layout()`, and `child_bounds_from_layout()`).
  This extraction must happen first to stay under the 500-line hard limit.

  `ContainerWidget::build_layout_box()` currently always produces `BoxContent::Flex`.
  To support grid layout, add a `LayoutMode` enum field (Flex vs Grid) that switches
  `build_layout_box()` output. This avoids duplicating the 213 lines of child management
  and event dispatch logic that a separate `GridWidget` would require. `direction`, `align`,
  `justify` become no-ops in grid mode (documented via `debug_assert!` or silent ignore).

  Add a factory method:
  ```rust
  pub fn grid(columns: GridColumns, gap: f32) -> Self { ... }
  ```

  `ContainerWidget` currently stores `direction`, `align`, `justify`, `gap` — fields
  specific to flex layout. Grid layout needs `columns`, `row_gap`, `column_gap` instead.
  The `LayoutMode` enum approach carries dead flex fields when in grid mode (and vice versa),
  but this is acceptable to avoid duplicating the event dispatch submodule.

---

## 07.2 RichText

**File(s):** `oriterm_ui/src/widgets/rich_label/mod.rs` (new widget),
`oriterm_ui/src/widgets/rich_label/tests.rs` (sibling test file)

Multi-span colored text for the font preview.

- [x] **Add module declaration** to `oriterm_ui/src/widgets/mod.rs`: add `pub mod rich_label;`
  in alphabetical order (after `pub mod panel;` on line 20, before `pub mod scroll;` on
  line 21).
- [x] Define `TextSpan` in `rich_label/mod.rs`. `TextStyle` already exists at
  `crate::text::TextStyle` with fields: `font_family`, `size`, `weight`, `color`,
  `align`, `overflow`. Import it — do not redefine:
  ```rust
  use crate::text::TextStyle;

  /// A single styled text run within a `RichLabel`.
  pub struct TextSpan {
      /// The text content of this span.
      pub text: String,
      /// Visual style for this span (color, size, weight).
      pub style: TextStyle,
  }
  ```
- [x] Define `RichLabel` widget implementing `Widget` trait:
  ```rust
  pub struct RichLabel {
      id: WidgetId,
      spans: Vec<TextSpan>,
  }

  impl RichLabel {
      pub fn new(spans: Vec<TextSpan>) -> Self { ... }
  }
  ```
- [x] `Widget::layout()` — produces a `LayoutBox::leaf()`. Measure each span sequentially
  via `ctx.measurer.measure(&span.text, &span.style, f32::INFINITY)`, sum widths for
  intrinsic width. Height = `max(metrics.height)` across all spans (`TextMetrics` has
  `width`/`height`/`line_count` — no separate ascent/descent).
  Returns `LayoutBox::leaf(total_width, max_height)`.
- [x] `Widget::draw()` — shape and draw each span at advancing x-offset. Call
  `ctx.measurer.shape(&span.text, &span.style, ...)` per span, then
  `ctx.draw_list.push_text(pos, shaped, span.style.color)` per span. `push_text()` takes
  `(Point, ShapedText, Color)`, so one call per span is required to use each span's color.
- [x] `Widget::sense()` returns `Sense::none()` (non-interactive — display only).
- [x] `Widget::is_focusable()` returns `false`.
- [x] `Widget::handle_mouse/hover/key` — return `WidgetResponse::ignored()` (no interaction).
- [x] Add `#[cfg(test)] mod tests;` at the bottom of `rich_label/mod.rs` and create
  `rich_label/tests.rs` following the sibling `tests.rs` pattern (no `mod tests { }` wrapper,
  `super::` imports).

---

## 07.3 Theme Extension

**File(s):** `oriterm_ui/src/theme/mod.rs`

Add tokens needed by the settings mockup.

**Existing UiTheme fields** (for reference — do not duplicate):
`bg_primary`, `bg_secondary`, `bg_hover`, `bg_active`, `fg_primary`, `fg_secondary`,
`fg_disabled`, `accent`, `border`, `shadow`, `close_hover_bg`, `close_pressed_bg`,
`corner_radius`, `spacing`, `font_size`, `font_size_small`, `font_size_large`.

- [x] Add 9 new fields to `UiTheme`:
  ```rust
  pub struct UiTheme {
      // ... existing 17 fields ...

      /// Input field background (darker than surface).
      pub bg_input: Color,
      /// Card/raised surface background.
      pub bg_card: Color,
      /// Card hover background.
      pub bg_card_hover: Color,
      /// Very muted text (descriptions, version labels, section rules).
      pub fg_faint: Color,
      /// Accent tint for active/selected backgrounds (low opacity).
      pub accent_bg: Color,
      /// Stronger accent tint (selected card, active nav item).
      pub accent_bg_strong: Color,
      /// Accent hover color (lighter accent for hover states).
      pub accent_hover: Color,
      /// Danger color (for destructive actions).
      pub danger: Color,
      /// Success color.
      pub success: Color,
  }
  ```
- [x] Update `UiTheme::dark()` with values from the mockup CSS variables:
  ```rust
  bg_input: Color::hex(0x12121a),
  bg_card: Color::hex(0x1c1c24),
  bg_card_hover: Color::hex(0x24242e),
  fg_faint: Color::hex(0x4e4e5e),
  accent_bg: Color::rgba(0.42, 0.55, 1.0, 0.08),
  accent_bg_strong: Color::rgba(0.42, 0.55, 1.0, 0.14),
  accent_hover: Color::hex(0x8aa4ff),
  danger: Color::hex(0xff6b6b),
  success: Color::hex(0x6bffb8),
  ```
- [x] Update `UiTheme::light()` with light-mode counterparts. Both `dark()` and `light()`
  are `const fn` — all values must be const-evaluable (`Color::hex()`, `Color::rgba()`,
  `Color::from_rgb_u8()` are all `const fn`). Suggested light values:
  ```rust
  bg_input: Color::from_rgb_u8(0xE8, 0xE8, 0xF0),
  bg_card: Color::from_rgb_u8(0xFF, 0xFF, 0xFF),
  bg_card_hover: Color::from_rgb_u8(0xF0, 0xF0, 0xF5),
  fg_faint: Color::from_rgb_u8(0x99, 0x99, 0xAA),
  accent_bg: Color::rgba(0.0, 0.47, 0.83, 0.08),
  accent_bg_strong: Color::rgba(0.0, 0.47, 0.83, 0.14),
  accent_hover: Color::hex(0x005A9E),
  danger: Color::hex(0xD32F2F),
  success: Color::hex(0x2E7D32),
  ```
- [x] **Sync point — `light_differs_from_dark_on_all_colors` test.** The test in
  `theme/tests.rs` (line 66) checks that dark and light differ on all color fields.
  **Add 9 new `assert_ne!` checks** to that test (one per new color field: `bg_input`,
  `bg_card`, `bg_card_hover`, `fg_faint`, `accent_bg`, `accent_bg_strong`, `accent_hover`,
  `danger`, `success`).
- [x] **Sync point — `const` theme sites** (no action needed, awareness only).
  `widgets/tests.rs:7` and `overlay/tests.rs:18` use `const TEST_THEME: UiTheme =
  UiTheme::dark()`. Since `dark()` remains `const fn` and all new field values are
  const-evaluable, these sites compile without changes.
- [x] Update widget `*Style::from_theme()` methods to use new tokens where Section 09/10
  consumers need them. Priority targets (these will be used by new widgets in Sections 09-10):
  - `TextInputStyle::from_theme()` — use `bg_input` for input background
  - `PanelStyle::from_theme()` — use `bg_card` for card surfaces
  - `DialogStyle::from_theme()` — use `bg_card` for dialog background
  - `MenuStyle::from_theme()` — use `bg_card` for menu background

  Lower priority (existing widgets work fine with current tokens; update if time permits):
  - `ButtonStyle::from_theme()` — `accent_hover` for hover state
  - `SliderStyle::from_theme()` — `accent_hover` for handle hover
  - `StatusBadgeStyle::from_theme()` — `danger`, `success` for badge variants
  - `LabelStyle::from_theme()` — `fg_faint` for subtitle/description labels
  - `SeparatorStyle::from_theme()` — `fg_faint` for rule color

---

## 07.4 Completion Checklist

### Grid Layout
- [x] `GridColumns` enum defined in `layout_box.rs` with `Debug, Clone, Copy, PartialEq` derives
- [x] `GridColumns` re-exported from `layout/mod.rs`
- [x] `BoxContent::Grid` variant added to `layout_box.rs`
- [x] `LayoutBox::grid()` convenience constructor added (includes `sense`, `hit_test_behavior`, `clip` fields)
- [x] `LayoutBox::with_row_gap()` and `with_column_gap()` builder methods added
- [x] `mod grid_solver;` declared in `layout/mod.rs`
- [x] `grid_solver.rs` created with `solve_grid()` function
- [x] `solver.rs`: `solve()` made `pub(super)` so `grid_solver` can call it recursively
- [x] `solver.rs`: `BoxContent::Grid` match arm delegates to `grid_solver::solve_grid()`
- [x] `solver.rs`: `use super::grid_solver;` import added
- [x] Grid solver handles edge cases: 0 children, `min_width > available_width`, `available_width <= 0`
- [x] `container/mod.rs` extracted below 500 lines (moved layout/cache helpers to `container/layout_build.rs`)
- [x] `ContainerWidget` supports grid mode via `LayoutMode` enum field and `grid()` factory method

### RichLabel
- [x] `pub mod rich_label;` added to `widgets/mod.rs`
- [x] `TextSpan` struct defined with `text: String` and `style: TextStyle` (imported from `crate::text`)
- [x] `RichLabel` widget implements `Widget` trait (layout, draw, sense, is_focusable, handle_mouse/hover/key)
- [x] `RichLabel::sense()` returns `Sense::none()`
- [x] `RichLabel::draw()` calls `push_text()` once per span at advancing x-offset
- [x] `#[cfg(test)] mod tests;` at bottom of `rich_label/mod.rs`

### Theme
- [x] 9 new fields added to `UiTheme` struct (`bg_input`, `bg_card`, `bg_card_hover`, `fg_faint`, `accent_bg`, `accent_bg_strong`, `accent_hover`, `danger`, `success`)
- [x] `UiTheme::dark()` populated with mockup-matching hex/rgba values
- [x] `UiTheme::light()` populated with light-mode counterparts (all `const` evaluable)
- [x] `theme/tests.rs`: `light_differs_from_dark_on_all_colors` updated with 9 new `assert_ne!` lines
- [x] Priority `*Style::from_theme()` methods updated: `TextInputStyle`, `PanelStyle`, `DialogStyle`, `MenuStyle`

### Tests (sibling `tests.rs` pattern per project rules)
- [x] Grid layout tests in `oriterm_ui/src/layout/tests.rs` (expand existing):
  - 6 items in AutoFill(240px) at 600px = 2 columns, 3 rows
  - Grid with 0 children produces height = 0
  - Grid with 1 child fills full width in single column
  - AutoFill with min_width > available_width produces 1 column
  - `Fixed(3)` with 9 items = 3 columns, 3 rows
  - Grid with gaps (row_gap + column_gap)
  - Grid content_rect invariant with padding
- [x] `layout/tests.rs` `walk_invariant()` updated to handle `BoxContent::Grid` children
- [x] RichLabel tests in `oriterm_ui/src/widgets/rich_label/tests.rs` (new sibling file):
  - Two spans with different colors produce correct layout width (40 + 48 = 88)
  - Empty spans vec produces zero-size layout
  - Single span matches equivalent `Label` layout dimensions
  - Sense is `none()`, not focusable, has widget ID
- [x] Theme tests in `oriterm_ui/src/theme/tests.rs` (expand existing):
  - All 9 new dark values are non-default (not `Color::TRANSPARENT` or zero)
  - All 9 new light values are non-default
  - Dark and light differ on all 9 new color fields
  - `UiTheme::dark()` still equals `UiTheme::default()` (regression guard — pre-existing `default_is_dark` test)
- [x] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** Grid layout with 6 items at AutoFill(240px) in a 600px container produces
2 columns and 3 rows. RichLabel with 3 colored spans renders each span at the correct
x-offset with the correct color. All 9 new theme tokens have distinct non-default values in
both dark and light themes, and dark differs from light on every new color field.
