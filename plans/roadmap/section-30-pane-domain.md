---
section: 30
title: Pane Extraction + Domain System
status: complete
reviewed: true
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

**Crate:** `oriterm_mux` (domain trait, registries), `oriterm` (Pane struct, LocalDomain impl)
**Dependencies:** `oriterm_mux` (section 29), `oriterm_core` (Term, Grid), `portable-pty`
**Prerequisite:** Section 29 complete (IDs and layout types available).

**Absorbs:** Section 15.1 (Tab Struct + Lifecycle) — the Pane struct inherits all hard-won patterns: ConPTY-safe shutdown, mode cache, lock-free dirty flags, PtyWriter shared between threads.

**Inspired by:**
- WezTerm: `Domain` trait (`mux/src/domain.rs`), `LocalDomain`, `RemoteDomain`, `WslDomain`
- Alacritty: `Tab` struct with `Arc<FairMutex<Term<EventProxy>>>` + separate `PtyWriter`

---

## 30.1 Pane Struct Extraction

Extract all per-shell-session state from what would have been Tab into a dedicated Pane struct. A Pane is the atomic unit of the mux — one shell process, one grid, one PTY connection.

**File:** `oriterm/src/pane/mod.rs`, `oriterm/src/pane/shutdown.rs`

- [x] `Pane` struct fields:
  - [x] `id: PaneId` — globally unique (from `oriterm_mux::id`)
  - [x] `domain_id: DomainId` — which domain spawned this pane
  - [x] `terminal: Arc<FairMutex<Term<MuxEventProxy>>>` — thread-shared terminal state
  - [x] `notifier: PaneNotifier` — direct PTY writer + shutdown channel
  - [x] `pty_control: PtyControl` — PTY resize handle
  - [x] `reader_thread: Option<JoinHandle<()>>` — reader thread join handle
  - [x] `pty: PtyHandle` — child process lifecycle
  - [x] `grid_dirty: Arc<AtomicBool>` — lock-free, set by reader thread
  - [x] `wakeup_pending: Arc<AtomicBool>` — coalesces wakeup events
  - [x] `mode_cache: Arc<AtomicU32>` — lock-free cache of `TermMode::bits()`
  - [x] `selection: Option<Selection>` — main-thread-only
  - [x] `search: Option<SearchState>` — main-thread-only
  - [x] `mark_cursor: Option<MarkCursor>` — keyboard-driven selection
  - [x] `title: String` — pane title (from OSC 2)
  - [x] `cwd: Option<String>` — current working directory (from OSC 7)
  - [x] `has_bell: bool` — bell indicator
- [x] `PaneParts` struct — groups constructor params for clippy compliance
- [x] `PaneNotifier` — duplicated from `tab::Notifier` (independent of `tab`)
- [x] `Pane::from_parts(PaneParts)` — constructor from pre-built parts
- [x] `Drop for Pane` — shutdown, kill, join with timeout, reap child
- [x] Lock-free accessors: `grid_dirty()`, `clear_grid_dirty()`, `clear_wakeup()`, `mode()`, `refresh_mode_cache()`
- [x] All Tab methods mirrored: `write_input()`, `scroll_to_bottom()`, `resize_grid()`, `resize_pty()`, selection/search/mark_cursor methods

**Tests:** `oriterm/src/pane/tests.rs`
- [x] Lock-free dirty flag: set and clear round-trip
- [x] Wakeup coalescing: swap returns previous value
- [x] Mode cache: store and load round-trip
- [x] Cross-thread atomic visibility

---

## 30.2 Domain Trait + LocalDomain

**Files:**
- `oriterm_mux/src/domain/mod.rs` — trait, `DomainState`, `SpawnConfig`
- `oriterm_mux/src/id/mod.rs` — `DomainId` newtype
- `oriterm/src/domain/local.rs` — `LocalDomain`
- `oriterm/src/domain/wsl.rs` — `WslDomain` stub

- [x] `DomainId(u64)` newtype — full ID family pattern (Display, sealed, MuxId, from_raw/raw)
- [x] `DomainState` enum: `Attached`, `Detached`
- [x] `SpawnConfig` struct: `cols`, `rows`, `shell`, `cwd`, `env`, `scrollback`
- [x] `Domain` trait: `id()`, `name()`, `state()`, `can_spawn()` (no `spawn_pane` — that's concrete)
- [x] `LocalDomain` — spawns via `portable-pty`, wires `MuxEventProxy`, returns `Pane`
- [x] `WslDomain` stub — `can_spawn()` returns `false`, `state()` returns `Detached`

**Deferred:**
- `SerialDomain` — requires `serialport` crate dependency
- `cursor_shape` field in `SpawnConfig` — added when cursor config is wired

**Tests:** `oriterm_mux/src/domain/tests.rs`
- [x] MockDomain: attached can spawn, detached cannot
- [x] SpawnConfig defaults and custom values
- [x] Domain trait is object-safe

---

## 30.3 Pane + Session Registries

**Files:**
- `oriterm_mux/src/registry/mod.rs` — `PaneEntry`, `PaneRegistry`, `SessionRegistry`
- `oriterm_mux/src/session/mod.rs` — `MuxTab`, `MuxWindow`

- [x] `PaneEntry`: `pane: PaneId`, `tab: TabId`, `domain: DomainId`
- [x] `PaneRegistry`: register, unregister, get, panes_in_tab, len, is_empty
- [x] `MuxTab`: split tree + floating layer + active pane + undo stack (capped at 32)
  - [x] `new()`, `set_tree()` (pushes undo), `undo_tree()`, `all_panes()`
- [x] `MuxWindow`: tabs vec + active tab index
  - [x] `add_tab()`, `remove_tab()` (adjusts active), `set_active_tab_idx()`
- [x] `SessionRegistry`: tab CRUD + window CRUD + `window_for_tab`

**Tests:** `oriterm_mux/src/registry/tests.rs`, `oriterm_mux/src/session/tests.rs`
- [x] PaneRegistry: register/unregister/get lifecycle
- [x] PaneRegistry: panes_in_tab returns correct subset
- [x] MuxTab: new tab has single pane, set_tree pushes undo, undo restores
- [x] MuxTab: undo stack capped at 32
- [x] MuxWindow: add/remove tabs, active tab adjustment, clamping
- [x] SessionRegistry: add/get/remove tabs and windows, window_for_tab

---

## 30.4 MuxEventProxy

**Files:**
- `oriterm/src/mux_event/mod.rs` — `MuxEvent`, `MuxEventProxy`, `MuxNotification`
- `oriterm/src/event.rs` — added `TermEvent::MuxWakeup`

- [x] `MuxEvent` enum: `PaneOutput`, `PaneExited`, `PaneTitleChanged`, `PaneCwdChanged`, `PaneBell`, `PtyWrite`, `ClipboardStore`, `ClipboardLoad`
- [x] `MuxEventProxy` implements `EventListener`:
  - [x] Maps all `Event` variants to `MuxEvent` variants
  - [x] Coalesces `Wakeup` via `AtomicBool` swap
  - [x] Sets `grid_dirty` on every wakeup (even coalesced)
  - [x] Sends `TermEvent::MuxWakeup` to wake winit event loop
- [x] `MuxNotification` enum: `PaneDirty`, `PaneClosed`, `TabLayoutChanged`, `WindowTabsChanged`, `Alert`
- [x] `TermEvent::MuxWakeup` added, handled in `App::user_event`

**Tests:** `oriterm/src/mux_event/tests.rs`
- [x] Wakeup sets grid_dirty and sends PaneOutput
- [x] Wakeup coalescing skips duplicate send
- [x] Wakeup after clear sends again
- [x] Bell, Title, ResetTitle, PtyWrite, ChildExit, ClipboardStore, ClipboardLoad all map correctly
- [x] Debug format

---

## 30.5 Section Completion

- [x] All 30.1–30.4 items complete
- [x] Pane struct: clean extraction from Tab, all lock-free patterns preserved
- [x] Domain trait: defined in `oriterm_mux`, `LocalDomain` implemented in `oriterm`
- [x] `WslDomain` stub compiles (full impl in Section 35)
- [x] PaneRegistry and SessionRegistry: central state management with correct lookups
- [x] MuxEventProxy: bridges PTY reader → mux → GUI with coalescing
- [x] `./build-all.sh` — full workspace cross-compiles
- [x] `./clippy-all.sh` — no warnings
- [x] `./test-all.sh` — all tests pass (157 mux + 1556 oriterm)
- [x] No `unsafe` code
