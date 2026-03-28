---
section: "01"
title: "UI Widgets Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in UI widgets"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Active Bugs"
    status: in-progress
---

# Section 01: UI Widgets Bugs

**Status:** In Progress
**Goal:** Track and fix all discovered bugs in UI widgets.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 01.1 Active Bugs

- [x] **BUG-01.1**: Dropdown indicator arrow invisible — uses Unicode glyph instead of SVG icon
  - **File(s)**: `oriterm_ui/src/widgets/dropdown/mod.rs:311-319`
  - **Root cause**: Dropdown renders its indicator as `"\u{25BE}"` (▾) via text shaping. IBM Plex Mono (the embedded UI font) doesn't include this glyph, so nothing renders.
  - **Found**: 2026-03-26 — visual verification during CSS framework Section 12
  - **Fixed**: 2026-03-26 — Added `IconId::DropdownArrow` (filled triangle, `IconStyle::Fill`, normalized from 10x6 viewbox). Replaced text-based shaping with `push_icon()` in Section 13.3.

- [x] **BUG-01.2**: All 6 disabled widgets still clickable — `disabled` flag never propagated to layout
  - **File(s)**: `oriterm_ui/src/widgets/button/mod.rs`, `toggle/mod.rs`, `dropdown/mod.rs`, `slider/widget_impl.rs`, `checkbox/mod.rs`, `text_input/widget_impl.rs`
  - **Root cause**: Every widget with a `disabled: bool` field built its `LayoutBox` without calling `.with_disabled(self.disabled)`. The `LayoutBox.disabled` flag stayed `false`, so `hit_test.rs:is_hittable()` never excluded disabled widgets from mouse interaction.
  - **Found**: 2026-03-26 — TPR-12-013 review of Section 12, then systematic audit of all widgets
  - **Fixed**: 2026-03-26 — Added `.with_disabled(self.disabled)` to `layout()` in all 6 widgets. Regression tests added in `button/tests.rs`: `disabled_button_layout_sets_disabled_flag`, `enabled_button_layout_clears_disabled_flag`, `disabled_button_not_hittable_in_harness`.

- [x] **BUG-01.3**: Slider drag not smooth — jumps around instead of following mouse
  - **File(s)**: `oriterm_ui/src/widgets/slider/mod.rs`, `widget_impl.rs`
  - **Root cause**: During mouse capture, the dispatch system passes fallback bounds from a different widget when the cursor leaves the slider. The slider used these wrong bounds in `value_from_x`, causing value jumps.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Cache `drag_bounds` at `DragStart` and use cached bounds throughout the drag instead of the live bounds passed to `on_action`

- [x] **BUG-01.4**: Toggle animation missing — thumb doesn't slide, only color changes
  - **File(s)**: `oriterm/src/app/dialog_rendering.rs`, `oriterm_ui/src/widgets/toggle/mod.rs`
  - **Root cause**: Dialog rendering used selective tree walks during animation. The selective walk's parent map only covered content widgets — chrome widgets (including toggles inside the dialog) were skipped, so `prepaint()` never called `tick()` on the animation property.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Dialog rendering now uses full tree walks (`None` tracker) when `ui_stale` is true, ensuring all animating widgets get `prepaint()` called. Also clears per-frame invalidation to prevent stale dirty marks from accumulating.

- [x] **BUG-01.5**: Text input cursor spans full input height — not bounded by padding
  - **File(s)**: `oriterm_ui/src/widgets/text_input/widget_impl.rs`
  - **Root cause**: Cursor rect used `inner.height()` (full padded area) instead of `shaped.height` (text line height). Selection highlight had the same issue.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Refactored paint to compute `line_h` from shaped text, then use it for both cursor and selection rects. Both are now vertically centered within the padded area matching the text position.

- [x] **BUG-01.6**: Number inputs not typeable — only up/down arrows work
  - **File(s)**: `oriterm_ui/src/widgets/number_input/mod.rs`
  - **Root cause**: `on_input` only handled ArrowUp/ArrowDown/MouseDown. No handler for character keys, backspace, or delete.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Added `input_buf` field for keyboard editing. Character keys (digits, '.', '-') append to buffer and parse into value. Backspace removes last char. Clicking the text area clears the buffer for fresh input. Arrow/stepper adjustments sync the buffer.

- [x] **BUG-01.7**: Text inputs missing hover border highlight and text cursor
  - **File(s)**: `oriterm_ui/src/widgets/text_input/widget_impl.rs`, plus 8 container widgets
  - **Root cause**: Same as BUG-01.8 — container widgets passed `interaction: None` to child DrawCtx, making `ctx.is_hot()` always return false. The hover border color check was correct in the code but could never trigger. CursorIcon::Text was already set in the layout.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Fixed by the interaction propagation fix (BUG-01.8). All controls now receive their interaction state during paint.

- [x] **BUG-01.8**: Dropdowns missing hover border highlight
  - **File(s)**: `oriterm_ui/src/widgets/setting_row/mod.rs`, plus 6 other container widgets
  - **Root cause**: Container widgets (setting_row, form_section, form_layout, form_row, settings_panel, panel, dialog/rendering, page_container) constructed child `DrawCtx` with `interaction: None, widget_id: None`. This made `ctx.is_hot()` always return `false` for all controls inside setting rows — dropdown, toggle, slider, text input, number input.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Propagated `ctx.interaction` and `Some(child.id())` through all 8 container widgets. All controls now correctly read their hover/focus/active state during paint.

- [x] **BUG-01.9**: Number input up/down buttons should show pointer cursor
  - **File(s)**: `oriterm_ui/src/widgets/number_input/mod.rs`
  - **Root cause**: Layout used default cursor icon. The widget is a single layout box, so cursor applies to the whole widget.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Added `.with_cursor_icon(CursorIcon::Pointer)` to layout. Updated test from `layout_cursor_icon_default` to `layout_cursor_icon_pointer`.

- [x] **BUG-01.10**: Color scheme cards too small with large gaps — not matching mockup
  - **File(s)**: `oriterm_ui/src/widgets/scheme_card/mod.rs`
  - **Root cause**: Card layout used `SizeSpec::Hug` (default from `LayoutBox::leaf`), which kept the card at its intrinsic 240px width. The grid solver computed wider column widths (e.g., 295px for 2 columns in 600px) but the card didn't expand to fill the cell.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4), screenshot comparison
  - **Fixed**: 2026-03-27 — Added `.with_width(SizeSpec::Fill)` to scheme card layout so cards expand to fill their grid cells, matching the mockup's `minmax(240px, 1fr)` behavior.

- [x] **BUG-01.11**: Color scheme cards missing hover effect
  - **File(s)**: `oriterm_ui/src/widgets/scheme_card/mod.rs`
  - **Root cause**: Paint code bypassed the VisualStateAnimator, reading `ctx.is_hot()` directly for instant bg changes. The animator was initialized with `common_states()` but its output was never used.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4), mockup highlights bg+border on hover
  - **Fixed**: 2026-03-27 — Non-selected state now reads `animator.get_bg_color()` for smooth hover transitions. Selected state still overrides directly (accent colors).

- [x] **BUG-01.12**: Palette section layout completely wrong — should be card with correct swatch shapes
  - **File(s)**: `oriterm/src/app/settings_overlay/form_builder/colors.rs`
  - **Root cause**: Palette content was a bare column with no visual containment. Mockup uses a `.color-editor` card with `bg_raised` background, `2px` border, `14px` padding. Also swatch gap was 8px (should be 6px) and palette gap was 12px (should be 10px).
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4), screenshot comparison
  - **Fixed**: 2026-03-27 — Wrapped palette content in `PanelWidget` with `bg_card`, 2px border, 14px padding, no corner radius or shadow. Fixed swatch gap to 6px and palette gap to 10px.

- [x] **BUG-01.13**: Toggle should auto-slide on click (not require drag)
  - **File(s)**: `oriterm_ui/src/widgets/toggle/mod.rs`
  - **Root cause**: Toggle only had ScrubController which required drag gesture. The DragEnd click fallback worked in tests but not in production because the drag-based path was unreliable for simple clicks.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Added ClickController alongside ScrubController. The on_action handler now maps Clicked → toggle() directly, providing a reliable click-to-toggle path with animation.
