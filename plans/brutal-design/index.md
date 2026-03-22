---
reroute: true
name: "Brutal Design"
full_name: "Brutal Design System — Settings Panel Visual Overhaul"
status: active
order: 3
---

# Brutal Design System Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **References:** `mockups/settings-brutal.html`

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

### Section 01: Theme Token Overhaul
**File:** `section-01-theme-tokens.md` | **Status:** Not Started

```
UiTheme, theme_tokens, bg_base, bg_surface, bg_raised, bg_hover, bg_active
bg_input, border, border_subtle, border_strong, text, text_muted, text_faint
text_bright, accent, accent_hover, accent_bg, accent_bg_strong
danger, success, warning, radius, shadow, brutal, flat
0px, corner_radius, no_shadow, no_rounded, sharp_corners
oriterm_ui/src/theme/mod.rs
```

---

### Section 02: Widget Styling — Zero Radius & Flat
**File:** `section-02-widget-styling.md` | **Status:** Not Started

```
corner_radius, border_radius, 0px, flat, sharp, brutal
ButtonStyle, ToggleStyle, SliderStyle, DropdownStyle, SchemeCardStyle
ScrollbarStyle, MenuStyle, SettingRowStyle, SidebarNavStyle
RectStyle, with_radius, shadow, with_shadow, Shadow
oriterm_ui/src/widgets/, oriterm_ui/src/draw/
```

---

### Section 03: Sidebar Enhancements
**File:** `section-03-sidebar.md` | **Status:** Not Started

```
sidebar, SidebarNavWidget, sidebar_search, version_label, config_path
section_title, nav_item, active_indicator, border_left, accent_bg_strong
// prefix, slash_slash, uppercase, letter_spacing, GENERAL, ADVANCED
icon, IconId, Sun, Palette, Type, Terminal, Keyboard, Window, Bell, Activity
oriterm_ui/src/widgets/sidebar_nav/mod.rs
```

---

### Section 04: Content Area Styling
**File:** `section-04-content-area.md` | **Status:** Not Started

```
section_title, horizontal_rule, // prefix, uppercase, letter_spacing
page_header, title_bright, description_muted, content_header
setting_row, form_section, form_layout, section_desc
SettingRowWidget, FormSection, content_body, content_padding
oriterm_ui/src/widgets/setting_row/, oriterm_ui/src/widgets/
oriterm/src/app/settings_overlay/form_builder/
```

---

### Section 05: Footer & Button Styling
**File:** `section-05-footer-buttons.md` | **Status:** Not Started

```
footer, ghost_button, primary_button, accent_button, save, cancel, reset
button_style, border_width, bg_transparent, hover_bg, pressed_bg
separator, footer_separator, border_strong
SettingsPanel, IdOverrideButton, ButtonStyle, ButtonWidget
oriterm_ui/src/widgets/settings_panel/mod.rs, oriterm_ui/src/widgets/button/
```

---

### Section 06: Verification
**File:** `section-06-verification.md` | **Status:** Not Started

```
visual_regression, mockup_comparison, screenshot, DPI, scale_factor
dark_theme, light_theme, hover, transition, animation
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Theme Token Overhaul | `section-01-theme-tokens.md` |
| 02 | Widget Styling — Zero Radius & Flat | `section-02-widget-styling.md` |
| 03 | Sidebar Enhancements | `section-03-sidebar.md` |
| 04 | Content Area Styling | `section-04-content-area.md` |
| 05 | Footer & Button Styling | `section-05-footer-buttons.md` |
| 06 | Verification | `section-06-verification.md` |
