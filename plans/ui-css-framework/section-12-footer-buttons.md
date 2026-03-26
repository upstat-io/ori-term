---
section: "12"
title: "Visual Fidelity: Footer + Buttons"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-25
goal: "The settings footer matches the mockup structurally and visually: it lives only in the right content column, the unsaved group and Reset/Cancel/Save button cluster are laid out correctly, the shared button primitive can express the required typography and disabled state, and footer dirty-state behavior stays synchronized with the real settings pipeline"
depends_on: ["01", "02", "03", "05", "06", "08", "10"]
sections:
  - id: "12.1"
    title: "Shared Button Typography + States"
    status: not-started
  - id: "12.2"
    title: "Right Column Footer Structure"
    status: not-started
  - id: "12.3"
    title: "Unsaved Indicator + Dirty State"
    status: not-started
  - id: "12.4"
    title: "Semantic Actions + Tests"
    status: not-started
  - id: "12.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "12.5"
    title: "Build & Verify"
    status: not-started
---

# Section 12: Visual Fidelity - Footer + Buttons

## Problem

The draft framed Section 12 as a footer styling pass, but the current implementation has deeper
layout and state-model problems.

What the tree actually shows today:

- `SettingsPanel` currently appends a full-width footer row below the entire settings content.
  That means the sidebar stops above the footer, which does not match the mockup's full-height
  sidebar.
- `SettingsPanel::paint()` then draws an opaque footer background hack across the full panel width,
  including the sidebar area (lines 395-401 of `settings_panel/mod.rs`).
- The mockup footer HTML is:
  - `footer-left` unsaved group (with `margin-right: auto`)
  - `Reset to Defaults`
  - `Cancel`
  - `Save`
  with the left group consuming the `margin-right: auto` slot and the buttons right-aligned via
  `justify-content: flex-end` + `gap: 8px`.
- The current footer layout is:
  - `Reset`
  - fill spacer
  - `Cancel`
  - fixed `8px`
  - `Save`
  and the unsaved indicator is painted as an overlay, not part of layout.
- Because the unsaved indicator is painted at the same left inset where the Reset button is laid
  out, the current implementation can overlap the Reset button.
- The current unsaved indicator is text only. The mockup requires a `14px` alert-circle icon, `6px`
  icon/text gap, uppercase tracked text, and left-group layout.
- The current button primitive cannot express the full footer typography:
  - no font weight in `ButtonStyle`
  - no letter spacing in `ButtonStyle`
  - no button-level text-transform support
  - no correct disabled-state border/opacity handling for `.btn-primary:disabled`
- The draft also got one mockup fact wrong: `.btn-primary` is `font-weight: 700`, not `500`.
  The base `.btn` class uses `font-weight: 500`; `.btn-primary` overrides to `700`.
- **Note (verified):** The dirty-state sync after `ResetDefaults` is NOT a current bug.
  `content_actions.rs:195` already sends `SettingsUnsaved(dirty)` to the rebuilt panel after reset.
  However, the footer widget extraction in 12.2 must preserve this behavior — the new footer widget
  must handle `SettingsUnsaved` in its own `accept_action()` so the indicator and Save button state
  update correctly.
- `oriterm_ui/src/widgets/settings_panel/mod.rs` is already `488` lines, so adding more footer
  logic there is not a maintainable path.

Section 12 therefore needs a structural rewrite, not just more `ButtonStyle` fields and visual
verification.

## Corrected Scope

Section 12 should keep the full mockup goal and implement it at the right boundaries:

1. extend the shared button primitive so footer buttons can match the mockup exactly (12.1)
2. move footer ownership out of the full-panel bottom bar and into the right content column (12.2)
3. make the unsaved group a real layout participant instead of paint-time overlay text (12.3)
4. keep footer dirty-state and semantic button actions synchronized with the real dialog pipeline (12.4)

12.1 (button typography) must come before 12.2 (footer structure) because the footer widget
composes `ButtonWidget` instances with the new style fields. Building the footer first would require
placeholder style code that gets rewritten immediately in 12.2.

This section should not preserve the current full-panel footer hack and try to patch around it with
more manual paint math.

**Dependency note:** Section 06 (Opacity + Display Control) is required for the `.btn-primary:disabled`
opacity treatment. Added to `depends_on`.

---


## 12.1 Shared Button Typography + States

### Goal

Extend the shared button primitive so the settings footer buttons match the mockup's typography and
disabled behavior without turning footer code into a one-off paint fork. This must land before the
footer widget is created (12.2) because the footer composes `ButtonWidget` with the new style fields.

### Files

- `oriterm_ui/src/widgets/button/mod.rs` (314 lines)
- `oriterm_ui/src/widgets/button/tests.rs`
- `mockups/settings-brutal.html`

### Mockup Facts

Common `.btn` typography:

- font size `12px`
- uppercase (`text-transform: uppercase`)
- letter spacing `0.04em` = `0.48px` at 12px
- padding `6px 16px`
- border width `2px`
- base weight `500`

Variant details:

- `btn-danger-ghost`
  - text `fg_secondary` at rest
  - danger border/text/background on hover
  - weight `500` (inherits base `.btn`)
- `btn-ghost`
  - text `fg_secondary` at rest
  - `border_strong` + bright text on hover
  - weight `500` (inherits base `.btn`)
- `btn-primary`
  - accent bg/border
  - dark text (`#0e0e12`)
  - `accent_hover` bg/border on hover
  - weight `700` (overrides base `.btn`)
  - disabled: `opacity: 0.4` on the entire button (not just fg/bg swap)

### Current Shared Primitive Gap

`ButtonStyle` (defined in `oriterm_ui/src/widgets/button/mod.rs:26-55`) currently has these fields:
`fg`, `hover_fg`, `bg`, `hover_bg`, `pressed_bg`, `border_color`, `hover_border_color`,
`border_width`, `corner_radius`, `padding`, `font_size`, `disabled_fg`, `disabled_bg`,
`focus_ring_color`.

Missing for footer buttons:

- **`weight: FontWeight`** — needed for 500 vs 700 distinction.
- **`letter_spacing: f32`** — needed for `0.48px` tracking.
- **`text_transform: TextTransform`** — Section 03 is complete; `TextTransform::Uppercase` is
  available on `TextStyle`. Adding it to `ButtonStyle` lets the button apply the transform through
  `text_style()` instead of requiring callers to pre-uppercase labels.
- **`disabled_opacity: f32`** — the mockup's `.btn-primary:disabled` applies `opacity: 0.4` to
  the entire button (bg, border, text all fade). The current `disabled_fg` + `disabled_bg` swap
  cannot express this. A single `disabled_opacity` field (default `1.0`) that modulates all
  channels in the disabled paint path is the simplest correct approach.
- **`disabled_border_color`** — NOT needed. The mockup's `.btn-primary:disabled` applies
  `opacity: 0.4` to the entire button. `disabled_opacity` modulates bg, border, AND fg alpha
  uniformly, which is the correct CSS `opacity` semantic. A separate border color field would
  conflict with the opacity approach and add unnecessary API surface.

### Required Changes to `ButtonWidget`

1. Add `weight`, `letter_spacing`, `text_transform`, `disabled_opacity` fields to `ButtonStyle`.
   Update `from_theme()` defaults: `weight: FontWeight::NORMAL`, `letter_spacing: 0.0`,
   `text_transform: TextTransform::None`, `disabled_opacity: 1.0`.

2. Update `ButtonWidget::text_style()` (line 189-191) to thread `weight`, `letter_spacing`, and
   `text_transform` into the returned `TextStyle`:
   ```rust
   fn text_style(&self) -> TextStyle {
       TextStyle {
           size: self.style.font_size,
           color: self.current_fg(),
           weight: self.style.weight,
           letter_spacing: self.style.letter_spacing,
           text_transform: self.style.text_transform,
           ..TextStyle::default()
       }
   }
   ```

3. Update `ButtonWidget::paint()` disabled path: when `self.disabled` AND
   `self.style.disabled_opacity < 1.0`, multiply bg, border, AND fg alpha by
   `self.style.disabled_opacity`. This matches CSS `opacity: 0.4` semantics (entire element fades).
   When `disabled_opacity == 1.0` (default), fall back to the existing `disabled_fg` / `disabled_bg`
   swap behavior for backward compatibility.

4. Update `ButtonWidget::current_fg()`: when `disabled_opacity < 1.0`, return `self.style.fg` with
   alpha multiplied by `disabled_opacity` (the full-opacity modulation path). When
   `disabled_opacity == 1.0`, use the existing `disabled_fg` swap (for non-primary buttons that
   swap colors instead of fading).

5. Update all existing `ButtonStyle` construction sites (currently 4 in `settings_panel/mod.rs`
   lines 146, 206, 222, 238) to include the new fields via `..ButtonStyle::from_theme(theme)`.
   The existing pre-uppercased labels (`"RESET TO DEFAULTS"`, `"CANCEL"`, `"SAVE"`) can switch
   to mixed-case + `text_transform: TextTransform::Uppercase` here. Also fix the Save button's
   horizontal padding from `20.0` to `16.0` to match the mockup's `.btn { padding: 6px 16px }`
   (all three footer buttons use the same padding).

### Checklist

- [ ] Add `weight: FontWeight` to `ButtonStyle`, default `FontWeight::NORMAL`
- [ ] Add `letter_spacing: f32` to `ButtonStyle`, default `0.0`
- [ ] Add `text_transform: TextTransform` to `ButtonStyle`, default `TextTransform::None`
- [ ] Add `disabled_opacity: f32` to `ButtonStyle`, default `1.0`
- [ ] Thread new fields through `ButtonWidget::text_style()` into `TextStyle`
- [ ] Apply `disabled_opacity` to bg/border/fg in the disabled paint path (CSS `opacity` semantics)
- [ ] When `disabled_opacity == 1.0`, preserve existing `disabled_fg`/`disabled_bg` swap for backward compatibility
- [ ] Update existing `ButtonStyle` construction sites in `settings_panel/mod.rs` to use new fields
- [ ] Update `button/tests.rs::with_style_applies_custom_style` (line 119) — test constructs
      `ButtonStyle` with all fields explicitly (no `..Default`); must add the new fields or
      switch to `..ButtonStyle::default()` for non-essential fields to avoid compile error
- [ ] Convert pre-uppercased button labels to mixed-case + `TextTransform::Uppercase`
- [ ] Fix Save button horizontal padding from `20.0` to `16.0` (all `.btn` use `6px 16px`)
- [ ] Keep `button/mod.rs` under 500 lines (currently 314; expect ~340 after changes)
- [ ] Verify `layout()` produces correct intrinsic width when `text_transform` is set (transform
      flows through `text_style()` -> `measure()` -> `MockMeasurer` which applies the transform)
- [ ] Add test: `button_disabled_fg_swap_when_no_opacity()` — when `disabled_opacity == 1.0` (default),
      disabled button still uses `disabled_fg`/`disabled_bg` swap, NOT opacity modulation. This guards
      the backward-compat branch.

---

## 12.2 Right Column Footer Structure

### Goal

Make the footer live only in the right content column so the sidebar remains full-height and the
footer layout matches the mockup's actual DOM structure.

### Files

- `oriterm_ui/src/widgets/settings_panel/mod.rs` (488 lines)
- `oriterm/src/app/settings_overlay/form_builder/mod.rs` (223 lines)
- new: `oriterm_ui/src/widgets/settings_footer/mod.rs`
- new: `oriterm_ui/src/widgets/settings_footer/tests.rs`

### Current Structural Mismatch

The mockup's footer belongs to the right pane, not the whole panel.

Current tree (from `SettingsPanel::build()` lines 130-173):

```text
SettingsPanel
  ContainerWidget::column
    [header row + header_sep]   (only in overlay mode)
    body row (sidebar + pages)  (SizeSpec::Fill — takes remaining height)
    footer_sep                  (separator with SIDEBAR_WIDTH left padding)
    footer row                  (buttons with SIDEBAR_WIDTH + 28 left padding)
```

Mockup structure (from `.settings-window` DOM):

```text
.settings-window
  nav.sidebar                   (full height, independent column)
  div.main                      (right column, flex-direction: column)
    div.page                    (flex: 1, scrollable content)
    div.footer                  (flex-shrink: 0, pinned bottom)
```

The current structure is why the sidebar does not extend to the bottom of the panel and why the
footer background has to be overpainted manually (`settings_panel/mod.rs` lines 395-401).

### Required Structure


Introduce a dedicated `SettingsFooterWidget` in `oriterm_ui` and compose it inside the right
content column. The composition happens in `build_settings_dialog()` in `form_builder/mod.rs`,
NOT inside `SettingsPanel`.

```text
SettingsPanel
  ContainerWidget::column
    [header row + header_sep]   (only in overlay mode)
    content row (SizeSpec::Fill)
      sidebar (SizeSpec::Fixed(200))
      right column (SizeSpec::Fill, Direction::Column)
        page container (SizeSpec::Fill, clipped)
        SettingsFooterWidget (SizeSpec::Fixed(FOOTER_HEIGHT))
```

### `IdOverrideButton` Accessibility

`IdOverrideButton` is currently `pub(super)` in `settings_panel/id_override_button.rs`, making it
visible only within `settings_panel`. The new `SettingsFooterWidget` needs it too. Two options:

1. **Move to `oriterm_ui/src/widgets/button/id_override.rs`** as `pub(crate)`. This is the cleaner
   option — `IdOverrideButton` is a general-purpose button utility, not settings-panel-specific.
   Update `settings_panel/mod.rs` to import from `super::button::id_override::IdOverrideButton`.
2. **Widen visibility to `pub(crate)`** in place. Quicker but leaves it in a misleading location.

Option 1 is preferred. The move should happen as part of 12.2 since it's needed before the footer
widget can be created.

Additionally, `IdOverrideButton` currently has no `set_disabled()` method. The footer needs to
toggle the Save button's disabled state when dirty state changes. Add a
`pub(crate) fn set_disabled(&mut self, disabled: bool)` method that delegates to
`self.inner.set_disabled(disabled)`. This is needed for 12.3's `accept_action` handler.

### SettingsPanel API Change

`SettingsPanel::build()` currently calls `Self::build_footer()` and appends `footer_sep` + `footer`
to its internal container. After this change, `build()` must NOT create any footer children —
the `content` parameter passed to `SettingsPanel` must already be the right-column widget
(containing pages + footer). The `SettingsPanel` simply wraps it in a row with optional header
chrome.

The `save_id`, `cancel_id`, `reset_id` fields **stay on `SettingsPanel`** but are no longer
allocated there. Instead, the constructor signature changes to accept the IDs from outside:

```rust
pub fn new(content: Box<dyn Widget>, footer_ids: (WidgetId, WidgetId, WidgetId), theme: &UiTheme) -> Self
pub fn embedded(content: Box<dyn Widget>, footer_ids: (WidgetId, WidgetId, WidgetId), theme: &UiTheme) -> Self
```

Where `footer_ids = (reset_id, cancel_id, save_id)` — read from
`SettingsFooterWidget::button_ids()` before the footer is boxed.

`SettingsPanel::on_action()` keeps its existing translation logic (`Clicked(save_id) ->
SaveSettings`, etc.) using the passed-in IDs. This is necessary because the app's event
dispatch calls `on_action()` only on the root content widget (SettingsPanel), not on nested
widgets (see "Action Propagation After Extraction" below).

`SettingsIds` in `form_builder/mod.rs` does NOT need these IDs — they are not used for
dispatch in `handle_dialog_content_action()`.

`FOOTER_HEIGHT` (currently `52.0` in `settings_panel/mod.rs`) must move to
`settings_footer/mod.rs` as `pub(crate) const FOOTER_HEIGHT: f32 = 52.0;`.

### Ownership Changes

**`SettingsPanel`** loses:
- `build_footer()` method (lines 189-268)
- `unsaved` field (move to footer widget)
- `accept_action()` handling of `SettingsUnsaved` (move to footer widget's `accept_action()`)
- Footer background overpaint hack in `paint()` (lines 384-403)

**`SettingsPanel`** keeps:
- `save_id`, `cancel_id`, `reset_id` fields (now passed in via constructor, not allocated)
- `close_id` field (for the overlay-mode close button)
- `on_action()` for ALL button translations (`Clicked(close_id)` -> `CancelSettings`,
  `Clicked(save_id)` -> `SaveSettings`, etc.) — required because the app calls `on_action()`
  only on the root content widget
- Panel chrome (shadow, border, rounded corners)
- Header bar (overlay mode only)
- Layout cache

**`SettingsFooterWidget`** owns (as named typed fields, NOT Box<dyn Widget> children):
- `reset_button: IdOverrideButton`
- `cancel_button: IdOverrideButton`
- `save_button: IdOverrideButton`
- `unsaved_visibility: VisibilityWidget` (wraps a ContainerWidget row with icon + label)
- `dirty: bool`
- Separator (2px top border, drawn in `paint()`)
- `on_action()` — passthrough only (returns `Some(action)` unchanged). The footer's
  `on_action()` is never called by the framework in production because `on_action()` is
  single-hop (called only on the root content widget by the app layer). Button-to-semantic
  translation lives in `SettingsPanel::on_action()`.
- `accept_action()` handles `SettingsUnsaved(dirty)` — updates indicator visibility and
  Save button disabled state from the single `dirty` field
- `pub(crate) fn button_ids(&self) -> (WidgetId, WidgetId, WidgetId)` — returns
  `(reset_id, cancel_id, save_id)` for `SettingsPanel` constructor and tests

The footer must be a **custom `Widget` implementation** with typed fields for its children,
NOT a `ContainerWidget` wrapper. This is because it needs typed access to:
- `save_button.set_disabled()` — toggled from `accept_action()`
- `unsaved_visibility.set_mode()` — toggled from `accept_action()`

A `ContainerWidget` would store children as `Box<dyn Widget>`, preventing typed access.

The footer's `layout()` method builds a `LayoutBox::flex(Direction::Row, children)` where
children are the layout boxes from each typed child: `unsaved_visibility.layout(ctx)`,
`SpacerWidget::fill().layout(ctx)`, `reset_button.layout(ctx)`, `SpacerWidget::fixed(8,0).layout(ctx)`,
`cancel_button.layout(ctx)`, `SpacerWidget::fixed(8,0).layout(ctx)`, `save_button.layout(ctx)`.
The spacer widgets can be stored as typed fields OR inlined as layout-only LayoutBox leaves
(since they have no interaction state). The outer LayoutBox uses:
- `SizeSpec::Fill` width, `SizeSpec::Fixed(FOOTER_HEIGHT)` height
- `Align::Center` cross-axis alignment
- Padding: `Insets::tlbr(0.0, 28.0, 0.0, 28.0)` (content padding, no sidebar offset needed since
  the footer is already inside the right column)

**`build_settings_dialog()`** changes:
- Currently returns `(Box<dyn Widget>, SettingsIds)`
- Change return type to `(Box<dyn Widget>, SettingsIds, (WidgetId, WidgetId, WidgetId))`
  where the third element is `(reset_id, cancel_id, save_id)` from the footer
- Creates `SettingsFooterWidget::new(theme)`, reads `footer.button_ids()` before boxing
- Builds `right_column = ContainerWidget::column(pages + footer)` where
  `right_column = ContainerWidget::column(pages + footer)`
- Returns `content = ContainerWidget::row(sidebar + right_column)` (replaces current
  `ContainerWidget::row(sidebar + pages)`)
- Callers (`dialog_management.rs`, `content_actions.rs`) updated to pass `footer_ids`
  to `SettingsPanel::new(content, footer_ids, theme)` / `SettingsPanel::embedded(...)`

**Separator change:** The current footer separator uses `SIDEBAR_WIDTH` left padding to
skip the sidebar column (because the footer currently spans the full panel). After the
restructure, the footer lives entirely in the right column, so the separator spans the
full footer width with no left padding offset. The footer's `paint()` draws the separator
as a simple full-width 2px top border at `y = bounds.y()`.

### Action Propagation After Extraction

**Critical architecture constraint:** `on_action()` does NOT bubble through the widget tree
automatically. The framework's dispatch tree calls `on_action()` only on the widget whose
controller fired (the `IdOverrideButton`). The app layer then explicitly calls
`content_widget_mut().on_action()` on the root content widget (`SettingsPanel`) — this is a
single hop, not a tree walk. This means `SettingsFooterWidget::on_action()` is never called
by the framework. The `Clicked(save_id)` action would pass through `SettingsPanel::on_action()`
untranslated and arrive at `handle_dialog_content_action()` as a raw `Clicked` — breaking the
Save/Cancel/Reset buttons.

The two code paths that call `content_widget_mut().on_action()` are:
- `dialog_context/event_handling/mouse.rs` line 250 (mouse clicks)
- `dialog_context/content_key_dispatch.rs` line 99 (keyboard activation)

**Solution: keep `SettingsPanel` as the `on_action()` translation point.** `SettingsPanel`
must retain the footer button IDs and translate `Clicked(save_id)` -> `SaveSettings`, etc.
The `SettingsFooterWidget` still owns the buttons and manages their visual state, but the
button IDs must be passed up to `SettingsPanel` at construction time. The `SettingsPanel`
does NOT create the buttons — it just holds their IDs for `on_action()` translation.

Construction flow:
1. `build_settings_dialog()` creates `SettingsFooterWidget::new(theme)` (which allocates button IDs)
2. Footer exposes `pub(crate) fn button_ids(&self) -> (WidgetId, WidgetId, WidgetId)` — returns
   `(reset_id, cancel_id, save_id)`
3. `build_settings_dialog()` reads the IDs before boxing the footer
4. The IDs are passed to `SettingsPanel::new(content, footer_ids, theme)` (new parameter)
5. `SettingsPanel::on_action()` uses these IDs to translate `Clicked` to semantic actions

This preserves the existing app-layer pattern (single `on_action()` call on the root content
widget) without requiring framework changes.

The `SettingsFooterWidget::on_action()` implementation remains as a defensive passthrough
(`_ => Some(action)`) — it does NOT attempt to translate button clicks, because it will never
be called by the framework in production. The translation lives in `SettingsPanel`.

For `SettingsUnsaved(dirty)` (pushed down via `accept_action`):

1. `SettingsPanel::accept_action()` no longer intercepts `SettingsUnsaved` (it was removed)
2. Delegates to `self.container.accept_action(action)` as before
3. Container propagates to its children, reaching `SettingsFooterWidget`
4. `SettingsFooterWidget::accept_action()` handles it — updates `dirty` field, toggles
   indicator visibility, toggles Save button disabled state, returns `true`
5. `SettingsPanel::accept_action()` sees `handled == true`, calls `self.invalidate_cache()`

This works because `ContainerWidget::accept_action()` already propagates to all children.
The `accept_action` path is tree-walking (unlike `on_action` which is single-hop).
Verify this path works end-to-end in tests (12.4).

### Checklist

- [ ] Move `IdOverrideButton` from `settings_panel/id_override_button.rs` to `button/id_override.rs` as `pub(crate)`
- [ ] Add `pub(crate) mod id_override;` to `button/mod.rs` (after the existing code, before `#[cfg(test)]`)
- [ ] Add `pub(crate) fn set_disabled(&mut self, disabled: bool)` to `IdOverrideButton` that
      delegates to `self.inner.set_disabled(disabled)`
- [ ] Update `settings_panel/mod.rs`: remove `mod id_override_button;`, import from
      `super::button::id_override::IdOverrideButton` instead
- [ ] Create `oriterm_ui/src/widgets/settings_footer/mod.rs` with `#[cfg(test)] mod tests;`
- [ ] Create `oriterm_ui/src/widgets/settings_footer/tests.rs`
- [ ] Add `pub mod settings_footer;` to `oriterm_ui/src/widgets/mod.rs`
- [ ] Implement `SettingsFooterWidget` with three buttons + unsaved indicator group
- [ ] Implement `Widget` trait: `layout()`, `paint()`, `on_action()`, `accept_action()`,
      `for_each_child_mut()`, `for_each_child_mut_all()`, `focusable_children()`
- [ ] `for_each_child_mut()` must visit the visibility wrapper AND all three `IdOverrideButton`
      children so the framework can deliver prepaint, controllers, and lifecycle events
- [ ] `for_each_child_mut_all()` must visit ALL children (same as `for_each_child_mut` unless
      some children are conditionally skipped in the active path)
- [ ] `SettingsFooterWidget::new(theme: &UiTheme) -> Self` — creates all three buttons with
      theme-derived styles (using the new `ButtonStyle` fields from 12.1), allocates their
      `WidgetId`s internally, builds the unsaved indicator group wrapped in `VisibilityWidget`,
      sets initial `dirty: false` and `save_button` disabled
- [ ] Move `FOOTER_HEIGHT` constant from `settings_panel/mod.rs` to `settings_footer/mod.rs`
- [ ] Move footer button construction from `SettingsPanel::build_footer()` to `SettingsFooterWidget::new(theme)`
- [ ] Remove `build_footer()` and `unsaved` field from `SettingsPanel`
- [ ] Keep `save_id`, `cancel_id`, `reset_id` fields — change from self-allocated to
      constructor parameter (`footer_ids: (WidgetId, WidgetId, WidgetId)`)
- [ ] Remove `save_id()`, `cancel_id()` accessors from `SettingsPanel` (were `#[cfg(test)]`-only;
      no longer needed because tests use `SettingsFooterWidget::button_ids()` directly)
- [ ] Update `SettingsPanel::build()` to accept `footer_ids` parameter and no longer append
      footer_sep + footer to the container
- [ ] Remove footer background overpaint hack from `SettingsPanel::paint()`
- [ ] Remove `SettingsUnsaved` interception from `SettingsPanel::accept_action()`. Currently
      lines 462-465 short-circuit on `SettingsUnsaved` and return `true` before delegating to
      children. After removal, ALL actions (including `SettingsUnsaved`) flow through the
      `self.container.accept_action(action)` path at line 468, which propagates to children
      and eventually reaches `SettingsFooterWidget`. The existing `if handled { invalidate_cache() }`
      pattern handles the cache invalidation automatically.
- [ ] `SettingsPanel::on_action()` keeps all translation arms (`Clicked(save_id) -> SaveSettings`,
      `Clicked(cancel_id) -> CancelSettings`, `Clicked(reset_id) -> ResetDefaults`,
      `Clicked(close_id) -> CancelSettings`). The IDs now come from the constructor parameter
      instead of being self-allocated.
- [ ] Update `build_settings_dialog()` in `form_builder/mod.rs`:
      - Change return type to `(Box<dyn Widget>, SettingsIds, (WidgetId, WidgetId, WidgetId))`
      - Create `let footer = SettingsFooterWidget::new(theme)`
      - Read `let footer_ids = footer.button_ids()` BEFORE boxing
      - Build `right_column = ContainerWidget::column()` with `SizeSpec::Fill` width/height,
        containing `pages` (SizeSpec::Fill) + `footer` (SizeSpec::Fixed(FOOTER_HEIGHT))
      - Build `content = ContainerWidget::row(sidebar + right_column)` (replaces current
        `ContainerWidget::row(sidebar + pages)`)
      - Return `(content, ids, footer_ids)`
- [ ] Update callers of `build_settings_dialog()`:
      - `dialog_management.rs:395`: pass `footer_ids` to `SettingsPanel::embedded(content, footer_ids, theme)`
      - `content_actions.rs:126`: pass `footer_ids` to the rebuilt panel constructor
      - `action_handler/tests.rs:13`: update to handle 3-element tuple return
- [ ] Verify `SettingsPanel` shrinks well below 500 lines (expect ~300 after extraction)
- [ ] `SettingsFooterWidget::button_ids()` is `pub(crate)` (NOT `#[cfg(test)]`) because it is
      called by `build_settings_dialog()` in production to pass IDs to `SettingsPanel`
- [ ] Verify `SettingsFooterWidget` stays under 500 lines (expect ~200-250)

---

## 12.3 Unsaved Indicator + Dirty State

### Goal

Make the unsaved indicator a real footer-left layout group that matches the mockup visually and
stays synchronized with the actual pending-config dirty state.

### Files

- `oriterm_ui/src/widgets/settings_footer/mod.rs` (from 12.2)
- `oriterm_ui/src/icons/mod.rs` (add `AlertCircle` variant)
- new: `oriterm_ui/src/icons/footer.rs` (alert-circle icon path data)
- `oriterm/src/gpu/window_renderer/icons.rs` (add 14px resolution entry)

### Current Gaps

The current implementation (to be migrated in 12.2) is missing most of the mockup behavior:

- no left-group layout (indicator is painted at a hardcoded offset)
- no icon (text only — mockup has a 14px alert-circle SVG)
- no `6px` icon/text gap
- no tracked/weighted text style (currently plain `TextStyle::new(11.0, warning)`)
- no hide/show state semantics (currently drawn unconditionally when `unsaved == true`)

**Note:** The dirty-state sync after `ResetDefaults` is already correct in `content_actions.rs:195`.
The extraction in 12.2 preserves this by routing `SettingsUnsaved` through
`SettingsFooterWidget::accept_action()`. No fix needed in `content_actions.rs`.

### Required Footer-Left Model

The mockup footer layout is:

```text
.footer (display: flex, justify-content: flex-end, gap: 8px, padding: 12px 28px)
  .footer-left (margin-right: auto, display: flex, align-items: center, gap: 8px)
    .unsaved-indicator (display: flex, align-items: center, gap: 6px)
      svg (14x14)
      "Unsaved changes"
  button.btn-danger-ghost  "Reset to Defaults"
  button.btn-ghost         "Cancel"
  button.btn-primary       "Save"
```

The `margin-right: auto` on `.footer-left` pushes the buttons to the right. In the widget layout
system, this is expressed as:

```text
ContainerWidget::row (align: Center, width: Fill, height: Fixed(FOOTER_HEIGHT), padding: 12 28)
  unsaved_group (row, gap: 6, align: Center)   <-- visible only when dirty
    icon_widget (14x14)
    label_widget ("Unsaved changes")
  SpacerWidget::fill()                          <-- margin-right: auto equivalent
  reset_button
  SpacerWidget::fixed(8, 0)                     <-- gap: 8px between buttons
  cancel_button
  SpacerWidget::fixed(8, 0)
  save_button
```

When clean (not dirty), the unsaved group should be hidden. Two options:
1. **Remove from layout** — rebuild layout on dirty state change. Simple but invalidates cache.
2. **`VisibilityWidget` wrapper** — wrap the unsaved group in a `VisibilityWidget` (from Section 06)
   and toggle its mode between `VisibilityMode::Visible` and `VisibilityMode::DisplayNone`.
   `DisplayNone` collapses the group to zero size and skips paint/interaction.

Option 2 is better because it avoids full layout tree rebuilds. The `SettingsFooterWidget` holds
a reference to the `VisibilityWidget` wrapper and calls `set_mode()` in its `accept_action()`
handler when `SettingsUnsaved(dirty)` arrives:

```rust
fn accept_action(&mut self, action: &WidgetAction) -> bool {
    if let WidgetAction::SettingsUnsaved(dirty) = action {
        let mode = if *dirty { VisibilityMode::Visible } else { VisibilityMode::DisplayNone };
        self.unsaved_visibility.set_mode(mode);
        // save_button is stored separately, not inside the visibility wrapper
        self.save_button.set_disabled(!*dirty);
        self.dirty = *dirty;
        // Footer returns true here. SettingsPanel::accept_action() sees handled=true
        // and calls self.invalidate_cache() — the existing pattern at lines 468-474
        // already does this for any structural child change. No footer-level cache exists.
        return true;
    }
    // Propagate to children (buttons don't override accept_action, but
    // VisibilityWidget might need it for future actions).
    let mut handled = self.unsaved_visibility.accept_action(action);
    handled |= self.reset_button.accept_action(action);
    handled |= self.cancel_button.accept_action(action);
    handled |= self.save_button.accept_action(action);
    handled
}
```

**Important:** The save button must NOT be inside the `VisibilityWidget` — it's always visible,
only its disabled state changes. The `VisibilityWidget` wraps only the unsaved indicator group
(icon + label).

### Indicator Visual Requirements

Match the mockup's `.unsaved-indicator` CSS:

- icon: `14px` alert-circle (Feather Icons — circle with exclamation mark)
- icon/text gap: `6px`
- label size: `11px`
- weight: `FontWeight::NORMAL` (400) — inherited from body default (no explicit weight in mockup CSS)
- text-transform: `uppercase`
- letter spacing: `0.06em` at 11px = `0.66px`
- color: `theme.warning`

### Adding the AlertCircle Icon

The mockup SVG (`viewBox="0 0 24 24"`, stroke-only):
```html
<circle cx="12" cy="12" r="10"/>
<path d="M12 8v4M12 16h.01"/>
```

Steps:

1. Add `IconId::AlertCircle` variant to `oriterm_ui/src/icons/mod.rs` enum + `ALL` array +
   `path()` match arm.
2. Create `oriterm_ui/src/icons/footer.rs` with `ICON_ALERT_CIRCLE` path data. Add
   `mod footer;` to `icons/mod.rs`. Do NOT add to `sidebar_nav.rs` — that module is specifically
   for sidebar navigation icons. Use `IconStyle::Stroke(NAV_STROKE)` with the same stroke weight
   as other Feather-style icons. Convert SVG viewBox `0 0 24 24` to normalized coordinates by
   dividing all x/y values by 24.0 (e.g. `cx="12"` becomes `0.5`, `r="10"` becomes radius
   `10/24 = 0.4167`). The circle needs to be approximated with 4 cubic Bezier segments
   (standard quarter-circle approximation with control point offset `0.5523`).
3. Add `(IconId::AlertCircle, 14)` to the `ICON_SIZES` array in
   `oriterm/src/gpu/window_renderer/icons.rs` (currently 16 entries, becomes 17).
   Update the array size type annotation from `[(IconId, u32); 16]` to `[(IconId, u32); 17]`.

The icon is rendered through the same pipeline as sidebar nav icons — `DrawCtx::icons` provides
the resolved atlas entry, and the widget uses `scene.push_icon()` or a `push_quad` with UV
coordinates from `icons.get(IconId::AlertCircle, 14)`.

### Save Button Enabled State

The mockup's `.btn-primary:disabled` applies `opacity: 0.4`. The footer should:

- `Save` enabled when `pending_config != original_config` (dirty = true)
- `Save` disabled when clean (dirty = false)

Both the indicator visibility and Save disabled state are driven by the single `dirty: bool` field
in `SettingsFooterWidget`. Updated via `accept_action(SettingsUnsaved(dirty))`. No second boolean.

When `dirty` changes, the footer calls `save_button.set_disabled(!dirty)` and returns `true`
from `accept_action()`. The parent `SettingsPanel::accept_action()` sees `handled == true` and
calls `self.invalidate_cache()` to trigger layout recomputation on the next frame.

### Checklist

- [ ] Add `IconId::AlertCircle` to the icon enum, `ALL` array, and `path()` match
      (existing tests in `icons/tests.rs` iterate `ALL` and will automatically verify
      move-to, command count, normalized coords, and stroke width for the new variant)
- [ ] Create `oriterm_ui/src/icons/footer.rs` with `ICON_ALERT_CIRCLE` path data (normalized 0.0-1.0)
- [ ] Add `mod footer;` to `oriterm_ui/src/icons/mod.rs`
- [ ] Add `(IconId::AlertCircle, 14)` to `ICON_SIZES` in `icons.rs` and update array size `16` -> `17`
- [ ] Build unsaved indicator group as a layout child (icon + label, gap 6)
- [ ] Indicator typography: 11px, normal weight (400), uppercase, 0.66px letter spacing, warning color
- [ ] Hide indicator when clean using `VisibilityWidget` with `VisibilityMode::DisplayNone` (Section 06)
- [ ] Drive indicator visibility and Save disabled state from single `dirty` field
- [ ] Verify `SettingsFooterWidget::accept_action(SettingsUnsaved)` updates both indicator and Save button

---

## 12.4 Semantic Actions + Tests

### Goal

Keep footer semantics clean and add regression coverage for the structural, typography, and
dirty-state behaviors introduced in 12.1-12.3.

### Files

- `oriterm_ui/src/widgets/settings_footer/tests.rs` (from 12.2)
- `oriterm_ui/src/widgets/settings_panel/tests.rs` (update existing tests)
- `oriterm_ui/src/widgets/button/tests.rs` (add typography tests)
- `oriterm/src/app/settings_overlay/form_builder/tests.rs` (composition guard)

### Semantic Actions

Button-to-semantic translation stays in `SettingsPanel::on_action()` (see "Action Propagation
After Extraction" in 12.2 for why). The translations are unchanged:

- `Clicked(save_id)` -> `SaveSettings`
- `Clicked(cancel_id)` -> `CancelSettings`
- `Clicked(reset_id)` -> `ResetDefaults`

The `handle_dialog_content_action()` dispatch table in `content_actions.rs` remains unchanged.
No new `WidgetAction` variants are needed.

### Test Infrastructure Reality

Tests are split by crate boundary:

- **`oriterm_ui` tests** (headless, no GPU/platform): footer widget layout, paint, action
  translation, dirty state, button typography. Use `MockMeasurer`, `compute_layout`, `Scene`.
- **`oriterm` tests** (may need app state): composition tests verifying the footer is inside
  the right column. These use `build_settings_dialog()` directly.

Tests that require the full `App` state machine (e.g. reset-rebuild end-to-end) cannot be
headless widget tests. They belong in `oriterm/src/app/settings_overlay/tests.rs` or as
manual verification items.

### Required Tests — `settings_footer/tests.rs`

Footer widget in isolation (constructed directly, not via `build_settings_dialog`):

- **Construction:**
  - `fn new_does_not_panic()` — `SettingsFooterWidget::new(&theme)` constructs without panic
  - `fn initial_dirty_is_false()` — newly created footer has `dirty == false`
  - `fn focusable_children_returns_three_button_ids()` — `focusable_children()` returns exactly
    3 IDs (reset, cancel, save), all distinct

- **Layout:**
  - `fn footer_fixed_height()` — layout produces a node with height = `FOOTER_HEIGHT`
  - `fn unsaved_hidden_when_clean()` — when `dirty=false`, unsaved indicator group has zero width
  - `fn unsaved_visible_when_dirty()` — when `dirty=true`, unsaved indicator group has nonzero width
  - `fn unsaved_group_does_not_overlap_reset()` — SpacerWidget::fill() separates indicator from
    buttons; verify indicator right edge < reset button left edge

- **Action passthrough:**
  - `fn on_action_passes_through()` — `on_action(Clicked(any_id))` returns `Some(Clicked(any_id))`
    unchanged. The footer does NOT translate button clicks — that is `SettingsPanel`'s job
    (see "Action Propagation After Extraction" in 12.2).

- **Dirty-state behavior:**
  - `fn accept_unsaved_true_enables_save()` — `accept_action(SettingsUnsaved(true))` sets
    Save button enabled (not disabled)
  - `fn accept_unsaved_false_disables_save()` — `accept_action(SettingsUnsaved(false))` sets
    Save button disabled
  - `fn accept_unsaved_updates_indicator_visibility()` — dirty=true shows indicator,
    dirty=false hides it (test via layout width or scene text run count)

- **Paint:**
  - `fn paint_produces_separator_quad()` — scene has a 2px-tall quad at the top of the footer
    bounds matching `theme.border` color (painted directly, not via SeparatorWidget)
  - `fn paint_dirty_renders_warning_text()` — when dirty, scene has a text run with
    `weight == 400` and glyph count matching “UNSAVED CHANGES” (16 chars)
  - `fn paint_clean_no_warning_text()` — when `dirty=false`, scene has no text run containing
    “UNSAVED CHANGES” glyphs (the VisibilityWidget suppresses paint)

### Required Tests — `button/tests.rs`

- `fn button_style_weight_threads_to_text()` — create button with `weight: FontWeight::BOLD`,
  paint to Scene, verify text run `shaped.weight == 700`
- `fn button_style_letter_spacing_increases_width()` — create two buttons with same label,
  one with `letter_spacing: 2.0` and one with `0.0`. Compare layout widths;
  `spacing > 0` should produce wider layout
- `fn button_style_text_transform_uppercase()` — create button with label `”save”` and
  `text_transform: TextTransform::Uppercase`. `MockMeasurer` applies transforms before shaping
  (`mock_measurer/mod.rs:58`), so shaped glyph count matches `”SAVE”` (4 glyphs). Since both
  `”save”` and `”SAVE”` have 4 chars, verify correctness via layout: confirm the button's
  `layout()` produces the same width as a button with label `”SAVE”` and no transform (both
  should be `4 * 8px + padding`). This proves the transform is threaded through `text_style()`.
- `fn button_disabled_opacity_modulates_bg()` — create button with `bg: Color::WHITE`,
  `disabled_opacity: 0.4`, set disabled. Paint to Scene. Verify background quad fill
  has alpha `<= 0.5` (0.4 modulation of 1.0)
- `fn button_disabled_fg_swap_when_no_opacity()` — create button with `disabled_fg: Color::RED`,
  `disabled_opacity: 1.0` (default), set disabled. Paint to Scene. Verify text run color is
  `Color::RED` (the `disabled_fg` swap path), NOT the normal fg with alpha modulation.

### Required Tests — `button/id_override.rs` (after move from settings_panel)

`IdOverrideButton` currently has no tests. After the move to `button/id_override.rs`, add a
sibling `button/id_override/tests.rs` OR add tests directly in the file (it's only ~110 lines,
so inline `#[cfg(test)] mod tests { ... }` is acceptable at this size — but the project
convention is sibling files, so prefer splitting into `button/id_override/mod.rs` +
`button/id_override/tests.rs` if adding tests).

- `fn id_override_set_disabled_delegates()` — create `IdOverrideButton`, call `set_disabled(true)`,
  verify `is_focusable()` returns `false`. Call `set_disabled(false)`, verify `is_focusable()`
  returns `true`. (Tests the new `set_disabled` method added in 12.2.)
- `fn id_override_returns_overridden_id()` — create with known `WidgetId`, verify `id()` returns
  the override, not the inner button's ID.

### Required Tests — `settings_panel/tests.rs`

Update existing tests after footer extraction. **Note:** `make_panel()` creates a `SettingsPanel`
with a bare `FormLayout` as content. After extraction, `make_panel()` must pass dummy footer IDs
(3 fresh `WidgetId::next()` values) to match the new constructor signature. This panel has NO
footer widget (the footer is composed by `build_settings_dialog()`). Tests that verify footer
behavior belong in `settings_footer/tests.rs`.

- `fn on_action_maps_close_to_cancel_settings()` — keep (close button stays in SettingsPanel)
- `fn on_action_maps_save_to_save_settings()` — keep. SettingsPanel STILL translates
  `Clicked(save_id) -> SaveSettings`. Update to use dummy save_id from `make_panel()`.
- `fn on_action_maps_cancel_to_cancel_settings()` — keep, same update as save.
- `fn on_action_passes_through_other_actions()` — keep
- Remove `save_id()` and `cancel_id()` accessor usage — these methods are removed. Use
  the dummy IDs returned by `make_panel()` instead.
- `fn draws_without_panic()` — keep, verify still passes after footer extraction
- `fn focusable_children_includes_close_button()` — keep (only in overlay/chrome mode)
- `fn layout_has_fixed_width()` — keep
- `fn layout_hugs_content_height()` — keep, but verify height expectation is still valid
  (panel without footer may be shorter; adjust assertion if needed)
- `fn for_each_child_mut_yields_container_not_buttons()` — keep

### Required Tests — `form_builder/tests.rs`

- Update existing tests for new 3-element return type: `dialog_builds_without_panic`,
  `settings_ids_all_distinct`, `content_widget_has_valid_id`, `all_page_ids_are_set`,
  `scheme_card_ids_captured`, `sidebar_id_captured`, `dialog_builds_with_update_info` —
  all destructure `(content, ids, _footer_ids)` instead of `(content, ids)`
- `fn footer_buttons_reachable_through_widget_tree()` — build the full dialog via
  `build_settings_dialog()` (returns `(content, ids, footer_ids)`), wrap in
  `SettingsPanel::embedded(content, footer_ids, &theme)`, call `focusable_children()`.
  Verify that the focusable set contains all 3 footer button IDs from the returned tuple.
- `fn accept_unsaved_reaches_footer()` — build full dialog via `build_settings_dialog()`,
  wrap in `SettingsPanel::embedded(content, footer_ids, &theme)`, call
  `panel.accept_action(&WidgetAction::SettingsUnsaved(true))`. Verify it returns `true`
  (the footer handled it). This guards the propagation path from panel -> container -> footer.
- `settings_ids_all_distinct` — NO change needed. `save_id`/`cancel_id`/`reset_id` were never
  in `SettingsIds` (they lived in `SettingsPanel`). The `collect_ids()` helper only collects
  from `SettingsIds` fields, so the expected count of 26 remains correct.

### Checklist

- [ ] Add ~14 tests to `settings_footer/tests.rs` (construction, layout, passthrough, dirty state, paint)
- [ ] Add 5 tests to `button/tests.rs` (weight, letter spacing, text transform, disabled opacity,
      backward-compat disabled_fg swap)
- [ ] Add 2 tests to `button/id_override.rs` or a sibling `button/tests.rs` section
      (set_disabled delegation, id override correctness)
- [ ] Update `settings_panel/tests.rs` — remove migrated action tests, keep passthrough +
      close mapping + structural tests, update `make_panel()` usage notes
- [ ] Add 2 composition tests to `form_builder/tests.rs` (footer reachability, unsaved propagation)
- [ ] Verify existing `form_builder/tests.rs` tests still pass (may need count adjustment)
- [ ] All tests assert specific values or structural properties, not just “doesn't panic”

---

## 12.R Third Party Review Findings

### Resolved Findings

- `TPR-12-001` The draft treated the footer as a full-panel bottom bar, but the mockup footer lives
  only in the right content column while the sidebar remains full-height.
- `TPR-12-002` The draft's claimed button order was wrong. The mockup layout is `footer-left`
  unsaved group first, then `Reset`, `Cancel`, `Save` as a right-aligned cluster.
- `TPR-12-003` The current unsaved indicator is painted at the same left inset where the Reset
  button is laid out, so it can overlap the button. This must be fixed structurally, not by more
  paint offsets.
- `TPR-12-004` The draft stated all buttons use `font-weight: 500`, but the mockup's
  `.btn-primary` uses `700`.
- `TPR-12-005` The current button primitive cannot express the footer's required typography or
  disabled primary state because `ButtonStyle` lacks weight, tracking, and correct disabled border /
  opacity support.
- `TPR-12-006` `settings_panel/mod.rs` is already near the repository file-size limit, so adding
  more footer-specific logic there is not maintainable. Footer ownership should be extracted.
- `TPR-12-007` **Invalid (verified 2026-03-25).** The original finding claimed `ResetDefaults` does
  not reapply `SettingsUnsaved(dirty)` to the rebuilt panel. This is incorrect —
  `content_actions.rs:195` already sends `panel.accept_action(&WidgetAction::SettingsUnsaved(dirty))`
  after rebuild. The footer extraction in 12.2 must preserve this existing behavior by having the
  new `SettingsFooterWidget::accept_action()` handle `SettingsUnsaved`.

---

## 12.5 Build & Verify

### Gate

```bash
timeout 150 ./build-all.sh
timeout 150 ./clippy-all.sh
timeout 150 ./test-all.sh
```

### Focused Verification

```bash
timeout 150 cargo test -p oriterm_ui settings_footer::tests
timeout 150 cargo test -p oriterm_ui settings_panel::tests
timeout 150 cargo test -p oriterm_ui button::tests
timeout 150 cargo test -p oriterm_ui button::id_override
timeout 150 cargo test -p oriterm_ui icons::tests
timeout 150 cargo test -p oriterm settings_overlay::form_builder::tests
timeout 150 cargo test -p oriterm settings_overlay::action_handler::tests
```

### File Size Verification

After all 12.x changes, verify these files stay under 500 lines:
- `oriterm_ui/src/widgets/button/mod.rs` (currently 314 lines, expect ~340 after 12.1)
- `oriterm_ui/src/widgets/button/id_override.rs` (moved from settings_panel, currently 103 lines,
  expect ~110 after adding `set_disabled`)
- `oriterm_ui/src/widgets/settings_panel/mod.rs` (currently 488 lines, expect ~320 after 12.2 —
  keeps `on_action()` translation but loses `build_footer()`, `paint()` overpaint, `unsaved`,
  and `accept_action` short-circuit)
- `oriterm_ui/src/widgets/settings_footer/mod.rs` (new, expect ~200-250 lines)
- `oriterm_ui/src/icons/footer.rs` (new, expect ~40-60 lines)
- `oriterm/src/app/settings_overlay/form_builder/mod.rs` (currently 223 lines, expect ~250 after 12.2)

### Manual Verification Checklist

- [ ] Footer appears only in the right content column
- [ ] Sidebar remains full-height and visually continuous to the bottom
- [ ] Unsaved group appears on the left without overlapping buttons
- [ ] Reset, Cancel, and Save form a right-aligned cluster with correct `8px` spacing
- [ ] Button labels render uppercase with correct letter spacing (0.48px at 12px)
- [ ] Reset/Cancel use medium weight (500), Save uses bold (700)
- [ ] Save disables correctly when there are no unsaved changes (opacity 0.4)
- [ ] Unsaved indicator shows alert-circle icon (14px) + tracked label when dirty
- [ ] Unsaved indicator hides cleanly when no unsaved changes
- [ ] Reset, Cancel, Save, and unsaved visuals match the mockup
- [ ] Hover states work correctly on all three buttons (danger-ghost, ghost, primary)
