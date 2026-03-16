---
section: "02"
title: "Sense & Hit Testing"
status: complete
goal: "Widgets declare what interactions they care about; hit testing respects Sense + interact_radius"
inspired_by:
  - "egui Sense enum (egui/src/sense.rs)"
  - "Flutter HitTestBehavior (rendering/proxy_box.dart)"
depends_on: ["01"]
reviewed: true
sections:
  - id: "02.1"
    title: "Sense Enum & Widget Trait Integration"
    status: complete
  - id: "02.2"
    title: "Hit Testing Overhaul"
    status: complete
    subsections:
      - id: "02.2a"
        title: "HitTestBehavior Enum"
        status: complete
      - id: "02.2b"
        title: "LayoutNode & LayoutBox Extensions"
        status: complete
      - id: "02.2c"
        title: "Hit Test Function Changes"
        status: complete
  - id: "02.3"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Sense & Hit Testing

**Status:** Not Started
**Goal:** Every widget declares a `Sense` (what interactions it cares about). Hit testing
filters by Sense, skipping widgets that don't care about the current event type. Labels
(`Sense::none()`) are never hit-tested. An `interact_radius` extends hit areas for small
widgets.

**Context:** Currently hit testing (`layout_hit_test()` in `oriterm_ui/src/input/hit_test.rs`)
walks the entire layout tree for every mouse event, testing every widget. Labels, separators,
and spacers participate in hit testing even though they never handle events. This wastes cycles
and produces confusing behavior when a separator "blocks" a click intended for the widget
behind it. (Note: a separate `oriterm_ui/src/hit_test/` module handles window chrome
hit testing; this section only modifies the widget-level one in `input/hit_test.rs`.)

**Reference implementations:**
- **egui** `egui/src/sense.rs`: `Sense::click()`, `Sense::drag()`, `Sense::click_and_drag()`,
  `Sense::hover()`, `Sense::focusable()` — widgets declare upfront what they care about
- **Flutter** `HitTestBehavior`: `deferToChild`, `opaque`, `translucent` — controls event
  pass-through behavior

**Depends on:** Section 01 (InteractionState types).

---

## 02.1 Sense Enum & Widget Trait Integration

**File(s):** `oriterm_ui/src/sense.rs` (new leaf module),
`oriterm_ui/src/widgets/mod.rs` (Widget trait)

### Sense type definition

- [x] Create `oriterm_ui/src/sense.rs` as a **leaf module** with zero intra-crate imports
  (depends only on `std`). Define `Sense` as a manual bitmask newtype, following the
  `Modifiers` pattern in `input/event.rs`. Do NOT use `bitflags` (not a dependency of
  `oriterm_ui`; if `bitflags` is added in Section 04 for `ControllerRequests`, migrate then).
  ```rust
  /// Declares what interactions a widget cares about.
  ///
  /// A bitflag set: widgets compose flags via `union()`. Hit testing
  /// skips widgets with `Sense::none()`.
  #[derive(Clone, Copy, PartialEq, Eq, Hash)]
  pub struct Sense(u8);

  impl Sense {
      const HOVER_BIT: u8 = 0b0001;
      const CLICK_BIT: u8 = 0b0010;
      const DRAG_BIT: u8  = 0b0100;
      const FOCUS_BIT: u8 = 0b1000;

      /// No interactions — invisible to hit testing.
      pub const fn none() -> Self { Self(0) }
      /// Hover tracking only.
      pub const fn hover() -> Self { Self(Self::HOVER_BIT) }
      /// Click events (implies hover).
      pub const fn click() -> Self { Self(Self::HOVER_BIT | Self::CLICK_BIT) }
      /// Drag events (implies hover).
      pub const fn drag() -> Self { Self(Self::HOVER_BIT | Self::DRAG_BIT) }
      /// Click and drag (implies hover).
      pub const fn click_and_drag() -> Self {
          Self(Self::HOVER_BIT | Self::CLICK_BIT | Self::DRAG_BIT)
      }
      /// Keyboard focus only (no hover/click/drag).
      pub const fn focusable() -> Self { Self(Self::FOCUS_BIT) }
      /// All interactions.
      pub const fn all() -> Self {
          Self(Self::HOVER_BIT | Self::CLICK_BIT | Self::DRAG_BIT | Self::FOCUS_BIT)
      }

      /// Combines two sense sets (bitwise OR).
      #[must_use]
      pub const fn union(self, other: Self) -> Self { Self(self.0 | other.0) }
      /// Whether this sense set is empty.
      pub const fn is_none(self) -> bool { self.0 == 0 }
  }
  ```
- [x] Implement `Default` for `Sense` returning `Sense::none()`. Required by `LayoutNode`
  (02.2b) for nodes without an associated widget.
- [x] Implement `Debug` manually: print active flags like `Sense(HOVER | CLICK)`, following
  the `Modifiers` Debug pattern in `input/event.rs`.
- [x] Add unit tests in `oriterm_ui/src/sense/tests.rs` (rename `sense.rs` to
  `sense/mod.rs` + `sense/tests.rs` per test-organization rules). Test `union()`, `is_none()`,
  `Default`, and that `click()` implies hover but not drag.
- [x] Add `pub mod sense;` to `oriterm_ui/src/lib.rs`.
- [x] Add convenience re-export in `interaction/mod.rs`:
  `pub use crate::sense::Sense;`

### Module placement constraints

Both `Sense` and `HitTestBehavior` (02.2a) must be **leaf modules** at
`oriterm_ui/src/sense.rs` and `oriterm_ui/src/hit_test_behavior.rs` respectively.
They must NOT live inside `interaction/` or `input/` because:
- `layout/layout_node.rs` imports them
- `interaction/parent_map.rs` already imports `layout::LayoutNode`
- `input/hit_test.rs` already imports `layout::LayoutNode`

Placing either type inside `interaction/` or `input/` would create circular imports
(`layout -> interaction -> layout` or `layout -> input -> layout`). Leaf modules with
zero intra-crate imports avoid this entirely.

Import paths:
- `layout/layout_node.rs`: `use crate::sense::Sense;` and
  `use crate::hit_test_behavior::HitTestBehavior;`
- `input/hit_test.rs`: `use crate::sense::Sense;` and
  `use crate::hit_test_behavior::HitTestBehavior;`

### Widget trait integration

- [x] Add `fn sense(&self) -> Sense` as a **default method** on the `Widget` trait
  (in `oriterm_ui/src/widgets/mod.rs`), returning `Sense::all()`. This preserves current
  behavior where every widget participates in hit testing. Section 08 overrides per-widget.

  **Pre-flight check**: Verify no existing widget method is named `sense()` — a name
  collision would shadow the trait default and cause silent bugs.

- [x] Add `fn hit_test_behavior(&self) -> HitTestBehavior` as a **default method** on the
  `Widget` trait, returning `HitTestBehavior::DeferToChild`. Same rationale: backward-
  compatible default, refined in Section 08.

  **Pre-flight check**: Verify no existing widget method is named `hit_test_behavior()`.

  **Impact**: These 2 default methods add to a trait with 25+ implementors. Because they
  have defaults, existing code compiles unchanged.

### Per-widget Sense values (reference for Section 08)

These are the final per-widget values that Section 08 will set when overriding the
`Sense::all()` default. Listed here as the design specification:

  - `LabelWidget`: `Sense::none()`
  - `SeparatorWidget`: `Sense::none()`
  - `SpacerWidget`: `Sense::none()`
  - `ButtonWidget`: `Sense::click()`
  - `ToggleWidget`, `CheckboxWidget`: `Sense::click()`
  - `DropdownWidget`: `Sense::click().union(Sense::focusable())`
  - `SliderWidget`: `Sense::drag().union(Sense::focusable())`
  - `TextInputWidget`: `Sense::click_and_drag().union(Sense::focusable())`
  - `ScrollWidget`: `Sense::drag()` (for scrollbar)
  - `ContainerWidget`: `Sense::none()` (default, overridable)
  - `DialogWidget`: `Sense::none()` (container, delegates)
  - `PanelWidget`: `Sense::none()`
  - `StackWidget`: `Sense::none()`
  - `FormLayout`, `FormSection`, `FormRow`: `Sense::none()`
  - `SettingsPanel`: `Sense::none()`
  - `MenuWidget`: `Sense::click()` (menu items are clickable)
  - `WindowChromeWidget`: `Sense::none()` (children handle clicks)
  - `WindowControlButton`, `IdOverrideButton`: `Sense::click()`
  - `TabBarWidget`: `Sense::click_and_drag()` (click tabs, drag to reorder)
  - `TerminalGridWidget` (in `oriterm` crate): `Sense::click_and_drag().union(Sense::focusable())`
  - `TerminalPreviewWidget` (in `oriterm` crate): `Sense::none()`

### Disabled widget behavior

- [x] A disabled widget (`InteractionState::is_disabled() == true`) is treated as
  `Sense::none()` during hit testing, regardless of its declared Sense. This prevents
  disabled buttons from stealing clicks from widgets behind them.

  **Implementation**: Store a `disabled: bool` field on `LayoutNode` alongside `sense`
  (see 02.2b). Populate it during layout from
  `InteractionManager::get_state(id).is_disabled()`. This keeps hit testing pure — no
  `InteractionManager` reference needed in the hit test function.

---

## 02.2 Hit Testing Overhaul

**File(s):** `oriterm_ui/src/hit_test_behavior.rs` (new leaf module),
`oriterm_ui/src/input/hit_test.rs`, `oriterm_ui/src/layout/layout_node.rs`,
`oriterm_ui/src/layout/layout_box.rs`, `oriterm_ui/src/layout/solver.rs`

Subsections are ordered by dependency: 02.2a defines `HitTestBehavior`, 02.2b adds fields
to `LayoutNode`/`LayoutBox` (which depend on 02.2a), and 02.2c modifies hit test functions
(which depend on 02.2b).

### 02.2a HitTestBehavior Enum

- [x] Create `oriterm_ui/src/hit_test_behavior.rs` as a **leaf module** with zero
  intra-crate imports (see module placement constraints in 02.1). Define:
  ```rust
  /// Controls how a widget participates in hit testing relative to its children.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum HitTestBehavior {
      /// Hit-test children first; self only if no child handles (default).
      DeferToChild,
      /// This widget absorbs the event — children behind it don't receive.
      Opaque,
      /// Both this widget and children behind it can receive the event.
      Translucent,
  }

  impl Default for HitTestBehavior {
      fn default() -> Self {
          Self::DeferToChild
      }
  }
  ```
  No unit tests needed — `HitTestBehavior` has no methods beyond derived traits and
  `Default`. It is exercised through hit test integration tests in 02.2c.
- [x] Add `pub mod hit_test_behavior;` to `oriterm_ui/src/lib.rs`.
- [x] Add convenience re-export in `oriterm_ui/src/input/mod.rs`:
  `pub use crate::hit_test_behavior::HitTestBehavior;`

### 02.2b LayoutNode & LayoutBox Extensions

**Design decision**: Store `sense`, `hit_test_behavior`, `clip`, `disabled`, and
`interact_radius` directly in `LayoutNode`. This keeps hit testing a pure function of the
layout tree (no external lookups needed). Data flows:
Widget -> LayoutBox -> solver -> LayoutNode.

- [x] **Add 5 fields to `LayoutNode`** (`oriterm_ui/src/layout/layout_node.rs`).
  Import from the leaf modules (`crate::sense::Sense` and
  `crate::hit_test_behavior::HitTestBehavior`) — NOT from `interaction/` or `input/`.
  The `layout/` module currently has zero imports from `input/` or `interaction/`, and
  this invariant must be preserved.
  ```rust
  use crate::hit_test_behavior::HitTestBehavior;
  use crate::sense::Sense;

  pub struct LayoutNode {
      pub rect: Rect,
      pub content_rect: Rect,
      pub children: Vec<Self>,
      pub widget_id: Option<WidgetId>,
      /// Sense flags for hit-test filtering. Default: `Sense::none()`.
      pub sense: Sense,
      /// Hit-test behavior. Default: `DeferToChild`.
      pub hit_test_behavior: HitTestBehavior,
      /// When `true`, children are clipped to this node's `rect` during
      /// hit testing and rendering.
      pub clip: bool,
      /// When `true`, widget is disabled and treated as `Sense::none()`.
      pub disabled: bool,
      /// Expands the hit area beyond `rect` for small targets.
      /// `0.0` means no expansion (default).
      pub interact_radius: f32,
  }
  ```
  Update `LayoutNode::new()` defaults: `sense: Sense::default()`,
  `hit_test_behavior: HitTestBehavior::DeferToChild`, `clip: false`,
  `disabled: false`, `interact_radius: 0.0`.
  Add builder methods: `with_sense(Sense)`, `with_hit_test_behavior(HitTestBehavior)`,
  `with_clip(bool)`, `with_disabled(bool)`, `with_interact_radius(f32)`.

  **File size check**: `layout_node.rs` is currently 47 lines. Adding 5 fields, 5 builder
  methods, and updated `new()` brings it to ~105 lines. Well under the 500-line limit.

- [x] **Update `make_node` helper in `input/tests.rs`** (line 15): This is the only struct
  literal construction of `LayoutNode` in the codebase. Add the 5 new fields with defaults.
  The solver (`solver.rs`) uses `LayoutNode::new()` exclusively, so it is unaffected.

- [x] **Add 5 matching fields to `LayoutBox`** (`oriterm_ui/src/layout/layout_box.rs`):
  ```rust
  use crate::hit_test_behavior::HitTestBehavior;
  use crate::sense::Sense;

  pub struct LayoutBox {
      // ... existing fields ...
      /// Sense flags for hit-test filtering.
      pub sense: Sense,
      /// Hit-test behavior.
      pub hit_test_behavior: HitTestBehavior,
      /// Whether children are clipped to this box's bounds.
      pub clip: bool,
      /// Whether the widget is disabled (treated as `Sense::none()`).
      pub disabled: bool,
      /// Hit area expansion for small targets (pixels).
      pub interact_radius: f32,
  }
  ```
  Update `LayoutBox::leaf()` and `LayoutBox::flex()` defaults:
  `sense: Sense::default()`, `hit_test_behavior: HitTestBehavior::DeferToChild`,
  `clip: false`, `disabled: false`, `interact_radius: 0.0`.
  Add builder methods: `with_sense(Sense)`, `with_hit_test_behavior(HitTestBehavior)`,
  `with_clip(bool)`, `with_disabled(bool)`, `with_interact_radius(f32)`.

  **File size check**: `layout_box.rs` is currently 204 lines. Adding 5 fields, 5 builder
  methods, and defaults in 2 constructors brings it to ~260 lines. Under limit.

- [x] **Propagate in solver** (`oriterm_ui/src/layout/solver.rs`): After each
  `node.widget_id = layout_box.widget_id;` line, add propagation for all 5 fields.
  Three sites must be updated:
  - `solve_leaf()` (line 100)
  - `arrange_children()` (line 327)
  - `solve_empty()` (line 353)
  ```rust
  node.sense = layout_box.sense;
  node.hit_test_behavior = layout_box.hit_test_behavior;
  node.clip = layout_box.clip;
  node.disabled = layout_box.disabled;
  node.interact_radius = layout_box.interact_radius;
  ```

  **File size check**: `solver.rs` is currently 428 lines. Adding ~18 lines of propagation
  (6 lines x 3 sites) brings it to ~446. Under limit.

- [x] **Wire `clip` from `ContainerWidget`**: In `ContainerWidget::build_layout_box()`
  (method defined at line 291 of `container/mod.rs`, called by `layout()` at line 374),
  chain `.with_clip(true)` when `self.clip_children` is `true`. This makes clipping a
  first-class layout tree property.

### 02.2c Hit Test Function Changes

**File size projection**: `hit_test.rs` is currently 119 lines. This subsection adds
`WidgetHitTestResult` (~20 lines), `HitEntry` (~10 lines), Sense filtering (~30 lines),
`HitTestBehavior` branching (~20 lines), `interact_radius` with tie-breaking (~40 lines),
clip handling (~10 lines). Estimated total: ~250 lines. Well under 500. If
`interact_radius` tie-breaking pushes the file larger, extract it into a
`hit_test/interact_radius.rs` submodule.

#### Sense filtering

- [x] Modify **all three** public hit test functions to apply Sense filtering:
  - `layout_hit_test()` — returns `Option<WidgetId>`, unchanged return type. Skip nodes
    where `node.sense.is_none() && node.widget_id.is_some()` (or `node.disabled`).
    Production callers (`container/mod.rs:310` `hit_test_children`,
    `routing.rs:107` `InputState::process_mouse_event`) continue to work unchanged.
  - `layout_hit_test_clipped()` — same Sense filtering as `layout_hit_test()`. Already
    handles external clip rects.
  - `layout_hit_test_path()` — change return type from `Vec<WidgetId>` to
    `WidgetHitTestResult` (see below). Same Sense filtering logic.

  Layout wrapper nodes without `widget_id` (used for structural grouping) are always
  traversed regardless of Sense — only nodes with `widget_id.is_some()` are filtered.

#### WidgetHitTestResult and HitEntry types

- [x] Define `WidgetHitTestResult` and `HitEntry` in `input/hit_test.rs`. Path ordering
  is **root-to-leaf** (outermost ancestor first, deepest hit widget last), matching
  `update_hot_path`'s expectation and `layout_hit_test_path`'s current `Vec<WidgetId>`
  ordering.
  ```rust
  /// Result of a path-collecting hit test.
  ///
  /// Contains the full ancestor chain from root to the deepest hit widget,
  /// with bounds and sense data for each entry.
  #[derive(Debug, Clone)]
  pub struct WidgetHitTestResult {
      /// Widgets hit, ordered root-to-leaf (outermost ancestor first,
      /// deepest hit widget last). Matches `update_hot_path` ordering.
      pub path: Vec<HitEntry>,
  }

  /// A single entry in a hit test path.
  #[derive(Debug, Clone, Copy, PartialEq)]
  pub struct HitEntry {
      /// The widget that was hit.
      pub widget_id: WidgetId,
      /// The widget's layout bounds (unmodified by `interact_radius`).
      pub bounds: Rect,
      /// The widget's declared sense.
      pub sense: Sense,
  }

  impl WidgetHitTestResult {
      /// Extracts widget IDs for passing to `InteractionManager::update_hot_path`.
      pub fn widget_ids(&self) -> Vec<WidgetId> {
          self.path.iter().map(|e| e.widget_id).collect()
      }

      /// Returns the deepest (leaf) hit entry, if any.
      pub fn deepest(&self) -> Option<&HitEntry> {
          self.path.last()
      }

      /// Whether any widget was hit.
      #[must_use]
      pub fn is_empty(&self) -> bool {
          self.path.is_empty()
      }
  }
  ```
- [x] Re-export `WidgetHitTestResult` and `HitEntry` from `oriterm_ui/src/input/mod.rs`.

#### Return type change blast radius

- [x] **Update `layout_hit_test_path` callers** after changing its return type:
  - `oriterm_ui/src/input/mod.rs` line 15 — update re-export.
  - `oriterm_ui/src/interaction/tests.rs` — **5** call sites (lines 559, 569, 579, 595,
    613). Currently compare against `Vec<WidgetId>`. Update to use `.widget_ids()`:
    `assert_eq!(path.widget_ids(), vec![root_id, mid_id, leaf_id])`.
  - No production callers outside tests yet. The event pipeline wiring is in Section 03.

#### HitTestBehavior logic in recursive functions

- [x] Modify the recursive `hit_test_node()` and `hit_test_path_node()` to handle
  `HitTestBehavior`:
  1. **`DeferToChild`** (default): Test children first (back-to-front). If a child hits,
     return it. If no child hits but the node itself has a `widget_id` and non-none Sense,
     return the node. (Current behavior, unchanged.)
  2. **`Opaque`**: When the point is inside the node's rect, return the node immediately
     without testing children. The node absorbs the event.
  3. **`Translucent`**: Include both the node and any hit children in the path. Relevant
     only for `hit_test_path_node()` (the path-collecting variant). For `hit_test_node()`
     (which returns a single `WidgetId`), Translucent behaves like DeferToChild (deepest
     child wins).

#### Clip flag handling

- [x] When a `LayoutNode` has `clip: true`, constrain child hit testing to the node's
  `rect`. Set the clip rect to `node.rect` before recursing into children.

  **`layout_hit_test_clipped()` retained**: The `LayoutNode.clip` flag handles
  `ContainerWidget::clip_children`. The external `clip: Option<Rect>` parameter on
  `layout_hit_test_clipped()` handles scroll viewport clipping (where the clip rect
  differs from the layout node's rect). Both mechanisms coexist.

#### interact_radius

- [x] Implement `interact_radius` expansion in hit testing. `interact_radius` is already
  stored on `LayoutNode` (02.2b). Widgets that need expanded hit areas (close buttons,
  scrollbar thumbs) set it to `2.0` in their `layout()` impl. The hit test function
  inflates `node.rect` by `interact_radius` for containment checks only — the `bounds`
  field in `HitEntry` stores the original (uninflated) rect.

- [x] **Tie-breaking**: When `interact_radius` causes overlapping inflated rects among
  siblings, the widget whose center is nearest to the test point wins. Implementation:
  1. When iterating a parent's children (back-to-front), check if any child in the group
     has `interact_radius > 0.0`. If none do, use the fast path (return on first hit).
  2. If any child has `interact_radius > 0.0`, collect all sibling candidates whose
     inflated rect contains the test point, then pick the one with the nearest center.
  3. This tie-breaking applies only within a single parent's children, not across the
     tree.

  **Complexity note**: This is the most complex part of Section 02. Consider extracting
  the tie-breaking logic into a helper function (e.g., `pick_nearest_candidate()`) to
  keep `hit_test_node()` under 50 lines.

#### Integration with InteractionManager

- [x] Feed `WidgetHitTestResult` to `InteractionManager::update_hot_path()`:
  extract `result.widget_ids()` (root-to-leaf `Vec<WidgetId>`) and pass the slice to
  `update_hot_path(&mut self, new_path: &[WidgetId])`. No API change needed on
  `InteractionManager`.

---

## 02.3 Completion Checklist

### Types and modules

- [x] `Sense` manual bitmask newtype in `oriterm_ui/src/sense/mod.rs` with `none()`,
  `click()`, `drag()`, `click_and_drag()`, `hover()`, `focusable()`, `all()`, `union()`,
  `is_none()`, `Default` impl, and manual `Debug` impl
- [x] `sense/tests.rs` with unit tests for `union()`, `is_none()`, `Default`, flag
  composition
- [x] `pub mod sense;` declared in `oriterm_ui/src/lib.rs`
- [x] `crate::interaction` re-exports `Sense` for convenience
- [x] `HitTestBehavior` enum in `oriterm_ui/src/hit_test_behavior.rs` with `DeferToChild`,
  `Opaque`, `Translucent` and `Default` impl
- [x] `pub mod hit_test_behavior;` declared in `oriterm_ui/src/lib.rs`
- [x] `HitTestBehavior` re-exported from `oriterm_ui/src/input/mod.rs` for convenience
- [x] **No circular imports verified**: `sense/mod.rs` and `hit_test_behavior.rs` have zero
  intra-crate imports. `layout/` imports them directly via `crate::sense` and
  `crate::hit_test_behavior`. `layout/` has zero imports from `input/` or `interaction/`.

### Layout tree extensions

- [x] `LayoutNode` has 5 new fields: `sense`, `hit_test_behavior`, `clip`, `disabled`,
  `interact_radius` with defaults (none/DeferToChild/false/false/0.0)
- [x] `LayoutNode` builder methods: `with_sense()`, `with_hit_test_behavior()`,
  `with_clip()`, `with_disabled()`, `with_interact_radius()`
- [x] `LayoutBox` has matching 5 fields with builder methods
- [x] Solver propagates all 5 fields from `LayoutBox` to `LayoutNode` in **all three**
  solve functions: `solve_leaf` (line 100), `arrange_children` (line 327),
  `solve_empty` (line 353)
- [x] `ContainerWidget::build_layout_box()` sets `clip: true` when
  `self.clip_children` is true

### Widget trait integration

- [x] `sense()` default method on `Widget` trait returns `Sense::all()` (preserves current
  behavior; Section 08 overrides per-widget)
- [x] `hit_test_behavior()` default method on `Widget` trait returns `DeferToChild`
- [x] No name collision with existing widget methods (verified before adding)

### Hit test function changes

- [x] All three hit test functions (`layout_hit_test`, `layout_hit_test_clipped`,
  `layout_hit_test_path`) skip `Sense::none()` and disabled nodes
- [x] `WidgetHitTestResult` and `HitEntry` structs defined in `input/hit_test.rs`
- [x] `WidgetHitTestResult` path ordered **root-to-leaf** (matches `update_hot_path`)
- [x] `WidgetHitTestResult::widget_ids()` convenience method for `update_hot_path`
- [x] `WidgetHitTestResult` and `HitEntry` re-exported from `input/mod.rs`
- [x] `HitTestBehavior::Opaque` blocks children behind the node
- [x] `HitTestBehavior::DeferToChild` passes to children first (default, current behavior)
- [x] `HitTestBehavior::Translucent` includes both parent and children in path
- [x] `LayoutNode.clip == true` clips children to parent rect during hit testing
- [x] `interact_radius` inflates hit area; center-distance tie-breaking among siblings
- [x] `WidgetHitTestResult` fed to `InteractionManager::update_hot_path()` via
  `.widget_ids()`

### Blast radius updates

- [x] `input/tests.rs` `make_node` helper updated with 5 new `LayoutNode` fields
- [x] `interaction/tests.rs` `layout_hit_test_path` call sites updated for
  `WidgetHitTestResult` return type (5 call sites: lines 559, 569, 579, 595, 613)

### Unit tests (in `oriterm_ui/src/input/tests.rs`)

- [x] Label between two buttons does not block click on button behind it
- [x] `Sense::none()` node is skipped; widget with `Sense::click()` behind it receives hit
- [x] `interact_radius` makes 10px widget clickable from 12px away (radius 2.0)
- [x] `interact_radius` tie-breaking: two adjacent widgets, point equidistant between them,
  nearest center wins
- [x] Clipping container (`clip: true`) prevents hit testing children outside clip bounds
- [x] `HitTestBehavior::Opaque` — container absorbs hit, children do not appear in path
- [x] `HitTestBehavior::Translucent` — both parent and child appear in path
- [x] Disabled widget (disabled: true) is skipped during hit testing

### Build verification

- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Hit testing with a tree of
`[ContainerWidget(LabelWidget, ButtonWidget, LabelWidget)]` correctly skips both
`LabelWidget`s and returns only the `ButtonWidget` in the hit path.
`WidgetHitTestResult` path is root-to-leaf and `widget_ids()` can be passed directly
to `update_hot_path()`.
