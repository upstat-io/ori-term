---
section: "06"
title: "Opacity + Display Control"
status: not-started
reviewed: true
third_party_review:
  status: resolved
  updated: 2026-03-23
goal: "Reusable framework primitives provide subtree opacity, layout-preserving hidden state, display:none-style removal, and pointer hit-test suppression without bespoke per-widget hacks; GPU scene conversion multiplies subtree opacity with compositor opacity"
inspired_by:
  - "CSS opacity"
  - "CSS visibility: hidden"
  - "CSS display: none"
  - "CSS pointer-events: none"
depends_on: []
sections:
  - id: "06.1"
    title: "Scene Opacity Stack"
    status: not-started
  - id: "06.2"
    title: "Visibility + Display Modifiers"
    status: not-started
  - id: "06.3"
    title: "Pointer Events Suppression"
    status: not-started
  - id: "06.4"
    title: "Consumer Boundaries"
    status: not-started
  - id: "06.5"
    title: "Tests"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "06.6"
    title: "Build & Verify"
    status: not-started
---

# Section 06: Opacity + Display Control

## Problem

The original draft correctly identified subtree opacity as missing, but it placed display control at
the wrong layer and misdescribed some current behavior.

What the tree actually has today:

- [oriterm_ui/src/draw/scene/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/scene/mod.rs)
  has clip, offset, and layer-background stacks only. There is no opacity stack.
- [oriterm_ui/src/draw/scene/content_mask.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/scene/content_mask.rs)
  carries only a clip rect.
- [oriterm/src/gpu/scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs)
  already accepts a compositor-level `opacity: f32`, but only applies it while converting quads,
  lines, text, and icons.
- [oriterm_ui/src/widgets/page_container/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/page_container/mod.rs)
  already gives the settings dialog active-page-only layout, paint, focus, and traversal. Section
  06 should generalize that behavior, not pretend page switching has no implementation today.
- [oriterm_ui/src/window_root/pipeline.rs](/home/eric/projects/ori_term/oriterm_ui/src/window_root/pipeline.rs)
  rebuilds registration, focus order, and key contexts from `for_each_child_mut()`, so a generic
  display feature must integrate with widget-tree traversal, not only layout solver output.
- widget hit testing lives in
  [oriterm_ui/src/input/hit_test.rs](/home/eric/projects/ori_term/oriterm_ui/src/input/hit_test.rs),
  not [oriterm_ui/src/hit_test/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/hit_test/mod.rs)
  which is window-chrome hit testing.

The real missing capabilities are:

1. widgets cannot fade an entire subtree without manually baking alpha into every color
2. there is no reusable framework-level equivalent of `visibility: hidden` or `display: none`
3. there is no subtree-level pointer hit-test suppression separate from semantic `disabled`

## Corrected Scope

Section 06 should add three reusable framework primitives:

- a scene opacity stack, exposed ergonomically through `DrawCtx`
- widget modifiers for `Visible`, `Hidden`, and `DisplayNone` subtree behavior
- a layout-node `pointer_events` gate used only by pointer hit testing

This keeps the implementation aligned with the current architecture:

- opacity belongs at the scene/content-mask boundary
- `display:none` semantics belong at the widget-tree traversal boundary, because registration,
  focus order, key contexts, prepaint, and app integration all depend on active traversal
- pointer-events suppression belongs in the input hit-test path, not in `disabled`

This section should not try to remove existing consumer-specific code that already works. It should
replace bespoke patterns only where the new primitives actually improve them.

---

## 06.1 Scene Opacity Stack

### Goal

Let any widget paint a subtree at reduced opacity without manually rewriting all descendant colors.

### Files

- [oriterm_ui/src/draw/scene/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/scene/mod.rs)
- [oriterm_ui/src/draw/scene/stacks.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/scene/stacks.rs)
- [oriterm_ui/src/draw/scene/paint.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/scene/paint.rs)
- [oriterm_ui/src/draw/scene/content_mask.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/scene/content_mask.rs)
- [oriterm_ui/src/widgets/contexts.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/contexts.rs)
- [oriterm/src/gpu/scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs)

### Scene Changes

Add an opacity stack and resolved cumulative value to `Scene`:

```rust
opacity_stack: Vec<f32>,
cumulative_opacity: f32,
```

API surface:

```rust
pub fn push_opacity(&mut self, opacity: f32) { ... }
pub fn pop_opacity(&mut self) { ... }
pub fn current_opacity(&self) -> f32 { ... }
pub fn opacity_stack_is_empty(&self) -> bool { ... }
```

Normalization rules at the scene boundary:

- finite values are clamped to `0.0..=1.0`
- `NaN` and infinities normalize to `1.0` so bad inputs do not poison the scene state
- stacked opacity composes multiplicatively

Initialize `cumulative_opacity` to `1.0` in `Scene::new()` and restore it in `Scene::clear()`.
Extend the `build_scene()` debug assertion so unbalanced opacity pushes fail the same way unbalanced
clip/offset/layer-background pushes already do.

### ContentMask Change

Extend `ContentMask` to carry opacity as resolved paint-time state:

```rust
pub struct ContentMask {
    pub clip: Rect,
    pub opacity: f32,
}
```

`ContentMask::unclipped()` becomes `{ clip: infinite_rect, opacity: 1.0 }`, and
`current_content_mask()` in `paint.rs` resolves both clip and opacity.

### GPU Conversion

`convert_scene()` already has compositor opacity. Subtree opacity is a second multiplier:

```rust
let effective_opacity = opacity * primitive.content_mask.opacity;
```

Apply that in:

- `convert_quad()`
- `convert_scene_line()`
- `convert_scene_text()`
- `convert_scene_icon()`

[oriterm/src/gpu/scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs)
currently ignores `ImagePrimitive` entirely. This section should still thread opacity through
`ContentMask` now so the contract is ready, but the observable GPU behavior in this section remains
quad/line/text/icon until image rendering is implemented.

### DrawCtx Helper

Add scoped opacity helpers. There are two viable patterns given `DrawCtx`'s borrow structure:

**Option A — Direct scene access (simplest, matches existing codebase patterns):**

Widgets call `ctx.scene.push_opacity(opacity)` / `ctx.scene.pop_opacity()` directly, the same
way they already call `ctx.scene.push_clip()` and `ctx.scene.push_offset()`. No new `DrawCtx`
method is strictly needed; the ergonomic surface is already established by the existing clip/offset
stack API.

**Option B — Scope guard struct:**

```rust
pub struct OpacityGuard<'a> {
    scene: &'a mut Scene,
}

impl Drop for OpacityGuard<'_> {
    fn drop(&mut self) {
        self.scene.pop_opacity();
    }
}

impl DrawCtx<'_> {
    pub fn push_opacity(&mut self, opacity: f32) -> OpacityGuard<'_> {
        self.scene.push_opacity(opacity);
        OpacityGuard { scene: self.scene }
    }
}
```

This avoids the borrow-checker issue with a closure-based API (the callback `f: impl FnOnce(&mut
DrawCtx)` would need to reborrow `self` while `scene` is already mutably borrowed by the
push/pop pair). The scope guard ensures balanced push/pop via RAII.

Either option is acceptable. The non-negotiable part is that `Scene` must expose `push_opacity()`
and `pop_opacity()` and the `build_scene()` assertion must catch unbalanced stacks.

### Checklist

- [ ] Add opacity stack state to `Scene`
- [ ] Normalize/clamp opacity at the scene boundary
- [ ] Extend `ContentMask` with `opacity`
- [ ] Multiply subtree opacity with compositor opacity in scene conversion
- [ ] Expose opacity through either direct `Scene::push_opacity()`/`pop_opacity()` or a `DrawCtx` scope guard
- [ ] Extend `build_scene()` stack-balance assertion to include `opacity_stack_is_empty()`

---

## 06.2 Visibility + Display Modifiers

### Goal

Provide reusable subtree semantics for both `visibility: hidden` and `display: none`, instead of
burying that behavior inside `PageContainerWidget`.

### Files

- [oriterm_ui/src/widgets/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/mod.rs)
- new module:
  [oriterm_ui/src/widgets/modifiers/](/home/eric/projects/ori_term/oriterm_ui/src/widgets/modifiers)
- [oriterm_ui/src/layout/layout_box.rs](/home/eric/projects/ori_term/oriterm_ui/src/layout/layout_box.rs)
- [oriterm_ui/src/window_root/pipeline.rs](/home/eric/projects/ori_term/oriterm_ui/src/window_root/pipeline.rs)
  for behavioral verification, not for the primary implementation

### New Visibility Enum

Add a reusable wrapper-level visibility enum:

```rust
pub enum VisibilityMode {
    Visible,
    Hidden,
    DisplayNone,
}
```

Semantics:

- `Visible`: normal layout, paint, hit test, focus, traversal
- `Hidden`: participates in layout but does not paint, does not register descendants for
  interaction, and does not contribute focusable descendants
- `DisplayNone`: contributes zero layout size and is skipped for paint, interaction, focus, and
  active traversal

### Wrapper Widget

Add a wrapper widget in `widgets/modifiers/visibility.rs`, for example:

```rust
pub struct VisibilityWidget {
    id: WidgetId,
    child: Box<dyn Widget>,
    mode: VisibilityMode,
}
```

This wrapper is the correct abstraction boundary because it controls the widget-tree traversal that
the current framework already uses for:

- interaction registration / GC
- focus order collection
- key-context collection
- prepare / prepaint walks
- action propagation

### Layout Behavior

`VisibilityWidget::layout()` should behave differently by mode:

- `Visible`: delegate child layout unchanged
- `Hidden`: delegate child layout, then recursively scrub widget-routing metadata from the returned
  `LayoutBox` tree so it occupies space but contributes no widget IDs or hit-testable descendants
- `DisplayNone`: return a zero-size leaf box

The important correction versus the original draft is that a plain `LayoutBox.visible` flag is not
enough. It would only affect layout unless every traversal path also learned to honor it, and some
of those traversals happen before layout even exists.

### Layout Scrubbing Helper

To make `Hidden` feasible, add a recursive helper on `LayoutBox`, e.g.:

```rust
pub fn for_layout_only(mut self) -> Self { ... }
```

This helper should recursively:

- clear `widget_id`
- set `sense` to `Sense::none()`
- reset hit-test-specific metadata
- preserve the actual sizing, padding, flex/grid structure, and clipping needed for layout

That allows the hidden subtree to keep its geometry without leaking stale widget IDs into the
computed layout tree.

### Traversal Behavior

`VisibilityWidget` should implement:

- `for_each_child_mut()`:
  visit child only in `Visible`
- `for_each_child_mut_all()`:
  always visit child so full-tree maintenance tools still have access when needed
- `focusable_children()`:
  empty for `Hidden` and `DisplayNone`
- `accept_action()`:
  no-op for `Hidden` and `DisplayNone`
- `paint()`:
  no-op for `Hidden` and `DisplayNone`

### Why This Improves Page Switching

[PageContainerWidget](/home/eric/projects/ori_term/oriterm_ui/src/widgets/page_container/mod.rs)
already behaves like `DisplayNone` for inactive pages. Section 06 should generalize that behavior
so other containers can use it too.

This section does not need to delete `PageContainerWidget`. It should either:

- keep it as the specialized page-switching container and treat it as the reference behavior, or
- refactor it internally to reuse `VisibilityMode::DisplayNone` once the generic wrapper exists

Both are acceptable as long as the generic primitive is added and existing page-switch behavior
stays correct.

### Checklist

- [ ] Add `VisibilityMode`
- [ ] Add a reusable visibility/display wrapper widget in `oriterm_ui/src/widgets/modifiers/visibility.rs`
- [ ] Create `oriterm_ui/src/widgets/modifiers/mod.rs` with `#[cfg(test)] mod tests;` and `oriterm_ui/src/widgets/modifiers/tests.rs`
- [ ] Add recursive `LayoutBox` scrubbing for layout-only hidden content
- [ ] Ensure active traversal skips hidden/display-none descendants
- [ ] Keep `for_each_child_mut_all()` available for full-tree maintenance

---

## 06.3 Pointer Events Suppression

### Goal

Add subtree-level pointer hit-test suppression without conflating it with semantic disabled state.

### Files

- [oriterm_ui/src/layout/layout_box.rs](/home/eric/projects/ori_term/oriterm_ui/src/layout/layout_box.rs)
- [oriterm_ui/src/layout/layout_node.rs](/home/eric/projects/ori_term/oriterm_ui/src/layout/layout_node.rs)
- [oriterm_ui/src/layout/solver.rs](/home/eric/projects/ori_term/oriterm_ui/src/layout/solver.rs)
- [oriterm_ui/src/layout/grid_solver.rs](/home/eric/projects/ori_term/oriterm_ui/src/layout/grid_solver.rs)
- [oriterm_ui/src/input/hit_test.rs](/home/eric/projects/ori_term/oriterm_ui/src/input/hit_test.rs)
- new wrapper:
  [oriterm_ui/src/widgets/modifiers/](/home/eric/projects/ori_term/oriterm_ui/src/widgets/modifiers)

### Correction to the Draft

`disabled` is not the right implementation of CSS-like `pointer-events: none`.

Current `disabled` is stronger:

- it is stored in `LayoutBox` / `LayoutNode`
- it suppresses hit testing
- widgets often also use it for disabled visuals and focusability decisions

`pointer-events: none` should only block pointer hit testing for a subtree. It should not
implicitly change layout, paint, or keyboard policy.

### New Field

Add a subtree gate to `LayoutBox` and `LayoutNode`:

```rust
pub pointer_events: bool,
```

Default: `true`.

Add `with_pointer_events(bool)` to `LayoutBox`, and propagate the field through both flex and grid
solvers into `LayoutNode`.

### Hit-Test Behavior

In [input/hit_test.rs](/home/eric/projects/ori_term/oriterm_ui/src/input/hit_test.rs), early-out
before child traversal when `pointer_events == false`:

```rust
if !node.pointer_events {
    return None;
}
```

This blocks hover, click, and drag hit testing for the entire subtree while leaving layout and
paint intact.

### Wrapper Surface

Add a small wrapper widget, or a field on the new modifiers wrapper, that delegates child layout
and simply flips the root layout box to `pointer_events = false`.

That gives parent-controlled pointer-event suppression without forcing every container widget to
grow custom per-child logic.

### Keyboard Policy

Keep keyboard policy separate:

- `pointer_events = false` only blocks pointer hit testing
- widgets that must be fully disabled still use semantic disabled state and focusability controls

That separation matches CSS better and avoids baking "disabled" assumptions into a purely pointer
feature.

### Checklist

- [ ] Add `pointer_events: bool` to `LayoutBox` and `LayoutNode`
- [ ] Add `with_pointer_events(bool)`
- [ ] Propagate the field through flex and grid solvers
- [ ] Early-out in widget hit testing when pointer events are disabled
- [ ] Expose a reusable wrapper or modifier API for parent-controlled use

---

## 06.4 Consumer Boundaries

### Goal

Adopt the new primitives where they solve real framework gaps, without rewriting working consumers
just for symmetry.

### Current Consumers to Respect

- [oriterm_ui/src/widgets/page_container/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/page_container/mod.rs)
  already gives inactive pages zero layout and traversal cost
- [oriterm_ui/src/widgets/sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs)
  already uses explicit alpha for inactive icons
- many disabled widgets already use dedicated disabled colors rather than subtree opacity alone

### Recommended Adoption

1. Use `ctx.scene.push_opacity(0.4)` / `ctx.scene.pop_opacity()` (or the scope-guard equivalent
   from Section 06.1) at the row or subtree boundary for mockup cases that are truly subtree
   opacity, such as disabled settings rows.
2. Keep existing per-color alpha where the visual is intentionally only partial, such as inactive
   sidebar icons at `0.7`.
3. Generalize page-switch behavior with `VisibilityMode::DisplayNone` semantics, but do not claim
   that this section is the first implementation of page hiding.

### Important App-Level Caveat

The app-side settings dialog still does explicit registration GC, focus-order rebuilds, parent-map
updates, and cached-layout invalidation around page switches in
[content_actions.rs](/home/eric/projects/ori_term/oriterm/src/app/dialog_context/content_actions.rs).

Section 06 should not quietly delete that logic unless dialog integration is also refactored. The
framework primitive lives in `oriterm_ui`; app code still owns its current page-switch rebuild
sequence.

### Checklist

- [ ] Apply subtree opacity at the correct boundary for disabled-row visuals
- [ ] Keep existing icon alpha behavior unless a consumer truly wants subtree fade
- [ ] Treat `PageContainerWidget` as existing behavior to generalize, not as a fake blocker
- [ ] Preserve app-level page-switch rebuild logic unless the app integration is explicitly updated

---

## 06.5 Tests

### Scene / Conversion

In [oriterm_ui/src/draw/scene/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/draw/scene/tests.rs):

- `fn opacity_stack_push_pop_balance()` — push and pop produce balanced state; unbalanced push triggers debug assertion
- `fn opacity_stack_multiplicative_composition()` — pushing 0.5 then 0.5 produces cumulative 0.25
- `fn opacity_stack_clamps_to_unit_range()` — values outside 0.0..=1.0 are clamped
- `fn opacity_stack_normalizes_nan_and_infinity()` — NaN and infinity normalize to 1.0 (no poison)
- `fn content_mask_captures_opacity_on_quad()` — quad primitive stores `ContentMask.opacity`
- `fn content_mask_captures_opacity_on_text()` — text primitive stores `ContentMask.opacity`
- `fn content_mask_opacity_default_is_one()` — `ContentMask::unclipped()` has `opacity: 1.0`

In [oriterm/src/gpu/scene_convert/tests.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/tests.rs):

- `fn subtree_opacity_multiplies_with_compositor_opacity_quad()` — quad with 0.5 subtree opacity and 0.8 compositor opacity produces 0.4 effective
- `fn subtree_opacity_multiplies_with_compositor_opacity_text()` — text opacity composition

### Visibility / Display Wrappers

Add focused tests in `oriterm_ui/src/widgets/modifiers/tests.rs`:

- `fn visible_mode_delegates_layout_paint_traversal()` — `Visible` delegates layout, paint, and traversal
- `fn hidden_mode_preserves_layout_size()` — `Hidden` preserves layout size
- `fn hidden_mode_emits_no_scene_primitives()` — `Hidden` emits no scene primitives
- `fn display_none_produces_zero_layout_size()` — `DisplayNone` produces zero layout size
- `fn for_each_child_mut_skips_hidden()` — `for_each_child_mut()` skips hidden/display-none descendants
- `fn for_each_child_mut_all_visits_hidden()` — `for_each_child_mut_all()` still visits hidden descendants
- `fn focusable_children_empty_for_hidden()` — `focusable_children()` returns empty for `Hidden` and `DisplayNone`
- `fn accept_action_noop_for_hidden()` — `accept_action()` is no-op for `Hidden` and `DisplayNone`

### Pointer Events

In [oriterm_ui/src/input/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/input/tests.rs):

- `fn pointer_events_false_blocks_hit_test()` — subtree with `pointer_events = false` is not hit-testable
- `fn pointer_events_false_preserves_layout()` — layout geometry is unchanged when pointer events are disabled
- `fn pointer_events_false_allows_paint()` — child subtree is still paintable when pointer events are disabled
- `fn pointer_events_true_by_default()` — default `LayoutBox` has `pointer_events = true`

### Pipeline / WindowRoot

Add regression coverage in
[oriterm_ui/src/window_root/tests.rs](/home/eric/projects/ori_term/oriterm_ui/src/window_root/tests.rs)
or the new modifiers tests:

- `fn visibility_toggle_gcs_stale_registrations()` — GCs stale interaction registrations after toggling to hidden
- `fn visibility_toggle_drops_focus_targets()` — drops removed focus targets from focus order
- `fn visibility_toggle_preserves_full_tree_access()` — keeps `for_each_child_mut_all()` access for full-tree maintenance

### Checklist

- [ ] Scene tests cover opacity stack and `ContentMask.opacity`
- [ ] Scene-convert tests cover compositor-opacity multiplication
- [ ] Modifiers tests cover `Visible`, `Hidden`, and `DisplayNone`
- [ ] Input tests cover `pointer_events = false`
- [ ] Pipeline or window-root tests cover stale-registration and focus-order cleanup

---

## 06.R Third Party Review Findings

### Resolved Findings

1. `TPR-06-001`:
   The draft claimed page switching still needed a display-none feature for zero layout cost, but
   [PageContainerWidget](/home/eric/projects/ori_term/oriterm_ui/src/widgets/page_container/mod.rs)
   already provides active-page-only layout, paint, and traversal.

2. `TPR-06-002`:
   A `LayoutBox.visible` field by itself is insufficient because registration, focus collection,
   key-context collection, prepare, and prepaint all walk the widget tree via `for_each_child_mut`
   outside the layout solver.

3. `TPR-06-003`:
   Mapping `disabled` directly to CSS `pointer-events: none` is incorrect. `disabled` is a stronger
   semantic state and is already used beyond pointer hit testing.

4. `TPR-06-004`:
   The draft cited the wrong hit-test module. Widget hit testing is implemented in
   [input/hit_test.rs](/home/eric/projects/ori_term/oriterm_ui/src/input/hit_test.rs), while
   [hit_test/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/hit_test/mod.rs) handles
   frameless window chrome.

5. `TPR-06-005`:
   Existing widget code already uses manual alpha in some places
   ([sidebar_nav/mod.rs](/home/eric/projects/ori_term/oriterm_ui/src/widgets/sidebar_nav/mod.rs),
   tab-bar close-button fades, scrollbar thumb alpha). Section 06 must add reusable subtree opacity,
   not misrepresent the tree as having no opacity usage at all.

6. `TPR-06-006`:
   [scene_convert/mod.rs](/home/eric/projects/ori_term/oriterm/src/gpu/scene_convert/mod.rs)
   still ignores `ImagePrimitive`, so Section 06 can only promise immediate GPU opacity effects for
   quads, lines, text, and icons unless image rendering lands at the same time.

---

## 06.6 Build & Verify

### Gate

```bash
./build-all.sh
./clippy-all.sh
./test-all.sh
```

### Verification Steps

1. `cargo test -p oriterm_ui draw::scene` and the relevant modifiers/input/window-root tests pass.
2. `cargo test -p oriterm gpu::scene_convert` passes with subtree-opacity assertions.
3. Visual: disabled settings rows dim as a subtree at `0.4` opacity.
4. Visual: inactive sidebar icons remain `0.7` alpha without unintended full-row fade.
5. Visual: hidden-but-layout-preserving content reserves space but paints nothing.
6. Visual: display-none content contributes zero size and does not respond to hover/click.

### Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] `build_scene()` asserts balanced opacity pushes
- [ ] visibility toggles do not leak stale interaction state
- [ ] pointer-events suppression affects hit testing only
