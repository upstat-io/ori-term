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
