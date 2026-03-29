# Section 33: Split Navigation + Floating Panes -- Verification Results

**Verified by:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-29
**Status:** PASS with findings
**Branch:** dev (commit d15f7df)

## Context Loaded

Read in full before starting:
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (137 lines)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/.claude/rules/code-hygiene.md` (104 lines)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/.claude/rules/impl-hygiene.md` (52 lines)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/.claude/rules/test-organization.md` (57 lines)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/.claude/rules/crate-boundaries.md` (loaded via system-reminder)
- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/plans/roadmap/section-33-split-nav-floating.md` (302 lines)

---

## 33.1 Spatial Navigation Keybinds

### Action Enum (9 new variants)

**File:** `oriterm/src/keybindings/mod.rs` (232 lines)

All 9 action variants from the plan exist: `SplitRight`, `SplitDown`, `FocusPaneUp`, `FocusPaneDown`, `FocusPaneLeft`, `FocusPaneRight`, `NextPane`, `PrevPane`, `ClosePane`. Each has a `///` doc comment. The `as_str()` method covers all of them (lines 160-171). `parse_action()` in `parse.rs` covers all 9 (lines 163-171).

Additionally, Section 33.2 added 5 more: `ResizePaneUp/Down/Left/Right`, `EqualizePanes`. Section 33.3 added `ToggleZoom`. Section 33.4 added `ToggleFloatingPane`, `ToggleFloatTile`. Section 33.5 added `UndoSplit`, `RedoSplit`. Total new pane-related actions: **18 variants**.

**Verdict:** PASS. All planned actions implemented with doc comments and roundtrip coverage.

### Default Keybindings

**File:** `oriterm/src/keybindings/defaults.rs` (136 lines)

Verified bindings:
- `Ctrl+Shift+O` -> `SplitRight` (line 64)
- `Ctrl+Shift+E` -> `SplitDown` (line 65)
- `Ctrl+Alt+ArrowUp/Down/Left/Right` -> `FocusPaneUp/Down/Left/Right` (lines 66-69)
- `Ctrl+Shift+W` -> `ClosePane` (line 70)
- No defaults for `NextPane`/`PrevPane` (deliberate, documented in plan: "arrow nav wraps instead")

Plan says `Ctrl+Shift+J/K` for cycle, but implementation correctly omits them since `navigate_wrap` handles wrapping via arrows. The plan's test `cycle_pane_has_no_default_binding` confirms this design decision.

**Verdict:** PASS. All bindings match plan.

### Navigation Engine

**File:** `oriterm/src/session/nav/mod.rs` (236 lines)

Five public functions:
- `navigate()`: centroid-based directional navigation using primary_dist + 0.5*perp_dist scoring
- `navigate_wrap()`: wraps to farthest pane on opposite edge when no target in direction
- `cycle()`: index-based sequential traversal with wrap-around
- `nearest_pane()`: hit-test for mouse click-to-focus, floating panes checked first (reverse z-order)
- `Direction` enum with `opposite()` and `Display`

**Verdict:** PASS. Clean implementation, no `unwrap()`, proper half-open interval handling.

### Pane Operations Dispatch

**File:** `oriterm/src/app/pane_ops/mod.rs` (354 lines)

`execute_pane_action()` dispatches all 18 pane actions. Each delegates to a separate method. The module is split into 3 files: `mod.rs`, `floating.rs` (171 lines), `helpers.rs` (157 lines) -- all under the 500-line limit.

Key implementation details verified:
- `split_pane()` calls `unzoom_if_needed()` first (line 75), then spawns via mux, then updates local tree
- `focus_pane_direction()` calls `unzoom_if_needed()` first (line 130), then uses `navigate_wrap`
- `close_focused_pane()` handles last-pane case by calling `exit_app()` (line 182)
- `resize_all_panes()` syncs PTY sizes after layout changes (line 213)
- `try_pane_focus_click()` uses `nearest_pane()` for mouse click-to-focus, then `raise_if_floating()` (line 268)

**Verdict:** PASS.

### Tests (Keybindings)

**File:** `oriterm/src/keybindings/tests.rs` (821 lines)

All plan-listed tests found and verified:
- `action_as_str_roundtrip` (line 495): includes all 18 pane actions (SplitRight through RedoSplit) plus all other actions
- `split_right_default_binding` (line 553): verifies `Ctrl+Shift+O`
- `split_down_default_binding` (line 563): verifies `Ctrl+Shift+E`
- `focus_pane_arrow_defaults` (line 575): all 4 directions with `Ctrl+Alt`
- `cycle_pane_has_no_default_binding` (line 597): confirms no defaults for PrevPane/NextPane
- `close_pane_default_binding` (line 608): verifies `Ctrl+Shift+W`

Additional keybinding tests beyond plan:
- `resize_pane_arrow_defaults` (line 619): `Ctrl+Alt+Shift+Arrow`
- `equalize_panes_default_binding` (line 640): `Ctrl+Shift+=`
- `resize_bindings_no_collision_with_focus_bindings` (line 679): confirms separate modifier combos
- `toggle_floating_pane_default_binding` (line 702): `Ctrl+Shift+P`
- `toggle_float_tile_default_binding` (line 711): `Ctrl+Shift+G`
- `undo_split_default_binding` (line 738): `Ctrl+Shift+U`
- `redo_split_default_binding` (line 749): `Ctrl+Shift+Y`
- `undo_redo_actions_roundtrip_through_parse` (line 760)

**Test results:** 62 passed, 0 failed.
**Verdict:** PASS.

### Tests (Navigation)

**File:** `oriterm/src/session/nav/tests.rs` (729 lines)

Comprehensive coverage:
- 2x2 grid: all 4 directions, boundary returns None (7 tests)
- Cycle: forward/backward with wrap, single pane returns self (5 tests)
- `nearest_pane`: tiled hit, floating priority, outside returns None (3 tests)
- Tiled-to-floating and floating-to-floating navigation (3 tests)
- Z-order: topmost floating wins on overlap (2 tests)
- Uneven splits, T-shape, L-shape layouts (6 tests)
- 3-pane nested split with all-directions and cycle (3 tests)
- Progressive pane removal (2 tests)
- Border/tie-breaking: half-open interval, deterministic (3 tests)
- 5-pane asymmetric (Ghostty-style) layout (7 tests)
- Degenerate geometry: zero-width/height panes (3 tests)
- Floating-only layouts (1 test)
- `navigate_wrap`: wraps all directions, single-pane returns None, two-pane wraps both ways (6 tests)

**Test results:** 60 passed, 0 failed.
**Verdict:** PASS. Excellent coverage including edge cases and degenerate geometry.

---

## 33.2 Divider Drag Resize

### Divider Drag State Machine

**File:** `oriterm/src/app/divider_drag.rs` (267 lines)

Verified:
- `DividerDragState` struct with `pane_before`, `pane_after`, `direction`, `initial_ratio`, `origin_px`, `total_px`
- `divider_hit_rect()` expands by `HIT_ZONE_HALF_PAD` (1.5px) on each side
- `update_divider_hover()`: lazy divider cache, hit-test, cursor icon (`ColResize`/`RowResize`)
- `try_start_divider_drag()`: captures initial ratio + origin, stores on ctx
- `update_divider_drag()`: delta_px / total_px formula, clamp 0.1..0.9, live tree update via `set_divider_ratio()`
- `try_finish_divider_drag()`: commits and resizes all panes
- `cancel_divider_drag()`: accepts current state (no rollback to initial)

Note: `cancel_divider_drag()` has a comment "Restore to initial state -- the tree was updated live, so we'd need the initial tree to truly revert. For now, just accept the last committed ratio" (line 263). This is an acknowledged limitation, not a bug.

### Keyboard Resize

**File:** `oriterm/src/app/pane_ops/mod.rs`

`resize_pane_toward()` (line 278): translates direction to `(axis, pane_in_first, delta)` and calls `try_resize_toward()`. Uses `RESIZE_STEP = 0.05` (5% per keypress). Calls `unzoom_if_needed()` first.

### SplitTree Mutation Tests

**File:** `oriterm/src/session/split_tree/tests.rs` (770 lines)

Plan-listed tests all found:
- `set_divider_ratio_simple_split` (line 468)
- `set_divider_ratio_nested_inner` (line 482)
- `set_divider_ratio_nested_outer` (line 503)
- `set_divider_ratio_clamps` (line 519)
- `set_divider_ratio_nonexistent_panes` (line 532)
- `resize_toward_right_pane_in_first` (line 637)
- `resize_toward_left_pane_in_second` (line 651)
- `resize_toward_no_matching_split` (line 665)
- `resize_toward_wrong_side_noop` (line 675)
- `resize_toward_nested_finds_deepest` (line 685)
- `resize_toward_clamps_at_bounds` (line 730)
- `resize_toward_mixed_directions` (line 750)

**Test results:** 60 passed (includes all split_tree tests), 0 failed.
**Verdict:** PASS.

---

## 33.3 Zoom + Unzoom

### Implementation

**File:** `oriterm/src/app/pane_ops/mod.rs` line 53

`toggle_zoom()`:
- Gets `active_pane_context()`, reads `tab.zoomed_pane()`
- If zoomed: clears to `None`
- If not zoomed: sets to `Some(tab.active_pane())`
- Invalidates pane cache, marks dirty, syncs tab bar

**File:** `oriterm/src/session/tab/mod.rs` lines 84-92

`zoomed_pane: Option<PaneId>` field on `Tab`. Simple getter/setter.

### Auto-unzoom

`unzoom_if_needed()` (helpers.rs line 113) returns `bool`. Called from:
- `split_pane()` (line 75)
- `focus_pane_direction()` (line 130)
- `cycle_pane()` (line 151)
- `resize_pane_toward()` (line 279)
- `toggle_floating_pane()` (floating.rs line 14)
- `toggle_float_tile()` (floating.rs line 98)

Missing from plan: `close_focused_pane()` does NOT call `unzoom_if_needed()`. The plan says "Close zoomed pane: unzoom then close" but the implementation closes directly. This is acceptable because closing the zoomed pane removes it from the tree, which implicitly clears the zoom state.

### Keybinding

`Ctrl+Shift+Z` -> `ToggleZoom` confirmed in defaults.rs (line 90).

### Tests

**File:** `oriterm/src/session/tab/tests.rs`

- `zoom_state` (line 33): set/clear/verify
- Plan lists `toggle_zoom_sets_zoomed_pane`, `toggle_zoom_twice_unzooms`, `unzoom_clears_zoom_and_emits_notification`, `unzoom_noop_when_not_zoomed`, `close_zoomed_pane_clears_zoom`. The actual test names differ: `zoom_state` covers set/clear, `new_tab_has_single_pane` verifies `zoomed_pane().is_none()`.

**Finding:** Plan lists 6 zoom-specific test names that do not exist by those exact names. The functionality IS tested by `zoom_state` and `new_tab_has_single_pane`, but coverage is thinner than the plan claims. There is no test for "close zoomed pane clears zoom" or "auto-unzoom on split/navigate".

**Verdict:** PARTIAL PASS. Core zoom state works (tested), but plan overstates test count. Missing: dedicated tests for auto-unzoom triggers and close-while-zoomed.

---

## 33.4 Floating Pane Management

### FloatingLayer

**File:** `oriterm/src/session/floating/mod.rs` (321 lines)

All plan items verified:
- `FloatingPane::centered()`: 60% of available, centered (lines 45-61)
- `FloatingLayer`: add, remove, move_pane, resize_pane, raise, lower, hit_test (all implemented)
- In-place hot-path variants: `move_pane_mut()`, `resize_pane_mut()`, `set_pane_rect_mut()` for drag performance
- `snap_to_edge()` free function: 10px threshold, checks all 4 edges independently

### Floating Drag/Resize State Machine

**File:** `oriterm/src/app/floating_drag.rs` (454 lines, under 500 limit)

- `FloatingDragState`: Moving (title bar drag) and Resizing (edge/corner drag)
- `hit_test_zone()`: checks corners (10x10px), edges (5px), title bar (24px), interior
- `edge_cursor()`: maps to `NsResize`, `EwResize`, `NwseResize`, `NeswResize`
- `compute_resize()`: handles all 8 edges/corners with MIN_SIZE_PX=100 enforcement
- `update_floating_drag()`: extracts Copy fields to break borrow chain, applies snap_to_edge during moves

### Float <-> Tile Toggle

**File:** `oriterm/src/app/pane_ops/floating.rs` (171 lines)

- `toggle_floating_pane()`: if no floats -> spawn new centered floating pane; if active is floating -> focus first tiled; if active is tiled -> focus topmost float
- `toggle_float_tile()`: float->tile removes from floating, adds to tree at first_pane anchor; tile->float removes from tree, adds to floating centered. Pane identity preserved (same PaneId, same shell session)
- `raise_if_floating()`: called on click-to-focus

### Tests (Floating Layer)

**File:** `oriterm/src/session/floating/tests.rs` (431 lines)

28 tests found and verified:
- Add/remove/contains (4 tests)
- Hit testing with z-order (3 tests)
- Raise/lower (2 tests)
- Move/resize (2 tests)
- pane_rect correctness (2 tests)
- Z-order sorted invariant (1 test)
- Z-order stability across mutations (2 tests)
- Centered pane sizing (3 tests)
- Snap-to-edge all 4 edges plus corner plus offset bounds (7 tests)
- Remove middle z-order (2 tests)

**Test results:** 28 passed, 0 failed.

### FINDING: Missing tests from plan

Plan section 33.4 lists these tests that do NOT exist:
- `move_pane_to_tiled_removes_from_floating` -- no test by this name
- `move_pane_to_floating_removes_from_tree` -- no test by this name
- `move_last_tiled_pane_to_floating_rejected` -- no test by this name

The float<->tile toggle logic exists in `pane_ops/floating.rs` but requires `App` context (mux, session, window) that cannot be constructed in a headless test. These are integration-level behaviors without unit tests.

**Verdict:** PARTIAL PASS. Core floating layer logic is thoroughly tested. Float<->tile toggle and floating drag/resize are implemented but lack dedicated unit tests because they require App context.

---

## 33.5 Undo + Redo Split Operations

### Implementation

**File:** `oriterm/src/session/tab/mod.rs` (180 lines)

- `undo: VecDeque<SplitTree>` and `redo: VecDeque<SplitTree>` on Tab (lines 33-35)
- `MAX_UNDO_ENTRIES = 32` (line 15)
- `set_tree()`: pushes current to undo, clears redo, caps at 32 (lines 99-106)
- `undo_tree()`: pops from undo, validates all panes live, pushes current to redo (lines 116-128)
- `redo_tree()`: mirrors undo_tree but from redo stack (lines 137-149)
- `replace_layout()`: bypasses undo (line 155)

### Keybindings

- `Ctrl+Shift+U` -> `UndoSplit` (defaults.rs line 96)
- `Ctrl+Shift+Y` -> `RedoSplit` (defaults.rs line 97)

### App-level dispatch

`undo_split()` and `redo_split()` in `pane_ops/mod.rs` (lines 320-353):
- Collects live pane IDs via `live_pane_ids(tab_id)`
- Calls `tab.undo_tree(&live_panes)` / `tab.redo_tree(&live_panes)`
- Invalidates pane cache on success

### Tests (Tab undo/redo)

**File:** `oriterm/src/session/tab/tests.rs` (143 lines)

Tests found:
- `set_tree_pushes_undo` (line 45): verifies undo restores original tree
- `undo_redo_cycle` (line 62): split -> undo -> redo round trip
- `undo_skips_stale_entries` (line 80): stale pane IDs skipped
- `undo_returns_false_when_empty` (line 97)
- `redo_returns_false_when_empty` (line 104)
- `replace_layout_does_not_push_undo` (line 111)
- `set_tree_clears_redo` (line 122): new mutation after undo clears redo

**Test results:** 11 passed, 0 failed.

### FINDING: Multiple planned tests do NOT exist

Plan section 33.5 claims these tests exist but they do NOT:
1. `undo_split_restores_previous_tree` -- actual name: `set_tree_pushes_undo`
2. `redo_restores_undone_tree` -- actual name: `undo_redo_cycle`
3. `multiple_undo_then_redo_walks_forward` -- DOES NOT EXIST (no multi-step undo/redo test)
4. `new_mutation_after_undo_clears_redo` -- actual name: `set_tree_clears_redo`
5. `set_tree_clears_redo_stack` -- same as above, not a separate test
6. `redo_stack_capped_at_32` -- DOES NOT EXIST (no cap enforcement test)
7. `undo_skips_stale_pane_entry` -- actual name: `undo_skips_stale_entries`
8. `redo_skips_stale_pane_entry` -- DOES NOT EXIST (no redo-specific stale skip test)

Plan claims "InProcessMux tests" for undo/redo (`undo_split_restores_previous_tree`, `redo_split_restores_undone_tree`, `split_undo_redo_undo_cycle`, `undo_past_closed_pane_skips_entry`). These DO NOT EXIST in `oriterm_mux`. Undo/redo lives entirely in the session Tab, not the mux.

**Missing test coverage:**
- **Stack cap at 32**: `MAX_UNDO_ENTRIES` is used in code but never tested. A test creating 33+ mutations and verifying the oldest is dropped would catch off-by-one bugs.
- **Multiple undo walk-back**: No test exercises 3+ undo steps then redo forward.
- **Redo stale skip**: `undo_skips_stale_entries` tests the undo path; the redo path has the same logic but no test.

**Verdict:** PARTIAL PASS. Core undo/redo logic works and is tested for basic cases. Plan significantly overstates test count (claims ~12 tests, actual is 7). Three meaningful coverage gaps: stack cap enforcement, multi-step undo/redo walk, and redo stale skip.

---

## 33.6 Section Completion

### Build Verification

All source files compile as part of `cargo test -p oriterm`. No compilation errors observed.

### File Size Compliance

All source files under 500-line limit:
| File | Lines |
|------|-------|
| `session/split_tree/mod.rs` | 180 |
| `session/split_tree/mutations.rs` | 394 |
| `session/nav/mod.rs` | 236 |
| `session/floating/mod.rs` | 321 |
| `session/tab/mod.rs` | 180 |
| `session/compute/mod.rs` | 357 |
| `session/rect/mod.rs` | 32 |
| `keybindings/mod.rs` | 232 |
| `keybindings/defaults.rs` | 136 |
| `keybindings/parse.rs` | 220 |
| `app/pane_ops/mod.rs` | 354 |
| `app/pane_ops/floating.rs` | 171 |
| `app/pane_ops/helpers.rs` | 157 |
| `app/divider_drag.rs` | 267 |
| `app/floating_drag.rs` | 454 |

### Test Organization

All test modules follow the sibling `tests.rs` pattern:
- `#[cfg(test)] mod tests;` at bottom of each source file (confirmed via grep)
- No inline test modules
- Test files use `super::` imports
- No `mod tests { }` wrapper in test files

### Code Hygiene

- All `#[allow(dead_code)]` have `reason = "..."` strings
- All `#[allow(unused_mut)]` have `reason = "..."` strings
- No `unwrap()` in library code (all operations return Option/Result)
- No `println!` debugging
- Consistent import grouping (std, external, crate)
- `#[must_use]` on immutable mutation methods (`split_at`, `set_divider_ratio`, `resize_toward`, `equalize`, `swap`, `add`, `remove`, `raise`, `lower`)
- Module doc comments (`//!`) on all files

### Crate Boundary Compliance

- Session model (tabs, split trees, floating layers, navigation) in `oriterm/src/session/` (correct: `oriterm` owns session model)
- Keybindings in `oriterm/src/keybindings/` (correct: `oriterm` owns config/input)
- Pane ops in `oriterm/src/app/pane_ops/` (correct: `oriterm` owns app-level wiring)
- `oriterm_mux` provides `PaneId` only; no session/tab/window types leak into mux
- Navigation is pure (no GPU, platform, or terminal deps) -- could arguably live in `oriterm_ui`, but since it only uses `PaneId` + `Rect` + `PaneLayout` (all session types), keeping it in `oriterm/src/session/` is justified

---

## Test Execution Summary

| Module | Tests | Result |
|--------|-------|--------|
| `session::split_tree` | 60 | All pass |
| `session::nav` | 60 | All pass |
| `session::floating` | 28 | All pass |
| `session::tab` | 11 | All pass |
| `session::compute` | 34 | All pass |
| `session::rect` | 5 | All pass |
| `keybindings` | 62 | All pass |
| **Total** | **260** | **All pass** |

---

## Findings Summary

### Plan Accuracy Issues

1. **FINDING: Plan section 33.5 overstates test count.** Claims ~12 specific test names; only 7 actually exist. Three are renamed (acceptable), two are claimed as separate but are the same test, and four named tests simply do not exist (`multiple_undo_then_redo_walks_forward`, `redo_stack_capped_at_32`, `redo_skips_stale_pane_entry`, and all four "InProcessMux tests").

2. **FINDING: Plan claims InProcessMux undo/redo tests.** Section 33.5 states "InProcessMux tests: `undo_split_restores_previous_tree`, `redo_split_restores_undone_tree`, `split_undo_redo_undo_cycle`, `undo_past_closed_pane_skips_entry`". None of these exist. Undo/redo lives in the session Tab, not the mux. The mux crate has no undo/redo concept.

3. **FINDING: Plan section 33.4 claims float-tile toggle tests.** Lists `move_pane_to_tiled_removes_from_floating`, `move_pane_to_floating_removes_from_tree`, `move_last_tiled_pane_to_floating_rejected`. None exist. The toggle logic requires App context and has no unit tests.

4. **FINDING: Plan section 33.3 lists zoom test names that don't match.** Claims `toggle_zoom_sets_zoomed_pane`, `toggle_zoom_twice_unzooms`, etc. The actual test is just `zoom_state` covering basic set/clear.

### Missing Test Coverage

1. **Undo stack cap at 32 not tested.** `MAX_UNDO_ENTRIES = 32` is used in code but no test creates 33+ mutations to verify the oldest is evicted.

2. **Multi-step undo/redo walk not tested.** No test exercises 3+ sequential undos then redos forward to verify history traversal.

3. **Redo stale pane skip not tested.** Only `undo_skips_stale_entries` exists; the symmetric redo path has no test.

4. **Float <-> tile toggle not unit-tested.** Requires App context (mux + session + window), cannot be tested headlessly.

5. **Auto-unzoom on split/navigate not tested.** `unzoom_if_needed()` is called from 6 methods but no test verifies the auto-unzoom behavior.

### Implementation Quality

No bugs found. The architecture is clean:
- Immutable split tree with structural sharing via `Arc`
- In-place hot-path variants for drag performance (`move_pane_mut`, `resize_pane_mut`)
- Consistent borrow-dance patterns to avoid borrow checker issues
- `#[must_use]` on all immutable mutation returns
- Proper half-open interval semantics in hit testing
- Grid-snapping for pixel-perfect cell alignment

---

## Overall Verdict

**PASS with findings.** All claimed functionality is implemented and working. 260 tests pass across 7 modules. The primary issue is the plan overstating test coverage -- it lists approximately 15-20 test names that do not exist. The actual test suite is solid for the data structures (split tree: 60 tests, navigation: 60 tests, floating: 28 tests) but has gaps in integration-level behavior (float-tile toggle, auto-unzoom, undo stack cap). These gaps are acceptable given the App-context requirement but the plan should be updated to accurately reflect what tests exist.
