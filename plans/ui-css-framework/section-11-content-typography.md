---
section: "11"
title: "Visual Fidelity: Content Area + Typography"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "Page header, section titles, setting rows, and content spacing in the right-hand content area match the mockup's typography and layout exactly"
depends_on: ["01", "02", "03", "04"]
sections:
  - id: "11.1"
    title: "Page Header"
    status: not-started
  - id: "11.2"
    title: "Section Title with Divider"
    status: not-started
  - id: "11.3"
    title: "Setting Row Layout"
    status: not-started
  - id: "11.4"
    title: "Content Body Padding"
    status: not-started
  - id: "11.5"
    title: "Section Spacing"
    status: not-started
  - id: "11.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "11.6"
    title: "Build & Verify"
    status: not-started
---

# Section 11: Visual Fidelity — Content Area + Typography

**Status:** Not Started
**Goal:** The right-hand content area of the settings dialog — page headers, section titles with divider lines, setting rows with labels and controls, and all spacing/padding — matches the mockup CSS pixel-for-pixel at 100% DPI.

**Production code paths:**
- Form builder: `oriterm/src/app/settings_overlay/form_builder/appearance.rs` — `build_page_header()`, `section_title()`, page constants
- Setting row: `oriterm_ui/src/widgets/setting_row/mod.rs` — `SettingRowWidget`, `MIN_HEIGHT`, `ROW_PADDING`, `NAME_FONT_SIZE`, etc.
- Container layout: `oriterm_ui/src/widgets/container/mod.rs` — `ContainerWidget` padding and gap
- Text rendering: depends on Sections 01 (multi-size fonts), 02 (font weight), 03 (text transform + letter spacing), 04 (line height)

**Observable change:** Text in the content area renders at correct sizes with proper weight differentiation (bold titles vs regular body), uppercase transforms are applied, letter spacing is visible, and vertical rhythm matches the mockup.

---

## 11.1 Page Header

**File(s):** `oriterm/src/app/settings_overlay/form_builder/appearance.rs` (`build_page_header()`)

### Mockup CSS

```css
.content-header h1 {
    font-size: 18px;
    font-weight: 700;                     /* Bold */
    text-transform: uppercase;
    letter-spacing: 0.05em;               /* 18px * 0.05 = 0.9px */
    color: var(--text-bright);            /* #eeeeef */
    margin: 0;
}
.content-header .subtitle {
    font-size: 12px;
    color: var(--text-muted);             /* #9494a8 */
    margin-top: 4px;
}
.content-header {
    padding: 24px 28px 20px;
}
```

### Current code (`build_page_header()`)

```rust
const TITLE_FONT_SIZE: f32 = 18.0;              // matches 18px
const TITLE_LETTER_SPACING: f32 = 0.9;          // 18 * 0.05 = 0.9px — matches
const DESC_FONT_SIZE: f32 = 12.0;               // matches 12px

let title = LabelWidget::new(title_text).with_style(LabelStyle {
    font_size: TITLE_FONT_SIZE,                  // 18.0
    weight: FontWeight::Bold,                    // 700 — matches
    letter_spacing: TITLE_LETTER_SPACING,        // 0.9
    color: theme.fg_bright,                      // #eeeeef — matches
    ..LabelStyle::from_theme(theme)
});
let desc = LabelWidget::new(desc_text).with_style(LabelStyle {
    font_size: DESC_FONT_SIZE,                   // 12.0
    color: theme.fg_secondary,                   // #9494a8 — matches
    ..LabelStyle::from_theme(theme)
});

// Container padding
Insets::tlbr(24.0, 28.0, 20.0, 28.0)            // matches 24px 28px 20px
// Gap between title and subtitle
.with_gap(4.0)                                   // matches margin-top: 4px
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Title font size | 18px | 18.0 | Yes |
| Title font weight | 700 (Bold) | `FontWeight::Bold` | Yes (once Section 02 lands) |
| Title text-transform | uppercase | Passed in as `"APPEARANCE"` | Yes (manually uppercased) |
| Title letter-spacing | 0.05em = 0.9px | 0.9 | Yes (once Section 03 lands) |
| Title color | `--text-bright` (#eeeeef) | `theme.fg_bright` (#eeeeef) | Yes |
| Subtitle font size | 12px | 12.0 | Yes |
| Subtitle color | `--text-muted` (#9494a8) | `theme.fg_secondary` (#9494a8) | Yes |
| Padding top | 24px | 24.0 | Yes |
| Padding left/right | 28px | 28.0 | Yes |
| Padding bottom | 20px | 20.0 | Yes |
| Title-subtitle gap | 4px | 4.0 | Yes |

### Dependencies

- **Section 01 (Multi-Size Fonts)**: The 18px title must actually render larger than the 13px body text. Until Section 01 lands, all text renders at the grid cell font size regardless of `TextStyle.size`. Once landed, verify that the title is visually larger.
- **Section 02 (Font Weight)**: `FontWeight::Bold` (700) must produce visibly heavier glyphs. Until landed, bold text looks the same as regular.
- **Section 03 (Letter Spacing)**: The 0.9px letter spacing must produce visible character spreading. Until landed, `letter_spacing` is stored but not applied.
- **Section 03 (Text Transform)**: Consider replacing the manual `"APPEARANCE"` string with a `TextTransform::Uppercase` flag on `LabelStyle`. This makes the transform systematic rather than relying on callers to pass uppercase strings. However, the current approach is correct and works — this is a refinement, not a bug.

### Checklist

- [ ] All values match mockup (verified in code: all match).
- [ ] Title renders at 18px after Section 01 lands.
- [ ] Title renders bold after Section 02 lands.
- [ ] Letter spacing visible after Section 03 lands.
- [ ] No code changes needed — values already correct. Verification-only.

---

## 11.2 Section Title with Divider

**File(s):** `oriterm/src/app/settings_overlay/form_builder/appearance.rs` (`section_title()`)

### Mockup CSS

```css
.section-title {
    font-size: 11px;
    font-weight: 500;                     /* Medium weight */
    text-transform: uppercase;
    letter-spacing: 0.15em;               /* 11px * 0.15 = 1.65px */
    color: var(--text-faint);             /* #8c8ca0 */
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
}
.section-title::before {
    content: '//';                        /* prefix */
}
.section-title::after {
    content: '';
    flex: 1;
    height: 2px;
    background: var(--border);            /* #2a2a36 — divider line */
}
```

### Current code (`section_title()`)

```rust
const SECTION_FONT_SIZE: f32 = 11.0;            // matches 11px
pub(super) const SECTION_LETTER_SPACING: f32 = 1.6;  // 11 * 0.15 = 1.65px ≈ 1.6

let label = LabelWidget::new(format!("// {}", text.to_uppercase())).with_style(LabelStyle {
    font_size: SECTION_FONT_SIZE,                // 11.0
    letter_spacing: SECTION_LETTER_SPACING,      // 1.6
    color: theme.fg_faint,                       // #8c8ca0 — matches
    ..LabelStyle::from_theme(theme)
});
let rule = SeparatorWidget::horizontal().with_style(SeparatorStyle {
    thickness: 2.0,                              // matches 2px
    color: theme.border,                         // #2a2a36 — matches
    ..SeparatorStyle::from_theme(theme)
});
ContainerWidget::row()
    .with_width(SizeSpec::Fill)
    .with_align(Align::Center)                   // center-aligned vertically — matches
    .with_gap(10.0)                              // matches gap: 10px
    .with_child(Box::new(label))
    .with_child(Box::new(rule))
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Font size | 11px | 11.0 | Yes |
| Font weight | 500 (Medium) | Not set (defaults to Regular/400) | No |
| Uppercase | `text-transform: uppercase` | `.to_uppercase()` | Yes |
| Letter spacing | 0.15em = 1.65px | 1.6 | Close enough |
| Color | `--text-faint` (#8c8ca0) | `theme.fg_faint` | Yes |
| `//` prefix | `::before { content: '//' }` | `format!("// {}", ...)` | Yes |
| Gap | 10px | 10.0 | Yes |
| Divider height | 2px | 2.0 | Yes |
| Divider color | `--border` (#2a2a36) | `theme.border` | Yes |
| Divider extends to full width | `flex: 1` | `SizeSpec::Fill` on separator | Yes (if separator widget supports Fill) |
| Margin-bottom | 8px | `TITLE_ROW_GAP = 8.0` | Yes |

### Issues to fix

1. **Font weight**: Mockup uses `font-weight: 500` (Medium). Current code does not set `weight` in the `LabelStyle`, so it defaults to `FontWeight::Regular` (400). Once Section 02 (Font Weight) lands, add `weight: FontWeight::MEDIUM` to the `LabelStyle`:

   ```rust
   let label = LabelWidget::new(...).with_style(LabelStyle {
       font_size: SECTION_FONT_SIZE,
       weight: FontWeight::MEDIUM,              // <-- add this
       letter_spacing: SECTION_LETTER_SPACING,
       color: theme.fg_faint,
       ..LabelStyle::from_theme(theme)
   });
   ```

   Note: `FontWeight::MEDIUM` (500) may not exist yet. Section 02 defines the numeric weight system. If only `Regular` (400) and `Bold` (700) are available initially, 500 will map to the nearest available face — likely Regular. The visual difference between 400 and 500 is subtle but mockup-specified.

2. **Letter spacing precision**: `SECTION_LETTER_SPACING` is 1.6 but exact value is 1.65 (11 * 0.15). This 0.05px difference is sub-pixel and visually imperceptible. No change needed.

3. **Separator Fill behavior**: Verify that `SeparatorWidget` with the default layout actually fills the remaining width when placed in a row. If the separator's `layout()` returns a fixed width, the divider line will not extend to the right edge. It must return `SizeSpec::Fill` for its primary axis. Check `oriterm_ui/src/widgets/separator/mod.rs`.

### Checklist

- [ ] Font weight changed to Medium (500) once Section 02 provides it.
- [ ] Separator extends to full remaining width (verify layout behavior).
- [ ] All other values match (font size, letter spacing, color, gap, divider thickness).
- [ ] Visual verification after Sections 01-03 land.

---

## 11.3 Setting Row Layout

**File(s):** `oriterm_ui/src/widgets/setting_row/mod.rs`

### Mockup CSS

```css
.setting-row {
    display: flex;
    align-items: center;
    padding: 10px 14px;
    min-height: 44px;
    gap: 24px;
}
.setting-row:hover {
    background: var(--bg-raised);          /* #1c1c24 */
}
.setting-label .name {
    font-size: 13px;
    color: var(--text);                    /* #d4d4dc */
}
.setting-label .desc {
    font-size: 11.5px;
    color: var(--text-muted);              /* #9494a8 */
    margin-top: 2px;
}
.setting-control {
    flex-shrink: 0;
    margin-left: 24px;                     /* explicit gap from label area */
}
```

### Current code

```rust
const MIN_HEIGHT: f32 = 44.0;                   // matches 44px
const NAME_FONT_SIZE: f32 = 13.0;               // matches 13px
const DESC_FONT_SIZE: f32 = 11.5;               // matches 11.5px
const CORNER_RADIUS: f32 = 0.0;                 // brutal — matches (no radius)
const ROW_PADDING: Insets = Insets::vh(10.0, 14.0);  // matches 10px 14px
const LABEL_CONTROL_GAP: f32 = 24.0;            // matches 24px
const NAME_DESC_GAP: f32 = 2.0;                 // matches margin-top: 2px
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Padding | 10px 14px | `Insets::vh(10.0, 14.0)` | Yes |
| Min height | 44px | `MIN_HEIGHT = 44.0` | Yes |
| Label-control gap | 24px | `LABEL_CONTROL_GAP = 24.0` | Yes |
| Name font size | 13px | 13.0 | Yes |
| Name color | `--text` (#d4d4dc) | `theme.fg_primary` (#d4d4dc) | Yes |
| Desc font size | 11.5px | 11.5 | Yes |
| Desc color | `--text-muted` (#9494a8) | `theme.fg_secondary` (#9494a8) | Yes |
| Name-desc gap | 2px | `NAME_DESC_GAP = 2.0` | Yes |
| Hover bg | `--bg-raised` (#1c1c24) | `theme.bg_card` (#1c1c24) | Yes |
| Corner radius | 0 (brutal) | 0.0 | Yes |

### Issues to verify

1. **Control flex-shrink**: Mockup sets `flex-shrink: 0` on the control so it never gets compressed. The current layout uses `LayoutBox::flex(Direction::Row, ...)` with the label as `SizeSpec::Fill` and the control as `Hug`. Verify the control never gets squished when the label text is long. If the container's layout engine respects `Hug` as non-shrinkable, this is fine.

2. **Hover bg color verification**: `theme.bg_card = Color::hex(0x1C_1C_24)` is documented as `--bg-raised`. The mockup uses `--bg-raised` for hover: `#1c1c24`. These match. The animator uses `common_states(Color::TRANSPARENT, theme.bg_card, ...)` — normal=transparent, hover=bg_card. Correct.

3. **Multi-size font rendering**: The name (13px) and description (11.5px) must render at different sizes. Until Section 01 lands, they may render at the same size. No code change needed — just verification after Section 01.

### Checklist

- [ ] All spacing/padding values match (verified in code: all match).
- [ ] Name and description render at different sizes (verify after Section 01).
- [ ] Hover background is `--bg-raised` (#1c1c24) — matches `theme.bg_card`.
- [ ] Control does not shrink when label is long.
- [ ] No code changes needed — verification-only.

---

## 11.4 Content Body Padding

**File(s):** `oriterm/src/app/settings_overlay/form_builder/appearance.rs`

### Mockup CSS

```css
.content-body {
    padding: 0 28px 28px;
    overflow-y: auto;
}
```

### Current code

```rust
pub(super) const PAGE_PADDING: Insets = Insets::vh(0.0, 28.0);

// In build_settings_page():
let mut body = ContainerWidget::column()
    .with_padding(Insets::tlbr(
        0.0,                    // top: 0 — matches
        PAGE_PADDING.left,      // left: 28px — matches
        PAGE_PADDING.top,       // bottom: 0.0 — MISMATCH
        PAGE_PADDING.right,     // right: 28px — matches
    ))
```

### Issue

The mockup has `padding: 0 28px 28px` which means:
- top: 0px
- left/right: 28px
- bottom: 28px

The current code (appearance.rs line 86-91) uses `PAGE_PADDING.top` as the bottom padding argument:
```rust
.with_padding(Insets::tlbr(
    0.0,               // top: correct
    PAGE_PADDING.left,  // left: 28.0, correct
    PAGE_PADDING.top,   // bottom: 0.0, WRONG — should be 28.0
    PAGE_PADDING.right, // right: 28.0, correct
))
```

`PAGE_PADDING = Insets::vh(0.0, 28.0)` gives `top=0, bottom=0, left=28, right=28`. The code uses `.top` (0.0) where it should use `.bottom` — but `.bottom` is also 0.0 since `vh()` sets both to the same value.

**Fix**: Change the body padding to:
```rust
Insets::tlbr(0.0, PAGE_PADDING.left, 28.0, PAGE_PADDING.right)
```

Or change `PAGE_PADDING` to `Insets::tlbr(0.0, 28.0, 28.0, 28.0)` to encode the asymmetric padding directly.

### Checklist

- [ ] Bottom padding changed from 0px to 28px.
- [ ] Verify scroll content does not get clipped by the new bottom padding.
- [ ] Build passes.

---

## 11.5 Section Spacing

**File(s):** `oriterm/src/app/settings_overlay/form_builder/appearance.rs`

### Mockup CSS

```css
.settings-section {
    margin-bottom: 28px;
}
.settings-section:last-child {
    margin-bottom: 0;
}
```

### Current code

```rust
pub(super) const SECTION_GAP: f32 = 24.0;
```

### Comparison

The mockup uses `margin-bottom: 28px` between sections. Current code uses `SECTION_GAP = 24.0`.

### Issue

4px discrepancy: mockup has 28px, current has 24px.

**Fix**: Change `SECTION_GAP` from 24.0 to 28.0:
```rust
pub(super) const SECTION_GAP: f32 = 28.0;
```

### Note on `:last-child` behavior

The mockup's `:last-child { margin-bottom: 0 }` means the last section has no bottom margin. `ContainerWidget::column().with_gap(28.0)` applies the gap between children, not after the last one. So the container's gap behavior already matches this — no trailing gap after the last section.

### Checklist

- [ ] `SECTION_GAP` changed from 24.0 to 28.0.
- [ ] Verify no visual regression in section spacing.

---

## 11.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 11.6 Build & Verify

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
- [ ] Visual verification: content area typography matches mockup
