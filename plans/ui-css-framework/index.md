---
reroute: true
name: "CSS UI Framework"
full_name: "CSS-Equivalent UI Framework Features"
status: active
order: 5
---

# CSS-Equivalent UI Framework Features Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **References:** `mockups/settings-brutal.html`, `plans/brutal-design-pass-2/`

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

### Section 01: Multi-Size Font Rendering
**File:** `section-01-multi-size-fonts.md` | **Status:** Complete

```
font-size, multi-size, TextStyle.size, size_q6, RasterKey
FontCollection, UiFontMeasurer, shape_text, TextContext
glyph atlas, ppem, cell_metrics, size_px, size_key
scene_convert/text.rs, ui_measurer.rs, ui_text.rs
collection/mod.rs, collection/face.rs, font/mod.rs
```

---

### Section 02: Numeric Font Weight System
**File:** `section-02-font-weight.md` | **Status:** In Progress (1 item blocked by §10-13)

```
font-weight, FontWeight, 100-900, Regular, Medium, Bold
GlyphStyle, face_idx, resolve, synthetic, SyntheticFlags
text/mod.rs, font/mod.rs, collection/resolve.rs, collection/metadata.rs
CSS 400, 500, 600, 700, weight selection, nearest-match
face_variations, wght axis, resolve_ui_weight, rasterize_with_weight
create_shaping_faces_for_weight, ShapedText.weight, RasterKey.weight
```

---

### Section 03: Text Transform + Letter Spacing
**File:** `section-03-text-transform.md` | **Status:** Complete

```
text-transform, uppercase, lowercase, capitalize, TextTransform
letter-spacing, letter_spacing, 0.05em, 0.15em, em-to-px
LabelStyle, TextStyle, LabelWidget, section_title
sidebar_nav, button, appearance.rs
```

---

### Section 04: Line Height Control
**File:** `section-04-line-height.md` | **Status:** Complete

```
line-height, line_height, ShapedText.height, baseline
1.3, 1.4, 1.5, 1.6, multiplier, leading
text/mod.rs, ui_measurer.rs, cell_metrics
layout height, text block sizing
```

---

### Section 05: Per-Side Borders
**File:** `section-05-per-side-borders.md` | **Status:** Complete

```
border-left, border-top, border-right, border-bottom
Border, BorderSides, RectStyle, border_width, UiRectWriter
border.rs, rect_style.rs, ui_rect.wgsl, ui_rect_writer
push_ui_rect, corner_radii, corner ownership, SDF
sidebar right border, footer top border, nav active left border
144-byte, dedicated UI rect instance, per-side colors
```

---

### Section 06: Opacity + Display Control
**File:** `section-06-opacity-display.md` | **Status:** Complete

```
opacity, display:none, visibility, hidden, disabled
0.4, 0.7, alpha modulation, pointer-events
Widget, DrawCtx, Scene, ContentMask
page switching, disabled controls, inactive icons
```

---

### Section 07: Scrollbar Styling
**File:** `section-07-scrollbar-styling.md` | **Status:** Complete

```
scrollbar, scrollbar-width, scrollbar-track, scrollbar-thumb
6px, thin, hover state, webkit-scrollbar
ScrollWidget, ScrollbarStyle, scroll/mod.rs
content-body, overflow-y:auto, transparent track
```

---

### Section 08: Icon Path Verification
**File:** `section-08-icon-verification.md` | **Status:** In Progress

```
IconId, IconPath, SVG, viewBox, stroke, fill
Sun, Palette, Type, Terminal, Keyboard, Window, Bell, Activity
sidebar nav icons, 16px, ICON_SIZES, icon_cache
icons/mod.rs, mockup SVG paths, data:image/svg+xml
```

---

### Section 09: Settings Content Completeness
**File:** `section-09-settings-content.md` | **Status:** Complete

```
Unfocused opacity, Window decorations, Tab bar style
Decorations section, appearance page, settings rows
SettingRowWidget, form_builder, appearance.rs
DropdownWidget, SliderWidget, setting completeness
```

---

### Section 10: Visual Fidelity — Sidebar + Navigation
**File:** `section-10-sidebar-fidelity.md` | **Status:** Not Started

```
sidebar, SidebarNavWidget, nav-item, active state
accent-bg-strong, border-left, 3px indicator
search field, sidebar-footer, version label, config path
sidebar height, full-height, padding, row height
```

---

### Section 11: Visual Fidelity — Content Area + Typography
**File:** `section-11-content-typography.md` | **Status:** Not Started

```
content-header, page title, APPEARANCE, section-title
section divider line, ::after pseudo-element
setting-row, setting-label, .name, .desc
font sizes, 18px title, 13px body, 11.5px desc, 11px header
```

---

### Section 12: Visual Fidelity — Footer + Buttons
**File:** `section-12-footer-buttons.md` | **Status:** Not Started

```
footer, sticky footer, border-top, UNSAVED CHANGES
btn-primary, btn-ghost, btn-danger-ghost
RESET TO DEFAULTS, CANCEL, SAVE
button padding, font-weight 500/700, uppercase, letter-spacing
```

---

### Section 13: Visual Fidelity — Widget Controls
**File:** `section-13-widget-controls.md` | **Status:** Not Started

```
SliderWidget, ToggleWidget, DropdownWidget
slider track width 120px, thumb 12x14, accent color
toggle 38x20, track border, thumb slide
dropdown min-width 140px, padding, arrow indicator
setting row spacing, control alignment, right-hand rail
```

---

### Section 14: Verification + Visual Regression
**File:** `section-14-verification.md` | **Status:** Not Started

```
screenshot comparison, side-by-side, pixel perfect
build gate, clippy, test-all, build-all
visual regression, DPI scaling, 100% DPI
mockup match, settings-brutal.html
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Multi-Size Font Rendering | `section-01-multi-size-fonts.md` |
| 02 | Numeric Font Weight System | `section-02-font-weight.md` |
| 03 | Text Transform + Letter Spacing | `section-03-text-transform.md` |
| 04 | Line Height Control | `section-04-line-height.md` |
| 05 | Per-Side Borders | `section-05-per-side-borders.md` |
| 06 | Opacity + Display Control | `section-06-opacity-display.md` |
| 07 | Scrollbar Styling | `section-07-scrollbar-styling.md` |
| 08 | Icon Path Verification | `section-08-icon-verification.md` |
| 09 | Settings Content Completeness | `section-09-settings-content.md` |
| 10 | Visual Fidelity: Sidebar + Nav | `section-10-sidebar-fidelity.md` |
| 11 | Visual Fidelity: Content + Typography | `section-11-content-typography.md` |
| 12 | Visual Fidelity: Footer + Buttons | `section-12-footer-buttons.md` |
| 13 | Visual Fidelity: Widget Controls | `section-13-widget-controls.md` |
| 14 | Verification + Visual Regression | `section-14-verification.md` |
