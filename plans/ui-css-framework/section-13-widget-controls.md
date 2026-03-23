---
section: "13"
title: "Visual Fidelity: Widget Controls"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "Slider, toggle, and dropdown widget controls match the mockup's exact dimensions, colors, border widths, and spacing"
depends_on: ["01", "02"]
sections:
  - id: "13.1"
    title: "Slider"
    status: not-started
  - id: "13.2"
    title: "Toggle"
    status: not-started
  - id: "13.3"
    title: "Dropdown"
    status: not-started
  - id: "13.4"
    title: "Row Spacing Consistency"
    status: not-started
  - id: "13.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "13.5"
    title: "Build & Verify"
    status: not-started
---

# Section 13: Visual Fidelity — Widget Controls

**Status:** Not Started
**Goal:** Every interactive control in the settings panel — sliders, toggles, dropdowns — matches the mockup's CSS dimensions, colors, and interaction states. Row spacing between settings produces a consistent visual rhythm.

**Production code paths:**
- Slider: `oriterm_ui/src/widgets/slider/mod.rs` (`SliderStyle`, `SliderWidget`)
- Slider paint: `oriterm_ui/src/widgets/slider/widget_impl.rs`
- Toggle: `oriterm_ui/src/widgets/toggle/mod.rs` (`ToggleStyle`, `ToggleWidget`)
- Dropdown: `oriterm_ui/src/widgets/dropdown/mod.rs` (`DropdownStyle`, `DropdownWidget`)
- Row gap: `oriterm/src/app/settings_overlay/form_builder/appearance.rs` (`ROW_GAP`)

**Observable change:** Controls are visually identical to the mockup — slider track is 120px wide and 4px tall with gray color and blue thumb, toggle is 38x20 with square corners, dropdown has 140px min-width with correct padding, and setting rows have consistent vertical rhythm.

---

## 13.1 Slider

**File(s):** `oriterm_ui/src/widgets/slider/mod.rs` (`SliderStyle::from_theme()`)

### Mockup CSS

```css
input[type="range"] {
    width: 120px;                          /* fixed track width */
    height: 4px;                           /* thin track */
    background: var(--border);             /* #2a2a36 — gray, not blue */
    border: none;
    outline: none;
    appearance: none;
}

input[type="range"]::-webkit-slider-thumb {
    width: 12px;
    height: 14px;                          /* rectangular, taller than wide */
    background: var(--accent);             /* #6d9be0 — blue thumb */
    border: 2px solid var(--bg-surface);   /* #16161c — dark border */
    cursor: pointer;
    appearance: none;
}

/* Value label to the right */
.slider-value {
    font-size: 12px;
    color: var(--text-muted);              /* #9494a8 */
    width: 48px;
    text-align: right;
}
```

### Current code (`SliderStyle::from_theme()`)

```rust
Self {
    width: 120.0,                          // matches 120px
    track_height: 4.0,                     // matches 4px
    track_bg: theme.border,                // #2a2a36 — matches --border
    fill_color: theme.border,              // #2a2a36 — matches (gray fill, not accent)
    track_radius: theme.corner_radius,     // 0.0 — matches (brutal, no radius)
    thumb_width: 12.0,                     // matches 12px
    thumb_height: 14.0,                    // matches 14px
    thumb_color: theme.accent,             // #6d9be0 — matches --accent
    thumb_hover_color: theme.accent_hover, // #85ade8 — matches --accent-hover
    thumb_border_color: theme.bg_primary,  // #16161c — matches --bg-surface
    thumb_border_width: 2.0,               // matches 2px
    disabled_bg: theme.bg_secondary,
    disabled_fill: theme.fg_disabled,
    focus_ring_color: theme.accent,
    value_font_size: 12.0,                 // matches 12px
}
```

### Detailed comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Track width | 120px | 120.0 | Yes |
| Track height | 4px | 4.0 | Yes |
| Track bg | `--border` (#2a2a36) | `theme.border` (#2a2a36) | Yes |
| Track fill color | `--border` (same as track) | `theme.border` | Yes |
| Track radius | 0 (brutal) | `theme.corner_radius` (0.0) | Yes |
| Thumb width | 12px | 12.0 | Yes |
| Thumb height | 14px | 14.0 | Yes |
| Thumb bg | `--accent` (#6d9be0) | `theme.accent` | Yes |
| Thumb border | 2px `--bg-surface` | 2.0, `theme.bg_primary` (#16161c) | Yes |
| Value font size | 12px | 12.0 | Yes |
| Value color | `--text-muted` (#9494a8) | Drawn with `theme.fg_secondary` | Verify |
| Value width | 48px | `VALUE_LABEL_WIDTH = 48.0` | Yes |
| Value alignment | right | Positioned at right edge | Verify |

### Issues to verify

1. **Value label color**: The constant `VALUE_LABEL_WIDTH = 48.0` matches the mockup's `width: 48px`. The value label is drawn in the `widget_impl.rs` file. Check that it uses `theme.fg_secondary` (#9494a8) matching `--text-muted`.

2. **Value label alignment**: Mockup uses `text-align: right`. The paint code should right-align the value text within the 48px label area. Verify in `widget_impl.rs`.

3. **Track width behavior**: The slider's `width: 120.0` is the total widget width (track + thumb overhang). The actual track rendering area should be `120.0 - thumb_width` to prevent the thumb from extending beyond the widget bounds. Verify the track bounds calculation in `track_bounds()`:
   ```rust
   fn track_bounds(&self, bounds: Rect) -> Rect {
       let label_space = VALUE_LABEL_WIDTH + VALUE_GAP;
       let w = (bounds.width() - label_space).max(self.style.thumb_width);
       Rect::new(bounds.x(), bounds.y(), w, bounds.height())
   }
   ```
   The layout returns width `120.0 + VALUE_GAP + VALUE_LABEL_WIDTH = 120 + 12 + 48 = 180px` total. The track area is `180 - 60 = 120px`. This is correct.

4. **Fill portion**: The mockup shows the track as uniform gray (`--border`), with no colored fill to the left of the thumb. Current code sets `fill_color: theme.border` (same as track_bg), which means the fill is invisible. This is correct — the fill blends with the track.

### Checklist

- [ ] All `SliderStyle` fields match mockup (verified: all match).
- [ ] Value label rendered in `--text-muted` color (verify in `widget_impl.rs`).
- [ ] Value label right-aligned within 48px area (verify in `widget_impl.rs`).
- [ ] No code changes needed — verification-only (unless issues found in `widget_impl.rs`).

---

## 13.2 Toggle

**File(s):** `oriterm_ui/src/widgets/toggle/mod.rs` (`ToggleStyle::from_theme()`)

### Mockup CSS

```css
.toggle {
    width: 38px;
    height: 20px;
    border: 2px solid var(--border);       /* #2a2a36 */
    background: var(--bg-active);          /* #2a2a36 */
    position: relative;
    cursor: pointer;
    transition: all 0.15s ease;
}
.toggle .thumb {
    width: 12px;                           /* = height - 2*padding - 2*border */
    height: 12px;                          /* where padding=3, border=2: 20-6-4=10? */
    background: var(--text-faint);         /* #8c8ca0 */
    position: absolute;
    top: 3px;                              /* thumb_padding */
    left: 3px;                             /* thumb_padding */
    transition: transform 0.15s ease;
}
.toggle.checked {
    border-color: var(--accent);           /* #6d9be0 */
    background: var(--accent-bg-strong);   /* rgba(109,155,224,0.14) */
}
.toggle.checked .thumb {
    background: var(--accent);             /* #6d9be0 */
    transform: translateX(18px);           /* width - 2*padding - thumb_size = 38-6-12 = 20? */
}
```

### Thumb size math

Track dimensions: 38 x 20, border: 2px, thumb_padding: 3px.

Inner area (after border): `38 - 4 = 34` wide, `20 - 4 = 16` tall.
Thumb area (after padding): `34 - 6 = 28` travel width, `16 - 6 = 10` height.

Wait, that gives thumb height = 10, but the mockup says 12px. Let's recalculate:

If `thumb = height - 2 * thumb_padding - 2 * border`:
`20 - 2*3 - 2*2 = 20 - 6 - 4 = 10px`. But mockup says 12x12.

Alternative: thumb_padding might be measured from the border edge, not the outer edge.
`thumb = height - 2 * border - 2 * thumb_padding = 20 - 4 - 2*2 = 12`. With padding=2.
Or: `thumb = height - 2 * thumb_padding = 20 - 2*4 = 12`. With padding=4.

Let's check the current code:
```rust
width: 38.0,
height: 20.0,
thumb_padding: 3.0,
border_width: 2.0,
```

And in `paint()`:
```rust
let thumb_size = s.height - s.thumb_padding * 2.0;  // 20 - 6 = 14
```

So current thumb_size = 14. But the mockup says 12x12.

The `paint()` code calculates `thumb_size = height - 2 * thumb_padding`. With `thumb_padding = 3.0`, this gives `20 - 6 = 14`. The thumb is 14x14, but the mockup says 12x12.

### Current code (`ToggleStyle::from_theme()`)

```rust
Self {
    width: 38.0,                           // matches 38px
    height: 20.0,                          // matches 20px
    off_bg: theme.bg_active,               // #2a2a36 — matches --bg-active
    off_hover_bg: theme.bg_hover,          // #24242e — matches --bg-hover
    on_bg: theme.accent_bg_strong,         // rgba(0.14) — matches --accent-bg-strong
    off_thumb_color: theme.fg_faint,       // #8c8ca0 — matches --text-faint
    on_thumb_color: theme.accent,          // #6d9be0 — matches --accent
    thumb_padding: 3.0,                    // matches top: 3px
    border_width: 2.0,                     // matches 2px
    off_border_color: theme.border,        // #2a2a36 — matches --border
    on_border_color: theme.accent,         // #6d9be0 — matches --accent
    disabled_bg: theme.bg_secondary,
    disabled_thumb: theme.fg_disabled,
    focus_ring_color: theme.accent,
}
```

### Detailed comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Track width | 38px | 38.0 | Yes |
| Track height | 20px | 20.0 | Yes |
| Off bg | `--bg-active` (#2a2a36) | `theme.bg_active` | Yes |
| Off border | 2px `--border` | 2.0, `theme.border` | Yes |
| On bg | `--accent-bg-strong` (0.14) | `theme.accent_bg_strong` | Yes |
| On border | 2px `--accent` | 2.0, `theme.accent` | Yes |
| Off thumb color | `--text-faint` (#8c8ca0) | `theme.fg_faint` | Yes |
| On thumb color | `--accent` (#6d9be0) | `theme.accent` | Yes |
| Thumb padding | 3px | 3.0 | Yes |
| Thumb size | 12x12 | 14x14 (computed) | **NO** |
| Translate X (checked) | 18px | computed from travel | Verify |

### Issues to fix

1. **Thumb size mismatch**: Current code computes `thumb_size = height - 2 * thumb_padding = 20 - 6 = 14`. But the mockup specifies 12x12 thumbs. The thumb_padding should account for the border:

   Correct calculation: `thumb_size = height - 2 * border_width - 2 * thumb_padding = 20 - 4 - 6 = 10`. That's even smaller.

   Alternatively, the mockup's `top: 3px; left: 3px` positions the thumb 3px from the border edge (inside the border). So the thumb occupies `height - 2 * (border + padding) = 20 - 2*(2+3) = 20 - 10 = 10`. That gives 10, not 12.

   But the mockup explicitly says `width: 12px; height: 12px`. This means the thumb_padding is not 3px — it's computed differently. With thumb=12 and height=20: `padding = (20 - 12) / 2 = 4`. But `top: 3px` says the padding from the outer edge is 3px. With 2px border, the inner padding is `3 - 2 = 1px`. Then `thumb = 20 - 2*2 (border) - 2*1 (inner pad) = 14`. Still wrong.

   The CSS `top: 3px` is from the border-box edge. The thumb has `position: absolute; top: 3px; left: 3px`. This means 3px from the outer edge of the toggle (including border). So the thumb's top is at 3px, its bottom is at 3+12 = 15px, and the track bottom is at 20px. That leaves 5px below (2px border + 3px padding). This is asymmetric... unless the border is NOT included in the box model calculation (CSS `box-sizing: content-box` vs `border-box`).

   With `box-sizing: border-box` (the mockup uses `*, *::before, *::after { box-sizing: border-box }`), the toggle's total size including borders is 38x20. Inner area: 34x16. `top: 3px` from the outer edge means 3 - 2 (border) = 1px from inner edge. Thumb size 12. Bottom: 16 - 1 - 12 = 3px (matches top). So inner padding is 1px each side, border is 2px each side.

   **In our paint code**: `thumb_size = height - thumb_padding * 2.0 = 20 - 6 = 14`. We need it to be 12.

   The thumb_padding in our code represents the distance from the outer edge to the thumb edge. With `thumb_padding = 3.0` and `border_width = 2.0`, the thumb is positioned starting at `bounds.y() + 3.0`, which is 3px from the outer edge (1px from the inner/border edge). The thumb height should be `height - 2 * thumb_padding = 20 - 6 = 14` if thumb_padding means "from outer edge." But the mockup says 12.

   **Root cause**: The current `thumb_size` formula does not subtract the border width. The thumb lives inside the border, so:
   ```
   thumb_size = height - 2 * border_width - 2 * inner_padding
   ```
   where `inner_padding = thumb_padding - border_width = 3 - 2 = 1`.

   So: `thumb_size = 20 - 4 - 2 = 14`. Still 14!

   Hmm. Let me re-examine. If `top: 3px` with `border-box` means 3px from the content edge (inside the border), then: `thumb_size = height - 2*border - 2*top = 20 - 4 - 6 = 10`. No.

   Actually, in CSS with `position: absolute` inside a `position: relative` container with `border-box`, `top: 3px` means 3px from the padding edge (inside border). So the thumb starts at 3px from the inner edge. Thumb = 12px. Bottom space = 16 - 3 - 12 = 1px. That's asymmetric.

   But `left: 3px` and `translateX(18px)` when checked: starting at 3, moves 18, so thumb-left = 21. Thumb-right = 21 + 12 = 33. Inner width = 34. Right space = 34 - 33 = 1px. So the spacing is: 3px left, 1px right when off; 21px left, 1px right when on. That does look asymmetric.

   **Pragmatic approach**: The mockup says 12x12. Change the paint code to use `thumb_size = height - 2 * (thumb_padding + border_width)`:
   ```rust
   let thumb_size = s.height - 2.0 * (s.thumb_padding + s.border_width);
   // 20 - 2*(3+2) = 20 - 10 = 10
   ```
   That gives 10, not 12.

   **Alternative**: Change `thumb_padding` to 4.0. Then `thumb_size = 20 - 8 = 12`. The thumb is at `bounds.y() + 4.0`, which is 4px from outer edge = 2px from inner edge. That gives 12x12 thumbs with 2px inner padding. And the thumb Y position: `bounds.y() + s.thumb_padding = bounds.y() + 4.0`.

   **Action**: Change `thumb_padding` from 3.0 to 4.0 in `ToggleStyle::from_theme()`. This produces:
   - `thumb_size = 20 - 8 = 12` -- matches mockup.
   - Thumb position: 4px from outer edge = 2px from inner edge.
   - Travel: `38 - 8 - 12 = 18px` -- matches mockup's `translateX(18px)`.

   **NOTE:** The test helper `test_toggle_style()` in `toggle/tests.rs` already uses `thumb_padding: 4.0`, so test assertions already expect the correct value. Only the production `from_theme()` needs updating.

2. **TranslateX verification**: With `thumb_padding = 4.0`:
   - Travel = `width - 2 * thumb_padding - thumb_size = 38 - 8 - 12 = 18`.
   - Mockup: `translateX(18px)`. Matches.

### Checklist

- [ ] `thumb_padding` changed from 3.0 to 4.0 in `ToggleStyle::from_theme()`.
- [ ] Thumb size renders as 12x12 (verify: `height - 2 * thumb_padding = 20 - 8 = 12`).
- [ ] Travel distance is 18px (verify: `width - 2 * thumb_padding - thumb_size = 38 - 8 - 12 = 18`).
- [ ] All toggle colors match mockup (verified: all match).
- [ ] Drag discrimination threshold updated if it depends on travel.
- [ ] `cargo test -p oriterm_ui` toggle tests pass.

---

## 13.3 Dropdown

**File(s):** `oriterm_ui/src/widgets/dropdown/mod.rs` (`DropdownStyle::from_theme()`)

### Mockup CSS

```css
.dropdown-select {
    min-width: 140px;
    padding: 6px 30px 6px 10px;            /* top right bottom left */
    border: 2px solid var(--border);       /* #2a2a36 */
    background: var(--bg-input);           /* #12121a */
    color: var(--text);                    /* #d4d4dc */
    font-size: 12px;
    cursor: pointer;
    appearance: none;
    transition: border-color 0.15s;
}
.dropdown-select:hover {
    border-color: var(--text-faint);       /* #8c8ca0 */
}
.dropdown-select:focus {
    border-color: var(--accent);           /* #6d9be0 */
    outline: none;
}
/* Dropdown arrow indicator — CSS background SVG chevron */
.dropdown-select {
    background-image: url("data:image/svg+xml,...chevron...");
    background-repeat: no-repeat;
    background-position: right 10px center;
    background-size: 10px;
}
```

### Current code (`DropdownStyle::from_theme()`)

```rust
Self {
    fg: theme.fg_primary,                  // #d4d4dc — matches --text
    bg: theme.bg_input,                    // #12121a — matches --bg-input
    hover_bg: theme.bg_input,              // #12121a — bg doesn't change on hover (correct)
    pressed_bg: theme.bg_input,            // #12121a — bg doesn't change on press (correct)
    border_color: theme.border,            // #2a2a36 — matches --border
    hover_border_color: theme.fg_faint,    // #8c8ca0 — matches --text-faint
    focus_border_color: theme.accent,      // #6d9be0 — matches --accent
    border_width: 2.0,                     // matches 2px
    corner_radius: theme.corner_radius,    // 0.0 — matches (brutal)
    padding: Insets::tlbr(6.0, 10.0, 6.0, 30.0),  // MISMATCH — see below
    font_size: 12.0,                       // matches 12px
    min_width: 140.0,                      // matches 140px
    indicator_width: 20.0,                 // space for arrow
    indicator_color: theme.fg_faint,       // #8c8ca0 — matches
    disabled_fg: theme.fg_disabled,
    disabled_bg: theme.bg_secondary,
    focus_ring_color: theme.accent,
}
```

### Detailed comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Min width | 140px | 140.0 | Yes |
| Padding | 6px 30px 6px 10px (T R B L) | `Insets::tlbr(6.0, 10.0, 6.0, 30.0)` | **Reversed** |
| Border | 2px `--border` | 2.0, `theme.border` | Yes |
| Bg | `--bg-input` (#12121a) | `theme.bg_input` | Yes |
| Text color | `--text` (#d4d4dc) | `theme.fg_primary` | Yes |
| Font size | 12px | 12.0 | Yes |
| Hover border | `--text-faint` | `theme.fg_faint` | Yes |
| Focus border | `--accent` | `theme.accent` | Yes |
| Corner radius | 0 (brutal) | 0.0 | Yes |
| Indicator | SVG chevron | Unicode `\u{25BE}` (filled triangle) | Approximate |

### Issues to fix

1. **Padding order**: Verified correct. `Insets::tlbr(top, left, bottom, right)` with `(6.0, 10.0, 6.0, 30.0)` gives `top=6, left=10, bottom=6, right=30`. CSS `padding: 6px 30px 6px 10px` means `top=6, right=30, bottom=6, left=10`. Same values, different ordering convention. **No change needed.**

2. **Indicator rendering**: The mockup uses an SVG chevron as a background image. Current code uses Unicode `\u{25BE}` (filled down-pointing triangle character). This is visually similar but not identical to an SVG chevron. Consider:
   - If the icon system (Section 08) provides a chevron icon, use it instead.
   - Otherwise, the Unicode triangle is an acceptable approximation.

3. **Indicator position**: Mockup: `background-position: right 10px center`. The chevron is 10px from the right edge, vertically centered. Current code: `bounds.right() - 10.0 - shaped.width / 2.0`. This centers the triangle glyph at 10px from right. Should be fine.

### Checklist

- [ ] Padding order verified: `tlbr(6.0, 10.0, 6.0, 30.0)` matches mockup `6px 30px 6px 10px` (CSS TRBL -> our TLBR).
- [ ] All colors match mockup.
- [ ] Indicator position 10px from right edge, vertically centered.
- [ ] Min-width 140px confirmed.
- [ ] Consider replacing Unicode triangle with SVG chevron icon (after Section 08).
- [ ] No code changes needed — verification-only (unless padding order issue confirmed).

---

## 13.4 Row Spacing Consistency

**File(s):** `oriterm/src/app/settings_overlay/form_builder/appearance.rs` (`ROW_GAP`)

### Mockup CSS

```css
.settings-section .setting-row + .setting-row {
    /* No gap — rows are flush */
    /* The only spacing comes from each row's internal padding */
}
```

### Current code

```rust
pub(super) const ROW_GAP: f32 = 2.0;
```

### Comparison

The mockup has setting rows flush against each other with no gap between them. The only vertical spacing is each row's own 10px top/bottom padding (from `ROW_PADDING`). The current code has `ROW_GAP = 2.0`, adding 2px between rows.

### Issue

The 2px gap creates a visible seam between rows where the background shows through. When hovering row N, the row above and below show a 2px gap between the hover highlights. The mockup has no such gap — rows are flush, and hover backgrounds tile seamlessly.

### Fix

Change `ROW_GAP` from 2.0 to 0.0:
```rust
pub(super) const ROW_GAP: f32 = 0.0;
```

However, note that `ROW_GAP` is also used as the gap in the section container (between section title and first row). The section title uses `TITLE_ROW_GAP = 8.0` for its own gap. Verify that changing `ROW_GAP` to 0.0 does not affect the gap between the section title and first row.

Looking at `build_window_section()`:
```rust
ContainerWidget::column()
    .with_gap(ROW_GAP)                    // This is between ALL children: title + rows
    .with_child(title)
    .with_child(Box::new(opacity_row))
    .with_child(Box::new(blur_row))
```

Wait — this uses `ROW_GAP` for the gap between the title and the first row too! But the title-to-row gap should be `TITLE_ROW_GAP = 8.0`, not `ROW_GAP = 2.0`. This looks like a bug: the title uses a separate container in `build_theme_section()` with `TITLE_ROW_GAP`, but `build_window_section()` puts the title and rows in the same container with `ROW_GAP`.

**Fix**: Restructure sections so the title and row container are separate:
```rust
let title = section_title("Window", theme);
let rows = ContainerWidget::column()
    .with_width(SizeSpec::Fill)
    .with_gap(0.0)                         // rows are flush
    .with_child(Box::new(opacity_row))
    .with_child(Box::new(blur_row));

ContainerWidget::column()
    .with_width(SizeSpec::Fill)
    .with_gap(TITLE_ROW_GAP)               // 8px between title and rows
    .with_child(title)
    .with_child(Box::new(rows))
```

Or keep the flat structure but use `ROW_GAP = 0.0` and add a spacer after the title:
```rust
.with_child(title)
.with_child(Box::new(SpacerWidget::fixed(0.0, TITLE_ROW_GAP)))
.with_child(Box::new(opacity_row))
.with_child(Box::new(blur_row))
```

### Checklist

- [ ] `ROW_GAP` changed from 2.0 to 0.0.
- [ ] Section builders restructured so title-to-row gap is `TITLE_ROW_GAP` (8px) and row-to-row gap is 0px.
- [ ] `build_theme_section()` already uses separate container — verify.
- [ ] `build_window_section()` restructured to separate title from row container.
- [ ] `build_decorations_section()` (from Section 09) follows same pattern.
- [ ] Hover backgrounds tile seamlessly with no visible gap.

---

## 13.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 13.5 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] Toggle thumb renders at 12x12 (not 14x14)
- [ ] Row-to-row gap is 0px (hover backgrounds tile seamlessly)
- [ ] Visual verification: all widget controls match mockup
