---
section: 9
title: "Session & Tab/Window Management"
domain: "oriterm/src/app/tab_management/, oriterm/src/session/"
status: not-started
---

# Section 09: Session & Tab/Window Management

Bugs in tab lifecycle, window management, tab movement, split trees, floating panes, and navigation.

## Open Bugs

- [ ] `[BUG-09-4][low]` **`oriterm/src/app/init/mod.rs` is 611 lines, 111 over the 500-line limit** — found by §09A.N post-split file-size sweep.
  Repro: `wc -l oriterm/src/app/init/mod.rs` prints `611`.
  Subsystem: `oriterm/src/app/init/mod.rs`.
  Analysis: Pre-existing BLOAT — mixes window creation, GPU init, initial-tab construction, handoff-tab creation, mux wiring, font pipeline boot, session bootstrap. Natural split points: GPU+font boot into `app/init/boot.rs`; handoff-tab creation into `app/init/handoff.rs`; session bootstrap into `app/init/session.rs`; keep the top-level `App::init()` orchestration in `mod.rs`.

- [ ] `[BUG-09-5][low]` **`oriterm/src/app/mod.rs` is 543 lines, 43 over the 500-line limit** — found by §09A.N post-split file-size sweep.
  Repro: `wc -l oriterm/src/app/mod.rs` prints `543`.
  Subsystem: `oriterm/src/app/mod.rs`.
  Analysis: Pre-existing BLOAT — `App` struct carries the aggregate application state and the file mixes struct definition with helper methods (focused_ctx, cell metrics broadcast, directional nav plumbing). Natural split: accessors + helpers into `app/accessors.rs`; keep `App` struct definition + `App::new()` + event-loop entry in `mod.rs`.

- [ ] `[BUG-09-6][low]` **`oriterm/src/cli/mod.rs` is 535 lines, 35 over the 500-line limit** — found by §09A.N post-split file-size sweep.
  Repro: `wc -l oriterm/src/cli/mod.rs` prints `535`.
  Subsystem: `oriterm/src/cli/mod.rs`.
  Analysis: Pre-existing BLOAT — CLI arg parsing + subcommand dispatch share one file. Natural split: per-subcommand handlers into `cli/commands/` directory.

- [ ] `[BUG-09-1][high]` **"Move to New Window" context menu action creates blank window** — found by manual.
  Repro: Right-click a tab > "Move to New Window" > new window appears blank. Dragging the same tab off (tear-off) works correctly.
  Subsystem: `oriterm/src/app/tab_management/move_ops.rs` (`move_tab_to_new_window_embedded`)
  Root cause (likely): The embedded path uses `create_window()` (which spawns a fresh pane/tab), moves the requested tab, then tries to close the initial tab. Compare with the working `tear_off_tab()` in `oriterm/src/app/tab_drag/tear_off.rs` which uses `create_window_bare()` (no initial tab), directly inserts the moved tab, pre-renders both windows, and explicitly shows the new window. The context menu path likely fails to properly activate the moved tab's content, wire up the pane rendering, or pre-render the new window.
  Found: 2026-03-31 | Source: manual
  Note: Active work in roadmap section 32 (tab-window-mux) and section 44 (multi-process-windows) touches this area.

- [ ] `[BUG-09-2][medium]` **`move_tab_to_window` resizes focused window's panes instead of destination window's** — found by tpr-review.
  Repro: Move a tab from window A to unfocused window B. `resize_all_panes()` called from `move_tab_to_window` uses `focused_ctx()` via `compute_pane_layouts`, so it recomputes and resizes the FOCUSED window's active tab — not the moved tab in window B. The moved tab's panes retain window A's grid dimensions.
  Detail: `oriterm/src/app/tab_management/move_ops.rs:39` calls `self.resize_all_panes()`. `resize_all_panes` (`oriterm/src/app/pane_ops/mod.rs:215`) calls `compute_pane_layouts()` which uses `self.active_window` + `self.focused_ctx()` (`oriterm/src/app/redraw/multi_pane/pane_layouts.rs:15-27`) — tied to the focused window, not the destination. If the destination window is unfocused at move time (cross-window moves from context menu or API), the moved tab's panes are not resized to fit the destination window's grid.
  Impact: Latent correctness issue. Currently masked because most tab-move paths (tear-off, move-to-new-window) end up making the destination focused before it matters. But per TPR-07-002-gemini (Section 07 round 8), Section 32's planned multi-window support will exercise the background-window move path where this bug manifests as stale grid dimensions on the moved panes.
  Proposed fix: Refactor `resize_all_panes` to take an explicit target window parameter, or add a `resize_panes_in_tab(tab_id, winit_id)` helper. `move_tab_to_window` then targets the destination window explicitly. Also audit `tear_off_tab` for the same pattern (it does its own session manipulation but still calls `resize_all_panes` implicitly via `handle_redraw`).
  Subsystem: `oriterm/src/app/pane_ops/mod.rs`, `oriterm/src/app/redraw/multi_pane/pane_layouts.rs`, `oriterm/src/app/tab_management/move_ops.rs`
  Found: 2026-04-13 | Source: tpr-review | Reviewer: gemini (TPR-07-002-gemini during spec-conformance Section 07 round 8 review)
  Note: Pre-existing code pattern that Section 07 round 7 surfaced when adding `seed_pane_with_window_cell_metrics` (which DOES target the destination window correctly — the LEAK is that `resize_all_panes` does not).

- [ ] `[BUG-09-3][low]` **No integration test for cross-window pane seeding** — found by tpr-review.
  Repro: N/A — test-coverage gap, not a behavioral bug.
  Detail: `App::seed_pane_with_window_cell_metrics` is called from `move_tab_to_window` and `tear_off_tab` after panes cross a window boundary, to seed each moved pane with the destination window's current cell metrics. Full integration tests verifying the seed helper is actually invoked from those call sites with the correct destination-window metrics require an App fixture with a mux + window + session that does not yet exist in the test infrastructure.
  Impact: A future refactor that removes the `seed_pane_with_window_cell_metrics` calls from move/tear-off paths would pass existing unit tests but silently regress cross-window cell-metric correctness.
  Proposed fix: Either (a) add an `App` test fixture under `oriterm/src/app/test_support.rs` that constructs a minimal `App` + `EmbeddedMux` + `WindowContext` + `Session` without GPU, suitable for unit-testing the seed behavior; or (b) add an architecture test under `oriterm/tests/architecture.rs` that greps for the call-site patterns (lightweight but catches accidental removal). Option (a) is the proper fix; option (b) is a stopgap.
  Subsystem: `oriterm/src/app/cell_metrics/`, `oriterm/src/app/test_support.rs`, `oriterm/tests/architecture.rs`
  Found: 2026-04-13 | Source: tpr-review | Reviewer: codex (TPR-07-001-codex during spec-conformance Section 07 round 8 review)
  Note: The companion short-circuit rule in `broadcast_cell_metrics_to_window` was separately flagged by `TPR-07-003-gemini` in the same round but is now covered by the `try_claim_broadcast` helper + 10 unit tests (R8 fix on commit ab6a6d8f); this bug is scoped to cross-window seed integration testing only.

## Resolved Bugs

(none yet)
