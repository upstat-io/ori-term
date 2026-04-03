---
section: "03"
title: "Render Dispatch Consolidation"
status: not-started
reviewed: false
goal: "Consolidate duplicated render dispatch loops and fix dialog scene shrink gap"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Consolidate Dirty Window Loops"
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

**Context:** The dirty-window render loop pattern (collect dirty → save focus → loop dispatch → restore focus) appears twice with near-identical skeletons. Within `render_dirty_windows`, the windows and dialogs loops are also structurally duplicated. Additionally, dialog windows skip `scene.maybe_shrink()` in post-render cleanup.

---

## 03.1 Consolidate Dirty Window Loops

**File(s):** `oriterm/src/app/render_dispatch.rs`

`render_dirty_windows()` has two structurally identical loops (lines 30-47 for windows, lines 60-70 for dialogs):
```
collect dirty → for each: clear_dirty → dispatch → clear_invalidation
```

The only differences are: collection source (`self.windows` vs `self.dialogs`) and dispatch method (`handle_redraw()` vs `render_dialog()`).

- [ ] Extract the shared loop skeleton. Options:
  **(a)** A helper method `dispatch_dirty<F>(&mut self, collect: ..., handler: F)` parameterized by a closure (may have borrow issues with `&mut self`).
  **(b)** Refactor the dialog loop to follow a `for i in 0..scratch.len()` pattern identical to the window loop, reducing visual duplication even if not fully extracted.
  **(c)** If borrow issues prevent closure extraction, reduce to a shared comment pattern and ensure both loops are byte-for-byte identical in structure (currently close but not identical).
- [ ] Verify post-render buffer shrinking still runs for both windows and dialogs.

---

## 03.2 Consolidate Modal Loop Render

**File(s):** `oriterm/src/app/event_loop_helpers/mod.rs`, `oriterm/src/app/render_dispatch.rs`

`modal_loop_render()` (event_loop_helpers/mod.rs:93-128) duplicates the core of `render_dirty_windows()` (render_dispatch.rs:30-47):
- Both collect dirty windows into `scratch_dirty_windows`
- Both save/restore `focused_window_id` and `active_window`
- Both loop: clear_dirty → set focus → `handle_redraw()` → clear_invalidation
- Only differences: `modal_loop_render` has a DPI check preamble, no dialog rendering, no timing instrumentation, no post-render shrink.

- [ ] Evaluate extraction feasibility. Since `modal_loop_render` is `#[cfg(target_os = "windows")]` only, consider:
  **(a)** Extract the shared focus-swap-dispatch loop into a private helper, called by both `render_dirty_windows` and `modal_loop_render`.
  **(b)** Have `modal_loop_render` call `render_dirty_windows` directly (if the DPI check and missing dialog render are safe to add/skip).
- [ ] If extraction isn't clean due to the DPI check preamble being interleaved, at minimum add a `// NOTE: skeleton mirrors render_dirty_windows()` cross-reference comment in both locations.

---

## 03.3 Fix Dialog Scene Shrink Gap

**File(s):** `oriterm/src/app/render_dispatch.rs`

Post-render buffer shrinking (lines 76-88) applies `chrome_scene.maybe_shrink()` to terminal windows but NOT to dialog windows. `DialogWindowContext` has a `scene: Scene` field (at `dialog_context/mod.rs:54`) that should also be shrunk.

- [ ] Add `ctx.scene.maybe_shrink()` to the dialog post-render shrink loop:
  ```rust
  for ctx in self.dialogs.values_mut() {
      if let Some(renderer) = ctx.renderer.as_mut() {
          renderer.maybe_shrink_buffers();
      }
      ctx.scene.maybe_shrink(); // <-- ADD THIS
      ctx.root.damage_mut().maybe_shrink();
  }
  ```

---

## 03.4 Extract is_any_dirty Helper

**File(s):** `oriterm/src/app/event_loop.rs`

The dirty-check computation is duplicated within 30 lines:
- Line 432: `let any_dirty = self.windows.values().any(|ctx| ctx.root.is_dirty()) || self.dialogs.values().any(|ctx| ctx.root.is_dirty());`
- Line 459: `let still_dirty = self.windows.values().any(|c| c.root.is_dirty()) || self.dialogs.values().any(|c| c.root.is_dirty());`

Identical computation with different variable names (temporal distinction: before vs after render).

- [ ] Extract a method on `App`:
  ```rust
  fn is_any_window_dirty(&self) -> bool {
      self.windows.values().any(|c| c.root.is_dirty())
          || self.dialogs.values().any(|c| c.root.is_dirty())
  }
  ```
- [ ] Replace both inline computations with calls to `is_any_window_dirty()`.

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] Dialog scene buffer is shrunk in post-render (gap fixed)
- [ ] `is_any_window_dirty()` method exists and is used for all dirty checks
- [ ] Render dispatch loops are consolidated or clearly cross-referenced
- [ ] `./test-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** No duplicated dirty-check computation. Dialog scene buffers are shrunk. Modal loop render skeleton is either extracted or cross-referenced with render_dirty_windows.
