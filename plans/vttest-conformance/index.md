---
reroute: true
name: "VTTest"
full_name: "VTTest Conformance"
status: queued
order: 2
---

# VTTest Conformance Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Terminal Size Reporting
**File:** `section-01-terminal-size.md` | **Status:** Complete

```
terminal size, CSI 18t, text_area_size_chars, DA, DA1, DA2
device attributes, identify_terminal, DSR, device status report
PTY size, PtySize, cols, rows, 80x24, 97x33, 120x40
vttest_border_fills, border test, screen 01_01
status.rs, handler/status.rs, status_text_area_size_chars
```

---

### Section 02: Origin Mode & Scroll Regions
**File:** `section-02-origin-mode.md` | **Status:** Complete

```
origin mode, DECOM, DECSTBM, scroll region, goto_origin_aware
CUP, cursor position, VPA, vertical position absolute
screen 01_02, border origin mode, vttest_origin_mode_matches
handler/helpers.rs, handler/modes.rs, grid/scroll/mod.rs
scroll_region, region_start, region_end, set_scrolling_region
```

---

### Section 03: Screen Features & DECCOLM
**File:** `section-03-screen-features.md` | **Status:** Complete

```
DECCOLM, 132 column mode, column mode, ColumnMode
DECAWM, auto wrap, line wrap, wrap around
tab stops, HTS, TBC, tab setting, tab reset
screen features, menu 2, wrap test, scroll test
soft scroll, jump scroll, smooth scroll
reverse video, light background, dark background
SGR, graphic rendition, bold, underline, blink, inverse
```

---

### Section 04: Character Sets & VT102
**File:** `section-04-charsets-vt102.md` | **Status:** Complete

```
character sets, G0, G1, SCS, designate character set
line drawing, DEC special graphics, box drawing
VT102, ICH, DCH, IL, DL, insert char, delete char
insert line, delete line, insert mode, IRM
menu 3, menu 8, character set test, VT102 test
```

---

### Section 05: Fade Blink
**File:** `section-05-fade-blink.md` | **Status:** Complete

```
cursor blink, fade blink, smooth blink, ColorEase
easing, cubic bezier, ease in, ease out, animation
CursorBlink, cursor_blink_visible, cursor_opacity, blink interval
instance alpha, push_cursor, build_cursor, alpha parameter
wezterm colorease, animation_fps, blink_rate
multi-frame capture, frame sequence, opacity ramp
```

---

### Section 05B: Text Blink Rendering (SGR 5/6)
**File:** `section-05b-text-blink.md` | **Status:** Not Started

```
text blink, SGR 5, SGR 6, slow blink, rapid blink
CellFlags::BLINK, blinking text, text_blink_opacity
fg_dim, push_glyph, glyph alpha, cell opacity
text_blink_rate_ms, text_blink_fade, FrameInput
fill_frame_shaped, GlyphEmitter, vttest menu 2 screen 13-14
```

---

### Section 06: Test Automation Expansion
**File:** `section-06-test-expansion.md` | **Status:** Not Started

```
vttest automation, menu 5 keyboard, menu 6 terminal reports
menu 7 VT52, menu 8 VT102, structural assertions
golden images, visual regression, insta snapshots
VtTestSession, PtyResponder, navigate menu
assert_border_fills, grid_chars, grid_to_text
compare_with_reference, headless_env, render_to_pixels
```

---

### Section 07: Verification & Metrics
**File:** `section-07-verification.md` | **Status:** Not Started

```
conformance audit, pass rate, 90% target
test matrix, coverage gaps, regression check
build-all, clippy-all, test-all, architecture tests
vttest pass rate, menu-by-menu scoring
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Terminal Size Reporting | `section-01-terminal-size.md` |
| 02 | Origin Mode & Scroll Regions | `section-02-origin-mode.md` |
| 03 | Screen Features & DECCOLM | `section-03-screen-features.md` |
| 04 | Character Sets & VT102 | `section-04-charsets-vt102.md` |
| 05 | Fade Blink | `section-05-fade-blink.md` |
| 05B | Text Blink Rendering (SGR 5/6) | `section-05b-text-blink.md` |
| 06 | Test Automation Expansion | `section-06-test-expansion.md` |
| 07 | Verification & Metrics | `section-07-verification.md` |
