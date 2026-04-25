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

- [x] `[BUG-09-1][high]` **"Move to New Window" context menu action creates blank window**
  Resolved: fixed on 2026-04-24. See plans/bug-tracker/fix-BUG-09-001.md. Refactored `move_tab_to_new_window_embedded` (`oriterm/src/app/tab_management/move_ops.rs`) to mirror the working `tear_off_tab` sequence (`oriterm/src/app/tab_drag/tear_off.rs`): use `create_window_bare()` (hidden, no initial tab) instead of `create_window()` (visible, with auto-spawned tab), insert the moved tab directly via `win.insert_tab_at(0, tab_id)`, pump mux events, seed each moved pane with the new window's cell metrics, sync tab bars + refresh platform rects on BOTH windows, pre-render the new window via focused-id + active-window swap with `handle_redraw()`, pre-render the source, then `set_visible(true)`. Removed the dead `move_tab_to_window` helper (closed BUG-09-2 as OBE). 4 commits on dev (321a10ad..dd582549) including 3 rounds of /tpr-review to both-clean convergence. 2 architecture-tests pin the canonical call sequence in order (mirror invariant + dead-helper-stays-removed) — they catch any future omission, reordering, or resurrection of the buggy helper.

- [x] `[BUG-09-2][medium]` **`move_tab_to_window` resizes focused window's panes instead of destination window's**
  Resolved: OBE on 2026-04-24. The `move_tab_to_window` helper itself was removed during the BUG-09-1 fix — its only caller (`move_tab_to_new_window_embedded`) was refactored to mirror the working `tear_off_tab` pattern (direct session insert + per-pane `seed_pane_with_window_cell_metrics(new_winit_id, pid)` + explicit pre-render of the new window via focused-id swap). The destination-targeted seed call replaces the broken `resize_all_panes()` flow this bug described. No remaining caller of the buggy resize path. See plans/bug-tracker/fix-BUG-09-001.md and the BUG-09-1 fix commit chain.

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
