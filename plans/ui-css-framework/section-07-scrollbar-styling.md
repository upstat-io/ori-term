---
section: "07"
title: "Scrollbar Styling"
status: in-progress
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-25
goal: "A shared overlay scrollbar styling system supports theme-derived rest/hover/drag colors, transparent or styled tracks, configurable rest/hover thickness with separate hit slop, and axis-aware rendering used by ScrollWidget and other scrollable widgets"
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
    status: complete
  - id: "07.7"
    title: "ScrollWidget Controller Migration"
    status: in-progress
  - id: "07.6"
    title: "Build & Verify"
    status: complete
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

### Configurable Visual Thickness, Separate Hit Target

The old `width * 1.5` hover expansion was replaced with a configurable `hover_thickness` field
(user-approved override of original fixed-6px spec — see TPR-07-007).

The implementation provides:

- `thickness` for the rest-state rendered width (`6px` default)
- `hover_thickness` for the hovered/dragging rendered width (configurable, default wider than rest)
- larger invisible `track_hit_rect` / `thumb_hit_rect` computed from `hit_slop`

Set `hover_thickness == thickness` to disable hover expansion entirely.

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

- [x] `[TPR-07-014][medium]` `oriterm_ui/src/widgets/scroll/mod.rs:313` — `ScrollWidget` still handles scrollbar hover/drag/capture through raw `on_input()` mouse branches instead of the controller pipeline required by `CLAUDE.md`.
  Evidence: `ScrollWidget` processes `MouseDown`/`MouseMove`/`MouseUp` directly in `on_input()` and owns manual drag/hover state (`v_bar`/`h_bar`) even though the repo rule explicitly says scroll thumbs must go through event controllers, not raw event methods.
  Impact: Section 07 is marked complete while the primary settings-page scrollbar remains a framework exception, so hover/active behavior is not unified with the rest of the UI system.
  Resolved 2026-03-25: accepted. Concrete implementation tasks added as §07.7 "ScrollWidget Controller Migration." The scrollbar overlay is not in the hit-test tree (it exists only in paint), so a custom Capture-phase controller with shared geometry is needed. Standard ScrubController in Bubble/Target phase would not fire for scrollbar clicks because the child widget is always the hit-test target.

- [x] `[TPR-07-015][low]` `oriterm_ui/src/widgets/menu/widget_impl.rs:142` — Wheel scrolling while the cursor is over the menu scrollbar repopulates `hovered` with the row behind the bar, so menu hover feedback can disagree with pointer location.
  Resolved 2026-03-25: accepted and fixed. Added `if !self.scrollbar_state.track_hovered` guard in the `Scroll` branch of `on_input()`, matching the existing guard in the `MouseMove` path. Added `scroll_wheel_over_scrollbar_keeps_hover_clear` regression test.

- [x] `[TPR-07-012][high]` `oriterm_ui/src/widgets/menu/widget_impl.rs:195` — `MenuWidget` item selection still depends on stale hover state, so a direct click can no-op or select the wrong entry.
  Resolved 2026-03-24: accepted and fixed. `handle_drag_start()` now updates `self.hovered` from the press position when mode is `ItemPress`, so a click without prior `MouseMove` selects the correct item. Added `item_click_without_prior_mouse_move` unit test.

- [x] `[TPR-07-013][low]` `plans/ui-css-framework/section-07-scrollbar-styling.md:9` — Section 07 still describes a fixed-width/no-hover-growth scrollbar even though the accepted behavior now expands on hover.
  Resolved 2026-03-24: accepted and fixed. Updated section goal, verification steps, and §07.2 body text to reflect the configurable `hover_thickness` contract (user-approved override of original fixed-6px spec via TPR-07-007).

- [x] `[TPR-07-007][medium]` `oriterm_ui/src/widgets/scrollbar/mod.rs:39` — Hover and drag still widen the rendered scrollbar instead of keeping a fixed 6px visual.
  Resolved 2026-03-24: rejected — user explicitly requested hover expansion after testing. The plan's fixed-6px spec was overridden by user feedback: "scrollbar is far too small, it needs to get wider on mouse over." The `hover_thickness` field is intentional and configurable (set equal to `thickness` to disable expansion).

- [x] `[TPR-07-008][medium]` `oriterm_ui/src/widgets/menu/widget_impl.rs:86` — `MenuWidget` never adopted the shared scrollbar state/input contract and always paints its scrollbar in the rest state.
  Resolved 2026-03-24: accepted and fixed. Added `MenuScrollbarState` to `MenuWidget` with drag/hover tracking, routed `MouseDown`/`MouseMove`/`MouseUp` through the shared `compute_rects`/`pointer_to_offset`/`drag_delta_to_offset` helpers, `draw_scrollbar()` now uses `scrollbar_state.visual_state()`, added `lifecycle()` for hover reset on hot loss, and added 8 scrollbar interaction tests covering hover, drag, track click, visual state transitions, hot loss, and non-scrollable menu handling.

- [x] `[TPR-07-009][high]` `oriterm_ui/src/widgets/menu/mod.rs:207` — `MenuWidget`'s scrollbar mouse-down/up path is dead under the real event pipeline, so menu scrollbars still cannot be clicked or dragged in production.
  Resolved 2026-03-24: accepted and fixed. Removed `ClickController` from `MenuWidget` entirely — menus don't need multi-click detection. All input (item press/release, scrollbar drag, track click) now handled directly in `on_input()`, which is the production path since no controllers block it. Added `press_pos` field for item click detection (press → capture → release → `Selected` action). Added 3 item-click unit tests (`item_click_emits_selected`, `click_on_separator_does_not_select`, `release_outside_menu_does_not_select`) and 3 WidgetTestHarness integration tests (`harness_item_click_produces_selected`, `harness_scrollbar_drag_captures_and_releases`, `harness_scrollbar_track_click_does_not_capture`) that exercise the full propagation pipeline.

- [x] `[TPR-07-010][medium]` `oriterm_ui/src/widgets/menu/widget_impl.rs:54` — The menu scrollbar fix reintroduces a manual input path instead of using the repository’s required controller-driven interaction pipeline.
  Resolved 2026-03-24: accepted and fixed. Added `HoverController` + `ScrubController` to `MenuWidget` (controllers field, initialized in `new()`). Press/drag input now flows through `ScrubController` → `on_action()` with zone discrimination via `DragMode` enum (`ScrollbarThumb`, `ScrollbarTrack`, `ItemPress`). Removed `press_pos` field. `on_input()` now only handles idle `MouseMove` (item/scrollbar hover) and `Scroll` (wheel). Controllers return actual controller slices instead of `&[]`. Added `menu_has_controllers` and `menu_sense_includes_drag` tests; updated all interaction tests to use `on_action()` for press/drag/release. Harness tests confirm full pipeline integration (capture/release via `ScrubController`).

- [x] `[TPR-07-011][low]` `oriterm_ui/src/widgets/scrollbar/mod.rs:268` — Hover expansion infers scrollbar axis from the rect’s aspect ratio, so square thumbs/tracks expand in the wrong direction.
  Resolved 2026-03-24: accepted and fixed. Added `axis: ScrollbarAxis` field to `ScrollbarRects` (set by `compute_rects()`). `expand_rect_inward()` now takes an explicit `ScrollbarAxis` parameter instead of inferring from aspect ratio. Added 3 regression tests: `hover_expansion_uses_axis_not_aspect_ratio` (vertical square thumb), `hover_expansion_horizontal_square_thumb`, and `compute_rects_stores_axis`.

---

## 07.7 ScrollWidget Controller Migration

### Goal

Migrate `ScrollWidget`'s scrollbar interaction from raw `on_input()` mouse handling to the
controller + action pipeline required by `CLAUDE.md` for scroll thumbs.

### Design Constraint

The scrollbar overlay is not part of the layout/hit-test tree — it exists only in paint. When the
user clicks on the scrollbar region, the hit-test path finds the child widget (which fills the full
viewport), not the scrollbar. A standard Bubble/Target-phase ScrubController on ScrollWidget would
never fire for scrollbar clicks because the child is always the deepest hit target.

### Approach: Capture-Phase Custom Controller

Create a `ScrollbarCaptureController` that runs in Capture phase (parent before child):

1. The controller checks if the mousedown position is in the scrollbar hit rect
2. If YES → emit `DragStart`, return `handled=true`, capture pointer (child never sees the event)
3. If NO → return `handled=false` (event propagates to child normally)
4. During drag → emit `DragUpdate`/`DragEnd` as usual

Geometry sharing: the widget and controller share scrollbar hit rects via
`Rc<RefCell<ScrollbarHitRects>>`. The widget updates this after computing rects (in the input/paint
helpers). The controller reads it during event handling.

### Files

- new: `oriterm_ui/src/controllers/scrollbar_capture.rs`
- modify: `oriterm_ui/src/controllers/mod.rs`
- modify: `oriterm_ui/src/widgets/scroll/mod.rs`
- modify: `oriterm_ui/src/widgets/scroll/input.rs`

### Checklist

- [x] Create `ScrollbarCaptureController` in `oriterm_ui/src/controllers/scrollbar_capture/`
- [x] Controller uses Capture phase with shared `Rc<RefCell<ScrollbarHitZones>>`
- [x] Controller emits `DragStart`/`DragUpdate`/`DragEnd` for scrollbar thumb and track clicks
- [x] Controller returns `handled=false` for clicks outside scrollbar (child gets the event)
- [x] ScrollWidget adds `controllers()` and `controllers_mut()` returning the capture controller
- [x] ScrollWidget's `on_action()` handles `DragStart`/`DragUpdate`/`DragEnd` for scrollbar state
- [x] Remove scrollbar MouseDown/MouseMove/MouseUp handling from `on_input()` (keep Scroll + keys + hover)
- [ ] Add harness tests verifying scrollbar drag works through the full propagation pipeline
- [ ] Add harness test verifying child widgets still receive clicks in non-scrollbar areas
- [x] Existing scroll tests continue to pass (1813 tests pass)

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
3. Visual: settings content-body scrollbar is `6px` at rest, expands to `hover_thickness` on hover,
   thumb rest color matches `theme.border`, and hover color matches `theme.fg_faint`.
4. Visual: hover expansion is configurable via `ScrollbarStyle::hover_thickness` (set equal to
   `thickness` to disable expansion). User-approved override of original fixed-6px spec (TPR-07-007).
5. Visual: long menus use the shared scrollbar style path instead of a hardcoded white-alpha thumb.

### Checklist

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [x] settings page scrollbar matches mockup colors and thickness
- [x] horizontal and both-axis scrollbar rendering is covered
- [x] menu scrollbar no longer uses a duplicated hardcoded renderer
