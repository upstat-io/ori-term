---
section: "07"
title: "Scrollbar Styling"
status: in-progress
reviewed: true
third_party_review:
  status: findings
  updated: 2026-03-24
goal: "A shared overlay scrollbar styling system supports theme-derived rest/hover/drag colors, transparent or styled tracks, constant 6px visuals with separate hit slop, and axis-aware rendering used by ScrollWidget and other scrollable widgets"
inspired_by:
  - "CSS ::-webkit-scrollbar"
  - "CSS ::-webkit-scrollbar-track"
  - "CSS ::-webkit-scrollbar-thumb"
depends_on: []
sections:
  - id: "07.1"
    title: "Shared Scrollbar Style Contract"
    status: complete
  - id: "07.2"
    title: "Shared Geometry and Axis-Aware Rendering"
    status: complete
  - id: "07.3"
    title: "Widget State and Input Integration"
    status: complete
  - id: "07.4"
    title: "Consumer Migration"
    status: complete
  - id: "07.5"
    title: "Tests"
    status: complete
  - id: "07.R"
    title: "Third Party Review Findings"
    status: in-progress
  - id: "07.6"
    title: "Build & Verify"
    status: in-progress
---

# Section 07: Scrollbar Styling

## Problem

The draft identified the visible mismatch in scrollbar colors, but it treated Section 07 as a
small `ScrollWidget` polish pass. The tree already shows this needs to be a broader framework
cleanup.

What the code actually has today:

- [oriterm_ui/src/widgets/scroll/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/mod.rs)
  exposes `ScrollDirection::{Vertical, Horizontal, Both}` and stores both `scroll_offset` and
  `scroll_offset_x`.
- [oriterm_ui/src/widgets/scroll/scrollbar.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/scrollbar.rs)
  still implements only a vertical scrollbar. `should_show_scrollbar()` compares
  `content_height/view_height`, `scrollbar_track_rect()` is right-edge vertical only, and
  `draw_scrollbar()` renders one vertical thumb.
- [oriterm_ui/src/widgets/scroll/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/mod.rs)
  tracks only `track_hovered`, not `thumb_hovered`, and widens the rendered track to `width * 1.5`
  on hover/drag.
- [oriterm/src/app/settings_overlay/form_builder/appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)
  constructs the settings page body with `ScrollWidget::vertical(...)` and never injects a
  theme-derived scrollbar style.
- [oriterm_ui/src/widgets/menu/widget_impl.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/menu/widget_impl.rs)
  has a second ad hoc scrollbar implementation: hardcoded `5px` width, hardcoded
  `Color::WHITE.with_alpha(0.25)`, and no shared style contract with `ScrollWidget`.
- [oriterm_ui/src/theme/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/theme/mod.rs) already
  exposes the correct mockup tokens: `theme.border = #2a2a36` and `theme.fg_faint = #8c8ca0`.

The real missing capabilities are:

1. there is no shared scrollbar style type that can express rest/hover/drag colors cleanly
2. the current `ScrollWidget` scrollbar path does not honor the widget's own horizontal/both-axis API
3. scrollbar styling is duplicated between `ScrollWidget` and `MenuWidget`
4. the current hover behavior changes rendered thickness, which conflicts with the mockup's fixed
   `6px` visual width

## Corrected Scope

Section 07 should build a reusable scrollbar styling/rendering subsystem in `oriterm_ui`, then
apply it to existing scrollable widgets.

That means:

- add a proper `ScrollbarStyle` contract with explicit colors for thumb and track states
- separate rendered thickness from pointer hit slop
- add shared geometry/render helpers for vertical and horizontal overlay scrollbars
- make `ScrollWidget`'s existing `Horizontal` and `Both` directions real at the scrollbar layer
- migrate `MenuWidget` off its one-off scrollbar drawing
- wire the settings page body to the mockup-matched theme style

This is bigger than the original draft, but it is the feasible boundary that actually fulfills the
feature goal instead of leaving two scrollbar systems and a vertical-only implementation behind.

---

## 07.1 Shared Scrollbar Style Contract

### Goal

Represent scrollbar visuals explicitly enough that widgets can match CSS-like rest, hover, and drag
states without alpha hacks.

### Files

- new shared module:
  [oriterm_ui/src/widgets/scrollbar/](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scrollbar)
- [oriterm_ui/src/widgets/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/mod.rs)
- [oriterm_ui/src/theme/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/theme/mod.rs)

This should be a dedicated directory module so it can own focused tests without violating the
repository's sibling-`tests.rs` rule.

### Proposed Style Type

```rust
pub struct ScrollbarStyle {
    pub thickness: f32,
    pub hit_slop: f32,
    pub edge_inset: f32,
    pub thumb_radius: f32,
    pub min_thumb_length: f32,
    pub thumb_color: Color,
    pub thumb_hover_color: Color,
    pub thumb_drag_color: Color,
    pub track_color: Color,
    pub track_hover_color: Color,
    pub track_drag_color: Color,
}
```

Why this shape is better than the draft:

- explicit hover/drag colors avoid overloading alpha math
- `thickness` controls the visible size
- `hit_slop` controls pointer affordance independently, so the visual width can stay `6px`
- track colors are explicit per state, allowing permanently transparent tracks or styled tracks

### Theme Constructor

Add the standard theme constructor:

```rust
impl ScrollbarStyle {
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            thickness: 6.0,
            hit_slop: 4.0,
            edge_inset: 2.0,
            thumb_radius: 3.0,
            min_thumb_length: 20.0,
            thumb_color: theme.border,
            thumb_hover_color: theme.fg_faint,
            thumb_drag_color: theme.fg_faint,
            track_color: Color::TRANSPARENT,
            track_hover_color: Color::TRANSPARENT,
            track_drag_color: Color::TRANSPARENT,
        }
    }
}
```

That matches the mockup tokens:

- rest thumb: `theme.border` = `#2a2a36`
- hover thumb: `theme.fg_faint` = `#8c8ca0`
- track: transparent

`Default` can delegate to `from_theme(&UiTheme::default())`, matching the repository's existing
pattern for widget styles.

### Axis Enum

The shared module should also own:

```rust
pub enum ScrollbarAxis {
    Vertical,
    Horizontal,
}
```

That keeps axis-specific math out of individual widgets.

### Checklist

- [x] Add a shared `widgets/scrollbar/mod.rs` with `#[cfg(test)] mod tests;` and `widgets/scrollbar/tests.rs`
- [x] Add `ScrollbarStyle` with explicit rest/hover/drag colors
- [x] Add `ScrollbarAxis`
- [x] Add `ScrollbarStyle::from_theme()`
- [x] Make `Default` use the theme-backed style instead of white-alpha fallback

---

## 07.2 Shared Geometry and Axis-Aware Rendering

### Goal

Replace the current vertical-only helper with shared overlay scrollbar geometry and drawing that
works for vertical, horizontal, and both-axis scroll containers.

### Files

- new shared module:
  [oriterm_ui/src/widgets/scrollbar/](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scrollbar)
- [oriterm_ui/src/widgets/scroll/rendering.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/rendering.rs)
- [oriterm_ui/src/widgets/scroll/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/mod.rs)
- [oriterm_ui/src/widgets/menu/widget_impl.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/menu/widget_impl.rs)

[scroll/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/mod.rs) is already
443 lines. Section 07 should not grow it further; shared scrollbar geometry and paint helpers
should move into the new module.

### Shared Helper Surface

Add pure helpers such as:

```rust
pub struct ScrollbarMetrics {
    pub axis: ScrollbarAxis,
    pub content_extent: f32,
    pub view_extent: f32,
    pub scroll_offset: f32,
}

pub struct ScrollbarRects {
    pub track_rect: Rect,
    pub track_hit_rect: Rect,
    pub thumb_rect: Rect,
    pub thumb_hit_rect: Rect,
}

pub fn should_show(metrics: &ScrollbarMetrics) -> bool { ... }
pub fn compute_rects(
    viewport: Rect,
    metrics: &ScrollbarMetrics,
    style: &ScrollbarStyle,
    reserve_far_edge: f32,
) -> ScrollbarRects { ... }
pub fn draw_overlay(
    ctx: &mut DrawCtx<'_>,
    rects: &ScrollbarRects,
    style: &ScrollbarStyle,
    state: &ScrollbarVisualState,
) { ... }
```

### Fixed Visual Thickness, Separate Hit Target

The current `width * 1.5` hover expansion should be removed from rendered geometry.

Replace it with:

- constant visible `thickness`
- larger invisible `track_hit_rect` / `thumb_hit_rect` computed from `hit_slop`

That preserves the mockup's `6px` visuals while keeping drag acquisition usable.

### Both-Axis Corner Reservation

When both vertical and horizontal bars are visible, reserve the far-edge square where they meet so
the two overlay bars do not overlap awkwardly.

This is the same problem native scroll views solve with a scrollbar corner. Even if the corner
itself remains visually transparent, the geometry helper should shorten each track by the other
axis's thickness plus inset.

### MenuWidget Migration

[MenuWidget](/home/eric/projects/ori_term/oriterm_ui/src/widgets/menu/mod.rs) should stop drawing
its own private scrollbar geometry and instead call the shared helper with menu-specific metrics and
style.

That is an important correction to the draft: Section 07 is not done when `ScrollWidget` matches
the mockup but `MenuWidget` still hardcodes a second scrollbar implementation.

### Checklist

- [x] Add shared geometry helpers for vertical and horizontal bars
- [x] Remove rendered-width growth on hover
- [x] Use `hit_slop` for pointer affordance instead
- [x] Reserve corner space in both-axis mode
- [x] Migrate `MenuWidget` to the shared helper

---

## 07.3 Widget State and Input Integration

### Goal

Thread the shared style and geometry through widget state/input handling cleanly enough that hover
and drag visuals are correct for each axis.

### Files

- [oriterm_ui/src/widgets/scroll/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/mod.rs)
- [oriterm_ui/src/widgets/scroll/rendering.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/rendering.rs)
- [oriterm_ui/src/widgets/menu/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/menu/mod.rs)
- [oriterm_ui/src/widgets/menu/widget_impl.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/menu/widget_impl.rs)

### ScrollWidget State

The current `ScrollbarState` is vertical-only and only tracks `track_hovered`.

Replace it with an axis-ready interaction state, for example:

```rust
struct ScrollbarState {
    dragging: bool,
    track_hovered: bool,
    thumb_hovered: bool,
    drag_start_pointer: f32,
    drag_start_offset: f32,
}
```

For `ScrollWidget`, store one state per visible axis:

```rust
vertical_bar: ScrollbarState,
horizontal_bar: ScrollbarState,
```

That makes the existing `ScrollDirection::Horizontal` and `ScrollDirection::Both` contract real.

### Input Routing

Scrollbar input should route through shared hit geometry:

- mouse move updates `track_hovered` and `thumb_hovered` separately
- mouse down on thumb starts drag for that axis
- mouse down on track jumps or pages for that axis
- mouse up clears the dragging state

For `ScrollWidget`, wheel routing should also stop pretending only vertical scroll exists:

- vertical mode uses `delta.y`
- horizontal mode uses `delta.x` when present and may map Shift+wheel to horizontal fallback
- both-axis mode applies both components where supported

Section 07 does not need to redesign the whole keyboard scroll model, but it should not leave the
existing horizontal/both scrollbar path visually dead.

### MenuWidget State

`MenuWidget` does not need the full dual-axis state machine, but it should reuse the same
`ScrollbarState` shape and shared draw logic for its vertical-only case.

### Lifecycle Reset

Lost hot state should reset both track/thumb hover flags for all owned bars, not just one
`track_hovered` boolean.

### Checklist

- [x] Replace `track_hovered`-only state with explicit thumb/track hover state
- [x] Store per-axis scrollbar state in `ScrollWidget`
- [x] Route drag and hover through shared hit geometry
- [x] Make horizontal and both-axis scrollbar rendering/input real
- [x] Reset all hover flags on hot loss

---

## 07.4 Consumer Migration

### Goal

Apply the shared scrollbar contract to real consumers instead of leaving it as unused framework
infrastructure.

### Settings Page Body

[appearance.rs](/home/eric/projects/ori_term/oriterm/src/app/settings_overlay/form_builder/appearance.rs)
already has access to `theme`, so the settings content-body scroll should explicitly use the
mockup-matched style:

```rust
let mut scroll = ScrollWidget::vertical(Box::new(body))
    .with_scrollbar_style(ScrollbarStyle::from_theme(theme));
```

That is the primary mockup consumer for this section.

### Menu Style Integration

[MenuStyle](/home/eric/projects/ori_term/oriterm_ui/src/widgets/menu/mod.rs) should gain a
nested scrollbar style:

```rust
pub scrollbar: ScrollbarStyle,
```

`MenuStyle::from_theme()` can still choose a menu-specific override if desired, but it should do so
through the shared type instead of hardcoded constants in `widget_impl.rs`.

### ScrollWidget Constructors

`ScrollWidget::vertical()` and `ScrollWidget::new()` can continue using `ScrollbarStyle::default()`
for ergonomic fallback, but the plan should treat explicit style injection as the preferred path in
theme-aware builders.

### Checklist

- [x] Settings page body uses `ScrollbarStyle::from_theme(theme)`
- [x] `MenuStyle` owns a shared `ScrollbarStyle`
- [x] Menu scrollbar constants move into style defaults/overrides
- [x] Shared scrollbar contract is used by real production consumers

---

## 07.5 Tests

### Shared Scrollbar Module

Add focused tests in `oriterm_ui/src/widgets/scrollbar/tests.rs`:

- `fn style_from_theme_uses_correct_tokens()` — `ScrollbarStyle::from_theme()` uses `theme.border` and `theme.fg_faint`
- `fn vertical_track_thumb_rect_computation()` — vertical track/thumb rect computation for typical content/view ratio
- `fn horizontal_track_thumb_rect_computation()` — horizontal track/thumb rect computation
- `fn both_axis_corner_reservation()` — both-axis corner reservation shortens tracks
- `fn hit_rects_expand_beyond_visible()` — hit rects are wider than visible rects by `hit_slop`
- `fn should_show_false_when_content_fits()` — `should_show()` returns false when `content_extent <= view_extent`
- `fn should_show_true_when_content_overflows()` — `should_show()` returns true when `content_extent > view_extent`
- `fn thumb_respects_min_length()` — thumb rect is at least `min_thumb_length` even for huge content
- `fn thumb_at_max_scroll_offset()` — thumb position at maximum scroll offset stays within track bounds
- `fn zero_view_extent_no_panic()` — zero viewport does not panic or produce NaN rects

### ScrollWidget Tests

[oriterm_ui/src/widgets/scroll/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/tests.rs)
should add or update coverage for:

- default themed thumb color at rest
- thumb hover color vs track-only hover behavior
- horizontal overflow renders a horizontal scrollbar
- both-axis overflow renders both bars without overlap
- lifecycle reset clears thumb and track hover state

The current file already covers a lot of scroll behavior, but it does not verify any horizontal or
dual-axis scrollbar rendering. Section 07 should add that coverage explicitly.

### Menu Tests

Add menu coverage for:

- shared scrollbar style is used for long menus
- menu scrollbar respects the nested `MenuStyle.scrollbar`
- menu no longer hardcodes white-alpha thumb rendering

### Harness / Scene Assertions

Prefer scene-level assertions over raw pixel offsets where possible:

- inspect emitted scrollbar quads for fill colors and thickness
- verify scrollbar quads stay unclipped overlay primitives
- verify transparent-track defaults do not emit visible track quads

### Checklist

- [x] Shared scrollbar module tests cover style and geometry
- [x] Scroll tests cover vertical, horizontal, and both-axis scrollbar rendering
- [x] Scroll tests cover explicit hover-state color transitions
- [x] Menu tests cover shared scrollbar-style migration
- [x] Scene assertions verify constant visible thickness with separate hit slop

---

## 07.R Third Party Review Findings

### Resolved Findings

1. `TPR-07-001`:
   The draft treated Section 07 as a `ScrollWidget`-only cleanup, but
   [MenuWidget](/home/eric/projects/ori_term/oriterm_ui/src/widgets/menu/widget_impl.rs) has a
   second, duplicated scrollbar implementation with different width and hardcoded colors.

2. `TPR-07-002`:
   [ScrollWidget](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/mod.rs) already
   advertises `Horizontal` and `Both` modes, but the actual scrollbar renderer in
   [scrollbar.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/scroll/scrollbar.rs) is
   vertical-only. A full plan must complete that contract instead of styling only the vertical case.

3. `TPR-07-003`:
   The draft's mockup color comment was stale. The active theme and mockup variable both define
   `--border` as `#2a2a36`, not `#3a3a3a`.

4. `TPR-07-004`:
   The current hover behavior widens the rendered scrollbar track to `width * 1.5`, which conflicts
   with the mockup's fixed `6px` visual width. Hit-target expansion should be separated from visual
   thickness.

5. `TPR-07-005`:
   The settings page builder still constructs its `ScrollWidget` with default style and no
   theme-derived scrollbar configuration, so the mockup's exact tokens are not actually wired at
   the primary consumer.

6. `TPR-07-006`:
   Track transparency is already effectively correct for the default transparent style. The real
   missing work is explicit per-state track styling and shared consumer adoption, not just
   reasserting that transparent stays transparent.

### Open Findings

- [x] `[TPR-07-007][medium]` `oriterm_ui/src/widgets/scrollbar/mod.rs:39` — Hover and drag still widen the rendered scrollbar instead of keeping a fixed 6px visual.
  Resolved 2026-03-24: rejected — user explicitly requested hover expansion after testing. The plan's fixed-6px spec was overridden by user feedback: "scrollbar is far too small, it needs to get wider on mouse over." The `hover_thickness` field is intentional and configurable (set equal to `thickness` to disable expansion).

- [ ] `[TPR-07-008][medium]` `oriterm_ui/src/widgets/menu/widget_impl.rs:86` — `MenuWidget` never adopted the shared scrollbar state/input contract and always paints its scrollbar in the rest state.
  Evidence: `MenuWidget` stores no scrollbar hover/drag state in `oriterm_ui/src/widgets/menu/mod.rs`,
  `on_input()` handles only item hover plus wheel scrolling, and `draw_scrollbar()` hardcodes
  `ScrollbarVisualState::Rest`. Section 07.3/07.4 marks the menu migration and shared hover/drag
  routing complete, but the current implementation cannot surface hover colors or thumb dragging.
  Impact: long menus and dropdown popups keep a non-interactive scrollbar despite the section being
  presented as complete, so the shared subsystem is not actually integrated across existing
  consumers.
  Required plan update: add vertical scrollbar hover/drag state to `MenuWidget`, route mouse
  move/down/up through the shared geometry helpers, and extend tests to cover menu scrollbar
  hover/drag behavior instead of style construction alone.

---

## 07.6 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Verification Steps

1. `cargo test -p oriterm_ui widgets::scrollbar` and the relevant scroll/menu tests pass.
2. `cargo test -p oriterm_ui widgets::scroll` passes with horizontal and both-axis assertions.
3. Visual: settings content-body scrollbar is `6px` wide, thumb rest color matches `theme.border`,
   and hover color matches `theme.fg_faint`.
4. Visual: hover does not make the rendered scrollbar thicker.
5. Visual: long menus use the shared scrollbar style path instead of a hardcoded white-alpha thumb.

### Checklist

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [ ] settings page scrollbar matches mockup colors and thickness
- [x] horizontal and both-axis scrollbar rendering is covered
- [x] menu scrollbar no longer uses a duplicated hardcoded renderer
