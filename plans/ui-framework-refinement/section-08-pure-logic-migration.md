---
section: "08"
title: "Pure Logic Migration"
status: complete
reviewed: true
goal: "Move pure UI logic currently stranded in oriterm/src/app/ into oriterm_ui where possible, consolidating reusable types and making UI utilities headless-testable. Logic with oriterm_mux dependencies stays in oriterm but is restructured as pure functions."
inspired_by:
  - "GPUI (all UI logic in gpui crate, app is thin consumer)"
  - "masonry (foundational library pattern — UI framework owns all reusable logic)"
depends_on: ["07"]
sections:
  - id: "08.1"
    title: "Cursor Utilities"
    status: complete
  - id: "08.2"
    title: "Geometry & Hit Testing Consolidation"
    status: complete
  - id: "08.3"
    title: "Drag State Machines (Evaluation — No Migration)"
    status: complete
  - id: "08.4"
    title: "Context Menu State (Evaluation — No Migration)"
    status: complete
  - id: "08.5"
    title: "Mark Mode Pure Logic"
    status: complete
  - id: "08.6"
    title: "Audit Completeness — Modules NOT Migrated"
    status: complete
  - id: "08.7"
    title: "Completion Checklist"
    status: complete
---

# Section 08: Pure Logic Migration

**Status:** Not Started
**Goal:** Every piece of pure UI logic (no GPU, no platform, no terminal dependencies) that currently lives in `oriterm/src/app/` moves to `oriterm_ui`. After this section, `oriterm/src/app/` contains only: platform wiring, GPU rendering, terminal-specific operations, mux-coupled logic (types depending on `oriterm_mux`), and thin delegation to `oriterm_ui`.

**Context:** An architectural audit identified ~870 lines of pure UI logic scattered across `oriterm/src/app/` — cursor blink timers, hit testing geometry, drag state machines, context menu state, and mark mode key handling. These are testable without a GPU or terminal but currently can't be tested without the full app. Moving them to `oriterm_ui` makes them headless-testable and consolidates reusable geometry types (e.g., `ResizeEdge` in `floating_drag.rs` is useful for any resizable UI element).

**Reference implementations:**
- **GPUI**: All UI logic lives in the `gpui` crate. The app binary (`zed`) is a thin consumer.
- **masonry**: Foundational library owns all reusable widget/interaction logic. Higher-level frameworks build on it.

**Depends on:** Section 07 (soft ordering dependency — WindowRoot establishes the architectural boundary between `oriterm_ui` and `oriterm`; migrating pure logic after that boundary is formalized avoids rework).

---

## 08.1 Cursor Utilities

**File(s):**
- `oriterm/src/app/cursor_blink/mod.rs` (90 lines) → `oriterm_ui/src/animation/cursor_blink/mod.rs`
- `oriterm/src/app/cursor_blink/tests.rs` → `oriterm_ui/src/animation/cursor_blink/tests.rs`
- `oriterm/src/app/cursor_hide/mod.rs` (60 lines) → `oriterm_ui/src/interaction/cursor_hide/mod.rs`
- `oriterm/src/app/cursor_hide/tests.rs` → `oriterm_ui/src/interaction/cursor_hide/tests.rs`

Pure timing and decision logic. `CursorBlink` has zero external dependencies. `cursor_hide` depends on `winit::keyboard::{Key, NamedKey}` for key type matching, but `oriterm_ui` already depends on `winit`, so this is compatible.

**Step order: library crate (`oriterm_ui`) first, then binary crate (`oriterm`).**

- [x] Add `pub mod cursor_blink;` to `oriterm_ui/src/animation/mod.rs` and re-export `CursorBlink`.

- [x] Move `CursorBlink` struct and all methods to `oriterm_ui/src/animation/cursor_blink/mod.rs`. Pure time-based state machine (visibility toggle on elapsed time, 90 lines). **Visibility upgrade:** all `pub(crate)` items (`CursorBlink`, `new`, `is_visible`, `set_interval`, `reset`, `update`, `next_toggle`) become `pub` for cross-crate use. The `#[cfg(test)] const DEFAULT_BLINK_INTERVAL` also moves (tests reference it via `super::`). **Test convention:** file ends with `#[cfg(test)] mod tests;` (sibling tests.rs pattern, no inline test bodies).

- [x] Move cursor blink tests to `oriterm_ui/src/animation/cursor_blink/tests.rs`. Tests backdating `blink.epoch` directly access the private `epoch` field — this works because tests are a submodule with `super::` access. Verify this access pattern is preserved. **Test file convention:** no `mod tests { }` wrapper in the file; imports use `super::` for parent module items.

- [x] Add `pub mod cursor_hide;` to `oriterm_ui/src/interaction/mod.rs` and re-export `HideContext` and `should_hide_cursor`.

- [x] Move `should_hide_cursor()` function and `HideContext` to `oriterm_ui/src/interaction/cursor_hide/mod.rs`. Pure decision function (60 lines) — depends on `winit::keyboard::{Key, NamedKey}` for key type matching (already available in `oriterm_ui`). **Visibility upgrade:** `HideContext` and `should_hide_cursor` are `pub(crate)` — become `pub`. The private helper `is_modifier_only` stays private. **Test convention:** file ends with `#[cfg(test)] mod tests;`.

- [x] Move cursor hide tests to `oriterm_ui/src/interaction/cursor_hide/tests.rs`. **Test file convention:** no `mod tests { }` wrapper; imports use `super::` for parent module items.

- [x] Update `oriterm/src/app/` to import from `oriterm_ui`:
  ```rust
  use oriterm_ui::animation::CursorBlink;
  use oriterm_ui::interaction::cursor_hide::{HideContext, should_hide_cursor};
  ```
  Consumers: `oriterm/src/app/mod.rs` (line 62, 166: `CursorBlink` import + field), `oriterm/src/app/constructors.rs` (line 12, 129: `CursorBlink::new`), `oriterm/src/app/keyboard_input/mod.rs` (line 249, 256: `HideContext` + `should_hide_cursor`).

- [x] Remove the old module directories from `oriterm/src/app/` and their `mod` declarations from `oriterm/src/app/mod.rs` (`mod cursor_blink;` at line 13, `mod cursor_hide;` at line 14).

---

## 08.2 Geometry & Hit Testing Consolidation

**File(s):**
- `oriterm/src/app/floating_drag.rs` (lines 29-130, pure geometry: `ResizeEdge`, `HitZone`, `hit_test_zone`, `edge_cursor`; plus `compute_resize` at lines 401-454) → `oriterm_ui/src/interaction/resize/mod.rs`

Extracts reusable resize geometry from `floating_drag.rs` into `oriterm_ui`. Note: `divider_drag.rs` uses `SplitDirection` (Vertical/Horizontal) for its axis, not `ResizeEdge` — there is no duplication to eliminate, but the extracted types are generally useful.

**Rect type decision:** `floating_drag.rs` uses `session::Rect` (simple `{x, y, width, height}` struct), but `oriterm_ui` has `geometry::Rect<U>` (composed as `Point<U>` + `Size<U>`). The moved functions should use `oriterm_ui::geometry::Rect<Logical>` with the Point+Size API (`rect.origin.x`, `rect.size.width`). Callers in `floating_drag.rs` must convert between the two `Rect` types at the boundary (add `From` impl or manual conversion). Alternatively, the moved functions can accept raw `(x, y, width, height)` parameters to avoid coupling to either Rect type — evaluate during implementation.

**Step order: library crate (`oriterm_ui`) first, then binary crate (`oriterm`).**

- [x] Add `pub mod resize;` to `oriterm_ui/src/interaction/mod.rs` and re-export key types (`ResizeEdge`, `HitZone`, `resize_cursor`, `hit_test_floating_zone`, `compute_resize`).

- [x] Create `oriterm_ui/src/interaction/resize/mod.rs` with consolidated types:
  ```rust
  /// Edge or corner of a resizable region.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum ResizeEdge {
      Top, Bottom, Left, Right,
      TopLeft, TopRight, BottomLeft, BottomRight,
  }
  ```
  Note: the source `ResizeEdge` only derives `Debug, Clone, Copy` and is `pub(super)`. Adding `PartialEq, Eq` is intentional for test assertion support. Changing to `pub` is required for cross-crate use.

  ```rust
  /// Result of hit-testing a point against a floating pane's zones.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum HitZone {
      TitleBar,
      Edge(ResizeEdge),
      Interior,
  }
  ```
  Note: `HitZone` is currently a private enum in `floating_drag.rs`. It becomes `pub` in the new location.

  ```rust
  /// Maps a resize edge to the appropriate cursor icon.
  pub fn resize_cursor(edge: ResizeEdge) -> CursorIcon { ... }

  /// Hit-tests a point against a floating pane's zones (corners, edges, title bar, interior).
  ///
  /// Corresponds to `hit_test_zone()` in floating_drag.rs (currently a private function).
  /// Thresholds are passed as parameters (the original uses module constants).
  pub fn hit_test_floating_zone(
      px: f32,
      py: f32,
      rect_x: f32,
      rect_y: f32,
      rect_w: f32,
      rect_h: f32,
      edge_threshold: f32,
      corner_size: f32,
      title_bar_height: f32,
  ) -> Option<HitZone> { ... }
  ```
  **Warning:** `hit_test_floating_zone` has 9 parameters, which exceeds the 3-parameter threshold for config/options struct per coding standards. Consider grouping `(rect_x, rect_y, rect_w, rect_h)` into a tuple or lightweight struct, and `(edge_threshold, corner_size, title_bar_height)` into a `HitTestConfig` struct. Evaluate during implementation — if the call sites are clearer with raw parameters (only one or two callers), raw parameters are acceptable.

- [x] Extract `compute_resize()` from `floating_drag.rs` (lines 401-454) — pure geometry computation for resizing a rect by dragging an edge. Move to `resize/mod.rs`. **Note:** `compute_resize` depends on the `MIN_SIZE_PX` constant (100.0 pixels). Either: (a) move the constant to `resize/mod.rs` as a `pub const`, or (b) add `min_size` as a parameter for reusability. Option (b) is preferred — it makes the function useful for non-floating-pane resize scenarios. The `floating_drag.rs` call site passes `MIN_SIZE_PX` (which stays as a local constant there).

  **Note:** `compute_resize` returns `(Rect, bool)` where `Rect` is `session::Rect`. The moved version should use raw `(f32, f32, f32, f32)` or a new lightweight struct for the result rect (since `session::Rect` is not available in `oriterm_ui`). Alternatively, return a new `ResizeResult { x, y, width, height, needs_move }` struct for clarity. This avoids coupling the extracted function to any specific `Rect` type.

  **Test convention:** `resize/mod.rs` must end with `#[cfg(test)] mod tests;` (sibling tests.rs pattern).

  **Estimated file size:** ~135 lines (`ResizeEdge` ~10, `HitZone` ~5, `resize_cursor` ~10, `hit_test_floating_zone` ~45, `compute_resize` ~55, module doc + imports ~10). Well under 500-line limit.

- [x] Update `oriterm/src/app/floating_drag.rs` to import from `oriterm_ui::interaction::resize`. The original constants (`TITLE_BAR_HEIGHT`, `EDGE_THRESHOLD`, `CORNER_SIZE`, `MIN_SIZE_PX`) stay in `floating_drag.rs` as call-site values passed to the now-parameterized functions.

- [x] Remove the `ResizeEdge`, `HitZone`, `hit_test_zone`, `edge_cursor`, and `compute_resize` definitions from `floating_drag.rs`. After removal, `floating_drag.rs` drops from 454 lines to ~340 lines (state types + `impl App` methods + `DragInfo`). **Deviation:** Used `ResizeRect` struct + `HitTestConfig` struct instead of raw params to satisfy clippy's too-many-arguments lint. Also added a thin `hit_test_zone` wrapper in `floating_drag.rs` for `session::Rect` → `ResizeRect` conversion.

- [x] Add unit tests in `oriterm_ui/src/interaction/resize/tests.rs` (these are **new tests** — the original code in `floating_drag.rs` has no existing test file):
  - Hit testing: point inside each zone (corner, edge, title bar, interior), point outside rect returns `None`.
  - Cursor mapping: each `ResizeEdge` variant maps to the correct `CursorIcon`.
  - `compute_resize`: all 8 edge/corner variants, min size clamping, `needs_move` flag correctness for top/left/corner drags vs right/bottom drags.
  - **Test file convention:** no `mod tests { }` wrapper; imports use `super::` for parent module items.

---

## 08.3 Drag State Machines (Evaluation — No Migration)

**File(s):**
- `oriterm/src/app/floating_drag.rs` (state types) — evaluate migration
- `oriterm/src/app/divider_drag.rs` (state types) — evaluate migration

The drag state enums and their type definitions are pure state machines. The `impl App` methods that mutate session state stay in `oriterm`.

**Dependency analysis and recommendation:**

Both `FloatingDragState` and `DividerDragState` depend on `oriterm_mux::PaneId`:
- `FloatingDragState::Moving { pane_id: PaneId, ... }` and `FloatingDragState::Resizing { pane_id: PaneId, ... }`
- `DividerDragState { pane_before: PaneId, pane_after: PaneId, ... }`

Additionally, `DividerDragState` depends on `crate::session::SplitDirection` (defined in `oriterm/src/session/split_tree/mod.rs`), and `FloatingDragState::Resizing` depends on `session::Rect`.

Options evaluated:
1. **Generic ID parameter** (`FloatingDragState<Id>`) — adds generics complexity at every use site, not worth it for 2 types.
2. **Add `oriterm_mux` to `oriterm_ui`** — breaks the layering (`oriterm_ui` is a pure UI framework).
3. **Keep in `oriterm`** — simplest, no API changes, no dependency changes.

**Decision:** Option 3 — **keep both types in `oriterm`**. The `PaneId` and `SplitDirection` coupling is fundamental to these types' purpose. Moving them would require either breaking the crate layering or introducing generics that add complexity without benefit. The pure geometry extracted in 08.2 (`ResizeEdge`, `HitZone`, `compute_resize`) is the reusable portion; the state machines themselves are application-specific.

- [x] Confirm `FloatingDragState` stays in `oriterm/src/app/floating_drag.rs` — depends on `PaneId` and `session::Rect`. No migration.

- [x] Confirm `DividerDragState` stays in `oriterm/src/app/divider_drag.rs` — depends on `PaneId` and `SplitDirection`. No migration.

- [x] Confirm the private `DragInfo` enum in `floating_drag.rs` stays — it is a borrow-chain workaround tightly coupled to `FloatingDragState`.

- [x] Keep all `impl App { fn handle_*_drag() }` methods in `oriterm/src/app/` — they mutate session state.

- [x] **This section is an evaluation-only checkpoint** — no code changes, no imports to update. The rationale above serves as the documented decision for future audits.

---

## 08.4 Context Menu State (Evaluation — No Migration)

**File(s):**
- `oriterm/src/app/context_menu/mod.rs` (145 lines) — evaluate migration; recommendation is to keep in `oriterm`

Context menu state management. Defines `ContextMenuState` and menu builders.

- [x] Evaluate `ContextMenuState` struct migration: `ContextMenuState` contains `Vec<Option<ContextAction>>`, and `ContextAction` stays in `oriterm` (it references `TabId`). **Options:**
  - (a) Make `ContextMenuState` generic: `ContextMenuState<A>` with `actions: Vec<Option<A>>`. Allows move to `oriterm_ui` but adds generics complexity.
  - (b) Keep `ContextMenuState` in `oriterm` alongside `ContextAction` — simpler, no migration needed.
  - **Recommendation:** Option (b) — the type is small (15 lines) and tightly coupled to `ContextAction`. Not worth the generic parameter complexity.

- [x] Evaluate menu builders (`build_dropdown_menu`, `build_tab_context_menu`, `build_grid_context_menu`):
  - All three builders construct `ContextAction` values, and `ContextAction` stays in `oriterm` (see below). Therefore **no builders can move** unless `ContextAction` is also moved or split.
  - `build_tab_context_menu()` additionally takes `tab_id: crate::session::TabId` — doubly cannot move.
  - **Decision:** All builders stay in `oriterm` alongside `ContextAction`.

- [x] Keep `ContextAction` enum in `oriterm` — it references `crate::session::TabId` (via `MoveToNewWindow(TabId)`) and terminal-specific actions (`Copy`, `Paste`, `SelectAll`).

- [x] **Final decision for 08.4:** Everything stays in `oriterm`. `ContextMenuState`, `ContextAction`, and all builders are tightly coupled via `TabId` and terminal-specific action variants. No migration, no imports to update. This section is an evaluation-only checkpoint — the rationale above serves as the documented decision for future audits.

---

## 08.5 Mark Mode Pure Logic

**File(s):**
- `oriterm/src/app/mark_mode/mod.rs` (423 lines) → partial migration (see dependency evaluation below)
- `oriterm/src/app/mark_mode/motion.rs` (195 lines, pure functions) → `oriterm_ui/src/interaction/mark_mode/motion/mod.rs`

The motion functions in `motion.rs` are already pure (no grid access, no locks). `handle_mark_mode_key()` reads a `SnapshotGrid` and returns mutations — it never touches app state directly.

- [x] **Critical dependency evaluation**: `handle_mark_mode_key` depends on:
  - `SnapshotGrid` — defined in `oriterm/src/app/snapshot_grid/mod.rs`, wraps `oriterm_mux::PaneSnapshot`. Cannot move to `oriterm_ui` without adding `oriterm_mux` dependency.
  - `oriterm_mux::MarkCursor` — `MarkModeResult` contains `Option<MarkCursor>`.
  - `oriterm_core::{Selection, SelectionMode, SelectionPoint, Side}` — `oriterm_ui` already depends on `oriterm_core`, so these are fine.
  - `winit::event::KeyEvent` and `winit::keyboard::*` — `oriterm_ui` already depends on `winit`, so these are fine.

  **Options:**
  1. Abstract `SnapshotGrid` behind a trait (e.g., `trait GridQuery { fn cols(&self) -> usize; fn lines(&self) -> usize; ... }`) defined in `oriterm_ui`, implemented in `oriterm` for `SnapshotGrid`.
  2. ~~Move `SnapshotGrid` to `oriterm_core`~~ — **invalid**: `SnapshotGrid` imports `oriterm_mux::{PaneSnapshot, WireCell, WireCellFlags}`, and `oriterm_mux` already depends on `oriterm_core`, so this would create a circular dependency.
  3. Add `oriterm_mux` as a dependency of `oriterm_ui` (undesirable — breaks the layering).
  4. Only move `motion.rs` (truly pure) and the type definitions; keep `handle_mark_mode_key` in `oriterm`.

  **Recommendation:** Option 4 for now — move `motion.rs` and types, keep the dispatch function in `oriterm`.

**Step order: library crate (`oriterm_ui`) first, then binary crate (`oriterm`).**

- [x] Add `pub mod mark_mode;` to `oriterm_ui/src/interaction/mod.rs` and re-export `MarkAction`, `SelectionUpdate`.

- [x] Create `oriterm_ui/src/interaction/mark_mode/mod.rs` with moved type definitions:
  - `enum MarkAction` (currently `pub(crate)`) — no external deps, purely an enum of outcomes. **Visibility upgrade:** `pub(crate)` → `pub`.
  - `enum SelectionUpdate` (currently `pub(crate)`) — contains `Selection` from `oriterm_core` (already a dep of `oriterm_ui`). **Visibility upgrade:** `pub(crate)` → `pub`.
  - `pub mod motion;` declaration for the motion submodule.
  - **Test convention:** file must end with `#[cfg(test)] mod tests;` — though the types-only module may have no tests of its own (motion tests live in `motion/tests.rs`). Add `#[cfg(test)] mod tests;` only if there are tests to put there; otherwise omit.
  - **Estimated file size:** ~30 lines (module doc + imports + `MarkAction` + `SelectionUpdate` + mod declarations). Well under 500-line limit.

  Types that **cannot move**:
  - `struct MarkModeResult` (pub(crate)) — contains `Option<MarkCursor>` from `oriterm_mux`. **Cannot move** without adding `oriterm_mux` dependency. Keep in `oriterm`.
  - `enum Motion` is private to `mark_mode/mod.rs` and tightly coupled to `handle_mark_mode_key` — keep with the dispatch function.
  - **Practical recommendation:** Move `MarkAction` and `SelectionUpdate` only. Keep `MarkModeResult` in `oriterm` since it binds the others to `MarkCursor`. The dispatch function in `oriterm` constructs `MarkModeResult` from the moved types.

- [x] Move `motion.rs` (195 lines of pure functions: `move_left`, `move_right`, etc.) to `oriterm_ui/src/interaction/mark_mode/motion/mod.rs`. These have zero external dependencies beyond their own types (`AbsCursor`, `GridBounds`, `WordContext`). **Visibility upgrade:** all `pub(crate)` items → `pub`. **Directory module conversion:** the original flat file `motion.rs` becomes a directory module (`motion/mod.rs` + `motion/tests.rs`) so tests can live in a sibling file. Add `#[cfg(test)] mod tests;` at the bottom of `motion/mod.rs`.

- [x] Update `oriterm/src/app/mark_mode/mod.rs` to import moved types from `oriterm_ui`:
  ```rust
  use oriterm_ui::interaction::mark_mode::{MarkAction, SelectionUpdate};
  use oriterm_ui::interaction::mark_mode::motion::{AbsCursor, GridBounds, WordContext};
  ```
  Remove the local definitions of `MarkAction` and `SelectionUpdate`. The `pub(crate) mod motion;` declaration changes to an external import. **Note:** `mark_mode/mod.rs` currently has `pub(crate) mod motion;` at line 10 — this must be removed and replaced with the `use` import above.

- [x] Keep the following functions in `oriterm/src/app/mark_mode/mod.rs` — all depend on `SnapshotGrid` or `MarkCursor`:
  - `handle_mark_mode_key()` — takes `&SnapshotGrid`, returns `MarkModeResult` (contains `MarkCursor`).
  - `apply_motion()` — calls `SnapshotGrid::stable_to_absolute`, `absolute_to_stable`.
  - `resolve_motion()` — pure but private helper tightly coupled to `handle_mark_mode_key`.
  - `extract_word_context()` — calls `SnapshotGrid::word_boundaries`, `first_visible_absolute`.
  - `extend_or_create_selection()` — takes `MarkCursor` parameters.
  - `select_all()` — calls `SnapshotGrid::absolute_to_stable`, `total_rows`, `cols`.
  - `ensure_visible()` — calls `SnapshotGrid::stable_to_absolute`, `first_visible_absolute`, `lines`.

  **Post-migration file size:** `mark_mode/mod.rs` currently has 423 lines. After removing `MarkAction` (~15 lines), `SelectionUpdate` (~5 lines), and the `pub(crate) mod motion;` declaration, it drops to ~400 lines. It keeps `MarkModeResult`, `Motion`, and all 7 functions listed above. Under 500-line limit.

- [x] Keep `impl App` methods that apply `MarkModeResult` to pane state in `oriterm`.

- [x] Move mark mode tests — split the 1046-line `tests.rs` by dependency:

  **Warning: This is the riskiest step in Section 08.** The 1046-line test file must be surgically split between two crates. Before splitting, read the entire test file to identify which test functions use `PaneSnapshot`/`SnapshotGrid`/`MarkCursor` imports (must stay) vs. which only use `AbsCursor`/`GridBounds`/`WordContext` (can move). A missed dependency will cause compilation errors in the wrong crate. Run `cargo test -p oriterm_ui` and `cargo test -p oriterm` after splitting to verify.

  - **Can move** (motion tests): Tests for `move_left`, `move_right`, `move_up`, `move_down`, `page_up`, `page_down`, `line_start`, `line_end`, `buffer_start`, `buffer_end`, `word_left`, `word_right`. These only use `AbsCursor`, `GridBounds`, `WordContext` — all pure types moving to `oriterm_ui`. Move to `oriterm_ui/src/interaction/mark_mode/motion/tests.rs`.
  - **Must stay** (dispatch + grid tests): Tests for `handle_mark_mode_key`, `select_all`, `extend_or_create_selection`, `ensure_visible`, `extract_word_context`. These construct `PaneSnapshot` fixtures, wrap them in `SnapshotGrid`, and import `MarkCursor` from `oriterm_mux`. They stay in `oriterm/src/app/mark_mode/tests.rs`.
  - After splitting, `oriterm/src/app/mark_mode/tests.rs` imports moved types from `oriterm_ui::interaction::mark_mode` (for `MarkAction`, `SelectionUpdate`) and from `oriterm_ui::interaction::mark_mode::motion` (for `AbsCursor`, `GridBounds`).
  - **Test file convention:** both test files use `super::` imports for their parent module, no `mod tests { }` wrapper.

---

## 08.6 Audit Completeness — Modules NOT Migrated

The following `oriterm/src/app/` modules were examined but are **not** candidates for migration. This section documents why, so future audits do not re-evaluate them:

- [x] **`tab_drag/mod.rs`** (430 lines): Contains `DragPhase` enum, `TabDragState` struct, `TornOffPending` struct, and three pure helper functions (`compute_drag_visual_x`, `compute_insertion_index`, `exceeds_tear_off`). Pure content totals ~95 lines but `TabDragState` and `TornOffPending` depend on `crate::session::TabId` (and `TornOffPending` on `winit::window::WindowId`), the pure functions use tab bar constants from `oriterm_ui`, and all `impl App` methods mutate session state. **Decision:** Not worth migrating — the types have `TabId`/`WindowId` coupling and the pure functions are already using `oriterm_ui` constants via imports.

- [x] **`event_loop_helpers/mod.rs`** (272 lines): Contains `ControlFlowInput`, `ControlFlowDecision`, and `compute_control_flow()` — a pure function that decides `ControlFlow::Wait` vs `ControlFlow::WaitUntil`. This is testable without a GPU (it already has tests). However, it is event-loop-specific logic that has no reuse outside `oriterm`. **Decision:** Stays in `oriterm` — pure but application-specific, not framework-reusable.

- [x] **`perf_stats.rs`** (225 lines): Pure counter/logging logic. No GPU, platform, or terminal dependency (except `crate::platform::memory::rss_bytes()` for profiling mode RSS). Not useful as a framework type — it's application telemetry. **Decision:** Stays in `oriterm`.

- [x] **`cursor_hover.rs`** (204 lines): `HoverResult` struct is pure, but the `impl App` methods depend on `WindowRenderer` (for cell metrics), `mouse_selection::GridCtx`, `url_detect`, and pane snapshots (via mux). **Decision:** Stays in `oriterm`.

- [x] **`search_ui.rs`** (183 lines): All `impl App` methods — depends on the mux instance (for search lifecycle, query management, match navigation) and pane snapshots (for reading current query). **Decision:** Stays in `oriterm`.

The following modules were also examined and are clearly not migration candidates (platform wiring, GPU rendering, `impl App` methods with mux/session dependencies, or already in `oriterm_ui`). Listed for completeness so future audits can skip them:

- [x] **`chrome/`**: Platform-specific window chrome (DWM, title bar). Platform wiring.
- [x] **`clipboard_ops/`**: `impl App` clipboard methods using platform clipboard API.
- [x] **`config_reload/`**: `impl App` methods reloading config and applying to mux/GPU.
- [x] **`constructors.rs`**: `App::new()` — platform + GPU + mux initialization.
- [x] **`dialog_context/`**: `DialogWindowContext` — wraps `WindowRoot` + `WindowRenderer` (GPU).
- [x] **`dialog_management.rs`**: `impl App` dialog lifecycle methods.
- [x] **`dialog_rendering.rs`**: `impl App` dialog GPU rendering.
- [x] **`event_loop.rs`**: Winit event loop handler — platform wiring.
- [x] **`init/`**: App initialization — platform + GPU + mux setup.
- [x] **`keyboard_input/`**: `impl App` keyboard dispatch — mux-coupled.
- [x] **`mouse_input.rs`**: `impl App` mouse dispatch — mux-coupled.
- [x] **`mouse_report/`**: Terminal mouse reporting protocol — mux-coupled.
- [x] **`mouse_selection/`**: `impl App` selection methods — mux/grid-coupled.
- [x] **`mux_pump/`**: Mux event pump — mux-coupled by definition.
- [x] **`pane_accessors.rs`**: `impl App` pane access helpers — session/mux-coupled.
- [x] **`pane_ops/`**: `impl App` pane lifecycle — session/mux-coupled.
- [x] **`redraw/`**: `impl App` redraw orchestration — GPU-coupled.
- [x] **`render_dispatch.rs`**: `impl App` render dispatch — GPU-coupled.
- [x] **`settings_overlay/`**: Settings dialog UI — `impl App` methods.
- [x] **`snapshot_grid/`**: `SnapshotGrid` wrapping `PaneSnapshot` — mux-coupled.
- [x] **`tab_bar_input.rs`**: `impl App` tab bar click handling.
- [x] **`tab_management/`**: `impl App` tab lifecycle — session/mux-coupled.
- [x] **`widget_pipeline/`**: Widget pipeline functions moved to `oriterm_ui::pipeline` in Section 01.2a. The directory still exists as a thin re-export shim (27 lines of `pub(crate) use` + one wrapper function) plus `tests.rs` (563 lines). Not a migration candidate — it is already migrated.
- [x] **`window_context.rs`**: `WindowContext` wrapping `WindowRoot` + renderer — GPU-coupled.
- [x] **`window_management.rs`**: `impl App` window lifecycle — platform/session-coupled.
- [x] **`mod.rs`**: `App` struct definition, field declarations, `mod` declarations — the module root. Not a migration candidate.
- [x] **`tests.rs`**: App-level integration tests (368 lines) — tests the full `App` pipeline. Not a migration candidate.

---

## 08.7 Completion Checklist

**Migrated to `oriterm_ui`:**
- [x] `CursorBlink` lives in `oriterm_ui/src/animation/cursor_blink/` with `pub` visibility
- [x] `should_hide_cursor` and `HideContext` live in `oriterm_ui/src/interaction/cursor_hide/` with `pub` visibility
- [x] `ResizeEdge` and `HitZone` are extracted to `oriterm_ui/src/interaction/resize/mod.rs` with `pub` visibility
- [x] `hit_test_floating_zone`, `resize_cursor`, and `compute_resize` are extracted to `oriterm_ui` with parameterized thresholds (no hardcoded constants)
- [x] Mark mode motion pure functions (`move_left`, `move_right`, etc.) and types (`AbsCursor`, `GridBounds`, `WordContext`) live in `oriterm_ui/src/interaction/mark_mode/motion/mod.rs` with `pub` visibility
- [x] `MarkAction` and `SelectionUpdate` live in `oriterm_ui/src/interaction/mark_mode/mod.rs` with `pub` visibility

**Confirmed staying in `oriterm` (with documented rationale):**
- [x] `FloatingDragState` stays in `oriterm` (depends on `PaneId` + `session::Rect`)
- [x] `DividerDragState` stays in `oriterm` (depends on `PaneId` + `SplitDirection`)
- [x] `ContextMenuState`, `ContextAction`, and all menu builders stay in `oriterm` (depends on `TabId`)
- [x] `MarkModeResult` stays in `oriterm` (depends on `MarkCursor` from `oriterm_mux`)
- [x] `handle_mark_mode_key` and all `SnapshotGrid`-dependent functions stay in `oriterm`
- [x] All `oriterm/src/app/` modules have documented migration rationale in Section 08.6

**Integration:**
- [x] `oriterm/src/app/` imports from `oriterm_ui` for all moved types (no stale local definitions)
- [x] Old module directories/files removed from `oriterm/src/app/` after migration
- [x] `mod` declarations in `oriterm/src/app/mod.rs` removed for deleted modules
- [x] All moved modules have tests in their new `oriterm_ui` location
- [x] Mark mode tests correctly split: motion tests in `oriterm_ui`, dispatch tests in `oriterm`
- [x] No pure UI logic **without `oriterm_mux` dependencies** remains in `oriterm/src/app/`
- [x] All new modules with tests follow sibling `tests.rs` convention (directory module, `#[cfg(test)] mod tests;` at bottom, no inline test bodies, no `mod tests { }` wrapper in test file)
- [x] `motion.rs` converted from flat file to directory module (`motion/mod.rs` + `motion/tests.rs`) per test-organization rules

**Verification:**
- [x] `timeout 150 ./test-all.sh` passes
- [x] `./fmt-all.sh` clean
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` clean

**Exit Criteria:** `oriterm/src/app/` contains only platform wiring, GPU rendering, terminal-specific operations, mux-coupled logic, and thin delegation. Pure UI utilities without `oriterm_mux` dependencies are in `oriterm_ui`, headless-testable, and covered by tests that run without a GPU. Logic with `oriterm_mux` dependencies (mark mode dispatch, drag state types using `PaneId`, context menu builders using `ContextAction`) stays in `oriterm` but is structured as pure functions with clear boundaries.
