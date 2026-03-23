---
section: "10"
title: "Visual Fidelity: Sidebar + Navigation"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "The settings sidebar matches the mockup's structure and interaction model: a full-height 200px rail with a real search input, precise section/header/nav/footer spacing, correct active and hover treatment, working modified dots, and interactive footer metadata"
depends_on: ["01", "02", "03", "08"]
sections:
  - id: "10.1"
    title: "Sidebar Structure + Module Split"
    status: not-started
  - id: "10.2"
    title: "Search Field Fidelity + Behavior"
    status: not-started
  - id: "10.3"
    title: "Section Headers + Nav Rhythm"
    status: not-started
  - id: "10.4"
    title: "Active States + Modified Dots"
    status: not-started
  - id: "10.5"
    title: "Footer Metadata + Actions"
    status: not-started
  - id: "10.6"
    title: "Tests"
    status: not-started
  - id: "10.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "10.7"
    title: "Build & Verify"
    status: not-started
---

# Section 10: Visual Fidelity - Sidebar + Navigation

## Problem

The draft treated Section 10 as a mostly visual cleanup, but the current tree shows the sidebar is
missing several real behaviors and API surfaces required by the mockup.

What the code actually has today:

- `oriterm_ui/src/widgets/sidebar_nav/mod.rs` is a single `509`-line custom widget that already
  exceeds the repository's file-size rule before this section adds more state or interactions.
- The widget paints a placeholder search box directly in `paint_search_field()` instead of using a
  real input control. There is no search icon, no focus-border treatment, and no text editing.
- The current widget API only exposes `with_version(...)` and `with_config_path(...)`. The mockup
  footer also includes an `Update Available` link plus hover/click treatment for the config-path
  row.
- The current active-row background is painted only to the right of the `3px` indicator strip,
  while the CSS model applies background to the full row box and overlays the left border.
- Section title and row spacing are still bundled into coarse constants such as
  `SECTION_TITLE_HEIGHT = 28.0`, even though the mockup uses separate search spacing, title bottom
  margins, and inter-section top margins.
- The widget can paint modified dots, but no current app call site drives `set_page_modified(...)`,
  so the mockup's unsaved-state indicator is not actually wired.
- `mockups/settings-brutal.html` uses a real `<input type="text">` for the sidebar search field,
  not a decorative placeholder. The repo already has a reusable `TextInputWidget`, so keeping the
  sidebar on a fake paint-only path would be a regression in architecture, not a simplification.

Section 10 should keep the full fidelity goal, but move the implementation to boundaries that can
actually support it.

## Corrected Scope

Section 10 should deliver the sidebar as a real interactive surface, not a static paint pass:

1. split `sidebar_nav` into maintainable submodules before adding more logic
2. replace the painted search placeholder with a real search input using shared text-input
   behavior and mockup-specific styling
3. encode sidebar geometry explicitly instead of hiding spacing in a few approximate constants
4. wire modified dots to real per-page dirty state
5. extend the footer contract so `Update Available` and the config-path row can render and act
   like real interactive targets

The implementation does not need to throw away `SidebarNavWidget`. The more feasible path is to
keep one top-level widget for existing settings/page-container routing, then give it better internal
structure and state.

---

## 10.1 Sidebar Structure + Module Split

### Goal

Move the sidebar onto a maintainable widget boundary that can support search, footer actions, and
precise layout without growing the current monolith further.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
- `oriterm_ui/src/widgets/sidebar_nav/tests.rs`
- `oriterm/src/app/settings_overlay/form_builder/mod.rs`

### Required Structure

Split `sidebar_nav` before landing the fidelity work:

```text
oriterm_ui/src/widgets/sidebar_nav/
    mod.rs
    geometry.rs
    input.rs
    paint.rs
    tests.rs
```

Recommended ownership:

- `mod.rs`
  - public types (`NavSection`, `NavItem`, `SidebarNavWidget`, style structs)
  - builder/accessor API
  - shared widget state
- `geometry.rs`
  - exact sidebar rect math
  - section/title/item/footer/search metrics
  - hit-test enums for nav items and footer targets
- `input.rs`
  - pointer/focus/key routing
  - search-input delegation
  - footer target activation
- `paint.rs`
  - background, search field, titles, items, footer, separators

### Widget Contract Changes

Keep one `SidebarNavWidget`, but expand its internal model so the section can fulfill the mockup:

- add an internal real search control rather than a painted placeholder
- add optional footer update metadata
- add hover/hit-test state for:
  - nav rows
  - update link
  - config-path row
- add a bulk dirty-state API (`set_modified_pages(bitset)` or equivalent) so app code can update
  the whole sidebar coherently instead of toggling one bit at a time through ad hoc calls

The top-level widget ID should remain stable for page-selection routing. If footer targets need
their own actions, expose dedicated internal IDs or explicit action variants rather than overloading
the nav-row `Selected { index }` path.

### Checklist

- [ ] Split `sidebar_nav` into submodules before adding new fidelity logic
- [ ] Keep one top-level `SidebarNavWidget` for page-selection integration
- [ ] Add explicit internal state for search and footer interactions
- [ ] Add a coherent modified-pages update API
- [ ] Bring `mod.rs` back under the repository file-size limit

---

## 10.2 Search Field Fidelity + Behavior

### Goal

Replace the fake painted search placeholder with a real input that matches the mockup visually and
behaves like an actual control.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
- `oriterm_ui/src/widgets/sidebar_nav/input.rs`
- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `oriterm_ui/src/widgets/text_input/mod.rs`
- `oriterm_ui/src/widgets/text_input/widget_impl.rs`
- `oriterm_ui/src/icons/mod.rs`
- `oriterm/src/gpu/icon_rasterizer/mod.rs`

### Current Boundary Problem

The repo already has `TextInputWidget`, including focus handling, caret rendering, selection, and
`WidgetAction::TextChanged`. The current sidebar bypasses that shared path and paints a dead box
instead.

That is no longer the right boundary. Section 10 should reuse shared text-input behavior and apply
mockup-specific styling on top.

### Implementation Plan

Add a real search field to `SidebarNavWidget` using the existing text-input behavior path.

Recommended approach:

- embed a `TextInputWidget` inside `SidebarNavWidget`, or extract a shared helper from
  `TextInputWidget` if direct embedding proves awkward
- route search-rect input/focus/paint through that shared input state instead of duplicating text
  editing logic inside the sidebar widget
- add a sidebar-specific search style:
  - height `28px`
  - background `theme.bg_primary` (`#16161c`)
  - border width `2px`
  - unfocused border `theme.border`
  - focused border `theme.accent`
  - placeholder color `theme.fg_faint`
  - font size `12px`
  - padding equivalent to mockup `6px 8px 6px 26px`

The default `TextInputStyle::from_theme()` is not enough on its own because:

- `bg_input` is currently `#12121a`, not the mockup's `#16161c`
- default border width is `1px`, not `2px`
- default placeholder color is `fg_disabled`, not `fg_faint`

### Search Icon

The mockup includes a leading search icon inside the field. Section 08 covered the eight sidebar
page icons, but not this glyph.

Section 10 should therefore add the search icon through the shared icon pipeline:

- add a dedicated sidebar-search glyph to `IconId` and the icon registry, or
- add a text-input leading-icon capability and feed it from the same shared icon/rasterizer path

Do not keep a magic empty left padding with no icon rendered.

### Search Behavior

The field should be real, not cosmetic:

- keyboard-editable
- focusable
- accent focus border on focus
- query-driven local filtering of sidebar content

The minimum useful behavior is local filtering over section titles and item labels, preserving
original order. That is feasible because the settings sidebar has only a small fixed set of pages.

To keep page state coherent:

- filtering must not silently switch pages
- if the active page does not match the query, keep its row visible and marked active until the
  user selects a matching row or clears the query

This section does not need to invent a global full-text index of all settings descriptions, but it
must stop treating the search field as decorative paint.

### Checklist

- [ ] Replace `paint_search_field()` placeholder logic with a real input path
- [ ] Apply exact mockup search styling instead of default `TextInputStyle`
- [ ] Add and render the leading search icon through the shared icon system
- [ ] Support focus, caret, selection, and text editing
- [ ] Filter sidebar sections/items by query without forcing an automatic page switch

---

## 10.3 Section Headers + Nav Rhythm

### Goal

Match the mockup's spacing and typography by making sidebar geometry explicit instead of relying on
approximate bundled constants.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/geometry.rs`
- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `mockups/settings-brutal.html`

### Layout Facts From The Mockup

The mockup uses separate spacing rules:

- sidebar width `200px`
- sidebar padding `16px 0`
- search container padding `0 10px`
- search-to-title gap `12px`
- title padding `0 16px`
- title bottom margin `8px`
- non-first title top margin `20px`
- nav row horizontal padding `16px`
- nav row vertical margin `1px`
- icon/text gap `10px`

The current widget collapses several of these into broad constants like `SECTION_TITLE_HEIGHT` and
hardcoded icon/text offsets. That is why the draft's "verify later" approach is too weak.

### Required Geometry Rewrite

Replace the current rough layout math with explicit geometry helpers for:

- search field rect
- first title baseline/rect
- non-first title top margin
- nav row outer rect
- content inset inside the active-indicator border area
- footer block rect

Do not treat `ITEM_HEIGHT = 32.0` as authoritative CSS truth. The mockup CSS defines nav-row
padding and margin, not a hard row height. Section 10 should derive or codify the outer row rect
that matches the rendered mockup, then keep that geometry in one place.

### Typography Requirements

Section titles must keep:

- font size `10`
- regular weight
- uppercase transform
- letter spacing `1.5`
- `// ` prefix
- `theme.fg_faint`

If Section 03 lands a shared text-transform/letter-spacing path, use it. Until then, keep the
sidebar on whatever code path actually renders the spacing correctly.

### Checklist

- [ ] Replace bundled section-spacing constants with explicit geometry helpers
- [ ] Match search, title, section, and row spacing to the mockup's real CSS structure
- [ ] Keep title typography at `10px`, regular, uppercase, `1.5` letter spacing
- [ ] Centralize all sidebar geometry in one module instead of scattering offsets in paint code

---

## 10.4 Active States + Modified Dots

### Goal

Make active, hover, and dirty indicators match the mockup both visually and behaviorally.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `oriterm_ui/src/widgets/sidebar_nav/input.rs`
- `oriterm/src/app/settings_overlay/action_handler/mod.rs`
- `oriterm/src/app/dialog_context/content_actions.rs`

### Active + Hover Painting

The current widget already has the right colors, but one geometry detail is wrong:

- active/hover backgrounds are painted only to the right of the `3px` indicator strip

The mockup CSS applies background to the full nav row and overlays the left border via
`border-left-color`. Section 10 should mirror that model:

- paint active/hover background across the full row rect
- paint the `3px` active indicator on top
- keep icon opacity at `0.7` for normal and hover
- lift icon opacity to `1.0` only for the active row

### Nav Item Insets

The current icon/text offsets are still too tight relative to the mockup's `16px` horizontal
padding and `10px` icon/text gap. The geometry rewrite in Section 10.3 should drive exact content
insets here rather than a second round of local constants.

### Modified Dots

The widget can already paint a `6px` warning dot, but there are no current call sites for
`set_page_modified(...)`. Section 10 must wire the data flow, not just preserve the paint code.

Required integration:

- compute per-page dirty state from the settings overlay's pending-vs-original config state
- update the sidebar whenever page-dirty state changes
- preserve the existing warning color and right-edge placement from the mockup

The dirty-state mapping should live with the settings overlay/dialog state, not inside the generic
sidebar widget. The sidebar should consume a page-dirty bitset; it should not become responsible
for understanding config diffs.

### Checklist

- [ ] Paint active and hover backgrounds across the full row box
- [ ] Keep the `3px` active indicator as an overlay, not as a clipped background substitute
- [ ] Correct icon/text insets to match mockup spacing
- [ ] Wire modified-dot state from real settings dirty data
- [ ] Keep dirty-dot paint at `6px` and `theme.warning`

---

## 10.5 Footer Metadata + Actions

### Goal

Make the footer match the mockup's content, spacing, and interaction model instead of rendering two
static text strings at the bottom edge.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
- `oriterm_ui/src/widgets/sidebar_nav/geometry.rs`
- `oriterm_ui/src/widgets/sidebar_nav/input.rs`
- `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
- `oriterm/src/app/settings_overlay/form_builder/mod.rs`
- `oriterm/src/app/dialog_context/content_actions.rs`

### Current Gap

The mockup footer contains:

- version text
- `Update Available` link with accent hover color and underline on hover
- config-path row with dim default opacity, ellipsis behavior, and accent hover treatment

The current widget can only show:

- version text
- config path text

There is no way to express an update link, no hover state for footer rows, and no action-routing
contract for footer clicks.

### Footer Contract Rewrite

Extend `SidebarNavWidget` so the footer can be configured intentionally:

- keep `with_version(...)`
- keep `with_config_path(...)`
- add optional update metadata, for example:
  - label text
  - available version / tooltip text
  - whether the link is currently shown

The exact API shape can vary, but it must be able to represent the mockup footer directly instead
of encoding it in comments.

### Interaction Model

Add separate hover/hit targets for:

- update link
- config path row

For emitted actions, use one of these two clean paths:

1. give the footer targets their own widget IDs and emit `Clicked(id)` for them
2. add an explicit sidebar/footer action variant if the shared widget-action enum would otherwise
   become ambiguous

Do not overload nav-item `Selected { index }` for footer actions.

### Visual Requirements

Implement the mockup footer layout explicitly:

- footer padding equivalent to mockup `8px 16px`
- `4px` gap between version row and config path row
- right-sidebar border remains `2px`
- version text `11px`, faint color
- update link `10px`, accent by default, `accent_hover` on hover
- config path `10px`, faint color at reduced opacity by default
- config path truncates with ellipsis inside the available width
- config path hover restores full opacity and uses accent coloring

The config-path row does not need a custom font override if the UI font remains monospace by
default; if that assumption changes, this section should add an explicit family instead of allowing
fidelity drift.

### App Integration

Update the settings sidebar builder in `form_builder/mod.rs` to populate the richer footer model.

That includes:

- continuing to set the package version
- providing the config path through the real config-path helper instead of a hardcoded string when
  appropriate
- providing update-link metadata only when the app actually has update information to show

### Checklist

- [ ] Add a real footer data contract for update metadata and config-path behavior
- [ ] Add distinct hover/hit regions for update and config-path targets
- [ ] Emit non-ambiguous actions for footer interactions
- [ ] Match footer spacing, colors, and hover treatment to the mockup
- [ ] Truncate config-path text with ellipsis inside the footer width

---

## 10.6 Tests

### Goal

Turn sidebar fidelity into repeatable regression coverage instead of relying on manual inspection.

### Files

- `oriterm_ui/src/widgets/sidebar_nav/tests.rs`
- any new sidebar-nav submodule tests needed after the split
- `oriterm/src/app/settings_overlay/form_builder/tests.rs`
- `oriterm/src/app/settings_overlay/action_handler/tests.rs`

### Required Coverage

Keep the current interaction tests, but add the coverage the existing suite is missing.

Add scene-level sidebar paint assertions for:

- `fn paint_sidebar_full_height_background()` — full-height sidebar background and `2px` right border
- `fn paint_search_field_shape_and_colors()` — search field primitive shape and colors
- `fn paint_search_icon_present()` — search icon presence
- `fn paint_active_row_full_background_with_indicator()` — full-row active background plus `3px` indicator
- `fn paint_footer_version_and_config_path()` — footer version/update/config-path primitives
- `fn paint_modified_dot_on_dirty_page()` — modified-dot painting when page is dirty
- `fn paint_no_modified_dot_on_clean_page()` — no modified dot when page is clean

Add input/state tests for:

- `fn search_field_receives_focus()` — search-field focus
- `fn search_field_text_editing()` — search-field text editing
- `fn search_filters_nav_items()` — local filtering hides non-matching items
- `fn search_preserves_active_page()` — active-page preservation while filtering
- `fn search_empty_query_shows_all()` — clearing query restores all items
- `fn footer_update_link_hover()` — footer update link hover hit testing
- `fn footer_config_path_hover()` — footer config-path hover hit testing
- `fn footer_click_emits_action()` — footer click action routing

Add integration tests for:

- `fn sidebar_builder_populates_footer_metadata()` — footer builder metadata from the settings sidebar builder
- `fn modified_dots_update_on_dirty_state_change()` — modified-dot updates when settings dirty state changes

Use the existing `Scene`-based widget test pattern already present elsewhere in `oriterm_ui`; do not
leave this section with only hit-test and keyboard-navigation coverage.

### Checklist

- [ ] Add scene-based paint assertions for sidebar fidelity
- [ ] Add search interaction and filtering tests
- [ ] Add footer hover/click routing tests
- [ ] Add modified-dot integration coverage
- [ ] Preserve existing nav selection and keyboard behavior tests

---

## 10.R Third Party Review Findings

### Resolved Findings

- `TPR-10-001` The draft overstated what already matched. The current sidebar still lacks a real
  search control, a search icon, footer update-link content, and footer interaction states.
- `TPR-10-002` `oriterm_ui/src/widgets/sidebar_nav/mod.rs` is already over the repository file-size
  limit at `509` lines, so the section must split the module before adding more fidelity logic.
- `TPR-10-003` The draft treated the search box as a paint-only concern even though the repo already
  has a reusable `TextInputWidget`. Section 10 should use the shared text-input behavior path.
- `TPR-10-004` The current active-row background geometry does not match the mockup CSS model
  because it is clipped to the right of the indicator strip instead of painting across the full row.
- `TPR-10-005` The footer API cannot currently represent the mockup's `Update Available` link or
  emit distinct footer actions. Section 10 must extend the widget contract instead of "verifying"
  behavior that does not exist.
- `TPR-10-006` Modified-dot painting exists, but no current app code drives page dirty state into
  the sidebar. The fidelity plan must include the missing data flow.
- `TPR-10-007` The existing sidebar tests are mostly behavioral and do not verify rendered scene
  fidelity, so visual regressions would currently pass unnoticed.

---

## 10.7 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Focused Verification

- run the targeted sidebar widget tests in `oriterm_ui`
- run the settings overlay/form-builder tests that cover footer metadata and dirty-state integration
- verify the search field, nav rows, and footer match the mockup in the live settings overlay

Suggested commands:

```bash
cargo test -p oriterm_ui sidebar_nav::tests
cargo test -p oriterm settings_overlay::form_builder::tests
cargo test -p oriterm settings_overlay::action_handler::tests
```

Manual verification checklist:

- [ ] Sidebar rail is full-height, `200px` wide, with `#0e0e12` background and `2px` right border
- [ ] Search field matches the mockup visually and behaves like a real input
- [ ] Section headers and nav-row spacing match the mockup
- [ ] Active, hover, and modified states match the mockup
- [ ] Footer shows version, optional update link, and config-path behavior correctly
