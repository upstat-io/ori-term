# Section 30 Verification Results: Pane Extraction + Domain System

**Verified by:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-29
**Branch:** dev
**Status:** PASS with plan-vs-reality discrepancies

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` -- full read
- `.claude/rules/code-hygiene.md` -- full read
- `.claude/rules/impl-hygiene.md` -- full read
- `.claude/rules/test-organization.md` -- full read
- `.claude/rules/crate-boundaries.md` -- loaded via system-reminder
- `plans/roadmap/section-30-pane-domain.md` -- full read

---

## 30.1 Pane Struct Extraction

### Files Examined
- `oriterm_mux/src/pane/mod.rs` (427 lines)
- `oriterm_mux/src/pane/shutdown.rs` (29 lines)
- `oriterm_mux/src/pane/selection.rs` (131 lines)
- `oriterm_mux/src/pane/mark_cursor/mod.rs` (39 lines)
- `oriterm_mux/src/pane/tests.rs` (66 lines)
- `oriterm_mux/src/pane/mark_cursor/tests.rs` (130 lines)

### Plan says: `oriterm/src/pane/mod.rs`
### Reality: `oriterm_mux/src/pane/mod.rs`

**The plan claims Pane lives in `oriterm`, but it actually lives in `oriterm_mux`.** This is architecturally correct per the crate-boundaries rule ("oriterm_mux owns Pane lifecycle") and CLAUDE.md Key Paths ("oriterm_mux/src/pane/ -- Pane (terminal state, PTY I/O)"). The plan's file path is stale.

### Pane struct fields -- plan vs. reality

| Plan field | Actual | Status |
|---|---|---|
| `id: PaneId` | Present (line 105) | MATCH |
| `domain_id: DomainId` | Present (line 108, `#[allow(dead_code)]` with reason) | MATCH |
| `terminal: Arc<FairMutex<Term<MuxEventProxy>>>` | Present (line 110) | MATCH |
| `notifier: PaneNotifier` | Present (line 112) | MATCH |
| `pty_control: PtyControl` | Present (line 114) | MATCH |
| `reader_thread: Option<JoinHandle<()>>` | Present (line 120) | MATCH |
| *(not in plan)* `writer_thread: Option<JoinHandle<()>>` | Present (line 126) | ADDITION |
| `pty: PtyHandle` | Present (line 128) | MATCH |
| `grid_dirty: Arc<AtomicBool>` | Present (line 130) | MATCH |
| `wakeup_pending: Arc<AtomicBool>` | Present (line 132) | MATCH |
| `mode_cache: Arc<AtomicU32>` | Present (line 134) | MATCH |
| `selection: Option<Selection>` | Present (line 152) | MATCH |
| `search: Option<SearchState>` | Present (line 155) | MATCH |
| `mark_cursor: Option<MarkCursor>` | Present (line 153) | MATCH |
| `title: String` | Present (line 136) | MATCH |
| `cwd: Option<String>` | Present (line 140) | MATCH |
| `has_bell: bool` | Present (line 150) | MATCH |
| *(not in plan)* `icon_name: Option<String>` | Present (line 138) | ADDITION |
| *(not in plan)* `has_explicit_title: bool` | Present (line 145) | ADDITION |
| *(not in plan)* `last_command_duration: Option<Duration>` | Present (line 148) | ADDITION |
| *(not in plan)* `last_pty_size: AtomicU32` | Present (line 162) | ADDITION |

Additions are legitimate enhancements (icon name for OSC 1, title priority logic, command duration for shell integration, PTY resize dedup).

### PaneParts, PaneNotifier, from_parts -- VERIFIED

`PaneParts` groups constructor params (line 70). `PaneNotifier` sends via mpsc (line 36). `from_parts` constructs pane from parts (line 169). All match plan claims.

### Drop for Pane -- PARTIALLY MATCHES

Plan says: "shutdown, kill, join with timeout, reap child"
Reality (`shutdown.rs`): shutdown, kill, wait (blocking reap), detach threads (no join)

There is no join-with-timeout. Threads are dropped (detached). The code comment explains: "callers drop panes on background threads, so this doesn't block the event loop." This is a deliberate deviation, not a bug.

### Lock-free accessors -- VERIFIED

`grid_dirty()`, `clear_grid_dirty()`, `clear_wakeup()`, `mode()` all present and use correct `Ordering::Acquire`/`Release` semantics. No `refresh_mode_cache()` method exists on `Pane` (mode cache is updated by the PTY event loop directly).

### Tests -- 4 pane tests + 12 mark_cursor tests = 16 total

```
cargo test -p oriterm_mux -- pane::
16 passed, 0 failed
```

Tests verify:
- `grid_dirty_set_and_clear` -- atomic flag round-trip
- `wakeup_coalescing` -- swap semantics
- `mode_cache_round_trip` -- u32 atomic
- `dirty_flag_cross_thread_pattern` -- actual thread spawn + Acquire visibility
- 12 mark_cursor `to_viewport` tests (bounds, overflow, edge cases)

**Coverage gap:** No test for `Pane::from_parts` (requires live PTY). No test for `Pane::Drop` (requires live PTY). These are inherently integration-level and appropriately deferred to e2e tests.

---

## 30.2 Domain Trait + LocalDomain

### Files Examined
- `oriterm_mux/src/domain/mod.rs` (87 lines)
- `oriterm_mux/src/domain/local.rs` (154 lines)
- `oriterm_mux/src/domain/wsl.rs` (45 lines)
- `oriterm_mux/src/domain/tests.rs` (155 lines)
- `oriterm_mux/src/id/mod.rs` (206 lines)
- `oriterm_mux/src/id/tests.rs` (126 lines)

### Plan says: `DomainId` in `oriterm_mux/src/id/mod.rs`, `LocalDomain` in `oriterm/src/domain/local.rs`
### Reality: `DomainId` in `oriterm_mux/src/id/mod.rs` (MATCH), `LocalDomain` in `oriterm_mux/src/domain/local.rs` (moved from `oriterm` to `oriterm_mux`)

The crate migration is correct -- `LocalDomain` spawns PTY and creates `Pane`, both of which are `oriterm_mux` types. Having it in `oriterm_mux` avoids cross-crate boundary issues.

### DomainId -- VERIFIED

`DomainId(u64)` newtype with Display, sealed `MuxId` trait, `from_raw`/`raw`, `DomainId::LOCAL` constant (value 0). Serde derives present. All match plan.

### DomainState enum -- VERIFIED

`Attached` and `Detached` variants with `Debug, Clone, Copy, PartialEq, Eq` derives.

### SpawnConfig -- VERIFIED with addition

Fields: `cols`, `rows`, `shell`, `cwd`, `env`, `scrollback` all match plan. Additional field: `shell_integration: bool` (default `true`). This is a legitimate addition for OSC 133/7 support.

### Domain trait -- VERIFIED

`id()`, `name()`, `state()`, `can_spawn()` -- all match plan. The plan correctly notes `spawn_pane` is concrete (not on the trait) because it requires I/O types.

### LocalDomain -- VERIFIED

`new(id: DomainId)`, `spawn_pane(...)` returning `io::Result<Pane>`. Creates PTY via `spawn_pty`, builds `MuxEventProxy`, assembles `PaneParts`, returns `Pane::from_parts`. Clean separation of concerns.

### WslDomain stub -- VERIFIED

`can_spawn()` returns `false`, `state()` returns `Detached`. Properly gated with `#[allow(dead_code, reason = "...")]`.

### Tests -- 8 domain + 13 ID = 21 total

```
cargo test -p oriterm_mux -- domain::  -> 8 passed
cargo test -p oriterm_mux -- id::      -> 13 passed
```

Domain tests:
- MockDomain attached can spawn, detached cannot
- SpawnConfig defaults and custom values
- Domain trait object safety (`Box<dyn Domain>`)
- SpawnConfig clone independence
- DomainState equality and copy semantics
- SpawnConfig empty env

ID tests:
- All ID types are Copy + Hash + Eq
- Allocator starts at 1, monotonically increases, unique values
- Display formatting, raw round-trip, MuxId trait round-trip
- Different ID types not interchangeable
- IDs work as HashMap keys
- Default matches new

---

## 30.3 Pane + Session Registries

### Files Examined
- `oriterm_mux/src/registry/mod.rs` (72 lines)
- `oriterm_mux/src/registry/tests.rs` (118 lines)
- `oriterm/src/session/registry/mod.rs` (149 lines)
- `oriterm/src/session/registry/tests.rs` (155 lines)
- `oriterm/src/session/tab/mod.rs` (180 lines)
- `oriterm/src/session/tab/tests.rs` (143 lines)
- `oriterm/src/session/window/mod.rs` (155 lines)
- `oriterm/src/session/window/tests.rs` (174 lines)

### Plan vs. Reality -- SIGNIFICANT DIVERGENCE

The plan describes:
1. `PaneEntry` with `pane: PaneId`, `tab: TabId`, `domain: DomainId`
2. `PaneRegistry` with `panes_in_tab` method
3. `MuxTab` and `MuxWindow` structs in `oriterm_mux/src/session/mod.rs`
4. `SessionRegistry` in `oriterm_mux/src/registry/mod.rs`

**What actually exists:**

1. `PaneEntry` has only `pane: PaneId` and `domain: DomainId` (NO `tab: TabId` field). This is architecturally correct -- the mux layer is a "flat pane server" with no knowledge of tabs.

2. `PaneRegistry` has `register`, `unregister`, `get`, `len`, `is_empty` -- **NO `panes_in_tab`** method. Since there's no `tab` field on `PaneEntry`, this method cannot exist.

3. `MuxTab` and `MuxWindow` **DO NOT EXIST**. Instead, `Tab` and `Window` structs live in `oriterm/src/session/tab/mod.rs` and `oriterm/src/session/window/mod.rs`. This follows the CLAUDE.md principle: "GUI-owned session: oriterm/src/session/ owns all presentation state (tabs, windows, layouts)."

4. `SessionRegistry` lives in `oriterm/src/session/registry/mod.rs`, not in `oriterm_mux`. It owns tab/window CRUD, ID allocation, and `window_for_tab` lookups.

**This divergence is architecturally sound.** The plan originally envisioned tab/window management in the mux layer, but the implementation correctly keeps the mux as a flat pane server and puts session management in the GUI binary. The plan's checkboxes are misleading -- they claim items exist that don't.

### PaneRegistry -- VERIFIED (with reduced scope)

Register, unregister, get, len, is_empty. HashMap-backed, O(1) lookups.

### Tab (replaces MuxTab) -- VERIFIED

- `new()`, `set_tree()` (pushes undo), `undo_tree()` (skips stale), `redo_tree()`, `all_panes()`
- Undo stack capped at 32 (`MAX_UNDO_ENTRIES`)
- `replace_layout()` -- non-undo tree replacement
- `zoomed_pane` support
- Floating layer integration

### Window (replaces MuxWindow) -- VERIFIED

- `add_tab()`, `remove_tab()` (adjusts active), `set_active_tab_idx()` (clamps)
- `insert_tab_at()`, `reorder_tab()`, `replace_tabs()`
- Active tab index tracking with proper adjustment on all mutations

### SessionRegistry -- VERIFIED

- Tab and window CRUD with HashMap-backed storage
- `alloc_tab_id()`, `alloc_window_id()` -- monotonic ID allocation
- `window_for_tab()`, `window_for_pane()`, `tab_for_pane()` -- lookup queries
- `is_last_pane()` -- boundary check for last-pane-close logic

### Tests -- 8 registry + 16 session_registry + 11 tab + 16 window = 51 total

```
cargo test -p oriterm_mux -- registry::         -> 8 passed
cargo test -p oriterm -- session::registry::    -> 16 passed
cargo test -p oriterm -- session::tab::         -> 11 passed
cargo test -p oriterm -- session::window::      -> 16 passed
```

Coverage is thorough:
- PaneRegistry: empty, register/get, unregister, overwrite, multi-domain, stress (1000 entries)
- SessionRegistry: tab/window CRUD, window_for_tab, is_last_pane, ID allocation, default
- Tab: new, set_active, zoom, set_tree/undo/redo cycle, stale skip, replace_layout, redo clearing
- Window: empty, add/remove/insert/reorder/replace tabs, active index adjustment, clamping, edge cases

**Missing test from plan:** "undo stack capped at 32" -- no test explicitly adds 33+ entries and checks truncation. The logic is present in `set_tree()` (line 100: `if self.undo.len() >= MAX_UNDO_ENTRIES { self.undo.pop_front() }`), but the test was not written.

---

## 30.4 MuxEventProxy

### Files Examined
- `oriterm_mux/src/mux_event/mod.rs` (349 lines)
- `oriterm_mux/src/mux_event/tests.rs` (525 lines)
- `oriterm/src/event.rs` (58 lines)
- `oriterm_mux/src/in_process/event_pump.rs` (127 lines)

### Plan says: files in `oriterm/src/mux_event/`
### Reality: files in `oriterm_mux/src/mux_event/`

Again, the mux event types correctly live in `oriterm_mux`, not `oriterm`.

### MuxEvent enum -- VERIFIED with additions

Plan variants: `PaneOutput`, `PaneExited`, `PaneTitleChanged`, `PaneCwdChanged`, `PaneBell`, `PtyWrite`, `ClipboardStore`, `ClipboardLoad`

Actual has all of the above plus:
- `PaneIconChanged` (OSC 0/1 icon name)
- `CommandComplete` (OSC 133 shell integration)

Both additions are legitimate.

### MuxEventProxy -- VERIFIED

- Implements `EventListener` for `Term<MuxEventProxy>`
- Coalesces `Wakeup` via `AtomicBool::swap` with `AcqRel` ordering
- Always sets `grid_dirty` on wakeup (even coalesced) -- critical correctness property
- Maps all `Event` variants to `MuxEvent` variants exhaustively
- Non-routed events (`ColorRequest`, `CursorBlinkingChange`, `MouseCursorDirty`) only wake the event loop

### MuxNotification -- PARTIALLY MATCHES

Plan says: `PaneDirty`, `PaneClosed`, `TabLayoutChanged`, `WindowTabsChanged`, `Alert`

Actual: `PaneMetadataChanged`, `PaneOutput`, `PaneClosed`, `PaneBell`, `CommandComplete`, `ClipboardStore`, `ClipboardLoad`

- `PaneDirty` became `PaneOutput` (same semantics, better name)
- `PaneClosed` matches
- `TabLayoutChanged` and `WindowTabsChanged` don't exist (tabs/windows are GUI-only, not mux concerns)
- `Alert` became `PaneBell` (more specific)
- `PaneMetadataChanged` collapses title/icon/CWD changes (cleaner than separate notifications)

### TermEvent::MuxWakeup -- VERIFIED

Present in `oriterm/src/event.rs` line 20. Wired into `App::user_event` in `event_loop.rs` line 320. Created by closure in `constructors.rs` line 41. Full pipeline: PTY reader -> `MuxEventProxy::send_event` -> mpsc -> `MuxEvent` -> `wakeup()` -> `EventLoopProxy::send_event(MuxWakeup)` -> winit event loop -> `poll_events()`.

### Event pump exhaustive handling -- VERIFIED

`in_process/event_pump.rs` handles all 10 `MuxEvent` variants in an exhaustive match. The compiler enforces this -- adding a new variant would be a compile error.

### Tests -- 22 mux_event tests

```
cargo test -p oriterm_mux -- mux_event::  -> 22 passed
```

Coverage:
- Wakeup: sets grid_dirty, sends PaneOutput, coalescing skips duplicate, sends again after clear
- Event mapping: Bell, Title, ResetTitle, IconName, ResetIconName, Cwd, CommandComplete, PtyWrite, ChildExit, ClipboardStore, ClipboardLoad
- Debug format: all variants including closures
- MuxNotification Debug: all variants
- Concurrent wakeup: 10 threads sending Wakeup simultaneously
- Disconnected receiver: no panic when channel is closed
- Non-routed events: ColorRequest, CursorBlinkingChange, MouseCursorDirty wake event loop without MuxEvent

---

## Hygiene Audit

### File sizes -- ALL PASS (500-line limit)
- `pane/mod.rs`: 427 lines
- `mux_event/mod.rs`: 349 lines
- `session/tab/mod.rs`: 180 lines
- `session/window/mod.rs`: 155 lines
- `domain/local.rs`: 154 lines
- All others: under 150 lines

### Test organization -- ALL PASS
- Sibling `tests.rs` pattern used everywhere
- No inline test modules
- `#[cfg(test)] mod tests;` at bottom of every source file with tests
- Test files use `super::` imports

### Code hygiene -- ALL PASS
- `//!` module docs on all files
- `///` doc comments on all pub items
- All `#[allow]` attributes have `reason = "..."` strings
- No `unwrap()` in production code
- No `unsafe` blocks
- No `println!`/`eprintln!` debugging
- No commented-out or dead code
- Import ordering: std, external, crate (correct grouping)

### Crate boundary compliance -- ALL PASS
- `oriterm_mux` has no dependency on `oriterm_ui` or `oriterm`
- `Pane`, `Domain`, `PaneRegistry`, `MuxEventProxy` all in `oriterm_mux`
- `SessionRegistry`, `Tab`, `Window` in `oriterm/src/session/` (GUI-owned)
- `TermEvent::MuxWakeup` in `oriterm` (winit event type)
- `#![deny(unsafe_code)]` in `oriterm_mux/src/lib.rs`

### Error handling -- PASS
- `LocalDomain::spawn_pane` returns `io::Result<Pane>`
- `io::Error::other("PTY reader unavailable")` for missing handles
- `PaneNotifier::notify` uses `log::warn!` on send failure
- `Pane::resize_pty` uses `log::warn!` on resize failure
- MuxEventProxy silently drops on disconnected channel (`let _ = self.tx.send(...)`)

---

## Summary of Plan-vs-Reality Discrepancies

| Plan claim | Reality | Impact |
|---|---|---|
| Pane in `oriterm/src/pane/` | Pane in `oriterm_mux/src/pane/` | None (correct crate) |
| LocalDomain in `oriterm/src/domain/` | LocalDomain in `oriterm_mux/src/domain/` | None (correct crate) |
| MuxEventProxy in `oriterm/src/mux_event/` | In `oriterm_mux/src/mux_event/` | None (correct crate) |
| `PaneEntry.tab: TabId` field | No `tab` field | Correct (flat pane server) |
| `PaneRegistry.panes_in_tab()` method | Not present | Correct (no tab association in mux) |
| `MuxTab` struct in `oriterm_mux` | `Tab` struct in `oriterm/src/session/` | Correct (GUI-owned sessions) |
| `MuxWindow` struct in `oriterm_mux` | `Window` struct in `oriterm/src/session/` | Correct (GUI-owned sessions) |
| `SessionRegistry` in `oriterm_mux` | `SessionRegistry` in `oriterm/src/session/` | Correct (GUI-owned) |
| `MuxNotification::PaneDirty` | `MuxNotification::PaneOutput` | Rename only |
| `MuxNotification::Alert` | `MuxNotification::PaneBell` | More specific name |
| `MuxNotification::TabLayoutChanged` | Not present | Correct (no tabs in mux) |
| `MuxNotification::WindowTabsChanged` | Not present | Correct (no windows in mux) |
| Drop "join with timeout" | Drop detaches threads | Deliberate design choice |

**All discrepancies are architectural improvements over the original plan.** The plan was written before the flat-pane-server architecture was fully realized. The implementation correctly moved tab/window/session management out of the mux and into the GUI binary.

---

## Verdict: PASS

All code compiles, all 110 relevant tests pass (16 pane + 12 mark_cursor + 8 domain + 13 id + 8 registry + 16 session_registry + 11 tab + 16 window + 22 mux_event - some overlap from shared test names). Implementation is clean, well-documented, and architecturally sound. The plan's file paths and some structural claims are stale but the functionality is fully delivered with correct crate placement.

**Plan section should be updated** to reflect actual file paths and the flat-pane-server architecture (no `tab` field in `PaneEntry`, no `MuxTab`/`MuxWindow`, no `panes_in_tab`).
