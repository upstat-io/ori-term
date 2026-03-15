---
section: "07"
title: "Layout Extensions & Theme"
status: not-started
goal: "Grid layout, RichText, and extended theme tokens support the settings mockup's visual requirements"
inspired_by:
  - "CSS Grid auto-fill (CSS Grid Layout)"
depends_on: []
reviewed: false
sections:
  - id: "07.1"
    title: "Grid Layout"
    status: not-started
  - id: "07.2"
    title: "RichText"
    status: not-started
  - id: "07.3"
    title: "Theme Extension"
    status: not-started
  - id: "07.4"
    title: "Completion Checklist"
    status: not-started
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
on a single line for the font preview panel. The theme also needs ~6 additional color tokens
beyond what `UiTheme` currently provides.

**Depends on:** Nothing (independent of event/animation work).

---

## 07.1 Grid Layout

**File(s):** `oriterm_ui/src/layout/grid_solver.rs` (new file for solving logic),
`oriterm_ui/src/layout/layout_box.rs` (new `BoxContent::Grid` variant)

Add `BoxContent::Grid` variant to the layout engine.

- [ ] Define `GridColumns` enum:
  ```rust
  pub enum GridColumns {
      /// Fixed number of columns.
      Fixed(usize),
      /// Fill as many columns as fit, each at least `min_width` wide.
      /// Remaining space distributed equally (CSS `auto-fill` behavior).
      AutoFill { min_width: f32 },
  }
  ```
- [ ] Add `BoxContent::Grid` variant to `oriterm_ui/src/layout/layout_box.rs`:
  ```rust
  // Existing variants (for reference — do not duplicate):
  //   Leaf { intrinsic_width: f32, intrinsic_height: f32 },
  //   Flex { direction, align, justify, gap, children },
  // New variant:
  Grid { columns: GridColumns, row_gap: f32, column_gap: f32, children: Vec<LayoutBox> },
  ```
- [ ] Implement grid solving in `oriterm_ui/src/layout/grid_solver.rs` (**new file**, not
  inline in `solver.rs` which is already 428 lines):
  1. Resolve column count:
     - `Fixed(n)`: use n
     - `AutoFill { min_width }`: `max(1, floor(available_width / (min_width + column_gap)))`
  2. Compute column width: `(available_width - (n-1) * column_gap) / n`
  3. Lay out children into grid cells: row-major order, wrap at column count
  4. Row height = max height of children in that row
  5. Total height = sum of row heights + (rows-1) * row_gap
- [ ] Update the `solve()` function in `solver.rs` to handle `BoxContent::Grid` in
  its match arm (currently only handles `Leaf` and `Flex`). Add a single `BoxContent::Grid { .. }`
  arm that delegates to `grid_solver::solve_grid()`. The grid solving logic itself lives
  in the new `grid_solver.rs` file to stay within the 500-line file limit.
- [ ] Grid children are solved recursively: each child gets a `LayoutConstraints` with
  `max_width = column_width` and `max_height = f32::INFINITY` (height determined by
  content). After solving all children, row heights are computed as `max(child_heights
  in row)`.
- [ ] Add `ContainerWidget::grid()` factory:
  ```rust
  pub fn grid(columns: GridColumns, gap: f32) -> Self { ... }
  ```
- [ ] Unit tests: 6 items in AutoFill(240px) with 600px available = 2 columns of 3 rows

---

## 07.2 RichText

**File(s):** `oriterm_ui/src/widgets/rich_label/mod.rs` (new widget)

Multi-span colored text for the font preview.

- [ ] Define `TextSpan`:
  ```rust
  pub struct TextSpan {
      pub text: String,
      pub style: TextStyle,
  }
  ```
- [ ] Define `RichLabel` widget:
  ```rust
  pub struct RichLabel {
      id: WidgetId,
      spans: Vec<TextSpan>,
  }

  impl RichLabel {
      pub fn new(spans: Vec<TextSpan>) -> Self { ... }
  }
  ```
- [ ] Layout: shape each span sequentially, sum widths. Height = max baseline + descent.
  For multi-line: wrap at max_width boundary (span-level wrapping, not character-level).
- [ ] Paint: draw each span at advancing x-offset with its own color/weight.
- [ ] `sense()` returns `Sense::none()` (non-interactive)
- [ ] Unit tests: two spans with different colors render at correct x-offsets

---

## 07.3 Theme Extension

**File(s):** `oriterm_ui/src/theme/mod.rs`

Add tokens needed by the settings mockup.

**Existing UiTheme fields** (for reference — do not duplicate):
`bg_primary`, `bg_secondary`, `bg_hover`, `bg_active`, `fg_primary`, `fg_secondary`,
`fg_disabled`, `accent`, `border`, `shadow`, `close_hover_bg`, `close_pressed_bg`,
`corner_radius`, `spacing`, `font_size`, `font_size_small`, `font_size_large`.

- [ ] Add new fields to `UiTheme`:
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
- [ ] Update `UiTheme::dark()` with values from the mockup CSS variables:
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
- [ ] Update `UiTheme::light()` with appropriate light-mode counterparts
- [ ] Update all widget `*Style::from_theme()` methods to use new tokens where appropriate

---

## 07.4 Completion Checklist

- [ ] `BoxContent::Grid` variant with `AutoFill` and `Fixed` column modes
- [ ] Grid solver correctly computes column count, widths, and row heights
- [ ] `ContainerWidget::grid()` factory creates grid containers
- [ ] `RichLabel` renders multi-span colored text sequentially
- [ ] `UiTheme` has all tokens needed by the settings mockup
- [ ] `UiTheme::dark()` populated with mockup-matching colors
- [ ] `UiTheme::light()` has reasonable light-mode counterparts
- [ ] Existing widget styles updated to use new tokens where appropriate
- [ ] Unit tests: grid layout, rich label span measurement, theme token completeness
- [ ] Unit tests: grid with 0 children renders as empty (height = 0)
- [ ] Unit tests: grid with 1 child fills full width in single column
- [ ] Unit tests: AutoFill with min_width > available_width produces 1 column
- [ ] Grid layout test file: `oriterm_ui/src/layout/tests.rs` (expand existing)
- [ ] RichLabel test file: `oriterm_ui/src/widgets/rich_label/tests.rs`
- [ ] Theme test file: `oriterm_ui/src/theme/tests.rs` (expand existing)
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** Grid layout with 6 items at AutoFill(240px) in a 600px container produces
2 columns. RichLabel with 3 colored spans renders correctly. All new theme tokens have
non-default values in both dark and light themes.
