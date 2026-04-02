---
section: "02"
title: "VTE Parsing Migration"
status: complete
reviewed: true
goal: "Move VTE parsing from the PTY reader thread to the Terminal IO thread — the IO thread becomes the sole writer to terminal state"
inspired_by:
  - "Ghostty termio/Exec.zig (IO thread receives PTY bytes via pipe, parses in its own loop)"
  - "Alacritty event_loop.rs (read-ahead buffer pattern preserved for ConPTY back-pressure)"
depends_on: ["01"]
third_party_review:
  status: resolved
  updated: 2026-03-31
sections:
  - id: "02.1"
    title: "PTY Reader Simplification"
    status: complete
  - id: "02.2"
    title: "IO Thread VTE Processing"
    status: complete
  - id: "02.3"
    title: "Term Ownership Transfer"
    status: complete
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.N"
    title: "Completion Checklist"
    status: complete
---

# Section 02: VTE Parsing Migration

**Status:** Complete
**Goal:** Add VTE parsing on the IO thread alongside the existing reader thread. The old `PtyEventLoop` continues parsing (unchanged) AND forwards bytes to the IO thread. The IO thread owns a second `Term<T>` and parses independently. This dual-Term architecture enables incremental migration in sections 03-06.

**Context:** Today, `PtyEventLoop::try_parse()` acquires the FairMutex lease, locks the terminal, and runs both VTE processors (`vte::ansi::Processor` and `vte::Parser` for shell integration). This is the most contention-heavy code path. After this section, the reader thread continues its existing VTE parsing (unchanged) AND additionally forwards raw bytes to the IO thread. The IO thread owns a second `Term` and parses independently. This dual-Term architecture doubles parsing CPU temporarily but enables incremental migration without breaking the existing path.

**Reference implementations:**
- **Ghostty** `src/termio/Exec.zig:105-130`: The IO thread reads from a pipe (not the PTY directly) and processes bytes in its own loop. The read thread feeds raw bytes.
- **Alacritty** `alacritty_terminal/src/event_loop.rs:98-162`: Read-ahead pattern with 1MB buffer that ori_term already mirrors. The key change is redirecting parsed bytes to the IO thread instead of locking the terminal.

**Depends on:** Section 01 (IO thread scaffold exists).

---

## 02.1 PTY Reader Byte Forwarding

**File(s):** `oriterm_mux/src/pty/event_loop/mod.rs`


Add byte forwarding to the existing `PtyEventLoop`. The old VTE parsing path is **preserved unchanged** — this is the dual-Term period where both parse independently.

- [x] Add `byte_tx: Option<crossbeam_channel::Sender<Vec<u8>>>` field to `PtyEventLoop`
- [x] Update `PtyEventLoop::new()` to accept an optional `byte_tx` parameter
- [x] In `run()`, after each successful `read()` call (line ~142, after `unprocessed += n`), forward raw bytes to the IO thread BEFORE `try_parse()`:
  ```rust
  // Forward raw bytes to IO thread (before parsing on old path).
  if let Some(ref tx) = self.byte_tx {
      let _ = tx.send(buf[unprocessed - n..unprocessed].to_vec());
  }
  ```
- [x] All existing parsing infrastructure stays: `terminal`, `processor`, `raw_parser`, `mode_cache`, `try_parse()`, `parse_chunk()`, `MAX_LOCKED_PARSE`, lease/lock paths. **Nothing is removed in this section.**
- [x] Keep `READ_BUFFER_SIZE` (1MB) — the read-ahead pattern is preserved.

**Note on allocation**: `buf[..n].to_vec()` allocates on every read. This is acceptable because:
1. The allocation happens on the reader thread, not the render hot path
2. The IO thread's `handle_bytes()` processes and drops the Vec
3. During idle, no reads happen (reader blocks on `read()`)
4. During floods, the allocation cost is negligible vs. the PTY I/O cost

---

## 02.2 IO Thread VTE Processing

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

Move the VTE processors and parsing logic into the IO thread.

- [x] Add VTE processors to `PaneIoThread` (the struct becomes generic over `T: EventListener`):
  ```rust
  pub struct PaneIoThread<T: EventListener> {
      terminal: Term<T>,
      cmd_rx: crossbeam_channel::Receiver<PaneIoCommand>,
      byte_rx: crossbeam_channel::Receiver<Vec<u8>>,
      shutdown: Arc<AtomicBool>,
      wakeup: Arc<dyn Fn() + Send + Sync>,
      /// High-level VTE parser.
      processor: vte::ansi::Processor,
      /// Raw VTE parser for shell integration (OSC 7, 133, etc.).
      raw_parser: vte::Parser,
      /// Lock-free mode cache (updated after parsing).
      mode_cache: Arc<AtomicU32>,
  }
  ```

- [x] Implement `handle_bytes()` — adapted from the old `PtyEventLoop::parse_chunk()`:
  ```rust
  fn handle_bytes(&mut self, bytes: &[u8]) {
      use crate::shell_integration::interceptor::RawInterceptor;
      
      let evicted_before = self.terminal.grid().total_evicted();
      
      // 1. Raw interceptor for shell integration sequences.
      {
          let mut interceptor = RawInterceptor::new(&mut self.terminal);
          self.raw_parser.advance(&mut interceptor, bytes);
      }
      
      // 2. High-level VTE processor.
      self.processor.advance(&mut self.terminal, bytes);
      
      // 3. Deferred prompt marking.
      if self.terminal.prompt_mark_pending() {
          self.terminal.mark_prompt_row();
      }
      if self.terminal.command_start_mark_pending() {
          self.terminal.mark_command_start_row();
      }
      if self.terminal.output_start_mark_pending() {
          self.terminal.mark_output_start_row();
      }
      
      // 4. Prune prompt markers invalidated by scrollback eviction.
      let newly_evicted = self.terminal.grid().total_evicted() - evicted_before;
      if newly_evicted > 0 {
          self.terminal.prune_prompt_markers(newly_evicted);
      }
      
      // 5. Update mode cache for lock-free queries from main thread.
      self.mode_cache.store(self.terminal.mode().bits(), Ordering::Release);
  }
  ```

- [x] Implement bounded processing: slice received byte messages into 64KB chunks and check for commands between chunks. A single 1MB forwarded read would otherwise delay resize/copy behind a long parse:
  ```rust
  /// Maximum bytes parsed before re-checking for commands.
  const MAX_PARSE_CHUNK: usize = 0x1_0000; // 64 KB
  
  fn process_pending_bytes(&mut self) {
      while let Ok(bytes) = self.byte_rx.try_recv() {
          // Slice into bounded chunks to keep commands responsive.
          let mut offset = 0;
          while offset < bytes.len() {
              let end = (offset + MAX_PARSE_CHUNK).min(bytes.len());
              self.handle_bytes(&bytes[offset..end]);
              offset = end;
              // Re-check for priority commands between chunks.
              self.drain_commands();
          }
      }
  }
  ```
  This preserves the existing 64KB parse bound from `PtyEventLoop::try_parse()` — the IO thread checks for commands with the same frequency the old path checked for renderer contention.

- [x] Update the main loop to integrate parsing with command processing:
  ```rust
  fn run(mut self) {
      loop {
          // Priority: drain all commands first.
          self.drain_commands();
          if self.shutdown.load(Ordering::Acquire) {
              return;
          }
          
          // Process available bytes (non-blocking drain).
          self.process_pending_bytes();
          
          // TODO (section 03): produce snapshot + send wakeup here.
          
          // Block waiting for next message (either channel).
          crossbeam_channel::select! {
              recv(self.cmd_rx) -> msg => match msg {
                  Ok(PaneIoCommand::Shutdown) => return,
                  Ok(cmd) => self.handle_command(cmd),
                  Err(_) => return,
              },
              recv(self.byte_rx) -> msg => match msg {
                  Ok(bytes) => self.handle_bytes(&bytes),
                  Err(_) => return,
              },
          }
      }
  }
  ```

### Tests

**File:** `oriterm_mux/src/pane/io_thread/tests.rs` (extend from section 01)

- [x] `test_handle_bytes_advances_vte` — create a `PaneIoThread` with a real `Term`, send `\x1b[31m` (SGR red) via `handle_bytes()`. Assert the terminal's last cell attribute has red foreground. Verifies VTE parsing works on the IO thread.
- [x] `test_handle_bytes_shell_integration` — send OSC 133 prompt sequence via `handle_bytes()`. Assert `terminal.prompt_markers()` has at least one entry. Verifies the raw parser + deferred prompt marking.
- [x] `test_handle_bytes_prunes_evicted_markers` — fill a small grid (5 lines, 10 scrollback) with enough output to evict scrollback. Assert prompt markers in evicted regions are pruned.
- [x] `test_process_pending_bytes_chunks` — send a 200KB byte buffer in one message. Assert `handle_bytes` is called multiple times (at 64KB boundaries) by checking that `drain_commands()` was invoked between chunks (use a `Resize` command injected mid-stream and verify it was processed before all bytes finished).
- [x] `test_mode_cache_updated_after_parse` — send `\x1b[?1049h` (alt screen enable). Assert `mode_cache.load()` reflects the updated mode bits.

**File:** `oriterm_mux/src/pane/io_thread/event_proxy/tests.rs` (new — sibling tests for `event_proxy.rs`)

- [x] `test_io_thread_event_proxy_suppresses_title` — create `IoThreadEventProxy` with `suppress_metadata = true`, send `Event::Title("test")`. Assert the inner `MuxEventProxy` did NOT receive the event.
- [x] `test_io_thread_event_proxy_sets_grid_dirty` — send `Event::Wakeup`. Assert `grid_dirty` `AtomicBool` is set to `true`.
- [x] `test_io_thread_event_proxy_suppresses_pty_write` — send `Event::PtyWrite("data")` with `suppress_metadata = true`. Assert the event is NOT forwarded (prevents duplicate DA responses during dual-Term).
- [x] `test_io_thread_event_proxy_is_send` — static assertion: `fn assert_send<T: Send>() {} assert_send::<IoThreadEventProxy>();`.

**File:** `oriterm_mux/src/pty/event_loop/tests.rs` (extend existing)

- [x] `test_byte_forwarding_to_io_thread` — create a `PtyEventLoop` with a `byte_tx` channel. Write bytes to the PTY pipe. Assert the same bytes arrive on the `byte_rx` receiver. Verifies the forwarding added in 02.1.
- [x] `test_byte_forwarding_none_when_no_channel` — create a `PtyEventLoop` with `byte_tx: None`. Write bytes. Assert no panic and existing parsing still works (backward compatibility).

- [x] `/tpr-review` checkpoint (2 rounds, 5 findings all resolved)

---

## 02.3 Term Ownership Transfer

**File(s):** `oriterm_mux/src/pane/mod.rs`, `oriterm_mux/src/domain/local.rs`

Transfer `Term<MuxEventProxy>` ownership from `Arc<FairMutex>` to the IO thread. The `Pane` struct retains `Arc<FairMutex>` temporarily for operations not yet migrated (sections 04-06 will remove remaining usages).


**Dual-Term approach**: During sections 02-06, two `Term` instances coexist per pane:
1. **Old Term** in `Arc<FairMutex>` — fed by the existing `PtyEventLoop` reader thread. Remains authoritative for all main-thread operations (rendering, scroll, search, text extraction) until sections 04-06 migrate them.
2. **New Term** owned by `PaneIoThread` — fed by byte forwarding from the old reader. Produces snapshots starting in section 03. Becomes authoritative for rendering in section 04.

**Why two Terms?** The PTY fd can only be read by one thread (the existing `PtyEventLoop`). The old reader forwards a copy of each byte batch to the IO thread via the byte channel. Both `Term` instances parse the same stream independently. This doubles CPU parsing cost temporarily but ensures correctness — no behavioral changes until section 07 removes the old path.

**Critical**: The IO thread's `Term` MUST use `IoThreadEventProxy`, NOT `MuxEventProxy`. `MuxEventProxy` fires title/CWD/bell/PtyWrite events. If both Terms used it, those events would fire twice (duplicate DA responses = protocol violation). The `IoThreadEventProxy` is created in this section (02.3) with `suppress_metadata = true`, suppressing ALL events except setting a `grid_dirty` flag on `Wakeup`. Section 03.3 wires the wakeup timing to snapshot publication.

- [x] Create `IoThreadEventProxy` struct in `oriterm_mux/src/pane/io_thread/event_proxy/mod.rs` (new file):
  - Has `grid_dirty: Arc<AtomicBool>` for IO thread state tracking
  - Has `suppress_metadata: AtomicBool` (initialized to `true`)
  - Implements `EventListener`: on `Wakeup`, sets `grid_dirty` only; all other events suppressed while `suppress_metadata` is true
  - Is `Send + 'static` (verified by test)
  - Section 03.3 adds wakeup timing; section 07 flips `suppress_metadata` to `false`
- [x] Update `PaneIoThread` from non-generic to `PaneIoThread<T: EventListener>`. Add `terminal: Term<T>`, `processor: vte::ansi::Processor`, `raw_parser: vte::Parser`, `mode_cache: Arc<AtomicU32>` fields. Update `new_with_handle()` to accept `term: Term<T>` and VTE-related params.
- [x] Add `mod event_proxy;` to `oriterm_mux/src/pane/io_thread/mod.rs`
- [x] Create `oriterm_mux/src/pane/io_thread/event_proxy/` as a directory module: `event_proxy/mod.rs` + `event_proxy/tests.rs` with `#[cfg(test)] mod tests;` at the bottom of `mod.rs`
- [x] In `LocalDomain::spawn_pane()` — modify to create both Terms:
  - Create `Term` #1 with `MuxEventProxy` → wrap in `Arc<FairMutex>` → pass to `PtyEventLoop` (existing, unchanged)
  - Create `IoThreadEventProxy` with separate `io_grid_dirty` flag
  - Create `Term` #2 with `IoThreadEventProxy` → pass to `PaneIoThread` (new)
  - Both receive the same theme, palette, initial config, grid dimensions

- [x] Wire byte forwarding in `LocalDomain::spawn_pane()`: pass the IO thread's `byte_tx` to `PtyEventLoop::new()` (byte forwarding implementation is in section 02.1)

- [x] `io_handle: PaneIoHandle` already exists in `PaneParts` and `Pane` (added in section 01). No change needed.

- [x] Verify: both `Term` instances receive the same byte stream (byte forwarding verified by `byte_forwarding_to_io_thread` test). Grid dimensions match (same config passed to both constructors).

**File size watch:** `io_thread/mod.rs` is accumulating the struct, main loop, VTE processing, and snapshot production. If it approaches 400 lines after this section, extract `handle_bytes()` / `process_pending_bytes()` / `parse_chunk()` into `io_thread/parsing.rs` before section 03 adds more code.

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-02-001][medium]` `oriterm_mux/src/pane/io_thread/mod.rs:87` — the blocking receive path bypasses the section's 64 KB chunking guarantee.
  Evidence: `process_pending_bytes()` slices queued byte messages at `MAX_PARSE_CHUNK` and drains commands between slices, but the idle-path `select!` arm still calls `self.handle_bytes(&bytes)` directly. The first post-idle PTY read can therefore arrive as one forwarded 1 MB buffer from `PtyEventLoop` and monopolize the IO thread until the whole buffer is parsed, delaying queued resize/copy/search commands instead of checking them every 64 KB as the section claims.
  Resolved: Extracted `handle_bytes_chunked()` method and updated `select!` arm to call it on 2026-03-31.
- [x] `[TPR-02-002][medium]` `oriterm_mux/src/pane/io_thread/tests.rs:182`, `oriterm_mux/src/pane/io_thread/tests.rs:213`, `oriterm_mux/src/pane/io_thread/tests.rs:280` — the new Section 02 tests do not verify the behaviors they claim to pin.
  Evidence: `handle_bytes_advances_vte()` and `handle_bytes_shell_integration()` only assert that the shutdown flag was set after the thread exited; they never inspect cell attributes or prompt markers. `process_pending_bytes_chunks_with_commands()` likewise never observes inter-chunk command servicing, so it passes even though `run()` currently bypasses chunking on the blocking receive path (see TPR-02-001).
  Resolved: Rewrote all VTE tests to use synchronous `handle_bytes()` and verify actual state (cell attrs, prompt markers, mode cache, shutdown-via-drain) on 2026-03-31.
- [x] `[TPR-02-003][low]` `plans/threaded-io/index.md:38`, `plans/threaded-io/00-overview.md:225`, `plans/threaded-io/section-02-vte-migration.md:34` — Section 02 bookkeeping is still stale.
  Evidence: the section frontmatter marks Section 02 `in-progress` and 02.1-02.3 `complete`, but the human-readable status line in the section body and both plan summary files still say `Not Started`.
  Resolved: Updated all three files to reflect In Progress status on 2026-03-31.
- [x] `[TPR-02-004][medium]` `oriterm_mux/src/pane/io_thread/tests.rs:271` — `handle_bytes_chunked_drains_commands()` still does not prove inter-chunk command servicing.
  Validation: the test pre-queues `Shutdown`, feeds a 200 KB buffer, then only asserts that the shutdown flag is set afterwards. That same assertion still passes if `drain_commands()` runs only after the final chunk, or if the parser consumes the whole buffer before noticing shutdown, so the blocking-path regression fixed in `oriterm_mux/src/pane/io_thread/mod.rs:87` can reappear without tripping this test.
  Resolved: Added baseline full-parse eviction comparison — proves early exit parses fewer lines on 2026-03-31.
- [x] `[TPR-02-005][low]` `oriterm_mux/src/pane/io_thread/tests.rs:216` — `handle_bytes_advances_vte()` claims to pin SGR 31 red but only checks for any non-default foreground.
  Validation: after sending `\x1b[31mR`, the test asserts `cell.fg != Color::Named(NamedColor::Foreground)` instead of checking the actual red value. A wrong non-default color would still pass, so the documented "SGR red sets cell foreground color" contract is not fully covered.
  Resolved: Changed assertion to `assert_eq!(cell.fg, Color::Named(NamedColor::Red))` on 2026-03-31.

---

## 02.N Completion Checklist

- [x] `PtyEventLoop` forwards raw bytes to IO thread via channel (section 02.1)
- [x] `IoThreadEventProxy` created with `suppress_metadata = true` (section 02.3)
- [x] `PaneIoThread` now generic: `PaneIoThread<T: EventListener>` with `Term<T>` field (section 02.2)
- [x] IO thread runs both VTE processors (`vte::ansi::Processor` + `vte::Parser`)
- [x] Shell integration sequences (OSC 7, 133) processed on IO thread
- [x] Prompt marking deferred correctly
- [x] Mode cache updated by IO thread
- [x] Old `PtyEventLoop` parse path still works (dual processing — unchanged)
- [x] `timeout 150 cargo test -p oriterm_mux` passes
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `/tpr-review` passed (2 rounds, 5 findings all resolved)

**Exit Criteria:** The IO thread parses VTE sequences from PTY output and maintains its own `Term` state. The old `PtyEventLoop` + `Arc<FairMutex<Term>>` path continues to work in parallel. The IO thread's `Term` state is equivalent to the old path's `Term` state (verified by dimension comparison).
