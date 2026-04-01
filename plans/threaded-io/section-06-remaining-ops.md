---
section: "06"
title: "Remaining State Operations"
status: not-started
reviewed: true
goal: "Migrate all remaining terminal state operations (scroll, search, theme, text extraction, etc.) to IO thread commands — eliminating all direct FairMutex access from the main thread"
inspired_by:
  - "Ghostty termio/message.zig (all terminal mutations flow through typed messages)"
depends_on: ["05"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "06.1"
    title: "Scroll Operations"
    status: not-started
  - id: "06.2"
    title: "Theme & Visual Config"
    status: not-started
  - id: "06.3"
    title: "Text Extraction (Clipboard)"
    status: not-started
  - id: "06.4"
    title: "Search Operations"
    status: not-started
  - id: "06.5"
    title: "Mark Mode & Prompt Navigation"
    status: not-started
  - id: "06.6"
    title: "Daemon Server Dispatch Migration"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Remaining State Operations

**Status:** Not Started
**Goal:** Migrate all remaining operations that currently lock `Arc<FairMutex<Term>>` on the main thread to IO thread commands. After this section, no code outside the IO thread touches `Term` or `Grid`.

**Context:** Sections 04-05 migrated the render and resize paths. Several other operations still lock the terminal from the main thread: scroll, theme changes, cursor shape, text extraction, search, mark mode, and prompt navigation. Each becomes a `PaneIoCommand` variant. Fire-and-forget operations (scroll, theme) need no response. Request-response operations (text extraction) use a reply channel.

**Reference implementations:**
- **Ghostty** `src/termio/message.zig`: All mutations are message variants — focus, scroll, color_change, etc.

**Depends on:** Section 05 (resize migrated to IO thread; `PtyControl` moved to IO thread).

---

## 06.1 Scroll Operations

**File(s):** `oriterm_mux/src/backend/embedded/mod.rs`, `oriterm_mux/src/pane/mod.rs`

Scroll operations currently call `pane.scroll_display()` / `pane.scroll_to_bottom()` which lock the terminal.

- [ ] Route `EmbeddedMux::scroll_display()` through IO command:
  ```rust
  fn scroll_display(&mut self, pane_id: PaneId, delta: isize) {
      if let Some(pane) = self.panes.get(&pane_id) {
          pane.send_io_command(PaneIoCommand::ScrollDisplay(delta));
      }
      self.snapshot_dirty.insert(pane_id);
  }
  ```

- [ ] Route `scroll_to_bottom()`:
  ```rust
  fn scroll_to_bottom(&mut self, pane_id: PaneId) {
      if let Some(pane) = self.panes.get(&pane_id) {
          pane.send_io_command(PaneIoCommand::ScrollToBottom);
      }
      self.snapshot_dirty.insert(pane_id);
  }
  ```

- [ ] Route `scroll_to_previous_prompt()` and `scroll_to_next_prompt()`:
  These currently return `bool` (whether scrolling happened). With async commands, we can't return synchronously. The UI already ignores the return value (verified: `action_dispatch.rs:164,175` discards it).

- [ ] **Trait signature change**: Update `MuxBackend::scroll_to_previous_prompt()` and `scroll_to_next_prompt()` return type from `bool` to `()`. Update ALL three implementors simultaneously: `EmbeddedMux` (`backend/embedded/mod.rs`), `DaemonMux` (`backend/client/rpc_methods.rs`), and `InProcessMux` if applicable. Update any callers in `oriterm/src/app/` that check the return value.

- [ ] IO thread handlers:
  ```rust
  PaneIoCommand::ScrollDisplay(delta) => {
      self.terminal.grid_mut().scroll_display(delta);
  }
  PaneIoCommand::ScrollToBottom => {
      if self.terminal.grid().display_offset() > 0 {
          self.terminal.grid_mut().scroll_display(isize::MIN);
      }
  }
  PaneIoCommand::ScrollToPreviousPrompt => {
      self.terminal.scroll_to_previous_prompt();
  }
  PaneIoCommand::ScrollToNextPrompt => {
      self.terminal.scroll_to_next_prompt();
  }
  ```

---

## 06.2 Theme & Visual Config

**File(s):** `oriterm_mux/src/backend/embedded/mod.rs`

- [ ] Route `set_pane_theme()`:
  ```rust
  fn set_pane_theme(&mut self, pane_id: PaneId, theme: Theme, palette: Palette) {
      if let Some(pane) = self.panes.get(&pane_id) {
          pane.send_io_command(PaneIoCommand::SetTheme(theme, palette));
      }
      self.snapshot_dirty.insert(pane_id);
  }
  ```

- [ ] Route `set_cursor_shape()`:
  ```rust
  fn set_cursor_shape(&mut self, pane_id: PaneId, shape: CursorShape) {
      if let Some(pane) = self.panes.get(&pane_id) {
          pane.send_io_command(PaneIoCommand::SetCursorShape(shape));
      }
      self.snapshot_dirty.insert(pane_id);
  }
  ```

- [ ] Route `mark_all_dirty()`:
  ```rust
  fn mark_all_dirty(&mut self, pane_id: PaneId) {
      if let Some(pane) = self.panes.get(&pane_id) {
          pane.send_io_command(PaneIoCommand::MarkAllDirty);
      }
      self.snapshot_dirty.insert(pane_id);
  }
  ```

- [ ] Route `set_image_config()`:
  ```rust
  fn set_image_config(&mut self, pane_id: PaneId, config: ImageConfig) {
      if let Some(pane) = self.panes.get(&pane_id) {
          pane.send_io_command(PaneIoCommand::SetImageConfig(config));
      }
  }
  ```

- [ ] IO thread handlers for each (straightforward — call the same methods on `self.terminal` that were previously called under lock).

- [ ] Route `Reset` command (if/when a terminal reset API is added to `MuxBackend`):
  ```rust
  PaneIoCommand::Reset => {
      self.terminal.reset();
  }
  ```
  Note: No `MuxBackend::reset()` method exists today. The project has `dead_code = "deny"`, so the `Reset` variant MUST either be used by a `MuxBackend::reset()` method or removed from `PaneIoCommand` in section 01. Do not leave an unused variant -- clippy will reject it.

---

## 06.3 Text Extraction (Clipboard)

**File(s):** `oriterm_mux/src/backend/embedded/mod.rs`

Text extraction needs a response — it reads grid cells and returns a String. This uses a request-response pattern via a reply channel.

- [ ] Route `extract_text()`:
  ```rust
  fn extract_text(&mut self, pane_id: PaneId, sel: &Selection) -> Option<String> {
      let pane = self.panes.get(&pane_id)?;
      let (tx, rx) = crossbeam_channel::bounded(1);
      pane.send_io_command(PaneIoCommand::ExtractText {
          selection: sel.clone(),
          reply: tx,
      });
      // Block until the IO thread responds or the pane is closed
      // (channel disconnects). No fixed timeout — the IO thread
      // processes this between parse chunks, so even during a long
      // reflow the reply arrives within one 64KB chunk boundary.
      rx.recv().ok()?
  }
  ```

- [ ] Route `extract_html()` similarly with its own reply channel.

- [ ] IO thread handlers:
  ```rust
  PaneIoCommand::ExtractText { selection, reply } => {
      let text = oriterm_core::selection::extract_text(
          self.terminal.grid(), &selection
      );
      let result = if text.is_empty() { None } else { Some(text) };
      let _ = reply.send(result);
  }
  ```

- [ ] **Use `recv_timeout()` as a safety net.** While the IO thread should respond within one 64KB parse chunk (~microseconds), a hard block on the main winit thread risks freezing the entire UI if the IO thread is stuck. Use `crossbeam_channel::Receiver::recv_timeout(Duration::from_millis(100))`. If timeout fires, return `None` — the user can retry the copy. If the pane is closed, the `Sender` is dropped and `recv()` returns `Err(RecvTimeoutError::Disconnected)`, which is the correct cancellation path.
  ```rust
  use std::time::Duration;
  rx.recv_timeout(Duration::from_millis(100)).ok().flatten()
  ```


- [ ] `/tpr-review` checkpoint

### Tests (06.1-06.3)

**File:** `oriterm_mux/src/pane/io_thread/tests.rs` (extend)

- [ ] `test_scroll_display_command` — send `ScrollDisplay(5)`, assert `terminal.grid().display_offset()` is 5 (after producing enough scrollback).
- [ ] `test_scroll_to_bottom_command` — scroll up, send `ScrollToBottom`, assert `display_offset == 0`.
- [ ] `test_scroll_to_previous_prompt_command` — set up prompt markers, send `ScrollToPreviousPrompt`. Assert viewport scrolled to the prompt line.
- [ ] `test_set_theme_command` — send `SetTheme(dark_theme, dark_palette)`. Assert `terminal.palette()` matches the new palette.
- [ ] `test_set_cursor_shape_command` — send `SetCursorShape(Block)`. Assert `terminal.cursor_shape()` is `Block`.
- [ ] `test_mark_all_dirty_command` — send `MarkAllDirty`. Assert all lines are dirty in the terminal's damage tracker.
- [ ] `test_extract_text_reply` — write "hello world" to the terminal, send `ExtractText` with a selection covering the text and a reply channel. Assert `rx.recv()` returns `Some("hello world")`.
- [ ] `test_extract_text_timeout_safety` — shut down the IO thread (send `Shutdown`, join it), then send `ExtractText` on the now-disconnected channel. Assert `rx.recv_timeout(100ms)` returns `Err` (not a hang). Verifies the main thread doesn't block forever on a dead IO thread.
- [ ] `test_extract_html_reply` — write styled text, send `ExtractHtml`. Assert reply contains both HTML and plain text strings.

---

## 06.4 Search Operations

**File(s):** `oriterm_mux/src/backend/embedded/mod.rs`

Search state currently lives on `Pane` (not `Term`), but `search_set_query()` locks the terminal to read the grid. This needs to move to the IO thread.

- [ ] Move `SearchState` from `Pane` to `PaneIoThread`. Rationale: `search_set_query()` currently calls `pane.terminal().lock()` to read the grid (see `EmbeddedMux::search_set_query()` in `embedded/mod.rs:184-193`). The grid will be on the IO thread, so search execution must happen there.

- [ ] The search commands are already defined in `PaneIoCommand` (section 01). Route all search operations through them:
  - `EmbeddedMux::open_search()` → `PaneIoCommand::OpenSearch`
  - `EmbeddedMux::close_search()` → `PaneIoCommand::CloseSearch`
  - `EmbeddedMux::search_set_query()` → `PaneIoCommand::SearchSetQuery(query)`
  - `EmbeddedMux::search_next_match()` → `PaneIoCommand::SearchNextMatch`
  - `EmbeddedMux::search_prev_match()` → `PaneIoCommand::SearchPrevMatch`

- [ ] IO thread handlers: the IO thread owns `SearchState` as a field on `PaneIoThread`. `SearchSetQuery` calls `search.set_query(query, self.terminal.grid())` directly — no lock needed since the IO thread owns both.

- [ ] **Search results in snapshots**: Search matches, focused index, and query must be included in `RenderableContent` so both the renderer AND daemon snapshot metadata can display them without needing `pane.search()`. Add these fields to `RenderableContent` (in `oriterm_core/src/term/renderable/mod.rs`):
  ```rust
  pub search_active: bool,
  pub search_query: String,
  pub search_matches: Vec<SearchMatch>,  // compacted match positions
  pub search_focused: Option<u32>,       // focused match index
  pub search_total_matches: u32,
  ```
  The IO thread fills these during `produce_snapshot()` by reading its `SearchState`.
- [ ] Update `fill_snapshot_metadata()` in `oriterm_mux/src/server/snapshot.rs` to read search state from `RenderableContent` instead of `pane.search()`. The search block (lines 214-240) switches from `pane.search().matches()` etc. to `render_buf.search_matches`, `render_buf.search_query`, etc.

- [ ] `is_search_active()` — keep a `search_active: AtomicBool` on `Pane` that the IO thread updates via `OpenSearch`/`CloseSearch` commands. This allows the main thread to query search state without a reply channel. Alternatively, read from the latest snapshot.

---

## 06.5 Mark Mode & Prompt Navigation

**File(s):** `oriterm_mux/src/pane/mod.rs`

Mark mode and prompt navigation currently lock the terminal for cursor position reads.

- [ ] `enter_mark_mode()` currently calls `self.scroll_to_bottom()` (locks terminal) then locks again to read cursor position (see `pane/mod.rs:363-379`). Both operations must happen atomically on the IO thread:
  ```rust
  PaneIoCommand::EnterMarkMode { reply } => {
      // Scroll to bottom first (same as current enter_mark_mode).
      if self.terminal.grid().display_offset() > 0 {
          self.terminal.grid_mut().scroll_display(isize::MIN);
      }
      // Read cursor position.
      let g = self.terminal.grid();
      let cursor = g.cursor();
      let abs_row = g.scrollback().len() + cursor.line();
      let stable = StableRowIndex::from_absolute(g, abs_row);
      let mc = MarkCursor { row: stable, col: cursor.col().0 };
      let _ = reply.send(mc);
  }
  ```
  The main thread sends the command, blocks on the reply (with `recv_timeout(100ms)`), and stores the result on `Pane.mark_cursor`.

- [ ] `exit_mark_mode()` and `set_mark_cursor()` are Pane-local (no terminal access needed) — no change.

- [ ] Selection operations: `Pane.selection` is Pane-local. No migration needed — selection already works from snapshot data.

- [ ] `is_selection_dirty()` / `clear_selection_dirty()` currently lock the terminal (see `embedded/mod.rs:392-400`). Add a `selection_dirty: AtomicBool` to `Pane` that the IO thread sets when `Term::selection_dirty` becomes true. The IO thread checks this after each VTE parse cycle and updates the atomic. The main thread reads/clears the atomic without locking.

- [ ] `check_selection_invalidation()` (in `pane/selection.rs:39-53`) locks the terminal to read `is_selection_dirty()`. After migration, this reads the `selection_dirty` atomic instead.

- [ ] `command_output_selection()` and `command_input_selection()` (in `pane/selection.rs:92-131`) lock the terminal to read grid state and prompt markers. These need to become IO thread commands with reply channels:
  ```rust
  PaneIoCommand::SelectCommandOutput {
      reply: crossbeam_channel::Sender<Option<Selection>>,
  },
  PaneIoCommand::SelectCommandInput {
      reply: crossbeam_channel::Sender<Option<Selection>>,
  },
  ```
  Add these variants to `PaneIoCommand` in section 01.

---

## 06.6 Daemon Server Dispatch Migration


**File(s):** `oriterm_mux/src/server/dispatch/mod.rs`

The daemon server dispatch has the same pattern as `EmbeddedMux` — it locks the terminal for theme, cursor, dirty, resize, search, and text extraction. All must route through `Pane::send_io_command()` instead.

- [ ] Audit all `pane.terminal().lock()` calls in `server/dispatch/mod.rs` (currently 7 call sites: lines 140, 164, 172, 196+199, 234, 286, 303)
- [ ] Route each through the same `Pane::send_io_command()` helper that EmbeddedMux uses
- [ ] Request-response calls (text extraction, search query) use the same `recv_timeout()` pattern as EmbeddedMux
- [ ] Verify `SnapshotCache::build()` no longer locks terminal (migrated in section 04.2)

### Tests (06.4-06.6)

**File:** `oriterm_mux/src/pane/io_thread/tests.rs` (extend)

- [ ] `test_open_close_search` — send `OpenSearch`, assert IO thread's `SearchState` is `Some`. Send `CloseSearch`, assert `None`.
- [ ] `test_search_set_query_finds_matches` — write "foo bar foo" to terminal, send `OpenSearch` then `SearchSetQuery("foo")`. Assert the search state has 2 matches.
- [ ] `test_search_next_prev_match` — set up 3 matches. Send `SearchNextMatch` twice. Assert focused index advances. Send `SearchPrevMatch`, assert it goes back.
- [ ] `test_search_results_in_snapshot` — set query, produce snapshot. Assert `RenderableContent.search_active == true`, `search_total_matches > 0`, `search_query == "foo"`.
- [ ] `test_enter_mark_mode_reply` — send `EnterMarkMode` with reply channel. Assert reply contains a `MarkCursor` with valid row/col. Assert the terminal was scrolled to bottom first.
- [ ] `test_selection_dirty_atomic` — write output that invalidates selection. Assert the `selection_dirty` `AtomicBool` on `Pane` is set to `true` by the IO thread.
- [ ] `test_select_command_output_reply` — set up prompt markers and command output zone. Send `SelectCommandOutput` with reply. Assert reply contains a valid `Selection` covering the output zone.
- [ ] `test_select_command_input_reply` — same as above but for `SelectCommandInput`.

**File:** `oriterm_mux/src/backend/embedded/tests.rs` (extend)

- [ ] `test_embedded_search_routes_through_io` — call `EmbeddedMux::search_set_query()`. Assert no `terminal().lock()` call occurs (the old lock path is removed). Verify search results appear in the next snapshot.
- [ ] `test_embedded_scroll_routes_through_io` — call `EmbeddedMux::scroll_display()`. Assert the command was sent to the IO thread (not a direct lock).

- [ ] **Define `SearchMatch` type in `oriterm_core`**: The `search_matches: Vec<SearchMatch>` field added to `RenderableContent` needs a `SearchMatch` struct. Define it in `oriterm_core/src/search.rs` or alongside `RenderableContent`. It should contain at minimum: `start_row: usize`, `start_col: usize`, `end_row: usize`, `end_col: usize` (compacted match positions for rendering highlights).

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 06.N Completion Checklist

- [ ] `scroll_display()`, `scroll_to_bottom()` route through IO commands
- [ ] `scroll_to_previous_prompt()`, `scroll_to_next_prompt()` route through IO commands
- [ ] `set_pane_theme()`, `set_cursor_shape()`, `mark_all_dirty()` route through IO commands
- [ ] `set_image_config()` routes through IO command
- [ ] `extract_text()`, `extract_html()` use request-response via reply channel
- [ ] Search operations (open, close, query, next, prev) route through IO commands
- [ ] Search state included in snapshots for rendering
- [ ] Mark mode cursor read uses IO command with reply
- [ ] Selection dirty state accessible without terminal lock
- [ ] `command_output_selection()`, `command_input_selection()` use IO commands with reply
- [ ] No remaining `pane.terminal().lock()` calls in `EmbeddedMux` methods
- [ ] No remaining `pane.terminal().lock()` calls in `server/dispatch` handlers
- [ ] `timeout 150 cargo test -p oriterm_mux` passes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** Every `MuxBackend` method, every `server/dispatch` handler, and every `Pane` method that previously locked the terminal now routes through `PaneIoCommand`. `grep -rn "terminal().lock()\|self.terminal.lock()" oriterm_mux/src/` returns zero results outside tests. Actual call sites to migrate (verified): 8 in `backend/embedded/mod.rs`, 7 in `server/dispatch/mod.rs` (6 `.lock()` + 1 `.clone().lock()` pattern), 3 in `server/snapshot.rs`, 5 in `pane/mod.rs`, 3 in `pane/selection.rs` — **26 total**. The pane-internal locks (scroll_to_bottom, scroll_display, resize_grid, enter_mark_mode, prompt nav, selection checks, command zone selection) are handled by sections 05-06 making those Pane methods delegate to `send_io_command()`, then removed entirely in section 07.
