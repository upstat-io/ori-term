---
section: "08"
title: "Pure Logic Migration"
status: not-started
reviewed: false
goal: "Move pure UI logic currently stranded in oriterm/src/app/ into oriterm_ui where possible, consolidating reusable types and making UI utilities headless-testable. Logic with oriterm_mux dependencies stays in oriterm but is restructured as pure functions."
inspired_by:
  - "GPUI (all UI logic in gpui crate, app is thin consumer)"
  - "masonry (foundational library pattern — UI framework owns all reusable logic)"
depends_on: ["07"]
sections:
  - id: "08.1"
    title: "Cursor Utilities"
    status: not-started
  - id: "08.2"
    title: "Geometry & Hit Testing Consolidation"
    status: not-started
  - id: "08.3"
    title: "Drag State Machines"
    status: not-started
  - id: "08.4"
    title: "Context Menu State"
    status: not-started
  - id: "08.5"
    title: "Mark Mode Pure Logic"
    status: not-started
  - id: "08.6"
    title: "Audit Completeness — Modules NOT Migrated"
    status: not-started
  - id: "08.7"
    title: "Completion Checklist"
    status: not-started
---

# Section 08: Pure Logic Migration

**Status:** Not Started
**Goal:** Every piece of pure UI logic (no GPU, no platform, no terminal dependencies) that currently lives in `oriterm/src/app/` moves to `oriterm_ui`. After this section, `oriterm/src/app/` contains only: platform wiring, GPU rendering, terminal-specific operations, mux-coupled logic (types depending on `oriterm_mux`), and thin delegation to `oriterm_ui`.

**Context:** An architectural audit identified ~870 lines of pure UI logic scattered across `oriterm/src/app/` — cursor blink timers, hit testing geometry, drag state machines, context menu state, and mark mode key handling. These are testable without a GPU or terminal but currently can't be tested without the full app. Moving them to `oriterm_ui` makes them headless-testable and consolidates reusable geometry types (e.g., `ResizeEdge` in `floating_drag.rs` is useful for any resizable UI element).

**Reference implementations:**
- **GPUI**: All UI logic lives in the `gpui` crate. The app binary (`zed`) is a thin consumer.
- **masonry**: Foundational library owns all reusable widget/interaction logic. Higher-level frameworks build on it.

**Depends on:** Section 07 (WindowRoot must exist as the composition target; some utilities integrate with it).

---

## 08.1 Cursor Utilities

**File(s):**
- `oriterm/src/app/cursor_blink/mod.rs` (90 lines) → `oriterm_ui/src/animation/cursor_blink/mod.rs`
- `oriterm/src/app/cursor_blink/tests.rs` → `oriterm_ui/src/animation/cursor_blink/tests.rs`
- `oriterm/src/app/cursor_hide/mod.rs` (60 lines) → `oriterm_ui/src/interaction/cursor_hide/mod.rs`
- `oriterm/src/app/cursor_hide/tests.rs` → `oriterm_ui/src/interaction/cursor_hide/tests.rs`

Pure timing and decision logic. `CursorBlink` has zero external dependencies. `cursor_hide` depends on `winit::keyboard::{Key, NamedKey}` for key type matching, but `oriterm_ui` already depends on `winit`, so this is compatible.

- [ ] Move `CursorBlink` struct and all methods to `oriterm_ui/src/animation/cursor_blink/mod.rs`. This is a pure time-based state machine (visibility toggle on elapsed time, 90 lines). No API changes needed.

- [ ] Move cursor blink tests to `oriterm_ui/src/animation/cursor_blink/tests.rs`.

- [ ] Move `should_hide_cursor()` function and `HideContext` to `oriterm_ui/src/interaction/cursor_hide/mod.rs`. Pure decision function (60 lines) — depends on `winit::keyboard::{Key, NamedKey}` for key type matching (already available in `oriterm_ui`).

- [ ] Move cursor hide tests to `oriterm_ui/src/interaction/cursor_hide/tests.rs`.

- [ ] Update `oriterm/src/app/` to import from `oriterm_ui`:
  ```rust
  use oriterm_ui::animation::CursorBlink;
  use oriterm_ui::interaction::cursor_hide::should_hide_cursor;  // cursor_hide is a directory module
  ```

- [ ] Remove the old module directories from `oriterm/src/app/`.

- [ ] Add `pub mod cursor_blink;` to `oriterm_ui/src/animation/mod.rs` and re-export `CursorBlink`.

- [ ] Add `pub mod cursor_hide;` to `oriterm_ui/src/interaction/mod.rs`.

---

## 08.2 Geometry & Hit Testing Consolidation

**File(s):**
- `oriterm/src/app/floating_drag.rs` (lines 29-130, pure geometry: `ResizeEdge`, `HitZone`, `hit_test_zone`, `edge_cursor`, `compute_resize`) → `oriterm_ui/src/interaction/resize/mod.rs`

Extracts reusable resize geometry from `floating_drag.rs` into `oriterm_ui`. Note: `divider_drag.rs` uses `SplitDirection` (Vertical/Horizontal) for its axis, not `ResizeEdge` — there is no duplication to eliminate, but the extracted types are generally useful.

- [ ] Create `oriterm_ui/src/interaction/resize/mod.rs` with consolidated types:
  ```rust
  /// Edge or corner of a resizable region.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum ResizeEdge {
      Top, Bottom, Left, Right,
      TopLeft, TopRight, BottomLeft, BottomRight,
  }

  /// Maps a resize edge to the appropriate cursor icon.
  pub fn resize_cursor(edge: ResizeEdge) -> CursorIcon { ... }

  /// Hit-tests a point against a floating pane's zones (corners, edges, title bar, interior).
  ///
  /// Corresponds to `hit_test_zone()` in floating_drag.rs (currently a private function).
  pub fn hit_test_floating_zone(
      point: Point,
      rect: Rect,
      edge_threshold: f32,
      corner_size: f32,
      title_bar_height: f32,
  ) -> Option<HitZone> { ... }
  ```

- [ ] Extract `compute_resize()` from `floating_drag.rs` (lines 401-454) — pure geometry computation for resizing a rect by dragging an edge. Move to `resize/mod.rs`.

- [ ] Add `pub mod resize;` to `oriterm_ui/src/interaction/mod.rs` and re-export key types (`ResizeEdge`, `HitZone`, `resize_cursor`, `hit_test_floating_zone`, `compute_resize`).

- [ ] Update `oriterm/src/app/floating_drag.rs` to import from `oriterm_ui::interaction::resize`.

- [ ] Remove the `ResizeEdge` definition from `floating_drag.rs` (it is the only definition — no duplicate in `divider_drag.rs`).

- [ ] Add unit tests in `oriterm_ui/src/interaction/resize/tests.rs` for hit testing and cursor mapping.

---

## 08.3 Drag State Machines

**File(s):**
- `oriterm/src/app/floating_drag.rs` (state types) → `oriterm_ui/src/interaction/floating_drag.rs`
- `oriterm/src/app/divider_drag.rs` (state types) → `oriterm_ui/src/interaction/divider_drag.rs`

The drag state enums and their type definitions are pure state machines. The `impl App` methods that mutate session state stay in `oriterm`.

- [ ] Move `FloatingDragState` enum to `oriterm_ui/src/interaction/floating_drag.rs`. Note: both variants contain `pane_id: PaneId` from `oriterm_mux`, so the same dependency issue as `DividerDragState` applies. Options: generic ID parameter, abstract behind a trait, or keep in `oriterm`. `HitZone` was already moved to `resize.rs` in 08.2.

- [ ] Move `DividerDragState` struct to `oriterm_ui/src/interaction/divider_drag.rs`. It depends on `oriterm_mux::PaneId` and `crate::session::SplitDirection` — evaluate whether to add `oriterm_mux` as a dependency of `oriterm_ui` or abstract `PaneId` behind a generic ID type.

- [ ] Keep all `impl App { fn handle_*_drag() }` methods in `oriterm/src/app/` — they mutate session state.

- [ ] Update imports in `oriterm/src/app/floating_drag.rs` and `divider_drag.rs`.

---

## 08.4 Context Menu State

**File(s):**
- `oriterm/src/app/context_menu/mod.rs` (145 lines) — evaluate migration; recommendation is to keep in `oriterm`

Context menu state management. Defines `ContextMenuState` and menu builders.

- [ ] Evaluate `ContextMenuState` struct migration: `ContextMenuState` contains `Vec<Option<ContextAction>>`, and `ContextAction` stays in `oriterm` (it references `TabId`). **Options:**
  - (a) Make `ContextMenuState` generic: `ContextMenuState<A>` with `actions: Vec<Option<A>>`. Allows move to `oriterm_ui` but adds generics complexity.
  - (b) Keep `ContextMenuState` in `oriterm` alongside `ContextAction` — simpler, no migration needed.
  - **Recommendation:** Option (b) — the type is small (15 lines) and tightly coupled to `ContextAction`. Not worth the generic parameter complexity.

- [ ] Evaluate menu builders (`build_dropdown_menu`, `build_tab_context_menu`, `build_grid_context_menu`):
  - All three builders construct `ContextAction` values, and `ContextAction` stays in `oriterm` (see below). Therefore **no builders can move** unless `ContextAction` is also moved or split.
  - `build_tab_context_menu()` additionally takes `tab_id: crate::session::TabId` — doubly cannot move.
  - **Decision:** All builders stay in `oriterm` alongside `ContextAction`.

- [ ] Keep `ContextAction` enum in `oriterm` — it references `crate::session::TabId` (via `MoveToNewWindow(TabId)`) and terminal-specific actions (`Copy`, `Paste`, `SelectAll`).

- [ ] **Final decision for 08.4:** Everything stays in `oriterm`. `ContextMenuState`, `ContextAction`, and all builders are tightly coupled via `TabId` and terminal-specific action variants. No migration, no imports to update. This section is an evaluation-only checkpoint.

---

## 08.5 Mark Mode Pure Logic

**File(s):**
- `oriterm/src/app/mark_mode/mod.rs` (423 lines) → partial migration (see dependency evaluation below)
- `oriterm/src/app/mark_mode/motion.rs` (195 lines, pure functions) → `oriterm_ui/src/interaction/mark_mode/motion.rs`

The motion functions in `motion.rs` are already pure (no grid access, no locks). `handle_mark_mode_key()` reads a `SnapshotGrid` and returns mutations — it never touches app state directly.

- [ ] **Critical dependency evaluation**: `handle_mark_mode_key` depends on:
  - `SnapshotGrid` — defined in `oriterm/src/app/snapshot_grid/mod.rs`, wraps `oriterm_mux::PaneSnapshot`. Cannot move to `oriterm_ui` without adding `oriterm_mux` dependency.
  - `oriterm_mux::MarkCursor` — `MarkModeResult` contains `Option<MarkCursor>`.
  - `oriterm_core::{Selection, SelectionMode, SelectionPoint, Side}` — `oriterm_ui` already depends on `oriterm_core`, so these are fine.
  - `winit::event::KeyEvent` and `winit::keyboard::*` — `oriterm_ui` already depends on `winit`, so these are fine.

  **Options:**
  1. Abstract `SnapshotGrid` behind a trait (e.g., `trait GridQuery { fn cols(&self) -> usize; fn lines(&self) -> usize; ... }`) defined in `oriterm_ui`, implemented in `oriterm` for `SnapshotGrid`.
  2. Move `SnapshotGrid` to `oriterm_core` (it only depends on `oriterm_core` and `oriterm_mux` types).
  3. Add `oriterm_mux` as a dependency of `oriterm_ui` (undesirable — breaks the layering).
  4. Only move `motion.rs` (truly pure) and the type definitions; keep `handle_mark_mode_key` in `oriterm`.

  **Recommendation:** Option 4 for now — move `motion.rs` and types, keep the dispatch function in `oriterm`.

- [ ] Move pure type definitions to `oriterm_ui/src/interaction/mark_mode/mod.rs`:
  - `enum MarkAction` (pub(crate)) — no external deps, purely an enum of outcomes. Can move.
  - `enum SelectionUpdate` (pub(crate)) — contains `Selection` from `oriterm_core` (already a dep of `oriterm_ui`). Can move.
  - `struct MarkModeResult` (pub(crate)) — contains `Option<MarkCursor>` from `oriterm_mux`. **Cannot move** without adding `oriterm_mux` dependency. Keep in `oriterm` or replace `MarkCursor` with a generic cursor type.
  - `enum Motion` is private to `mark_mode/mod.rs` and tightly coupled to `handle_mark_mode_key` — keep with the dispatch function.
  - **Practical recommendation:** Move `MarkAction` and `SelectionUpdate` only. Keep `MarkModeResult` in `oriterm` since it binds the others to `MarkCursor`. The dispatch function in `oriterm` constructs `MarkModeResult` from the moved types.

- [ ] Move `motion.rs` (195 lines, truly pure functions: `move_left`, `move_right`, etc.). These have zero external dependencies beyond their own types (`AbsCursor`, `GridBounds`, `WordContext`).

- [ ] Keep `handle_mark_mode_key()` in `oriterm` until `SnapshotGrid` dependency is resolved (see options above).

- [ ] Keep `impl App` methods that apply `MarkModeResult` to pane state in `oriterm`.

- [ ] Move mark mode tests (1046 lines) — only tests for `motion.rs` functions can move immediately. Tests for `handle_mark_mode_key` depend on `SnapshotGrid` test fixtures and stay in `oriterm` until the dependency is resolved.

---

## 08.6 Audit Completeness — Modules NOT Migrated

The following `oriterm/src/app/` modules were examined but are **not** candidates for migration. This section documents why, so future audits do not re-evaluate them:

- [ ] **`tab_drag/mod.rs`** (430 lines): Contains `DragPhase` enum and `TabDragState` struct (pure state machine types), but `TabDragState` depends on `crate::session::TabId` and all `impl App` methods mutate session state. The pure types are small (~30 lines) and tightly coupled to the app-level tab reorder logic. **Decision:** Not worth migrating — the types are small and the coupling is deep.

- [ ] **`event_loop_helpers/mod.rs`** (277 lines): Contains `ControlFlowInput`, `ControlFlowDecision`, and `compute_control_flow()` — a pure function that decides `ControlFlow::Wait` vs `ControlFlow::WaitUntil`. This is testable without a GPU (it already has tests). However, it is event-loop-specific logic that has no reuse outside `oriterm`. **Decision:** Stays in `oriterm` — pure but application-specific, not framework-reusable.

- [ ] **`perf_stats.rs`** (225 lines): Pure counter/logging logic. No GPU, platform, or terminal dependency (except `crate::platform::memory::rss_bytes()` for profiling mode RSS). Not useful as a framework type — it's application telemetry. **Decision:** Stays in `oriterm`.

- [ ] **`cursor_hover.rs`**: `HoverResult` struct is pure, but the `impl App` methods depend on `PaneSnapshot`, `WindowRenderer`, and URL detection. **Decision:** Stays in `oriterm`.

- [ ] **`search_ui.rs`**: All `impl App` methods — depends on `MuxBackend`, pane state, clipboard. **Decision:** Stays in `oriterm`.

---

## 08.7 Completion Checklist

- [ ] `CursorBlink` lives in `oriterm_ui/src/animation/cursor_blink/`
- [ ] `should_hide_cursor` lives in `oriterm_ui/src/interaction/cursor_hide/mod.rs`
- [ ] `ResizeEdge` is extracted to `oriterm_ui/src/interaction/resize/mod.rs` (moved from `floating_drag.rs`)
- [ ] `hit_test_floating_zone` and `resize_cursor` are extracted to `oriterm_ui`
- [ ] `FloatingDragState` either lives in `oriterm_ui` (with generic ID param or after abstracting PaneId) or stays in `oriterm` (due to `oriterm_mux::PaneId` dependency)
- [ ] `ContextMenuState` stays in `oriterm` (contains `ContextAction` which references `TabId`) — unless made generic
- [ ] All context menu builders stay in `oriterm` (they construct `ContextAction` which references `TabId`)
- [ ] Mark mode `motion.rs` pure functions live in `oriterm_ui`
- [ ] `MarkAction` and `SelectionUpdate` live in `oriterm_ui`; `MarkModeResult` stays in `oriterm` (depends on `oriterm_mux::MarkCursor`)
- [ ] Modules that stay in `oriterm` have documented rationale in Section 08.6 (tab_drag, event_loop_helpers, perf_stats, cursor_hover, search_ui)
- [ ] No pure UI logic **without `oriterm_mux` dependencies** remains in `oriterm/src/app/` (only platform wiring, GPU, terminal ops, mux-coupled logic, and delegation)
- [ ] All moved modules have tests in their new location
- [ ] `oriterm/src/app/` imports from `oriterm_ui` for all moved types
- [ ] `timeout 150 cargo test -p oriterm_ui` passes
- [ ] `timeout 150 cargo test -p oriterm` passes
- [ ] `./clippy-all.sh` clean
- [ ] `./build-all.sh` clean

**Exit Criteria:** `oriterm/src/app/` contains only platform wiring, GPU rendering, terminal-specific operations, mux-coupled logic, and thin delegation. Pure UI utilities without `oriterm_mux` dependencies are in `oriterm_ui`, headless-testable, and covered by tests that run without a GPU. Logic with `oriterm_mux` dependencies (mark mode dispatch, drag state types using `PaneId`, context menu builders using `ContextAction`) stays in `oriterm` but is structured as pure functions with clear boundaries.
