---
reroute: true
name: "Font Quality"
full_name: "Font Rendering Quality"
status: active
order: 2
---

# Font Rendering Quality Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Bug Fixes
**File:** `section-01-bug-fixes.md` | **Status:** Complete

```
TPR-04-006, TPR-04-008, TPR-04-010, BUG-04-002
DPI change, scale factor, set_hinting_and_format, UI font, GlyphFormat::Alpha
atlas gutter, stale texels, upload_glyph(), GLYPH_PADDING, texture zero, padding strips
grid Y, row Y, per-row rounding, (oy + row * ch).round(), base_y, scene_convert
prepare/mod.rs, prepare/emit.rs, prepare/unshaped.rs, prepare/dirty_skip/mod.rs, fill_frame_incremental
blurry, blur, softening, bilinear, interpolation, fractional
draw_prompt_markers, build_cursor, draw_url_hover_underline
```

---

### Section 02: Advanced Font Rendering Settings
**File:** `section-02-advanced-settings.md` | **Status:** Not Started

```
hinting, HintingMode, Full, None, auto-detect, from_scale_factor
subpixel, SubpixelMode, Rgb, Bgr, GlyphFormat, subpixel_mode, LCD
subpixel_positioning, Option<bool>, subpx_bin, quarter-pixel, snapped, dead config, TPR-04-007
atlas_filtering, AtlasFiltering, FilterMode, Linear, Nearest, sampler, AtlasBindGroup, TPR-04-011, bind_groups/mod.rs, to_filter_mode
settings, dialog, font page, Advanced section, dropdown, SettingsIds
form_builder/font.rs, action_handler, config reload, resolve_hinting
config_reload/mod.rs, apply_font_changes, font_changed detection
build_settings_dialog, scale_factor, opacity, threading
rendering page, subpixel_toggle, remove, migrate
Auto default, auto-detection, scale factor, opacity
GlyphEmitter, helpers.rs, scene_convert/text.rs, grid_raster_keys
TextContext, scene_convert/mod.rs, scene_append.rs, scene_raster_keys
```

---

### Section 03: Verification
**File:** `section-03-verification.md` | **Status:** Not Started

```
test matrix, build, clippy, test-all, build-all, clippy-all
bug tracker, TPR resolution, BUG-04-002, section-04-fonts
visual regression, DPI change, fractional scale
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Bug Fixes | `section-01-bug-fixes.md` |
| 02 | Advanced Font Rendering Settings | `section-02-advanced-settings.md` |
| 03 | Verification | `section-03-verification.md` |
