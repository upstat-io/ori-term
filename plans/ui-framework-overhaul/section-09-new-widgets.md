---
section: "09"
title: "New Widget Library"
status: complete
goal: "All new widgets needed by the settings mockup, built on the new framework"
inspired_by:
  - "mockups/settings-brutal.html — the design spec"
depends_on: ["08"]
reviewed: true
sections:
  - id: "09.1"
    title: "SidebarNav"
    status: complete
  - id: "09.2"
    title: "PageContainer"
    status: complete
  - id: "09.3"
    title: "SettingRow"
    status: complete
  - id: "09.4"
    title: "SchemeCard"
    status: complete
  - id: "09.5"
    title: "ColorSwatchGrid & SpecialColorSwatch"
    status: complete
  - id: "09.6"
    title: "CodePreview"
    status: complete
  - id: "09.7"
    title: "CursorPicker"
    status: complete
  - id: "09.8"
    title: "KeybindRow & KbdBadge"
    status: complete
  - id: "09.9"
    title: "NumberInput & RangeSlider"
    status: complete
  - id: "09.10"
    title: "Completion Checklist"
    status: complete
---

# Section 09: New Widget Library

**Status:** Not Started
**Goal:** Every widget needed by `mockups/settings-brutal.html` exists, renders correctly, and uses
the new framework (controllers, visual states, animation behaviors).

**Context:** The settings mockup (`mockups/settings-brutal.html`) requires 12 new widgets
(plus RichLabel from Section 07). Each is built on the new framework established in
Sections 01-08: interaction state for hover, controllers for click/drag, visual state
animators for transitions, and the new theme tokens for colors.

**File size warning**: The following widgets are likely to approach 300+ lines and should be
monitored during implementation. If any exceeds 400 lines, split into submodules BEFORE
continuing (e.g., `scheme_card/mod.rs` + `scheme_card/preview.rs`):
- **SchemeCard** — terminal preview rendering + swatch bar + selection state + multiple visual elements
- **SidebarNav** — section titles + nav items + icons + active indicator + hover per item
- **ColorSwatchGrid** — per-cell hover + click + grid layout + label rendering

**Depends on:** Section 08 (Widget Trait — all new widgets use the new trait).

**Prerequisite -- New IconId variants**: The SidebarNav widget references icon IDs
(`IconId::Sun`, `IconId::Palette`, etc.) that do not exist in the current `IconId` enum
(`oriterm_ui/src/icons/mod.rs`). The following variants must be added:
- `Sun` — Appearance page icon
- `Palette` — Colors page icon
- `Type` — Font page icon
- `Terminal` — Terminal page icon
- `Keyboard` — Keybindings page icon
- `Window` — Window page icon
- `Bell` — Bell page icon
- `Activity` — Rendering page icon

Each new variant requires:
1. Add variant to `IconId` enum (icons/mod.rs line ~49)
2. Add match arm in `IconId::path()` (icons/mod.rs line ~66)
3. Define static `IconPath` constant with SVG path commands
4. Add to `ALL_ICONS` array in tests (icons/tests.rs line ~109)

This is a sync point: all 4 locations must be updated together.

---

## 09.1 SidebarNav

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs`

The left navigation panel from the mockup: section titles, nav items with icons, active indicator.

- [x] Define `NavItem`:
  ```rust
  pub struct NavItem {
      pub label: String,
      pub icon: Option<IconId>,
      pub page_index: usize,
  }
  ```
- [x] Define `SidebarNavWidget`:
  - Fixed width (200px logical)
  - Background: `theme.bg_secondary` (darker than primary surface)
  - Section titles: uppercase, small font, `fg_faint` color
  - Nav items: per-item VisualStateAnimator for hover transitions
  - Active item: `accent_bg_strong` background, `accent` text color
  - Hover: `bg_hover` background via VisualStateAnimator
  - Emits: `WidgetAction::Selected { id, index: page_index }` via `on_input`
  - Version label at bottom: `fg_faint`, small font
- [x] Visual states: `CommonStates { Normal, Hovered, Active }` per nav item
  (Active here means "selected page", not mouse-down)
- [x] `for_each_child_mut()`: leaf widget — no child widgets, default no-op is correct
- [x] `layout()`: fixed-width column with vertical stack of items
- [x] Tests in `sidebar_nav/tests.rs`: construction, layout width, nav item count,
  hit testing, page index resolution, active index tracking

---

## 09.2 PageContainer

**File(s):** `oriterm_ui/src/widgets/page_container/mod.rs`

Shows one child at a time, switches on command.

- [x] Define `PageContainerWidget`:
  ```rust
  pub struct PageContainerWidget {
      id: WidgetId,
      pages: Vec<Box<dyn Widget>>,
      active_page: usize,
  }
  ```
- [x] `layout()`: only lay out the active page. Other pages get zero-size layout.
- [x] `paint()`: only paint the active page.
- [x] `set_active_page(index)`: switch pages, `request_paint()`
- [x] `accept_action()`: handle `Selected` from SidebarNav to switch pages
- [x] `for_each_child_mut()`: iterate over all pages (needed for widget registration)
- [x] `sense()`: `Sense::none()` (delegates to active page's children)
- [x] Tests in `page_container/tests.rs`: page switching, layout only measures active page,
  `accept_action` routes `Selected`, child count

---

## 09.3 SettingRow (Enhanced FormRow)

**File(s):** `oriterm_ui/src/widgets/setting_row/mod.rs`

Two-line label with hover background highlight.

- [x] Define `SettingRowWidget`:
  - Left side: name (13px, `fg_primary`) + description (11.5px, `fg_secondary`)
  - Right side: control widget (dropdown, toggle, slider, etc.)
  - Full-width hover background: rounded rect, `bg_card` on hover (100ms fade)
  - Minimum height: 44px
  - HoverController for hover state
  - Visual states: `CommonStates { Normal, Hovered }`
  - `sense()`: `Sense::hover()` (row itself is hoverable, control handles clicks)
  - `for_each_child_mut()`: yields the right-side control widget
  - Tests in `setting_row/tests.rs`: layout height >= 44px, two-line label rendering,
    hover state, child control delegation

---

## 09.4 SchemeCard

**File(s):** `oriterm_ui/src/widgets/scheme_card/mod.rs`

Color scheme preview card with terminal preview and swatch bar.

- [x] Define `SchemeCardWidget`:
  - Rounded container (8px corners)
  - Title bar: scheme name + optional "Active" badge
  - Mini terminal preview: monospace text on scheme's background color
    (use `RichLabel` with scheme's ANSI colors for syntax-highlighted text)
  - Swatch bar: 8 colored rectangles in a row (ANSI colors 0-7)
  - States: Normal (transparent border), Hovered (subtle border), Selected (accent border)
  - Selected state: `accent` border + `accent_bg` background tint
  - ClickController to select
  - `sense()`: `Sense::click()`
  - Emits: `WidgetAction::Selected { id, index: scheme_index }`
- [x] Terminal preview rendering:
  - Fixed-height area (56px) with scheme background
  - Render 2 lines of monospace text using scheme's foreground/ANSI colors
  - Text content: `$ cargo build --release` + `Compiling ori_term v0.1.0`
  - Uses current terminal font at 11px size
- [x] Data type: `SchemeCardData { name: String, bg: Color, fg: Color, ansi: [Color; 8], selected: bool }`
  passed via constructor — widget does not own scheme definitions
- [x] Tests in `scheme_card/tests.rs`: layout dimensions, swatch bar renders 8 colors,
  selection state, click emits `Selected`

---

## 09.5 ColorSwatchGrid & SpecialColorSwatch

**File(s):** `oriterm_ui/src/widgets/color_swatch/mod.rs`

Clickable color grids for palette editing.

- [x] Define `ColorSwatchGrid`:
  - 8 columns, dynamic rows
  - Each cell: colored rounded square (6px corners)
  - Cell label below: index number (9.5px, `fg_faint`)
  - Hover: enlarge cell slightly (redraw at 115% size, not transform)
  - Click: emit `Selected { id, index }` for future color picker
  - HoverController + ClickController per cell
  - Use grid layout from Section 07 (`GridColumns::Fixed(8)`)
  - Tests in `color_swatch/tests.rs`: grid layout produces 8 columns,
    click emits `Selected` with correct index, hover enlargement

- [x] Define `SpecialColorSwatch`:
  - Large swatch (28x28px, 6px corners) + label + hex value
  - Label: 11px `fg_primary`
  - Hex value: 10px monospace `fg_faint`
  - Container with hover background
  - 4-column grid layout for the foreground/background/cursor/selection row

---

## 09.6 CodePreview

**File(s):** `oriterm_ui/src/widgets/code_preview/mod.rs`

Syntax-highlighted font preview panel.

- [x] Define `CodePreviewWidget`:
  - Background: `bg_card` with rounded corners (8px)
  - Label: "Preview" in uppercase small text
  - Content: multi-line `RichLabel` with syntax-highlighted Rust code
  - Uses the terminal's configured font family and size
  - Colors from hardcoded syntax theme (keyword=purple, function=blue,
    string=green, comment=gray, number=orange)
  - `sense()`: `Sense::none()` (display only)
  - Tests in `code_preview/tests.rs`: layout dimensions, produces RichLabel spans,
    sense is none

---

## 09.7 CursorPicker

**File(s):** `oriterm_ui/src/widgets/cursor_picker/mod.rs`

Visual cursor style selector with 3 options.

- [x] Define `CursorPickerWidget`:
  - 3 card options side by side (Block, Bar, Underline)
  - Each card: rounded container (8px) with cursor demo + label
  - Cursor demos:
    - Block: character with accent background
    - Bar: character with 2px accent bar on left
    - Underline: character with 2px accent line on bottom
  - Active card: accent border + `accent_bg` background
  - Hover: subtle border
  - ClickController per card
  - Emits: `WidgetAction::Selected { id, index }` (0=Block, 1=Bar, 2=Underline)
  - `sense()`: `Sense::click()`
  - Tests in `cursor_picker/tests.rs`: 3 cards rendered, selection state,
    click emits `Selected` with correct index

---

## 09.8 KeybindRow & KbdBadge

**File(s):** `oriterm_ui/src/widgets/keybind/mod.rs`

Keybinding display with styled key badges.

- [x] Define `KbdBadge`:
  - Small rounded rect (4px corners) with bottom border thicker (2px for keycap depth)
  - Background: `bg_input`, border: `border` color
  - Text: 11px `fg_primary`
  - `sense()`: `Sense::none()` (display only)

- [x] Define `KeybindRow`:
  - Left: action name label (13px `fg_primary`)
  - Right: row of KbdBadge widgets separated by "+" labels
  - Hover: `bg_card` background
  - `sense()`: `Sense::hover()`
  - Future: click to rebind (not in initial implementation)
  - Tests in `keybind/tests.rs`: badge renders key text, row layout with badges,
    hover state

---

## 09.9 NumberInput & RangeSlider

**File(s):** `oriterm_ui/src/widgets/number_input/mod.rs`,
  `oriterm_ui/src/widgets/range_slider/mod.rs`

**WidgetAction coverage**: Existing `WidgetAction` variants in `oriterm_ui/src/action.rs`
suffice for all new widgets. The new widgets primarily use `Selected` (SidebarNav,
SchemeCard, CursorPicker, ColorSwatch) and `ValueChanged` (NumberInput, RangeSlider).
`DoubleClicked`/`TripleClicked`/`DragStart`/`DragUpdate`/`DragEnd`/`ScrollBy` were
added in Section 04. No new variants needed.

- [x] **NumberInput**:
  - Numeric text input with min/max/step constraints
  - Centered text, compact width (80px)
  - Arrow key increment/decrement
  - Background: `bg_input`, border: `border`
  - Focus: accent border
  - FocusController + ClickController
  - Emits: `WidgetAction::ValueChanged { id, value }`
  - Tests in `number_input/tests.rs`: min/max clamping, step increment,
    arrow key behavior, `ValueChanged` emission

- [x] **RangeSlider** (enhanced Slider) — existing `SliderWidget` satisfies all requirements:
  - Horizontal track with filled portion (accent color)
  - Rounded thumb (14px circle) with shadow
  - Value label to the right (monospace, `fg_secondary`)
  - DragController for thumb
  - HoverController for thumb highlight
  - `sense()`: `Sense::drag().union(Sense::focusable())`
  - Emits: `WidgetAction::ValueChanged { id, value }`
  - Note: existing `SliderWidget` (`oriterm_ui/src/widgets/slider/`) can serve as a
    reference or base — evaluate extending it vs. creating new widget
  - Tests in `range_slider/tests.rs`: drag updates value, value clamping,
    filled track proportion, `ValueChanged` emission

---

## 09.10 Completion Checklist

- [x] SidebarNav renders with section titles, nav items, icons, active state
- [x] PageContainer switches pages on nav selection
- [x] SettingRow shows name + description with hover background
- [x] SchemeCard renders terminal preview + swatch bar with selection state
- [x] ColorSwatchGrid renders 8-column grid with hover enlargement
- [x] SpecialColorSwatch renders swatch + label + hex value
- [x] CodePreview renders syntax-highlighted code with terminal font
- [x] CursorPicker shows 3 cursor options with selection state
- [x] KeybindRow shows action name + KbdBadge key labels
- [x] NumberInput accepts numeric input with constraints
- [x] RangeSlider shows filled track + value label (existing `SliderWidget`)
- [x] All widgets use new framework (controllers, visual states, sense)
- [x] All widgets render correctly at 100% and 150% DPI
- [x] 8 new `IconId` variants added with SVG path definitions and test coverage
- [x] Each new widget has a `tests.rs` sibling file (per test organization rules):
  `sidebar_nav/tests.rs`, `page_container/tests.rs`, `setting_row/tests.rs`,
  `scheme_card/tests.rs`, `color_swatch/tests.rs`, `code_preview/tests.rs`,
  `cursor_picker/tests.rs`, `keybind/tests.rs`, `number_input/tests.rs`
  (RangeSlider covered by existing `slider/tests.rs`)
- [x] Each new widget module declared in `oriterm_ui/src/widgets/mod.rs`
- [x] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** Each new widget renders correctly in isolation (verified via test or
manual inspection). All widgets use the new framework — no manual hover tracking, no
legacy event methods.
