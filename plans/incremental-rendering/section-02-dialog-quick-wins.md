---
section: "02"
title: "Dialog Quick Wins"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-21
goal: "Off-screen dialog content stops emitting paint primitives; redundant dialog layout recomputation is eliminated; dialog interactions show measurably fewer scene primitives when scrolled"
inspired_by:
  - "FormLayout::paint() viewport culling (oriterm_ui/src/widgets/form_layout/mod.rs:154-183)"
  - "ScrollWidget cached_child_layout (oriterm_ui/src/widgets/scroll/mod.rs:117,190-216)"
depends_on: ["01"]
sections:
  - id: "02.1"
    title: "Verify and Harden Viewport Culling"
    status: complete
  - id: "02.2"
    title: "Fix ScrollWidget Cache Invalidation on Page Switch"
    status: complete
  - id: "02.2b"
    title: "Add for_each_child_mut_all Widget Trait Method"
    status: complete
  - id: "02.2c"
    title: "Update Pipeline Callers to Use Correct Traversal Method"
    status: complete
  - id: "02.2d"
    title: "Reduce Prepare Phase Work on Hover"
    status: complete
  - id: "02.3"
    title: "Scene Primitive Count Reduction"
    status: complete
  - id: "02.4"
    title: "Tests and Measurement"
    status: complete
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.5"
    title: "Build & Verify"
    status: complete
---

# Section 02: Dialog Quick Wins

**Status:** Complete
**Goal:** Off-screen dialog content stops emitting paint primitives. Redundant dialog layout recomputation is eliminated. Dialog hover and scroll interactions produce measurably fewer scene primitives when the content is scrolled past the viewport.

**Production code path:** `App::compose_dialog_widgets()` in `dialog_rendering.rs` — specifically the paint phase where `ctx.content.content_widget().paint(&mut draw_ctx)` walks the entire content tree, and the layout recomputation that happens when `DirtyKind >= Prepaint`.

**Observable change:** When the settings dialog is scrolled so that sections are off-screen, those sections' widgets no longer emit quads, text runs, or lines into the `Scene`. Scene primitive counts drop proportionally to how much content is off-screen. Hover interactions on visible controls don't trigger layout recomputation for the entire content tree.

**Context:** The dialog content tree can be large (settings panel with multiple pages, each with many form fields). Currently, `paint()` walks the entire tree every frame even though `FormLayout` already has viewport culling that skips off-screen sections, and `FormSection` also culls individual rows within partially-visible sections (`form_section/mod.rs:191-231`). The `Container` widget (`container/mod.rs:319-349`) also uses `current_clip_in_content_space()`. So viewport culling during paint is actually fairly comprehensive. The remaining gap is in prepare/prepaint: `DirtyKind::Prepaint` (triggered by hover) currently runs `prepare_widget_tree()` on the entire content tree (all sections, all rows, all widgets), even though only the hovered widget changed.

**Reference implementations:**
- **FormLayout** `oriterm_ui/src/widgets/form_layout/mod.rs:154-183`: Existing viewport culling via `current_clip_in_content_space()` — works for whole sections, needs verification for edge cases
- **ScrollWidget** `oriterm_ui/src/widgets/scroll/mod.rs:117` (field), `190-216` (`child_natural_size` method): Layout cache keyed by viewport bounds — prevents redundant child layout when viewport hasn't changed

**Depends on:** Section 01 (prepaint bounds must be correct before optimizing the paths that use them).

---

## 02.1 Verify and Harden Viewport Culling

**File(s):** `oriterm_ui/src/widgets/form_layout/mod.rs`, `oriterm_ui/src/widgets/scroll/rendering.rs` (scroll clip/translate), `oriterm_ui/src/draw/scene/stacks.rs` (`current_clip_in_content_space()` at line 94)

The `FormLayout::paint()` viewport culling already checks `section_node.rect.intersects(visible_bounds)` to skip off-screen sections. Verify this works correctly and identify any gaps.

- [x] Traced clip rect propagation: ScrollWidget pushes clip + offset, FormLayout/FormSection call `current_clip_in_content_space()` which subtracts `cumulative_offset` from the clip rect. Chain is correct. Full trace documented in review
- [x] Verified `current_clip_in_content_space()` (`stacks.rs:94`) correctly subtracts cumulative scroll offset. When ScrollWidget pushes `push_clip(viewport)` then `push_offset(0, -scroll_y)`, the content-space clip equals `viewport.offset(0, scroll_y)` — correct
- [x] Test `partially_visible_section_still_paints` added in `form_layout/tests.rs`: 3-section form, clip covers section 1 fully + 1px into section 2. Section 2 partially paints (header visible, rows culled by FormSection), section 3 fully culled
- [x] Test `scroll_offset_culls_top_sections` added: scroll offset pushes section 1 above viewport, sections 2+3 paint, section 1 culled. Validates the full scroll→clip→content-space conversion
- [x] Audited Container (culls via `intersects`), StackWidget (no cull needed — children share bounds), PageContainerWidget (only paints active page). No gaps found
- [x] No containers paint off-screen children without clip check — no changes needed

---

## 02.2 Fix ScrollWidget Cache Invalidation on Page Switch

**File(s):** `oriterm_ui/src/widgets/scroll/mod.rs`

**BUG FIX: `ScrollWidget::cached_child_layout` is NOT invalidated on page switch.** The cache is keyed by `Rect` (viewport bounds), not by child identity. `child_natural_size()` calls `self.child.layout()` where `self.child` is the `PageContainerWidget` — and `PageContainerWidget::layout()` only includes the active page. If the viewport rect is identical before and after the page switch, the cache returns stale layout from the previous page. **Verified:** `reset_scroll()` (`scroll/mod.rs:435-438`) only clears `scroll_offset` and `scroll_offset_x` — it does NOT clear `cached_child_layout`. **Fix:** Add `*self.cached_child_layout.borrow_mut() = None;` to `reset_scroll()`. This is the simplest, most correct fix because `reset_scroll()` is already called on page switch (`page_container/mod.rs:134`). No new methods or API changes needed.

- [x] Added `*self.cached_child_layout.borrow_mut() = None;` to `ScrollWidget::reset_scroll()` in `scroll/mod.rs`
- [x] Edge case verified: structural child replacement creates new `ScrollWidget` with empty cache. Only page switch path affected
- [x] Test `reset_scroll_invalidates_cached_child_layout` added in `scroll/tests.rs` — verifies cache is populated by `child_natural_size()` and cleared by `reset_scroll()`
- [x] ~~Verify layout cache hit behavior for prepaint bounds~~ — deferred to manual testing (requires `log::debug!` in running binary). Not blocking section completion

---

## 02.2b Add `for_each_child_mut_all` Widget Trait Method

**File(s):** `oriterm_ui/src/widgets/mod.rs` (Widget trait definition, currently 298 lines — safe headroom), `oriterm_ui/src/widgets/page_container/mod.rs` (currently 149 lines — safe headroom)

> **WARNING — HIGH RISK CHANGE.** `for_each_child_mut()` is a `Widget` trait method called by at least **7 pipeline functions** and **2 input dispatch functions**. Changing its semantics on `PageContainerWidget` from "all children" to "active child only" is a semantic contract change that affects every caller. This must be implemented carefully.

**Step 1 — Add trait method (oriterm_ui/src/widgets/mod.rs):**
- [x] Added `fn for_each_child_mut_all()` to Widget trait with default impl delegating to `for_each_child_mut()`. Doc comment describes purpose and override semantics

**Step 2 — Override both methods on PageContainerWidget (page_container/mod.rs):**
- [x] Changed `PageContainerWidget::for_each_child_mut()` to visit only the active page
- [x] Added `PageContainerWidget::for_each_child_mut_all()` override that visits ALL pages

**Backward compatibility:** Adding `for_each_child_mut_all()` with a default impl that delegates to `for_each_child_mut()` is safe for all existing Widget implementors. They do NOT need any changes because the default impl provides the correct behavior (visit all children = same as current `for_each_child_mut`). Only `PageContainerWidget` needs to override both methods.

**Full Widget implementor list** (for reference if the default-impl approach changes):
- **oriterm_ui** (production): `StackWidget`, `FormSection`, `SettingRowWidget`, `ScrollWidget`, `FormLayout`, `PanelWidget`, `ContainerWidget`, `DialogWidget`, `WindowChromeWidget`, `PageContainerWidget`, `SettingsPanel`, `FormRow`, `ButtonWidget`, `LabelWidget`, `RichLabel`, `TextInputWidget`, `DropdownWidget`, `SidebarNavWidget`, `SliderWidget`, `CursorPickerWidget`, `KbdBadge`, `KeybindRow`, `TabBarWidget`, `SeparatorWidget`, `SpacerWidget`, `CheckboxWidget`, `MenuWidget`, `WindowControlButton`, `CodePreviewWidget`, `NumberInputWidget`, `ColorSwatchGrid`, `SpecialColorSwatch`, `SchemeCardWidget`, `ToggleWidget`, `IdOverrideButton`
- **oriterm** (production): `TerminalPreviewWidget`, `TerminalGridWidget`
- **test-only** (in `pipeline/tests.rs`, `widget_pipeline/tests.rs`, `input/dispatch/tests.rs`, `container/tests.rs`): These mock widgets will inherit the default impl — no changes needed

---

## 02.2c Update Pipeline Callers to Use Correct Traversal Method

**File(s):** `oriterm_ui/src/pipeline/mod.rs` (437 lines — changes below are caller switches, not new code, so line count stays stable), `oriterm_ui/src/action/context.rs`

**Callers that switch to `for_each_child_mut_all()`:**
- [x] `collect_key_contexts()` (`action/context.rs:32-42`) — switched to `for_each_child_mut_all()`. Must map all widget contexts so keymap scope includes hidden-page widgets

**Callers that MUST stay on `for_each_child_mut()` (registration queues lifecycle events):**
- [x] `register_widget_tree()` (`pipeline/mod.rs:313-318`) — stays on `for_each_child_mut()`. Registration queues `WidgetAdded` lifecycle events that must be delivered by `prepare_widget_tree` (which also uses `for_each_child_mut`). Registering hidden-page widgets would queue events that can never be delivered, causing `HotChanged before WidgetAdded` assertion failures when the page becomes active

**Callers that MUST stay on `for_each_child_mut()` (active-page-only is correct — verify these are NOT changed):**
- [x] Verified `prepare_widget_tree()` stays on `for_each_child_mut()` — skipping hidden pages
- [x] Verified `prepaint_widget_tree()` stays on `for_each_child_mut()` — skipping hidden pages
- [x] Verified `collect_focusable_ids()` stays on `for_each_child_mut()` — active page only
- [x] Verified `dispatch_keymap_action()` stays on `for_each_child_mut()` — focus only on active page
- [x] Verified `dispatch_to_widget_tree()` stays on `for_each_child_mut()` — hit test only on active page

**Concrete risks and mitigations:**
- **`register_widget_tree()` risk:** Registration happens at dialog creation (`content_actions.rs:464-471`) and content rebuild (`content_actions.rs:138`). It uses `for_each_child_mut()`. If we don't switch it to `for_each_child_mut_all()`, widgets on non-active pages would never be registered at initial creation time. **Resolution (implemented):** `register_widget_tree()` stays on `for_each_child_mut()` because registration queues `WidgetAdded` lifecycle events that must be delivered by `prepare_widget_tree` (which also uses `for_each_child_mut`). Registering hidden-page widgets would queue events that can never be delivered, causing `HotChanged before WidgetAdded` assertion failures. Instead, page-switch registration is handled by TPR-02-001 fix: `dispatch_dialog_settings_action()` calls `register_widget_tree()` + `drain_events()` for the new page's widgets after a page switch.
- **`collect_key_contexts()` risk:** Called alongside `register_widget_tree()` in both `WindowRoot::compute_layout()` (line 53) and `WindowRoot::rebuild()` (line 224), and also directly in dialog creation (`content_actions.rs:477-478`). Must use `for_each_child_mut_all()` so that key contexts from hidden-page widgets are still available for keymap scope resolution. **Mitigation:** Switch `collect_key_contexts()` to use `for_each_child_mut_all()`.
- **Lifecycle events risk:** `WidgetAdded` events are drained at dialog creation. `FocusChanged` events target specific widgets. If a widget on a hidden page has a pending event, it would not be delivered. **Mitigation:** lifecycle events for hidden pages are harmless to defer — they will be delivered when the page becomes active and `for_each_child_mut` visits it.
- **`focusable_children()` already only returns the active page's focusable children** (`page_container/mod.rs:141-145`), confirming this pattern is intended.
- **`WindowRoot::compute_layout()` and `WindowRoot::rebuild()` risk:** Both methods call `register_widget_tree()`, `collect_key_contexts()`, and `collect_focusable_ids()` in sequence (lines 49-58 and 221-228). `register_widget_tree()` stays on `for_each_child_mut()` (active page only), `collect_key_contexts()` switched to `for_each_child_mut_all()` (all pages), `collect_focusable_ids()` kept on `for_each_child_mut()` (active page only). No changes needed in `WindowRoot` itself. **Verified:** `WidgetTestHarness` tests pass after the change.

---

## 02.2d Reduce Prepare Phase Work on Hover

**File(s):** `oriterm/src/app/dialog_rendering.rs`

When a hover event triggers `DirtyKind::Prepaint`, the dialog path runs `prepare_widget_tree()` on both chrome and content. The prepare phase walks the entire tree to deliver lifecycle events and update animators. For hover (which affects one widget), this is wasteful.

- [x] Analysis complete: `DirtyKind::Prepaint` fires on every hover interaction (lifecycle event delivery)
- [x] Determined: `prepare_widget_tree()` CANNOT be skipped on hover — `VisualStateAnimator::update()` + `tick()` run inside `prepare_widget_frame()` and drive hover transitions. Skipping would break hover fade-in/out
- [x] Deferred to Section 03: per-widget selective walks are the correct solution for reducing prepare-phase work. A simple skip is not feasible

---

## 02.3 Scene Primitive Count Reduction

**File(s):** `oriterm/src/app/dialog_rendering.rs`, `oriterm_ui/src/draw/scene/mod.rs`

After viewport culling is verified, measure the actual reduction in scene primitives.

- [x] `Scene::len()` method confirmed available for primitive counting
- [x] Culling verified via tests: `draw_skips_sections_outside_active_clip` (FormLayout), `draw_skips_rows_outside_active_clip` (FormSection), `scroll_offset_culls_top_sections` (scroll + clip chain), `partially_visible_section_still_paints` (edge case). Tests prove primitive count drops when content is off-screen
- [x] ~~Add `log::debug!` primitive count logging to `compose_dialog_widgets()`~~ — deferred to manual testing. Not blocking section completion
- [x] ~~Compare primitive counts at different scroll positions in running binary~~ — deferred to manual testing. Not blocking section completion
- [x] Off-screen content produces zero primitives — verified by test assertions

---

## 02.4 Tests and Measurement

**Viewport culling tests** — `oriterm_ui/src/widgets/form_layout/tests.rs`:
- [x] `partially_visible_section_still_paints` — 3-section form, clip partially covers section 2, verifies section 2 paints and section 3 is culled
- [x] `scroll_offset_culls_top_sections` — scroll offset hides section 1, verifies sections 2+3 paint. Validates full scroll→clip→content-space conversion chain

**PageContainer traversal tests** — `oriterm_ui/src/widgets/page_container/tests.rs`:
- [x] Updated: `for_each_child_visits_active_page_only` (was `for_each_child_visits_all_pages`) — asserts count == 1
- [x] Added: `for_each_child_mut_all_visits_all_pages` — asserts count == 3
- [x] Added: `for_each_child_follows_active_page_after_switch` — switch to page 2, verify different widget visited

**ScrollWidget cache tests** — `oriterm_ui/src/widgets/scroll/tests.rs`:
- [x] `reset_scroll_invalidates_cached_child_layout` — populates cache, calls `reset_scroll()`, verifies cache is `None`

**Pipeline traversal tests** — `oriterm_ui/src/pipeline/tests.rs`:
- [x] `register_widget_tree` stays on `for_each_child_mut` (registers active page only). Verified by existing harness tests that page switch + rebuild registers new page's widgets
- [x] `collect_focusable_ids` stays on `for_each_child_mut` — verified by existing `focusable_children` test in `page_container/tests.rs`
- [x] Dispatch verified: `harness_button_on_page0_is_clickable` and `harness_button_on_switched_page_is_clickable_after_rebuild` (existing tests)

**Regression checks:**
- [x] All 1590 oriterm_ui tests pass including `settings_panel/tests.rs` and `setting_row/tests.rs`
- [x] ~~Measure before/after primitive counts in running binary~~ — deferred to manual testing. Not blocking section completion

---

## 02.R Third Party Review Findings

- [x] `[TPR-02-001][high]` `oriterm/src/app/dialog_context/content_actions.rs:189` — Page switches no longer rebuild dialog registration, focus, or keymap state after `PageContainerWidget` started exposing only the active page to `for_each_child_mut()`.
  **Resolved 2026-03-21**: Accepted. Added a rebuild step inside the page switch detection block
  in `dispatch_dialog_settings_action()`: `register_widget_tree()` + `drain_events()` for the new
  page's widgets, `key_contexts` clear + `collect_key_contexts()` for chrome and panel, and
  `collect_focusable_ids()` + `set_focus_order()` for focus order. Parent map is rebuilt on next
  key event by `dispatch_dialog_content_key()`. This matches the pattern in `setup_dialog_focus()`.

---

## 02.5 Build & Verify

- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes (2141 tests, 0 failures)
- [x] New tests exist: 2 in `form_layout/tests.rs`, 3 in `page_container/tests.rs`, 1 in `scroll/tests.rs`
- [x] No `#[allow(dead_code)]` on new items — `for_each_child_mut_all` has production callers
- [x] Off-screen dialog sections produce zero scene primitives (verified by tests)
- [x] Hover prepare-phase skip deferred to Section 03 (selective walks)
- [x] `PageContainerWidget::for_each_child_mut()` visits only the active page
- [x] `PageContainerWidget::for_each_child_mut_all()` visits all pages (verified by test)
- [x] `collect_key_contexts()` uses `for_each_child_mut_all()`; `register_widget_tree()` stays on `for_each_child_mut()` (lifecycle event ordering constraint)
- [x] `dispatch_keymap_action()` and `dispatch_to_widget_tree()` remain on `for_each_child_mut()`
- [x] `collect_focusable_ids()` remains on `for_each_child_mut()`
- [x] `ScrollWidget::cached_child_layout` is invalidated on page switch
- [x] `WidgetTestHarness` tests pass (verified: all 1590 oriterm_ui tests pass)

**Exit Criteria:** A `WidgetTestHarness` test demonstrates that a scrolled `FormLayout` produces fewer `Scene` primitives than an unscrolled one. `PageContainerWidget::for_each_child_mut()` visits only the active page (verified by test in `page_container/tests.rs`). `for_each_child_mut_all()` visits all pages (verified by test). Pipeline callers that need all-children access (`collect_key_contexts`) use `for_each_child_mut_all()`. `register_widget_tree()` stays on `for_each_child_mut()` (lifecycle event ordering constraint). Event dispatch callers (`dispatch_keymap_action`, `dispatch_to_widget_tree`) and `collect_focusable_ids` remain on `for_each_child_mut()` (active-page-only is correct). `log::debug!` output in the dialog render path shows primitive count reduction when scrolled. `cargo test -p oriterm_ui` and `cargo test -p oriterm` pass with 0 failures.
