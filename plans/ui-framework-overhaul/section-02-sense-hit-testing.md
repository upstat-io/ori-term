---
section: "02"
title: "Sense & Hit Testing"
status: not-started
goal: "Widgets declare what interactions they care about; hit testing respects Sense + interact_radius"
inspired_by:
  - "egui Sense enum (egui/src/sense.rs)"
  - "Flutter HitTestBehavior (rendering/proxy_box.dart)"
depends_on: ["01"]
reviewed: false
sections:
  - id: "02.1"
    title: "Sense Enum"
    status: not-started
  - id: "02.2"
    title: "Hit Testing Overhaul"
    status: not-started
  - id: "02.3"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Sense & Hit Testing

**Status:** Not Started
**Goal:** Every widget declares a `Sense` (what interactions it cares about). Hit testing
filters by Sense, skipping widgets that don't care about the current event type. Labels
(`Sense::None`) are never hit-tested. An `interact_radius` extends hit areas for small
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

## 02.1 Sense Enum

**File(s):** `oriterm_ui/src/interaction/sense.rs`

- [ ] Define `Sense` as a bitflag set. **Note:** `bitflags` is not currently a dependency of
  `oriterm_ui` — either add `bitflags = "2"` to `oriterm_ui/Cargo.toml`, or implement the
  bitmask manually (4 flags is simple enough for a manual impl):
  ```rust
  bitflags! {
      pub struct Sense: u8 {
          const HOVER = 0b0001;      // Receives hover tracking (hot state)
          const CLICK = 0b0010;      // Receives click events
          const DRAG  = 0b0100;      // Receives drag events
          const FOCUS = 0b1000;      // Can receive keyboard focus
      }
  }

  impl Sense {
      pub const fn none() -> Self { Self::empty() }
      pub const fn hover() -> Self { Self::HOVER }
      pub const fn click() -> Self { Self::HOVER.union(Self::CLICK) }
      pub const fn drag() -> Self { Self::HOVER.union(Self::DRAG) }
      pub const fn click_and_drag() -> Self {
          Self::HOVER.union(Self::CLICK).union(Self::DRAG)
      }
      pub const fn focusable() -> Self { Self::FOCUS }
      pub const fn all() -> Self {
          Self::HOVER.union(Self::CLICK).union(Self::DRAG).union(Self::FOCUS)
      }
  }
  ```
- [ ] Add `fn sense(&self) -> Sense` to the Widget trait (see Section 08 for full trait)
- [ ] Default Sense values for existing widgets:
  - `Label`: `Sense::none()`
  - `Separator`: `Sense::none()`
  - `Spacer`: `Sense::none()`
  - `Button`: `Sense::click()`
  - `Toggle`, `Checkbox`: `Sense::click()`
  - `Dropdown`: `Sense::click().union(Sense::FOCUS)`
  - `Slider`: `Sense::drag().union(Sense::FOCUS)`
  - `TextInput`: `Sense::click_and_drag().union(Sense::FOCUS)`
  - `ScrollWidget`: `Sense::drag()` (for scrollbar)
  - `Container`: `Sense::none()` (default, overridable)

---

## 02.2 Hit Testing Overhaul

**File(s):** `oriterm_ui/src/input/hit_test.rs`

- [ ] Add `HitTestBehavior` enum:
  ```rust
  pub enum HitTestBehavior {
      /// Hit-test children first; self only if no child handles (default).
      DeferToChild,
      /// This widget absorbs the event — children behind it don't receive.
      Opaque,
      /// Both this widget and children behind it can receive the event.
      Translucent,
  }
  ```
- [ ] Modify `layout_hit_test()` to:
  1. Skip widgets with `Sense::none()` (never participate in hit testing)
  2. Respect `HitTestBehavior` for containers
  3. Return `HitTestResult` with the full hit path (not just deepest widget):
     ```rust
     pub struct HitTestResult {
         /// Widgets hit, from deepest to shallowest.
         pub path: Vec<HitEntry>,
     }

     pub struct HitEntry {
         pub widget_id: WidgetId,
         pub bounds: Rect,
         pub sense: Sense,
     }
     ```
- [ ] Implement `interact_radius: f32` (default 2.0px):
  - Extend widget bounds by `interact_radius` during hit testing
  - When multiple widgets overlap due to extended bounds, nearest center wins
  - Helps with small targets (close buttons, scrollbar thumbs)
- [ ] Feed `HitTestResult` to `InteractionManager::update_hot_path()`
- [ ] Respect `ContainerWidget::clip_children` during hit testing: when a container
  has `clip_children = true`, its child layout rects are clipped to the container's
  bounds. The existing `layout_hit_test_clipped()` function already accepts a clip
  rect — the modified hit test must propagate the clip rect from clipping containers
  to their children.
- [ ] **Ordering note**: `fn sense(&self) -> Sense` is added to the Widget trait in
  Section 08. During Section 02 implementation, the hit test can use a temporary
  `fn widget_sense(id: WidgetId) -> Sense` lookup or implement the trait change early.
  The recommended approach is to add `sense()` as a default method returning
  `Sense::click()` (backward compatible) in Section 02, then refine in Section 08.

---

## 02.3 Completion Checklist

- [ ] `Sense` bitflag set with `none()`, `click()`, `drag()`, `click_and_drag()`,
  `hover()`, `focusable()`, `all()`
- [ ] Hit testing skips `Sense::none()` widgets
- [ ] `HitTestResult` returns full path from deepest to shallowest
- [ ] `interact_radius` extends hit areas for small widgets
- [ ] `HitTestBehavior::Opaque` blocks children behind
- [ ] `HitTestBehavior::DeferToChild` passes to children first (default)
- [ ] Unit tests: label between two buttons doesn't block click on button behind it
- [ ] Unit tests: interact_radius makes 10px widget clickable from 12px away
- [ ] Unit tests: clipping container prevents hit testing children outside clip bounds
- [ ] Test file: `oriterm_ui/src/input/tests.rs` (expand existing hit test tests)
- [ ] `./test-all.sh` green, `./clippy-all.sh` green, `./build-all.sh` green

**Exit Criteria:** Hit testing with a tree of [Container(Label, Button, Label)] correctly
skips both Labels and returns only the Button in the hit path.
