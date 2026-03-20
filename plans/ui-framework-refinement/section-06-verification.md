---
section: "06"
title: "Verification & Polish"
status: not-started
reviewed: true
goal: "Comprehensive integration testing, visual regression verification, performance validation, and documentation for all changes introduced in Sections 01-09."
depends_on: ["01", "02", "03", "04", "05", "07", "08", "09"]
sections:
  - id: "06.1"
    title: "Test Matrix"
    status: not-started
  - id: "06.2"
    title: "Performance Validation"
    status: not-started
  - id: "06.3"
    title: "Context Capability Audit"
    status: not-started
  - id: "06.4"
    title: "Documentation"
    status: not-started
  - id: "06.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Verification & Polish

**Status:** Not Started
**Goal:** Prove all changes from Sections 01-09 work together as a cohesive system. Verify no performance regressions. Audit context types for capability restrictions. Update documentation.

**Depends on:** All prior sections (01-05, 07-09).

**Scope note:** Most items in 06.1 and 06.3 are verification of existing code. However, several items in 06.2 require new implementation:
1. Scene pipeline alloc regression test (new integration test binary, ~100-150 lines).
2. Frame time Criterion benchmark (new bench, ~80-100 lines).
3. `maybe_shrink()` methods on `Scene` and `DamageTracker` + wiring (~30 lines).
4. `DamageTracker::merge_overlapping()` fix to replace `vec![false; N]` with a reusable buffer (~15 lines).
5. `log::debug!` counters for damage tracking and layout caching benefit (~20 lines total).
The overview estimates ~600 lines for Section 06; these implementation items account for ~300-400 of those.

---

## 06.1 Test Matrix

Verify that tests from implementing sections (01-05, 07-09) exist and pass. Where tests are missing, add them following the sibling `tests.rs` pattern per `test-organization.md`. The architectural boundary tests live in `oriterm/tests/architecture.rs` (integration test) per Section 09.

- [ ] **Test Harness (Section 01):**
  - Input simulation covers all event types (mouse, keyboard, scroll, drag)
  - State inspection covers all interaction states (hot, active, focused, disabled)
  - Time control advances animations deterministically
  - Paint capture produces expected Scene primitives (quads, text_runs, etc.)
  - Widget queries find by sense and position; `find_by_name()` is a stub (returns `None` -- full name lookup requires immutable tree traversal not yet available); ID-based access via `get_widget(id)` and `interaction_state(id)` works
  - RenderScheduler integration: anim frame and deferred repaint scheduling works
  - `apply_requests` handles all flags: SET_ACTIVE, CLEAR_ACTIVE, REQUEST_FOCUS, FOCUS_NEXT, FOCUS_PREV
  - Shared pipeline functions accessible from both oriterm and oriterm_ui

- [ ] **Safety Rails (Section 02):**
  - Double-visit of a child during dispatch triggers debug panic
  - Cross-phase mismatch (dispatch child not in layout set) triggers debug panic
  - Lifecycle event before WidgetAdded triggers debug panic
  - All existing container widgets pass all assertions without modification
  - Asymmetric child sets produce no false positives (e.g., `PageContainerWidget` yields all pages to `for_each_child_mut` but only lays out the active page -- dispatch superset of layout is valid)

- [ ] **Scene Architecture (Section 03):**
  - Scene typed arrays (quads, text_runs, lines, icons, images) populated correctly by all widget `paint()` methods
  - ContentMask resolved at push time from clip/offset stacks -- no stack commands in output
  - DamageTracker identifies changed regions via per-widget primitive hash diff
  - DamageTracker `reset()` triggers full-scene dirty on next `compute_damage()` (resize/theme/font/scale change path)
  - `build_scene()` produces correct Scene with balanced stacks (debug assertion)
  - `convert_scene()` produces correct GPU instances with per-instance clip fields
  - Per-instance clip: terminal-tier instances carry `ContentMask::unclipped()` (NEG_INFINITY bounds), chrome/overlay tiers carry real clip rects
  - Instance buffer size is 96 bytes (up from 80) with clip fields at offsets [80-95]
  - **(Manual)** Shader-side clipping works on all three backends (Vulkan, DX12, Metal). Not CI-automatable; verify manually on each platform or via visual regression screenshots
  - InvalidationTracker uses `dirty_map: HashMap<WidgetId, DirtyKind>` + `full_invalidation: bool` (single map with `DirtyKind` severity levels, no separate `paint_dirty`/`layout_dirty` fields). Verify `invalidate_all()` sets `full_invalidation` which causes `max_dirty_kind()` to return `Layout`
  - No DrawList, DrawCommand, SceneCache, SceneNode, ClipContext, ClipSegment, or TierClips references remain in the codebase

- [ ] **Prepaint Phase (Section 04):**
  - Layout-only changes (structural) run all three phases
  - Prepaint-level changes (hover) skip layout, run prepaint + paint
  - Paint-only changes (cursor blink) skip layout + prepaint, run paint only
  - Phase invocation counts match expected values in test assertions
  - End-to-end 3-pass pipeline: layout -> prepaint -> paint flow through WindowRoot produces correct Scene. Verify the phases compose correctly when orchestrated by `WindowRoot.dispatch_event()` + `WindowRoot.compute_layout()` + `build_scene()`
  - `collect_layout_bounds()` produces correct `HashMap<WidgetId, Rect>` for container with nested children (bounds match `LayoutNode` tree)
  - `DirtyKind` ordering: `Clean < Paint < Prepaint < Layout` (Ord derive correctness)
  - `InvalidationTracker.max_dirty_kind()` returns highest severity across all tracked widgets

- [ ] **Action/Keymap (Section 05):**
  - Actions dispatch through keymap to correct widget via `dispatch_keymap_action()` tree walk
  - Context scoping gates bindings correctly (Dialog context, Settings context)
  - `build_context_stack()` produces correct stack from focus path + pre-built `HashMap<WidgetId, &'static str>` context map
  - `dispatch_keymap_action()` tree walk finds the correct widget by ID in a nested container hierarchy and delivers the action
  - Runtime rebinding works (change binding -> new key activates action)
  - Default bindings cover all existing shortcuts (except TextEditController)
  - Keymap lookup runs first; unmatched keys fall through to remaining controllers (TextEditController, FocusController click-to-focus)
  - TextEditController still handles text editing keys after keymap integration (character input, cursor movement, selection -- NOT migrated to keymap per Section 05 design decision)
  - KeyActivationController, DropdownKeyController, MenuKeyController, and SliderKeyController all removed; button/toggle/checkbox/dropdown/menu/slider behavior unchanged via keymap
  - Per-widget `handle_keymap_action()` returns correct `WidgetAction` for migrated widgets: DropdownWidget (NavigateDown/Up/Confirm/Dismiss), MenuWidget (same + Space), SliderWidget (IncrementValue/DecrementValue/ValueToMin/ValueToMax), ButtonWidget/ToggleWidget/CheckboxWidget (Activate)
  - `FocusNext`/`FocusPrev` keymap actions correctly set `ControllerRequests::FOCUS_NEXT`/`FOCUS_PREV` flags (framework-level, not widget-level)

- [ ] **WindowRoot (Section 07):**
  - WindowRoot constructable in a `#[test]` without GPU or platform dependencies
  - WidgetTestHarness wraps WindowRoot (not raw fields)
  - WindowContext and DialogWindowContext wrap WindowRoot + platform/GPU state
  - Event routing through WindowRoot: overlays take priority over widget tree
  - WindowRoot pipeline methods work individually: `compute_layout()` produces valid LayoutNode tree, `dispatch_event()` updates interaction state and collects actions, `prepare()` delivers lifecycle events and ticks animations, `rebuild()` re-registers widgets and rebuilds focus order
  - FocusManager integration: Tab key press through WindowRoot advances focus to next focusable widget
  - RenderScheduler available in production via WindowRoot (previously test-only); verify `scheduler.next_wake_time()` returns correct values for active animations
  - Overlay test helpers work: `push_popup()`, `has_overlays()`, `dismiss_overlays()` on WidgetTestHarness
  - Compositor integration: `LayerTree` and `LayerAnimator` owned by WindowRoot participate in overlay rendering and animation
  - No duplicate framework field declarations across WindowRoot, WindowContext, DialogWindowContext (verify via code audit)
  - All existing harness tests pass after WindowRoot unification

- [ ] **Pure Logic Migration (Section 08):**
  - CursorBlink tests pass in oriterm_ui (moved from oriterm)
  - ResizeEdge and hit testing pass in oriterm_ui
  - Mark mode motion functions pass in oriterm_ui
  - No pure UI logic without oriterm_mux dependencies remains in oriterm/src/app/

- [ ] **Boundary Enforcement (Section 09):**
  - Architectural tests pass: WindowRoot headless, event propagation, overlay routing, interaction state propagation
  - Crate dependency direction tests pass (oriterm_ui has no oriterm/oriterm_mux/oriterm_ipc deps; oriterm_core has no oriterm_ui/oriterm_mux/oriterm_ipc deps; oriterm_mux has no oriterm_ui/oriterm deps; oriterm_ipc has no oriterm_* deps)
  - oriterm_ui has no GPU/font deps (no wgpu, tiny-skia, swash, skrifa, rustybuzz in Cargo.toml dependencies)
  - CLAUDE.md and crate-boundaries.md are up to date
  - `.claude/rules/crate-boundaries.md` exists with ownership rules for all 5 workspace crates

---

## 06.2 Performance Validation

- [ ] **Idle CPU:** Verify zero idle CPU beyond cursor blink (existing invariant preserved). Run `compute_control_flow()` pure function tests in `oriterm/src/app/event_loop_helpers/tests.rs` and confirm no new spurious wakeup sources were introduced by WindowRoot's `RenderScheduler`
- [ ] **Allocation regression (oriterm_core):** Existing `oriterm_core/tests/alloc_regression.rs` still passes -- `renderable_content_into()` performs zero heap allocations after warmup. Run with `timeout 150 cargo test -p oriterm_core --test alloc_regression`
- [ ] **(Implementation)** **Allocation regression (Scene pipeline):** Create a new integration test binary (`oriterm_ui/tests/scene_alloc_regression.rs` or `oriterm/tests/scene_alloc_regression.rs`) with `#[global_allocator]` counting allocator (same pattern as `oriterm_core/tests/alloc_regression.rs`; cannot reuse it because `#[global_allocator]` is per-binary). Verify `build_scene()` and `convert_scene()` perform zero heap allocations after warmup (same grid size, no images, no combining marks). Scene's typed arrays (`Vec<Quad>`, `Vec<TextRun>`, etc.) must reuse capacity via `clear()`. DamageTracker's `HashMap` fields must swap via `std::mem::swap` (no per-frame allocation). **Prerequisite:** Fix `DamageTracker::merge_overlapping()` which currently allocates `vec![false; N]` per call -- replace with a reusable `Vec<bool>` field on the struct (~15 lines)
- [ ] **(Implementation)** **Frame time benchmark:** Create a Criterion benchmark (`oriterm/benches/scene_convert.rs` or `oriterm_ui/benches/scene_convert.rs`) that benchmarks `build_scene() + convert_scene()` using a synthetic Scene with ~2000 quads + ~2000 text_runs (representative 80x24 terminal). Establish baseline, then verify no perceptible regression. The project already has Criterion benchmarks in `oriterm_core/benches/` (grid, vte_throughput) but nothing for the Scene/GPU conversion path
- [ ] **Instance buffer size:** Verify 96-byte instances (up from 80) do not cause measurable GPU upload regression. For a typical 80x24 terminal (~3840 instances), the delta is ~61 KB -- expected to be negligible relative to ~300 KB baseline and atlas textures. Validate via manual frame timing comparison
- [ ] **(Implementation)** **Damage tracking benefit:** Add a `log::debug!` counter in the render dispatch path that logs when `DamageTracker::has_damage()` returns false (frame skipped) vs true (frame drawn). Run a manual session: type some text, wait idle, resize, scroll. Verify from logs that clean frames are skipped. This is observational validation, not an automated test
- [ ] **(Implementation)** **Layout caching benefit:** Add a `log::debug!` in the app layer's phase-gating code (where `max_dirty_kind()` decides which phases to run). Run a manual session: hover buttons (Prepaint-only), wait for cursor blink (Paint-only), resize (Layout). Verify from logs that the correct phases are skipped
- [ ] **(Implementation)** **Buffer shrink discipline:** Add `maybe_shrink()` methods to `Scene` (typed arrays) and `DamageTracker` (`dirty_regions`, `merge_scratch`), then wire them into the post-render shrink path in `render_dispatch.rs`. Apply the standard threshold: `if capacity > 4 * len && capacity > 4096 -> shrink_to(len * 2)`. Currently `maybe_shrink()` exists on `InstanceWriter`, `PreparedFrame`, `ShapingScratch`, and `RenderableContent`, but not on Scene or DamageTracker buffers. Estimated ~30 lines of new code

---

## 06.3 Context Capability Audit

Verify each context type exposes only phase-appropriate methods. These restrictions are enforced structurally by which fields exist on each context struct, not by compile-time traits. Verification is a manual code review of `oriterm_ui/src/widgets/contexts.rs` checking that no context struct has fields it should not (e.g., `LayoutCtx` should not gain a `scene` field). This is a one-time review; future drift is prevented by code review discipline.

- [ ] `LayoutCtx` -- has `measurer: &dyn TextMeasurer` and `theme: &UiTheme`. CANNOT request paint (no `FrameRequestFlags`), CANNOT access interaction state (no `InteractionManager`), CANNOT emit draw commands (no `scene`)
- [ ] `DrawCtx` -- can emit draw commands to Scene (`scene: &mut Scene`), read theme, measure text, request anim frame/paint via `FrameRequestFlags`. Has `icons: Option<&ResolvedIcons>` for GPU icon rendering, `bounds: Rect`, `now: Instant`, `widget_id: Option<WidgetId>`. CANNOT modify widget state. Still exposes `is_hot()`, `is_active()`, `is_interaction_focused()` via `interaction: Option<&InteractionManager>` during migration period (only ButtonWidget migrated to prepaint in Section 04; remaining widgets still read interaction state in `paint()`)
- [ ] `PrepaintCtx` -- can read interaction state (`is_hot`, `is_hot_direct`, `is_active`, `is_interaction_focused`) via `interaction: Option<&InteractionManager>`. Can read theme, read current time (`now: Instant`) for animation interpolation. Can request anim frame and repaint via `frame_requests: Option<&FrameRequestFlags>`. Receives `widget_id: WidgetId` and `bounds: Rect` from layout. CANNOT emit draw commands (no `scene` field), CANNOT modify layout
- [ ] `EventCtx` -- can read interaction state (`is_hot`, `is_active`, `is_interaction_focused`) via `interaction: Option<&InteractionManager>`. Can measure text (`measurer: &dyn TextMeasurer`), read theme, read `bounds: Rect` and `widget_id: Option<WidgetId>`. Can request anim frame and repaint via `frame_requests: Option<&FrameRequestFlags>`. Can build child contexts via `for_child()`. CANNOT emit draw commands (no `scene`)
- [ ] `LifecycleCtx` -- has `widget_id: WidgetId`, per-widget `interaction: &InteractionState` (read-only), and `requests: ControllerRequests` for side effects (PAINT, SET_ACTIVE, CLEAR_ACTIVE, REQUEST_FOCUS). CANNOT emit draw commands, CANNOT access the full `InteractionManager`
- [ ] `AnimCtx` -- has `widget_id: WidgetId`, `now: Instant`, `requests: ControllerRequests` for side effects (SET_ACTIVE, CLEAR_ACTIVE, REQUEST_FOCUS). Can request animation frame and repaint via `frame_requests: Option<&FrameRequestFlags>`. Has no `InteractionManager` reference (cannot read interaction state directly). CANNOT emit draw commands

---

## 06.4 Documentation

- [ ] Update CLAUDE.md with new test infrastructure (how to run harness tests, how to write new harness tests)
- [ ] Update CLAUDE.md with action/keymap pattern (how to declare actions, how to add keybindings)
- [ ] Verify module-level `//!` doc comments exist and are up-to-date on: `testing/mod.rs`, `action/keymap/mod.rs`, `draw/scene/mod.rs`, `draw/damage/mod.rs`, `pipeline/mod.rs`, `window_root/mod.rs`
- [ ] Verify module-level `//!` doc comments on modules added by Sections 07-08: `window_root/pipeline.rs`, `interaction/cursor_hide/mod.rs`, `interaction/resize/mod.rs`, `interaction/mark_mode/mod.rs`, `animation/cursor_blink/mod.rs`
- [ ] Verify CLAUDE.md "Crate Boundaries" section added by Section 09.3 is consistent with actual crate dependency state after all sections complete
- [ ] Verify CLAUDE.md "UI Framework -- Zero Exceptions Rule" mentions WindowRoot per Section 09.3

---

## 06.5 Completion Checklist

**06.1 -- Test Matrix:**
- [ ] Test matrix covers all features (every checkbox in 06.1 verified)
- [ ] Test Harness (01): input simulation, state inspection, time control, paint capture, widget queries, RenderScheduler, apply_requests, shared pipeline
- [ ] Safety Rails (02): double-visit panic, cross-phase panic, lifecycle ordering panic, all containers pass, asymmetric child sets no false positives
- [ ] Scene Architecture (03): typed arrays correct, ContentMask resolved, DamageTracker diff, DamageTracker reset, build_scene balanced, convert_scene GPU instances, per-instance clip tiers, instance size 96 bytes, shader clipping 3 backends (manual), InvalidationTracker uses dirty_map + full_invalidation, no dead types remain
- [ ] Prepaint Phase (04): 3-phase gating correct, phase counts correct, end-to-end pipeline through WindowRoot, collect_layout_bounds correct, DirtyKind ordering, max_dirty_kind
- [ ] Action/Keymap (05): keymap dispatch via dispatch_keymap_action, context scoping, build_context_stack, rebinding, default bindings, keymap-first fallthrough, TextEditController unaffected, 4 controllers removed, per-widget handle_keymap_action, FocusNext/FocusPrev flags
- [ ] WindowRoot (07): headless construction, harness wraps WindowRoot, WindowContext/DialogWindowContext wrap WindowRoot, overlay priority, individual pipeline methods, FocusManager Tab nav, RenderScheduler in production, overlay helpers, compositor integration, no duplicate fields, all existing harness tests pass
- [ ] Pure Logic Migration (08): CursorBlink in oriterm_ui, ResizeEdge in oriterm_ui, mark mode in oriterm_ui, no pure UI logic stranded in oriterm
- [ ] Boundary Enforcement (09): architectural tests pass (headless, propagation, overlay routing, interaction state), all 5 crate dependency directions correct, oriterm_ui no GPU/font deps, CLAUDE.md + crate-boundaries.md up to date

**06.2 -- Performance:**
- [ ] Performance validated (frame time benchmark passes or manual timing shows no perceptible regression)
- [ ] Idle CPU invariant preserved (event_loop_helpers tests pass, no spurious wakeups from RenderScheduler)
- [ ] `oriterm_core/tests/alloc_regression.rs` passes (zero alloc after warmup for renderable_content_into)
- [ ] Scene pipeline zero-alloc after warmup (build_scene + compute_damage + convert_scene) -- requires new test binary
- [ ] Instance buffer 96-byte size does not cause measurable GPU upload regression
- [ ] Buffer shrink discipline applied to Scene and DamageTracker buffers -- requires new `maybe_shrink()` methods

**06.3 -- Context Audit:**
- [ ] Context capability audit complete (no phase-inappropriate fields on any context struct)
- [ ] All 6 context types verified against `oriterm_ui/src/widgets/contexts.rs`: LayoutCtx, DrawCtx, PrepaintCtx, EventCtx, LifecycleCtx, AnimCtx

**06.4 -- Documentation:**
- [ ] Documentation updated (CLAUDE.md + module docs)
- [ ] Module docs verified on all new modules from Sections 01-09
- [ ] CLAUDE.md crate boundaries section consistent with final state

**Build Gates:**
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green (includes cross-compile target `x86_64-pc-windows-gnu`)
- [ ] All source files (excluding tests.rs) under 500-line limit
- [ ] Zero dead code warnings (`dead_code = "deny"` in project lints)

**Exit Criteria:** All 8 prior sections (01-05, 07-09) validated by integration tests. Frame time benchmark shows no regression (or manual timing confirms no perceptible slowdown). Zero allocation regressions in hot paths (`alloc_regression.rs` passes; Scene pipeline alloc test passes). Context types enforce phase-appropriate restrictions via struct field presence (verified by code review of `contexts.rs`). `./test-all.sh && ./clippy-all.sh && ./build-all.sh` all pass cleanly. No dead code, no oversized files, no stale documentation.
