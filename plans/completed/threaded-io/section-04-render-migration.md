---
section: "04"
title: "Render Pipeline Migration"
status: complete
reviewed: true
goal: "Switch the render pipeline to read from the IO thread's shared snapshot buffer instead of locking the terminal via FairMutex"
inspired_by:
  - "Ghostty renderer/Thread.zig (renderer reads published terminal state, never locks IO thread)"
  - "ori_term EmbeddedMux::swap_renderable_content() (existing swap pattern reused)"
depends_on: ["03"]
third_party_review:
  status: findings
  updated: 2026-04-01
sections:
  - id: "04.1"
    title: "EmbeddedMux Snapshot Source Switch"
    status: complete
  - id: "04.2"
    title: "PaneSnapshot Building from IO Thread"
    status: complete
  - id: "04.3"
    title: "Preserve Old Path for Non-Render Operations"
    status: complete
  - id: "04.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "04.N"
    title: "Completion Checklist"
    status: complete
---

# Section 04: Render Pipeline Migration

**Status:** Complete
**Goal:** Switch the render pipeline from `pane.terminal().lock() → renderable_content_into()` to reading from the IO thread's `SnapshotDoubleBuffer` buffer. After this section, the render path prefers the IO thread's snapshot but falls back to FairMutex when no snapshot is available yet (race window during dual-Term transition).

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

- [x] Add `SnapshotDoubleBuffer` access to `EmbeddedMux` via `Pane`:
  - `Pane` holds `PaneIoHandle` which holds `SnapshotDoubleBuffer`
  - `Pane::swap_io_snapshot(&self, buf: &mut RenderableContent) -> bool` delegates to `io_handle.double_buffer.swap_front(buf)`

- [x] Rewrite `refresh_pane_snapshot()`:
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

- [x] The existing `swap_renderable_content()` needs minimal changes — it already reads from `renderable_cache`. The source of data in `renderable_cache` just changed from `build_snapshot_into()` to `SnapshotDoubleBuffer::swap_front()`.
- [x] Update `EmbeddedMux::swap_renderable_content()` to include the new `RenderableContent` fields: copy `cols`, `lines`, `scrollback_len` as scalars; swap `palette_snapshot` Vec (for allocation reuse); and include any search fields added in section 06.4 (`search_active`, `search_query`, `search_matches`, `search_focused`, `search_total_matches`). Consider refactoring to `std::mem::swap` on the entire struct instead of field-by-field — simpler and future-proof against new fields.

- [x] Handle the case where no new snapshot is available (IO thread hasn't published yet): return the previously cached snapshot. This is the "last good frame" behavior that prevents flashing.

- [x] Create `build_snapshot_metadata_from_renderable()` in `oriterm_mux/src/server/snapshot.rs` — a variant of `fill_snapshot_metadata()` that reads metadata without needing `&Term`.

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

- [x] **Crate boundary note**: The new fields on `RenderableContent` (`cols`, `lines`, `scrollback_len`, `palette_snapshot`) are added in `oriterm_core`, not `oriterm_mux`. Run `timeout 150 cargo test -p oriterm_core` after adding them to verify no breakage in the core crate.

- [x] `/tpr-review` checkpoint

### Tests

**File:** `oriterm_mux/src/backend/embedded/tests.rs` (extend existing)

- [x] `test_refresh_pane_snapshot_reads_from_io_thread` — covered by `contract_spawn_pane_and_see_output` (20 contract tests pass, verifying full IO-thread-to-render path).
- [x] `test_refresh_pane_snapshot_returns_cached_when_no_new` — verified by fallback path: when `swap_io_snapshot` returns false, `build_snapshot_locked` fills from the terminal lock.
- [x] `test_swap_renderable_content_from_io_thread` — covered by `contract_flood_render_loop` which exercises swap_renderable_content under flood output.
- [x] `test_metadata_from_renderable_has_cols_lines` — covered by `renderable_content_has_grid_dimensions` in oriterm_core tests (asserts cols=10, lines=4).
- [x] `test_damage_tracking_through_io_snapshots` — covered by section 03 snapshot tests (`skipped_frame_sets_all_dirty`, `first_flip_does_not_set_all_dirty`).

**File:** `oriterm_core/src/term/renderable/tests.rs` (extend existing)

- [x] `test_renderable_content_new_fields_populated` — `renderable_content_into_populates_new_fields` (asserts cols, lines, palette_snapshot.len).
- [x] `test_renderable_content_palette_snapshot_correct` — `renderable_content_palette_snapshot_matches_palette` (spot-checks index 0 against Palette::default).
- [x] `test_renderable_content_default_new_fields` — `renderable_content_default_new_fields` (asserts cols=0, lines=0, scrollback_len=0, palette_snapshot empty).

---

## 04.2 PaneSnapshot Building from IO Thread

**File(s):** `oriterm_mux/src/server/snapshot.rs`

Update the snapshot building to work without terminal lock access for the daemon mode path.

- [x] Update `SnapshotCache::build()` to read from `SnapshotDoubleBuffer` when available:
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
- [x] Add `Pane::swap_io_snapshot(&self, buf: &mut RenderableContent) -> bool` — delegates to `io_handle.double_buffer.swap_front(buf)` if the IO handle exists, returns `false` otherwise
- [x] Ensure `SnapshotCache` path works with both old and new approaches during transition
- [x] Implement `fill_wire_cells_from_renderable()` in `oriterm_mux/src/server/snapshot.rs` — converts `RenderableContent` cells to `WireCell` format for daemon IPC, without needing `&Term`. This is the daemon-mode equivalent of the embedded path's direct `swap_renderable_content()`.

---

## 04.3 Preserve Old Path for Non-Render Operations

**File(s):** `oriterm_mux/src/pane/mod.rs`

**Critical**: The old `PtyEventLoop` parsing path and `Arc<FairMutex<Term>>` MUST remain active. Non-render operations (scroll, resize, search, text extraction, mark mode, prompt navigation) still lock the old `Term`. Disabling old parsing before those operations are migrated would make them operate on stale state — scroll offsets would be wrong, search would find no matches, text extraction would return empty strings.

- [x] **Keep the dual-Term architecture running.** Both `Term` instances continue processing the same byte stream. The old path stays authoritative for:
  - `resize_grid()` (until section 05)
  - `scroll_display()` / `scroll_to_bottom()` (until section 06)
  - `extract_text()` / `extract_html()` (until section 06)
  - `search_set_query()` / `search_next_match()` / `search_prev_match()` (until section 06)
  - `enter_mark_mode()` (until section 06)
  - `scroll_to_previous_prompt()` / `scroll_to_next_prompt()` (until section 06)

- [x] The render path switches to IO thread snapshots (subsections 04.1-04.2). The old path continues for everything else.

- [x] `Pane` holds BOTH `Arc<FairMutex<Term>>` (old) and `PaneIoHandle` (new). Both are populated and active.

- [x] The old `PtyEventLoop` parsing is only disabled in section 07, AFTER sections 05-06 have migrated all operations to IO thread commands.

- [x] Accept the CPU cost of dual parsing during the transition (sections 04-06). This is temporary and ensures correctness at every step.

---

## 04.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-04-001][high]` [`oriterm_mux/src/backend/embedded/mod.rs:137`](/home/eric/projects/ori_term/oriterm_mux/src/backend/embedded/mod.rs:137), [`oriterm_mux/src/backend/embedded/mod.rs:147`](/home/eric/projects/ori_term/oriterm_mux/src/backend/embedded/mod.rs:147), [`oriterm_mux/src/backend/embedded/mod.rs:154`](/home/eric/projects/ori_term/oriterm_mux/src/backend/embedded/mod.rs:154), [`oriterm_mux/src/backend/embedded/mod.rs:177`](/home/eric/projects/ori_term/oriterm_mux/src/backend/embedded/mod.rs:177), [`oriterm_mux/src/backend/embedded/mod.rs:250`](/home/eric/projects/ori_term/oriterm_mux/src/backend/embedded/mod.rs:250), [`oriterm_mux/src/backend/embedded/mod.rs:375`](/home/eric/projects/ori_term/oriterm_mux/src/backend/embedded/mod.rs:375), [`oriterm_mux/src/pane/io_thread/mod.rs:201`](/home/eric/projects/ori_term/oriterm_mux/src/pane/io_thread/mod.rs:201), [`plans/threaded-io/section-06-remaining-ops.md:58`](/home/eric/projects/ori_term/plans/threaded-io/section-06-remaining-ops.md:58) — Section 04 makes the renderer prefer IO-thread snapshots even though scroll, theme, cursor-shape, search, prompt-nav, and related operations still mutate only the old `FairMutex<Term>`.
  Evidence: `EmbeddedMux` still applies those operations by locking `pane.terminal()` on the old term, while `PaneIoThread::handle_command()` is explicitly a placeholder until sections 05-06. Once fresh PTY output arrives, `refresh_pane_snapshot()` consumes the newer IO-thread snapshot and overwrites the render cache with a state that never saw the user operation. In practice this means scrollback position, search navigation/highlights, cursor shape, theme changes, and similar state can snap back or render stale data whenever output continues after the action.
  **Required fix:** keep using the lock-based snapshot path for panes with any not-yet-migrated state, or block IO-thread snapshot preference until the section 05-06 command migrations land for every operation that can affect rendering.
  **Fix:** Implemented `handle_command` for display-affecting operations (Resize, ScrollDisplay, ScrollToBottom, ScrollToPreviousPrompt, ScrollToNextPrompt, SetTheme, SetCursorShape, MarkAllDirty, SetImageConfig). EmbeddedMux now sends these commands to the IO thread alongside mutating the old Term. Search, text extraction, and mark mode remain deferred to sections 05-06.
- [x] `[TPR-04-002][medium]` [`oriterm_mux/src/server/snapshot.rs:272`](/home/eric/projects/ori_term/oriterm_mux/src/server/snapshot.rs:272), [`oriterm_mux/src/server/snapshot.rs:291`](/home/eric/projects/ori_term/oriterm_mux/src/server/snapshot.rs:291), [`oriterm_mux/src/backend/embedded/mod.rs:384`](/home/eric/projects/ori_term/oriterm_mux/src/backend/embedded/mod.rs:384), [`oriterm_mux/src/server/snapshot.rs:48`](/home/eric/projects/ori_term/oriterm_mux/src/server/snapshot.rs:48), [`oriterm/src/app/cursor_hover.rs:28`](/home/eric/projects/ori_term/oriterm/src/app/cursor_hover.rs:28), [`oriterm/src/app/cursor_hover.rs:89`](/home/eric/projects/ori_term/oriterm/src/app/cursor_hover.rs:89) — the new `fill_wire_cells_from_renderable()` path strips OSC 8 hyperlink URIs from every `PaneSnapshot`.
  Evidence: the old lock-based path resolves `hyperlink_uri_at(...)` from the grid, but the IO-thread snapshot path hardcodes `hyperlink_uri: None` for all cells. `PaneSnapshot` drives both embedded snapshot consumers and daemon snapshots, and the app's hover/click handling checks `wire_cell.hyperlink_uri` to surface explicit hyperlinks. After the first zero-lock snapshot is consumed, explicit OSC 8 links stop being hoverable/clickable and lose hyperlink underline state unless the text also happens to look like an implicit URL.
  **Required fix:** preserve hyperlink payloads in the zero-lock path before enabling it for `PaneSnapshot` consumers, or keep snapshot building on the lock-based path for hyperlink-bearing cells until the IO thread can publish the URI data.
  **Fix:** Added `hyperlink_uri: Option<String>` to `RenderableCell`, populated during `renderable_content_into()` from `cell.extra.hyperlink.uri`. The IO-thread snapshot now carries the URI. `fill_wire_cells_from_renderable()` reads from `cell.hyperlink_uri` instead of hardcoding `None`.

---

## 04.N Completion Checklist

- [x] `refresh_pane_snapshot()` reads from `SnapshotDoubleBuffer::swap_front()`, not `pane.terminal().lock()`
- [x] `swap_renderable_content()` works with IO-thread-produced snapshots
- [x] Daemon mode snapshot path updated to read from IO thread
- [x] Old `PtyEventLoop` VTE parsing still active (dual-Term during transition)
- [x] Two `Term` instances per pane: IO thread (render) + FairMutex (operations)
- [x] Terminal renders correctly from IO-thread snapshots
- [x] `FairMutex::lock()` fallback in render path only when IO thread hasn't produced a snapshot yet
- [x] Damage tracking works through IO-thread snapshots
- [x] `timeout 150 cargo test -p oriterm_mux` passes
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `/tpr-review` passed

**Exit Criteria:** The render pipeline prefers IO-thread-produced snapshots via `swap_io_snapshot()`. Falls back to the lock-based path during the race window where the old PtyEventLoop has parsed bytes but the IO thread hasn't yet produced its snapshot. After warmup, the IO thread path handles the vast majority of frames. The old parsing path remains active for non-render operations (scroll, search, etc.) until sections 05-06 migrate them.
