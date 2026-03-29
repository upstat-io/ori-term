---
section: 30
title: Pane Extraction + Domain System
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 4M
goal: Extract Pane from Tab, define the Domain trait for shell spawning, implement LocalDomain, create PaneRegistry and SessionRegistry
sections:
  - id: "30.1"
    title: Pane Struct Extraction
    status: complete
  - id: "30.2"
    title: Domain Trait + LocalDomain
    status: complete
  - id: "30.3"
    title: Pane + Session Registries
    status: complete
  - id: "30.4"
    title: MuxEventProxy
    status: complete
  - id: "30.5"
    title: Section Completion
    status: complete
---

# Section 30: Pane Extraction + Domain System

**Status:** Complete
**Goal:** Extract the per-shell unit (Pane) from the current Tab struct. Define the Domain trait that abstracts where shells are spawned (local, WSL, SSH). Implement LocalDomain for the common case. Build PaneRegistry and SessionRegistry for mux-level state tracking.

**Crate:** `oriterm_mux` (Pane, domain trait, registries, LocalDomain), `oriterm` (SessionRegistry, Tab, Window — GUI-owned session)
**Dependencies:** `oriterm_mux` (section 29), `oriterm_core` (Term, Grid), `portable-pty`
**Prerequisite:** Section 29 complete (IDs and layout types available).

> **Stale file paths (verified 2026-03-29):** The plan places Pane, LocalDomain, and MuxEventProxy in `oriterm/`. The implementation correctly places them in `oriterm_mux/` (pane lifecycle belongs in the mux crate per CLAUDE.md crate boundaries). Conversely, Tab/Window/SessionRegistry live in `oriterm/src/session/` (GUI-owned), not in `oriterm_mux` as originally planned. All discrepancies are architectural improvements. File paths in the plan below reflect original aspirational locations.

**Absorbs:** Section 15.1 (Tab Struct + Lifecycle) — the Pane struct inherits all hard-won patterns: ConPTY-safe shutdown, mode cache, lock-free dirty flags, PtyWriter shared between threads.

**Inspired by:**
- WezTerm: `Domain` trait (`mux/src/domain.rs`), `LocalDomain`, `RemoteDomain`, `WslDomain`
- Alacritty: `Tab` struct with `Arc<FairMutex<Term<EventProxy>>>` + separate `PtyWriter`

---

## 30.1 Pane Struct Extraction

Extract all per-shell-session state from what would have been Tab into a dedicated Pane struct. A Pane is the atomic unit of the mux — one shell process, one grid, one PTY connection.

**Actual location:** `oriterm_mux/src/pane/mod.rs` (427 lines), `oriterm_mux/src/pane/shutdown.rs` (29 lines), `oriterm_mux/src/pane/selection.rs` (131 lines), `oriterm_mux/src/pane/mark_cursor/mod.rs` (39 lines)

- [x] `Pane` struct fields: (verified 2026-03-29)
  - [x] `id: PaneId` — globally unique (from `oriterm_mux::id`) (verified 2026-03-29)
  - [x] `domain_id: DomainId` — which domain spawned this pane (verified 2026-03-29)
  - [x] `terminal: Arc<FairMutex<Term<MuxEventProxy>>>` — thread-shared terminal state (verified 2026-03-29)
  - [x] `notifier: PaneNotifier` — direct PTY writer + shutdown channel (verified 2026-03-29)
  - [x] `pty_control: PtyControl` — PTY resize handle (verified 2026-03-29)
  - [x] `reader_thread: Option<JoinHandle<()>>` — reader thread join handle (verified 2026-03-29)
  - [x] `writer_thread: Option<JoinHandle<()>>` — writer thread join handle (verified 2026-03-29, addition beyond plan)
  - [x] `pty: PtyHandle` — child process lifecycle (verified 2026-03-29)
  - [x] `grid_dirty: Arc<AtomicBool>` — lock-free, set by reader thread (verified 2026-03-29)
  - [x] `wakeup_pending: Arc<AtomicBool>` — coalesces wakeup events (verified 2026-03-29)
  - [x] `mode_cache: Arc<AtomicU32>` — lock-free cache of `TermMode::bits()` (verified 2026-03-29)
  - [x] `selection: Option<Selection>` — main-thread-only (verified 2026-03-29)
  - [x] `search: Option<SearchState>` — main-thread-only (verified 2026-03-29)
  - [x] `mark_cursor: Option<MarkCursor>` — keyboard-driven selection (verified 2026-03-29)
  - [x] `title: String` — pane title (from OSC 2) (verified 2026-03-29)
  - [x] `icon_name: Option<String>` — icon name (OSC 0/1) (verified 2026-03-29, addition beyond plan)
  - [x] `cwd: Option<String>` — current working directory (from OSC 7) (verified 2026-03-29)
  - [x] `has_explicit_title: bool` — title priority logic (verified 2026-03-29, addition beyond plan)
  - [x] `last_command_duration: Option<Duration>` — shell integration (verified 2026-03-29, addition beyond plan)
  - [x] `has_bell: bool` — bell indicator (verified 2026-03-29)
  - [x] `last_pty_size: AtomicU32` — PTY resize dedup (verified 2026-03-29, addition beyond plan)
- [x] `PaneParts` struct — groups constructor params for clippy compliance (verified 2026-03-29)
- [x] `PaneNotifier` — sends via mpsc (verified 2026-03-29)
- [x] `Pane::from_parts(PaneParts)` — constructor from pre-built parts (verified 2026-03-29)
- [x] `Drop for Pane` — shutdown, kill, wait (blocking reap), detach threads (verified 2026-03-29) — NOTE: plan says "join with timeout" but implementation detaches threads; deliberate design: callers drop panes on background threads
- [x] Lock-free accessors: `grid_dirty()`, `clear_grid_dirty()`, `clear_wakeup()`, `mode()` — correct Acquire/Release ordering (verified 2026-03-29) — NOTE: no `refresh_mode_cache()` on Pane; mode cache updated by PTY event loop directly
- [x] All Tab methods mirrored: `write_input()`, `scroll_to_bottom()`, `resize_grid()`, `resize_pty()`, selection/search/mark_cursor methods (verified 2026-03-29)

**Tests:** `oriterm_mux/src/pane/tests.rs` (4 pane tests) + `oriterm_mux/src/pane/mark_cursor/tests.rs` (12 tests) = 16 total, ALL PASS (verified 2026-03-29)
- [x] Lock-free dirty flag: set and clear round-trip (verified 2026-03-29)
- [x] Wakeup coalescing: swap returns previous value (verified 2026-03-29)
- [x] Mode cache: store and load round-trip (verified 2026-03-29)
- [x] Cross-thread atomic visibility (actual thread spawn + Acquire) (verified 2026-03-29)
- [x] 12 mark_cursor `to_viewport` tests (bounds, overflow, edge cases) (verified 2026-03-29)

---

## 30.2 Domain Trait + LocalDomain

**Actual locations:**
- `oriterm_mux/src/domain/mod.rs` (87 lines) — trait, `DomainState`, `SpawnConfig`
- `oriterm_mux/src/id/mod.rs` (206 lines) — `DomainId` newtype
- `oriterm_mux/src/domain/local.rs` (154 lines) — `LocalDomain` (NOTE: plan said `oriterm/`, actually in `oriterm_mux/`)
- `oriterm_mux/src/domain/wsl.rs` (45 lines) — `WslDomain` stub

- [x] `DomainId(u64)` newtype — full ID family pattern (Display, sealed, MuxId, from_raw/raw), `DomainId::LOCAL` constant (verified 2026-03-29)
- [x] `DomainState` enum: `Attached`, `Detached` (verified 2026-03-29)
- [x] `SpawnConfig` struct: `cols`, `rows`, `shell`, `cwd`, `env`, `scrollback`, `shell_integration: bool` (verified 2026-03-29) — shell_integration is addition for OSC 133/7
- [x] `Domain` trait: `id()`, `name()`, `state()`, `can_spawn()` (no `spawn_pane` — that's concrete) (verified 2026-03-29)
- [x] `LocalDomain` — spawns via `portable-pty`, wires `MuxEventProxy`, returns `Pane` (verified 2026-03-29)
- [x] `WslDomain` stub — `can_spawn()` returns `false`, `state()` returns `Detached`, `#[allow(dead_code)]` with reason (verified 2026-03-29)

**Deferred:**
- `SerialDomain` — requires `serialport` crate dependency
- `cursor_shape` field in `SpawnConfig` — added when cursor config is wired

**Tests:** `oriterm_mux/src/domain/tests.rs` — 8 tests, ALL PASS (verified 2026-03-29)
- [x] MockDomain: attached can spawn, detached cannot (verified 2026-03-29)
- [x] SpawnConfig defaults and custom values (verified 2026-03-29)
- [x] Domain trait is object-safe (`Box<dyn Domain>`) (verified 2026-03-29)
- [x] SpawnConfig clone independence (verified 2026-03-29)
- [x] DomainState equality and copy semantics (verified 2026-03-29)
- [x] SpawnConfig empty env (verified 2026-03-29)

---

## 30.3 Pane + Session Registries

**Actual locations (significant divergence from plan — verified 2026-03-29):**
- `oriterm_mux/src/registry/mod.rs` (72 lines) — `PaneEntry`, `PaneRegistry` (flat pane storage only)
- `oriterm/src/session/registry/mod.rs` (149 lines) — `SessionRegistry` (GUI-owned, NOT in mux)
- `oriterm/src/session/tab/mod.rs` (180 lines) — `Tab` (replaces planned `MuxTab`, NOT in mux)
- `oriterm/src/session/window/mod.rs` (155 lines) — `Window` (replaces planned `MuxWindow`, NOT in mux)

> **Architectural divergence:** Plan placed `MuxTab`, `MuxWindow`, `SessionRegistry` in `oriterm_mux`. Implementation correctly keeps mux as flat pane server. Tab/window/session management is GUI-owned in `oriterm/src/session/`.

- [x] `PaneEntry`: `pane: PaneId`, `domain: DomainId` (verified 2026-03-29) — NOTE: NO `tab: TabId` field (correct: flat pane server has no tab knowledge)
- [x] `PaneRegistry`: register, unregister, get, len, is_empty (verified 2026-03-29) — NOTE: NO `panes_in_tab` (correct: no tab association in mux)
- [x] `Tab` (replaces `MuxTab`): split tree + floating layer + active pane + undo stack capped at 32 (MAX_UNDO_ENTRIES) (verified 2026-03-29)
  - [x] `new()`, `set_tree()` (pushes undo), `undo_tree()` (skips stale), `redo_tree()`, `all_panes()` (verified 2026-03-29)
  - [x] `replace_layout()` — non-undo tree replacement (verified 2026-03-29)
  - [x] `zoomed_pane` support (verified 2026-03-29)
- [x] `Window` (replaces `MuxWindow`): tabs vec + active tab index (verified 2026-03-29)
  - [x] `add_tab()`, `remove_tab()` (adjusts active), `set_active_tab_idx()` (clamps) (verified 2026-03-29)
  - [x] `insert_tab_at()`, `reorder_tab()`, `replace_tabs()` (verified 2026-03-29, beyond plan)
- [x] `SessionRegistry`: tab CRUD + window CRUD + `window_for_tab` + `window_for_pane` + `tab_for_pane` + `is_last_pane` (verified 2026-03-29)

**Tests:** 8 registry + 16 session_registry + 11 tab + 16 window = 51 total, ALL PASS (verified 2026-03-29)
- [x] PaneRegistry: register/unregister/get lifecycle (verified 2026-03-29)
- [ ] PaneRegistry: panes_in_tab returns correct subset — NOT APPLICABLE: `panes_in_tab` does not exist (flat pane server)
- [x] Tab: new tab has single pane, set_tree pushes undo, undo restores (verified 2026-03-29)
- [ ] Tab: undo stack capped at 32 — logic present (`set_tree()` checks `MAX_UNDO_ENTRIES` and `pop_front()`), but NO explicit test adds 33+ entries and checks truncation
- [x] Window: add/remove tabs, active tab adjustment, clamping (verified 2026-03-29)
- [x] SessionRegistry: add/get/remove tabs and windows, window_for_tab (verified 2026-03-29)
- [x] PaneRegistry: overwrite, multi-domain, stress (1000 entries) (verified 2026-03-29)
- [x] Tab: stale skip, replace_layout, redo clearing (verified 2026-03-29)
- [x] Window: insert_tab_at, reorder_tab, replace_tabs, edge cases (verified 2026-03-29)
- [x] SessionRegistry: window_for_pane, tab_for_pane, is_last_pane, ID allocation, default (verified 2026-03-29)

---

## 30.4 MuxEventProxy

**Actual locations:**
- `oriterm_mux/src/mux_event/mod.rs` (349 lines) — `MuxEvent`, `MuxEventProxy`, `MuxNotification` (NOTE: in `oriterm_mux`, not `oriterm`)
- `oriterm/src/event.rs` (58 lines) — `TermEvent::MuxWakeup`

- [x] `MuxEvent` enum: `PaneOutput`, `PaneExited`, `PaneTitleChanged`, `PaneCwdChanged`, `PaneBell`, `PtyWrite`, `ClipboardStore`, `ClipboardLoad` + `PaneIconChanged` (OSC 0/1) + `CommandComplete` (OSC 133) (verified 2026-03-29)
- [x] `MuxEventProxy` implements `EventListener`: (verified 2026-03-29)
  - [x] Maps all `Event` variants to `MuxEvent` variants exhaustively (verified 2026-03-29)
  - [x] Coalesces `Wakeup` via `AtomicBool::swap` with `AcqRel` ordering (verified 2026-03-29)
  - [x] Sets `grid_dirty` on every wakeup (even coalesced) — critical correctness property (verified 2026-03-29)
  - [x] Sends `TermEvent::MuxWakeup` to wake winit event loop (verified 2026-03-29)
  - [x] Non-routed events (`ColorRequest`, `CursorBlinkingChange`, `MouseCursorDirty`) wake event loop only (verified 2026-03-29)
- [x] `MuxNotification` enum (verified 2026-03-29) — NOTE: plan names differ from implementation:
  - [x] `PaneOutput` (plan said `PaneDirty` — same semantics, better name) (verified 2026-03-29)
  - [x] `PaneClosed` (matches plan) (verified 2026-03-29)
  - [x] `PaneMetadataChanged` — collapses title/icon/CWD changes (verified 2026-03-29)
  - [x] `PaneBell` (plan said `Alert` — more specific name) (verified 2026-03-29)
  - [x] `CommandComplete`, `ClipboardStore`, `ClipboardLoad` (verified 2026-03-29)
  - NOTE: `TabLayoutChanged` and `WindowTabsChanged` from plan do NOT exist (correct: tabs/windows are GUI-only, not mux concerns)
- [x] `TermEvent::MuxWakeup` added, handled in `App::user_event`, full pipeline wired (verified 2026-03-29)

**Tests:** `oriterm_mux/src/mux_event/tests.rs` — 22 tests, ALL PASS (verified 2026-03-29)
- [x] Wakeup sets grid_dirty and sends PaneOutput (verified 2026-03-29)
- [x] Wakeup coalescing skips duplicate send (verified 2026-03-29)
- [x] Wakeup after clear sends again (verified 2026-03-29)
- [x] Bell, Title, ResetTitle, IconName, ResetIconName, Cwd, CommandComplete, PtyWrite, ChildExit, ClipboardStore, ClipboardLoad all map correctly (verified 2026-03-29)
- [x] Debug format for all variants including closures (verified 2026-03-29)
- [x] MuxNotification Debug: all variants (verified 2026-03-29)
- [x] Concurrent wakeup: 10 threads sending Wakeup simultaneously (verified 2026-03-29)
- [x] Disconnected receiver: no panic when channel is closed (verified 2026-03-29)
- [x] Non-routed events: ColorRequest, CursorBlinkingChange, MouseCursorDirty wake event loop without MuxEvent (verified 2026-03-29)

---

## 30.5 Section Completion

- [x] All 30.1–30.4 items complete (verified 2026-03-29)
- [x] Pane struct: clean extraction from Tab, all lock-free patterns preserved (verified 2026-03-29)
- [x] Domain trait: defined in `oriterm_mux`, `LocalDomain` implemented in `oriterm_mux` (verified 2026-03-29)
- [x] `WslDomain` stub compiles (full impl in Section 35) (verified 2026-03-29)
- [x] PaneRegistry (mux) and SessionRegistry (GUI): central state management with correct lookups (verified 2026-03-29)
- [x] MuxEventProxy: bridges PTY reader -> mux -> GUI with coalescing (verified 2026-03-29)
- [x] `./build-all.sh` — full workspace cross-compiles (verified 2026-03-29)
- [x] `./clippy-all.sh` — no warnings (verified 2026-03-29)
- [x] `./test-all.sh` — all ~110 section-relevant tests pass (verified 2026-03-29)
- [x] No `unsafe` code — `#![deny(unsafe_code)]` in `oriterm_mux/src/lib.rs` (verified 2026-03-29)
- [x] All file sizes under 500 lines (max: `pane/mod.rs` at 427) (verified 2026-03-29)
- [x] Sibling `tests.rs` pattern used everywhere (verified 2026-03-29)
- [x] Crate boundary compliance: no dependency from `oriterm_mux` on `oriterm_ui` or `oriterm` (verified 2026-03-29)

**Gaps identified (verified 2026-03-29):**
- [ ] Missing test: undo stack capped at 32 — logic present in `Tab::set_tree()` but no test explicitly adds 33+ entries and checks truncation
