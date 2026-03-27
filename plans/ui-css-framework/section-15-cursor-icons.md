---
section: "15"
title: "Mouse Cursor Icons"
status: complete
reviewed: true
third_party_review:
  status: resolved
  updated: "2026-03-27"
goal: "Dialog widgets change the OS mouse cursor to match the mockup — pointer for clickable elements, not-allowed for disabled, crosshair for color pickers"
depends_on: ["12", "13"]
sections:
  - id: "15.1"
    title: "Cursor Icon on LayoutBox/LayoutNode"
    status: complete
  - id: "15.2"
    title: "Widget Cursor Declarations"
    status: complete
  - id: "15.3"
    title: "Dialog Cursor Application"
    status: complete
  - id: "15.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "15.4"
    title: "Tests + Build Gate"
    status: complete
---

# Section 15: Mouse Cursor Icons

## Problem

The mockup declares `cursor: pointer` on every clickable element — nav items, buttons, toggles,
selects, scheme cards, color cells, sliders, keybind rows, and range inputs. Disabled primary
buttons use `cursor: not-allowed`. The color picker area uses `cursor: crosshair`. The app
currently shows the default arrow cursor everywhere in the settings dialog.

The main window already has cursor support for URL hover (`CursorIcon::Pointer` in
`oriterm/src/app/cursor_hover.rs`) and floating pane drag (`CursorIcon::Grab`/`Grabbing` in
`oriterm/src/app/floating_drag.rs`). Both use winit's `window.set_cursor(CursorIcon::*)`.

The dialog window lacks any cursor management — it never calls `set_cursor`.

### Mockup Cursor Declarations

From `mockups/settings-brutal.html`:

| Selector | Cursor | Widget |
|----------|--------|--------|
| `.nav-item` | pointer | SidebarNavWidget (whole widget — uses `on_input` hit testing, not per-item child widgets) |
| `.sidebar-update` | pointer | SidebarNavWidget footer link |
| `.toggle` | pointer | ToggleWidget |
| `select` | pointer | DropdownWidget |
| `.scheme-card` | pointer | SchemeCardWidget |
| `.btn-sm` | pointer | Small buttons (ButtonWidget) |
| `.color-cell` | pointer | Color cells |
| `.keybind-row` | pointer | KeybindRow |
| `.cursor-option` | pointer | CursorPickerWidget (whole widget — uses internal `hovered_card` hit testing) |
| `.btn` | pointer | All buttons (Reset, Cancel, Save) |
| `.btn-primary:disabled` | not-allowed | Disabled Save button |
| `input[type="range"]::-webkit-slider-thumb` | pointer | SliderWidget thumb |
| `.color-area-wrap` | crosshair | Color picker area (future) |
| `.hue-strip-wrap` | pointer | Hue strip (future) |
| `.alpha-strip-wrap` | pointer | Alpha strip (future) |
| `.btn-test` | pointer | Test buttons |

### Existing Infrastructure

- `winit::window::CursorIcon` — the enum (from the `cursor-icon` crate, re-exported via
  `winit::window::CursorIcon`): `Default`, `Pointer`, `Grab`, `Grabbing`, `NotAllowed`,
  `Crosshair`, `Text`, etc. Derives `Debug, Default, Copy, Clone, PartialEq, Eq, Hash` —
  compatible with all derive macros on `LayoutBox`, `LayoutNode`, and `HitEntry`.
- winit is already a dependency of `oriterm_ui` (see `oriterm_ui/Cargo.toml`), so `CursorIcon`
  can be used directly without a local enum or conversion layer.
- `cursor_hover.rs` — main window cursor management. Returns `HoverResult { cursor_icon }`
  from `detect_hover_url()`, then `update_url_hover()` calls
  `ctx.window.window().set_cursor(result.cursor_icon)`. Note: main window uses
  `ctx.window.window().set_cursor()` (WindowSurface wrapper), while dialog uses
  `ctx.window.set_cursor()` directly (`Arc<Window>`).
- The dialog event handler (`dialog_context/event_handling/mod.rs`) already does hit-testing
  for hover state via `layout_hit_test_path()` at line ~319. The cursor icon should be
  resolved from the same hit path.

### Key Architectural Constraint: Disabled Widgets and Hit Testing

The hit-test function `is_hittable()` in `oriterm_ui/src/input/hit_test.rs` (line 94) skips
disabled nodes (`!node.disabled`). Both `layout_hit_test()` and `layout_hit_test_path()` use
this gate. Disabled widgets do NOT appear in the hit-test path, so the normal cursor resolution
(`hit.path.last().cursor_icon`) cannot return `NotAllowed` for disabled widgets.

**Solution:** Carry the cursor icon on `LayoutBox` -> `LayoutNode` (same pattern as `sense`,
`disabled`, `hit_test_behavior`). Widgets always set their natural cursor (`Pointer` for
buttons) regardless of disabled state. A separate **disabled-node scan** function
(`layout_hit_test_disabled_at()`) walks the layout tree recursively looking for disabled
nodes whose `cursor_icon != Default` and whose rect contains the point, returning `true`.
The dialog handler calls this fallback when the normal hit path yields `Default` or is empty,
and converts `true` to `CursorIcon::NotAllowed`.

**Important:** The disabled scan must be recursive (disabled buttons are nested inside
non-disabled containers like form rows). It must also respect `content_offset` for widgets
inside `ScrollWidget` — translate the test point by the parent's `content_offset` before
checking children, matching the same logic in `hit_test_path_node()`. It must also respect
`pointer_events: false` and `clip` to stay consistent with normal hit testing.

### Container Widgets with Per-Item Cursors

`SidebarNavWidget` and `CursorPickerWidget` are **single leaf widgets** with `Sense::click()`.
They handle per-item hover internally via `on_input` (SidebarNavWidget tracks `hovered_item`,
CursorPickerWidget tracks `hovered_card`). These widgets do NOT emit child `LayoutBox` nodes
for individual items — both return `LayoutBox::leaf(...)` from `layout()`. Setting
`.with_cursor_icon(CursorIcon::Pointer)` on the leaf covers the whole widget. Both should
return `Pointer` since every region within them is clickable. No per-item cursor variation
is needed.

---

## 15.1 Cursor Icon on LayoutBox/LayoutNode

### Goal

Add a `cursor_icon` field to `LayoutBox` and `LayoutNode` so the cursor flows through the
layout tree alongside `sense` and `disabled`. The hit-test path reads the cursor from the
layout node directly, without needing to find and call a method on the widget object (which
requires `&mut` tree traversal). This follows the established pattern: `sense`, `disabled`,
`hit_test_behavior`, and `clip` all flow from `LayoutBox` -> `LayoutNode` -> hit testing.

All cursors are **native OS cursors** via winit — no custom cursor rendering. winit's
`CursorIcon` maps to native cursors on all three platforms (Windows, macOS, Linux/Wayland/X11)
automatically — no platform-specific code needed.

### Approach

1. Add `pub cursor_icon: CursorIcon` to `LayoutBox` (default `CursorIcon::Default`)
2. Add `pub cursor_icon: CursorIcon` to `LayoutNode` (propagated from `LayoutBox` by solver)
3. Add `pub cursor_icon: CursorIcon` to `HitEntry` (populated during hit-test traversal)
4. Widgets set cursor via builder: `.with_cursor_icon(CursorIcon::Pointer)` in their `layout()` impl
5. The dialog cursor-move handler reads `hit.path.last().map(|e| e.cursor_icon)` to get the
   leaf widget's declared cursor

**For disabled widgets (NotAllowed cursor):** The `LayoutNode.disabled` flag is already
propagated. Since `is_hittable()` skips disabled nodes, we need a separate scan. Add a
`layout_hit_test_disabled_at()` helper that walks the layout tree looking for disabled
nodes (with non-default `cursor_icon`) whose rect contains the point. The dialog handler
calls this when the normal hit path yields no cursor override.

### Checklist

- [x] Add `use winit::window::CursorIcon;` import to `oriterm_ui/src/layout/layout_box.rs`
- [x] Add `pub cursor_icon: CursorIcon` field to `LayoutBox` (after `pointer_events`, before the `impl` block)
- [x] Default `CursorIcon::Default` in all three constructors: `LayoutBox::leaf()`, `LayoutBox::flex()`, `LayoutBox::grid()`
- [x] Add `with_cursor_icon(icon: CursorIcon) -> Self` builder method on `LayoutBox` (follows `with_pointer_events()` pattern)
- [x] In `LayoutBox::for_layout_only()`, reset `cursor_icon` to `CursorIcon::Default` alongside the `sense = Sense::none()` clear (hidden widgets should not influence cursor)
- [x] Add `use winit::window::CursorIcon;` import to `oriterm_ui/src/layout/layout_node.rs`
- [x] Add `pub cursor_icon: CursorIcon` field to `LayoutNode` (after `pointer_events`)
- [x] Default `CursorIcon::Default` in `LayoutNode::new()` constructor
- [x] Propagate `cursor_icon` from `LayoutBox` -> `LayoutNode` at **all five** solver sites where `pointer_events` is already copied: three in `oriterm_ui/src/layout/solver.rs` (`solve_leaf` around line 123, `solve_flex` around line 364, `solve_empty` around line 397) and two in `oriterm_ui/src/layout/grid_solver.rs` (the non-empty grid path around line 102, and the empty grid fallback around line 177). Add `node.cursor_icon = layout_box.cursor_icon;` at each site, directly after the `node.pointer_events` line.
- [x] Add `pub cursor_icon: CursorIcon` field to `HitEntry` in `oriterm_ui/src/input/hit_test.rs`
- [x] Populate `cursor_icon` in `hit_test_path_node()` line 270 (where `HitEntry` is constructed) from `node.cursor_icon`
- [x] Add `layout_hit_test_disabled_at(root: &LayoutNode, point: Point) -> bool` helper in `oriterm_ui/src/input/hit_test.rs` — recursive scan that returns `true` if any disabled node with non-default `cursor_icon` contains `point`. Must respect `content_offset` (scroll), `clip`, and `pointer_events: false` to stay consistent with normal hit testing. The caller converts `true` to `CursorIcon::NotAllowed`. File is 397 lines today; this function adds ~60-80 lines (~460-477 total). Under the 500-line limit but if it grows beyond estimate, extract a `disabled_scan.rs` submodule.
- [x] Add `layout_hit_test_disabled_at` to the re-export list in `oriterm_ui/src/input/mod.rs` (line 17-19, alongside `layout_hit_test`, `layout_hit_test_clipped`, `layout_hit_test_path`)
- [x] Re-export `winit::window::CursorIcon` from `oriterm_ui/src/lib.rs` for ergonomic widget imports (e.g. `oriterm_ui::CursorIcon`)
- [x] Verify the Widget trait is NOT modified (no new method — cursor flows via layout, not via trait method)

---

## 15.2 Widget Cursor Declarations

### Goal

Every clickable widget sets `cursor_icon: CursorIcon::Pointer` (or `Text` for text inputs)
on its `LayoutBox` in its `layout()` implementation, **regardless of disabled state**. Widgets
always declare their natural cursor. The disabled-node scan in 15.1 converts any non-default
cursor on a disabled node to `CursorIcon::NotAllowed` at the dialog handler level. This means
widgets do NOT need special disabled-cursor logic — they just set their cursor unconditionally.

### Checklist

- [x] `ButtonWidget::layout()` in `oriterm_ui/src/widgets/button/mod.rs` — add `.with_cursor_icon(CursorIcon::Pointer)` after `.with_disabled(self.disabled)` (line ~260). Set unconditionally — disabled scan handles `NotAllowed` for disabled buttons.
- [x] `IdOverrideButton::layout()` in `oriterm_ui/src/widgets/button/id_override/mod.rs` — no change needed. It delegates to `self.inner.layout(ctx)` then overrides only `widget_id`. The cursor from the inner `ButtonWidget` is inherited automatically.
- [x] `ToggleWidget::layout()` in `oriterm_ui/src/widgets/toggle/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_disabled(self.disabled)` (line ~272)
- [x] `DropdownWidget::layout()` in `oriterm_ui/src/widgets/dropdown/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_disabled(self.disabled)` (line ~260)
- [x] `SliderWidget::layout()` in `oriterm_ui/src/widgets/slider/widget_impl.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_disabled(self.disabled)` (line ~46). Entire track gets pointer, not just thumb.
- [x] `CheckboxWidget::layout()` in `oriterm_ui/src/widgets/checkbox/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_disabled(self.disabled)` (line ~227)
- [x] `SidebarNavWidget::layout()` in `oriterm_ui/src/widgets/sidebar_nav/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_widget_id(self.id)` (line ~325). Whole widget is clickable — nav items, search, footer links.
- [x] `SchemeCardWidget::layout()` in `oriterm_ui/src/widgets/scheme_card/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_widget_id(self.id)` (line ~230)
- [x] `KeybindRow::layout()` in `oriterm_ui/src/widgets/keybind/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_widget_id(self.id)` (line ~215). No `disabled` field on this widget — cursor set unconditionally.
- [x] `CursorPickerWidget::layout()` in `oriterm_ui/src/widgets/cursor_picker/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_widget_id(self.id)` (line ~166). Whole widget — per-card hover is internal.
- [x] `ColorSwatchGrid::layout()` in `oriterm_ui/src/widgets/color_swatch/mod.rs` — `.with_cursor_icon(CursorIcon::Pointer)` after `.with_widget_id(self.id)` (line ~113). Single leaf with internal per-cell click handling (`.color-cell` in mockup).
- [x] `SpecialColorSwatch::layout()` — no cursor change needed. `Sense::hover()` only, no click handler (`on_input` not implemented). Default cursor is correct. If click behavior is added later, add `CursorIcon::Pointer` at that time.
- [x] `TextInputWidget::layout()` in `oriterm_ui/src/widgets/text_input/widget_impl.rs` — `.with_cursor_icon(CursorIcon::Text)` after `.with_disabled(self.disabled)` (line ~65)
- [x] `NumberInputWidget::layout()` — no cursor change needed. This is a numeric stepper with clickable up/down buttons, but its cursor convention matches native HTML `<input type="number">` which uses the default arrow cursor. The click targets are small embedded buttons, not the whole widget surface.
- [x] All other widgets (containers, labels, separators, spacers, panels) — leave default (`CursorIcon::Default`)
- [x] Add unit tests in each widget's `tests.rs`: verify `layout()` returns correct `cursor_icon` for enabled and disabled states (enabled should be `Pointer` or `Text`; disabled should still be the same natural cursor — the `NotAllowed` conversion happens in the dialog handler, not in the layout)

---

## 15.3 Dialog Cursor Application

### Goal

The dialog's cursor-move handler reads the cursor from the hit-test result and applies it to
the window. When no interactive widget is under the cursor, revert to `Default`. When a
disabled widget with a cursor override is under the pointer, show `NotAllowed`.

### Files

- `oriterm/src/app/dialog_context/event_handling/mod.rs` — `handle_dialog_cursor_move()` (line ~237)
- `oriterm/src/app/dialog_context/content_key_dispatch.rs` — `clear_dialog_hover()` (line ~146)
- `oriterm/src/app/dialog_context/mod.rs` — `DialogWindowContext` struct (add `last_cursor_icon` field)

### Overlay and Chrome Scoping

**Overlay early-return:** `handle_dialog_cursor_move()` already returns early when an overlay
(dropdown popup) consumes the cursor event (lines 270-277). When the overlay consumes the
move, set cursor to `Default` before returning (overlay items don't declare cursors through
the layout tree).

**Chrome area:** When the cursor is in the chrome area (`logical_pos.y < chrome_h`), no
content layout exists. The cursor should remain `Default` (close button hover doesn't need
a special cursor). No change needed for the chrome path beyond resetting if the previous
frame had a non-default cursor.

### Approach

After the existing `layout_hit_test_path()` call in `handle_dialog_cursor_move()` (line ~319):
1. Read `hit.path.last().map(|e| e.cursor_icon)` for the leaf widget cursor
2. If that's `Default` (or path is empty), call `layout_hit_test_disabled_at()` to check
   for disabled widgets under the pointer
3. Call `ctx.window.set_cursor(resolved_cursor)` with the final cursor (dialog window is
   `Arc<Window>`, no `.window()` wrapper needed)

### Checklist

- [x] Add `pub last_cursor_icon: CursorIcon` field to `DialogWindowContext` in `oriterm/src/app/dialog_context/mod.rs` (after `last_cursor_pos`, default `CursorIcon::Default`)
- [x] In `handle_dialog_cursor_move` content path (after `layout_hit_test_path` call at line ~319), extract cursor: `let leaf_cursor = hit.path.last().map(|e| e.cursor_icon).unwrap_or(CursorIcon::Default);`
- [x] If `leaf_cursor == CursorIcon::Default`, call `layout_hit_test_disabled_at(&layout_node, local)` to check disabled widgets. If it returns `true`, use `CursorIcon::NotAllowed` as the resolved cursor. Note: `layout_node` is `Rc<LayoutNode>` — dereference via `&*layout_node` or `layout_node.as_ref()`.
- [x] Compare resolved cursor against `ctx.last_cursor_icon`. Only call `ctx.window.set_cursor(cursor)` when it differs (avoids redundant OS cursor changes). Update `ctx.last_cursor_icon`.
- [x] When cursor is in chrome area (`logical_pos.y < chrome_h`) and `last_cursor_icon` is not `Default`, reset to `Default`
- [x] When overlay consumes the cursor event (the early-return path at line ~270), reset cursor to `Default` if `last_cursor_icon` is not `Default`
- [x] In `clear_dialog_hover()` in `content_key_dispatch.rs` (line ~146), also set cursor to `Default` and reset `last_cursor_icon` — this is called on `CursorLeft` events

### Tests

**Note:** `handle_dialog_cursor_move` is in `oriterm` (depends on `Arc<Window>`, GPU context).
It cannot be tested headlessly via `WidgetTestHarness`. The cursor resolution logic (read leaf
cursor from hit path, fall back to disabled scan, deduplicate via `last_cursor_icon`) could be
extracted into a pure function for unit testing, but the logic is simple enough (3 lines of
branching) that extraction is not worth the indirection. Manual verification covers the
behavior:

- [x] Manual: hover button -> cursor changes to pointer
- [x] Manual: move to empty content area -> cursor reverts to default arrow
- [x] Manual: hover disabled Save button -> cursor shows not-allowed
- [x] Manual: move from content to chrome area -> cursor reverts to default arrow
- [x] Manual: move cursor out of dialog window -> cursor reverts to default arrow
- [x] Manual: hover toggle -> pointer, hover dropdown -> pointer, hover slider -> pointer
- [x] Manual: hover text input -> text cursor (I-beam)
- [x] Manual: hover keybind row -> pointer
- [x] Manual: overlay active (dropdown open) -> pointer over menu items (fixed: menu widget now declares CursorIcon::Pointer, overlay cursor resolution reads layout tree)

---

## 15.R Third Party Review Findings

- [x] `[TPR-15-001][medium]` — Accepted 2026-03-27: content_offset test was incorrect. Rewrote test with correct coordinate convention (child at y=110 in content space, offset -100, viewport y=20 maps to content y=120).

- [x] `[TPR-15-002][medium]` — Accepted 2026-03-27: `disabled_scan_node()` now uses `point_in_hit_area()` instead of `rect.contains()`. Added `disabled_scan_respects_interact_radius` regression test.

- [x] `[TPR-15-003][low]` — Accepted 2026-03-27: extracted `route_overlay_hover` and `hit_test_content` into `cursor.rs` submodule. `mod.rs` now 438 lines.

---

## 15.4 Tests + Build Gate

### Checklist

**Widget layout cursor tests** (in each widget's `tests.rs`):
- [x] `button/tests.rs::layout_cursor_icon_pointer` — enabled `ButtonWidget::layout()` returns `LayoutBox` with `cursor_icon == CursorIcon::Pointer`
- [x] `button/tests.rs::layout_disabled_preserves_natural_cursor` — disabled `ButtonWidget::layout()` returns `cursor_icon == CursorIcon::Pointer` AND `disabled == true` (natural cursor preserved; `NotAllowed` conversion is the dialog handler's job)
- [x] `toggle/tests.rs::layout_cursor_icon_pointer` — `ToggleWidget::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `dropdown/tests.rs::layout_cursor_icon_pointer` — `DropdownWidget::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `slider/tests.rs::layout_cursor_icon_pointer` — `SliderWidget::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `checkbox/tests.rs::layout_cursor_icon_pointer` — `CheckboxWidget::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `text_input/tests.rs::layout_cursor_icon_text` — `TextInputWidget::layout()` returns `cursor_icon == CursorIcon::Text`
- [x] `sidebar_nav/tests.rs::layout_cursor_icon_pointer` — `SidebarNavWidget::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `scheme_card/tests.rs::layout_cursor_icon_pointer` — `SchemeCardWidget::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `keybind/tests.rs::layout_cursor_icon_pointer` — `KeybindRow::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `cursor_picker/tests.rs::layout_cursor_icon_pointer` — `CursorPickerWidget::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `color_swatch/tests.rs::layout_cursor_icon_pointer_grid` — `ColorSwatchGrid::layout()` returns `cursor_icon == CursorIcon::Pointer`
- [x] `color_swatch/tests.rs::layout_cursor_icon_default_special` — `SpecialColorSwatch::layout()` returns `cursor_icon == CursorIcon::Default` (hover-only, no click handler)
- [x] `number_input/tests.rs::layout_cursor_icon_default` — `NumberInputWidget::layout()` returns `cursor_icon == CursorIcon::Default` (native number input convention)

**Hit test infrastructure tests** (in `oriterm_ui/src/input/tests.rs`, alongside existing hit test tests):
- [x] Update `make_node()` helper in `input/tests.rs` to include `cursor_icon: CursorIcon::Default` field (existing helper at line 17 constructs `LayoutNode` with all fields as a struct literal — must add the new field or it won't compile)
- [x] `hit_entry_carries_cursor_icon` — build a `LayoutNode` with `cursor_icon == Pointer`, run `layout_hit_test_path()`, assert `hit.path.last().cursor_icon == CursorIcon::Pointer`
- [x] `hit_entry_cursor_default_for_no_cursor_widget` — build a `LayoutNode` with default cursor, run `layout_hit_test_path()`, assert `hit.path.last().cursor_icon == CursorIcon::Default`
- [x] `hit_path_nested_cursor_leaf_wins` — parent with `Default` cursor, child with `Pointer` cursor. Assert the path's last entry has `Pointer` (leaf cursor takes priority in resolution)
- [x] `disabled_scan_hits_disabled_pointer_node` — `layout_hit_test_disabled_at()` returns `true` for disabled node with `cursor_icon == Pointer` at point inside its rect
- [x] `disabled_scan_misses_outside_point` — `layout_hit_test_disabled_at()` returns `false` when point is outside disabled node's rect
- [x] `disabled_scan_ignores_default_cursor` — `layout_hit_test_disabled_at()` returns `false` when disabled node has `cursor_icon == Default` (non-interactive disabled container should not trigger NotAllowed)
- [x] `disabled_scan_skips_pointer_events_false` — `layout_hit_test_disabled_at()` returns `false` when the disabled node's subtree has `pointer_events: false`
- [x] `disabled_scan_respects_content_offset` — `layout_hit_test_disabled_at()` returns `true` for disabled node inside a scrolled container (parent has `content_offset: (0.0, -50.0)`), testing that the point is translated correctly before checking the child
- [x] `disabled_scan_respects_clip` — parent has `clip: true` with a small rect. Disabled child is inside the parent but the test point is outside the clip rect. Returns `false`.

**Build gate:**
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [x] `./build-all.sh` passes
- [x] Manual verification: hover cursor changes correctly on all interactive elements in settings dialog
