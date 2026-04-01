---
section: "04"
title: "Render Pipeline Migration"
status: not-started
reviewed: true
goal: "Switch the render pipeline to read from the IO thread's shared snapshot buffer instead of locking the terminal via FairMutex"
inspired_by:
  - "Ghostty renderer/Thread.zig (renderer reads published terminal state, never locks IO thread)"
  - "ori_term EmbeddedMux::swap_renderable_content() (existing swap pattern reused)"
depends_on: ["03"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "EmbeddedMux Snapshot Source Switch"
    status: not-started
  - id: "04.2"
    title: "PaneSnapshot Building from IO Thread"
    status: not-started
  - id: "04.3"
    title: "Preserve Old Path for Non-Render Operations"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: Render Pipeline Migration

**Status:** Not Started
**Goal:** Switch the render pipeline from `pane.terminal().lock() → renderable_content_into()` to reading from the IO thread's `SnapshotDoubleBuffer` buffer. After this section, the render path never acquires the FairMutex.

**Context:** Today, `EmbeddedMux::refresh_pane_snapshot()` calls `build_snapshot_into(pane, ...)` which locks the terminal. The renderer holds the lock for the entire snapshot extraction (iterating all visible cells, resolving colors, extracting images). After this section, the render path takes the pre-built `RenderableContent` from the IO thread's shared buffer — no terminal lock needed.

This is the critical switchover point for rendering. After this section, the render hot path no longer acquires the FairMutex. However, the old `PtyEventLoop` parsing path MUST remain active — non-render operations (scroll, search, text extraction, resize) still use the old `Arc<FairMutex<Term>>` until sections 05-06 migrate them.

**Reference implementations:**
- **Ghostty** `src/renderer/Thread.zig`: Renderer thread reads terminal state that was prepared by the IO thread. No direct lock on terminal state.
- **ori_term** `oriterm_mux/src/backend/embedded/mod.rs:350-364`: `swap_renderable_content()` already uses `std::mem::swap` for zero-allocation content transfer. This pattern is reused — just the source changes from `build_snapshot_into` to `SnapshotDoubleBuffer::swap_front()`.

**Depends on:** Section 03 (IO thread produces and publishes snapshots).

---

## 04.1 EmbeddedMux Snapshot Source Switch

**File(s):** `oriterm_mux/src/backend/embedded/mod.rs`

Change `refresh_pane_snapshot()` and `swap_renderable_content()` to read from the IO thread's shared buffer instead of locking the terminal.

- [ ] Add `SnapshotDoubleBuffer` access to `EmbeddedMux` via `Pane`:
  - `Pane` holds `PaneIoHandle` which holds `SnapshotDoubleBuffer`
  - `Pane::swap_io_snapshot(&self, buf: &mut RenderableContent) -> bool` delegates to `io_handle.double_buffer.swap_front(buf)`

- [ ] Rewrite `refresh_pane_snapshot()`:
  ```rust
  fn refresh_pane_snapshot(&mut self, pane_id: PaneId) -> Option<&PaneSnapshot> {
      let pane = self.panes.get(&pane_id)?;
      
      // Swap the latest snapshot from the IO thread's double buffer
      // into our persistent renderable_cache (zero-allocation swap).
      let render_buf = self.renderable_cache.entry(pane_id).or_default();
      pane.swap_io_snapshot(render_buf); // returns bool, but we proceed either way
      
      // Build PaneSnapshot from the renderable content (for IPC/test paths).
      let render_buf = self.renderable_cache.get(&pane_id)?;
      let snapshot = self.snapshot_cache.entry(pane_id).or_default();
      // Fill snapshot metadata without locking the terminal.
      // NOTE: Some metadata (title, CWD, search state) lives on Pane,
      // not on Term — those are already accessible without a lock.
      build_snapshot_metadata_from_renderable(pane, render_buf, snapshot);
      
      self.snapshot_dirty.remove(&pane_id);
      self.snapshot_cache.get(&pane_id)
  }
  ```

- [ ] The existing `swap_renderable_content()` needs minimal changes — it already reads from `renderable_cache`. The source of data in `renderable_cache` just changed from `build_snapshot_into()` to `SnapshotDoubleBuffer::swap_front()`.
- [ ] Update `EmbeddedMux::swap_renderable_content()` to include the new `RenderableContent` fields: copy `cols`, `lines`, `scrollback_len` as scalars; swap `palette_snapshot` Vec (for allocation reuse); and include any search fields added in section 06.4 (`search_active`, `search_query`, `search_matches`, `search_focused`, `search_total_matches`). Consider refactoring to `std::mem::swap` on the entire struct instead of field-by-field — simpler and future-proof against new fields.

- [ ] Handle the case where no new snapshot is available (IO thread hasn't published yet): return the previously cached snapshot. This is the "last good frame" behavior that prevents flashing.

- [ ] Create `build_snapshot_metadata_from_renderable()` in `oriterm_mux/src/server/snapshot.rs` — a variant of `fill_snapshot_metadata()` that reads metadata without needing `&Term`.

  **Metadata field sourcing** (from current `fill_snapshot_metadata` in `snapshot.rs`):
  
  | Field | Current source | New source (no lock) |
  |-------|---------------|---------------------|
  | `cursor` | `render_buf.cursor` | Same (already in `RenderableContent`) |
  | `palette` | `term.palette()` | **Must add to `RenderableContent`** |
  | `title` | `pane.effective_title()` | Same (Pane-local, no lock) |
  | `icon_name` | `pane.icon_name()` | Same (Pane-local) |
  | `cwd` | `pane.cwd()` | Same (Pane-local) |
  | `has_unseen_output` | `pane.has_unseen_output()` | Same (Pane-local) |
  | `modes` | `render_buf.mode.bits()` | Same (already in `RenderableContent`) |
  | `scrollback_len` | `grid.scrollback().len()` | **Must add to `RenderableContent`** |
  | `display_offset` | `render_buf.display_offset` | Same (already in `RenderableContent`) |
  | `stable_row_base` | `render_buf.stable_row_base` | Same (already in `RenderableContent`) |
  | `cols` | `grid.cols()` | **Must add to `RenderableContent`** |
  | `lines` | `grid.lines()` | **Must add to `RenderableContent`** |
  | `search_*` | `pane.search()` | Same (Pane-local, until section 06.4 moves search to IO thread) |

  **Required changes to `RenderableContent`** (in `oriterm_core/src/term/renderable/mod.rs`):
  - Add `pub cols: usize` field
  - Add `pub lines: usize` field (visible viewport height)
  - Add `pub scrollback_len: usize` field
  - Add `pub palette_snapshot: Vec<[u8; 3]>` field (270 entries, pre-resolved RGB — allocated once, reused via buffer swap)
  - Populate these in `renderable_content_into()` — trivial reads from `Grid` and `Palette`
  - Update `Default` impl to initialize new fields to 0/empty
  - Update `maybe_shrink()` — `palette_snapshot` is fixed-size (270), no shrink needed

  This keeps the snapshot self-contained. The IO thread's `renderable_content_into()` fills all needed metadata alongside cells. The main thread reads the snapshot without any terminal lock.

- [ ] **Crate boundary note**: The new fields on `RenderableContent` (`cols`, `lines`, `scrollback_len`, `palette_snapshot`) are added in `oriterm_core`, not `oriterm_mux`. Run `timeout 150 cargo test -p oriterm_core` after adding them to verify no breakage in the core crate.

- [ ] `/tpr-review` checkpoint

### Tests

**File:** `oriterm_mux/src/backend/embedded/tests.rs` (extend existing)

- [ ] `test_refresh_pane_snapshot_reads_from_io_thread` — spawn a pane with IO thread, write "hello" to the PTY, wait for snapshot to appear in `SnapshotDoubleBuffer`. Call `refresh_pane_snapshot()`. Assert returned `PaneSnapshot` has cells containing "hello". Verifies the full IO-thread-to-render path.
- [ ] `test_refresh_pane_snapshot_returns_cached_when_no_new` — call `refresh_pane_snapshot()` twice without new IO thread activity. Assert the second call returns the same snapshot (cached, no stale data).
- [ ] `test_swap_renderable_content_from_io_thread` — produce a snapshot via IO thread, call `refresh_pane_snapshot()` then `swap_renderable_content()`. Assert the swapped content has cells. Verify Vec capacities are retained (allocation reuse).
- [ ] `test_metadata_from_renderable_has_cols_lines` — produce a snapshot from a 80x24 grid. Assert `PaneSnapshot` has `cols == 80` and `lines == 24` from the new `RenderableContent` fields.
- [ ] `test_damage_tracking_through_io_snapshots` — produce two snapshots: first with all_dirty, second with only line 5 dirty. Assert the second snapshot's damage reflects only line 5 (not all_dirty).

**File:** `oriterm_core/src/term/renderable/tests.rs` (extend existing)

- [ ] `test_renderable_content_new_fields_populated` — call `renderable_content_into()` on a Term with known grid dimensions. Assert `cols`, `lines`, `scrollback_len`, and `palette_snapshot` are correctly filled.
- [ ] `test_renderable_content_palette_snapshot_correct` — set specific palette colors, produce snapshot. Assert `palette_snapshot` entries match the expected RGB values.
- [ ] `test_renderable_content_default_new_fields` — verify `Default::default()` initializes `cols`, `lines`, `scrollback_len` to 0 and `palette_snapshot` to empty Vec.

---

## 04.2 PaneSnapshot Building from IO Thread

**File(s):** `oriterm_mux/src/server/snapshot.rs`

Update the snapshot building to work without terminal lock access for the daemon mode path.

- [ ] Update `SnapshotCache::build()` to read from `SnapshotDoubleBuffer` when available:
  ```rust
  pub fn build(&mut self, pane_id: PaneId, pane: &Pane) -> &PaneSnapshot {
      let cached = self.cache.entry(pane_id).or_default();
      // Try IO thread snapshot first (zero-lock path).
      if pane.swap_io_snapshot(&mut self.render_buf) {
          build_snapshot_metadata_from_renderable(pane, &self.render_buf, cached);
          fill_wire_cells_from_renderable(&self.render_buf, cached);
      } else {
          // Fallback: lock terminal (until section 07 removes this path).
          let mut term = pane.terminal().lock();
          build_snapshot_inner_into(&term, pane, cached, &mut self.render_buf);
          term.reset_damage();
      }
      &self.cache[&pane_id]
  }
  ```
- [ ] Add `Pane::swap_io_snapshot(&self, buf: &mut RenderableContent) -> bool` — delegates to `io_handle.double_buffer.swap_front(buf)` if the IO handle exists, returns `false` otherwise
- [ ] Ensure `SnapshotCache` path works with both old and new approaches during transition
- [ ] Implement `fill_wire_cells_from_renderable()` in `oriterm_mux/src/server/snapshot.rs` — converts `RenderableContent` cells to `WireCell` format for daemon IPC, without needing `&Term`. This is the daemon-mode equivalent of the embedded path's direct `swap_renderable_content()`.

---

## 04.3 Preserve Old Path for Non-Render Operations

**File(s):** `oriterm_mux/src/pane/mod.rs`

**Critical**: The old `PtyEventLoop` parsing path and `Arc<FairMutex<Term>>` MUST remain active. Non-render operations (scroll, resize, search, text extraction, mark mode, prompt navigation) still lock the old `Term`. Disabling old parsing before those operations are migrated would make them operate on stale state — scroll offsets would be wrong, search would find no matches, text extraction would return empty strings.

- [ ] **Keep the dual-Term architecture running.** Both `Term` instances continue processing the same byte stream. The old path stays authoritative for:
  - `resize_grid()` (until section 05)
  - `scroll_display()` / `scroll_to_bottom()` (until section 06)
  - `extract_text()` / `extract_html()` (until section 06)
  - `search_set_query()` / `search_next_match()` / `search_prev_match()` (until section 06)
  - `enter_mark_mode()` (until section 06)
  - `scroll_to_previous_prompt()` / `scroll_to_next_prompt()` (until section 06)

- [ ] The render path switches to IO thread snapshots (subsections 04.1-04.2). The old path continues for everything else.

- [ ] `Pane` holds BOTH `Arc<FairMutex<Term>>` (old) and `PaneIoHandle` (new). Both are populated and active.

- [ ] The old `PtyEventLoop` parsing is only disabled in section 07, AFTER sections 05-06 have migrated all operations to IO thread commands.

- [ ] Accept the CPU cost of dual parsing during the transition (sections 04-06). This is temporary and ensures correctness at every step.

---

## 04.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 04.N Completion Checklist

- [ ] `refresh_pane_snapshot()` reads from `SnapshotDoubleBuffer::swap_front()`, not `pane.terminal().lock()`
- [ ] `swap_renderable_content()` works with IO-thread-produced snapshots
- [ ] Daemon mode snapshot path updated to read from IO thread
- [ ] Old `PtyEventLoop` VTE parsing still active (dual-Term during transition)
- [ ] Two `Term` instances per pane: IO thread (render) + FairMutex (operations)
- [ ] Terminal renders correctly from IO-thread snapshots
- [ ] No `FairMutex::lock()` calls in the render hot path (only in non-render ops)
- [ ] Damage tracking works through IO-thread snapshots
- [ ] `timeout 150 cargo test -p oriterm_mux` passes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** The render pipeline reads exclusively from IO-thread-produced snapshots. No `FairMutex::lock()` in the render hot path. The terminal displays correctly with content from the IO thread. The old parsing path remains active for non-render operations (scroll, search, etc.) until sections 05-06 migrate them.
