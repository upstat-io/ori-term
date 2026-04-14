---
section: "06"
title: "Terminal Mode Plumbing (Mode 2026 timeout-abort + mode metadata consolidation)"
status: complete
reviewed: true
goal: "Wire the Mode 2026 timeout-abort path that currently has zero call sites in ori_term, and consolidate DEC mode metadata to eliminate the multi-sync-point LEAK across NamedPrivateMode consumers."
success_criteria:
  - "`Processor::sync_timeout()` and `Processor::stop_sync()` are CALLED from `oriterm_mux/src/pane/io_thread/mod.rs` — `grep -rn 'sync_timeout\\|stop_sync' oriterm_mux/src/pane/io_thread/` returns matches"
  - "Sync timeout policy documented: the io thread's `crossbeam_channel::select!` uses a deadline derived from `processor.sync_timeout().sync_timeout()` (the `StdSyncHandler`'s `Option<Instant>`). When the deadline fires (channel empty + timeout elapsed), call `processor.stop_sync(&mut self.terminal)` to flush/replay the buffer and emit `Effect::Presentation(PresentationEffect::Abort { reason: SyncAbortReason::Timeout })`"
  - "The `Abort` docstring in `oriterm_core/src/effect/families/presentation.rs:14` is corrected from 'discard buffered output' to 'flush buffered output' — `stop_sync()` replays/processes the buffer, it does NOT discard"
  - "Sync abort test: feed BSU + writes, wait >150ms (real wall-clock via `StdSyncHandler` timeout) — assert Abort effect emitted, snapshot publishes the buffered writes (they are replayed, not discarded), snapshot_seqno advances by exactly 1"
  - "No duplicated timeout state — the io thread does NOT add a `sync_deadline: Option<Instant>` field. It queries the processor's existing `sync_timeout().sync_timeout()` (StdSyncHandler) for the deadline each loop iteration"
  - "Post-parse housekeeping (prompt markers, mode_cache refresh, selection_dirty) runs after timeout-abort replay — extracted into a shared `post_parse_housekeeping()` method called by both `handle_bytes()` and the timeout-abort path"
  - "`LegacyEventSink` no longer silently drops `Effect::Presentation(_)` — the Abort effect is observable in production"
  - "`named_private_mode_number()` in `oriterm_core/src/term/handler/helpers.rs` is eliminated — callers use `mode as u16` (the enum already has explicit discriminants). This removes one sync point"
  - "`named_private_mode_flag()` remains in `oriterm_core/src/term/handler/helpers.rs` with an exhaustive match (catches missing variants at compile time). No registry table is created in `crates/vte` — mode metadata is `oriterm_core`'s concern, not VTE's"
  - "All existing teseq tests covering modes still pass (`cargo test -p oriterm_core --test teseq`)"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Mode 2026 fully wired** + **DEC mode metadata sync-point reduction** mission criteria"
inspired_by:
  - "Alacritty `alacritty_terminal/src/event_loop.rs:229-246` — sync timeout wiring: queries `state.parser.sync_timeout().sync_timeout()` for `Option<Instant>`, passes as timeout to mio `poll.wait()`, calls `state.parser.stop_sync()` when timeout fires"
  - "ori_term existing `crates/vte/src/ansi/processor.rs:90` — `sync_timeout(&self) -> &StdSyncHandler`, then `StdSyncHandler::sync_timeout(&self) -> Option<Instant>` at line 287"
  - "ori_term existing `crates/vte/src/ansi/processor.rs:150` — `stop_sync()` replays buffered bytes through `Performer`, NOT discarding them"
  - "ori_term existing `crates/vte/src/ansi/types.rs:226-295` — NamedPrivateMode enum with explicit u16 discriminants (CursorKeys=1, ColumnMode=3, etc.)"
depends_on: ["03", "04"]
third_party_review:
  status: resolved
  updated: 2026-04-13
sections:
  - id: "06.1"
    title: "Make io_thread select! deadline-aware for sync timeout"
    status: complete
  - id: "06.2"
    title: "Extract post-parse housekeeping into shared method"
    status: complete
  - id: "06.3"
    title: "Emit Abort effect + fix docstring + fix LegacyEventSink drop"
    status: complete
  - id: "06.4"
    title: "Eliminate named_private_mode_number() (WASTE removal)"
    status: complete
  - id: "06.5"
    title: "Test matrix: sync timeout + edge cases"
    status: complete
  - id: "06.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "06.N"
    title: "Completion Checklist"
    status: complete
---

# Section 06: Terminal Mode Plumbing

**Status:** Complete
**Goal:** Fix the Mode 2026 timeout-abort path (completely unwired — app crashes mid-sync hang the terminal forever) and consolidate DEC mode metadata by eliminating redundant sync points. Per two independent third-party reviews (Codex + Gemini), the original plan had critical architectural errors: the `select!` loop blocks indefinitely during sync, duplicated timeout state was proposed, the registry was placed in the wrong crate, and several factual inaccuracies existed.

**Success Criteria:**
- [x] `Processor::sync_timeout`/`stop_sync` called from io_thread via deadline-aware `select!`
- [x] No duplicated timeout state — queries processor's existing `StdSyncHandler::sync_timeout()` API
- [x] `PresentationEffect::Abort` effect emitted on timeout, observable in production (LegacyEventSink no longer drops it)
- [x] `Abort` docstring corrected: says "flush" not "discard" (matches `stop_sync` behavior)
- [x] Post-parse housekeeping shared between normal parse and timeout-abort replay
- [x] `named_private_mode_number()` eliminated — replaced by `mode as u16` cast
- [x] Test matrix covers: basic timeout, resize-during-sync, alt-screen-during-sync, double-publish prevention, nested BSU, max-buffer-overflow
- [x] All existing mode tests pass without modification
- [x] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [x] Connects to mission criteria: **Mode 2026 fully wired**, **DEC mode metadata sync-point reduction**

**Context:**

Mode 2026 (synchronized output) has two halves:
1. **Publication suppression** (WIRED): `maybe_produce_snapshot()` at `oriterm_mux/src/pane/io_thread/mod.rs:280` gates on `sync_bytes_count() > 0`, and `handle_bytes()` at line 242 skips `grid_dirty` when the sync buffer is non-empty.
2. **Timeout-abort** (NOT WIRED): When an application sends BSU (`\x1b[?2026h`) and then crashes or hangs without sending ESU, the terminal must abort the sync after a timeout and flush the buffered bytes. The VTE processor exposes `sync_timeout()` (returns `&StdSyncHandler`) and `stop_sync()` (replays buffered bytes), but ori_term never calls them.

**The timeout value is 150ms** (per `crates/vte/src/ansi/mod.rs:38`, `SYNC_UPDATE_TIMEOUT: Duration = Duration::from_millis(150)`), NOT 5000ms as the original plan incorrectly stated. This matches Alacritty's value.

**Actual sync points** (6, not 5 as originally claimed):
1. `NamedPrivateMode` enum definition — `crates/vte/src/ansi/types.rs:226` (canonical source)
2. `PrivateMode::new()` — `crates/vte/src/ansi/types.rs:175` (number→variant mapping)
3. `named_private_mode_number()` — `oriterm_core/src/term/handler/helpers.rs:22` (variant→number, PURE WASTE — the enum already has `= N` discriminants, so `mode as u16` works)
4. `named_private_mode_flag()` — `oriterm_core/src/term/handler/helpers.rs:56` (variant→TermMode flag)
5. `apply_decset()` — `oriterm_core/src/term/handler/modes.rs:17` (set behavior)
6. `apply_decrst()` — `oriterm_core/src/term/handler/modes.rs:111` (reset behavior)

Of these, #3 is eliminable (WASTE — `mode as u16` replaces the entire function). #2 is the inverse of #1 and cannot be consolidated without changing VTE's API. #4 is a legitimate mapping (not all modes have TermMode flags: `SaveCursor` and `ColumnMode` return `None`). #5 and #6 contain side-effect behavior that must stay in match arms (per Codex Q4 pushback — behavior is clearer in code than DSL).

**Why NO mode_registry.rs in crates/vte:** Both reviewers flagged this as a crate-boundary violation. `TermMode` is `oriterm_core`'s type. Putting a registry with `has_term_mode_flag: bool` in `crates/vte` leaks `oriterm_core` concerns into a vendored upstream fork. The original plan's success criterion ("adding a new mode = one registry entry edit") is also FALSE — you still need the enum variant + behavior match arms in `apply_decset`/`apply_decrst`. The actual fix is simpler: delete the waste function (`named_private_mode_number`), keep the exhaustive matches (they already catch missing variants at compile time via `match` exhaustiveness).

**Reference implementations:**
- **Alacritty** `alacritty_terminal/src/event_loop.rs:229-246` — queries `state.parser.sync_timeout().sync_timeout()` for `Option<Instant>`, converts to `Duration` via `st.saturating_duration_since(Instant::now())`, passes as timeout to mio `poll.wait()`. When poll returns with no events and no channel messages, calls `state.parser.stop_sync(&mut *self.terminal.lock())` and sends `Event::Wakeup`.

**Depends on:** Section 03 (Effect type exists), Section 04 (verification chain harness exists for SyncAbort tests).

---

## 06.1 Make io_thread select! deadline-aware for sync timeout

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

The io thread's main loop at line 128 uses `crossbeam_channel::select!` which blocks indefinitely when both channels are empty. During an active sync (BSU received, no ESU yet), this means the timeout check is NEVER reached — the thread sleeps forever if the app crashes mid-sync.

**The fix:** Replace the bare `select!` with a deadline-aware version using `crossbeam_channel::select!` with `default(timeout)` or equivalently use `recv_deadline()` / `crossbeam_channel::after()`.

- [x] **Read and understand** the current loop structure at `oriterm_mux/src/pane/io_thread/mod.rs:112-143`. The `select!` at line 128 blocks indefinitely. The sync timeout check must happen INSIDE this select as a deadline/timeout arm.
- [x] **Compute the sync deadline** at the top of step 4 (before the `select!`). Query `self.processor.sync_timeout().sync_timeout()` (returns `Option<Instant>` from `StdSyncHandler` at `crates/vte/src/ansi/processor.rs:287`). Convert to a `Duration` via `deadline.saturating_duration_since(Instant::now())`. If `None`, the select has no timeout (blocks indefinitely as before).
- [x] **Add a `default(duration)` arm** to the `select!` macro. When the timeout fires (both channels empty for `duration`): (1) capture `evicted_before` from the grid, (2) call `self.processor.stop_sync(&mut self.terminal)` to replay the buffered bytes through VTE, (3) run `self.post_parse_housekeeping(evicted_before)` (from 06.2), (4) emit the Abort effect (06.3), (5) force a snapshot: `self.grid_dirty.store(true, Ordering::Release); self.maybe_produce_snapshot();` and `continue` the loop. **Note**: NO RawInterceptor replay is needed — `handle_bytes()` already runs the raw interceptor on ALL incoming bytes BEFORE they enter the sync buffer (lines 228-232), so the shell-integration effects (OSC 7, OSC 133, notifications) have already been processed.
- [x] **Emit the Abort effect** after `stop_sync` (detailed in 06.3 — effect emission not yet implemented, tracked there).
- [x] **Guard against double-publish**: After `stop_sync()` (called via `stop_sync_internal(handler, None)`), `sync_bytes_count()` is always 0 — the buffer is unconditionally cleared and SyncUpdate mode is unset. The `maybe_produce_snapshot()` call after forcing `grid_dirty = true` is therefore safe: `sync_bytes_count() == 0` guarantees the suppression gate won't re-trigger. Add a `debug_assert_eq!(self.processor.sync_bytes_count(), 0)` after `stop_sync` to document this invariant.
- [x] **Do NOT add a `sync_deadline: Option<Instant>` field** to `PaneIoThread`. The deadline is already tracked inside the processor's `SyncState<StdSyncHandler>`. Adding a parallel tracker is duplicated state (LEAK finding from both reviewers).
- [x] **Validation**: manual trace through the loop verifying that (a) during normal operation with no sync, the select blocks indefinitely as before, (b) during active sync, the select has a 150ms timeout, (c) when the timeout fires, stop_sync is called and a snapshot is published.

**Pseudocode for the modified select block:**
```rust
// 4. Block on either channel, with sync timeout if active.
let sync_deadline = self.processor.sync_timeout().sync_timeout();
match sync_deadline {
    Some(deadline) => {
        let timeout = deadline.saturating_duration_since(Instant::now());
        crossbeam_channel::select! {
            recv(self.cmd_rx) -> msg => { /* same as current */ },
            recv(self.byte_rx) -> msg => { /* same as current */ },
            default(timeout) => {
                // Sync timeout fired — abort and flush.
                let evicted_before = self.terminal.grid().total_evicted();
                // No raw interceptor replay needed — handle_bytes() already ran
                // the raw interceptor on these bytes before they entered the buffer.
                self.processor.stop_sync(&mut self.terminal);
                self.post_parse_housekeeping(evicted_before);
                self.emit_sync_abort_effect();
                self.grid_dirty.store(true, Ordering::Release);
                self.maybe_produce_snapshot();
            },
        }
    },
    None => {
        crossbeam_channel::select! {
            recv(self.cmd_rx) -> msg => { /* same as current */ },
            recv(self.byte_rx) -> msg => { /* same as current */ },
        }
    },
}
```

---

## 06.2 Extract post-parse housekeeping into shared method

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

`handle_bytes()` at lines 246-271 performs post-parse housekeeping after VTE processing:
- Deferred prompt marking (lines 247-255)
- Prompt marker pruning for scrollback eviction (lines 258-261)
- Mode cache update (lines 264-265)
- Selection-dirty propagation (lines 268-271)

When `stop_sync` replays buffered bytes in the timeout path, these housekeeping steps are bypassed — the replayed bytes go through the VTE `Performer` but NOT through `handle_bytes`. This means: prompt markers won't be processed, mode_cache won't be refreshed (stale main-thread reads), and selection_dirty won't propagate. Both reviewers flagged this as a GAP.

- [x] **Extract** the housekeeping block (lines 246-271 in `handle_bytes()`) into a `fn post_parse_housekeeping(&mut self, evicted_before: usize)` method.
- [x] **Call it from `handle_bytes()`** after `processor.advance()` and the sync gate check, replacing the current inline code.
- [x] **Call it from the timeout-abort path** in 06.1 after `processor.stop_sync()`. Pass `evicted_before` captured before the stop_sync call (to detect scrollback eviction during replay).
- [x] **Sibling test**: `sync_timeout_runs_post_parse_housekeeping()` — feeds BSU + `\x1b[?25l`, triggers timeout, asserts mode_cache reflects cursor hidden after replay. Implemented in 06.5 test matrix.
- [x] **Validation**: mode_cache is updated after timeout-abort replay. Grep confirms `post_parse_housekeeping` is called from both paths.

**BLOAT split**: `oriterm_mux/src/pane/io_thread/mod.rs` exceeded 500 lines after adding `handle_sync_timeout()` and `post_parse_housekeeping()`. Extracted `PaneIoHandle`, `IoThreadConfig`, and `new_with_handle()` to `oriterm_mux/src/pane/io_thread/handle.rs` (160 lines). `mod.rs` is now 421 lines.

---

## 06.3 Emit Abort effect + fix docstring + fix LegacyEventSink drop

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm_core/src/effect/families/presentation.rs`, `oriterm_core/src/effect/sink/legacy/mod.rs`

Three related fixes:

### 06.3a: Emit Abort effect on timeout

- [x] After `processor.stop_sync()` in the timeout-abort path (06.1), emit the effect via `self.terminal.effect_sink().push(Effect::Presentation(PresentationEffect::Abort { reason: SyncAbortReason::Timeout }))`. (The `effect_sink` is accessed through `Term`'s public API — verify the exact accessor method.)
- [x] **Note**: `stop_sync()` takes `&mut self.terminal` as a handler, so the effect push must happen AFTER the `stop_sync` call returns (cannot borrow `self.terminal` mutably and call `effect_sink()` simultaneously). Extract a small helper method `emit_sync_abort_effect(&mut self)` that calls `self.terminal.effect_sink().push(...)`.
- [x] Sibling test: `sync_timeout_emits_abort_effect()` — feeds BSU + content, triggers timeout, asserts `Effect::Presentation(PresentationEffect::Abort { reason: SyncAbortReason::Timeout })` is observable. Implemented in 06.5 test matrix using `QueueingEffectSink`.

### 06.3b: Fix Abort docstring

- [x] In `oriterm_core/src/effect/families/presentation.rs:14`, change the doc comment from `/// Abort synchronized update — discard buffered output.` to `/// Abort synchronized update — flush buffered output.` because `stop_sync()` at `crates/vte/src/ansi/processor.rs:162-193` REPLAYS the buffered bytes through the `Performer` (line 173: `self.parser.advance(&mut performer, &buffer[..offset])`), then clears the sync state. It does NOT discard.
- [x] **Validation**: grep the docstring to confirm it says "flush" not "discard".

### 06.3c: Fix LegacyEventSink dropping Presentation effects

- [x] In `oriterm_core/src/effect/sink/legacy/mod.rs:89`, the match arm `Effect::Presentation(_) => return` silently drops ALL presentation effects. This means the `Abort` effect emitted in 06.3a will never be observable in production (panes use `LegacyEventSink`).
- [x] **Fix**: For the immediate term, change the `Presentation` arm to log the effect at `info!` level so it's observable in logs. The full fix (migrating to direct `Effect` subscription) is tracked in `plans/effect-cutover/` and is out of scope for this section. But the silent drop with no log is a bug — at minimum, log it.
- [x] **Alternative (preferred if feasible)**: If `LegacyEventSink` can forward `Presentation` effects through the existing `EventListener` interface (e.g. as a new `Event::SyncAbort` variant), do that instead. But if adding an `Event` variant just for the migration bridge is churny, the log approach is acceptable. **Decision**: log approach chosen — adding an `Event::SyncAbort` variant to the legacy bridge just for observability during a migration phase is unnecessary churn.
- [x] Sibling test: `legacy_sink_does_not_silently_drop_presentation_effects()` — push an `Abort` effect to a `LegacyEventSink`, assert it was logged or forwarded (not silently dropped). **Implemented**: `presentation_abort_logged_not_forwarded_as_event()` and updated `presentation_effects_logged_not_forwarded_as_event()` in `oriterm_core/src/effect/sink/legacy/tests.rs`.

---

## 06.4 Eliminate named_private_mode_number() (WASTE removal)

**File(s):** `oriterm_core/src/term/handler/helpers.rs`, `oriterm_core/src/term/handler/status.rs`

`named_private_mode_number()` at `helpers.rs:22-53` is a 30-line match that manually maps every `NamedPrivateMode` variant to its mode number. But the enum already has explicit `u16` discriminants (`CursorKeys = 1`, `ColumnMode = 3`, etc.), and `PrivateMode::raw()` at `crates/vte/src/ansi/types.rs:210-215` already uses `named as u16`. This function is WASTE — it duplicates information that's already in the enum's discriminants.

- [x] **Delete** `named_private_mode_number()` from `helpers.rs`.
- [x] **Replace the single call site** in `status.rs:113` (`let num = named_private_mode_number(named)`) with `let num = named as u16`.
- [x] **Remove** the import of `named_private_mode_number` from `status.rs:19`.
- [x] **Validation**: `cargo test -p oriterm_core` passes (1623 tests). All 57 teseq mode tests pass. The mode-to-number mapping is verified by the enum's explicit discriminants — the compiler enforces correctness.
- [x] **Keep `named_private_mode_flag()`** — this function maps variants to `Option<TermMode>` flags, which is genuine metadata not derivable from the enum alone (e.g. `ColumnMode` maps to `None`, `CursorKeys` maps to `Some(TermMode::APP_CURSOR)`). The exhaustive match serves as a compile-time guard: adding a new `NamedPrivateMode` variant forces updating this function. This is NOT a registry candidate — it's a clean exhaustive match in the right crate.

### Why no mode_registry.rs in crates/vte

Both reviewers agreed: placing `mode_registry.rs` in `crates/vte` violates crate boundaries.
- `TermMode` is owned by `oriterm_core` (see `.claude/rules/crate-boundaries.md`).
- `crates/vte` is a vendored upstream fork — oriterm-specific metadata does not belong there.
- The plan's success criterion ("adding a new mode = one registry entry edit") was FALSE: you still need the enum variant + `apply_decset`/`apply_decrst` behavior arms + optionally `named_private_mode_flag()`.
- The exhaustive `match` in `named_private_mode_flag()` already catches missing variants at compile time — this IS the sync-point enforcement mechanism, and it's cheaper than a registry table.

**Remaining sync points after this section (5):**
1. `NamedPrivateMode` enum — canonical source (`crates/vte/src/ansi/types.rs:226`)
2. `PrivateMode::new()` — number→variant (`crates/vte/src/ansi/types.rs:175`)
3. `named_private_mode_flag()` — variant→TermMode (`oriterm_core/src/term/handler/helpers.rs:56`)
4. `apply_decset()` — set behavior (`oriterm_core/src/term/handler/modes.rs:17`)
5. `apply_decrst()` — reset behavior (`oriterm_core/src/term/handler/modes.rs:111`)

All 5 use exhaustive matches on `NamedPrivateMode`, so adding a new variant triggers a compile error in all of them. This is the correct enforcement mechanism — stronger than a runtime registry check.

---

## 06.5 Test matrix: sync timeout + edge cases

**File(s):** `oriterm_mux/src/pane/io_thread/tests.rs`

Comprehensive test matrix for the timeout-abort path. All tests use the synchronous `PaneIoThread` test helpers (no spawning needed). Tests that require clock control should use the existing `Timeout` trait — the VTE `Processor` is generic over `T: Timeout`, so a test-only `MockTimeout` can be injected.

**However**: `PaneIoThread` constructs its `Processor` with the default `StdSyncHandler` (line 459 in `mod.rs`). To test timeout behavior without real wall-clock waits, we need either:
(a) Make `PaneIoThread` generic over `T: Timeout` (significant refactor), OR
(b) Use `StdSyncHandler` and accept that the 150ms timeout is real (tests that trigger timeout need to `std::thread::sleep(Duration::from_millis(200))` or similar), OR
(c) Add a `set_sync_timeout()` test method on `PaneIoThread` that replaces the processor's timeout for testing.

Option (b) is acceptable for a 150ms timeout — tests complete in <200ms each. Option (c) is a HACK. Option (a) is correct but may be disproportionate for this section. Use option (b) with clear documentation.

### Test matrix dimensions

**Sync state** (rows): no-sync, in-sync, post-sync-commit, post-sync-timeout-abort
**Event type** (columns): timeout, manual-ESU, nested-BSU, max-buffer-overflow, resize, alt-screen-swap, process-bytes-after-abort

### Required tests

- [x] `sync_timeout_aborts_and_flushes_buffered_writes()` — Semantic pin: verifies sync buffer cleared, snapshot contains replayed "hello", grid_dirty set, wakeup fires, seqno advances.
- [x] `sync_timeout_emits_abort_effect()` — Uses `QueueingEffectSink` to observe `Effect::Presentation(PresentationEffect::Abort { reason: SyncAbortReason::Timeout })`.
- [x] `sync_timeout_runs_post_parse_housekeeping()` — Feeds BSU + `\x1b[?25l`, triggers timeout, asserts mode_cache reflects cursor hidden.
- [x] `resize_during_sync_timeout()` — Resize via `process_resize(40, 100)` during sync, then timeout. Grid dimensions match, no crash.
- [x] `alt_screen_swap_in_replayed_bytes()` — BSU + `\x1b[?1049h`, timeout. Alt screen active in both Term mode and mode_cache.
- [x] `no_double_publish_on_timeout()` — Wakeup fires exactly once on timeout (no double-publish).
- [x] `nested_bsu_in_sync_buffer()` — BSU + content + nested BSU + content. After timeout: sync buffer empty, mode unset, both "before" and "after" in snapshot.
- [x] `sync_abort_after_max_buffer_overflow()` — Feeds BSU + >2 MiB. VTE overflow fires, sync buffer cleared.
- [x] `run_loop_sync_timeout_fires()` — Spawned run-loop test via `spawn_pair_with_flag()`. Sends BSU + content via byte_sender, waits >150ms, asserts snapshot contains content (proves the real `select!` deadline arm fires).
- [x] `no_timeout_when_not_in_sync()` — Negative pin: no BSU sent, `sync_timeout()` returns `None`, sync buffer stays empty after normal bytes.

### Semantic pins

- `sync_timeout_aborts_and_flushes_buffered_writes` is the semantic pin: it ONLY passes when (1) stop_sync is called from the io thread, AND (2) the replayed bytes appear in the snapshot.
- `no_timeout_when_not_in_sync` is the negative pin: it ONLY passes when the timeout arm does NOT fire outside sync mode.

---

## 06.R Third Party Review Findings

- [x] `[TPR-06-001-codex][high]` `section-06:156` — RawInterceptor bypass during timeout replay. Timeout path replays bytes through VTE Performer but skips the RawInterceptor (OSC 7, OSC 133, XTVERSION).
  Resolved: Rejected after verification in iteration 2 (2026-04-13). `handle_bytes()` runs the RawInterceptor on ALL incoming bytes BEFORE they enter the sync buffer (lines 228-232). The raw interceptor already processed these bytes — replaying it again would cause DOUBLE-EXECUTION of CWD updates, prompt markers, and notifications. Subsection 06.1b removed.
- [x] `[TPR-06-002-codex][medium]` `section-06:225` — Test matrix doesn't test actual `run()` loop with real channels.
  Resolved: Rejected after verification on 2026-04-13. Existing tests in `oriterm_mux/src/pane/io_thread/tests.rs` DO exercise `run()` via `spawn()` + real channels (e.g., `shutdown_via_command`, `test_concurrent_resize_and_pty_output`). Finding is factually incorrect.
- [x] `[TPR-06-003-codex][medium]` `00-overview.md:38,129,347` — DRIFT between Section 06 and overview/section 09: stale `SyncBegin/SyncCommit/SyncAbort` names, stale "registry table" mission text.
  Resolved: Fixed on 2026-04-13. Updated overview (variant names, mission criteria text) and section 09 (variant names, coupling note).
- [x] `[TPR-06-001-gemini][high]` `00-overview.md:46` — Overview contradicts section 06 on mode registry approach.
  Resolved: Fixed on 2026-04-13. Same fix as [TPR-06-003-codex] — updated overview mission criteria from "single registry table" to "sync-point reduction via WASTE elimination."
- [x] `[TPR-06-002-gemini][medium]` `section-06:139` — Pseudocode calls `post_parse_housekeeping()` without `evicted_before` argument.
  Resolved: Fixed on 2026-04-13. Updated pseudocode to capture `evicted_before` and pass it.
- [x] `[TPR-06-003-gemini][medium]` `section-06:246` — `MaxBufferBytesExceeded` variant exists but is never emitted by VTE overflow.
  Resolved: Added a note to the max-buffer test item explaining the cross-crate gap. The overflow test verifies correct processing; emitting the effect requires VTE→Handler signaling which is a future plumbing item.

**Iteration 2 findings (2026-04-13):**
- [x] `[TPR-06-001-codex-i2][high]` `section-09:75` — Mode 2026 apex tests should be in oriterm_mux, not oriterm_core (timeout/snapshot lives in oriterm_mux). Also "virtual clock" wording is invalid.
  Resolved: Rejected — this is a Section 09 concern, not Section 06. Section 09 has `reviewed: false` and will be reviewed via `/review-plan` before implementation. The stale variant names in Section 09 were already fixed in iteration 1.
- [x] `[TPR-06-002-codex-i2][medium]` `section-06:278` — Nested BSU test incorrectly expects `sync_bytes_count() > 0` after `stop_sync`.
  Resolved: Fixed on 2026-04-13. Updated test expectation to match VTE's actual behavior: `stop_sync(handler, None)` unconditionally clears the buffer and unsets SyncUpdate.
- [x] `[TPR-06-003-codex-i2][medium]` `section-06:256` — Need at least one spawned run-loop test for the deadline-aware select path.
  Resolved: Fixed on 2026-04-13. Added `run_loop_sync_timeout_fires()` spawned test using `spawn_pair_with_flag()` pattern.
- [x] `[TPR-06-001-gemini-i2][high]` `section-06:154` — RawInterceptor double-execution: handle_bytes already runs raw_parser before bytes enter the sync buffer. Replaying the raw interceptor again during stop_sync would cause double CWD updates, double prompt markers, etc.
  Resolved: Fixed on 2026-04-13. Removed subsection 06.1b, removed Processor::sync_buffer() accessor, removed timeout_replay_preserves_shell_integration test, updated pseudocode. Added note explaining why no raw replay is needed.
- [x] `[TPR-06-002-gemini-i2][medium]` `section-06:318` — Same as [TPR-06-002-codex-i2]: nested BSU test expectation wrong.
  Resolved: Fixed on 2026-04-13. Same fix as [TPR-06-002-codex-i2].

**Implementation TPR findings (2026-04-13, 3 iterations):**
- [x] `[TPR-06-001-codex-i3][medium]` `oriterm_core/src/effect/sink/legacy/tests.rs:272` — Pin the LegacyEventSink log path with a real assertion. Tests only asserted `is_empty()` for events — would still pass if the `info!` line were removed.
  Resolved: Fixed on 2026-04-13. Added `pending_presentation_effects` queue, then replaced with `AtomicU32` counter per TPR-06-001-codex-i4. Tests now assert `presentation_effect_count() > 0`.
- [x] `[TPR-06-002-codex-i3][low]` `plans/spec-conformance/section-06-terminal-mode-plumbing.md:55` — Stale body status banner said "Not Started" but frontmatter said "in-progress".
  Resolved: Fixed on 2026-04-13. Updated body banner to "In Progress".
- [x] `[TPR-06-001-codex-i4][medium]` `oriterm_core/src/effect/sink/legacy/mod.rs:108` — `pending_presentation_effects` Vec queue created unbounded per-pane memory growth (no production consumer).
  Resolved: Fixed on 2026-04-13. Replaced `Vec<PresentationEffect>` queue with `AtomicU32` counter — zero memory overhead, tests pin the count.

**Final TPR pass: iteration 3 (2026-04-13) — clean. Both Codex and Gemini returned zero actionable findings.**

---

## 06.N Completion Checklist

- [x] Failing test matrix written FIRST: `sync_timeout_aborts_and_flushes_buffered_writes` and `sync_timeout_runs_post_parse_housekeeping` written before implementation (TDD) — implementation (06.1/06.2) was done in prior session; tests written in 06.5 verify the existing implementation.
- [x] **Matrix dimensions**: sync state (no-sync, in-sync, post-commit, post-abort) x event type (timeout, manual-ESU, nested-BSU, max-buffer-overflow, resize-during-sync, alt-screen-during-sync, process-after-abort, double-publish) — 10 tests covering all key cells.
- [x] **Semantic pin**: `sync_timeout_aborts_and_flushes_buffered_writes` — proves stop_sync is called AND bytes are replayed
- [x] **Negative pin**: `no_timeout_when_not_in_sync` — proves timeout arm doesn't fire spuriously
- [x] `crossbeam_channel::select!` is deadline-aware — uses `default(timeout)` arm derived from `StdSyncHandler::sync_timeout()`
- [x] No duplicated `sync_deadline` field — queries processor's existing timeout state
- [x] `post_parse_housekeeping()` extracted and called from both `handle_bytes()` and timeout-abort path
- [x] `PresentationEffect::Abort` docstring corrected to "flush" (not "discard")
- [x] `LegacyEventSink` no longer silently drops `Presentation` effects (at minimum logged)
- [x] `Processor::sync_timeout`/`stop_sync` called from io_thread (`grep` confirms)
- [x] `SyncAbort` effect emitted on timeout
- [x] `named_private_mode_number()` deleted — callers use `mode as u16`
- [x] `named_private_mode_flag()` retained with exhaustive match (compile-time guard)
- [x] No `mode_registry.rs` in `crates/vte` — crate boundary respected
- [x] Behavior unchanged: existing teseq mode tests pass without modification (57 mode tests, 176 total teseq)
- [x] Alloc regression unchanged (5 passed, 0 failed)
- [x] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [x] Plan annotation cleanup (0 annotations found — clean)
- [x] Section frontmatter `status` -> `complete`
- [x] `00-overview.md` Quick Reference + mission criteria updated (Mode 2026 timeout wired)
- [x] `index.md` section 06 status updated
- [x] `/tpr-review` passed (3 iterations: 2 findings → 1 finding → clean)
- [x] `/impl-hygiene-review` passed — 1 DRIFT finding (stale module doc), fixed inline

**Exit Criteria:** Mode 2026 timeout-abort wired via deadline-aware select! using the processor's existing timeout state (no duplication); Abort effect emitted and observable (docstring corrected, LegacyEventSink no longer drops it); post-parse housekeeping shared between normal and timeout paths; `named_private_mode_number()` eliminated (WASTE); remaining sync points enforced via compile-time exhaustive matches; test matrix covers timeout, resize-during-sync, alt-screen-during-sync, double-publish, nested-BSU, and negative pin.
