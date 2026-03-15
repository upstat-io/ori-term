---
section: "09"
title: "New Widget Library"
status: not-started
goal: "All new widgets needed by the settings mockup, built on the new framework"
inspired_by:
  - "mockups/settings.html — the design spec"
depends_on: ["08"]
reviewed: false
sections:
  - id: "09.1"
    title: "SidebarNav"
    status: not-started
  - id: "09.2"
    title: "PageContainer"
    status: not-started
  - id: "09.3"
    title: "SettingRow"
    status: not-started
  - id: "09.4"
    title: "SchemeCard"
    status: not-started
  - id: "09.5"
    title: "ColorSwatchGrid & SpecialColorSwatch"
    status: not-started
  - id: "09.6"
    title: "CodePreview"
    status: not-started
  - id: "09.7"
    title: "CursorPicker"
    status: not-started
  - id: "09.8"
    title: "KeybindRow & KbdBadge"
    status: not-started
  - id: "09.9"
    title: "NumberInput & RangeSlider"
    status: not-started
  - id: "09.10"
    title: "Completion Checklist"
    status: not-started
---

# Section 09: New Widget Library

**Status:** Not Started
**Goal:** Every widget needed by `mockups/settings.html` exists, renders correctly, and uses
the new framework (controllers, visual states, animation behaviors).

**Context:** The settings mockup requires 12 new widgets (plus RichLabel from Section 07). Each is built on the new framework
established in Sections 01-08: interaction state for hover, controllers for click/drag,
visual state animators for transitions, and the new theme tokens for colors.

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

- [ ] Define `NavItem`:
  ```rust
  pub struct NavItem {
      pub label: String,
      pub icon: Option<IconId>,
      pub page_index: usize,
  }
  ```
- [ ] Define `SidebarNavWidget`:
  - Fixed width (200px logical)
  - Background: `theme.bg_secondary` (darker than primary surface)
  - Section titles: uppercase, small font, `fg_faint` color
  - Nav items: HoverController + ClickController
  - Active item: `accent_bg_strong` background, `accent` text color
  - Hover: `bg_hover` background transition (100ms EaseOut)
  - Emits: `WidgetAction::Selected { id, index: page_index }`
  - Rounded corners on items (6px)
  - Version label at bottom: `fg_faint`, small font
- [ ] Visual states: `CommonStates { Normal, Hovered, Active }` per nav item
  (Active here means "selected page", not mouse-down)

---

## 09.2 PageContainer

**File(s):** `oriterm_ui/src/widgets/page_container/mod.rs`

Shows one child at a time, switches on command.

- [ ] Define `PageContainerWidget`:
  ```rust
  pub struct PageContainerWidget {
      id: WidgetId,
      pages: Vec<Box<dyn Widget>>,
      active_page: usize,
  }
  ```
- [ ] `layout()`: only lay out the active page. Other pages get zero-size layout.
- [ ] `paint()`: only paint the active page.
- [ ] `set_active_page(index)`: switch pages, `request_paint()`
- [ ] `accept_action()`: handle `Selected` from SidebarNav to switch pages
- [ ] `sense()`: `Sense::none()` (delegates to active page's children)

---

## 09.3 SettingRow (Enhanced FormRow)

**File(s):** `oriterm_ui/src/widgets/setting_row/mod.rs`

Two-line label with hover background highlight.

- [ ] Define `SettingRowWidget`:
  - Left side: name (13px, `fg_primary`) + description (11.5px, `fg_secondary`)
  - Right side: control widget (dropdown, toggle, slider, etc.)
  - Full-width hover background: rounded rect, `bg_card` on hover (100ms fade)
  - Minimum height: 44px
  - HoverController for hover state
  - Visual states: `CommonStates { Normal, Hovered }`
  - `sense()`: `Sense::hover()` (row itself is hoverable, control handles clicks)

---

## 09.4 SchemeCard

**File(s):** `oriterm_ui/src/widgets/scheme_card/mod.rs`

Color scheme preview card with terminal preview and swatch bar.

- [ ] Define `SchemeCardWidget`:
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
- [ ] Terminal preview rendering:
  - Fixed-height area (56px) with scheme background
  - Render 2 lines of monospace text using scheme's foreground/ANSI colors
  - Text content: `$ cargo build --release` + `Compiling ori_term v0.1.0`
  - Uses current terminal font at 11px size

---

## 09.5 ColorSwatchGrid & SpecialColorSwatch

**File(s):** `oriterm_ui/src/widgets/color_swatch/mod.rs`

Clickable color grids for palette editing.

- [ ] Define `ColorSwatchGrid`:
  - 8 columns, dynamic rows
  - Each cell: colored rounded square (6px corners)
  - Cell label below: index number (9.5px, `fg_faint`)
  - Hover: enlarge cell slightly (redraw at 115% size, not transform)
  - Click: emit `Selected { id, index }` for future color picker
  - HoverController + ClickController per cell
  - Use grid layout from Section 07

- [ ] Define `SpecialColorSwatch`:
  - Large swatch (28x28px, 6px corners) + label + hex value
  - Label: 11px `fg_primary`
  - Hex value: 10px monospace `fg_faint`
  - Container with hover background
  - 4-column grid layout for the foreground/background/cursor/selection row

---

## 09.6 CodePreview

**File(s):** `oriterm_ui/src/widgets/code_preview/mod.rs`

Syntax-highlighted font preview panel.

- [ ] Define `CodePreviewWidget`:
  - Background: `bg_card` with rounded corners (8px)
  - Label: "Preview" in uppercase small text
  - Content: multi-line `RichLabel` with syntax-highlighted Rust code
  - Uses the terminal's configured font family and size
  - Colors from hardcoded syntax theme (keyword=purple, function=blue,
    string=green, comment=gray, number=orange)
  - `sense()`: `Sense::none()` (display only)

---

## 09.7 CursorPicker

**File(s):** `oriterm_ui/src/widgets/cursor_picker/mod.rs`

Visual cursor style selector with 3 options.

- [ ] Define `CursorPickerWidget`:
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

---

## 09.8 KeybindRow & KbdBadge

**File(s):** `oriterm_ui/src/widgets/keybind/mod.rs`

Keybinding display with styled key badges.

- [ ] Define `KbdBadge`:
  - Small rounded rect (4px corners) with bottom border thicker (2px for keycap depth)
  - Background: `bg_input`, border: `border` color
  - Text: 11px `fg_primary`
  - `sense()`: `Sense::none()` (display only)

- [ ] Define `KeybindRow`:
  - Left: action name label (13px `fg_primary`)
  - Right: row of KbdBadge widgets separated by "+" labels
  - Hover: `bg_card` background
  - `sense()`: `Sense::hover()`
  - Future: click to rebind (not in initial implementation)

---

## 09.9 NumberInput & RangeSlider

**File(s):** `oriterm_ui/src/widgets/number_input/mod.rs`,
  `oriterm_ui/src/widgets/range_slider/mod.rs`

**WidgetAction additions**: Review whether existing `WidgetAction` variants suffice
for all new widgets. Current variants cover: `Clicked`, `Toggled`, `ValueChanged`,
`TextChanged`, `Selected`, `OpenDropdown`, `DismissOverlay`, `MoveOverlay`,
`SaveSettings`, `CancelSettings`, `WindowMinimize/Maximize/Close`. The new widgets
primarily use `Selected` (SidebarNav, SchemeCard, CursorPicker, ColorSwatch) and
`ValueChanged` (NumberInput, RangeSlider). No new variants needed unless
`DoubleClicked`/`TripleClicked` are added for ClickController (Section 04). If they
are, add them to the `WidgetAction` enum in `widgets/mod.rs`.

- [ ] **NumberInput**:
  - Numeric text input with min/max/step constraints
  - Centered text, compact width (80px)
  - Arrow key increment/decrement
  - Background: `bg_input`, border: `border`
  - Focus: accent border
  - FocusController + ClickController
  - Emits: `WidgetAction::ValueChanged { id, value }`

- [ ] **RangeSlider** (enhanced Slider):
  - Horizontal track with filled portion (accent color)
  - Rounded thumb (14px circle) with shadow
  - Value label to the right (monospace, `fg_secondary`)
  - DragController for thumb
  - HoverController for thumb highlight
  - `sense()`: `Sense::drag().union(Sense::FOCUS)`
  - Emits: `WidgetAction::ValueChanged { id, value }`

---

## 09.10 Completion Checklist

- [ ] SidebarNav renders with section titles, nav items, icons, active state
- [ ] PageContainer switches pages on nav selection
- [ ] SettingRow shows name + description with hover background
- [ ] SchemeCard renders terminal preview + swatch bar with selection state
- [ ] ColorSwatchGrid renders 8-column grid with hover enlargement
- [ ] SpecialColorSwatch renders swatch + label + hex value
- [ ] CodePreview renders syntax-highlighted code with terminal font
- [ ] CursorPicker shows 3 cursor options with selection state
- [ ] KeybindRow shows action name + KbdBadge key labels
- [ ] NumberInput accepts numeric input with constraints
- [ ] RangeSlider shows filled track + value label
- [ ] All widgets use new framework (controllers, visual states, sense)
- [ ] All widgets render correctly at 100% and 150% DPI
- [ ] 8 new `IconId` variants added with SVG path definitions and test coverage
- [ ] Each new widget has a `tests.rs` sibling file (per test organization rules):
  `sidebar_nav/tests.rs`, `page_container/tests.rs`, `setting_row/tests.rs`,
  `scheme_card/tests.rs`, `color_swatch/tests.rs`, `code_preview/tests.rs`,
  `cursor_picker/tests.rs`, `keybind/tests.rs`, `number_input/tests.rs`,
  `range_slider/tests.rs`
- [ ] Each new widget module declared in `oriterm_ui/src/widgets/mod.rs`
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** Each new widget renders correctly in isolation (verified via test or
manual inspection). All widgets use the new framework — no manual hover tracking, no
legacy event methods.
