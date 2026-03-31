---
reroute: true
name: "Main Window Brutal"
full_name: "Main Window Brutal Design"
status: resolved
order: 1
---

# Main Window Brutal Design Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Tab Bar Brutal Styling
**File:** `section-01-tab-bar.md` | **Status:** Complete

```
tab bar, tab strip, tab height, flat tabs, brutal tabs
ACTIVE_TAB_RADIUS, TAB_BAR_HEIGHT, TAB_TOP_MARGIN, TAB_PADDING
accent bar, top bar, bottom bleed, border-bottom, modified dot
TabBarMetrics, TabBarColors, TabBarWidget, TabStrip
draw_tab, draw_all_tabs, draw_separators, tab_background_color
constants.rs, colors.rs, draw.rs, controls_draw.rs
golden test, tab_bar_brutal, visual_regression
```

---

### Section 02: Status Bar Widget
**File:** `section-02-status-bar.md` | **Status:** Complete

```
status bar, StatusBarWidget, StatusBarData, StatusBarColors
shell name, pane count, grid size, encoding, term type
22px height, bg-surface, border-top, text-faint, 11px font
oriterm_ui/src/widgets/status_bar/, Widget trait
WidgetTestHarness, golden test, status_bar_golden
from_theme, UiTheme, status_accent
```

---

### Section 03: Window Frame & Split Pane Styling
**File:** `section-03-frame-panes.md` | **Status:** Complete

```
window border, border-strong, 2px frame, window frame
split divider, divider hover, accent divider, col-resize
focus border, focused pane, pane outline, accent outline
DividerLayout, DividerDragState, append_dividers
append_focus_border, append_floating_decoration
divider_drag.rs, multi_pane/mod.rs, window_renderer
platform chrome, DWM, decorations, WS_EX_TOOLWINDOW
```

---

### Section 04: Production Wiring & Composed Tests
**File:** `section-04-wiring.md` | **Status:** Complete

```
production wiring, grid offset, TAB_BAR_HEIGHT, status bar height
WindowContext, WindowRoot, redraw pipeline, draw_frame
composed golden, main_window_chrome_golden
render_to_pixels, compare_with_reference
existing golden updates, tab_bar_emoji reference
```

---

### Section 05: Verification
**File:** `section-05-verification.md` | **Status:** Complete

```
verification, DPI scaling, 96dpi, 192dpi, cross-platform
build-all.sh, clippy-all.sh, test-all.sh
visual regression suite, golden comparison
PIXEL_TOLERANCE, MAX_MISMATCH_PERCENT
test matrix, edge cases
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Tab Bar Brutal Styling (Complete) | `section-01-tab-bar.md` |
| 02 | Status Bar Widget | `section-02-status-bar.md` |
| 03 | Window Frame & Split Pane Styling | `section-03-frame-panes.md` |
| 04 | Production Wiring & Composed Tests | `section-04-wiring.md` |
| 05 | Verification | `section-05-verification.md` |
