---
section: "01"
title: "Terminal IO Thread Scaffold"
status: not-started
reviewed: true
goal: "Create the PaneIoThread struct, command enum, channels, and basic message loop — the foundation all subsequent sections build on"
inspired_by:
  - "Ghostty termio/Termio.zig (IO thread owns terminal state, receives messages via mailbox)"
  - "Ghostty termio/message.zig (typed message enum for resize, focus, etc.)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Command Enum"
    status: not-started
  - id: "01.2"
    title: "PaneIoThread Struct & Main Loop"
    status: not-started
  - id: "01.3"
    title: "Channel & Handle Types"
    status: not-started
  - id: "01.4"
    title: "Integration with Pane Spawn"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Terminal IO Thread Scaffold

**Status:** Not Started
**Goal:** Create the `PaneIoThread` struct, `PaneIoCommand` enum, command/byte channels, and the basic message loop. This section produces the skeleton that all subsequent sections wire into.

**Context:** Today, terminal state (`Term<MuxEventProxy>`) is wrapped in `Arc<FairMutex>` and contended between the PTY reader thread and the main thread. The IO thread replaces this with exclusive ownership — one thread owns `Term`, receives commands via channel, and produces snapshots. This section creates the thread infrastructure without yet moving any logic into it.

**Reference implementations:**
- **Ghostty** `src/termio/Termio.zig`: IO thread struct owns terminal state and a mailbox. Messages are typed enums (`resize`, `scroll`, etc.). The thread loops: drain mailbox → process PTY data → produce frame.
- **Ghostty** `src/termio/message.zig`: `Message` union with variants for resize, focus, color change, etc. Each variant carries the data needed to process the command.

**Depends on:** None (foundation section).

---

## 01.1 Command Enum

**File(s):** `oriterm_mux/src/pane/io_thread/commands.rs` (new)

Define the command enum that the main thread sends to the IO thread. Each variant corresponds to an operation that currently locks the terminal on the main thread.

- [ ] Create `oriterm_mux/src/pane/io_thread/` directory module
- [ ] Add `pub(crate) mod io_thread;` to `oriterm_mux/src/pane/mod.rs`
- [ ] Create `commands.rs` with `PaneIoCommand` enum:
  ```rust
  /// Commands sent from the main thread to the Terminal IO thread.
  ///
  /// Each variant replaces a `pane.terminal().lock()` call site on the
  /// main thread. The IO thread processes commands in order, mutates
  /// `Term<T>`, and produces a fresh snapshot.
  pub enum PaneIoCommand {
      /// Resize grid and notify PTY. IO thread does Grid::resize()
      /// with reflow, then sends SIGWINCH.
      Resize { rows: u16, cols: u16 },
      /// Change viewport scroll offset.
      ScrollDisplay(isize),
      /// Reset to live view (display_offset = 0).
      ScrollToBottom,
      /// Scroll to nearest prompt above viewport.
      ScrollToPreviousPrompt,
      /// Scroll to nearest prompt below viewport.
      ScrollToNextPrompt,
      /// Change theme and palette.
      SetTheme(oriterm_core::Theme, oriterm_core::Palette),
      /// Change cursor shape (from config or DECSCUSR).
      SetCursorShape(oriterm_core::CursorShape),
      /// Force all lines dirty (after config change, etc.).
      MarkAllDirty,
      /// Update image protocol configuration.
      SetImageConfig(crate::backend::ImageConfig),
      /// Extract text from selection (response via oneshot).
      ExtractText {
          selection: oriterm_core::Selection,
          reply: crossbeam_channel::Sender<Option<String>>,
      },
      /// Extract HTML+text from selection (response via oneshot).
      ExtractHtml {
          selection: oriterm_core::Selection,
          font_family: String,
          font_size: f32,
          reply: crossbeam_channel::Sender<Option<(String, String)>>,
      },
      /// Open search mode.
      OpenSearch,
      /// Close search mode.
      CloseSearch,
      /// Set search query (triggers match computation on IO thread).
      SearchSetQuery(String),
      /// Navigate to next search match.
      SearchNextMatch,
      /// Navigate to previous search match.
      SearchPrevMatch,
      /// Enter mark mode — reply with cursor position.
      EnterMarkMode {
          reply: crossbeam_channel::Sender<crate::pane::MarkCursor>,
      },
      /// Full terminal reset.
      Reset,
      /// Select the command output zone nearest to viewport center.
      SelectCommandOutput {
          reply: crossbeam_channel::Sender<Option<oriterm_core::Selection>>,
      },
      /// Select the command input zone nearest to viewport center.
      SelectCommandInput {
          reply: crossbeam_channel::Sender<Option<oriterm_core::Selection>>,
      },
      /// Shut down the IO thread (sent during pane close).
      Shutdown,
  }
  ```
- [ ] Derive `Debug` for the non-channel variants (use manual `Debug` impl that skips `reply` fields)
- [ ] Verify the enum is `Send` (all fields must be `Send`)

---

## 01.2 PaneIoThread Struct & Main Loop

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs` (new)

Create the IO thread struct and message loop skeleton. In this section, the thread does NOT own `Term` — it just drains commands and bytes (logging them). Sections 02-03 add `Term` ownership, VTE parsing, and snapshot production.

- [ ] Create `mod.rs` with module structure:
  ```rust
  //! Terminal IO thread — will own `Term<T>` and process commands + PTY bytes.
  //!
  //! In this initial scaffold, the thread drains channels without processing.
  //! Section 02 adds Term ownership and VTE parsing.
  //! Section 03 adds snapshot production.
  
  mod commands;
  pub use commands::PaneIoCommand;
  
  // ... (struct + impl defined below)
  
  #[cfg(test)]
  mod tests;
  ```

- [ ] Define `PaneIoThread` struct (skeleton — no `Term` yet):
  ```rust
  pub struct PaneIoThread {
      /// Receives commands from the main thread.
      cmd_rx: crossbeam_channel::Receiver<PaneIoCommand>,
      /// Receives raw PTY bytes from the reader thread.
      byte_rx: crossbeam_channel::Receiver<Vec<u8>>,
      /// Shutdown flag (shared with reader/writer threads).
      shutdown: Arc<AtomicBool>,
      /// Wakeup callback — signals main thread that a new snapshot is ready.
      wakeup: Arc<dyn Fn() + Send + Sync>,
  }
  ```
  Note: `terminal: Term<T>`, `pty_control: PtyControl`, VTE processors, and `mode_cache` are added in section 02 when the struct becomes generic over `T: EventListener`.

- [ ] Implement the main loop skeleton:
  ```rust
  impl PaneIoThread {
      pub fn run(mut self) {
          loop {
              // 1. Drain all pending commands (priority over bytes).
              while let Ok(cmd) = self.cmd_rx.try_recv() {
                  if matches!(cmd, PaneIoCommand::Shutdown) {
                      return;
                  }
                  self.handle_command(cmd);
              }
              
              // 2. Process one batch of PTY bytes.
              // Block if no commands and no bytes (thread sleeps).
              // Use crossbeam_channel::select! to wait on either channel.
              crossbeam_channel::select! {
                  recv(self.cmd_rx) -> msg => match msg {
                      Ok(PaneIoCommand::Shutdown) => return,
                      Ok(cmd) => self.handle_command(cmd),
                      Err(_) => return, // channel disconnected
                  },
                  recv(self.byte_rx) -> msg => match msg {
                      Ok(bytes) => self.handle_bytes(&bytes),
                      Err(_) => return, // channel disconnected
                  },
              }
          }
      }
      
      fn handle_command(&mut self, cmd: PaneIoCommand) {
          // Placeholder — filled in sections 05-06.
          match cmd {
              PaneIoCommand::Shutdown => unreachable!("handled above"),
              _ => log::trace!("IO thread: command {:?}", cmd),
          }
      }
      
      fn handle_bytes(&mut self, _bytes: &[u8]) {
          // Placeholder — filled in section 02.
      }
  }
  ```

- [ ] Add `spawn()` method that creates the thread:
  ```rust
  pub fn spawn(self) -> io::Result<JoinHandle<()>> {
      thread::Builder::new()
          .name("terminal-io".into())
          .spawn(move || self.run())
  }
  ```

- [ ] Create `oriterm_mux/src/pane/io_thread/tests.rs` with basic tests:
  - Test `PaneIoThread` shutdown via `PaneIoCommand::Shutdown`
  - Test channel disconnect causes clean exit
  - Test `PaneIoHandle::send_command()` delivers to the IO thread

### Tests

**File:** `oriterm_mux/src/pane/io_thread/tests.rs`

All tests use real channels and threads (no mocks). The IO thread is lightweight enough to spawn in tests.

- [ ] `test_shutdown_via_command` — send `PaneIoCommand::Shutdown`, assert the IO thread's `JoinHandle` completes within 1 second (no hang). Verify the thread does not panic.
- [ ] `test_shutdown_via_channel_disconnect` — drop the `PaneIoHandle` (which drops `cmd_tx` and `byte_tx`). Assert the IO thread exits cleanly (channel disconnected path). Verify `JoinHandle::join()` returns `Ok(())`.
- [ ] `test_command_delivery_ordering` — send 5 commands (`ScrollDisplay(1)` through `ScrollDisplay(5)`), then `Shutdown`. Assert the IO thread receives all 5 before exiting (use a counter `Arc<AtomicUsize>` incremented in `handle_command`).
- [ ] `test_byte_delivery` — send 3 byte batches via `byte_sender()`, then `Shutdown`. Assert the IO thread receives all 3 batches (use a counter in `handle_bytes`).
- [ ] `test_handle_drop_sends_shutdown` — create a `PaneIoHandle`, drop it via `Drop` impl. Assert the IO thread exits cleanly. This validates the RAII shutdown pattern.
- [ ] `test_pane_io_command_is_send` — static assertion: `fn assert_send<T: Send>() {} assert_send::<PaneIoCommand>();`. Compile-time verification that the command enum can cross thread boundaries.
- [ ] `test_pane_io_command_debug` — assert `format!("{:?}", PaneIoCommand::Resize { rows: 24, cols: 80 })` produces readable output. Verify the manual `Debug` impl handles reply-channel variants without panicking (use `ExtractText` with a real channel).

- [ ] `/tpr-review` checkpoint

---

## 01.3 Channel & Handle Types

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

Define the handle type that the main thread holds to communicate with the IO thread.

- [ ] Define `PaneIoHandle`:
  ```rust
  /// Main-thread handle to a Terminal IO thread.
  ///
  /// Provides non-blocking command sending. The IO thread processes
  /// commands in order and produces snapshots. Created by
  /// `PaneIoThread::spawn_with_handle()`.
  pub struct PaneIoHandle {
      /// Send commands to the IO thread.
      cmd_tx: crossbeam_channel::Sender<PaneIoCommand>,
      /// Send raw PTY bytes to the IO thread (used by reader thread).
      byte_tx: crossbeam_channel::Sender<Vec<u8>>,
      /// IO thread join handle.
      join: Option<JoinHandle<()>>,
  }
  
  impl PaneIoHandle {
      /// Send a command to the IO thread (non-blocking).
      pub fn send_command(&self, cmd: PaneIoCommand) {
          if let Err(e) = self.cmd_tx.send(cmd) {
              log::warn!("IO thread command send failed: {e}");
          }
      }
      
      /// Clone the byte sender for the PTY reader thread.
      pub fn byte_sender(&self) -> crossbeam_channel::Sender<Vec<u8>> { //
          self.byte_tx.clone()
      }
      
      /// Shut down the IO thread and wait for it to exit.
      pub fn shutdown(&mut self) {
          let _ = self.cmd_tx.send(PaneIoCommand::Shutdown);
          if let Some(handle) = self.join.take() {
              let _ = handle.join();
          }
      }
  }
  
  impl Drop for PaneIoHandle {
      fn drop(&mut self) {
          self.shutdown();
      }
  }
  ```

- [ ] Add a `PaneIoThread::new_with_handle()` factory that creates channels, builds the thread, and returns both:
  ```rust
  /// Create the IO thread and its main-thread handle.
  ///
  /// Channels are created here and split between the two sides.
  /// The caller spawns the thread via `PaneIoThread::spawn()`.
  pub fn new_with_handle(
      shutdown: Arc<AtomicBool>,
      wakeup: Arc<dyn Fn() + Send + Sync>,
  ) -> (Self, PaneIoHandle) {
      let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
      let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
      let thread = Self { cmd_rx, byte_rx, shutdown, wakeup };
      let handle = PaneIoHandle { cmd_tx, byte_tx, join: None };
      (thread, handle)
  }
  ```
  The caller (in `LocalDomain::spawn_pane()`) does:
  ```rust
  let (io_thread, mut io_handle) = PaneIoThread::new_with_handle(shutdown, wakeup);
  let join = io_thread.spawn()?;
  io_handle.join = Some(join);
  ```

- [ ] Add channel library dependency: Neither `flume` nor `crossbeam-channel` is currently in the workspace. Add `crossbeam-channel = "0.5"` to `oriterm_mux/Cargo.toml` — it provides `crossbeam_channel::select!` for multi-channel waiting. `crossbeam-channel` is the standard choice for MPMC channels with `select!` in the Rust ecosystem.

---

## 01.4 Integration with Pane Spawn

**File(s):** `oriterm_mux/src/pane/mod.rs`, `oriterm_mux/src/domain/local.rs`

Wire the IO thread creation into the pane spawn path. At this stage, the IO thread runs alongside the existing FairMutex path — both coexist. The IO thread receives commands but doesn't yet process them.

- [ ] Add `io_handle: Option<PaneIoHandle>` field to `Pane` struct
- [ ] Add `io_handle` to `PaneParts`
- [ ] In `LocalDomain::spawn_pane()` (`domain/local.rs`):
  - Create command and byte channels (`crossbeam_channel::unbounded()` for both)
  - Construct `PaneIoThread` WITHOUT a `Term` — at this stage the IO thread is a skeleton that drains channels but doesn't process anything. The `PaneIoThread` struct in this section has no `terminal` field; that is added in section 02.
  - Spawn the IO thread alongside the existing PTY event loop
- [ ] Verify `crossbeam-channel` compiles on all three targets: `cargo check --target x86_64-pc-windows-gnu -p oriterm_mux`, `cargo check -p oriterm_mux` (Linux), macOS CI
- [ ] Verify IO thread starts and shuts down cleanly:
  - `./test-all.sh` passes (IO thread coexists silently)
  - `./build-all.sh` passes on all platforms
  - `./clippy-all.sh` clean

---

## 01.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 01.N Completion Checklist

- [ ] `PaneIoCommand` enum defined with all variant types
- [ ] `PaneIoThread` struct compiles with message loop skeleton
- [ ] `PaneIoHandle` provides `send_command()` and `byte_sender()`
- [ ] IO thread spawns alongside existing pane spawn
- [ ] IO thread shuts down cleanly on pane close
- [ ] `timeout 150 cargo test -p oriterm_mux` passes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** The IO thread scaffold exists and runs. `PaneIoCommand` is defined with all variants needed by sections 05-06. The thread starts on pane spawn, drains commands, and shuts down on pane close. No behavioral changes yet — the existing FairMutex path is untouched.
