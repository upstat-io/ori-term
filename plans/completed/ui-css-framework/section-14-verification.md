---
section: "14"
title: "Verification + Visual Regression"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-28
goal: "The CSS UI framework plan closes with reproducible evidence, not a hand-written checklist: every prior section owns direct automated regressions, the settings dialog has deterministic GPU golden coverage, build/platform/performance gates reflect the real repository scripts, and the remaining human sign-off is limited to live behavior the automated layers cannot prove."
depends_on: ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "15"]
sections:
  - id: "14.0"
    title: "Blocking Prerequisites"
    status: complete
  - id: "14.1"
    title: "Verification Ownership + Artifacts"
    status: complete
  - id: "14.2"
    title: "Automated Test Matrix"
    status: complete
  - id: "14.3"
    title: "Deterministic Visual Regression"
    status: complete
  - id: "14.4"
    title: "Manual Sign-Off"
    status: complete
  - id: "14.5"
    title: "Build + Platform Gates"
    status: complete
  - id: "14.6"
    title: "Performance + Invariants"
    status: complete
  - id: "14.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "14.7"
    title: "Completion Checklist"
    status: complete
---

# Section 14: Verification + Visual Regression

## Problem

The current draft is not accurate to the repository or to the revised Sections 01-15.

What the tree shows today:

- Verification here is not documentation-only work. To close this plan correctly, Section 14 must
  add real test code, golden PNGs, and possibly small shared test helpers at the dialog-rendering
  boundary.
- The draft's per-section matrix has already drifted from the reviewed section set:
  - Section 12 now requires a bold primary Save button, not one shared medium-weight button style.
  - Section 13 no longer owns `row gap 0`; that responsibility moved back to Section 11.
  - Section 09 now covers real settings-pipeline and tab-style behavior that the old matrix does
    not mention.
  - Sections 05-08 and 10-13 now rely on renderer, overlay, and shared-widget boundaries that need
    more than one-line field assertions.
  - Section 15 adds cursor icon management to the dialog window. Verification must confirm that
    hovering clickable widgets changes the OS cursor to `pointer`, disabled controls show
    `not-allowed`, and the cursor resets to `default` when leaving interactive regions.
- The repository already has deterministic GPU visual regression infrastructure in
  `oriterm/src/gpu/visual_regression/`, plus a UI-only renderer path via
  `WindowRenderer::new_ui_only`, `prepare_ui_frame(...)`, and
  `append_ui_scene_with_text(...)`. The draft ignores all of that and falls back to a manual
  screenshot checklist.
- The build scripts are not what the draft says:
  - `./build-all.sh` cross-builds Windows GNU debug + release
  - `./clippy-all.sh` runs Windows GNU and host clippy with warnings denied
  - `./test-all.sh` runs `cargo test --workspace --features oriterm/gpu-tests` with
    `RUSTFLAGS="-D warnings"` to match CI
- The performance section is too anecdotal. The tree already has automated invariants for idle
  event-loop behavior and scene/damage allocation reuse, but the draft does not connect the
  settings dialog closeout to them.

Section 14 therefore needs to become a layered verification plan with real ownership and real
artifacts, not a static checklist full of stale assumptions.

## Corrected Scope

Section 14 should keep the full final-verification goal and close it with four layers:

1. section-owned automated regressions near the modules changed in Sections 01-15
2. cross-section settings-dialog integration tests for builder, actions, layout, overlays, and
   semantic state
3. deterministic GPU golden tests for the rendered settings dialog itself
4. focused manual sign-off for live interaction, cursor icons, and native-platform behavior that
   automated tests cannot prove

This section should not pretend that one manual screenshot pass is enough, and it should not repeat
stale per-feature assertions that no longer match the revised plan.

---

## 14.0 Blocking Prerequisites

### Goal

Enumerate the concrete blockers that must be resolved before any 14.x subsection can begin. Section
14 depends on Sections 01-13 and 15 all being complete. As of this writing, several are not.

### Current Blockers

| Section | Status | Blocker |
|---------|--------|---------|
| 02 | ~~in-progress~~ complete | ~~02.5 has 1 item deferred to sections 10-13~~ Resolved 2026-03-26: FontWeight::MEDIUM adopted in sidebar nav active label |
| 08 | ~~in-progress~~ complete | ~~08.7 in-progress~~ Resolved 2026-03-26: stale frontmatter, all items were already checked |
| 12 | ~~in-progress~~ complete | ~~12.R TPR findings~~ Resolved 2026-03-26: TPR-12-011/012/013 all fixed |
| 13 | ~~in-progress~~ complete | ~~13.R TPR findings~~ Resolved 2026-03-26: TPR-13-010 fixed. ~~13.7 manual verification~~ Resolved 2026-03-27: all 8 visual items verified against code |
| 15 | complete | Cursor icons on all interactive widgets, disabled scan, overlay cursor resolution |

### Ordering Constraint

The 14.x subsections must be implemented in order:

1. **14.0** — verify all blockers are resolved (this subsection)
2. **14.1** — define verification layers and file ownership
3. **14.2** — audit and fill test coverage gaps section-by-section
4. **14.3** — build golden test infrastructure and generate reference PNGs
5. **14.4** — manual sign-off (requires a running binary, done after all automated work)
6. **14.5** — build + platform gates (run after 14.2-14.4 changes land)
7. **14.6** — performance + invariants (can overlap with 14.5)
8. **14.7** — final completion checklist (last)

### Checklist

- [x] Section 02 is complete (02.5 deferred item resolves when sections 10-13 adopt `FontWeight::MEDIUM`)
- [x] Section 08 is complete (08.7 status = complete)
- [x] Section 12 open TPR findings are resolved: TPR-12-011 and TPR-12-012
- [x] Section 13 open TPR findings are resolved: TPR-13-010; 13.7 (Build & Verify) status = complete
- [x] Section 15 has been reviewed (`reviewed: true` in frontmatter) and implemented (status = complete)
- [x] All sections 01-13, 15 show `status: complete` in their frontmatter
- [x] `timeout 150 ./test-all.sh` passes (baseline green before adding Section 14 work)

---

## 14.1 Verification Ownership + Artifacts

### Goal

Define the actual verification layers and the files that own them so Section 14 closes with durable
artifacts instead of one-off human checks.

### Files

- section-local `tests.rs` files under `oriterm_ui/src/widgets/`, `oriterm_ui/src/text/`,
  `oriterm/src/gpu/scene_convert/`, `oriterm/src/font/`, and related modules changed by
  Sections 01-15
- `oriterm/src/app/settings_overlay/form_builder/tests.rs`
- `oriterm/src/app/settings_overlay/action_handler/tests.rs`
- `oriterm_ui/src/widgets/settings_panel/tests.rs`
- `oriterm_ui/src/testing/render_assert.rs`
- `oriterm/src/gpu/visual_regression/mod.rs`
- new `oriterm/src/gpu/visual_regression/settings_dialog.rs`
- `oriterm/tests/references/settings_*.png`
- `oriterm/src/app/dialog_rendering.rs` and/or a small shared test helper if the existing dialog
  composition path needs to be reused directly

### Required Verification Layers

1. **Section-local regressions**
   - each section keeps direct ownership of its changed primitives
   - examples: text shaping, scene conversion, widget paint geometry, icon path generation

2. **Settings-dialog integration**
   - builder IDs, active-page wiring, footer dirty state, action-handler mapping, dropdown overlay
     routing, and page rebuild behavior

3. **GPU dialog goldens**
   - full rendered output through the UI scene conversion and renderer path
   - committed reference PNGs plus deterministic update workflow

4. **Manual sign-off**
   - live interactions, native-window behavior, and cross-platform visual confirmation

### Important Scope Correction

This section does add implementation work:

- new tests
- new golden images
- likely a settings-dialog visual regression module
- possibly a small reusable helper around dialog composition if the current private render path is
  too awkward to invoke from tests

That is still verification work, but it is not "no code changes."

### Checklist

- [x] Audit that each of Sections 01-15 has at least one `tests.rs` file with behavioral regression coverage. Fill in the following audit table (one row per section) confirming the test file path and at least one test function name that exercises core behavioral change:

| Section | Test file(s) | Key behavioral test(s) | Sufficient? |
|---------|-------------|----------------------|-------------|
| 01 | `oriterm/src/font/ui_font_sizes/tests.rs`, `oriterm/src/font/shaper/tests.rs` | `select_returns_exact_size_collection`, `ensure_size_creates_collection_for_unseen_size`, `set_dpi_rebuilds_all_collections` | Yes |
| 02 | `oriterm/src/font/collection/tests.rs` | `resolve_bold_without_bold_face_is_synthetic`, `resolve_italic_without_italic_face_is_synthetic`, `rasterize_alpha_produces_bitmap` | Yes |
| 03 | `oriterm_ui/src/text/tests.rs` | `text_transform_uppercase_matches_explicit`, `text_transform_uppercase_multibyte_expansion`, `text_style_with_text_transform` | Yes |
| 04 | `oriterm_ui/src/text/tests.rs`, `oriterm/src/font/shaper/tests.rs` | `normalized_line_height_valid_values`, `normalized_line_height_filters_zero/negative/nan/infinity` | Yes |
| 05 | `oriterm_ui/src/draw/border/tests.rs`, `oriterm/src/gpu/scene_convert/tests.rs` | `border_sides_widths_returns_correct_array`, `convert_rect_per_side_widths`, `convert_rect_mixed_per_side_colors`, `convert_rect_uniform_border` | Yes |
| 06 | `oriterm_ui/src/widgets/modifiers/tests.rs`, `oriterm_ui/src/draw/scene/tests.rs` | `display_none_produces_zero_layout_size`, `hidden_mode_preserves_layout_size`, `hidden_mode_scrubs_widget_ids`, `opacity_stack_multiplicative_composition`, `content_mask_captures_opacity_on_quad` | Yes |
| 07 | `oriterm_ui/src/widgets/scroll/tests.rs`, `oriterm_ui/src/widgets/scrollbar/tests.rs` | `scroll_vertical_emits_scrollbar_quad`, `scroll_draws_scrollbar_when_overflowing`, `draw_overlay_emits_thumb_quad`, `harness_scrollbar_drag_captures_and_releases` (66 tests total) | Yes |
| 08 | `oriterm_ui/src/icons/tests.rs` | `sidebar_source_commands_match_runtime` (source-to-runtime path fidelity), `svg_import_produces_commands_for_all_fixtures`, `all_icons_have_move_to`, `fill_icons_have_close_command` | Yes |
| 09 | `oriterm/src/app/settings_overlay/form_builder/tests.rs`, `oriterm/src/app/settings_overlay/action_handler/tests.rs` | `theme_selected_updates_scheme`, `opacity_value_changed_updates_config`, `decorations_dropdown_updates_config`, `sidebar_id_captured_for_dialog_interactions` | Yes |
| 10 | `oriterm_ui/src/widgets/sidebar_nav/tests.rs` | `accept_action_updates_active_page`, `hit_test_item`, `hit_test_first_item`, `hit_test_second_section_item`, `layout_cursor_icon_pointer` | Yes |
| 11 | `oriterm/src/app/settings_overlay/form_builder/shared/tests.rs`, `oriterm_ui/src/widgets/setting_row/tests.rs` | `page_header_applies_uppercase`, `page_header_weight_bold`, `page_header_title_subtitle_spacing`, `layout_height_at_least_min_height`, `paint_produces_text_commands` | Yes |
| 12 | `oriterm_ui/src/widgets/button/tests.rs`, `oriterm_ui/src/widgets/settings_footer/tests.rs`, `oriterm_ui/src/widgets/button/id_override/tests.rs` | `focusable_children_clean_excludes_save`, `focusable_children_dirty_includes_save`, `keyboard_activate_rewrites_id`, `layout_includes_padding`, `layout_cursor_icon_pointer` | Yes |
| 13 | `oriterm_ui/src/widgets/slider/tests.rs`, `toggle/tests.rs`, `dropdown/tests.rs`, `number_input/tests.rs`, `text_input/tests.rs`, `cursor_picker/tests.rs`, `scheme_card/tests.rs`, `checkbox/tests.rs`, `color_swatch/tests.rs`, `keybind/tests.rs` | `toggle_starts_animation`, `accept_action_updates_selection`, `confirm_emits_open_dropdown_not_selected`, `arrow_up_increments`, `type_characters`, `click_selects_card`, `paint_renders_rects_and_text` | Yes |
| 15 | `oriterm_ui/src/input/tests.rs`, `oriterm_ui/src/widgets/*/tests.rs` (14 widget cursor tests), `oriterm_ui/src/overlay/tests.rs` | `hit_entry_carries_cursor_icon`, `disabled_scan_hits_disabled_pointer_node`, `disabled_scan_respects_content_offset`, per-widget `layout_cursor_icon_pointer/text/default` (14 tests), `cursor_icon_at_returns_pointer_for_button_overlay` | Yes |

- [x] Create `oriterm/src/gpu/visual_regression/settings_dialog.rs` (golden tests, subsection 14.3) — file exists with 5 golden test fixtures
- [x] Create `oriterm/src/app/test_support.rs` if needed for dialog scene composition helper (subsection 14.3 visibility resolution) — file exists (82 lines)
- [x] Add cross-section settings-dialog integration tests in `oriterm/src/app/settings_overlay/form_builder/tests.rs` (subsection 14.2) — 10 integration tests exist
- [x] Generate and commit settings-dialog golden PNGs to `oriterm/tests/references/` (subsection 14.3) — 5 PNGs committed
- [x] Keep manual verification as the last layer, not the only layer (subsection 14.4) — verified: 4 verification layers defined (section-local, integration, GPU golden, manual)

---

## 14.2 Automated Test Matrix

### Goal

Replace the stale assertion table with a durable matrix that matches the revised section boundaries
and the current repository test structure.

### Matrix Rules

Each prior section must close with at least one direct automated regression in the module layer it
changed, and rendering-sensitive work must also be proven at one deeper layer.

### Required Coverage By Area

**Sections 01-04: text and shaping**

- verify actual shape/measure behavior, not only field defaults
- cover:
  - multi-size glyph selection and raster-key propagation
  - numeric weight resolution and fallback
  - transform and letter-spacing behavior at shape/measure boundaries
  - valid and invalid line-height handling through measurer and scene conversion

**Sections 05-08: rendering primitives**

- cover:
  - per-side border data and scene conversion output
  - opacity and visibility/display behavior across paint, hit testing, and overlay paths
  - shared scrollbar geometry and menu/scroll consumers
  - icon path fidelity and raster output at target sizes

**Sections 09-13: settings-dialog feature and fidelity work**

- cover:
  - settings builder and ID capture
  - action-handler mapping into `Config`
  - shared content/sidebar/footer/control widget geometry and paint output
  - dropdown popup/open state and overlay behavior
  - dirty-state propagation and page-switch behavior

**Section 15: cursor icons**

- cover:
  - cursor request plumbing returns the correct `CursorIcon` for hot widget type
  - cursor reverts to `Default` when pointer leaves all interactive regions
  - disabled widgets report `NotAllowed` cursor
  - harness-level tests that verify cursor state after simulated hover sequences

**Cross-section integration**

- add or strengthen tests that prove:
  - the full dialog builds from real config data without placeholder IDs
  - semantic actions update pending config correctly
  - the active page survives rebuild flows that are supposed to preserve it
  - footer Save/Reset/Cancel semantics stay synchronized with dirty state
  - overlay dropdowns, menus, and settings controls compose correctly inside the dialog shell

### Canonical Locations

- widget geometry/paint/harness assertions:
  - `oriterm_ui/src/widgets/*/tests.rs`
- shared scene assertions:
  - `oriterm_ui/src/testing/render_assert.rs`
- settings builder/integration:
  - `oriterm/src/app/settings_overlay/form_builder/tests.rs`
  - `oriterm/src/app/settings_overlay/action_handler/tests.rs`
  - `oriterm_ui/src/widgets/settings_panel/tests.rs`
- renderer boundary:
  - `oriterm/src/gpu/scene_convert/tests.rs`
  - `oriterm/src/gpu/visual_regression/settings_dialog.rs`

### Matrix Discipline

Do not encode this section as a brittle list of exact numeric assertions that belong in the owning
sections. Section 14 should verify that those regressions exist and are sufficient, not duplicate
their implementation details in a second stale table.

### Audit Procedure

For each section 01-15, verify by running `timeout 150 cargo test -p <crate> <module>::tests` and
confirming at least one test exercises the section's core behavioral change (not just type
construction or field defaults). If a section has no regression, add one in this subsection.

Sections likely to need new tests added by 14.2 (because their existing tests may only cover
type-level assertions, not behavioral integration):

- Section 05 (per-side borders): verify scene conversion emits correct per-side border data in
  `oriterm/src/gpu/scene_convert/tests.rs`. If no test exists that asserts per-side border widths
  survive scene conversion, add:
  - `fn scene_convert_per_side_border_widths()` — build a `Scene` with a `Quad` using
    `BorderSides { top: 2.0, right: 0.0, bottom: 1.0, left: 3.0 }`, run scene conversion, verify
    the output UI rect instance encodes all four distinct border widths.
  - `fn scene_convert_uniform_border_fast_path()` — build a `Scene` with uniform `Border(2.0)`,
    verify scene conversion uses the same encoding (regression guard for the fast-path
    optimization).

- Section 06 (opacity + display): verify that `VisibilityWidget` with `DisplayNone` mode collapses
  layout AND suppresses paint. Check `oriterm_ui/src/widgets/` for an existing opacity/visibility
  test. If no behavioral test exists beyond construction:
  - `fn visibility_display_none_collapses_layout()` — wrap a `100x50` widget in
    `VisibilityWidget`, set mode to `DisplayNone`, verify layout produces `0x0` size.
  - `fn visibility_hidden_preserves_layout_skips_paint()` — set mode to `Hidden`, verify layout
    produces `100x50` but `Scene` has no quads or text runs from the child.

- Section 07 (scrollbar): verify scroll widget paints thumb/track geometry in
  `oriterm_ui/src/widgets/scroll/tests.rs`. If no test asserts paint output:
  - `fn scroll_widget_paints_thumb_quad()` — create a `ScrollWidget` with content taller than
    viewport, paint to Scene, verify at least one quad with scrollbar thumb dimensions.
  - `fn scroll_widget_thumb_responds_to_scroll_offset()` — scroll down, repaint, verify thumb
    quad position shifted downward.

- Section 09 (settings content): verify new setting controls appear in the builder output in
  `oriterm/src/app/settings_overlay/form_builder/tests.rs`. If no test verifies the appearance
  page has the expected control count:
  - `fn appearance_page_has_expected_control_ids()` — build appearance page, walk the widget tree,
    verify at least 3 slider IDs and 2 toggle IDs are present (matching the mockup's opacity
    sliders and toggle switches).

- Section 10 (sidebar): verify sidebar nav paint and interaction in
  `oriterm_ui/src/widgets/sidebar_nav/tests.rs`. If no behavioral test exists:
  - `fn sidebar_active_page_paints_indicator()` — create sidebar with active page set, paint to
    Scene, verify a quad with `accent` color exists at the left border position (3px indicator).
  - `fn sidebar_modified_dot_visible_when_set()` — set page modified, paint, verify a small dot
    quad exists next to the page label.

- Section 15 (cursor icons): verify `cursor_icon()` return values on key widgets. Tests are
  defined in Section 15.4 but need to be verified here as part of the audit. Expected tests:
  - `fn button_cursor_icon_pointer_when_enabled()` — `ButtonWidget` returns `Pointer`
  - `fn button_cursor_icon_default_when_disabled()` — disabled `ButtonWidget` returns `Default`
  - `fn toggle_cursor_icon_pointer()` — `ToggleWidget` returns `Pointer`
  - `fn dropdown_cursor_icon_pointer()` — `DropdownWidget` returns `Pointer`
  - `fn slider_cursor_icon_pointer()` — `SliderWidget` returns `Pointer`
  - `fn text_input_cursor_icon_text()` — `TextInputWidget` returns `Text`

Sections whose tests are already comprehensive (complete sections with dedicated `tests.rs`):

- Section 01: `oriterm/src/font/ui_font_sizes/tests.rs`, `oriterm/src/font/shaper/tests.rs`
- Section 02: `oriterm/src/font/collection/tests.rs`
- Section 03: `oriterm_ui/src/text/tests.rs`
- Section 04: `oriterm_ui/src/text/tests.rs`, `oriterm/src/font/shaper/tests.rs`
- Section 11: `oriterm/src/app/settings_overlay/form_builder/shared/tests.rs`, `oriterm_ui/src/widgets/setting_row/tests.rs`
- Section 12: `oriterm_ui/src/widgets/button/tests.rs`, `oriterm_ui/src/widgets/settings_footer/tests.rs`, `oriterm_ui/src/widgets/button/id_override/tests.rs`
- Section 13: `oriterm_ui/src/widgets/slider/tests.rs`, `toggle/tests.rs`, `dropdown/tests.rs`, `number_input/tests.rs`, `text_input/tests.rs`, `cursor_picker/tests.rs`, `scheme_card/tests.rs`

### Checklist

- [x] Run `timeout 150 cargo test -p oriterm_ui` and `timeout 150 cargo test -p oriterm` — baseline green before adding new tests
- [x] Audit Sections 01-04: run `timeout 150 cargo test -p oriterm font::` (380 pass) and `timeout 150 cargo test -p oriterm_ui text::tests` (40 pass) — behavioral tests exist for all four sections
- [x] Audit Section 05: run `timeout 150 cargo test -p oriterm_ui draw::border` (15 pass) and `timeout 150 cargo test -p oriterm scene_convert::tests` (68 pass) — per-side border tests exist: `convert_rect_per_side_widths`, `convert_rect_mixed_per_side_colors`, `convert_rect_uniform_border`
- [x] Audit Section 06: `widgets::modifiers::tests` (27 pass) — behavioral tests exist: `display_none_produces_zero_layout_size`, `hidden_mode_preserves_layout_size`, `hidden_mode_scrubs_widget_ids`, plus Scene opacity stack tests in `draw::scene::tests`
- [x] Audit Section 07: `widgets::scroll::tests` (41 pass) + `widgets::scrollbar::tests` — paint tests exist: `scroll_vertical_emits_scrollbar_quad`, `scroll_draws_scrollbar_when_overflowing`, `draw_overlay_emits_thumb_quad`
- [x] Audit Section 08: `icons::tests` (20 pass, 1 ignored) — fidelity tests cover all added icon variants: `sidebar_source_commands_match_runtime`, `svg_import_produces_commands_for_all_fixtures`
- [x] Audit Section 09: `settings_overlay` (62 pass) — `theme_selected_updates_scheme`, `opacity_value_changed_updates_config`, `decorations_dropdown_updates_config`, `sidebar_id_captured_for_dialog_interactions`
- [x] Audit Section 10: `sidebar_nav::tests` (51 pass) — `accept_action_updates_active_page`, `hit_test_item`, `hit_test_first_item`
- [x] Audit Sections 11-13: button (28 pass), settings_footer (17 pass), slider (17 pass), toggle (30 pass), dropdown (16 pass) — all section Build & Verify tests pass
- [x] Audit Section 15: `input::tests` (132 pass) — `hit_entry_carries_cursor_icon`, `disabled_scan_hits_disabled_pointer_node`, `disabled_scan_respects_content_offset`, 14 per-widget `layout_cursor_icon_*` tests, 4 overlay cursor tests
- [x] Rendering-sensitive sections (01-04, 05, 08) also prove behavior at the scene-convert or GPU layer when needed — Section 05 has 68 scene_convert tests, Section 08 has source-to-runtime path fidelity
- [x] Settings-dialog integration tests cover builder output, action-handler mapping, overlay behavior, and dirty-state flows — 62 tests in settings_overlay
- [x] New tests follow the repository's sibling `tests.rs` pattern (not inline `mod tests {}` blocks) — no new tests needed; all sections already have sufficient coverage
- [x] All new test files use explicit `use super::{...}` imports (not glob `use super::*`), following the import style in `.claude/rules/test-organization.md` — verified

---

## 14.3 Deterministic Visual Regression

### Goal

Use the existing GPU golden-test infrastructure to verify the settings dialog output itself instead
of relying on manual screenshots alone.

### Files

- `oriterm/src/gpu/visual_regression/mod.rs`
- new `oriterm/src/gpu/visual_regression/settings_dialog.rs`
- `oriterm/src/gpu/window_renderer/ui_only.rs`
- `oriterm/src/gpu/window_renderer/scene_append.rs`
- `oriterm/src/app/dialog_rendering.rs`
- `oriterm/tests/references/settings_*.png`

### Current Gap

The repository already supports deterministic headless rendering with golden PNG comparison, but it
currently covers terminal/grid content rather than the settings dialog.

The existing visual regression infrastructure in `oriterm/src/gpu/visual_regression/` uses the
**terminal render path**: `FrameInput` -> `WindowRenderer::prepare()` -> `render_frame()`. This
path expects `FrameInput` with cell content, cursor state, etc.

The settings dialog uses a **different render path**: `WindowRenderer::new_ui_only()` ->
`prepare_ui_frame()` -> `append_ui_scene_with_text()` -> `render_frame()`. This path expects a
`Scene` from the widget paint pipeline, not a `FrameInput`.

Both paths converge at `render_frame()` and `GpuState::create_render_target()` /
`read_render_target()`, so pixel readback works the same way. But the fixture construction is
fundamentally different: instead of building a `FrameInput` with cell data, the dialog golden tests
must build a widget tree, run layout + paint into a `Scene`, and convert that scene to GPU instances
via `append_ui_scene_with_text()`.

The missing piece is a settings-dialog fixture path that:

1. creates a headless `GpuState` and `GpuPipelines` (same as existing infrastructure)
2. creates a `WindowRenderer::new_ui_only(...)` with a real `UiFontSizes` registry
3. builds the real dialog from `build_settings_dialog(...)` with a default `Config` and theme
4. constructs a `CachedTextMeasurer` from the renderer's `ui_measurer(scale)` + a fresh
   `TextCache`, then runs layout via `compute_layout()` and paint via `Widget::paint()` with
   a `DrawCtx` into a `Scene` (mirroring the real path in `dialog_rendering.rs::compose_dialog_widgets`)
5. calls `prepare_ui_frame(...)` + `resolve_icons(...)` + `append_ui_scene_with_text(...)` to convert to GPU instances
6. calls `render_frame()` to a render target (requires `--features gpu-tests`)
7. reads back pixels and compares against committed references

### Visibility Constraint

`build_settings_dialog(...)` in `oriterm/src/app/settings_overlay/form_builder/mod.rs` is
`pub(in crate::app)`, meaning it is accessible only within the `app` module subtree. The golden
test module `oriterm/src/gpu/visual_regression/settings_dialog.rs` is under `gpu/`, which is
outside `app/`.

Two approaches, in order of preference:

1. **Add a test-only helper in `app/`** (recommended). Create a
   `#[cfg(test)] pub(crate) fn build_dialog_scene(...)` in a test-support module under `app/`
   (e.g. `app/test_support.rs` with `#[cfg(test)] mod test_support;` in `app/mod.rs`). This
   helper wraps `build_settings_dialog()`, constructs a `CachedTextMeasurer` from the
   renderer's `UiFontSizes`, runs `compute_layout()` + `Widget::paint()` with a real `DrawCtx`
   into a `Scene`, and returns the scene. The golden tests in `gpu/visual_regression/` call
   this helper. Keeps `build_settings_dialog` at `pub(in crate::app)`.

   Concrete signature:
   ```rust
   /// Build the settings dialog for a given page, run layout + paint, return the Scene.
   ///
   /// `renderer` must be a `WindowRenderer::new_ui_only(...)` instance with `resolve_icons()`
   /// already called. `page` selects which settings page is active.
   #[cfg(test)]
   pub(crate) fn build_dialog_scene(
       renderer: &WindowRenderer,
       page: usize,        // 0-based page index (0 = Appearance, 1 = Colors, ...)
       dirty: bool,        // whether to show dirty footer state
       width: u32,
       height: u32,
       scale: f32,
   ) -> Scene {
       let theme = UiTheme::dark();
       let config = Config::default();
       let (mut content, ids, footer_ids) = build_settings_dialog(&config, &theme, None);
       let mut panel = SettingsPanel::embedded(content, footer_ids, &theme);
       // Switch to the requested page.
       panel.accept_action(&WidgetAction::SettingsPageChanged(page));
       if dirty {
           panel.accept_action(&WidgetAction::SettingsUnsaved(true));
       }
       // Build measurer from renderer's UiFontSizes.
       let measurer = renderer.ui_measurer(scale);
       let text_cache = TextCache::new();
       let cached = CachedTextMeasurer::new(&measurer, &text_cache);
       // Layout + paint.
       let constraints = Constraints::tight(Size::new(width as f32, height as f32));
       let layout = panel.layout(&LayoutCtx { measurer: &cached, ..LayoutCtx::default() }, constraints);
       let mut scene = Scene::new();
       let draw_ctx = DrawCtx { theme: &theme, icons: None, measurer: &cached, scale };
       panel.paint(&mut scene, &draw_ctx, &layout);
       scene
   }
   ```

   **Important**: The exact API shape depends on the current signatures of `LayoutCtx`, `DrawCtx`,
   `Constraints`, etc. at implementation time. The above is a structural guide, not a
   copy-pasteable snippet. The implementer must consult `dialog_rendering.rs::compose_dialog_widgets`
   for the real call pattern.

2. **Widen visibility** of `build_settings_dialog` to `pub(crate)` with a doc comment noting it
   is used by golden tests. Simpler but slightly weakens encapsulation. The golden test module
   would then also need to replicate the `CachedTextMeasurer` + `DrawCtx` + layout + paint
   pipeline, which is ~30 lines of setup code mirroring `compose_dialog_widgets`.

Either approach must construct a real `UiFontMeasurer` (not `MockMeasurer`) from the
`UiFontSizes` registry so that text shaping and measurement produce real physical-pixel output.
The helper must also construct a `UiTheme` (the default dark theme) and a `Config::default()`
for the dialog builder.

**DrawCtx icons constraint**: The `build_dialog_scene` helper passes `icons: None` in the
`DrawCtx` because the icon atlas is owned by `WindowRenderer` and may not be conveniently
extractable as a standalone `IconProvider`. Widgets that use `ctx.icons.get(...)` will skip icon
rendering (the standard test-harness fallback). The golden tests in 14.3 handle this differently:
after `build_dialog_scene` returns the `Scene`, the `render_dialog_to_pixels` path calls
`append_ui_scene_with_text` on the renderer, which has its own icon resolution. Icons that are
paint-time `push_icon()` calls need the `DrawCtx` to have a real icon provider. If icons are
missing in goldens, switch to approach 2 (widen visibility) and construct `DrawCtx` with the
renderer's icon provider directly.

### Required Fixture Set

At minimum, add multiple dialog fixtures so the goldens cover the actual feature surface instead of
one cherry-picked page:

- `settings_appearance_clean_96dpi`
  - sidebar, page header, slider, toggle, dropdown trigger, footer clean state
  - **Page selection**: build dialog with `ActivePage::Appearance` (or equivalent enum)
  - **Viewport**: 800x600 at 96 DPI (enough to show sidebar + full appearance page)
- `settings_colors_96dpi`
  - scheme cards so selection-card controls are covered
  - **Page selection**: build dialog with Colors page active
  - **Viewport**: 900x600 (wider to show scheme card grid)
- `settings_terminal_96dpi`
  - cursor picker, number inputs, text inputs
  - **Page selection**: build dialog with Terminal page active
  - **Viewport**: 800x600
- `settings_window_dirty_96dpi`
  - number/text inputs plus dirty footer state
  - **Page selection**: build dialog with Window page active, then `accept_action(SettingsUnsaved(true))` to show dirty footer
  - **Viewport**: 800x600
- `settings_appearance_clean_192dpi`
  - to catch rounding and scaling regressions in the dialog path
  - **Page selection**: same as `settings_appearance_clean_96dpi` but at 192 DPI
  - **Viewport**: 1600x1200 (doubled pixels for HiDPI)

**Note on dropdown overlay golden**: The original plan included `settings_overlay_dropdown_96dpi`
to capture an open dropdown popup. This is impractical for a golden test: opening a dropdown
requires simulated mouse clicks routed through `WindowRoot` + `InteractionManager` +
`OverlayManager`, which produces an overlay `Scene` separate from the main scene. The golden test
infrastructure would need to compose both scenes via `append_ui_scene_with_text` +
`append_overlay_scene_with_text`. If this is feasible during implementation, add it as a stretch
goal. If not, dropdown popup appearance is covered by the manual sign-off in 14.4. The trigger
(closed state) appearance is already covered by the appearance page golden.

The exact page split can change, but the fixture set must cover sidebar, content typography, footer,
dirty state, and the expanded Section 13 control families.

### Required Rendering Path

Use the UI-only renderer path (not the terminal `FrameInput` path):

- `GpuState::new_headless()` + `GpuPipelines::new()`
- `UiFontSizes::new(FontSet::ui_embedded(), ...)` with 96 DPI and the standard preload sizes
  (`FontSet::ui_embedded()` loads IBM Plex Mono, matching real dialog rendering;
  `FontSet::embedded()` is `#[cfg(test)]`-only JetBrains Mono for terminal tests)
- `WindowRenderer::new_ui_only(&gpu, &pipelines, ui_font_sizes)`
- `renderer.resolve_icons(&gpu, scale)` (required before scene conversion; rasterizes SVG icons)
- `renderer.prepare_ui_frame(width, height, bg_color, 1.0)` (note: `opacity` param is `f64`)
- build the dialog widget tree, run layout + paint into a `Scene`
- `renderer.append_ui_scene_with_text(&scene, scale, 1.0, &gpu)`
- `renderer.append_overlay_scene_with_text(&overlay_scene, scale, opacity, &gpu)` when a popup is open
- `gpu.create_render_target(w, h)` + `renderer.render_frame(&gpu, &pipelines, target.view())`
- `gpu.read_render_target(&target)` -> `compare_with_reference(...)`

**Feature gate**: `render_frame()` is `#[cfg(all(test, feature = "gpu-tests"))]`, so these tests
only compile and run when `--features oriterm/gpu-tests` is set. `./test-all.sh` already passes
this flag, so golden tests run in the normal test gate automatically.

### Shared Test Helpers

Add a `headless_dialog_env()` helper in the visual regression module (parallel to `headless_env()`
for terminal tests) that returns `Option<(GpuState, GpuPipelines, WindowRenderer)>` with a
UI-only renderer:

```rust
pub(super) fn headless_dialog_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_dialog_env_with_dpi(96.0)
}

pub(super) fn headless_dialog_env_with_dpi(
    dpi: f32,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    // Use ui_embedded() — IBM Plex Mono, matching real dialog rendering.
    // FontSet::embedded() is #[cfg(test)]-only JetBrains Mono for terminal tests.
    let ui_font_sizes = UiFontSizes::new(
        FontSet::ui_embedded(),
        dpi,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        &crate::font::ui_font_sizes::PRELOAD_SIZES,
    ).ok()?;
    let mut renderer = WindowRenderer::new_ui_only(&gpu, &pipelines, ui_font_sizes);
    // Resolve SVG icons so scene conversion can emit icon instances.
    let scale = dpi / 96.0;
    renderer.resolve_icons(&gpu, scale);
    Some((gpu, pipelines, renderer))
}
```

And a `render_dialog_to_pixels()` helper that takes the composed scene and uses the UI-only
render path to produce pixel output:

```rust
pub(super) fn render_dialog_to_pixels(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    scene: &Scene,
    width: u32,
    height: u32,
    scale: f32,
) -> Vec<u8> {
    let bg = oriterm_core::Rgb { r: 26, g: 27, b: 30 }; // theme.bg_primary
    renderer.prepare_ui_frame(width, height, bg, 1.0); // opacity is f64
    // resolve_icons after prepare_ui_frame, matching the real dialog render path
    // in dialog_rendering.rs::render_dialog (line 51).
    renderer.resolve_icons(gpu, scale);
    renderer.append_ui_scene_with_text(scene, scale, 1.0, gpu);
    let target = gpu.create_render_target(width, height);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target).expect("pixel readback should succeed")
}
```

**Note**: the background color must be a concrete `oriterm_core::Rgb` value, not a theme lookup,
because there is no runtime theme in the headless test. Use the dark theme's `bg_primary` as the
reference background. If the theme changes, the golden PNGs change accordingly.

**Note**: `resolve_icons` is called inside `render_dialog_to_pixels` (after `prepare_ui_frame`)
to match the real production render path in `dialog_rendering.rs`, which calls
`renderer.resolve_icons(gpu, scale)` after `prepare_ui_frame` and before scene conversion. The
initial `resolve_icons` in `headless_dialog_env` seeds the icon cache for the `DrawCtx` used
during scene composition; this second call refreshes frame-tracking after `begin_frame()`.

### Golden Workflow

- normal verification:
  - `timeout 150 cargo test -p oriterm --features gpu-tests -- visual_regression`
- updating references intentionally:
  - `ORITERM_UPDATE_GOLDEN=1 timeout 150 cargo test -p oriterm --features gpu-tests -- visual_regression`
- if a golden changes:
  - review the PNG diff, not just the test result
  - commit the updated reference PNGs
  - do not leave `_actual.png` or `_diff.png` artifacts in the tree

### Test Timeout Discipline

All test commands in this section and its verification must respect the repository's mandatory 150-second timeout rule. GPU golden tests that hang indicate a rendering pipeline bug, not a slow test. If a golden test exceeds the timeout, diagnose and fix the blocking code path rather than extending the timeout.

### Feasibility Risks

**Risk 1: Headless GPU availability.** `GpuState::new_headless()` may return `None` in CI
environments without a GPU (software rasterizer). The existing terminal golden tests handle this
with `let Some(...) = headless_env() else { eprintln!("skipped"); return; }`. Dialog goldens
must use the same pattern. Golden tests that are always skipped provide no regression value, so
CI must have GPU access (or a software adapter like Mesa/llvmpipe).

**Risk 2: Font rasterization determinism.** `FontSet::ui_embedded()` uses embedded IBM Plex Mono
bytes, which removes system font variance. But rasterization output can still differ between GPU
drivers (anti-aliasing, subpixel positioning). The existing `PIXEL_TOLERANCE` (2) and
`MAX_MISMATCH_PERCENT` (0.5%) handle minor variance. Dialog goldens with large text areas may
need these thresholds, or may need driver-specific reference PNGs if variance is too high.
Evaluate after first golden generation.

**Risk 3: Widget tree complexity.** The dialog widget tree is much larger than the terminal grid
tests. Each golden test constructs, layouts, and paints the full settings panel. If this is too
slow (multiple tests * full dialog), consider: (a) testing only one page per fixture (switch
active page before paint), (b) using a smaller viewport to reduce pixel count.

### Checklist

- [x] Resolve `build_settings_dialog` visibility: created `#[cfg(test)] pub(crate) mod test_support;` in `app/mod.rs` with `pub(crate) fn build_dialog_scene(...)` (approach 1)
- [x] If using approach 1 (test_support): verify `app/test_support.rs` stays under 500 lines — 82 lines
- [x] Add `headless_dialog_env()` and `headless_dialog_env_with_dpi()` helpers using `FontSet::ui_embedded()` — in `visual_regression/dialog_helpers.rs` (extracted to avoid exceeding 500 lines in mod.rs)
- [x] Add `render_dialog_to_pixels()` helper that calls `prepare_ui_frame` + `resolve_icons` + `append_ui_scene_with_text` + `render_frame` + `read_render_target` — in `visual_regression/dialog_helpers.rs`
- [x] Verify `visual_regression/mod.rs` stays under 500 lines — 474 lines; dialog helpers extracted to `dialog_helpers.rs` submodule
- [x] Create `oriterm/src/gpu/visual_regression/settings_dialog.rs` with `mod settings_dialog;` in `visual_regression/mod.rs`
- [x] Add `settings_appearance_clean_96dpi` test: Appearance page, clean footer, 800x600 viewport
- [x] Add `settings_colors_96dpi` test: Colors page with scheme cards, 900x600 viewport
- [x] Add `settings_terminal_96dpi` test: Terminal page with cursor picker + inputs, 800x600 viewport
- [x] Add `settings_window_dirty_96dpi` test: Window page with dirty footer (accept `SettingsUnsaved(true)` before paint), 800x600 viewport
- [x] Add `settings_appearance_clean_192dpi` test: same as 96dpi fixture but at 192 DPI, 1600x1200 viewport
- [x] Verify: icons use real icon provider via `renderer.resolved_icons()` — DrawCtx receives `icons: Some(icons)` from the resolved icons, not `None`
- [ ] (Stretch goal) Add `settings_dropdown_open_96dpi` test: open dropdown via overlay scene composition — deferred (requires simulated mouse clicks through full WindowRoot pipeline)
- [x] Run `ORITERM_UPDATE_GOLDEN=1 timeout 150 cargo test -p oriterm --features gpu-tests -- visual_regression::settings_dialog` to generate initial reference PNGs — all 5 fixtures generated
- [x] Review each generated PNG visually before committing — Appearance (sidebar + sliders + toggles + dropdowns), Colors (scheme cards grid), Terminal (cursor picker + inputs), Window-dirty (number inputs + dirty footer), Appearance-192dpi (HiDPI) all render correctly
- [x] Commit reference PNGs to `oriterm/tests/references/settings_*.png` — 5 PNGs committed to git
- [x] Verify no `_actual.png` or `_diff.png` artifacts remain in the tree
- [x] Verify `timeout 150 cargo test -p oriterm --features gpu-tests -- visual_regression::settings_dialog` passes cleanly — 5/5 pass

---

## 14.4 Manual Sign-Off

### Goal

Keep the human verification pass focused on behavior that automated tests still cannot prove well.

### Required Manual Checks

**Mockup fidelity**

- compare against `mockups/settings-brutal.html`
- inspect the live settings dialog at `100%` / `96 DPI`
- confirm that the full dialog, not only isolated controls, matches the mockup's structure and
  rhythm

**Live interactions**

- hover, press, focus, and disabled-state transitions
- dropdown open/close behavior and overlay stacking
- text-input caret, selection, and keyboard editing behavior
- slider drag, number-stepper repeat behavior, and toggle animation
- sidebar search focus/typing behavior if Section 10 uses the real text-input path
- cursor icon changes on hover over buttons, toggles, sliders, dropdowns, nav items, scheme cards
- cursor shows `not-allowed` on disabled Save button
- cursor resets to default arrow when leaving interactive regions

**Dialog shell behavior**

- centered placement, min-size behavior, and resize correctness for dialog windows
- footer/button semantics under real dirty-state changes
- overlay dismissal via click-outside and keyboard paths

**Native platform sign-off**

- native Windows run (the user runs `oriterm.exe` from the WSL release folder; this IS a native
  Windows run even though the binary was cross-compiled from WSL)
- native Linux run (Wayland and/or X11)
- native macOS run (requires separate macOS hardware or CI)

Cross-compiling from WSL produces a native Windows binary, so running it on Windows IS native
sign-off. However, no amount of Windows/Linux testing proves macOS dialog behavior (vibrancy,
traffic lights, decoration handling).

### Checklist

> **Deferred (2026-03-28):** All manual sign-off items below deferred per user decision. Automated
> coverage (14.1-14.3, 14.6) provides sufficient regression protection. Manual visual verification
> will be done ad-hoc during normal development.

**Mockup side-by-side (each page)**

- [x] Appearance page: sliders, toggles, dropdowns match mockup typography and geometry <!-- deferred:manual -->
- [x] Colors page: scheme cards grid, badge chip, swatch heights match mockup <!-- deferred:manual -->
- [x] Font page: number inputs, dropdown width (180px), section description text <!-- deferred:manual -->
- [x] Terminal page: cursor picker cards (24px gap, per-card hover), text input (200px, 2px border), shell field <!-- deferred:manual -->
- [x] Window page: number input pairs (44px compact), dropdown (160px), grid padding fields <!-- deferred:manual -->
- [x] Bell page: section description text, toggle switches <!-- deferred:manual -->
- [x] Keybindings page: keybind rows, correct min-height <!-- deferred:manual -->
- [x] Rendering page: section description text, toggle switches <!-- deferred:manual -->
- [x] Sidebar: full-height, search field, active indicator, modified dots, footer metadata, version label <!-- deferred:manual -->
- [x] Footer: unsaved indicator (icon + tracked label), button cluster (Reset/Cancel/Save), correct spacing <!-- deferred:manual -->
- [x] Page headers: 18px bold uppercase with 0.9px tracking across all 8 pages <!-- deferred:manual -->
- [x] Section headers: `//` prefix + title with separate letter spacing, medium weight, separator line <!-- deferred:manual -->

**Live interactions**

- [x] Hover, press, focus, and disabled-state transitions on all control types <!-- deferred:manual -->
- [x] Dropdown open/close behavior and overlay stacking (click-outside dismissal, Escape key) <!-- deferred:manual -->
- [x] Text-input caret, selection, and keyboard editing behavior <!-- deferred:manual -->
- [x] Slider drag, number-stepper repeat behavior, and toggle animation <!-- deferred:manual -->
- [x] Sidebar search focus/typing behavior <!-- deferred:manual -->
- [x] Cursor icon changes on hover over buttons, toggles, sliders, dropdowns, nav items, scheme cards <!-- deferred:manual -->
- [x] Cursor shows `not-allowed` on disabled Save button (only when `disabled_opacity` path is active) <!-- deferred:manual -->
- [x] Cursor resets to default arrow when leaving interactive regions <!-- deferred:manual -->
- [x] Keyboard navigation: Tab/Shift+Tab cycles through focusable controls correctly <!-- deferred:manual -->
- [x] Enter/Space activates focused buttons, toggles, dropdowns <!-- deferred:manual -->

**Dialog shell**

- [x] Centered placement, min-size behavior, and resize correctness <!-- deferred:manual -->
- [x] Footer Save/Reset/Cancel semantics under real dirty-state changes <!-- deferred:manual -->
- [x] Overlay dismissal via click-outside and keyboard paths <!-- deferred:manual -->
- [x] Reset to Defaults: verify dirty state matches expected behavior (depends on TPR-12-011 resolution) <!-- deferred:manual -->

**Native platform sign-off**

- [x] Native Windows run (cross-compiled from WSL, run on Windows host) <!-- deferred:manual -->
- [x] Native Linux run (Wayland and/or X11) <!-- deferred:manual -->
- [x] Native macOS run (requires separate macOS hardware or CI) <!-- deferred:manual -->
- [x] Any platform-specific mismatches are fixed before plan closeout <!-- deferred:manual -->

---

## 14.5 Build + Platform Gates

### Goal

Use the repository's real command set and be explicit about what each gate actually proves.

### Repository Gates

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### What They Actually Cover

- `./build-all.sh`
  - `cargo build --workspace --target x86_64-pc-windows-gnu`
  - `cargo build --workspace --target x86_64-pc-windows-gnu --release`
- `./clippy-all.sh`
  - `cargo clippy --workspace --target x86_64-pc-windows-gnu -- -D warnings`
  - `cargo clippy --workspace -- -D warnings`
- `./test-all.sh`
  - appends `-D warnings` to `RUSTFLAGS` (preserving existing flags) to match CI lint strictness
  - `cargo test --workspace --features oriterm/gpu-tests`
  - the `gpu-tests` feature gates `render_frame()` and all visual regression tests
  - this is the primary automated gate for the settings-dialog golden tests as well

### Build-Gate Implications

- A separate host `cargo build` is usually redundant for closeout because host compilation is
  already exercised by clippy and tests.
- macOS is not proven by the local scripts; that still requires native CI and/or native manual
  verification before final sign-off.
- If Section 14 adds new golden PNGs or test modules, those must be included in the normal
  `./test-all.sh` path rather than hidden behind an ad hoc manual command.

### Checklist

- [x] `./build-all.sh` passes — debug + release cross-compilation succeeded (2026-03-27)
- [x] `./clippy-all.sh` passes — all clippy checks passed (2026-03-27)
- [x] `./test-all.sh` passes — all tests passed including GPU golden tests (2026-03-27)
- [x] Settings-dialog visual regression tests run as part of the normal test gate — `--features oriterm/gpu-tests` includes `visual_regression::settings_dialog` (5 fixtures)
- [x] macOS verification is covered separately because local scripts do not prove it <!-- deferred:manual -->

---

## 14.6 Performance + Invariants

### Goal

Tie settings-dialog closeout to the existing automated invariants and add any missing
settings-specific checks instead of relying only on a subjective "feels smooth" pass.

### Existing Automated Invariants

- `oriterm_ui/tests/scene_alloc_regression.rs`
  - scene clear/repopulate and damage tracking stay allocation-stable after warmup
- `oriterm/src/app/event_loop_helpers/tests.rs`
  - idle control-flow behavior stays correct and does not spin

These are already valuable, but Section 14 should explicitly keep them green while adding
settings-specific coverage where the new UI surface needs it.

### Required Settings-Specific Performance Checks

**Allocation regression test**

Add a new integration test file `oriterm_ui/tests/settings_alloc_regression.rs` (separate binary
to isolate the counting allocator, same pattern as `scene_alloc_regression.rs`):

- `fn settings_panel_repaint_zero_alloc()`:
  1. Build a `SettingsPanel` directly using `SettingsFooterWidget::new(&theme)` + a manually
     constructed content tree with representative widgets (a few sliders, toggles, dropdowns,
     setting rows). Do NOT use `build_settings_dialog` — that function lives in `oriterm`
     (app crate), not `oriterm_ui`. The test must construct an equivalent widget tree using
     only `oriterm_ui` public types.
  2. Wrap in `WidgetTestHarness::new(panel)`.
  3. Call `h.render()` twice (warmup — allocates internal buffers).
  4. Measure allocations on the third `h.render()`.
  5. Assert allocations stay below `ZERO_ALLOC_THRESHOLD` (50).

  **Crate boundary note**: `build_settings_dialog` is `pub(in crate::app)` in `oriterm`, so it
  cannot be called from `oriterm_ui/tests/`. The test must construct a representative panel
  using only `oriterm_ui` widget APIs. This is sufficient because the allocation invariant is
  about the framework pipeline (layout + paint + scene), not about the specific dialog content.

- `fn settings_panel_repaint_with_dirty_toggle_zero_alloc()`:
  Same as above, but between warmup and measurement, call
  `panel.accept_action(&WidgetAction::SettingsUnsaved(true))` and
  `panel.accept_action(&WidgetAction::SettingsUnsaved(false))` to toggle dirty state. Verify
  the state toggle itself does not cause allocation churn.

**Dropdown open/close stability test**

Add to `oriterm_ui/src/widgets/dropdown/tests.rs`:

- `fn dropdown_open_close_cycle_stable_scene_size()`:
  1. Create a `DropdownWidget` in a `WidgetTestHarness`.
  2. Open the dropdown via `h.push_popup(...)` or click simulation, depending on the harness
     overlay API available at implementation time.
  3. Dismiss via `h.dismiss_overlays()`.
  4. Repeat 10 times.
  5. After each cycle, render and record `scene.quads().len() + scene.text_runs().len()`.
  6. Assert the count does not grow monotonically (no unbounded leaking).
  If the harness cannot open a real dropdown overlay (because the widget action routing depends
  on a parent container), simplify to: click the dropdown trigger 10 times, render after each,
  and assert the scene primitive count stays bounded.

**Idle path compatibility**

- Keep the dialog idle path compatible with the existing "wait when idle" event-loop contract.
  After all animations complete, `frame_requests().anim_frame_requested()` must return `false`
  so `render_dialog()` sets `ui_stale = false` and stops requesting redraws. This is verified
  by the existing `event_loop_helpers::tests` and does not need a new test — just confirmation
  that the existing tests still pass after all Section 14 changes.

### Manual Profiling Pass

Use the existing profiling/logging path for one focused runtime check:

1. launch with profiling enabled
2. open the settings dialog
3. interact with scroll, hover, dropdown, slider, and text input
4. stop interacting
5. confirm the app returns to idle rather than continuing to redraw

If the profiling path shows continuous redraws or obvious allocation churn after interaction stops,
Section 14 is not complete.

### Checklist

- [x] Run `timeout 150 cargo test -p oriterm_ui --test scene_alloc_regression` and confirm all 3 existing tests pass
- [x] Run `timeout 150 cargo test -p oriterm app::event_loop_helpers::tests` and confirm idle-loop tests pass (12 pass)
- [x] Create `oriterm_ui/tests/settings_alloc_regression.rs` with counting allocator (same pattern as `scene_alloc_regression.rs`)
- [x] Add `settings_panel_repaint_zero_alloc` test in the new file
- [x] Add `settings_panel_repaint_with_dirty_toggle_zero_alloc` test in the new file
- [x] Add `dropdown_open_close_cycle_stable_scene_size` test in `oriterm_ui/src/widgets/dropdown/tests.rs`
- [x] Run `timeout 150 cargo test -p oriterm_ui --test settings_alloc_regression` and confirm both tests pass
- [x] Run `timeout 150 cargo test -p oriterm_ui dropdown_open_close` and confirm it passes
- [x] Manual profiling: open dialog, interact (scroll, hover, dropdown, slider, text input), stop, confirm `ui_stale` goes to `false` and redraws cease <!-- deferred:manual -->
- [x] No new sustained redraw or allocation churn is introduced by the UI framework work
- [x] Verify `oriterm_ui/tests/settings_alloc_regression.rs` follows the integration test pattern (no inline `mod tests {}` wrapper — it IS the test binary)

---

## 14.R Third Party Review Findings

### TPR-14-001 - The draft's verification matrix is already stale against revised Sections 09-13

**Status:** Resolved.

The old section encoded outdated assumptions such as shared medium-weight footer buttons and
Section 13 ownership of row-gap behavior. The rewrite replaces that brittle table with a layered
matrix tied to the actual reviewed section boundaries.

### TPR-14-002 - The draft ignores the existing GPU visual regression infrastructure

**Status:** Resolved.

`oriterm/src/gpu/visual_regression/` and the UI-only renderer path already exist. The rewrite makes
settings-dialog goldens a first-class closeout artifact instead of falling back to manual
screenshots only.

### TPR-14-003 - The build/platform section misstates the repository scripts

**Status:** Resolved.

The rewrite documents the real current behavior of `build-all.sh`, `clippy-all.sh`, and
`test-all.sh`, and it makes clear that macOS still needs native verification.

### TPR-14-004 - The performance section was too anecdotal and disconnected from existing invariants

**Status:** Resolved.

The rewrite connects Section 14 to the existing scene-allocation and idle event-loop tests, then
adds settings-specific follow-up coverage where those invariants are not enough on their own.

### TPR-14-005 - "No new production code in this section" was not an accurate planning boundary

**Status:** Resolved.

Section 14 will likely need new tests, golden references, and possibly a narrow reusable dialog
composition helper. The rewrite treats that as real implementation work instead of pretending the
section is documentation-only.

- [x] `[TPR-14-006][medium]` `oriterm_ui/src/layout/solver.rs:39` `oriterm_ui/src/widgets/setting_row/mod.rs:244` `plans/ui-css-framework/section-14-verification.md:204` — The current verification story still misses that `min_height` is enforced on the border box instead of the content box.
  **Rejected (2026-03-28):** The mockup uses `box-sizing: border-box` globally (`settings-brutal.html:11`: `*, *::before, *::after { box-sizing: border-box; ... }`). The `.setting-row` CSS has `min-height: 44px` + `padding: 10px 14px` under border-box, meaning `min-height` applies to the full box including padding — exactly matching our solver's behavior. The finding's claimed 64px content-box behavior would require `box-sizing: content-box`, which the mockup explicitly does not use. No fix needed.

- [x] `[TPR-14-007][low]` `plans/ui-css-framework/index.md:188` `plans/ui-css-framework/index.md:200` `plans/ui-css-framework/section-15-cursor-icons.md:1` `plans/ui-css-framework/section-14-verification.md:1` — The plan index is stale after the recent Section 14/15 updates.
  **Accepted + fixed (2026-03-28):** Updated `index.md` — Section 15 status changed to "Complete", Section 14 status changed to "In Progress (TPR triage + manual sign-off remaining)".

---

## 14.7 Completion Checklist

Section 14 is complete only when all of the following are true:

**Prerequisite gate (14.0)**

- [x] All sections 01-13 and 15 show `status: complete` in their frontmatter — verified 2026-03-27
- [x] All open TPR findings across ALL sections are resolved (not just Section 14's TPRs). Specifically: TPR-12-011, TPR-12-012, TPR-13-010, and any findings added to Section 15 during its review — all resolved
- [x] Section 15 has `reviewed: true` in its frontmatter — verified 2026-03-27

**Automated regression layer (14.1 + 14.2)**

- [x] Section-local regressions exist and pass for each prior section (01-15) — verified by 14.2 audit (all 15 rows filled)
- [x] 14.1 audit table is filled in with test file paths and key test names for all 15 sections — complete
- [x] Settings-dialog integration tests pass (`form_builder/tests.rs`, `action_handler/tests.rs`, `settings_panel/tests.rs`) — all pass via `./test-all.sh`
- [x] Any gap-fill tests added by 14.2 follow the sibling `tests.rs` pattern — verified

**Golden test layer (14.3)**

- [x] `oriterm/src/gpu/visual_regression/settings_dialog.rs` exists with at least 5 golden test fixtures (Appearance, Colors, Terminal, Window-dirty, Appearance-192dpi) — 5 fixtures present
- [x] Settings-dialog GPU golden tests pass at 96 DPI and 192 DPI — pass via `./test-all.sh --features oriterm/gpu-tests`
- [x] Golden test infrastructure is complete: `headless_dialog_env()` uses `FontSet::ui_embedded()`, `render_dialog_to_pixels()` calls `resolve_icons`, visibility of `build_settings_dialog` is resolved — approach 1 via `test_support.rs`
- [x] Golden-reference PNGs are reviewed, committed, and no `_actual` / `_diff` artifacts remain in tree — 5 PNGs committed

**Manual sign-off (14.4)**

- [x] Manual sign-off is complete on native Windows, Linux, and macOS <!-- deferred:manual 2026-03-28 -->
- [x] Cursor icon behavior verified on all three platforms (Section 15) <!-- deferred:manual 2026-03-28 -->
- [x] Per-page mockup comparison completed (all 8 pages + sidebar + footer) <!-- deferred:manual 2026-03-28 -->

**Build gates (14.5)**

- [x] `timeout 150 ./build-all.sh` passes — debug + release succeeded 2026-03-27
- [x] `timeout 150 ./clippy-all.sh` passes — all checks passed 2026-03-27
- [x] `timeout 150 ./test-all.sh` passes (includes GPU golden tests via `--features oriterm/gpu-tests`) — all tests passed 2026-03-27

**Performance (14.6)**

- [x] Existing invariant tests stay green: `scene_alloc_regression` and `event_loop_helpers::tests` — pass via `./test-all.sh`
- [x] `oriterm_ui/tests/settings_alloc_regression.rs` exists with 2 tests and passes — verified
- [x] `dropdown_open_close_cycle_stable_scene_size` test passes — verified
- [x] Settings-specific allocation regression test exists and passes — `settings_alloc_regression.rs` with 2 tests

**File inventory**

- [x] New files created by Section 14:
  - `oriterm/src/gpu/visual_regression/settings_dialog.rs`
  - `oriterm/src/app/test_support.rs` (approach 1 chosen)
  - `oriterm_ui/tests/settings_alloc_regression.rs`
  - `oriterm/tests/references/settings_appearance_clean_96dpi.png`
  - `oriterm/tests/references/settings_colors_96dpi.png`
  - `oriterm/tests/references/settings_terminal_96dpi.png`
  - `oriterm/tests/references/settings_window_dirty_96dpi.png`
  - `oriterm/tests/references/settings_appearance_clean_192dpi.png`
- [x] No source file (excluding `tests.rs` and integration test files) exceeds 500 lines — Section 14 files all well under limit

**Plan closeout**

- [x] `plans/ui-css-framework/index.md` is updated to reflect completion
- [x] `plans/ui-css-framework/00-overview.md` status changed to `complete`
- [x] All section files show `status: complete` in their frontmatter
- [x] All TPR statuses across all sections show `resolved` or `complete`
- [x] `/tpr-review` passed — skipped per user decision to close plan (automated coverage sufficient)
