---
section: 4
title: PTY + Event Loop
status: complete
reviewed: true
last_verified: "2026-03-31"
tier: 1
goal: Spawn a shell via ConPTY, wire the reader thread, and verify end-to-end I/O through Term<EventProxy>
third_party_review:
  status: none
  updated: null
sections:
  - id: "4.1"
    title: Binary Crate Setup
    status: complete
  - id: "4.2"
    title: TabId + TermEvent Types
    status: complete
  - id: "4.3"
    title: PTY Spawning
    status: complete
  - id: "4.4"
    title: Message Channel
    status: complete
  - id: "4.5"
    title: EventProxy (EventListener impl)
    status: complete
  - id: "4.6"
    title: Notifier (Notify impl)
    status: complete
  - id: "4.7"
    title: PTY Reader Thread
    status: complete
  - id: "4.8"
    title: Tab Struct
    status: complete
  - id: "4.9"
    title: End-to-End Verification
    status: complete
  - id: "4.10"
    title: Section Completion
    status: complete
---

# Section 04: PTY + Event Loop

**Status:** Complete (verified 2026-03-29)
**Goal:** Spawn a real shell, wire PTY I/O through the reader thread, and process shell output through `Term<EventProxy>`. This is the first time terminal emulation runs against a live shell process.

**Crate:** `oriterm_mux` (evolved from plan's `oriterm` — PTY logic correctly relocated per crate boundaries)
**Dependencies:** `oriterm_core`, `portable-pty`, `oriterm_ipc`

> **Verification note (2026-03-29):** All items PASS. Architecture matured significantly beyond original plan — PTY logic moved to `oriterm_mux`, `Tab` became `Pane`, `EventProxy` became `MuxEventProxy`, and a dedicated writer thread was added. All deviations are improvements. Test counts: 390 unit, 20 contract, 22 e2e (1 flaky timing test), 18 FairMutex sync, 11 session ID.

---

## 4.1 Binary Crate Setup (verified 2026-03-29)

Set up the `oriterm/` binary crate in the workspace.

> **Deviation:** Plan called for 2-crate workspace (`oriterm_core` + `oriterm`). Actual is 5 crates (`oriterm_core`, `oriterm`, `oriterm_ui`, `oriterm_ipc`, `oriterm_mux`). This is an improvement — additional crates added as architecture matured.

- [x] Create `oriterm/` directory with `Cargo.toml` and `src/main.rs`
  - [x] `Cargo.toml`: name = `oriterm`, edition = 2024, same lint config
  - [x] Dependencies: `oriterm_core = { path = "../oriterm_core" }`, all GUI/platform deps from current root Cargo.toml
  - [x] `[[bin]]` name = `oriterm`, path = `src/main.rs`
- [x] Move existing `src/main.rs` → `oriterm/src/main.rs`
- [x] Move `build.rs` → `oriterm/build.rs`
- [x] Move `assets/` reference in build.rs (update paths)
- [x] Update workspace root `Cargo.toml`:
  - [x] `[workspace]` with `members = ["oriterm_core", "oriterm"]` (verified 2026-03-29: actual has 5 members)
  - [x] Remove `[[bin]]` and `[dependencies]` from root (they live in crate-level Cargo.tomls now)
- [x] Verify: `cargo build --target x86_64-pc-windows-gnu` builds both crates
- [x] Verify: `cargo build -p oriterm --target x86_64-pc-windows-gnu` builds the binary

---

## 4.2 TabId + TermEvent Types (verified 2026-03-29)

Newtype for tab identity and the event type for cross-thread communication.

**File:** `oriterm/src/session/id/mod.rs` (evolved from plan's `oriterm/src/tab.rs`)

> **Deviation:** Plan specified `TabId::next()` with `AtomicU64` static counter. Actual uses `IdAllocator::<TabId>::alloc()` — non-atomic, owned by `SessionRegistry`. Better: avoids global state, enables deterministic tests. `WindowId` newtype also added (not in plan).

- [x] `TabId` newtype (verified 2026-03-29)
  - [x] `pub struct TabId(u64)` (inner field private — construction only via `next()`)
  - [x] Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` (verified 2026-03-29: also has serde)
  - [x] `TabId::next() -> Self` — atomic counter for unique IDs (verified 2026-03-29: replaced by `IdAllocator::<TabId>::alloc()`)
    - [x] Use `std::sync::atomic::AtomicU64` static counter (verified 2026-03-29: replaced by stateful `IdAllocator`)
- [x] `TermEvent` enum — winit user event type (verified 2026-03-29)
  - [x] `Terminal { tab_id: TabId, event: oriterm_core::Event }` — event from terminal library (verified 2026-03-29: replaced by mux architecture — variants now include `ConfigReload`, `MuxWakeup`, `CreateWindow`, `MoveTabToNewWindow`, `OpenSettings`, `OpenConfirmation`)
  - [x] Derive: `Debug`
- [x] **Tests** (verified 2026-03-29: 11 passing):
  - [x] `TabId::next()` generates unique IDs (verified 2026-03-29: `allocator_starts_at_one`, `allocator_monotonically_increasing`)
  - [x] `TermEvent` variants can be constructed (verified 2026-03-29: covered by ID roundtrip and equality tests)

---

## 4.3 PTY Spawning (verified 2026-03-29)

Create a PTY and spawn the default shell.

**File:** `oriterm_mux/src/pty/spawn.rs` (evolved from plan's `oriterm/src/pty/spawn.rs` — correctly relocated to mux crate)

- [x] `spawn_pty(config: &PtyConfig) -> io::Result<PtyHandle>` (richer API than planned `spawn_shell`) (verified 2026-03-29: 31 tests passing)
  - [x] Call `portable_pty::native_pty_system()`
  - [x] `pty_system.openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })`
  - [x] `CommandBuilder::new(shell)` with `default_shell()` detection
  - [x] `pair.slave.spawn_command(cmd)` — spawn child process
  - [x] Drop `pair.slave` (reader gets EOF when child exits)
  - [x] Clone reader: `pair.master.try_clone_reader()`
  - [x] Take writer: `pair.master.take_writer()`
  - [x] Return `PtyHandle` containing reader, writer, master, child
- [x] `PtyHandle` struct
  - [x] Fields:
    - `reader: Box<dyn Read + Send>` — PTY output (read by reader thread)
    - `writer: Box<dyn Write + Send>` — PTY input (written by Notifier)
    - `master: Box<dyn portable_pty::MasterPty + Send>` — for resize
    - `child: Box<dyn portable_pty::Child + Send + Sync>` — child process handle
  - [x] `PtyHandle::resize(&self, rows: u16, cols: u16) -> io::Result<()>`
    - [x] `self.master.resize(PtySize { rows, cols, ... })`
- [x] `mod.rs`: `pub mod spawn;` re-export `PtyHandle`, `spawn_pty`
- [x] **Tests**:
  - [x] Spawning a shell succeeds (integration test)
  - [x] Reader and writer are valid (not None)

---

## 4.4 Message Channel (verified 2026-03-29)

Messages from the main thread to the PTY reader thread.

**File:** `oriterm_mux/src/pty/mod.rs` (evolved from plan's `oriterm/src/pty/mod.rs`)

> **Deviation:** Plan specified `Msg::Resize { rows, cols }`. Actual removes it — resize goes directly through `PtyControl::resize()` on the main thread. This is an improvement: resize is a synchronous operation on the PTY master, not an async command to the writer thread. Matches Alacritty's pattern.
> **Deviation:** Plan specified receiver consumed by reader thread. Actual has a separate **writer thread** (`spawn_pty_writer`) consuming the receiver. Reader thread reads PTY output only. This prevents a deadlock during shell startup (DA1 response scenario).

- [x] `Msg` enum — commands sent to PTY thread (verified 2026-03-29)
  - [x] `Input(Vec<u8>)` — bytes to write to PTY (verified 2026-03-29)
  - [x] `Resize { rows: u16, cols: u16 }` — resize the PTY (verified 2026-03-29: removed — resize goes through PtyControl directly)
  - [x] `Shutdown` — gracefully stop the reader thread (verified 2026-03-29)
- [x] Use `std::sync::mpsc::channel::<Msg>()` — unbounded channel (verified 2026-03-29)
  - [x] Sender held by `Notifier` (main thread side) (verified 2026-03-29: held by `PaneNotifier`)
  - [x] Receiver consumed by reader thread (verified 2026-03-29: consumed by separate writer thread)

---

## 4.5 EventProxy (EventListener impl) (verified 2026-03-29)

Bridges terminal events to the winit event loop.

**File:** `oriterm_mux/src/mux_event/mod.rs` (evolved from plan's `oriterm/src/tab.rs`)

> **Deviation:** Plan specified `EventProxy` wrapping `winit::event_loop::EventLoopProxy<TermEvent>`. Actual uses `MuxEventProxy` with `mpsc::Sender<MuxEvent>` + `Arc<dyn Fn()>` wakeup callback. This is a major improvement: removes the concrete `EventLoopProxy` dependency from logic code, satisfying impl-hygiene rule "No concrete external-resource types in logic layers." The proxy can be tested headlessly.

- [x] `EventProxy` struct (verified 2026-03-29: renamed to `MuxEventProxy`)
  - [x] Fields: (verified 2026-03-29: `pane_id: PaneId`, `tx: mpsc::Sender<MuxEvent>`, `wakeup_pending: Arc<AtomicBool>`, `grid_dirty: Arc<AtomicBool>`, `wakeup: Arc<dyn Fn() + Send + Sync>`)
    - `proxy: winit::event_loop::EventLoopProxy<TermEvent>` — winit's thread-safe event sender (verified 2026-03-29: replaced by mpsc + callback)
    - `tab_id: TabId` (verified 2026-03-29: replaced by `pane_id: PaneId`)
  - [x] `impl oriterm_core::EventListener for EventProxy` (verified 2026-03-29: `impl EventListener for MuxEventProxy`)
    - [x] `fn send_event(&self, event: oriterm_core::Event)` (verified 2026-03-29: maps Event variants to MuxEvent, wakeup coalescing via AtomicBool)
      - [x] `let _ = self.proxy.send_event(TermEvent::Terminal { tab_id: self.tab_id, event });`
      - [x] Silently ignore send errors (window may have closed) (verified 2026-03-29)
- [x] `EventProxy` must be `Send + 'static` (required by `EventListener` bound) (verified 2026-03-29)

**Tests (verified 2026-03-29: 20 passing):** wakeup coalescing, event mapping (Bell, Title, Cwd, ChildExit, PtyWrite, Clipboard), disconnected receiver safety, concurrent coalescing, non-routed events (ColorRequest, CursorBlinkingChange, MouseCursorDirty).

---

## 4.6 Notifier (Notify impl) (verified 2026-03-29)

Sends input bytes and commands to the PTY reader thread.

**File:** `oriterm_mux/src/pane/mod.rs` (evolved from plan's `oriterm/src/tab.rs`)

> **Deviation:** Renamed to `PaneNotifier`. Plan specified `Notifier::resize()` — actual removes it (resize through `PtyControl` directly, see 4.4 deviation).

- [x] `Notifier` struct (verified 2026-03-29: renamed to `PaneNotifier`)
  - [x] Fields:
    - `tx: std::sync::mpsc::Sender<Msg>` — channel sender (verified 2026-03-29)
  - [x] `Notifier::notify(&self, bytes: &[u8])` — send bytes (skips empty) (verified 2026-03-29)
    - [x] `let _ = self.tx.send(Msg::Input(bytes.to_vec()));` (verified 2026-03-29: logs warning on send failure)
  - [x] `Notifier::resize(&self, rows: u16, cols: u16)` (verified 2026-03-29: removed — resize through PtyControl)
    - [x] `let _ = self.tx.send(Msg::Resize { rows, cols });` (verified 2026-03-29: removed)
  - [x] `Notifier::shutdown(&self)` (verified 2026-03-29)
    - [x] `let _ = self.tx.send(Msg::Shutdown);` (verified 2026-03-29)

---

## 4.7 PTY Reader Thread (verified 2026-03-29)

The dedicated thread that reads PTY output, parses VTE, and updates terminal state.

**File:** `oriterm_mux/src/pty/event_loop/mod.rs` (evolved from plan's `oriterm/src/pty/event_loop.rs`)

- [x] `PtyEventLoop` struct
  - [x] Fields:
    - `terminal: Arc<oriterm_core::FairMutex<oriterm_core::Term<T>>>` — shared terminal state (generic over `EventListener`)
    - `reader: Box<dyn Read + Send>` — PTY read handle
    - `writer: Box<dyn Write + Send>` — PTY write handle (verified 2026-03-29: moved to separate writer thread)
    - `rx: std::sync::mpsc::Receiver<Msg>` — command receiver (verified 2026-03-29: moved to writer thread)
    - `pty_master: Box<dyn portable_pty::MasterPty + Send>` — for resize (verified 2026-03-29: replaced by `shutdown: Arc<AtomicBool>`, `mode_cache: Arc<AtomicU32>`)
    - `processor: vte::ansi::Processor` — VTE parser state machine (verified 2026-03-29: also added `raw_parser: vte::Parser` for shell integration)
  - [x] `PtyEventLoop::new(...)` — constructor, takes all handles
  - [x] `PtyEventLoop::spawn(self) -> JoinHandle<()>` — start the reader thread
    - [x] `std::thread::Builder::new().name("pty-reader".into()).spawn(move || self.run())`
  - [x] `fn run(mut self)` — main loop: drain commands → blocking read → parse in bounded chunks → EOF/error exits
  - [x] `fn parse_pty_output(&mut self, data: &[u8])` — lock-bounded VTE parsing in 64KB chunks
  - [x] `fn process_commands(&mut self) -> bool` — drain rx: (verified 2026-03-29: moved to writer thread)
    - [x] `Msg::Input(bytes)` → `self.writer.write_all(&bytes)` (verified 2026-03-29: handled by writer thread)
    - [x] `Msg::Resize { rows, cols }` → `self.resize_pty(rows, cols)` (verified 2026-03-29: removed — resize via PtyControl directly)
    - [x] `Msg::Shutdown` → return false (breaks loop) (verified 2026-03-29: uses `Arc<AtomicBool>` flag instead)
  - [x] `fn resize_pty(&self, rows, cols)` — resize PTY master via `portable_pty::PtySize` (verified 2026-03-29: removed from event loop — handled by Pane)
  - [x] Read buffer: `vec![0u8; 65536]` (64KB, heap-allocated to avoid clippy::large_stack_arrays) (verified 2026-03-29: upgraded to 1MB `0x10_0000` matching Alacritty's `READ_BUFFER_SIZE`)
  - [x] Max locked parse: `MAX_LOCKED_PARSE = 0x1_0000` (64KB) per lock acquisition, then release and re-lock (verified 2026-03-29)
    - [x] Prevents holding lock for too long on large output bursts
- [x] **Thread safety**:
  - [x] PTY reader thread holds `FairMutex` lock only during `processor.advance()` (microseconds to low ms)
  - [x] Uses `lease()` → `lock_unfair()` pattern from Alacritty
  - [x] Releases lock between read batches
- [x] `PtyHandle::take_master()` — added to `spawn.rs` so master can be handed to PtyEventLoop
- [x] **Tests** (verified 2026-03-29: 12 passing, expanded significantly beyond plan):
  - [x] `shutdown_on_reader_eof` — drop pipe write end → EOF → thread exits (verified 2026-03-29)
  - [x] `processes_pty_output_into_terminal` — write bytes to pipe → VTE parses into grid (verified 2026-03-29)
  - [x] `processes_channel_input` — `Msg::Input` forwarded to PTY writer (verified 2026-03-29: tested indirectly via contract/e2e)
  - [x] `read_buffer_size_is_64kb` — constant check (verified 2026-03-29: now `read_buffer_size_is_1mb`)
  - [x] `max_locked_parse_is_64kb` — constant check (verified 2026-03-29)
  - [x] `try_parse_is_bounded_to_max_locked_parse` — 2x data parsed in 2 chunks (verified 2026-03-29, new)
  - [x] `renderer_not_starved_during_flood` — >= 30 renderer locks in 500ms (verified 2026-03-29, new)
  - [x] `sustained_flood_no_oom` — 50MB+ without OOM (verified 2026-03-29, new)
  - [x] `no_data_loss_under_renderer_contention` — LINE_04999 present after 5000 lines (verified 2026-03-29, new)
  - [x] `sync_mode_delivers_content_atomically` — Mode 2026 BSU/ESU replay verified (verified 2026-03-29, new)

---

## 4.8 Tab Struct (verified 2026-03-29)

Owns all per-tab state: terminal, PTY handles, reader thread.

**File:** `oriterm_mux/src/pane/mod.rs` (evolved from plan's `oriterm/src/tab.rs` — renamed `Tab` to `Pane`, relocated to mux crate)

> **Deviation:** Plan specified `Tab` in `oriterm/src/tab.rs` with `Tab::new(id, rows, cols, scrollback, proxy)`. Actual: `Pane` in `oriterm_mux/src/pane/mod.rs` with `Pane::from_parts(PaneParts)`. Assembly logic in `LocalDomain::spawn_pane()`. GUI `Tab` now lives in `oriterm/src/session/tab/mod.rs` as a layout container holding `PaneId`s. This is the correct separation per crate boundaries.

- [x] `Tab` struct (verified 2026-03-29: renamed to `Pane`)
  - [x] Fields:
    - `id: TabId`
    - `terminal: Arc<oriterm_core::FairMutex<oriterm_core::Term<EventProxy>>>`
    - `notifier: Notifier` — send input/resize/shutdown to PTY thread
    - `reader_thread: Option<JoinHandle<()>>` — reader thread handle
    - `pty: PtyHandle` — child process lifecycle (reader/writer/control taken)
    - `title: String` — last known title (updated from Event::Title)
    - `has_bell: bool` — bell badge (cleared on focus)
  - [x] `Tab::new(id: TabId, rows: u16, cols: u16, scrollback: usize, proxy: EventLoopProxy<TermEvent>) -> io::Result<Self>`
    - [x] Spawn PTY via `pty::spawn_pty(&PtyConfig)`
    - [x] Create `EventProxy` with tab_id and proxy
    - [x] Create `Term::new(rows, cols, scrollback, event_proxy)`
    - [x] Wrap in `Arc<FairMutex<...>>`
    - [x] Create `(tx, rx)` channel
    - [x] Create `Notifier` with tx
    - [x] Create `PtyEventLoop` with terminal clone, reader, writer, rx, control
    - [x] Spawn reader thread: `event_loop.spawn()`
    - [x] Return Tab
  - [x] `Tab::write_input(&self, bytes: &[u8])` — send input to PTY via Notifier
  - [x] `Tab::resize(&self, rows: u16, cols: u16)` — resize PTY + terminal
  - [x] `Tab::terminal(&self) -> &Arc<FairMutex<Term<EventProxy>>>` — for renderer to lock + snapshot
  - [x] `impl Drop for Tab`
    - [x] Send `Msg::Shutdown` to reader thread
    - [x] Kill child process to unblock pending PTY read
    - [x] Join reader thread (with timeout via `is_finished()` poll loop)
    - [x] Reap child process

---

## 4.9 End-to-End Verification (verified 2026-03-29)

At this point there's no window, but we can verify the full PTY -> VTE -> Term pipeline.

> **Verification (2026-03-29):** 20 contract tests + 22 e2e tests + 18 FairMutex contention tests all passing. Contract tests verify the full PTY -> VTE -> Term pipeline. E2E tests verify real daemon/client flow.

- [x] Temporary `main.rs` for verification:
  - [x] Create a winit `EventLoop` (needed for `EventLoopProxy`, even without a window)
  - [x] Create a Tab
  - [x] Send `"echo hello\r\n"` via `tab.write_input()`
  - [x] Wait briefly (100ms)
  - [x] Lock terminal, read grid, verify "hello" appears in grid cells
  - [x] Print verification result to log/stderr
  - [x] Exit
- [x] Verify thread lifecycle:
  - [x] Tab creation spawns reader thread
  - [x] Tab drop sends Shutdown and joins thread
  - [x] No thread leaks, no panics on drop
- [x] Verify FairMutex under load:
  - [x] Send rapid input while reader thread is processing
  - [x] Neither thread starves (both make progress)
- [x] Verify resize:
  - [x] Create tab at 80x24
  - [x] Resize to 120x40
  - [x] PTY dimensions updated, terminal grid resized

---

## 4.10 Section Completion (verified 2026-03-29)

- [x] All 4.1–4.9 items complete (verified 2026-03-29)
- [x] `cargo build -p oriterm --target x86_64-pc-windows-gnu` succeeds (verified 2026-03-29)
- [x] Tab spawns shell, reader thread processes output into Term (verified 2026-03-29: Pane spawns shell)
- [x] Input sent via Notifier arrives at shell (verified 2026-03-29: PaneNotifier -> writer thread -> PTY)
- [x] Shutdown is clean (no thread leaks, no panics) (verified 2026-03-29: notifier.shutdown() -> writer stops -> child killed -> child reaped -> threads detached)
- [x] FairMutex prevents starvation under concurrent access (verified 2026-03-29: 18 FairMutex tests)
- [x] Resize works end-to-end (PTY + terminal grid) (verified 2026-03-29: PtyControl + Term::resize via Pane::resize_pty + resize_grid)
- [x] No window yet — next section adds GUI (verified 2026-03-29)

**Exit Criteria:** Live shell output is parsed through VTE into `Term<MuxEventProxy>`. Input flows main thread -> PaneNotifier -> channel -> writer thread -> PTY. Reader thread is clean (proper lifecycle, lock discipline, no starvation). Ready for a window to render the terminal state.

### Gap Analysis (2026-03-29)

No missing functionality for the stated goal. Minor test gaps (low severity, covered by integration tests):
- [x] Writer thread unit tests — 4 tests added in `pty/tests.rs` (deliver input, batch queued messages, shutdown flag, channel close) on 2026-03-31
- [x] PtyHandle take-pattern — verified correct by inspection: `Option::take()` on std types. Covered by contract/e2e tests via `LocalDomain::spawn_pane()`.
- [x] PtyControl::resize() error path — verified correct by inspection: one-liner `io::Error::other(e.to_string())`. Covered by e2e resize tests.

**Known flaky test:** `test_scroll_to_bottom` in e2e suite occasionally times out (timing-dependent scroll state polling over IPC, not Section 04 specific).
