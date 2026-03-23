---
section: "07"
title: "Scrollbar Styling"
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
goal: "Scrollbar rendering matches the mockup — 6px thin overlay, transparent track, theme border color thumb with hover brightening to text-faint"
inspired_by:
  - "CSS ::-webkit-scrollbar, ::-webkit-scrollbar-track, ::-webkit-scrollbar-thumb"
depends_on: []
sections:
  - id: "07.1"
    title: "ScrollbarStyle Enhancement"
    status: not-started
  - id: "07.2"
    title: "Hover State for Scrollbar Thumb"
    status: not-started
  - id: "07.3"
    title: "Track Transparency"
    status: not-started
  - id: "07.4"
    title: "Tests"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "07.5"
    title: "Build & Verify"
    status: not-started
---

# Section 07: Scrollbar Styling

**Goal:** Verify and refine the `ScrollWidget`'s scrollbar rendering to match the mockup's CSS scrollbar styling. The mockup specifies:

```css
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border); }    /* #3a3a3a */
::-webkit-scrollbar-thumb:hover { background: var(--text-faint); } /* #888 */
```

**References:**
- `oriterm_ui/src/widgets/scroll/mod.rs` — `ScrollWidget`, `ScrollbarStyle`, `ScrollbarState`
- `oriterm_ui/src/widgets/scroll/scrollbar.rs` — `draw_scrollbar()`, `scrollbar_thumb_rect()`, `scrollbar_track_rect()`
- `oriterm_ui/src/widgets/scroll/rendering.rs` — `draw_impl()` (main scroll render path)
- `oriterm_ui/src/theme/mod.rs` — `UiTheme` (border color, text_faint color)
- `mockups/settings-brutal.html` — scrollbar CSS at lines ~110-115

---

## 07.1 ScrollbarStyle Enhancement

### Current ScrollbarStyle Fields

The existing `ScrollbarStyle` struct (at `oriterm_ui/src/widgets/scroll/mod.rs:38-49`):

```rust
pub struct ScrollbarStyle {
    pub width: f32,           // Default: 6.0
    pub thumb_color: Color,   // Default: Color::WHITE.with_alpha(0.25)
    pub track_color: Color,   // Default: Color::TRANSPARENT
    pub thumb_radius: f32,    // Default: 3.0
    pub min_thumb_height: f32, // Default: 20.0
}
```

### Comparison with Mockup

| Property | Mockup CSS | Current Default | Match? |
|----------|-----------|-----------------|--------|
| Width | `6px` | `6.0` | Yes |
| Track background | `transparent` | `Color::TRANSPARENT` | Yes |
| Thumb color (normal) | `var(--border)` = `#3a3a3a` | `Color::WHITE.with_alpha(0.25)` | No |
| Thumb color (hover) | `var(--text-faint)` = `#888` | Computed from `thumb_color` with boosted alpha | Partial |
| Thumb radius | Not specified (implied by 6px width) | `3.0` (half of width, fully rounded) | Yes |
| Min thumb height | Not specified | `20.0` | Reasonable |

### Required Changes

**Thumb color mismatch.** The default `thumb_color` is `WHITE.with_alpha(0.25)`, which renders as semi-transparent white. The mockup uses `#3a3a3a` (an opaque dark gray, the theme's `--border` color). These look similar on a dark background but differ on lighter surfaces.

The fix is not to change the hardcoded default but to have the scroll widget read the thumb color from `UiTheme` at render time. The `ScrollbarStyle` defaults are fine as fallbacks, but widgets constructing scroll containers for the settings panel should use theme-derived colors.

**Option A: Theme-aware defaults.** Add a `ScrollbarStyle::from_theme(theme: &UiTheme)` constructor:

```rust
impl ScrollbarStyle {
    /// Creates scrollbar style from the active theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            width: 6.0,
            thumb_color: theme.border,      // #3a3a3a
            track_color: Color::TRANSPARENT,
            thumb_radius: 3.0,
            min_thumb_height: 20.0,
        }
    }
}
```

**Option B: Hover color field.** Add an explicit `thumb_hover_color` field rather than computing it from alpha manipulation:

```rust
pub struct ScrollbarStyle {
    pub width: f32,
    pub thumb_color: Color,
    pub thumb_hover_color: Color,    // NEW
    pub thumb_drag_color: Color,     // NEW
    pub track_color: Color,
    pub thumb_radius: f32,
    pub min_thumb_height: f32,
}
```

**Recommendation: Both.** Add the hover/drag color fields and provide `from_theme()`:

```rust
impl ScrollbarStyle {
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            width: 6.0,
            thumb_color: theme.border,           // normal: #3a3a3a
            thumb_hover_color: theme.text_faint,  // hover: #888
            thumb_drag_color: theme.text_faint,   // drag: same as hover
            track_color: Color::TRANSPARENT,
            thumb_radius: 3.0,
            min_thumb_height: 20.0,
        }
    }
}
```

The existing alpha-based hover/drag colors in `draw_scrollbar()` become the fallback for the old `Default` implementation, but the `from_theme()` path uses explicit colors.

### Migration

Update scroll container construction in the settings panel to use `ScrollbarStyle::from_theme()` instead of `ScrollbarStyle::default()`. The terminal grid's scroll (if any) can keep the default or switch to theme-aware.

---

## 07.2 Hover State for Scrollbar Thumb

### Current Hover Tracking

The `ScrollbarState` struct tracks:
- `dragging: bool` — whether the thumb is being dragged
- `drag_start_y: f32` — Y position at drag start
- `drag_start_offset: f32` — scroll offset at drag start
- `track_hovered: bool` — whether the cursor is over the scrollbar track area

The existing `draw_scrollbar()` logic at `scrollbar.rs:81-88`:

```rust
let thumb_color = if self.scrollbar.dragging {
    s.thumb_color.with_alpha(0.6)
} else if self.scrollbar.track_hovered {
    s.thumb_color.with_alpha(0.4)
} else {
    s.thumb_color
};
```

### Issue: Track vs. Thumb Hover

The mockup CSS distinguishes between hovering over the scrollbar thumb specifically (`::-webkit-scrollbar-thumb:hover`) and the track. The current code tracks `track_hovered` (whether the cursor is anywhere in the scrollbar track area), not `thumb_hovered` (whether the cursor is specifically over the thumb).

For the mockup's behavior, track hover is actually fine. The scrollbar is only 6px wide — if you are hovering anywhere in the scrollbar area, you are visually "hovering the scrollbar." The CSS `::-webkit-scrollbar-thumb:hover` fires when the cursor is over the thumb, but since the thumb fills most of the track height for typical content ratios, the distinction is minimal.

### Recommendation: Add Thumb Hover Detection

For visual accuracy, add `thumb_hovered: bool` to `ScrollbarState` and detect it separately:

```rust
struct ScrollbarState {
    dragging: bool,
    drag_start_y: f32,
    drag_start_offset: f32,
    track_hovered: bool,
    thumb_hovered: bool,  // NEW: cursor is specifically over the thumb
}
```

In `handle_scrollbar_move()`:

```rust
// After track hover detection:
let thumb = self.scrollbar_thumb_rect(viewport, content_h, view_h);
let was_thumb_hovered = self.scrollbar.thumb_hovered;
self.scrollbar.thumb_hovered = thumb.contains(pos);
```

Update `draw_scrollbar()` to use the new fields with explicit colors:

```rust
let thumb_color = if self.scrollbar.dragging {
    s.thumb_drag_color
} else if self.scrollbar.thumb_hovered {
    s.thumb_hover_color
} else {
    s.thumb_color
};
```

### Track Width on Hover

The current code widens the scrollbar track from `width` to `width * 1.5` when hovered or dragging (in `scrollbar_track_rect()`). This is a nice interaction detail that the CSS mockup does not have. Whether to keep or remove it is a design decision.

**Recommendation:** Keep it. The wider track on hover improves drag target acquisition and provides visual feedback. It does not conflict with the mockup's styling.

---

## 07.3 Track Transparency

### Current Behavior

The current `draw_scrollbar()` at `scrollbar.rs:72-77`:

```rust
// Draw track background when hovered/dragging.
if self.scrollbar.track_hovered || self.scrollbar.dragging {
    let track = self.scrollbar_track_rect(ctx.bounds);
    let track_style =
        RectStyle::filled(s.track_color.with_alpha(0.15)).with_radius(s.thumb_radius);
    ctx.scene.push_quad(track, track_style);
}
```

The track is transparent by default (`track_color: Color::TRANSPARENT`) and only renders when hovered, using `track_color.with_alpha(0.15)`. Since `TRANSPARENT` already has `alpha = 0.0`, the track is invisible even when "drawn."

### Mockup Requirement

The mockup specifies `background: transparent` for the track. The current behavior matches this: no visible track background, only the thumb is visible.

### Verification

The behavior is already correct. No changes needed for track transparency.

When `from_theme()` is used, `track_color` remains `Color::TRANSPARENT`. The hover behavior can optionally show a subtle track by using `theme.surface.with_alpha(0.15)` in the `from_theme()` constructor, but this is a refinement, not a requirement.

If we want a subtle track on hover (which improves scroll affordance):

```rust
// In from_theme():
track_color: theme.border.with_alpha(0.1),
```

This renders a barely-visible track background only when hovering the scrollbar area, which is a common pattern in modern UIs.

---

## 07.4 Tests

### Unit Tests

**File:** `oriterm_ui/src/widgets/scroll/tests.rs`

#### Scrollbar Style from Theme

```rust
#[test]
fn scrollbar_style_from_theme_uses_border_color() {
    let theme = UiTheme::default();
    let style = ScrollbarStyle::from_theme(&theme);

    assert_eq!(style.width, 6.0);
    assert_eq!(style.thumb_color, theme.border);
    assert_eq!(style.thumb_hover_color, theme.text_faint);
    assert_eq!(style.track_color, Color::TRANSPARENT);
}
```

#### Scrollbar Width

```rust
#[test]
fn scrollbar_default_width_is_6px() {
    let style = ScrollbarStyle::default();
    assert!((style.width - 6.0).abs() < f32::EPSILON);
}
```

#### Thumb Hover Detection

```rust
#[test]
fn thumb_hover_detected_separately_from_track() {
    // Create a scroll widget with enough content to show a scrollbar.
    // Move the cursor to the thumb area — verify thumb_hovered is true.
    // Move the cursor to the track (below thumb) — verify thumb_hovered
    // is false but track_hovered is true.
}
```

#### Track Not Rendered When Not Hovered

```rust
#[test]
fn track_not_rendered_when_not_hovered() {
    // Create a scroll widget, render it without hover.
    // Verify no quad is emitted for the track area.
    // Only the thumb quad should be present.
}
```

### Harness Tests

**File:** `oriterm_ui/src/widgets/scroll/tests.rs`

Using `WidgetTestHarness` for end-to-end scrollbar interaction:

```rust
#[test]
fn scrollbar_thumb_changes_color_on_hover() {
    let content = Box::new(/* tall content widget */);
    let scroll = ScrollWidget::vertical(content);
    let mut h = WidgetTestHarness::new(scroll);

    // Render without hover.
    let scene1 = h.render();
    let thumb_quad_1 = find_scrollbar_thumb_quad(&scene1);

    // Move cursor to scrollbar area.
    let scrollbar_x = h.bounds().right() - 5.0;
    let scrollbar_y = h.bounds().y() + 10.0;
    h.mouse_move(Point::new(scrollbar_x, scrollbar_y));

    // Render with hover.
    let scene2 = h.render();
    let thumb_quad_2 = find_scrollbar_thumb_quad(&scene2);

    // Thumb color should differ (hover color is brighter).
    assert_ne!(thumb_quad_1.style.fill, thumb_quad_2.style.fill);
}
```

---

## 07.R Third Party Review Findings

Reserved for findings from `/review-plan` or external review. Not actionable until populated.

---

## 07.5 Build & Verify

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
- [ ] Existing scroll tests still pass (no regressions)
- [ ] Visual verification: scrollbar thumb is `#3a3a3a` at rest, brightens to `#888` on hover
- [ ] Track remains transparent (no visible background when not hovering)
- [ ] Scrollbar width is 6px as expected
