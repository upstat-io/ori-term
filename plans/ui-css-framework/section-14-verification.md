---
section: "14"
title: "Verification + Visual Regression"
status: not-started
reviewed: false
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "The CSS UI framework plan closes with reproducible evidence, not a hand-written checklist: every prior section owns direct automated regressions, the settings dialog has deterministic GPU golden coverage, build/platform/performance gates reflect the real repository scripts, and the remaining human sign-off is limited to live behavior the automated layers cannot prove."
depends_on: ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13"]
sections:
  - id: "14.1"
    title: "Verification Ownership + Artifacts"
    status: not-started
  - id: "14.2"
    title: "Automated Test Matrix"
    status: not-started
  - id: "14.3"
    title: "Deterministic Visual Regression"
    status: not-started
  - id: "14.4"
    title: "Manual Sign-Off"
    status: not-started
  - id: "14.5"
    title: "Build + Platform Gates"
    status: not-started
  - id: "14.6"
    title: "Performance + Invariants"
    status: not-started
  - id: "14.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "14.7"
    title: "Completion Checklist"
    status: not-started
---

# Section 14: Verification + Visual Regression

## Problem

The current draft is not accurate to the repository or to the revised Sections 01-13.

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
- The repository already has deterministic GPU visual regression infrastructure in
  `oriterm/src/gpu/visual_regression/`, plus a UI-only renderer path via
  `WindowRenderer::new_ui_only`, `prepare_ui_frame(...)`, and
  `append_ui_scene_with_text(...)`. The draft ignores all of that and falls back to a manual
  screenshot checklist.
- The build scripts are not what the draft says:
  - `./build-all.sh` cross-builds Windows GNU debug + release
  - `./clippy-all.sh` runs Windows GNU and host clippy with warnings denied
  - `./test-all.sh` runs `cargo test --workspace --features oriterm/gpu-tests`
- The performance section is too anecdotal. The tree already has automated invariants for idle
  event-loop behavior and scene/damage allocation reuse, but the draft does not connect the
  settings dialog closeout to them.

Section 14 therefore needs to become a layered verification plan with real ownership and real
artifacts, not a static checklist full of stale assumptions.

## Corrected Scope

Section 14 should keep the full final-verification goal and close it with four layers:

1. section-owned automated regressions near the modules changed in Sections 01-13
2. cross-section settings-dialog integration tests for builder, actions, layout, overlays, and
   semantic state
3. deterministic GPU golden tests for the rendered settings dialog itself
4. focused manual sign-off for live interaction and native-platform behavior that automated tests
   cannot prove

This section should not pretend that one manual screenshot pass is enough, and it should not repeat
stale per-feature assertions that no longer match the revised plan.

---

## 14.1 Verification Ownership + Artifacts

### Goal

Define the actual verification layers and the files that own them so Section 14 closes with durable
artifacts instead of one-off human checks.

### Files

- section-local `tests.rs` files under `oriterm_ui/src/widgets/`, `oriterm_ui/src/text/`,
  `oriterm/src/gpu/scene_convert/`, `oriterm/src/font/`, and related modules changed by
  Sections 01-13
- `oriterm/src/app/settings_overlay/form_builder/tests.rs`
- `oriterm/src/app/settings_overlay/action_handler/tests.rs`
- `oriterm_ui/src/widgets/settings_panel/tests.rs`
- `oriterm_ui/src/testing/render_assert.rs`
- `oriterm/src/gpu/visual_regression/mod.rs`
- new `oriterm/src/gpu/visual_regression/settings_dialog.rs`
- `oriterm/tests/references/*.png`
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

- [ ] Keep direct regressions near the modules changed by Sections 01-13
- [ ] Add cross-section settings-dialog integration coverage
- [ ] Add settings-dialog GPU goldens as committed artifacts
- [ ] Allow narrowly-scoped test-support extraction where needed
- [ ] Keep manual verification as the last layer, not the only layer

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

### Checklist

- [ ] Every section from 01-13 has at least one direct automated regression in its owning module
- [ ] Rendering-sensitive sections also prove behavior one layer deeper when needed
- [ ] Settings-dialog integration tests cover builder, actions, overlays, and dirty-state flows
- [ ] New tests follow the repository's sibling `tests.rs` pattern unless they are crate-level
      integration or golden tests

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

The missing piece is a settings-dialog fixture path that:

1. builds the real dialog from `build_settings_dialog(...)`
2. composes the real panel/dialog scene with a real UI measurer path, not `MockMeasurer`
3. converts that scene through the UI-only renderer path
4. compares the resulting pixels against committed references

### Required Fixture Set

At minimum, add multiple dialog fixtures so the goldens cover the actual feature surface instead of
one cherry-picked page:

- `settings_appearance_clean_96dpi`
  - sidebar, page header, slider, toggle, dropdown trigger, footer clean state
- `settings_overlay_dropdown_96dpi`
  - open dropdown popup path so trigger styling and menu styling are both covered
- `settings_colors_or_terminal_96dpi`
  - scheme cards or cursor picker so selection-card controls are covered
- `settings_window_or_font_dirty_96dpi`
  - number/text inputs plus dirty footer state
- one `192dpi` or equivalent HiDPI fixture
  - to catch rounding and scaling regressions in the dialog path

The exact page split can change, but the fixture set must cover sidebar, content typography, footer,
popup overlays, and the expanded Section 13 control families.

### Required Rendering Path

Prefer using the existing dialog rendering path and UI-only renderer primitives rather than
inventing a parallel mock renderer:

- `WindowRenderer::new_ui_only(...)`
- `prepare_ui_frame(...)`
- `append_ui_scene_with_text(...)`
- `append_overlay_scene_with_text(...)` when a popup is open
- `compare_with_reference(...)`

If needed, extract a narrow reusable helper from dialog rendering so tests can compose the real
settings scene without creating a full OS window.

### Golden Workflow

- normal verification:
  - `timeout 150 cargo test -p oriterm --features gpu-tests visual_regression`
- updating references intentionally:
  - `ORITERM_UPDATE_GOLDEN=1 timeout 150 cargo test -p oriterm --features gpu-tests visual_regression`
- if a golden changes:
  - review the PNG diff, not just the test result
  - commit the updated reference PNGs
  - do not leave `_actual.png` or `_diff.png` artifacts in the tree

### Test Timeout Discipline

All test commands in this section and its verification must respect the repository's mandatory 150-second timeout rule. GPU golden tests that hang indicate a rendering pipeline bug, not a slow test. If a golden test exceeds the timeout, diagnose and fix the blocking code path rather than extending the timeout.

### Checklist

- [ ] Add settings-dialog visual regression tests under `gpu/visual_regression/`
- [ ] Render through the real UI-only renderer path, not a fake scene-only shortcut
- [ ] Cover clean, dirty, popup-open, and card/control-heavy dialog states
- [ ] Include at least one HiDPI dialog fixture
- [ ] Commit reviewed reference PNGs and keep artifact cleanup disciplined

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

**Dialog shell behavior**

- centered placement, min-size behavior, and resize correctness for dialog windows
- footer/button semantics under real dirty-state changes
- overlay dismissal via click-outside and keyboard paths

**Native platform sign-off**

- native Windows run
- native Linux run
- native macOS run

Cross-compiling Windows from WSL is not a substitute for native visual sign-off, and no amount of
local Linux testing proves macOS dialog behavior.

### Checklist

- [ ] Manual comparison completed against the mockup at 96 DPI
- [ ] Live interaction states verified on the real dialog
- [ ] Native Windows, Linux, and macOS runs reviewed
- [ ] Any platform-specific mismatches are fixed before plan closeout

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
  - Windows GNU target and host target
  - warnings denied
- `./test-all.sh`
  - `cargo test --workspace --features oriterm/gpu-tests`
  - this is the primary automated gate for the settings-dialog golden tests as well

### Build-Gate Implications

- A separate host `cargo build` is usually redundant for closeout because host compilation is
  already exercised by clippy and tests.
- macOS is not proven by the local scripts; that still requires native CI and/or native manual
  verification before final sign-off.
- If Section 14 adds new golden PNGs or test modules, those must be included in the normal
  `./test-all.sh` path rather than hidden behind an ad hoc manual command.

### Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] Settings-dialog visual regression tests run as part of the normal test gate
- [ ] macOS verification is covered separately because local scripts do not prove it

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

- add at least one targeted regression for repeated settings-dialog composition and/or UI-scene
  conversion after warmup
- prove that opening a dropdown overlay and re-rendering the dialog does not create unbounded
  allocation churn or persistent animation scheduling
- keep the dialog idle path compatible with the existing "wait when idle" event-loop contract

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

- [ ] Existing allocation and idle-loop invariants remain green
- [ ] Add a settings-dialog-specific warm-path regression where current coverage is insufficient
- [ ] Manual profiling confirms the dialog returns to idle after interaction
- [ ] No new sustained redraw or allocation churn is introduced by the UI framework work

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

---

## 14.7 Completion Checklist

Section 14 is complete only when all of the following are true:

- [ ] Sections `01` through `13` are complete and their reviewed expectations are reflected in code
- [ ] Section-local regressions exist and pass for each prior section
- [ ] Settings-dialog integration tests pass
- [ ] Settings-dialog GPU golden tests pass at standard DPI and HiDPI
- [ ] Manual sign-off is complete on native Windows, Linux, and macOS
- [ ] `./build-all.sh`, `./clippy-all.sh`, and `./test-all.sh` all pass
- [ ] Existing invariant tests stay green and any new settings-specific performance checks pass
- [ ] Golden-reference PNGs are reviewed, committed, and no `_actual` / `_diff` artifacts remain
- [ ] `plans/ui-css-framework/index.md` is updated to reflect completion
- [ ] `plans/ui-css-framework/00-overview.md` and the section statuses remain consistent with the
      finished state
