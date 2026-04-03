---
section: "03"
title: "Render Dispatch Consolidation"
status: not-started
reviewed: true
goal: "Consolidate duplicated render dispatch loops and fix dialog scene shrink gap"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Cross-Reference Dirty Window Loops"
    status: not-started
  - id: "03.2"
    title: "Consolidate Modal Loop Render"
    status: not-started
  - id: "03.3"
    title: "Fix Dialog Scene Shrink Gap"
    status: not-started
  - id: "03.4"
    title: "Extract is_any_dirty Helper"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Render Dispatch Consolidation

**Status:** Not Started
**Goal:** Eliminate duplicated render dispatch patterns in `render_dispatch.rs` and `event_loop_helpers/mod.rs`, and fix the missing dialog scene buffer shrink.

**Context:** There are two levels of duplication. *Inter-function*: `render_dirty_windows()` and `modal_loop_render()` share a near-identical focus-swap loop. *Intra-function*: within `render_dirty_windows`, the windows and dialogs sub-loops follow the same collect-dirty / clear-dirty / dispatch / clear-invalidation skeleton, differing only in focus-swap and FramePhases accumulation (windows have both; dialogs have neither). Additionally, dialog windows skip `scene.maybe_shrink()` in post-render cleanup.

**Testing feasibility:** All functions (`render_dirty_windows`, `modal_loop_render`, `is_any_window_dirty`) are methods on `App`, which requires GPU context, wgpu surfaces, and winit windows. Unit testing is infeasible. Verification relies on: (1) `./build-all.sh` (including `--target x86_64-pc-windows-gnu` for `modal_loop_render`), (2) `./clippy-all.sh`, (3) structural grep confirmation, and (4) `/tpr-review`. No new unit tests are expected.

---

## 03.1 Cross-Reference Dirty Window Loops

**File(s):** `oriterm/src/app/render_dispatch.rs`

`render_dirty_windows()` has two sub-loops with a shared skeleton:
- **Windows loop** (lines 30-47): collect dirty / for each: clear_dirty / focus-swap / `handle_redraw()` (captures `FramePhases`) / clear_invalidation / restore focus.
- **Dialogs loop** (lines 60-70): collect dirty / for each: clear_dirty / `render_dialog(wid)` / clear_invalidation. No focus-swap, no phase accumulation.

**Why extraction is infeasible:** A generic helper parameterized by closure cannot work because both `handle_redraw()` and `render_dialog(wid)` take `&mut self`, and the closure would capture `&mut self` while the helper also borrows it. The windows loop has 5 extra lines (focus-swap, phase accumulation) that don't apply to dialogs. At 10-17 lines each, the duplication is within tolerable threshold for distinct-behavior loops.

- [ ] Replace the existing comment at line 52 (`// Render dirty dialog windows (reuse the same scratch buffer).`) with a cross-reference comment:
  ```rust
  // Render dirty dialog windows (reuse the same scratch buffer).
  // NOTE: inner loop parallels the windows loop above — both follow
  // collect-dirty → clear-dirty → dispatch → clear-invalidation.
  // Mirror structural changes across both loops.
  ```
- [ ] Verify both sub-loops have matching structural shape: collect / `for i in 0..scratch.len()` / `get_mut` clear_dirty / dispatch / `get_mut` clear_invalidation. Currently true (verified at 93 lines).
- [ ] Run `./build-all.sh` and `./clippy-all.sh`.

---

## 03.2 Consolidate Modal Loop Render

**File(s):** `oriterm/src/app/event_loop_helpers/mod.rs`, `oriterm/src/app/render_dispatch.rs`

`modal_loop_render()` (lines 66-131, `#[cfg(target_os = "windows")]`) duplicates the inner loop of `render_dirty_windows()` (lines 19-51):
- Both collect dirty windows into `scratch_dirty_windows`.
- Both save/restore `focused_window_id` and `active_window`.
- Both loop: clear_dirty / set focus / `handle_redraw()` / clear_invalidation.

**Differences in `modal_loop_render`:**
- DPI/size detection preamble (lines 77-93).
- Early return when no terminal windows are dirty (lines 102-104).
- No dialog rendering: only iterates terminal windows.
- Discards `FramePhases` (line 120: return value dropped).
- No post-render shrink (`maybe_shrink_buffers` / `scene.maybe_shrink()`).
- Pumps mux events first (line 67).
- Sets `last_render` without `perf.record_render()` (line 130).

**Approach:** Have `modal_loop_render` delegate to `render_dirty_windows()`.

Behavioral change analysis:
- **Dialog rendering added:** Harmless. Dialogs are rarely dirty during modal resize; rendering an empty collection is a no-op.
- **Perf telemetry added:** `perf.record_render()` runs during modal loops. Benign — more data, no side effects beyond bookkeeping.
- **Post-render shrink added:** No-ops when buffers are small (guard: `if capacity > 4 * len && capacity > 4096`).
- **FramePhases now accumulated:** `render_dirty_windows` accumulates phases and passes them to `perf.record_render()`. Currently discarded in modal loops. Benign addition of telemetry.
- **`last_render` recorded for zero-dirty-windows:** The early return (kept) prevents this.

- [ ] Refactor `modal_loop_render` in `event_loop_helpers/mod.rs`:
  1. Keep `self.pump_mux_events()` (line 67).
  2. Keep the DPI/size detection preamble (lines 77-93).
  3. Keep the early return when no windows are dirty (lines 95-104). Note: this currently only checks terminal windows. After delegation, `render_dirty_windows()` also renders dialogs, so a dirty dialog during modal loop would be skipped by this early return. This is acceptable: dialogs are rarely dirty during modal resize, and the asymmetry existed before (modal loops never rendered dialogs). If 03.4 has landed, consider using `is_any_window_dirty()` here instead for correctness.
  4. Replace lines 106-130 (the focus-swap loop + `last_render` update) with: `self.render_dirty_windows();`.
  5. The `last_render` update is now inside `render_dirty_windows()` — remove the redundant one.
- [ ] Verify `perf.record_render()` has no side effects beyond bookkeeping (confirmed: `perf_stats.rs:136-146` — increments counters and records frame time).
- [ ] Verify no dialog-specific state mutation in the dialogs loop of `render_dirty_windows` that could interfere with modal resize (confirmed: the loop calls `render_dialog(wid)` and `clear()` — no focus-swap or global state changes).
- [ ] Verify compilation on Windows target: `cargo build --target x86_64-pc-windows-gnu` or `./build-all.sh`. The method is `#[cfg(target_os = "windows")]` — host-only `cargo build` skips it.
- [ ] Verify `modal_loop_render` shrinks from ~65 lines to ~38 lines (pump + DPI preamble ~27 lines + early return ~10 lines + one delegation call).
- [ ] Verify `event_loop_helpers/mod.rs` stays under 500 lines (currently 431; removing ~25 lines brings it to ~406).
- [ ] Run `./build-all.sh` and `./clippy-all.sh`.

**Fallback:** If delegation causes unforeseen issues, add `// NOTE: inner loop mirrors render_dirty_windows()` comments in both locations instead.

---

## 03.3 Fix Dialog Scene Shrink Gap

**File(s):** `oriterm/src/app/render_dispatch.rs`

Post-render buffer shrinking (lines 76-91) applies `chrome_scene.maybe_shrink()` to terminal windows but not `scene.maybe_shrink()` to dialog windows. The terminal window loop (lines 76-82) shrinks 3 things: `renderer.maybe_shrink_buffers()`, `ctx.chrome_scene.maybe_shrink()`, `ctx.root.damage_mut().maybe_shrink()`. The dialog loop (lines 83-88) only shrinks 2: renderer and damage. Missing: `ctx.scene.maybe_shrink()`.

- [ ] Add `ctx.scene.maybe_shrink()` to the dialog post-render shrink loop:
  ```rust
  for ctx in self.dialogs.values_mut() {
      if let Some(renderer) = ctx.renderer.as_mut() {
          renderer.maybe_shrink_buffers();
      }
      ctx.scene.maybe_shrink();
      ctx.root.damage_mut().maybe_shrink();
  }
  ```
  Visibility: `scene` is `pub(super)` on `DialogWindowContext` (line 54 of `dialog_context/mod.rs`); `render_dispatch.rs` is in the `app` module, so access is valid. `Scene::maybe_shrink(&mut self)` is `pub` (line 125 of `oriterm_ui/src/draw/scene/mod.rs`).
- [ ] After the change, grep `render_dispatch.rs` for `maybe_shrink` to verify structural symmetry: terminal loop has 3 calls (renderer, chrome_scene, damage), dialog loop now has 3 calls (renderer, scene, damage). Note: `chrome_scene` vs `scene` is correct — different fields on different context types.
- [ ] Run `./build-all.sh` and `./clippy-all.sh`.

---

## 03.4 Extract is_any_dirty Helper

**File(s):** `oriterm/src/app/render_dispatch.rs` (new method), `oriterm/src/app/event_loop.rs` (call sites)

The dirty-check computation is duplicated within 30 lines of `event_loop.rs`:
- Line 432: `let any_dirty = self.windows.values().any(|ctx| ctx.root.is_dirty()) || self.dialogs.values().any(|ctx| ctx.root.is_dirty());`
- Line 459: `let still_dirty = self.windows.values().any(|c| c.root.is_dirty()) || self.dialogs.values().any(|c| c.root.is_dirty());`

Identical computation, different variable names (temporal distinction: before vs. after render).

Note: `urgent_redraw` (lines 435-442) and `has_animations` (lines 461-468) also iterate both collections but with different predicates. Extracting them is optional and low priority since each appears only once.

- [ ] Add `fn is_any_window_dirty(&self) -> bool` on `App` in `render_dispatch.rs`:
  ```rust
  /// Returns `true` if any terminal or dialog window needs rendering.
  fn is_any_window_dirty(&self) -> bool {
      self.windows.values().any(|c| c.root.is_dirty())
          || self.dialogs.values().any(|c| c.root.is_dirty())
  }
  ```
  `render_dispatch.rs` is 93 lines; adding this (~6 lines) brings it to ~99. With 03.3's +1 line, total ~100.
- [ ] Replace `let any_dirty = ...` (line 432 of `event_loop.rs`) with `let any_dirty = self.is_any_window_dirty();`.
- [ ] Replace `let still_dirty = ...` (line 459 of `event_loop.rs`) with `let still_dirty = self.is_any_window_dirty();`.
- [ ] Verify `event_loop.rs` stays under 500 lines (currently 497; removing 2 two-line expressions and replacing with 1-line calls saves ~2 lines, target ~495).
- [ ] Optionally, extract `is_any_urgent_redraw(&self) -> bool` and `has_active_animations(&self) -> bool` into `render_dispatch.rs` to further reduce `about_to_wait`. Low priority — do not block completion. If both extracted, `event_loop.rs` drops ~8 more lines to ~487; `render_dispatch.rs` grows to ~115.
- [ ] Run `./build-all.sh` and `./clippy-all.sh`.

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] Dialog scene buffer shrunk in post-render (03.3)
- [ ] Both post-render shrink loops have 3 `maybe_shrink` calls each (grep `render_dispatch.rs`)
- [ ] `is_any_window_dirty()` exists; both dirty checks in `event_loop.rs` use it (03.4)
- [ ] Grep `event_loop.rs` for `self.windows.values().any(|c| c.root.is_dirty())` returns zero matches
- [ ] Windows and dialogs sub-loops in `render_dirty_windows` have cross-reference comments (03.1)
- [ ] `modal_loop_render` delegates to `render_dirty_windows` or has cross-reference comments (03.2)
- [ ] `event_loop.rs` under 500 lines (target ~495)
- [ ] `event_loop_helpers/mod.rs` under 500 lines (target ~406)
- [ ] `render_dispatch.rs` under 500 lines (target ~100)
- [ ] No new unit tests required (GPU/platform methods — see testing feasibility note)
- [ ] `./test-all.sh` green
- [ ] `./build-all.sh` green (includes `--target x86_64-pc-windows-gnu`)
- [ ] `./clippy-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** No duplicated dirty-check computation. Dialog scene buffers are shrunk. Modal loop render skeleton is either consolidated into `render_dirty_windows` or clearly cross-referenced. All affected files under 500-line limit.
