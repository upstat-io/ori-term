---
section: "01"
title: "Shared PtySession Infrastructure"
status: not-started
reviewed: true
goal: "Create crates/oriterm_test_support with a shared PtySession (PTY+Term+VTE driver), then migrate both vttest text tests and vttest GPU golden tests onto it. Eliminate the VtTestSession LEAK between oriterm_core/tests/vttest/session.rs and oriterm/src/gpu/visual_regression/vttest/mod.rs (581 lines, ~240 lines duplicated byte-for-byte). Zero behavioral change to existing 198 insta snapshots and 98 golden PNGs."
success_criteria:
  - "`crates/oriterm_test_support/` crate exists and is registered as a workspace member"
  - "`PtySession::new(cmd, env, cols, rows)` spawns a child process under PTY and produces a working session usable by both text and GPU tests"
  - "`VtTestSession` is defined ONCE only (in `oriterm_core/tests/vttest/session.rs`) — no duplicate in `oriterm/src/gpu/visual_regression/vttest/mod.rs`. Both adapt the shared `PtySession`."
  - "`oriterm/src/gpu/visual_regression/vttest/mod.rs` is below the 500-line file limit after the PtySession layer is extracted (target: <300 lines)"
  - "`timeout 150 cargo test -p oriterm_core --test vttest` passes with all 198 snapshots unchanged (`INSTA_FORCE_PASS=0`, no `.snap.new` files generated)"
  - "`timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden` passes with all 98 PNG golden references unchanged (no diffs against `oriterm/tests/references/vttest_*.png`)"
  - "`./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` all green — zero regressions"
  - "Satisfies mission criteria: 'shared PtySession infrastructure', 'vttest text tests migrated', 'vttest GPU golden tests migrated', 'VtTestSession duplication eliminated (LEAK fixed)'"
inspired_by:
  - "ori_term VtTestSession (oriterm_core/tests/vttest/session.rs:40-213) — current canonical PTY+Term+VTE driver pattern"
  - "ori_term GPU VtTestSession (oriterm/src/gpu/visual_regression/vttest/mod.rs:53-284) — duplicate that this section eliminates"
  - "Alacritty extra/alacritty.info (alacritty/extra/alacritty.info, 112 lines) — terminfo source convention referenced for Section 02"
  - "WezTerm shared termwiz crate (wezterm/termwiz/) — workspace-internal test/util crate pattern"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Create crates/oriterm_test_support workspace member"
    status: not-started
  - id: "01.2"
    title: "PtySession core: spawn, drain, wait, send, grid_text"
    status: not-started
  - id: "01.3"
    title: "Migrate oriterm_core vttest text tests onto PtySession"
    status: not-started
  - id: "01.4"
    title: "Migrate oriterm vttest GPU golden tests onto PtySession"
    status: not-started
  - id: "01.5"
    title: "Verify zero behavioral change (snapshot + PNG byte-equality)"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Shared PtySession Infrastructure

**Status:** Not Started
**Goal:** Eliminate the `VtTestSession` LEAK by introducing a shared `crates/oriterm_test_support` crate. Both `oriterm_core/tests/vttest/` and `oriterm/src/gpu/visual_regression/vttest/` consume the shared `PtySession`. The 581-line `oriterm/src/gpu/visual_regression/vttest/mod.rs` (BLOAT) shrinks below the 500-line limit because the PTY/Term/VTE plumbing is no longer duplicated. After this section, every byte of the existing 198 insta snapshots and 98 golden PNGs is unchanged.

**Success Criteria:**

- [ ] `crates/oriterm_test_support` exists, builds, and is in `Cargo.toml` `workspace.members`
- [ ] `PtySession` exposes spawn/drain/wait/send/grid_text/grid_chars APIs that exactly match `VtTestSession`'s current behavior
- [ ] `oriterm_core/tests/vttest/session.rs` reduces to a thin adapter (`VtTestSession` constructed from `PtySession::spawn_vttest()`)
- [ ] `oriterm/src/gpu/visual_regression/vttest/mod.rs` no longer defines its own `PtyResponder` or `VtTestSession` — only GPU-specific helpers (`assert_golden`, `frame_input`, `frame_input_with_blink`, `cell_brightness`)
- [ ] `oriterm/src/gpu/visual_regression/vttest/mod.rs` line count drops from 581 → <300
- [ ] `timeout 150 cargo test -p oriterm_core --test vttest` — 198 insta snapshots all match (no new `.snap.new` files generated)
- [ ] `timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden` — 98 PNG goldens all match (zero pixel diffs)
- [ ] Architecture test (`cargo test -p oriterm --test architecture`, if it gates crate boundaries) still green
- [ ] Satisfies mission criteria #1, #2, #3, #4

**Context:** Two copies of `VtTestSession` currently exist. The text-test version at `oriterm_core/tests/vttest/session.rs:40-213` (239 lines) and the GPU-test version at `oriterm/src/gpu/visual_regression/vttest/mod.rs:53-284` (within a 581-line file). The bodies of `drain`, `drain_blocking`, `wait`, `wait_for`, `send`, `grid_text`, and the `PtyResponder` listener are byte-for-byte identical between the two. This is a `LEAK:algorithmic-duplication` (cross-crate, no shared home) AND a `BLOAT` finding (the GPU file exceeds the 500-line limit by 81 lines because it absorbs the PTY plumbing). Per `.claude/rules/impl-hygiene.md`: "Cross-crate duplication: even 2 instances = extract to a shared crate or shared type." This section is the extract.

The shared crate cannot live inside `oriterm_core` because the GPU tests in `oriterm/` need it too, and a `dev-dependency = oriterm_core` from `oriterm` is fine but having `oriterm_core` provide test-only types via its own dev-dependencies would not be reusable from `oriterm`. The clean answer is a new workspace-internal crate that both consume as a dev-dependency: `crates/oriterm_test_support`.

**Reference implementations:**
- **ori_term** `oriterm_core/tests/vttest/session.rs:1-239`: the canonical text-test session pattern (the source-of-truth this section preserves).
- **ori_term** `oriterm/src/gpu/visual_regression/vttest/mod.rs:1-300` (lines 53-284 are the duplicate): the GPU-test session pattern (the duplicate this section deletes).
- **WezTerm** `wezterm/termwiz/`: workspace-internal helper crate consumed across the wider workspace — the `crates/<name>` directory pattern this section adopts.
- **ori_term** `crates/portable-pty/`, `crates/vte/`, `crates/wgpu-hal/`: existing `crates/` subdirectory currently holds vendored externals; we add the first INTERNAL crate alongside them.

**Depends on:** None. This is the foundation section — every later section consumes the shared `PtySession`.

---

## 01.1 Create crates/oriterm_test_support workspace member

**File(s):** `Cargo.toml` (workspace), `crates/oriterm_test_support/Cargo.toml`, `crates/oriterm_test_support/src/lib.rs`

This subsection creates the workspace member with no logic in it — pure scaffolding so subsection 01.2 can land code into a buildable target.

- [ ] Create directory `crates/oriterm_test_support/` and `crates/oriterm_test_support/src/`.
  Verify before creating: the existing `crates/` already contains `portable-pty/`, `vte/`, `wgpu-hal/` (vendored externals). The new directory sits alongside them as the FIRST workspace-internal crate under `crates/`.

- [ ] Create `crates/oriterm_test_support/Cargo.toml`:
  ```toml
  [package]
  name = "oriterm_test_support"
  version.workspace = true
  edition.workspace = true
  description = "Shared PTY/Term/VTE test driver for oriterm conformance suites (vttest, tack, teseq)"
  license.workspace = true
  publish = false

  [dependencies]
  oriterm_core = { path = "../../oriterm_core" }
  portable-pty = "0.9.0"
  vte = { version = "0.15.0", features = ["ansi"] }

  [lints]
  workspace = true
  ```

  Notes:
  - `publish = false` — this is a workspace-internal test helper, never published to crates.io.
  - Depends on `oriterm_core` (for `Term`, `Theme`, `Event`, `EventListener`, `RenderableContent`).
  - Depends on `portable-pty` and `vte` directly because the shared session OWNS the PTY pair, the writer, the receiver thread, the `Term`, and the `vte::ansi::Processor`.
  - Allowed dependency direction (per `.claude/rules/crate-boundaries.md`): `oriterm_test_support → oriterm_core`. The new crate must NOT depend on `oriterm_ui`, `oriterm_mux`, or `oriterm` — it stays at the same architectural layer as `oriterm_core` (lower than UI/GPU).

- [ ] Update workspace `Cargo.toml` (root `/home/eric/projects/ori_term/Cargo.toml`):
  ```toml
  [workspace]
  members = [
      "oriterm_core",
      "oriterm",
      "oriterm_ui",
      "oriterm_ipc",
      "oriterm_mux",
      "crates/oriterm_test_support",
  ]
  ```
  The new entry uses the `crates/<name>` path explicitly because the existing 5 members are top-level directories — adding a `crates/`-rooted member is intentional to keep test infrastructure visually grouped under `crates/`.

- [ ] Create `crates/oriterm_test_support/src/lib.rs` as an empty index file with module declarations placeholder (the actual modules land in 01.2):
  ```rust
  //! Shared PTY/Term/VTE test driver for oriterm conformance suites.
  //!
  //! This crate is a dev-time helper, never published. It provides a single
  //! canonical [`PtySession`] type used by:
  //!   - `oriterm_core/tests/vttest/` (text-grid snapshot tests)
  //!   - `oriterm/src/gpu/visual_regression/vttest/` (GPU golden image tests)
  //!   - `oriterm_core/tests/tack/` (tack scenario catalog — Section 04+)
  //!
  //! Before this crate existed, the PTY/Term/VTE plumbing was duplicated
  //! byte-for-byte between two `VtTestSession` definitions. See
  //! `plans/tack-conformance/section-01-shared-pty-session.md` for the
  //! deduplication history.

  pub mod session;

  pub use session::{PtyResponder, PtySession};
  ```

- [ ] Stub `crates/oriterm_test_support/src/session.rs` with empty re-exports so the crate compiles cleanly before 01.2:
  ```rust
  //! Cross-suite PTY+Term+VTE driver. See [`PtySession`].

  // Implementation lands in 01.2.
  pub struct PtyResponder;
  pub struct PtySession;
  ```
  This keeps the build green between 01.1 and 01.2 while allowing 01.1 to be reviewed and committed independently if needed.

- [ ] Verify: `cargo build -p oriterm_test_support` succeeds with no warnings.
- [ ] Verify: `cargo metadata --format-version 1` shows the new member.
- [ ] Verify: `./build-all.sh` still green (the new crate must not break workspace compilation).
- [ ] Verify: `./clippy-all.sh` still green (no new warnings from the empty crate).

---

## 01.2 PtySession core: spawn, drain, wait, send, grid_text

**File(s):** `crates/oriterm_test_support/src/session.rs`

This is the heart of the deduplication. `PtySession` becomes the SINGLE canonical implementation of: spawn child under PTY, route bytes through `vte::ansi::Processor` into `Term<PtyResponder>`, write `PtyWrite` events back to the PTY, wait for grid contents, and serialize the grid to text. Both `oriterm_core` text tests and `oriterm` GPU tests adapt this same type.

**Pattern:** the implementation is a near-verbatim port of `oriterm_core/tests/vttest/session.rs:40-213`, with two changes:
1. The constructor is generalized: `new(cmd: CommandBuilder, cols: u16, rows: u16) -> Self` instead of hardcoding `vttest`. Helper constructors `spawn_vttest(...)`, `spawn_tack(...)` (Section 03), `spawn_command(...)` etc. wrap it.
2. `Term<PtyResponder>` is exposed via `pub fn term(&self) -> &Term<PtyResponder>` and `pub fn term_mut(&mut self) -> &mut Term<PtyResponder>` so adapters in `oriterm_core` and `oriterm` can both reach the underlying terminal without taking ownership.

- [ ] Define `PtyResponder` (verbatim port from `session.rs:13-37`):
  ```rust
  use std::io::{Read, Write};
  use std::sync::{Arc, Mutex};
  use std::thread;
  use std::time::Duration;

  use oriterm_core::event::{Event, EventListener};
  use oriterm_core::{Term, Theme};
  use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};

  /// Event listener that captures `PtyWrite` responses so the test driver
  /// can write them back to the PTY, completing DA/DSR query/response
  /// handshakes inside `vttest`, `tack`, and similar protocol-driven tools.
  pub struct PtyResponder {
      responses: Arc<Mutex<Vec<String>>>,
  }

  impl PtyResponder {
      #[must_use]
      pub fn new() -> Self {
          Self { responses: Arc::new(Mutex::new(Vec::new())) }
      }

      pub fn take_responses(&self) -> Vec<String> {
          std::mem::take(&mut *self.responses.lock().unwrap())
      }
  }

  impl Default for PtyResponder {
      fn default() -> Self { Self::new() }
  }

  impl EventListener for PtyResponder {
      fn send_event(&self, event: Event) {
          if let Event::PtyWrite(data) = event {
              self.responses.lock().unwrap().push(data);
          }
      }
  }
  ```

- [ ] Define `PtySession` struct (verbatim port from `session.rs:39-48`):
  ```rust
  /// PTY-driven test session: child process, byte channel, writer, Term, VTE.
  ///
  /// Owns the PTY pair exclusively. Drop kills and reaps the child
  /// (see `impl Drop` below) and tears the reader thread down.
  pub struct PtySession {
      rx: std::sync::mpsc::Receiver<Vec<u8>>,
      writer: Box<dyn Write + Send>,
      term: Term<PtyResponder>,
      proc: vte::ansi::Processor,
      cols: u16,
      rows: u16,
      child: Box<dyn Child + Send + Sync>,
  }
  ```
  Field privacy is critical: `term`, `writer`, `rx`, `proc` are all private. Adapter crates reach them only through accessor methods. This prevents leaking implementation details into vttest/tack/teseq code.



- [ ] **BLOCKER — explicit `impl Drop` to reap the child.** The `portable_pty::Child` trait is implemented on `std::process::Child` (see `crates/portable-pty/src/lib.rs:271`), and `std::process::Child` **does not kill the child on drop** (documented Rust std lib behavior — the child becomes an orphan). The current `VtTestSession::_child: Box<dyn Child + Send + Sync>` in `oriterm_core/tests/vttest/session.rs:47` has this bug today — every vttest test currently leaks a process. Section 01 MUST NOT replicate this bug in `PtySession`. Add an explicit `impl Drop`:
  ```rust
  impl Drop for PtySession {
      fn drop(&mut self) {
          // Best-effort kill: vttest/tack children may already have
          // exited (clean quit via `q\n`), in which case `kill` is a
          // no-op. If they're still running, `kill` sends SIGHUP on
          // Unix / TerminateProcess on Windows (see
          // crates/portable-pty/src/lib.rs:325-372 for the Child::kill
          // impl on std::process::Child).
          let _ = self.child.kill();
          // Reap the child so the process table entry is cleaned up.
          // Without this, each test run leaves a zombie until the test
          // binary itself exits. wait() consumes the exit status — we
          // discard it (test teardown doesn't inspect it).
          let _ = self.child.wait();
      }
  }
  ```


- [ ] **Pre-existing bug cleanup gate:** the current `oriterm_core/tests/vttest/session.rs` VtTestSession (239 lines) has the same reaping bug, and the plan adapts that file to the new shared type. After Section 01.3's rewrite, the new adapter inherits the `impl Drop` from `PtySession` automatically (because the adapter type-aliases `VtTestSession = PtySession`). Verify after 01.5: no zombie `vttest` or `tack` processes remain after the test suite runs. File via `/add-bug` as `pre-existing reaped here` ONLY if the verification shows any remaining leaks — but the Drop impl should close the issue outright.

- [ ] Implement the general constructor and the vttest helper:
  ```rust
  impl PtySession {
      /// Spawn `cmd` under a PTY of the given size.
      ///
      /// Returns a session ready for `drain()`/`wait()`/`send()`. Panics on
      /// PTY open / spawn / writer-clone failure — these are dev-time
      /// failures, never user-input. The caller is a `#[test]` function.
      #[must_use]
      pub fn spawn(cmd: CommandBuilder, cols: u16, rows: u16) -> Self {
          let pty_system = native_pty_system();
          let pair = pty_system
              .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
              .expect("failed to open PTY");

          let child = pair
              .slave
              .spawn_command(cmd)
              .expect("failed to spawn child under PTY");
          drop(pair.slave);

          let mut pty_reader = pair.master.try_clone_reader().expect("clone reader");
          let writer = pair.master.take_writer().expect("take writer");

          let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
          thread::spawn(move || {
              let mut buf = [0u8; 4096];
              loop {
                  match pty_reader.read(&mut buf) {
                      Ok(0) | Err(_) => break,
                      Ok(n) => {
                          if tx.send(buf[..n].to_vec()).is_err() {
                              break;
                          }
                      }
                  }
              }
          });

          let listener = PtyResponder::new();
          let term = Term::new(rows as usize, cols as usize, 0, Theme::default(), listener);
          let proc = vte::ansi::Processor::new();

          Self { rx, writer, term, proc, cols, rows, child }
      }
      //

      /// Convenience constructor that spawns `vttest` at the given size.
      ///
      /// vttest hardcodes 80x24 internally, so we pass the actual size as
      /// `LINESxMIN_COLS.MAX_COLS`. We set `MAX_COLS=132` so vttest's
      /// pass-1 (DECCOLM set) draws at 132 columns. Mode 40 (`ENABLE_MODE_3`)
      /// is preset so DECCOLM (mode 3) actually resizes the grid.
      ///
      /// Mirrors `oriterm_core/tests/vttest/session.rs::VtTestSession::new`
      /// (which delegates here after this section lands).
      #[must_use]
      pub fn spawn_vttest(cols: u16, rows: u16) -> Self {
          let mut cmd = CommandBuilder::new("vttest");
          cmd.arg(format!("{rows}x{cols}.132"));
          cmd.env("TERM", "xterm-256color");

          let mut session = Self::spawn(cmd, cols, rows);
          // Preset DECCOLM enable mode so vttest's 132-col iteration works.
          session.proc.advance(&mut session.term, b"\x1b[?40h");
          session
      }
  }
  ```

  **Cross-section reuse note:** Section 03 adds `spawn_tack(cols, rows)` as a sibling helper. Section 02's `TerminfoEnv` provides the env var pair (`TERM`, `TERMINFO_DIRS`) that tack-spawning helpers will set. Document this as a `// See: Section 03 spawn_tack helper` comment above `spawn_vttest` so the next implementer knows where the family of constructors lives.

- [ ] Port the four PTY-pump methods verbatim from `session.rs:114-184`:
  ```rust
  impl PtySession {
      /// Drain all currently-buffered PTY output into Term, writing
      /// captured `PtyWrite` responses back to the PTY.
      pub fn drain(&mut self) -> usize {
          let mut total = 0;
          while let Ok(data) = self.rx.try_recv() {
              self.proc.advance(&mut self.term, &data);
              total += data.len();
              for resp in self.term.event_listener().take_responses() {
                  let _ = self.writer.write_all(resp.as_bytes());
              }
              let _ = self.writer.flush();
          }
          total
      }

      /// Block until data arrives or timeout expires, then drain everything.
      pub fn drain_blocking(&mut self, timeout_ms: u64) -> usize {
          let mut total = 0;
          if let Ok(data) = self.rx.recv_timeout(Duration::from_millis(timeout_ms)) {
              self.proc.advance(&mut self.term, &data);
              total += data.len();
              for resp in self.term.event_listener().take_responses() {
                  let _ = self.writer.write_all(resp.as_bytes());
              }
              let _ = self.writer.flush();
          }
          total + self.drain()
      }

      /// Wait until no new PTY output arrives for `quiet_ms`.
      ///
      /// Uses blocking recv to avoid missing data that arrives between
      /// drain and sleep — important for multi-step DA/DSR handshakes
      /// where the queryer sends a follow-up after receiving a response.
      pub fn wait(&mut self, quiet_ms: u64) {
          loop {
              if self.drain_blocking(quiet_ms) == 0 {
                  break;
              }
          }
      }

      /// Wait until `needle` appears anywhere in `grid_text()`, with a
      /// hard timeout. Panics with the current grid on timeout — the
      /// panic message tells the test author exactly what was on screen
      /// when the wait failed.
      pub fn wait_for(&mut self, needle: &str, timeout_ms: u64) {
          let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
          loop {
              self.drain_blocking(100);
              let text = self.grid_text();
              if text.contains(needle) {
                  self.wait(200);
                  return;
              }
              if std::time::Instant::now() >= deadline {
                  panic!(
                      "timed out waiting for {needle:?} after {timeout_ms}ms.\nGrid:\n{}",
                      self.grid_text()
                  );
              }
          }
      }

      /// Send bytes to the child via the PTY writer, then wait for the
      /// screen to settle (300ms quiet period).
      pub fn send(&mut self, key: &[u8]) {
          self.writer.write_all(key).expect("write key");
          self.writer.flush().expect("flush");
          self.wait(300);
      }
  }
  ```

  These bodies are LITERAL copies from `session.rs:114-184`. Do not "improve" them as part of this section — that would defeat the zero-behavioral-change goal. Improvements (e.g., async-aware drain, better cancellation) belong in a follow-up section if needed.

- [ ] Implement grid serialization (`grid_text` and the free-function `grid_chars`), porting from `session.rs:187-229`:
  ```rust
  impl PtySession {
      /// Serialize the visible grid to text, preserving full width.
      ///
      /// Each line is terminated with `\n`. Empty cells are spaces.
      /// `\0` cells are normalized to ` ` (matches the historical
      /// `VtTestSession` behavior expected by the existing 198 insta
      /// snapshots).
      #[must_use]
      pub fn grid_text(&self) -> String {
          let content = self.term.renderable_content();
          let lines = content.lines;
          let cols = content.cols;

          let mut grid = vec![vec![' '; cols]; lines];
          for cell in &content.cells {
              if cell.line < lines && cell.column.0 < cols {
                  let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
                  grid[cell.line][cell.column.0] = ch;
              }
          }

          let mut out = String::new();
          for row in &grid {
              let line: String = row.iter().collect();
              out.push_str(&line);
              out.push('\n');
          }
          out
      }

      /// 2D grid of characters at the current viewport.
      #[must_use]
      pub fn grid_chars(&self) -> Vec<Vec<char>> {
          let content = self.term.renderable_content();
          let lines = content.lines;
          let cols = content.cols;

          let mut grid = vec![vec![' '; cols]; lines];
          for cell in &content.cells {
              if cell.line < lines && cell.column.0 < cols {
                  let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
                  grid[cell.line][cell.column.0] = ch;
              }
          }
          grid
      }

      /// Size label for snapshot naming (e.g., `"80x24"`).
      #[must_use]
      pub fn size_label(&self) -> String {
          format!("{}x{}", self.cols, self.rows)
      }
  }
  ```

  Note: the existing free function `grid_chars(term: &Term<PtyResponder>) -> Vec<Vec<char>>` in `session.rs:216-229` is folded into a method on `PtySession`. Adapter code in 01.3 updates call sites accordingly.

- [ ] Add accessor methods so adapters can reach the inner `Term` without taking ownership:
  ```rust
  impl PtySession {
      #[must_use]
      pub fn term(&self) -> &Term<PtyResponder> { &self.term }

      #[must_use]
      pub fn cols(&self) -> u16 { self.cols }

      #[must_use]
      pub fn rows(&self) -> u16 { self.rows }
  }
  ```
  No `term_mut()` accessor — exposing `&mut Term` would let callers bypass the VTE processor and mutate state behind the protocol parser's back. If a future test legitimately needs to mutate `Term` outside of byte-feeding, add a narrow operation method then.

- [ ] Add the cross-suite tool-availability checker:
  ```rust
  /// Check if `name` is installed and runnable on PATH.
  ///
  /// Used by integration tests to skip cleanly when a required tool
  /// (`vttest`, `tack`, `tic`, `reseq`, ...) is not available. The
  /// `--version` argument is the convention every well-behaved CLI
  /// supports; some (`vttest`) prefer `--help` — pass that explicitly.
  #[must_use]
  pub fn tool_available(name: &str, version_arg: &str) -> bool {
      std::process::Command::new(name)
          .arg(version_arg)
          .stdout(std::process::Stdio::null())
          .stderr(std::process::Stdio::null())
          .status()
          .is_ok()
  }

  /// Convenience: vttest specifically uses `--help` (it has no `--version`).
  #[must_use]
  pub fn vttest_available() -> bool {
      tool_available("vttest", "--help")
  }
  ```
  Both go in `crates/oriterm_test_support/src/session.rs` (or a sibling `availability.rs` submodule if `session.rs` exceeds 400 lines after porting). Per `.claude/rules/code-hygiene.md` 500-line cap: if `session.rs` is approaching 450 lines, split at the natural seam (`session.rs` for `PtySession`/`PtyResponder`, `availability.rs` for `tool_available`/`*_available` helpers).

- [ ] Add unit tests in `crates/oriterm_test_support/src/session/tests.rs` (per `.claude/rules/test-organization.md`: sibling `tests.rs` file, NOT inline `mod tests { ... }`):
  - [ ] `tool_available_returns_false_for_nonexistent_binary` — call `tool_available("definitely_not_a_real_program_xyz", "--version")`, assert `false`.
  - [ ] `vttest_available_matches_tool_available` — both should return the same boolean.
  - [ ] `pty_session_drains_simple_output` — spawn `printf hello` (or `echo hello`) under PTY, drain, assert `grid_text().contains("hello")`. Skip on Windows (no `printf`/`echo` standalone). Matrix: 80x24 only is sufficient — this test exists to prove the spawn/drain pipeline works, not to cover every size.

  Add `#[cfg(test)] mod tests;` (semicolon, no body) at the bottom of `session.rs`.

- [ ] Verify: `cargo test -p oriterm_test_support` passes.
- [ ] Verify: `./clippy-all.sh` green — pedantic + nursery clean for the new crate.
- [ ] Verify: `./build-all.sh` green for `x86_64-pc-windows-gnu` — the Windows-skipped tests should compile via `#[cfg(unix)]` gating.

---

## 01.3 Migrate oriterm_core vttest text tests onto PtySession

**File(s):** `oriterm_core/tests/vttest/session.rs`, `oriterm_core/Cargo.toml`, `oriterm_core/tests/vttest/menu1.rs` through `menu8.rs` (call-site updates only)

This subsection collapses `oriterm_core/tests/vttest/session.rs` from 239 lines of session code down to a thin adapter that re-exports the shared `PtySession`. Existing menu test files (`menu1.rs` … `menu8.rs`) update their `use` lines to pull from the adapter — no logic changes inside the menu tests themselves.

**TDD ordering — write the verification BEFORE deleting code:**

The "test matrix" for this subsection is the 198-snapshot regression check itself. Run it BEFORE migration to capture a known-good baseline (it should pass — establishing the baseline), then run it AFTER migration and assert exact byte equality.

- [ ] **Baseline capture:** `timeout 150 cargo test -p oriterm_core --test vttest 2>&1 | tee /tmp/vttest_pre.log` — record pass/fail of all menu tests.
- [ ] Confirm: zero `.snap.new` files exist under `oriterm_core/tests/vttest/snapshots/` after the baseline run. If any exist, the baseline is dirty and must be resolved first (this is a `/add-bug` candidate per CLAUDE.md, but should not be — the suite is currently passing per the overview's "198 snapshots" claim).

- [ ] Add `oriterm_test_support` to `oriterm_core`'s dev-dependencies in `oriterm_core/Cargo.toml`:
  ```toml
  [dev-dependencies]
  criterion = { version = "0.5", features = ["html_reports"] }
  insta = "1"
  oriterm_test_support = { path = "../crates/oriterm_test_support" }
  portable-pty = "0.9.0"
  serde = { version = "1", features = ["derive"] }
  toml = "0.8"
  ```
  Maintain alphabetical order. Note: `portable-pty` stays here as a direct dev-dep because `pty_size.rs` (see below) uses it directly without going through `PtySession`.

- [ ] **Delete the pre-existing `vttest_available()` in `oriterm_core/tests/vttest/session.rs:232-239`** before rewriting the file. The new adapter re-exports the shared crate's `vttest_available()` instead — two definitions in two crates would shadow each other depending on import order. Mention this explicitly so the implementer doesn't accidentally keep a second copy.

- [ ] Rewrite `oriterm_core/tests/vttest/session.rs` as a thin adapter (target: <60 lines including doc comments):
  ```rust
  //! Adapter that re-exports the shared `PtySession` infrastructure so
  //! existing vttest menu tests don't have to change their import paths
  //! beyond `super::session::*`.
  //!
  //! The session implementation lives in `crates/oriterm_test_support` —
  //! see `plans/tack-conformance/section-01-shared-pty-session.md` for
  //! the deduplication history.

  pub use oriterm_test_support::{PtyResponder, PtySession, vttest_available};

  /// Backwards-compatible alias.
  ///
  /// The pre-deduplication name was `VtTestSession`. Existing menu test
  /// files import it as `super::session::VtTestSession`; we keep the
  /// alias so call sites don't have to be touched in this section's
  /// scope. A follow-up cleanup pass can rename the import sites and
  /// drop the alias.
  pub type VtTestSession = PtySession;

  /// Construct a vttest session at the given size.
  ///
  /// Mirrors the historical `VtTestSession::new(cols, rows)` signature.
  /// Delegates to `PtySession::spawn_vttest`.
  #[must_use]
  pub fn new_vttest(cols: u16, rows: u16) -> PtySession {
      PtySession::spawn_vttest(cols, rows)
  }

  /// Backwards-compatible re-export of `grid_chars(&Term<PtyResponder>)`.
  ///
  /// The pre-deduplication API was a free function. Adapter wraps the
  /// new `PtySession::grid_chars()` method so menu test files keep
  /// calling `grid_chars(&session.term())`. Note: now takes `&PtySession`
  /// instead of `&Term<PtyResponder>` — call sites must update from
  /// `grid_chars(&s.term)` to `s.grid_chars()` (3 menu files). See the
  /// per-file checklist below.
  pub fn grid_chars(session: &PtySession) -> Vec<Vec<char>> {
      session.grid_chars()
  }
  ```

  **Important:** the `VtTestSession::new(cols, rows)` constructor cannot be exposed as a method on the type alias (Rust doesn't let you add inherent methods to a re-exported type from another crate). Replace `VtTestSession::new(cols, rows)` call sites with `new_vttest(cols, rows)` OR with `PtySession::spawn_vttest(cols, rows)` — use the latter (clearer at call site, no extra indirection). The per-menu checklist below enumerates exactly which lines change.

  After this rewrite, `oriterm_core/tests/vttest/session.rs` is ~50 lines (down from 239) and contains zero PTY plumbing.

- [ ] Update `oriterm_core/tests/vttest/menu1.rs` call sites:
  - [ ] Line 4: `use super::session::{VtTestSession, grid_chars, vttest_available};` → `use super::session::{PtySession, vttest_available}; use super::session::grid_chars;`
  - [ ] Every `VtTestSession::new(cols, rows)` → `PtySession::spawn_vttest(cols, rows)` (count: per the research, ~6 sites)
  - [ ] Every `grid_chars(&s.term)` (or similar) → `s.grid_chars()` (count: depends on usage)
  - [ ] Every `s.term.renderable_content()` → `s.term().renderable_content()` (field-to-accessor migration; the new `PtySession` exposes `term` as a method, not a public field).
  - [ ] Every `s.cols`/`s.rows` public field access → `s.cols()`/`s.rows()` accessor call. Run `grep -n '\.cols\|\.rows' oriterm_core/tests/vttest/menu1.rs` to enumerate.
  - [ ] Verify: `cargo test -p oriterm_core --test vttest -- vttest_menu1 vttest_border_fills_80x24` passes locally (one menu at a time keeps the failure surface small).

- [ ] Update `oriterm_core/tests/vttest/menu2.rs` call sites — same pattern. Verify menu2 tests pass.

- [ ] Update `oriterm_core/tests/vttest/menu3.rs` call sites — same pattern. Verify menu3 tests pass. Note: menu3 imports `grid_chars` directly; update to `s.grid_chars()` method calls.

- [ ] Update `oriterm_core/tests/vttest/menu4.rs` through `menu8.rs` call sites — same pattern, one file per checkbox row:
  - [ ] menu4.rs migrated, tests pass
  - [ ] menu5.rs migrated, tests pass
  - [ ] menu6.rs migrated, tests pass
  - [ ] menu7.rs migrated, tests pass
  - [ ] menu8.rs migrated, tests pass

- [ ] `oriterm_core/tests/vttest/pty_size.rs` does NOT use `VtTestSession` — it uses `portable_pty` directly to verify PTY size propagation. Leave it untouched. The dev-dep on `portable-pty` stays in `oriterm_core/Cargo.toml` for this file.

- [ ] **TPR checkpoint** — `/tpr-review` covering 01.1 + 01.2 + 01.3 (the foundation: shared crate creation, PtySession implementation, and the lower-risk text-test migration). Catches any LEAK that survives the extraction, any field/visibility mistakes in `PtySession`, and any subtle behavioral drift in the menu tests before the harder GPU migration in 01.4 lands on top of it.

---

## 01.4 Migrate oriterm vttest GPU golden tests onto PtySession

**File(s):** `oriterm/src/gpu/visual_regression/vttest/mod.rs`, `oriterm/src/gpu/visual_regression/vttest/menus_3_8.rs`, `oriterm/Cargo.toml`

This is the higher-risk half of the migration. The GPU file is 581 lines today and exceeds the 500-line limit (BLOAT). After this subsection, it shrinks to <300 lines and contains only GPU-specific helpers — no PTY/Term/VTE plumbing at all.

- [ ] Add `oriterm_test_support` to `oriterm`'s dev-dependencies in `oriterm/Cargo.toml`:
  ```toml
  [dev-dependencies]
  anyhow = "1"
  criterion = "0.5"
  image = { version = "0.25", default-features = false, features = ["png"] }
  oriterm_test_support = { path = "../crates/oriterm_test_support" }
  oriterm_ui = { path = "../oriterm_ui", features = ["testing"] }
  tempfile = "3"
  ```
  Note: `portable-pty` is currently a NORMAL dependency in `oriterm` (line 19 of `oriterm/Cargo.toml`) because mux integration uses it at runtime. Leave it as-is — the new dev-dep does not change normal dep wiring.

- [ ] **Delete the pre-existing `vttest_available()` in `oriterm/src/gpu/visual_regression/vttest/mod.rs:297`** as part of the cleanup below. The shared crate's re-export replaces it. Two definitions in two crates would shadow each other depending on import order.

- [ ] Delete the duplicate `PtyResponder` from `oriterm/src/gpu/visual_regression/vttest/mod.rs:27-50` (lines 27-50 in the current file).

- [ ] Delete the duplicate `VtTestSession` struct definition from `oriterm/src/gpu/visual_regression/vttest/mod.rs:52-121` (lines 52-121).

- [ ] Delete the duplicate `drain`, `drain_blocking`, `wait`, `wait_for`, `send`, `grid_text` method implementations on `VtTestSession` from `mod.rs:122-284` (the methods that mirror `oriterm_core/tests/vttest/session.rs:114-207`).

- [ ] Replace the deleted block with a thin adapter at the top of the file:
  ```rust
  use oriterm_test_support::{PtyResponder, PtySession, vttest_available};

  // Pre-deduplication, this file defined its own copy of VtTestSession.
  // It now consumes the shared PtySession from crates/oriterm_test_support
  // and adds GPU-specific helpers below.
  ```
  Drop the `mod.rs`-local `use std::io::{Read, Write}`, `use std::sync::{Arc, Mutex}`, `use std::thread`, `use std::time::Duration`, and `use portable_pty::{...}` lines that are no longer needed (those imports lived in the deleted code).

- [ ] Move the GPU-specific helpers (`assert_golden`, `frame_input`, `frame_input_with_blink`, `cell_brightness`) into a free-function or trait-extension form so they take `&PtySession` (or `&mut PtySession`) instead of being methods on the deleted `VtTestSession`:
  ```rust
  /// Render the current `PtySession` grid through the GPU and compare
  /// against a golden reference PNG. Mirrors the pre-deduplication
  /// `VtTestSession::assert_golden` method.
  pub(super) fn assert_golden(
      session: &PtySession,
      name: &str,
      gpu: &GpuState,
      pipelines: &GpuPipelines,
      renderer: &mut WindowRenderer,
  ) {
      let cell = /* ... existing CellMetrics derivation ... */;
      let input = frame_input(session, cell);
      // ... existing render_to_pixels + compare_with_reference body ...
  }

  /// Build a `FrameInput` for the current grid state.
  fn frame_input(session: &PtySession, cell: CellMetrics) -> FrameInput { /* ... */ }

  /// Build a `FrameInput` with explicit text-blink opacity.
  fn frame_input_with_blink(
      session: &PtySession,
      cell: CellMetrics,
      text_blink_opacity: f32,
  ) -> FrameInput { /* ... */ }

  /// Compute average pixel brightness in a cell-shaped region.
  fn cell_brightness(pixels: &[u8], width: u32, col: usize, row: usize, cw: f32, ch: f32) -> u32 {
      // ... unchanged ...
  }
  ```
  All four helpers are GPU-specific and stay in `oriterm/` (per `.claude/rules/crate-boundaries.md`: GPU types must not leak into `oriterm_core`, `oriterm_ui`, or `oriterm_test_support`).

- [ ] Update the call sites inside `mod.rs` (the `run_menu1_golden`, `run_menu2_golden`, `vttest_blink_multi_frame` test functions) to construct `PtySession::spawn_vttest(cols, rows)` and pass the session to `assert_golden(&session, ...)` instead of calling `session.assert_golden(...)`.

- [ ] Update any field access sites for `pub cols: u16` and `pub rows: u16` — the current `VtTestSession` exposes these as public fields (see `oriterm/src/gpu/visual_regression/vttest/mod.rs:207-208`: `self.cols as usize`, `self.rows as usize`). The new `PtySession` exposes them as accessor methods (`cols()`, `rows()`). Change `self.cols`/`self.rows` to `session.cols()`/`session.rows()` inside the free-function `frame_input(session, cell)` body.

- [ ] Update `oriterm/src/gpu/visual_regression/vttest/menus_3_8.rs`:
  - [ ] Line 3: `use super::{VtTestSession, vttest_available};` → `use super::{assert_golden, PtySession, vttest_available};`
  - [ ] Every `VtTestSession::new(cols, rows)` (5 sites per the research) → `PtySession::spawn_vttest(cols, rows)`
  - [ ] Every `s.assert_golden(name, ...)` → `assert_golden(&s, name, ...)`
  - [ ] Verify: `cargo test -p oriterm --features gpu-tests -- vttest_golden_menu3 vttest_golden_menu4` passes locally (one menu at a time).

- [ ] Verify line count of the rewritten `oriterm/src/gpu/visual_regression/vttest/mod.rs`:
  - [ ] Target: <300 lines (was 581)
  - [ ] Hard gate: <500 lines (the file MUST be under the BLOAT threshold from `.claude/rules/code-hygiene.md` after this subsection, no exceptions)
  - [ ] If still over 500, split: extract `frame_input` / `frame_input_with_blink` / `cell_brightness` into a sibling `frame_input.rs` submodule.

- [ ] Verify: `cargo build -p oriterm --features gpu-tests` succeeds with no warnings.
- [ ] Verify: `cargo test -p oriterm --features gpu-tests -- vttest_golden` passes — all 98 PNG goldens still match.
- [ ] **TPR checkpoint** — `/tpr-review` covering 01.4 specifically (the GPU migration is the higher-risk half — separate checkpoint from 01.1-01.3 to keep findings narrowly scoped).

---

## 01.5 Verify zero behavioral change (snapshot + PNG byte-equality)

**File(s):** None (verification only — no code changes)

The whole point of Section 01 is **zero behavioral change**. This subsection is the proof.

- [ ] Run `timeout 150 cargo test -p oriterm_core --test vttest` to completion. Expect: all menu tests pass.
- [ ] Confirm: zero `.snap.new` files generated under `oriterm_core/tests/vttest/snapshots/`. Use `find oriterm_core/tests/vttest/snapshots -name '*.snap.new'` — empty output is the gate.
- [ ] Snapshot count check: the count of `.snap` files under `oriterm_core/tests/vttest/snapshots/` is unchanged from the baseline. Compare against the overview's claimed 198. (Document the actual number found if it differs from 198 — the overview will be corrected during section 09.)

- [ ] Run `timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden` to completion. Expect: all golden tests pass.
- [ ] Golden PNG count check: the count of `vttest_*.png` files under `oriterm/tests/references/` is unchanged from the baseline (research found 98). Use `find oriterm/tests/references -name 'vttest_*.png' | wc -l` — must equal pre-migration count.
- [ ] Pixel diff check: the GPU test framework already enforces zero pixel diffs against golden references (any test that fails would surface it). Re-run twice to make sure the migrated code is deterministic — flaky golden image tests are bugs (per CLAUDE.md "Flaky tests ARE bugs"); if any flakes, file via `/add-bug` immediately and treat as a blocker for closing this section.

- [ ] Architecture / hygiene gate:
  - [ ] `find oriterm/src/gpu/visual_regression/vttest -name '*.rs' -exec wc -l {} \;` — every file under 500 lines. The new `mod.rs` MUST be under 500.
  - [ ] No file in `crates/oriterm_test_support/src/` exceeds 500 lines.
  - [ ] No new `#[allow(clippy::...)]` introduced.
  - [ ] No new `unsafe` blocks introduced.

- [ ] Cross-platform build gate:
  - [ ] `./build-all.sh` (cross-compiles to `x86_64-pc-windows-gnu`) — green
  - [ ] `./clippy-all.sh` — green, no new warnings
  - [ ] `timeout 150 ./test-all.sh` — green, no regressions in non-vttest tests either

---

## 01.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers.
If unresolved findings exist here:
- section frontmatter `status` must be `in-progress`
- `third_party_review.status` must be `findings`

When all findings are triaged:
- accepted findings are integrated into the relevant implementation subsection(s)
- rejected findings are closed with rationale
- all items in this block are marked resolved
- `third_party_review.status` becomes `resolved` or `none`
-->

- None.

---

## 01.N Completion Checklist

- [ ] `crates/oriterm_test_support/Cargo.toml` exists and the crate is in `workspace.members`
- [ ] `crates/oriterm_test_support/src/session.rs` defines `PtySession`, `PtyResponder`, `tool_available`, `vttest_available`
- [ ] `crates/oriterm_test_support/src/session/tests.rs` (sibling tests file) covers `tool_available`, `vttest_available`, and a basic `pty_session_drains_simple_output` smoke test
- [ ] `oriterm_core/tests/vttest/session.rs` is reduced to a thin adapter (<60 lines), exports `VtTestSession` as a type alias for `PtySession`
- [ ] All 8 menu test files in `oriterm_core/tests/vttest/` import `PtySession` from the adapter — no direct PTY plumbing remains in this directory
- [ ] `oriterm/src/gpu/visual_regression/vttest/mod.rs` is below 500 lines (target: <300) — no `PtyResponder` or `VtTestSession` definition remains here
- [ ] `oriterm/src/gpu/visual_regression/vttest/menus_3_8.rs` imports `PtySession` and `assert_golden` from the new layout
- [ ] `oriterm_test_support` is in `oriterm_core/Cargo.toml` `[dev-dependencies]` and `oriterm/Cargo.toml` `[dev-dependencies]`
- [ ] `timeout 150 cargo test -p oriterm_core --test vttest` — all menu tests pass; zero `.snap.new` files
- [ ] `timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden` — all golden tests pass; zero pixel diffs
- [ ] `cargo test -p oriterm_test_support` — internal unit tests pass
- [ ] `./build-all.sh` green (host + `x86_64-pc-windows-gnu`)
- [ ] `./clippy-all.sh` green — no new warnings, no `#[allow(clippy::...)]` added
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Plan annotation cleanup: no temporary scaffolding (`§`, `Phase`, `section-`, `BUG`, `TPR` markers) left in any `.rs` file
- [ ] All intermediate TPR checkpoint findings (from 01.3 and 01.4 checkpoints) resolved — see `01.R`
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`, all subsection statuses → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 01 marked Complete
  - [ ] `00-overview.md` Mission Success Criteria: criteria #1, #2, #3, #4 checkboxes ticked
  - [ ] `index.md` Section 01 status updated
  - [ ] Section 02's `depends_on: ["01"]` confirmed accurate; no stale assumptions
- [ ] `/tpr-review` final pass — Codex review finds zero critical or major findings (or all triaged through `01.R`)
- [ ] `/impl-hygiene-review last commit` final pass — hygiene review clean. MUST run AFTER `/tpr-review` is clean.

**Exit Criteria:** `crates/oriterm_test_support` exists with `PtySession` providing the canonical PTY+Term+VTE driver. `oriterm_core/tests/vttest/session.rs` is a <60-line adapter. `oriterm/src/gpu/visual_regression/vttest/mod.rs` is <300 lines and contains no PTY plumbing. `timeout 150 cargo test -p oriterm_core --test vttest` runs to completion with all 198 insta snapshots matching byte-for-byte. `timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden` runs to completion with all 98 PNG goldens matching pixel-for-pixel. Zero new clippy warnings, zero new `#[allow]` annotations, zero new `unsafe` blocks. The VtTestSession LEAK is closed; the BLOAT in `vttest/mod.rs` is closed.
