---
section: "10"
title: "Visual Fidelity: Sidebar + Navigation"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "Sidebar nav widget is pixel-accurate against the mockup — background color, width, search field, section titles, nav items (active/hover/normal), icons, modified dots, footer, and right border all match CSS values"
depends_on: ["01", "02", "03", "05", "08"]
sections:
  - id: "10.1"
    title: "Sidebar Background + Full Height"
    status: not-started
  - id: "10.2"
    title: "Search Field Styling"
    status: not-started
  - id: "10.3"
    title: "Section Title Styling"
    status: not-started
  - id: "10.4"
    title: "Nav Item Styling"
    status: not-started
  - id: "10.5"
    title: "Sidebar Footer"
    status: not-started
  - id: "10.6"
    title: "File Size Split"
    status: not-started
  - id: "10.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "10.7"
    title: "Build & Verify"
    status: not-started
---

# Section 10: Visual Fidelity — Sidebar + Navigation

**Status:** Not Started
**Goal:** Every visual element in the sidebar nav matches the mockup CSS at 100% DPI. Side-by-side comparison should show no differences in colors, spacing, typography, or interaction states.

**Production code paths:**
- `oriterm_ui/src/widgets/sidebar_nav/mod.rs` — `SidebarNavWidget` struct, `SidebarNavStyle`, `paint()`, `paint_nav_item()`, `paint_search_field()`, `paint_footer()`
- `oriterm_ui/src/theme/mod.rs` — `UiTheme::dark()` token values
- `oriterm_ui/src/widgets/sidebar_nav/tests.rs` — harness tests

**Observable change:** Sidebar matches mockup — correct background color, search field with proper border/bg/padding, section titles with correct size/weight/spacing, nav items with proper padding/indicator/icon opacity, footer with version/config path styling.

---

## 10.1 Sidebar Background + Full Height

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs`

### Mockup CSS

```css
.sidebar {
    width: 200px;
    background: var(--bg-base);        /* #0e0e12 */
    border-right: 2px solid var(--border); /* #2a2a36 */
    display: flex;
    flex-direction: column;
    /* full height of parent — no explicit height, flexbox stretch */
}
```

### Current code

- `SIDEBAR_WIDTH: f32 = 200.0` -- matches mockup (200px).
- `SidebarNavStyle::bg` = `theme.bg_secondary` = `Color::hex(0x0E_0E_12)` -- matches mockup `--bg-base`.
- Right border: painted as 2px filled rect at `bounds.width() - 2.0` with `self.style.border` color -- matches mockup `border-right: 2px solid var(--border)`.
- Height: `layout()` returns `SizeSpec::Fill` -- should produce full-height sidebar.

### Verification needed

- Confirm the sidebar actually renders to the full height of the settings panel. The parent `ContainerWidget::row()` in `settings_overlay` must give the sidebar full height. If the sidebar stops short of the footer, the container's cross-axis alignment is wrong (should be `Stretch`, not `Start`).
- If the sidebar does not fill to the bottom, the fix is in the parent layout, not in `SidebarNavWidget`.

### Checklist

- [ ] Sidebar background is `#0e0e12` (verified visually or in code: `theme.bg_secondary`).
- [ ] Right border is 2px, color `#2a2a36` (verified: `theme.border`).
- [ ] Sidebar width is 200px (verified: `SIDEBAR_WIDTH = 200.0`).
- [ ] Sidebar fills full height of settings panel (verify at runtime).

---

## 10.2 Search Field Styling

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs` (`paint_search_field()`)

### Mockup CSS

```css
.search-field {
    height: 28px;
    border: 2px solid var(--border);       /* #2a2a36 */
    background: var(--bg-surface);         /* #16161c */
    padding: 6px 8px 6px 26px;            /* left padding for search icon */
    font-size: 12px;
    color: var(--text-faint);             /* #8c8ca0 — placeholder text */
    margin: 0 10px;                       /* horizontal margin inside sidebar */
}
```

### Current code (`paint_search_field()`)

```rust
let field_h = 28.0;                          // matches 28px
let field_rect = Rect::new(x, y, w, field_h);
let bg_style = RectStyle::filled(ctx.theme.bg_primary)   // bg_primary = #16161c = --bg-surface
    .with_border(2.0, ctx.theme.border);                  // 2px, #2a2a36 = --border
// Placeholder text:
let style = TextStyle { size: 12.0, .. };                 // 12px
ctx.measurer.shape("Search settings...", &style, w - 32.0);
let text_y = y + (field_h - shaped.height) / 2.0;
ctx.scene.push_text(Point::new(x + 26.0, text_y), ...);  // left offset 26px
// Color: ctx.theme.fg_faint = #8c8ca0 = --text-faint
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Height | 28px | 28.0 | Yes |
| Border | 2px `--border` | 2.0, `theme.border` | Yes |
| Background | `--bg-surface` (#16161c) | `theme.bg_primary` (#16161c) | Yes |
| Left padding | 26px | text at `x + 26.0` | Yes |
| Font size | 12px | 12.0 | Yes |
| Placeholder color | `--text-faint` (#8c8ca0) | `theme.fg_faint` | Yes |
| Margin | 0 10px | field at `x` (sidebar_padding_x=10) | Yes |

### Issues to verify

1. **Search icon**: The mockup has a magnifying glass icon at the left side of the search field (the 26px left padding accommodates it). The current code does not paint a search icon. This requires Section 08 (Icon Path Verification) to land first. Once icons are available, add a 14px search icon at `(x + 6.0, y + (28 - 14) / 2.0)`.

2. **Right padding**: Mockup padding is `6px 8px 6px 26px`. The text wraps at `w - 32.0` (approximately `26 + 6 = 32` left+right padding equivalent). Verify this matches — the right text boundary should be at `w - 8px` from the left edge, meaning max text width is `w - 26 - 8 = w - 34`, not `w - 32`. Minor discrepancy.

### Checklist

- [ ] Search field height 28px confirmed.
- [ ] Border 2px `--border` confirmed.
- [ ] Background `--bg-surface` confirmed.
- [ ] Placeholder text 12px, `--text-faint` confirmed.
- [ ] Left text offset 26px confirmed.
- [ ] Search icon placeholder identified (blocked on Section 08).
- [ ] Right text boundary corrected if needed (`w - 34` vs `w - 32`).

---

## 10.3 Section Title Styling

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs` (section title paint in `paint()`)

### Mockup CSS

```css
.sidebar-section-title {
    font-size: 10px;
    font-weight: 400;              /* Regular weight */
    text-transform: uppercase;
    letter-spacing: 0.15em;        /* 10px * 0.15 = 1.5px */
    color: var(--text-faint);      /* #8c8ca0 */
    padding: 0 16px;
    margin-top: 16px;              /* except first section: margin-top 8px */
}
.sidebar-section-title::before {
    content: '// ';                /* monospace comment prefix */
}
```

### Current code

```rust
let title_style = TextStyle {
    size: 10.0,                           // matches 10px
    weight: FontWeight::Regular,          // matches 400
    letter_spacing: 1.5,                  // 10 * 0.15 = 1.5px — matches
    ..TextStyle::default()
};
let title_text = format!("// {}", section.title.to_uppercase());  // matches
ctx.scene.push_text(
    Point::new(x + 6.0, y),              // x + 6.0
    shaped,
    self.style.section_title_fg           // theme.fg_faint = #8c8ca0 — matches
);
y += SECTION_TITLE_HEIGHT;               // 28.0
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Font size | 10px | 10.0 | Yes |
| Font weight | 400 (Regular) | `FontWeight::Regular` | Yes |
| Uppercase | `text-transform: uppercase` | `.to_uppercase()` | Yes |
| Letter spacing | 0.15em = 1.5px | 1.5 | Yes |
| Color | `--text-faint` (#8c8ca0) | `theme.fg_faint` | Yes |
| `//` prefix | `::before { content: '// ' }` | `format!("// {}", ...)` | Yes |
| Left padding | 16px | `x + 6.0` (x already offset by SIDEBAR_PADDING_X=10) = 16px total | Yes |

### Issues to verify

1. **Text transform dependency**: If Section 03 (Text Transform + Letter Spacing) introduces a `TextTransform::Uppercase` mechanism, the manual `.to_uppercase()` call should be replaced with the framework's text transform. But until then, the manual call is correct.

2. **Section spacing**: Mockup has `margin-top: 16px` for non-first sections, `8px` for the first. Current code uses fixed `SECTION_TITLE_HEIGHT = 28.0` which includes the title line + spacing below. The vertical spacing between the search field and first section title, and between sections, should be verified at runtime.

3. **Letter spacing dependency**: The current `letter_spacing: 1.5` is set but may not be rendered yet — this depends on Section 03 landing. Until the font pipeline supports letter spacing, the text will render without extra spacing. Verify after Section 03 is complete.

### Checklist

- [ ] Font size, weight, color, prefix all match mockup.
- [ ] Left padding totals 16px from sidebar edge.
- [ ] Vertical spacing between sections matches mockup.
- [ ] Letter spacing renders correctly (after Section 03).

---

## 10.4 Nav Item Styling

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs` (`paint_nav_item()`)

### Mockup CSS

```css
.nav-item {
    padding: 7px 16px;
    height: 32px;                          /* fixed row height */
    font-size: 13px;
    color: var(--text-muted);              /* #9494a8 */
    border-left: 3px solid transparent;    /* reserve space for active indicator */
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
}

.nav-item:hover {
    background: var(--bg-hover);           /* #24242e */
    color: var(--text);                    /* #d4d4dc */
}

.nav-item.active {
    background: var(--accent-bg-strong);   /* rgba(109,155,224,0.14) */
    color: var(--accent);                  /* #6d9be0 */
    border-left-color: var(--accent);      /* #6d9be0 — 3px left border visible */
}

.nav-item .icon {
    width: 16px;
    height: 16px;
    opacity: 0.7;
}

.nav-item.active .icon {
    opacity: 1.0;
}

.nav-item .modified-dot {
    width: 6px;
    height: 6px;
    background: var(--warning);            /* #e0c454 */
    margin-left: auto;                     /* push to right edge */
}
```

### Current code (`paint_nav_item()`)

```rust
// Row height
const ITEM_HEIGHT: f32 = 32.0;                     // matches 32px

// Active indicator (3px left border)
const INDICATOR_WIDTH: f32 = 3.0;                   // matches 3px
let indicator = Rect::new(x, y, INDICATOR_WIDTH, ITEM_HEIGHT);
ctx.scene.push_quad(indicator, RectStyle::filled(self.style.active_fg));
// active_fg = theme.accent = #6d9be0                // matches

// Background
let bg = if is_active {
    self.style.active_bg       // theme.accent_bg_strong = rgba(0.427,0.608,0.878,0.14)
} else if hovered {
    self.style.hover_bg        // theme.bg_hover = #24242e
} else {
    Color::TRANSPARENT
};
// All match mockup.

// Icon
let icon_size = 16_u32;                              // matches 16px
let c = if is_active {
    self.style.active_fg       // full opacity — matches 1.0
} else if hovered {
    self.style.hover_fg.with_alpha(0.7)  // hover + 0.7
} else {
    self.style.item_fg.with_alpha(0.7)   // 0.7 opacity — matches
};

// Label
let style = TextStyle { size: 13.0, .. };            // matches 13px
let fg = if is_active {
    self.style.active_fg       // theme.accent = #6d9be0
} else if hovered {
    self.style.hover_fg        // theme.fg_primary = #d4d4dc
} else {
    self.style.item_fg         // theme.fg_secondary = #9494a8
};

// Modified dot
let dot_size = 6.0;                                  // matches 6px
ctx.theme.warning                                    // #e0c454 — matches
```

### Detailed comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Row height | 32px | `ITEM_HEIGHT = 32.0` | Yes |
| Padding (vertical) | 7px | `(32 - 13) / 2 = 9.5` centering | Close (see note) |
| Padding (horizontal) | 16px | `x + INDICATOR_WIDTH + 8.0` = 11px from item edge | No |
| Font size | 13px | 13.0 | Yes |
| Normal color | `--text-muted` (#9494a8) | `theme.fg_secondary` (#9494a8) | Yes |
| Hover bg | `--bg-hover` (#24242e) | `theme.bg_hover` (#24242e) | Yes |
| Hover text | `--text` (#d4d4dc) | `theme.fg_primary` (#d4d4dc) | Yes |
| Active bg | `--accent-bg-strong` (0.14) | `theme.accent_bg_strong` (0.14) | Yes |
| Active text | `--accent` (#6d9be0) | `theme.accent` (#6d9be0) | Yes |
| Active border-left | 3px `--accent` | 3px filled rect, `theme.accent` | Yes |
| Icon size | 16px | 16 | Yes |
| Icon opacity (normal) | 0.7 | `with_alpha(0.7)` | Yes |
| Icon opacity (active) | 1.0 | full color (no alpha mod) | Yes |
| Icon-text gap | 10px | icon at +8px, text at +32px, icon is 16px => gap = 32 - 8 - 16 = 8px | No (8 vs 10) |
| Modified dot | 6px, `--warning` | 6.0, `theme.warning` | Yes |

### Issues to fix

1. **Horizontal padding mismatch**: Mockup uses `padding: 7px 16px` meaning 16px from each edge. Current code uses `x + INDICATOR_WIDTH + 8.0 = x + 11`. The icon is at `x + 11`, but mockup puts it at `x + 3 (indicator) + 16 (padding) = x + 19`. With the 3px indicator being part of the item, the text/icon area starts at `x + 3 + 16 = x + 19`, not `x + 11`. The icon position needs to be `x + INDICATOR_WIDTH + 16.0` and the text `x + INDICATOR_WIDTH + 16.0 + 16.0 (icon) + 10.0 (gap) = x + 45.0`.

   Current icon X: `x + INDICATOR_WIDTH + 8.0 = x + 11.0`
   Mockup icon X: `x + INDICATOR_WIDTH + 13.0 = x + 16.0` (padding 16 - indicator 3 = 13 from indicator edge)

   This needs careful recalculation. The mockup's `padding: 7px 16px` includes the 3px border-left (the padding is inside the border). So:
   - Icon X: `x + 3 (indicator) + 16 (padding) = x + 19` -- but 16px padding seems too large for a 200px sidebar. Let's verify: sidebar width 200 - padding_x*2=20 = 180px item width. With 3px indicator + 16px left padding + 16px icon + 10px gap = 45px consumed by prefix, leaving 135px for text + 16px right padding = 119px for text. That's plausible.

   Current: text at `x + 32` (indicator 3 + 8 icon offset + 16 icon + 5 gap). This is tighter than mockup.

   **Action**: Adjust icon offset from `+8.0` to `+13.0` and text offset from `+32.0` to `+42.0` (13 + 16 icon + 10 gap + 3 indicator = 42 from x, or 16 + 16 + 10 = 42 from indicator).

2. **Icon-text gap**: Mockup has `gap: 10px`. Current code implicitly has 8px (32 - 8 - 16). Should be 10px.

3. **Hover icon opacity**: Current code applies `0.7` opacity to hover icons. But the mockup shows no explicit change to icon opacity on hover — only the text color changes. The icon should remain at `0.7` on hover, not change. Current code uses `hover_fg.with_alpha(0.7)` which would be `#d4d4dc` at 0.7 alpha — different from the normal state's `#9494a8` at 0.7 alpha. The hover icon color should use the hover text color at 0.7, or stay the same as normal. Check mockup carefully.

   Looking at the mockup CSS: `.nav-item .icon { opacity: 0.7; }` with no hover override except `.nav-item.active .icon { opacity: 1.0; }`. So on hover, icon stays at 0.7 opacity but picks up the parent's text color change. Current behavior (`hover_fg.with_alpha(0.7)`) is approximately correct — the icon tracks the text color but stays dimmed.

### Checklist

- [ ] Horizontal padding adjusted to 16px (from sidebar edge, inside indicator).
- [ ] Icon-text gap adjusted to 10px.
- [ ] Icon positions recalculated with correct padding.
- [ ] Hover icon color verified (should be hover text color at 0.7 alpha).
- [ ] All color values verified against mockup CSS variables.
- [ ] Modified dot right margin matches mockup (`margin-left: auto` pushes to right edge at `item_rect.right() - 16.0`).

---

## 10.5 Sidebar Footer

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs` (`paint_footer()`)

### Mockup CSS

```css
.sidebar-footer {
    padding: 12px 28px;
    border-top: 2px solid var(--border);   /* #2a2a36 */
    margin-top: auto;                      /* push to bottom */
}
.sidebar-footer .version {
    font-size: 11px;
    color: var(--text-faint);              /* #8c8ca0 */
}
.sidebar-footer .config-path {
    font-size: 10px;
    color: var(--text-faint);
    opacity: 0.7;                          /* dimmer than version */
    margin-top: 4px;
}
```

### Current code (`paint_footer()`)

```rust
fn paint_footer(&self, ctx: &mut DrawCtx<'_>, x: f32, item_w: f32) {
    let mut y = ctx.bounds.bottom() - 8.0;    // 8px from bottom

    // Config path
    let style = TextStyle { size: 10.0, .. };  // 10px — matches
    let fg = self.style.version_fg.with_alpha(0.7);  // faint + 0.7 — matches
    ctx.scene.push_text(Point::new(x + 6.0, y), ...);
    y -= 4.0;                                  // 4px gap — matches

    // Version label
    let style = TextStyle { size: 11.0, .. };  // 11px — matches
    ctx.scene.push_text(..., self.style.version_fg);   // faint — matches
}
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Padding | 12px 28px | 8px bottom, `x + 6.0` left (= 16px from edge) | No |
| Border-top | 2px `--border` | Not drawn | No |
| Version font | 11px, `--text-faint` | 11.0, `version_fg` (fg_faint) | Yes |
| Config path font | 10px, `--text-faint` at 0.7 | 10.0, `version_fg.with_alpha(0.7)` | Yes |
| Gap between items | 4px | 4.0 | Yes |
| Push to bottom | `margin-top: auto` | positioned from `bounds.bottom()` | Yes |

### Issues to fix

1. **Missing border-top**: Mockup has `border-top: 2px solid var(--border)`. Current `paint_footer()` does not draw this. Add a 2px horizontal line at the top of the footer area. This requires knowing the footer's top Y coordinate. Calculate: `footer_top = bounds.bottom() - footer_height`. Footer height = 12 (top pad) + 11 (version) + 4 (gap) + 10 (config path) + 12 (bottom pad) ~ 49px. Draw a `Rect::new(x, footer_top, item_w, 2.0)` filled with `self.style.border`.

   Alternatively, this is a per-side border (Section 05 dependency). If Section 05 provides `Border::top(2.0, color)`, use that. Otherwise, draw manually as a filled rect (same approach the right border uses).

2. **Padding mismatch**: Mockup has 12px vertical padding and 28px horizontal padding. Current code has 8px bottom margin and text at `x + 6.0`. The horizontal padding should be 28px from the sidebar edge (not 16px). Adjust text X to `bounds.x() + 28.0` instead of `x + 6.0` (where `x = bounds.x() + SIDEBAR_PADDING_X = bounds.x() + 10`, so `x + 6 = bounds.x() + 16`). Change to `bounds.x() + 28.0` or `x + 18.0`.

3. **Bottom padding**: Mockup has 12px. Current has 8px. Change `bounds.bottom() - 8.0` to `bounds.bottom() - 12.0`.

### Checklist

- [ ] Border-top 2px `--border` drawn at footer top.
- [ ] Horizontal padding adjusted to 28px from sidebar edge.
- [ ] Bottom padding adjusted to 12px.
- [ ] Version and config path text positions recalculated.
- [ ] Footer area height accounts for border + padding + text.

---

## 10.6 File Size Split

**File(s):** `oriterm_ui/src/widgets/sidebar_nav/mod.rs`

The sidebar nav module is currently 509 lines (over the 500-line limit). After the changes in 10.1-10.5, it will likely grow further.

### Split plan

Extract paint methods into a `paint` submodule:

```
oriterm_ui/src/widgets/sidebar_nav/
    mod.rs          ← struct, style, Widget impl (layout, on_input, lifecycle, accept_action)
    paint.rs        ← paint(), paint_nav_item(), paint_search_field(), paint_footer()
    tests.rs        ← existing tests
```

### Implementation

1. Create `oriterm_ui/src/widgets/sidebar_nav/paint.rs`.
2. Move `paint_nav_item()`, `paint_search_field()`, `paint_footer()`, and the `paint()` method body into paint helpers.
3. In `mod.rs`, add `mod paint;` and make the `Widget::paint()` impl delegate to a function in `paint.rs`.
4. The paint functions need access to `SidebarNavWidget` fields (`sections`, `active_page`, `hovered_item`, `style`, `version`, `config_path`, `modified_pages`) and `DrawCtx`. Pass `&self` or make the functions methods via an `impl SidebarNavWidget` block in `paint.rs`.
5. Make necessary fields `pub(super)` so the paint submodule can access them.

### Checklist

- [ ] `paint.rs` created with paint methods.
- [ ] `mod.rs` under 500 lines after split.
- [ ] No public API changes (all paint functions are private).
- [ ] `cargo test -p oriterm_ui` passes.
- [ ] `./clippy-all.sh` passes.

---

## 10.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 10.7 Build & Verify

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
- [ ] sidebar_nav/mod.rs under 500 lines after split
- [ ] Visual verification: sidebar matches mockup CSS values
