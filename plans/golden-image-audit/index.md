---
reroute: true
name: "Golden Audit"
full_name: "Golden Image Audit & Test Methodology Fix"
status: active
order: 1
---

# Golden Image Audit Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: DECSCNM Reverse Video Rendering
**File:** `section-01-decscnm.md` | **Status:** Not Started

```
DECSCNM, reverse video, mode 5, light background, dark background
PrivateMode, NamedPrivateMode, TermMode, REVERSE_VIDEO
CSI ? 5 h, CSI ? 5 l, set_mode, reset_mode
apply_decset, apply_decrst, modes.rs, types.rs
snapshot_palette, resolve_cell_colors, set_clear_color
FramePalette, RenderableContent, vttest 02_03, 02_04, 02_14
```

---

### Section 02: VT102 Insert/Delete Line with Scroll Regions
**File:** `section-02-vt102-scroll.md` | **Status:** Not Started

```
insert_lines, delete_lines, IL, DL, CSI L, CSI M
scroll region, DECSTBM, scroll_range_down, scroll_range_up
rotate_right, rotate_left, grid/scroll/mod.rs
vttest 08_vt102_08 through 08_vt102_12
assert_vt102_screen_structure, structural assertions
```

---

### Section 03: Text Blink Multi-Frame Verification
**File:** `section-03-text-blink-tests.md` | **Status:** Not Started

```
text blink, multi-frame capture, animation verification
text_blink_opacity, CursorBlink, intensity, next_change
text_blink_visible, text_blink_hidden, text_blink_half
frame sequence, opacity ramp, advance_time
render_to_pixels, FrameInput, CellFlags::BLINK
```

---

### Section 04: Golden Image Revalidation
**File:** `section-04-revalidation.md` | **Status:** Not Started

```
golden image, visual regression, reference PNG
compare_with_reference, ORITERM_UPDATE_GOLDEN
inverse_video, SGR 7, per-cell inverse
full audit, re-render, revalidation sweep
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | DECSCNM Reverse Video Rendering | `section-01-decscnm.md` |
| 02 | VT102 Insert/Delete Line with Scroll Regions | `section-02-vt102-scroll.md` |
| 03 | Text Blink Multi-Frame Verification | `section-03-text-blink-tests.md` |
| 04 | Golden Image Revalidation | `section-04-revalidation.md` |
