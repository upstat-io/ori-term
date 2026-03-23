---
section: "12"
title: "Visual Fidelity: Footer + Buttons"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "Footer bar layout, button styles (primary, ghost, danger-ghost), UNSAVED CHANGES indicator, and Reset-Cancel-Save ordering all match the mockup"
depends_on: ["02", "03", "05"]
sections:
  - id: "12.1"
    title: "Footer Layout"
    status: not-started
  - id: "12.2"
    title: "Button Styles"
    status: not-started
  - id: "12.3"
    title: "UNSAVED CHANGES Indicator"
    status: not-started
  - id: "12.4"
    title: "Reset Button Position"
    status: not-started
  - id: "12.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "12.5"
    title: "Build & Verify"
    status: not-started
---

# Section 12: Visual Fidelity — Footer + Buttons

**Status:** Not Started
**Goal:** The footer bar at the bottom of the settings panel matches the mockup — correct padding, border-top, button styles (primary/ghost/danger-ghost), UNSAVED CHANGES indicator with icon, and button ordering (Reset left, Cancel+Save right).

**Production code paths:**
- Footer builder: `oriterm_ui/src/widgets/settings_panel/mod.rs` (footer construction and rendering)
- Button widget: `oriterm_ui/src/widgets/button/mod.rs` (`ButtonStyle` — currently lacks `font_weight` and `letter_spacing` fields)
- UNSAVED indicator: `oriterm_ui/src/widgets/settings_panel/mod.rs` (paint-time overlay text)
- Label widget: `oriterm_ui/src/widgets/label/mod.rs` (`LabelStyle`)
- Spacer widget: `oriterm_ui/src/widgets/spacer/mod.rs` (for `margin-right: auto` equivalent)

**Observable change:** Footer renders with correct border-top, proper padding, all three button variants match mockup styling, UNSAVED CHANGES has a warning icon, and Reset is left-aligned while Cancel+Save are right-aligned.

---

## 12.1 Footer Layout

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs` (`build_footer()`)

### Mockup CSS

```css
.footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 12px 28px;
    border-top: 2px solid var(--border);   /* #2a2a36 */
    background: var(--bg-surface);         /* #16161c */
    flex-shrink: 0;
}
```

### Current code

```rust
const FOOTER_HEIGHT: f32 = 52.0;
// Footer padding — left side skips sidebar, both sides use 28px content padding.
let footer_pad = Insets::tlbr(0.0, SIDEBAR_WIDTH + 28.0, 0.0, 28.0);

let footer = ContainerWidget::row()
    .with_align(Align::Center)
    .with_width(SizeSpec::Fill)
    .with_height(SizeSpec::Fixed(FOOTER_HEIGHT))
    .with_padding(footer_pad)
    .with_child(Box::new(reset_btn))
    .with_child(Box::new(SpacerWidget::fill()))   // push remaining to right
    .with_child(Box::new(cancel_btn))
    .with_child(Box::new(SpacerWidget::fixed(8.0, 0.0)))  // 8px gap
    .with_child(Box::new(save_btn));
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Padding vertical | 12px | 0px (height=52, buttons ~24px, vertical centering ~14px each side) | Implicit via centering |
| Padding horizontal | 28px | 28px (right), `SIDEBAR_WIDTH + 28` (left, skip sidebar) | Yes |
| Gap | 8px | `SpacerWidget::fixed(8.0, 0.0)` between cancel/save | Yes |
| Border-top | 2px `--border` | Drawn as separate `footer_sep` with `SeparatorWidget` | Partial |
| Background | `--bg-surface` (#16161c) | Opaque bar drawn in `paint()` using panel bg | Yes |
| Height | auto (padding + content) | Fixed 52px | Acceptable |

### Issues to verify

1. **Footer height calculation**: Mockup has `padding: 12px 28px` meaning 12px top + 12px bottom = 24px vertical padding. Button height is ~28px (font 12px + padding 6px*2 + border 2px*2 = ~28px). Total: 24 + 28 = 52px. Current `FOOTER_HEIGHT = 52.0` matches.

2. **Border-top implementation**: The footer separator is drawn as a `SeparatorWidget::horizontal()` inside a `ContainerWidget::row()` with left padding = `SIDEBAR_WIDTH`. This means the border-top starts at the sidebar boundary and extends to the right edge. The mockup's `border-top` spans the full footer width (including under the sidebar). Verify whether the separator should span the full width or only the content area.

   Looking at the mockup: the footer only exists in the content area (right of sidebar). The sidebar has its own footer. So the current implementation (separator starts at `SIDEBAR_WIDTH`) is correct.

3. **Separator thickness**: The separator is configured with `thickness: 2.0` which matches `border-top: 2px`. Correct.

### Checklist

- [ ] Footer height 52px confirmed.
- [ ] Padding 12px 28px produces correct centering (verified via height = 52, content ~28).
- [ ] Border-top 2px at content area boundary (starts at sidebar width).
- [ ] Gap 8px between Cancel and Save.
- [ ] Background matches panel surface color.

---

## 12.2 Button Styles

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs` (`build_footer()`), `oriterm_ui/src/widgets/button/mod.rs` (`ButtonStyle`)

### Mockup CSS — btn-primary (Save)

```css
.btn-primary {
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;                /* 12px * 0.04 = 0.48px */
    padding: 6px 20px;
    border: 2px solid var(--accent);       /* #6d9be0 */
    background: var(--accent);             /* #6d9be0 */
    color: var(--bg-base);                 /* #0e0e12 — dark text on light bg */
}
.btn-primary:hover {
    background: var(--accent-hover);       /* #85ade8 */
    border-color: var(--accent-hover);
}
```

### Mockup CSS — btn-ghost (Cancel)

```css
.btn-ghost {
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 6px 16px;
    border: 2px solid var(--border);       /* #2a2a36 */
    background: transparent;
    color: var(--text-muted);              /* #9494a8 */
}
.btn-ghost:hover {
    background: var(--bg-hover);           /* #24242e */
    border-color: var(--border-strong);    /* #3a3a48 */
    color: var(--text);                    /* #d4d4dc */
}
```

### Mockup CSS — btn-danger-ghost (Reset to Defaults)

```css
.btn-danger-ghost {
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 6px 16px;
    border: 2px solid var(--border);       /* #2a2a36 */
    background: transparent;
    color: var(--text-muted);              /* #9494a8 */
}
.btn-danger-ghost:hover {
    border-color: var(--danger);           /* #c87878 */
    color: var(--danger);
    background: var(--danger-bg);          /* rgba(200,120,120,0.08) */
}
```

### Current code comparison

**Save button (btn-primary):**

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Font size | 12px | `font_size: 12.0` | Yes |
| Font weight | 500 | Not set (default Regular) | No |
| Text transform | uppercase | Text is `"SAVE"` (manual) | Yes |
| Letter spacing | 0.04em = 0.48px | Not set in ButtonStyle | No |
| Padding | 6px 20px | `Insets::vh(6.0, 20.0)` | Yes |
| Border | 2px `--accent` | `border_width: 2.0, border_color: theme.accent` | Yes |
| Bg | `--accent` | `bg: theme.accent` | Yes |
| Text color | `--bg-base` (#0e0e12) | `fg: theme.bg_secondary` (#0e0e12) | Yes |
| Hover bg | `--accent-hover` | `hover_bg: theme.accent_hover` (#85ade8) | Yes |
| Hover border | `--accent-hover` | `hover_border_color: theme.accent_hover` | Yes |

**Cancel button (btn-ghost):**

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Font size | 12px | `font_size: 12.0` | Yes |
| Font weight | 500 | Not set | No |
| Padding | 6px 16px | `Insets::vh(6.0, 16.0)` | Yes |
| Border | 2px `--border` | `border_width: 2.0, border_color: theme.border` | Yes |
| Bg | transparent | `bg: Color::TRANSPARENT` | Yes |
| Text color | `--text-muted` | `fg: theme.fg_secondary` | Yes |
| Hover bg | `--bg-hover` | `hover_bg: theme.bg_hover` | Yes |
| Hover border | `--border-strong` | `hover_border_color: theme.border_strong` | Yes |
| Hover text | `--text` (#d4d4dc) | `hover_fg: theme.fg_primary` | Yes |

**Reset button (btn-danger-ghost):**

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Font size | 12px | `font_size: 12.0` | Yes |
| Font weight | 500 | Not set | No |
| Padding | 6px 16px | `Insets::vh(6.0, 16.0)` | Yes |
| Border | 2px `--border` | `border_width: 2.0, border_color: theme.border` | Yes |
| Bg | transparent | `bg: Color::TRANSPARENT` | Yes |
| Text color | `--text-muted` | `fg: theme.fg_secondary` | Yes |
| Hover border | `--danger` | `hover_border_color: theme.danger` | Yes |
| Hover text | `--danger` | `hover_fg: theme.danger` | Yes |
| Hover bg | `--danger-bg` | `hover_bg: theme.danger_bg` | Yes |

### Issues to fix

1. **Font weight**: All three buttons use `font-weight: 500` (Medium) in the mockup. `ButtonStyle` currently has `font_size` but no `font_weight` field. Two options:

   a. Add a `font_weight: FontWeight` field to `ButtonStyle` (preferred — matches `LabelStyle` pattern).
   b. Set weight via `TextStyle` inside `paint()`.

   **Action**: Add `font_weight` field to `ButtonStyle`, default to `FontWeight::Regular`, set to `FontWeight::MEDIUM` in the footer button styles. This depends on Section 02 providing `FontWeight::MEDIUM`.

2. **Letter spacing**: All three buttons use `letter-spacing: 0.04em = 0.48px`. `ButtonStyle` has no `letter_spacing` field. Add it.

   **Action**: Add `letter_spacing: f32` field to `ButtonStyle`, default to `0.0`, set to `0.48` in footer buttons. Use in `text_style()` method. This depends on Section 03 providing letter spacing rendering.

3. **ButtonStyle field additions summary**:
   ```rust
   pub struct ButtonStyle {
       // ... existing fields ...
       /// Font weight (CSS font-weight).
       pub font_weight: FontWeight,
       /// Letter spacing in logical pixels (CSS letter-spacing).
       pub letter_spacing: f32,
   }
   ```

   Default: `font_weight: FontWeight::Regular, letter_spacing: 0.0`.
   Footer overrides: `font_weight: FontWeight::MEDIUM, letter_spacing: 0.48`.

### Checklist

- [ ] `font_weight` field added to `ButtonStyle`.
- [ ] `letter_spacing` field added to `ButtonStyle`.
- [ ] Footer buttons set `font_weight: FontWeight::MEDIUM`.
- [ ] Footer buttons set `letter_spacing: 0.48`.
- [ ] `text_style()` in `ButtonWidget` uses both new fields.
- [ ] All three button variants match mockup colors/padding/border (verified: all match).
- [ ] `./build-all.sh` and `./clippy-all.sh` pass.

---

## 12.3 UNSAVED CHANGES Indicator

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs` (`paint()`, around line 417)

### Mockup CSS

```css
.unsaved-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;                /* 11px * 0.06 = 0.66px */
    color: var(--warning);                 /* #e0c454 */
}
.unsaved-indicator svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: var(--warning);
    stroke-width: 2;
}
```

### Current code

```rust
if self.unsaved {
    if let Some(footer_node) = children.last() {
        let style = crate::text::TextStyle::new(11.0, ctx.theme.warning);
        let shaped = ctx.measurer.shape("UNSAVED CHANGES", &style, 200.0);
        let x = footer_node.rect.x() + SIDEBAR_WIDTH + 28.0;
        let y = footer_node.rect.y() + (footer_node.rect.height() - shaped.height) / 2.0;
        ctx.scene.push_text(Point::new(x, y), shaped, ctx.theme.warning);
    }
}
```

### Comparison

| Property | Mockup | Current | Match? |
|----------|--------|---------|--------|
| Font size | 11px | 11.0 | Yes |
| Font weight | 500 | Not set | No |
| Uppercase | `text-transform: uppercase` | `"UNSAVED CHANGES"` (manual) | Yes |
| Letter spacing | 0.06em = 0.66px | Not set | No |
| Color | `--warning` (#e0c454) | `ctx.theme.warning` | Yes |
| Warning icon | 14px SVG (triangle with !) | Not present | No |
| Icon-text gap | 6px | N/A (no icon) | No |
| X position | Reset button area (left side) | `SIDEBAR_WIDTH + 28.0` | See note |

### Issues to fix

1. **Warning icon**: The mockup has a warning triangle SVG icon (exclamation mark inside triangle) before the text. This needs:
   - An `IconId::Warning` variant in the icon system.
   - The SVG path for the warning triangle.
   - Icon rendered at 14x14px, stroked with `--warning` color.
   - Positioned at the current text X, with text shifted right by `14 + 6 = 20px`.

   This depends on Section 08 (Icon Path Verification) for the icon infrastructure. If icons are not yet available, skip the icon and just fix the text styling.

2. **Font weight + letter spacing**: The text should use `FontWeight::MEDIUM` (500) and letter spacing 0.66px. These depend on Sections 02 and 03 respectively. Apply once available:
   ```rust
   let style = TextStyle {
       size: 11.0,
       weight: FontWeight::MEDIUM,
       letter_spacing: 0.66,
       ..TextStyle::default()
   };
   ```

3. **X position**: The indicator is positioned at `SIDEBAR_WIDTH + 28.0` from the footer rect's left edge. This places it at the start of the content area (same as the Reset button). The mockup shows UNSAVED CHANGES text between the Reset button and the spacer. The current code draws it OVER the footer, which means it overlaps the Reset button.

   **Fix**: The indicator should be part of the footer layout, not a paint-time overlay. Options:
   a. Add it as a `LabelWidget` child of the footer `ContainerWidget`, positioned after the Reset button and before the spacer.
   b. Keep the overlay approach but adjust X to be after the Reset button: `reset_btn_right + 12px`.

   Option (a) is architecturally cleaner but requires making the unsaved state accessible at build time. Option (b) is a rendering fix. For now, option (b) is acceptable if the indicator positions correctly — it just needs to not overlap the Reset button.

### Checklist

- [ ] Warning icon added (after Section 08 provides icon infrastructure).
- [ ] Font weight set to Medium (500) (after Section 02).
- [ ] Letter spacing set to 0.66px (after Section 03).
- [ ] Indicator positioned to not overlap Reset button.
- [ ] Icon-text gap of 6px when icon is present.
- [ ] Color matches `--warning` (#e0c454) — already correct.

---

## 12.4 Reset Button Position

**File(s):** `oriterm_ui/src/widgets/settings_panel/mod.rs` (`build_footer()`)

### Mockup CSS

```css
.footer-left {
    margin-right: auto;                    /* push everything after it to the right */
}
/* Structure: [Reset] [auto-spacer] [UNSAVED text] [Cancel] [8px] [Save] */
/* OR: [Reset] [auto-spacer] [Cancel] [8px] [Save] when no unsaved changes */
```

### Current code

```rust
.with_child(Box::new(reset_btn))               // 1. Reset (left)
.with_child(Box::new(SpacerWidget::fill()))     // 2. Auto spacer (pushes right)
.with_child(Box::new(cancel_btn))               // 3. Cancel
.with_child(Box::new(SpacerWidget::fixed(8.0, 0.0)))  // 4. 8px gap
.with_child(Box::new(save_btn))                 // 5. Save (rightmost)
```

### Comparison

The ordering matches the mockup:
- Reset is leftmost.
- `SpacerWidget::fill()` acts as `margin-right: auto`, pushing Cancel+Save to the right edge.
- Cancel and Save are grouped with an 8px gap.

This is correct.

### Issues to verify

1. **UNSAVED CHANGES position in flow**: The mockup shows the UNSAVED CHANGES text between the Reset button and the Cancel button (after the spacer pushes it right). But the current implementation draws it as a paint-time overlay, not as a child of the footer row. This means it doesn't participate in the layout flow.

   If the UNSAVED CHANGES text needs to be in the footer layout (between spacer and Cancel), the indicator would need to be a `LabelWidget` child. But that would require rebuilding the footer container when `unsaved` changes, which is complex.

   **Decision**: The overlay approach is acceptable as long as positioning is correct. The text appears left-aligned in the content area when the Reset button is small enough. If they overlap, increase the indicator's X offset.

2. **Verify no overlap**: With the Reset button text "RESET TO DEFAULTS" at 12px, the button width is approximately `"RESET TO DEFAULTS".len() * 7 + 32 (padding) + 4 (border) ≈ 140px`. The indicator at `SIDEBAR_WIDTH + 28 = 228px` would start at pixel 228, while the Reset button ends at approximately `228 + 140 = 368px`. The indicator text at X=228 would be behind the Reset button. This is a bug.

   **Fix**: Position the UNSAVED CHANGES indicator after the Reset button. Either:
   - Calculate Reset button width and offset by it.
   - Move the indicator to the right side (between Cancel and Save, or before Cancel).
   - Make it a layout child, not a paint overlay.

### Checklist

- [ ] Reset button is leftmost in footer (verified: correct).
- [ ] SpacerWidget::fill() pushes Cancel+Save right (verified: correct).
- [ ] 8px gap between Cancel and Save (verified: correct).
- [ ] UNSAVED CHANGES indicator does not overlap Reset button (fix needed).
- [ ] Footer ordering matches mockup.

---

## 12.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 12.5 Build & Verify

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
- [ ] Visual verification: footer layout and button styles match mockup
