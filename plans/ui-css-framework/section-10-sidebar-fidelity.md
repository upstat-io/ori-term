---
section: "10"
title: "Visual Fidelity: Sidebar + Navigation"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-25
goal: "The settings sidebar matches the mockup's structure and interaction model: a full-height 200px rail with a real search input, precise section/header/nav/footer spacing, correct active and hover treatment, working modified dots, and interactive footer metadata"
depends_on: ["01", "02", "03", "08"]
sections:
  - id: "10.1"
    title: "Sidebar Structure + Module Split"
    status: not-started
  - id: "10.2"
    title: "Search Field Fidelity + Behavior"
    status: not-started
  - id: "10.3"
    title: "Section Headers + Nav Rhythm"
    status: not-started
  - id: "10.4"
    title: "Active States + Modified Dots"
    status: not-started
  - id: "10.5"
    title: "Footer Metadata + Actions"
    status: not-started
  - id: "10.6"
    title: "Tests"
    status: not-started
  - id: "10.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "10.7"
    title: "Build & Verify"
    status: not-started
---

# Section 10: Visual Fidelity - Sidebar + Navigation

## Problem

The current sidebar is missing several real behaviors and API surfaces required by the mockup.

What the code actually has today:

- `oriterm_ui/src/widgets/sidebar_nav/mod.rs` is a single `510`-line file (already exceeding the
  repository's 500-line limit) before this section adds more state or interactions.
- The widget paints a placeholder search box directly in `paint_search_field()` instead of using a
  real input control. There is no search icon, no focus-border treatment, and no text editing.
- The widget API only exposes `with_version(...)` and `with_config_path(...)`. The mockup
  footer also includes an `Update Available` link plus hover/click treatment for the config-path
  row.
- The active-row background is painted only to the right of the `3px` indicator strip,
  while the CSS model applies background to the full row box and overlays the left border.
- Section title and row spacing are bundled into coarse constants (`SECTION_TITLE_HEIGHT = 28.0`,
  `ITEM_HEIGHT = 32.0`), though the mockup uses separate search spacing, title bottom margins,
  and inter-section top margins.
- The widget can paint modified dots, but no app call site drives `set_page_modified(...)`,
  so the mockup's unsaved-state indicator is not actually wired.
- The current code applies `SIDEBAR_PADDING_X = 10.0` uniformly to nav items, titles, and footer,
  but the mockup has different horizontal insets per element: search container `0 10px`, nav items
  span full sidebar width with `padding: 7px 16px`, titles `padding: 0 16px`, footer
  `padding: 8px 16px`. Nav items are wrong (inset by 10px instead of spanning full width).

Section 10 delivers the sidebar as a real interactive surface, not a static paint pass:

1. split `sidebar_nav` into maintainable submodules before adding more logic
2. replace the painted search placeholder with a real search input using shared text-input
   behavior and mockup-specific styling
3. encode sidebar geometry explicitly instead of hiding spacing in approximate constants
4. wire modified dots to real per-page dirty state
5. extend the footer contract so `Update Available` and the config-path row can render and act
   like real interactive targets

The implementation keeps `SidebarNavWidget` as the top-level widget and gives it better internal
structure and state.

---

## 10.1 Sidebar Structure + Module Split

### Goal

Move the sidebar onto a maintainable widget boundary that can support search, footer actions, and
precise layout without growing the current monolith further.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
- `oriterm_ui/src/widgets/sidebar_nav/tests.rs`
- `oriterm/src/app/settings_overlay/form_builder/mod.rs`

### Required Structure

Split `sidebar_nav` before landing the fidelity work:

```text
oriterm_ui/src/widgets/sidebar_nav/
    mod.rs
    geometry.rs
    input.rs
    paint.rs
    tests.rs
```

Recommended ownership:

- `mod.rs`
  - public types (`NavSection`, `NavItem`, `SidebarNavWidget`, style structs)
  - builder/accessor API
  - shared widget state
- `geometry.rs`
  - exact sidebar rect math
  - section/title/item/footer/search metrics
  - hit-test enums for nav items and footer targets
- `input.rs`
  - pointer/focus/key routing
  - search-input delegation
  - footer target activation
- `paint.rs`
  - background, search field, titles, items, footer, separators

### Widget Tree Architecture Decision

`SidebarNavWidget` is currently a **leaf widget** — it has no `for_each_child_mut` override,
returns `LayoutBox::leaf()` from `layout()`, and handles all input directly via `on_input()`.
Embedding a `TextInputWidget` as a true child widget would require converting it to a container
(implement `for_each_child_mut`, change `layout()` to a tree, add `focusable_children`, route
`accept_action` to propagate actions).

**Decision**: Keep `SidebarNavWidget` as a leaf but **manage search state internally** using
shared editing primitives extracted into `TextEditingState`. This avoids a container conversion
that would require reworking the existing hit-test and input dispatch code.

The search field's visual rendering is painted directly by `paint.rs` using exact mockup styling,
which differs enough from `TextInputStyle` defaults that a custom paint path is cleaner than
style overrides.

### TextEditingState Extraction

The `TextInputWidget` editing helpers (`delete_selection`, `next_char_boundary`,
`prev_char_boundary`, `move_left`, `move_right`, `cursor_x`, `selection_range`) total ~80 lines
(lines 167, 222-301 in `text_input/mod.rs`). This exceeds the ~50-line duplication threshold, so
extraction is required. Extract a `TextEditingState` struct into
`oriterm_ui/src/text/editing/mod.rs` with `pub(crate)` visibility, following the sibling
`tests.rs` pattern (`editing/mod.rs` + `editing/tests.rs`). Both `TextInputWidget` and
`SidebarNavWidget` will use it. Add `pub mod editing;` to `oriterm_ui/src/text/mod.rs`.

`TextEditingState` must own at minimum: `text: String`, `cursor: usize`,
`selection_anchor: Option<usize>`. It must expose the same methods as the current helpers,
plus `insert_char`, `backspace`, `delete`, `home`, `end`, and `select_all`.

### Widget Contract Changes

Keep one `SidebarNavWidget`, but expand its internal model:

- add internal search state (`search_state: TextEditingState`, `search_focused: bool`)
- add optional footer update metadata
- add hover/hit-test state for: search field (focus border), update link, config-path row
  (nav row hover already exists via `hovered_item`)
- wire modified dots: the existing `set_page_modified(usize, bool)` API works, but the real gap
  is app-side call sites (wired in 10.4)

The top-level widget ID should remain stable for page-selection routing. Footer targets use
dedicated action variants rather than overloading the nav-row `Selected { index }` path.

### Checklist

- [ ] Extract `TextEditingState` into `oriterm_ui/src/text/editing/mod.rs` (~80 lines from
  `TextInputWidget`: `delete_selection`, `next/prev_char_boundary`, `move_left/right`,
  `cursor_x`, `selection_range`, plus new `insert_char`, `backspace`, `delete`, `home`, `end`,
  `select_all`). Create as directory module (`editing/mod.rs` + `editing/tests.rs`).
- [ ] Refactor `TextInputWidget` to use `TextEditingState` instead of its own inline helpers
- [ ] Add `pub mod editing;` to `oriterm_ui/src/text/mod.rs`
- [ ] Split `sidebar_nav` into submodules (`mod.rs`, `geometry.rs`, `input.rs`, `paint.rs`)
- [ ] Verify each resulting submodule stays under 500 lines
- [ ] Keep one top-level `SidebarNavWidget` as a leaf widget for page-selection integration
- [ ] Add internal search state (`search_state: TextEditingState`, `search_focused: bool`)
- [ ] Add `HoveredFooterTarget` enum for footer hover state
- [ ] Add optional footer update metadata fields

---

## 10.2 Search Field Fidelity + Behavior

### Goal

Replace the fake painted search placeholder with a real input that matches the mockup visually and
behaves like an actual control.

### Files

- `oriterm_ui/src/text/editing/mod.rs` (new — `TextEditingState` extracted in 10.1)
- `oriterm_ui/src/text/editing/tests.rs` (new — unit tests for `TextEditingState`)
- `oriterm_ui/src/text/mod.rs` (add `pub mod editing;`)
- `oriterm_ui/src/widgets/text_input/mod.rs` (refactor to use `TextEditingState`)
- `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
- `oriterm_ui/src/widgets/sidebar_nav/input.rs`
- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `oriterm_ui/src/icons/mod.rs` (add `IconId::Search` variant, `ALL`, `path()` match)
- `oriterm_ui/src/icons/sidebar_nav.rs` (add `ICON_SEARCH` static path definition)
- `oriterm/src/gpu/icon_rasterizer/mod.rs`
- `oriterm/src/gpu/window_renderer/icons.rs` (add `(IconId::Search, 12)` to `ICON_SIZES`)

### Current Boundary Problem

The repo has `TextInputWidget` with full editing support, but the sidebar bypasses it and paints
a dead box. Per the 10.1 architecture decision, the sidebar stays as a leaf widget with internal
search state via `TextEditingState`. The `TextInputWidget` helper methods are `pub(super)` and
not accessible from `sidebar_nav`; the 10.1 extraction to `pub(crate)` resolves this.

### Implementation Plan

Add a real search field to `SidebarNavWidget` using internal state. Use local constants for search
field dimensions in this subsection; 10.3 will centralize all geometry into `geometry.rs`.

- add `search_state: TextEditingState` and `search_focused: bool` fields
- handle keyboard input for search (character insert, backspace, delete, arrow keys, Home/End,
  Ctrl+A) when the search field is focused, delegating to `TextEditingState` methods
- handle mouse click on search field rect to toggle focus and position cursor
- paint the search field with exact mockup styling (not `TextInputStyle` defaults)
- sidebar-specific search style:
  - height `28px`
  - background `theme.bg_primary` (`#16161c` = CSS `--bg-surface`). Do NOT use `theme.bg_input`
    (`#12121a` = CSS `--bg-input`), which is for form inputs in the content area.
  - border width `2px`
  - unfocused border `theme.border`
  - focused border `theme.accent`
  - placeholder color `theme.fg_faint`
  - font size `12px`
  - padding equivalent to mockup `6px 8px 6px 26px` (TLBR — `26px` left includes space for
    the search icon)

### Search Icon

The mockup includes a leading search icon inside the field (a magnifying glass: circle + diagonal
line, rendered as a CSS `background-image` SVG at `12x12` px positioned at `8px center`). Section
08 covered the eight sidebar page icons but not this glyph.

Add the search icon through the shared icon pipeline:

- add `IconId::Search` variant to `oriterm_ui/src/icons/mod.rs`
- add it to `IconId::ALL`
- add the static `ICON_SEARCH: IconPath` definition in `oriterm_ui/src/icons/sidebar_nav.rs`
  (following the existing sidebar icon pattern) — the SVG is:
  circle(cx=11, cy=11, r=8) + line(21,21 to 16.65,16.65) in viewBox 0 0 24 24
- add a `Search` match arm in `IconId::path()` mapping to `sidebar_nav::ICON_SEARCH`
- add `(IconId::Search, 12)` to `WindowRenderer::ICON_SIZES` in
  `oriterm/src/gpu/window_renderer/icons.rs` and update the array length from 15 to 16
  (the existing `icon_sizes_covers_all_icon_ids` test will fail if this step is missed)
- paint it at `(search_x + 8.0, field_y + center)` inside the search field rect

### Search Behavior

The field must be real, not cosmetic:

- keyboard-editable (via `Key::Character`, `Key::Backspace`, `Key::Delete`, `Key::ArrowLeft/Right`,
  `Key::Home/End`, and `Ctrl+A` for select-all)
- focusable (click on search rect sets `search_focused = true` and positions cursor at click X)
- unfocusable (Escape key or click outside search rect clears focus)
- accent focus border on focus
- query-driven local filtering of sidebar content

Local filtering: case-insensitive (`str::to_lowercase()` comparison) over section titles and item
labels, preserving original order. Feasible because the settings sidebar has a small fixed set
of pages.

Page state coherence:

- filtering must not silently switch pages
- if the active page does not match the query, keep its row visible and marked active until the
  user selects a matching row or clears the query

### Checklist

- [ ] Add `search_state: TextEditingState` and `search_focused: bool` fields to `SidebarNavWidget`
- [ ] Replace `paint_search_field()` placeholder logic with real rendering (bg, border, caret, text)
- [ ] Use `theme.bg_primary` for search bg (not `theme.bg_input`)
- [ ] Apply exact mockup search styling: 28px height, 2px border, 12px font, `6px 8px 6px 26px` padding
- [ ] Add `IconId::Search` variant to `oriterm_ui/src/icons/mod.rs` (enum + ALL array + path() match)
- [ ] Add `ICON_SEARCH` static path definition to `oriterm_ui/src/icons/sidebar_nav.rs`
- [ ] Add `(IconId::Search, 12)` to `ICON_SIZES` in `window_renderer/icons.rs` (update array len 15 to 16)
- [ ] Render search icon at `(search_x + 8.0, vertically centered)` inside the field
- [ ] Handle keyboard input when search focused (chars, backspace, delete, arrows, Home/End, Ctrl+A)
- [ ] Handle mouse click on search rect to focus AND position cursor at click X offset
- [ ] Implement accent focus border transition on search field focus/unfocus
- [ ] Filter sidebar sections/items by query without forcing an automatic page switch
- [ ] Keep active page row visible even if it doesn't match the search query

---

## 10.3 Section Headers + Nav Rhythm

### Goal

Match the mockup's spacing and typography by making sidebar geometry explicit instead of relying on
approximate bundled constants.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/geometry.rs`
- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `mockups/settings-brutal.html`

### Layout Facts From The Mockup

The mockup uses separate spacing rules per element. The sidebar has `padding: 16px 0` —
**vertical only, zero horizontal** — so child elements have different horizontal insets.
The current code applies `SIDEBAR_PADDING_X=10` uniformly, which is only correct for the
search container.

- sidebar width `200px`, sidebar padding `16px 0` (vertical only — `SIDEBAR_PADDING_Y`)
- search container `.sidebar-search`: `padding: 0 10px` (inset 10px from each side)
- search field height `28px`, search-to-first-title gap `12px` (= `margin-bottom: 12px`)
- title `.sidebar-title`: `padding: 0 16px` (16px from sidebar left edge)
- title bottom margin `8px`
- non-first title top margin `20px` (`:not(:first-child)` rule)
- nav items `.nav-item`: span full sidebar width, `padding: 7px 16px`, `margin: 1px 0`,
  `border-left: 3px solid transparent`
- nav item font size `13px`, icon/text gap `10px`
- footer `.sidebar-footer`: `padding: 8px 16px` (16px from sidebar left edge)

**Current code vs. mockup horizontal positioning:**

| Element       | Current position          | Correct position (from sidebar left) |
|---------------|---------------------------|--------------------------------------|
| Search field  | `x` = sidebar+10          | sidebar+10 (correct)                 |
| Title text    | `x + 6` = sidebar+16      | sidebar+16 (correct by coincidence)  |
| Nav item rect | `x` = sidebar+10, w=180   | sidebar+0, w=200 (full width)        |
| Nav icon      | `x + 3 + 8` = sidebar+21  | sidebar+3+16 = sidebar+19            |
| Nav text      | `x + 3 + 32` = sidebar+45 | sidebar+3+16+16+10 = sidebar+45 (correct by coincidence) |
| Footer text   | `x + 6` = sidebar+16      | sidebar+16 (correct by coincidence)  |

The fix: stop using `SIDEBAR_PADDING_X` as a universal inset. Compute each element's rect from the
sidebar left edge using that element's own CSS padding.

**Nav item CSS model:** The mockup applies `border-left: 3px solid transparent` on **every** nav
item, not just active. The active state changes `border-left-color: var(--accent)`. The `3px`
indicator is part of the row's box model (left border), not a separate overlay. Content starts
after 3px border + 16px padding = 19px from the sidebar left edge.

**Derived nav row outer height:** 7px top padding + ~13px text + 7px bottom padding = ~27px
content + 1px top margin + 1px bottom margin = ~29px outer. The current `ITEM_HEIGHT = 32.0`
is an approximation — the exact value should be derived from padding + line height + margin.

### Required Geometry Rewrite

Replace the current rough layout math with explicit geometry helpers. **Nav items span the full
sidebar width** (not inset by `SIDEBAR_PADDING_X`). All geometry helpers accept
`sidebar_bounds: Rect` and compute rects from `sidebar_bounds.x()`.

Concrete helpers needed in `geometry.rs`:

- `search_field_rect(sidebar_bounds)` -> `Rect` at x+10, w=sidebar_w-20 (search has its own 10px inset)
- `title_rect(sidebar_bounds, is_first)` -> `Rect` at x+0, with internal 16px padding for text
- `nav_item_rect(sidebar_bounds)` -> `Rect` at x+0, full sidebar width, derived height
- `nav_content_x(sidebar_bounds)` -> icon X = sidebar_x + 3 + 16 = sidebar_x + 19
- `nav_text_x(sidebar_bounds)` -> text X = sidebar_x + 3 + 16 + 16 + 10 = sidebar_x + 45
- `footer_rect(sidebar_bounds)` -> `Rect` anchored to bottom, internal 16px padding

Replace `ITEM_HEIGHT = 32.0` with a derived value: `padding-top(7) + content(~13) +
padding-bottom(7) + margin(1+1) = ~29px`. Codify this in `geometry.rs`.

Replace `SECTION_TITLE_HEIGHT = 28.0` with explicit separate values: title text paint offset,
`8px` bottom margin, and conditional `20px` top margin for non-first titles.

Replace `SEARCH_AREA_HEIGHT = 40.0` with a value derived from `search_field_rect()` height + the
`12px` search-to-first-title gap.

### Typography Requirements

Section titles must keep:

- font size `10`
- regular weight
- uppercase transform
- letter spacing `1.5` (= CSS `0.15em * 10px`)
- `// ` prefix
- `theme.fg_faint`

If Section 03 lands a shared text-transform/letter-spacing path, use it.

### Checklist

- [ ] Stop applying `SIDEBAR_PADDING_X` to nav items — nav item rects span full sidebar width (200px)
- [ ] Keep `SIDEBAR_PADDING_X` only for search field (`.sidebar-search { padding: 0 10px }`)
- [ ] Replace `SECTION_TITLE_HEIGHT` with separate title bottom margin (8px) and non-first top margin (20px)
- [ ] Replace `ITEM_HEIGHT = 32.0` with derived value (~29px) from padding (7px) + content + margin (1px each)
- [ ] Replace `SEARCH_AREA_HEIGHT = 40.0` with derived value from search rect height + 12px gap
- [ ] Fix icon X: from sidebar_x+21 to sidebar_x+19
- [ ] Note: text X is already correct by coincidence (sidebar_x+45 = sidebar_x+3+16+16+10)
- [ ] Update title text X to use `bounds.x() + 16.0` directly (currently `x + 6.0 = bounds.x() + 16`
  is correct by coincidence, but `x` changes when `SIDEBAR_PADDING_X` is removed)
- [ ] Update footer text X similarly — use `bounds.x() + 16.0` directly
- [ ] Centralize all sidebar geometry in `geometry.rs` instead of scattering offsets in paint code
- [ ] Update `hit_test_item()` to use the same geometry helpers
- [ ] Account for search field filtering in hit-test: filtered-out items should not be hit-testable

---

## 10.4 Active States + Modified Dots

### Goal

Make active, hover, and dirty indicators match the mockup both visually and behaviorally.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `oriterm_ui/src/widgets/sidebar_nav/input.rs`
- `oriterm_ui/src/action/mod.rs` (add `PageDirty` variant to `WidgetAction`)
- `oriterm/src/app/dialog_context/content_actions.rs` (wire per-page dirty in `dispatch_dialog_settings_action`)

### Active + Hover Painting

The current widget has the right colors but the geometry and paint order are wrong — active/hover
backgrounds are painted only to the right of the `3px` indicator strip (`bg_x = x +
INDICATOR_WIDTH`, `bg_w = item_w - INDICATOR_WIDTH`).

The mockup CSS model: `border-left: 3px solid transparent` is on all items, and active sets
`border-left-color: var(--accent)`. The background covers the full row including the border area.

Correct paint order:

1. Paint active/hover background across the **full row rect** (from x=0 of the item, full width)
2. Paint the `3px` left border on top (transparent for inactive, accent for active)
3. Keep icon opacity at `0.7` for normal and hover rows
4. Lift icon opacity to `1.0` only for the active row
5. Hover text color = `theme.fg_primary` (= CSS `--text`), not `theme.fg_secondary`

### Nav Item Insets

After the 10.3 geometry rewrite, nav item rects span the full sidebar width. All offsets are
from the sidebar left edge (`sidebar_x`):

| Element  | Current (from sidebar_x)     | Correct (from sidebar_x) |
|----------|------------------------------|--------------------------|
| Icon X   | +10 + 3 + 8 = +21           | +3 + 16 = +19            |
| Text X   | +10 + 3 + 32 = +45          | +3 + 16 + 16 + 10 = +45  |

The icon is 2px too far right. The text is correct by coincidence. After 10.3, both offsets should
use `geometry.rs` helpers (`nav_content_x`, `nav_text_x`).

### Modified Dots

The widget can paint a `6px` warning dot via `set_page_modified(usize, bool)` and
`is_page_modified()`, but there are **no current call sites**. The data flow must be wired.

**Existing global dirty mechanism:** `dispatch_dialog_settings_action()` in
`content_actions.rs` already computes global dirty state (`pending_config != original_config`)
and sends `WidgetAction::SettingsUnsaved(dirty)` to the panel (line 236). Per-page dirty state
is additive — global tracks the chrome title indicator while per-page dots show which specific
pages have changes.

Required integration:

1. **Add a per-page dirty comparison function** in `oriterm/src/app/settings_overlay/` that
   compares pending vs original config per section:
   - page 0 (Appearance): `config.window.opacity`, `.blur`, `.unfocused_opacity`, `.decorations`,
     `.tab_bar_style`, `.colors.scheme`
   - page 1 (Colors): `config.colors.scheme`
   - page 2 (Font): `config.font.*`
   - page 3 (Terminal): `config.terminal.*`, `config.behavior.warn_on_paste`
   - page 4 (Keybindings): no settings yet — always clean
   - page 5 (Window): `config.window.tab_bar_position`, `.grid_padding`, `.restore_session`,
     `.columns`, `.rows`
   - page 6 (Bell): `config.bell.*`
   - page 7 (Rendering): `config.rendering.gpu_backend`, `config.font.subpixel_mode`

2. **Add `WidgetAction::PageDirty { page: usize, dirty: bool }` variant** to
   `oriterm_ui/src/action/mod.rs`. Handle it in `SidebarNavWidget::accept_action()` by
   calling `self.set_page_modified(page, dirty)`. In `dispatch_dialog_settings_action()`,
   after `handle_settings_action()` returns true, compute per-page dirty for each page and
   send `PageDirty` actions to the panel.

   **Exhaustive match update:** `PageDirty` is only meaningful to `SidebarNavWidget`. All
   other match sites need a no-op arm (`WidgetAction::PageDirty { .. } => {}`). Search for
   `WidgetAction::` across the workspace to find all sites — the `handle_dialog_content_action()`
   catch-all at line ~70 in `content_actions.rs` is the most important one to update.

3. Keep dirty-dot paint at `6px` and `theme.warning`, right-aligned with `16px` right margin.

### Checklist

- [ ] Paint active and hover backgrounds across the **full** row box (remove `bg_x = x + INDICATOR_WIDTH`)
- [ ] Paint `3px` left border for ALL items: transparent for inactive, `theme.accent` for active
- [ ] Paint background FIRST, then border ON TOP (matching CSS box model)
- [ ] Fix icon X: from sidebar_x+21 to sidebar_x+19 (see insets table)
- [ ] Verify text X remains at sidebar_x+45 after 10.3 geometry rewrite
- [ ] Add per-page dirty comparison function in `oriterm/src/app/settings_overlay/`
- [ ] Add `WidgetAction::PageDirty { page: usize, dirty: bool }` variant to `oriterm_ui/src/action/mod.rs`
- [ ] Update all exhaustive match sites for new `WidgetAction` variant (no-op arm at all sites
  except `SidebarNavWidget::accept_action()` which calls `set_page_modified()`)
- [ ] Wire per-page dirty computation in `dispatch_dialog_settings_action()` (`content_actions.rs`)
- [ ] Keep dirty-dot paint at `6px` and `theme.warning`, right-aligned with `16px` right margin

---

## 10.5 Footer Metadata + Actions

### Goal

Make the footer match the mockup's content, spacing, and interaction model instead of rendering two
static text strings at the bottom edge.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
- `oriterm_ui/src/widgets/sidebar_nav/geometry.rs`
- `oriterm_ui/src/widgets/sidebar_nav/input.rs`
- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `oriterm_ui/src/action/mod.rs` (add `FooterAction` variant to `WidgetAction`)
- `oriterm/src/app/settings_overlay/form_builder/mod.rs`
- `oriterm/src/app/dialog_context/content_actions.rs`

### Current Gap

The mockup footer contains:

- version text
- `Update Available` link with accent hover color and underline on hover
- config-path row with dim default opacity, ellipsis behavior, and accent hover treatment

The current widget can only show version text and config path text. There is no way to express an
update link, no hover state for footer rows, and no action-routing contract for footer clicks.

### Footer Contract Rewrite

Extend `SidebarNavWidget`:

- keep `with_version(...)`
- keep `with_config_path(...)`
- add optional update metadata: label text, tooltip (available version), visibility flag

### Interaction Model

Add separate hover/hit targets for:

- update link
- config path row

For emitted actions, **do not use** `WidgetAction::Clicked(id)` — the current
`handle_dialog_content_action()` routes all `Clicked` actions to `execute_confirmation()`
(line 61-63 in `content_actions.rs`), which would misfire for footer clicks. Instead, add a
dedicated `WidgetAction::FooterAction { target: FooterTarget }` variant.

Do not overload nav-item `Selected { index }` for footer actions.

### Visual Requirements

Implement the mockup footer layout explicitly:

- footer padding: `8px 16px`. After 10.3 removes the universal `SIDEBAR_PADDING_X` inset, use
  `bounds.x() + 16.0` directly (or a `footer_text_x()` geometry helper)
- `4px` gap between version row and config path row (current code has this)
- right-sidebar border remains `2px`
- version text `11px`, `theme.fg_faint` color (current code has this)
- update link `10px`, `font-weight: 500` (= `FontWeight::MEDIUM` — requires Section 02),
  `theme.accent` by default, `theme.accent_hover` on hover
- update link hover underline: paint a 1px line below the text baseline at `2px` offset (the
  framework has no built-in text underline; render manually via `push_quad` with a thin rect)
- version + update link on the same line with `6px` gap (mockup: `display: flex; gap: 6px`)
- config path `10px`, `theme.fg_faint` at `opacity: 0.7` by default
- config path truncation: measure shaped text width vs available footer width. If text exceeds
  available width, truncate the source string at a char boundary and append `\u{2026}` (ellipsis,
  width 1) before shaping. There is no framework-level `text-overflow: ellipsis`; this must be
  done before shaping.
- config path hover restores `opacity: 1.0` and uses `theme.accent` coloring

### App Integration

Update the settings sidebar builder in `form_builder/mod.rs` to populate the richer footer model:

- continue to set the package version
- provide the config path through the real config-path helper
- provide update-link metadata only when the app has update information

### Footer Action Handling

Add `open` crate as a dependency in `oriterm/Cargo.toml` for cross-platform file/URL opening.
Handle footer actions in `content_actions.rs`:

- **Config path click**: `open::that(&config_path)` opens the file in the system's default
  editor/handler. Works cross-platform (macOS, Linux, Windows). Degrade gracefully if `open`
  fails (log warning, do not panic).
- **Update link click**: `open::that(&update_url)` opens the URL in the default browser.
  Same cross-platform behavior and error handling.

### Checklist

- [ ] Add `update_label: Option<String>`, `update_tooltip: Option<String>` fields to widget
- [ ] Add `with_update_available(label, tooltip)` builder method
- [ ] Add `HoveredFooterTarget` enum (None, UpdateLink, ConfigPath) for footer hover state
- [ ] Add footer hit-test logic in `geometry.rs` for update link and config path rects
- [ ] Update footer text X to use `bounds.x() + 16.0` directly after 10.3 geometry rewrite
- [ ] Paint version + update link on same line with 6px gap
- [ ] Paint update link at 10px, font-weight 500, accent color (hover: accent_hover + manual underline)
- [ ] Paint config path at 10px, faint color, opacity 0.7 (hover: opacity 1.0, accent color)
- [ ] Truncate config-path text with manual ellipsis before shaping (no framework text-overflow)
- [ ] Add `WidgetAction::FooterAction { target: FooterTarget }` variant to `oriterm_ui/src/action/mod.rs`
- [ ] Define `FooterTarget` enum (UpdateLink, ConfigPath) in `sidebar_nav/mod.rs`
- [ ] Update all exhaustive match sites for new `FooterAction` variant (same pattern as `PageDirty`)
- [ ] Add `open` crate to `oriterm/Cargo.toml` for cross-platform file/URL opening
- [ ] Update `form_builder/mod.rs` to populate update metadata from real app state
- [ ] Handle footer actions in `content_actions.rs` via `open::that()`
- [ ] When `update_label` is `None`, do not paint or hit-test the update link region

---

## 10.6 Tests

### Goal

Turn sidebar fidelity into repeatable regression coverage instead of relying on manual inspection.

### Files

- `oriterm_ui/src/text/editing/tests.rs` (new — `TextEditingState` unit tests, list A)
- `oriterm_ui/src/widgets/sidebar_nav/tests.rs` (geometry, paint, search, footer tests — lists B-E)
- `oriterm/src/app/settings_overlay/form_builder/tests.rs` (builder metadata test, list F)

### Required Coverage

Keep the current interaction tests (22 existing tests in `sidebar_nav/tests.rs`), and add the
coverage the existing suite is missing.

**A. TextEditingState unit tests** (in `oriterm_ui/src/text/editing/tests.rs`):

- `fn empty_state_cursor_at_zero()` — new state has cursor=0, empty text
- `fn insert_ascii_updates_text_and_cursor()` — single char insert advances cursor
- `fn insert_multibyte_unicode()` — insert multi-byte character (e.g. `'ö'`, `'你'`), verify
  cursor lands on char boundary
- `fn backspace_at_start_is_noop()` — backspace at cursor=0 does nothing
- `fn backspace_removes_previous_char()` — backspace in middle removes the correct char
- `fn delete_at_end_is_noop()` — delete at end of text does nothing
- `fn delete_removes_next_char()` — delete in middle removes the correct char
- `fn move_left_at_start_is_noop()` — cursor stays at 0
- `fn move_right_at_end_is_noop()` — cursor stays at text.len()
- `fn select_all_selects_entire_text()` — Ctrl+A sets selection_anchor=0, cursor=text.len()
- `fn delete_selection_removes_range()` — selected text is deleted, cursor at start of range
- `fn home_moves_to_start()` — Home key moves cursor to 0
- `fn end_moves_to_end()` — End key moves cursor to text.len()

**B. Geometry unit tests** (in `oriterm_ui/src/widgets/sidebar_nav/tests.rs`):

- `fn search_field_rect_inset_10px()` — `search_field_rect()` returns rect at sidebar_x+10,
  width=sidebar_w-20
- `fn nav_item_rect_full_width()` — `nav_item_rect()` returns rect at sidebar_x+0, full 200px width
- `fn nav_content_x_at_sidebar_plus_19()` — icon X = sidebar_x + 19
- `fn nav_text_x_at_sidebar_plus_45()` — text X = sidebar_x + 45
- `fn title_rect_first_no_top_margin()` — first title has no top margin
- `fn title_rect_nonfirst_has_20px_top_margin()` — non-first title has 20px top margin
- `fn footer_rect_anchored_to_bottom()` — footer rect positioned at sidebar bottom
- `fn derived_nav_row_height()` — verify derived height (~29px) matches padding + content + margin

**C. Scene-level paint assertions** (in `oriterm_ui/src/widgets/sidebar_nav/tests.rs`):

- `fn paint_sidebar_full_height_background()` — full-height sidebar background and `2px` right border
- `fn paint_search_field_shape_and_colors()` — search field primitive shape and colors
- `fn paint_search_icon_present()` — search icon presence (verify `push_icon` was called with
  `IconId::Search` and size 12)
- `fn paint_active_row_full_background_with_indicator()` — full-row active background plus `3px`
  indicator (verify bg rect starts at row x, NOT at `x + INDICATOR_WIDTH`)
- `fn paint_hover_row_full_background()` — hover background also spans full row width
- `fn paint_footer_version_and_config_path()` — footer version/update/config-path primitives
- `fn paint_footer_update_link_accent_color()` — update link renders in accent color, not faint
- `fn paint_modified_dot_on_dirty_page()` — modified-dot painting when page is dirty
- `fn paint_no_modified_dot_on_clean_page()` — no modified dot when page is clean

**D. Search interaction tests** (in `oriterm_ui/src/widgets/sidebar_nav/tests.rs`):

- `fn search_click_sets_focus()` — click on search rect sets `search_focused = true`
- `fn search_click_positions_cursor()` — click at specific X offset positions cursor at
  corresponding text offset
- `fn search_typing_updates_text()` — character input when focused updates search text
- `fn search_backspace_deletes_char()` — backspace removes last character
- `fn search_filters_nav_items()` — local filtering hides non-matching items (hit test returns
  `None` for filtered-out items)
- `fn search_case_insensitive()` — filtering is case-insensitive ("font" matches "Font")
- `fn search_preserves_active_page()` — active-page preservation while filtering
- `fn search_empty_query_shows_all()` — clearing query restores all items
- `fn search_escape_unfocuses()` — Escape key unfocuses search field
- `fn search_no_results_shows_empty()` — query matching nothing hides all items without crash

**E. Footer interaction tests** (in `oriterm_ui/src/widgets/sidebar_nav/tests.rs`):

- `fn footer_update_link_hover()` — footer update link hover hit testing
- `fn footer_config_path_hover()` — footer config-path hover hit testing
- `fn footer_click_emits_footer_action()` — footer click emits `FooterAction`, not `Clicked`
- `fn footer_no_update_link_when_none()` — when `update_label` is `None`, no update link is
  rendered and hit-test returns `None` for that region

**F. Integration tests** (in `oriterm/src/app/settings_overlay/` test files):

- `fn sidebar_builder_populates_footer_metadata()` — in `form_builder/tests.rs`, footer builder
  metadata includes version and config path
- `fn per_page_dirty_comparison()` — per-page dirty comparison function correctly identifies which
  pages have pending changes. Test: appearance page dirty (opacity changed), font page dirty (size
  changed), keybindings page always clean, all pages clean when configs match.

Test pattern: geometry and state logic as unit tests, input/paint integration via
`WidgetTestHarness`.

### Checklist

- [ ] Add `TextEditingState` unit tests in `oriterm_ui/src/text/editing/tests.rs` (13 tests — list A)
- [ ] Add geometry unit tests for all `geometry.rs` helpers (8 tests — list B)
- [ ] Add scene-based paint assertions for sidebar fidelity (9 tests — list C)
- [ ] Add search interaction and filtering tests (10 tests — list D)
- [ ] Add footer hover/click/hidden-state tests (4 tests — list E)
- [ ] Add integration tests for builder and dirty-comparison (2 tests — list F)
- [ ] Preserve existing nav selection and keyboard behavior tests (22 existing)

---

## 10.R Third Party Review Findings

### Resolved Findings

- `TPR-10-001` The draft overstated what already matched. The current sidebar still lacks a real
  search control, a search icon, footer update-link content, and footer interaction states.
- `TPR-10-002` `oriterm_ui/src/widgets/sidebar_nav/mod.rs` is already over the repository file-size
  limit at `510` lines, so the section must split the module before adding more fidelity logic.
- `TPR-10-003` The draft treated the search box as a paint-only concern even though the repo already
  has a reusable `TextInputWidget`. Section 10 resolves this by adding real editing behavior
  internally (not by embedding `TextInputWidget` as a child — see 10.1 architecture decision).
- `TPR-10-004` The current active-row background geometry does not match the mockup CSS model
  because it is clipped to the right of the indicator strip instead of painting across the full row.
- `TPR-10-005` The footer API cannot currently represent the mockup's `Update Available` link or
  emit distinct footer actions. Section 10 must extend the widget contract instead of "verifying"
  behavior that does not exist.
- `TPR-10-006` Modified-dot painting exists, but no current app code drives page dirty state into
  the sidebar. The fidelity plan must include the missing data flow.
- `TPR-10-007` The existing sidebar tests are mostly behavioral and do not verify rendered scene
  fidelity, so visual regressions would currently pass unnoticed.

---

## 10.7 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

Suggested commands:

```bash
cargo test -p oriterm_ui sidebar_nav::tests
cargo test -p oriterm_ui text::editing::tests
cargo test -p oriterm settings_overlay::form_builder::tests
```

Manual verification checklist:

- [ ] Sidebar rail is full-height, `200px` wide, with `#0e0e12` background and `2px` right border
- [ ] Search field matches the mockup visually and behaves like a real input
- [ ] Section headers and nav-row spacing match the mockup
- [ ] Active, hover, and modified states match the mockup
- [ ] Footer shows version, optional update link, and config-path behavior correctly
