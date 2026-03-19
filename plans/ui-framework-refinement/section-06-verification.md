---
section: "06"
title: "Verification & Polish"
status: not-started
reviewed: false
goal: "Comprehensive integration testing, visual regression verification, performance validation, and documentation for all changes introduced in Sections 01-05."
depends_on: ["01", "02", "03", "04", "05"]
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
**Goal:** Prove all changes from Sections 01-05 work together as a cohesive system. Verify no performance regressions. Audit context types for capability restrictions. Update documentation.

**Depends on:** All prior sections.

---

## 06.1 Test Matrix

Build a comprehensive test matrix covering every feature through the new infrastructure.

- [ ] **Test Harness (Section 01):**
  - Input simulation covers all event types (mouse, keyboard, scroll, drag)
  - State inspection covers all interaction states (hot, active, focused, disabled)
  - Time control advances animations deterministically
  - Paint capture produces expected Scene primitives (quads, text_runs, etc.)
  - Widget queries find by ID, name, sense, position
  - RenderScheduler integration: anim frame and deferred repaint scheduling works
  - apply_requests handles all flags: SET_ACTIVE, CLEAR_ACTIVE, REQUEST_FOCUS, FOCUS_NEXT, FOCUS_PREV
  - Shared pipeline functions accessible from both oriterm and oriterm_ui

- [ ] **Safety Rails (Section 02):**
  - Double-visit of a child during dispatch triggers debug panic
  - Cross-phase mismatch (dispatch child not in layout set) triggers debug panic
  - Lifecycle event before WidgetAdded triggers debug panic
  - All existing container widgets pass all assertions without modification
  - Stashed/hidden widget handling produces no false positives

- [ ] **Scene Architecture (Section 03):**
  - Scene typed arrays (quads, text_runs, lines, icons, images) populated correctly by all widget paint() methods
  - ContentMask resolved at push time from clip/offset stacks — no stack commands in output
  - DamageTracker identifies changed regions via per-widget primitive hash diff
  - build_scene() produces correct Scene with balanced stacks (debug assertion)
  - convert_scene() produces correct GPU instances with per-instance clip fields
  - Shader-side clipping works on all three backends (Vulkan, DX12, Metal)
  - No DrawList, DrawCommand, SceneCache, or ClipSegment references remain

- [ ] **Prepaint Phase (Section 04):**
  - Layout-only changes (structural) run all three phases
  - Prepaint-level changes (hover) skip layout, run prepaint + paint
  - Paint-only changes (cursor blink) skip layout + prepaint, run paint only
  - Phase invocation counts match expected values in test assertions

- [ ] **Action/Keymap (Section 05):**
  - Actions dispatch through keymap to correct widget
  - Context scoping gates bindings correctly (Dialog context, Settings context)
  - Runtime rebinding works (change binding -> new key activates action)
  - Default bindings cover all existing shortcuts (except TextEditController)
  - Keymap lookup -> controller fallback coexistence works correctly
  - TextEditController still handles text editing keys after keymap integration
  - KeyActivationController removed, button/toggle/checkbox behavior unchanged

---

## 06.2 Performance Validation

- [ ] **Idle CPU:** Verify zero idle CPU beyond cursor blink (existing invariant preserved)
- [ ] **Frame time:** Measure frame time with Scene vs old DrawList -- target: no regression >5%
- [ ] **Damage tracking benefit:** Measure frames skipped due to clean regions
- [ ] **Layout caching benefit:** Measure layout passes skipped due to prepaint-only or paint-only dirty

---

## 06.3 Context Capability Audit

Verify each context type exposes only phase-appropriate methods (masonry pattern).

- [ ] `LayoutCtx` -- can measure text, read theme. CANNOT request paint, set active, access interaction state
- [ ] `DrawCtx` -- can emit draw commands, read interaction state (during migration). CANNOT modify widget state
- [ ] `PrepaintCtx` -- can read interaction state, resolve visual state properties, cache results on widget. CANNOT emit draw commands, CANNOT modify layout (hitboxes come from LayoutNode, not prepaint)
- [ ] `EventCtx` -- can read interaction state. CANNOT emit draw commands
- [ ] `LifecycleCtx` -- can request paint (via `ControllerRequests`). CANNOT emit draw commands
- [ ] `AnimCtx` -- can request animation frame. CANNOT modify interaction state

---

## 06.4 Documentation

- [ ] Update CLAUDE.md with new test infrastructure (how to run harness tests, how to write new harness tests)
- [ ] Update CLAUDE.md with action/keymap pattern (how to declare actions, how to add keybindings)
- [ ] Add module-level doc comments to `testing/mod.rs`, `action/keymap.rs`, `draw/scene/mod.rs`, `draw/damage/mod.rs`, `pipeline.rs`

---

## 06.5 Completion Checklist

- [ ] Test matrix covers all features (every checkbox in 06.1 verified)
- [ ] Performance validated (no >5% frame time regression)
- [ ] Context capability audit complete (no phase-inappropriate methods exposed)
- [ ] Documentation updated (CLAUDE.md + module docs)
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** All 5 prior sections validated by integration tests. Frame time within 5% of baseline. Context types enforce phase-appropriate restrictions. `./test-all.sh && ./clippy-all.sh && ./build-all.sh` all pass cleanly.
