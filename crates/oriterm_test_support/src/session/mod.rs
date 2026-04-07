//! Cross-suite PTY+Term+VTE driver. See [`PtySession`].
//!
//! This module is the canonical home for the PTY/Term/VTE plumbing that
//! used to be duplicated between `oriterm_core/tests/vttest/session.rs`
//! and `oriterm/src/gpu/visual_regression/vttest/mod.rs`. Both consumers
//! adapt this same type. The vttest constructor lives below; the tack
//! constructor lands in Section 03 of the tack-conformance plan.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use oriterm_core::event::{Event, EventListener};
use oriterm_core::{Term, Theme};
use portable_pty::{Child, CommandBuilder, ExitStatus, PtySize, native_pty_system};

use crate::terminfo::TerminfoEnv;

/// Captures `PtyWrite` responses so the test driver can write them back.
///
/// Used to complete DA/DSR query/response handshakes inside `vttest`,
/// `tack`, and similar protocol-driven tools. The struct is `pub` only
/// because it appears in `Term<PtyResponder>` — the type parameter that
/// `PtySession::term()` exposes through its return type. Both the
/// constructor and the response-buffer drain are `pub(crate)` so that
/// external callers cannot reach `session.term().event_listener()
/// .take_responses()` and steal the DA/DSR reply queue that
/// `PtySession::drain()` / `drain_blocking()` exclusively own.
pub struct PtyResponder {
    responses: Arc<Mutex<Vec<String>>>,
}

impl PtyResponder {
    /// Construct an empty responder with no buffered responses.
    pub(crate) fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Drain all buffered `PtyWrite` payloads, returning them in arrival
    /// order. The internal buffer is reset to empty.
    pub(crate) fn take_responses(&self) -> Vec<String> {
        std::mem::take(&mut *self.responses.lock().expect("PtyResponder mutex poisoned"))
    }
}

impl EventListener for PtyResponder {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(data) = event {
            self.responses
                .lock()
                .expect("PtyResponder mutex poisoned")
                .push(data);
        }
    }
}

/// PTY-driven test session: child process, byte channel, writer, Term, VTE.
///
/// Owns the PTY pair exclusively. `Drop` kills and reaps the child (see
/// [`PtySession::drop`]) and tears the reader thread down by closing the
/// channel. Adapter code reaches the inner [`Term`] via [`PtySession::term`]
/// without taking ownership.
pub struct PtySession {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
    term: Term<PtyResponder>,
    proc: vte::ansi::Processor,
    cols: u16,
    rows: u16,
    child: Box<dyn Child + Send + Sync>,
}

impl PtySession {
    /// Spawn `cmd` under a PTY of the given size.
    ///
    /// Returns a session ready for `drain()` / `wait()` / `send()`.
    /// Panics on PTY open / spawn / writer-clone failure — these are
    /// dev-time failures, never user input. The caller is a `#[test]`
    /// function.
    #[must_use]
    pub fn spawn(cmd: CommandBuilder, cols: u16, rows: u16) -> Self {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
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

        Self {
            rx,
            writer,
            term,
            proc,
            cols,
            rows,
            child,
        }
    }

    /// Convenience constructor that spawns `vttest` at the given size.
    ///
    /// vttest hardcodes 80x24 internally, so we pass the actual size as
    /// `LINESxMIN_COLS.MAX_COLS`. We set `MAX_COLS=132` so vttest's
    /// pass-1 (DECCOLM set) draws at 132 columns. Mode 40 (`ENABLE_MODE_3`)
    /// is preset so DECCOLM (mode 3) actually resizes the grid to
    /// 80/132 columns — vttest's 132-column iteration relies on this.
    ///
    /// See: [`Self::spawn_tack`] for the analogous tack constructor.
    #[must_use]
    pub fn spawn_vttest(cols: u16, rows: u16) -> Self {
        let mut cmd = CommandBuilder::new("vttest");
        cmd.arg(format!("{rows}x{cols}.132"));
        cmd.env("TERM", "xterm-256color");

        let mut session = Self::spawn(cmd, cols, rows);
        session.proc.advance(&mut session.term, b"\x1b[?40h");
        session
    }

    /// Spawn `tack` at the given grid size, using the supplied
    /// [`TerminfoEnv`] to pin `TERM`, `TERMINFO`, and `TERMINFO_DIRS`.
    ///
    /// `tack` reads the terminfo entry named by `$TERM` from the
    /// directories listed in `$TERMINFO_DIRS` (or `$TERMINFO` — some
    /// ncurses consumers honor only one of the two).
    /// [`TerminfoEnv::apply_env`] sets all three at once, hiding the
    /// env-var details from this call site. Tests in Sections 03-06 of
    /// the tack-conformance plan share this single canonical tack
    /// invocation site so any future change to terminfo plumbing happens
    /// in exactly one place.
    ///
    /// We do NOT pass `-i` — tack's init sequences are part of what we
    /// want to test.
    #[must_use]
    pub fn spawn_tack(env: &TerminfoEnv, cols: u16, rows: u16) -> Self {
        let mut cmd = CommandBuilder::new("tack");
        // Pass the term name as a positional arg so tack picks it up
        // under both ncurses and BSD curses, regardless of which env var
        // the implementation consults first.
        cmd.arg(env.term());
        env.apply_env(&mut cmd);
        Self::spawn(cmd, cols, rows)
    }

    /// Borrow the inner [`Term`] for grid inspection.
    ///
    /// No `term_mut()` accessor exists by design: exposing `&mut Term`
    /// would let callers bypass the VTE processor and mutate state
    /// behind the protocol parser's back. If a future test legitimately
    /// needs to mutate `Term` outside of byte-feeding, add a narrow
    /// operation method on `PtySession` instead.
    #[must_use]
    pub fn term(&self) -> &Term<PtyResponder> {
        &self.term
    }

    /// Number of columns the PTY was opened with.
    #[must_use]
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Number of rows the PTY was opened with.
    #[must_use]
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Drain all currently-buffered PTY output into Term, writing
    /// captured `PtyWrite` responses back to the PTY.
    pub fn drain(&mut self) -> usize {
        let mut total = 0;
        while let Ok(data) = self.rx.try_recv() {
            total += self.feed_and_flush(&data);
        }
        total
    }

    /// Block until data arrives or `timeout_ms` expires, then drain
    /// everything else still in the channel.
    pub fn drain_blocking(&mut self, timeout_ms: u64) -> usize {
        let mut total = 0;
        if let Ok(data) = self.rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            total += self.feed_and_flush(&data);
        }
        total + self.drain()
    }

    /// Feed `data` through the VTE processor, then write any captured
    /// `PtyWrite` responses back to the PTY. Shared core of `drain()`
    /// and `drain_blocking()`.
    fn feed_and_flush(&mut self, data: &[u8]) -> usize {
        self.proc.advance(&mut self.term, data);
        for resp in self.term.event_listener().take_responses() {
            // Best-effort: writer errors close the test session naturally
            // via Drop, so swallowing here is correct for test setup.
            let _ = self.writer.write_all(resp.as_bytes());
        }
        let _ = self.writer.flush();
        data.len()
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
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {needle:?} after {timeout_ms}ms.\nGrid:\n{}",
                self.grid_text()
            );
        }
    }

    /// Wait until the child process exits, with a hard timeout.
    ///
    /// Returns the [`ExitStatus`] on clean exit. Panics with the
    /// current grid contents on timeout — the panic message tells the
    /// test author exactly what was on screen when the child failed
    /// to exit.
    ///
    /// Used by tack/vttest tests after sending `q\n` (or whatever the
    /// tool's quit key is) to assert the child actually terminated,
    /// not just that the parent stopped reading bytes.
    ///
    /// Implementation note: this polls
    /// [`portable_pty::Child::try_wait`] (Unix: `waitpid(WNOHANG)`;
    /// Windows: `GetExitCodeProcess` — NOT `WaitForSingleObject`,
    /// see `crates/portable-pty/src/win/mod.rs` `WinChild::is_complete`).
    /// On the `Ok(None)` branch the method calls
    /// [`Self::drain_blocking`] to forward any late output, then
    /// sleeps 10 ms if nothing was drained — this bounds the poll
    /// rate to ~100 Hz so the loop never hot-spins between reader EOF
    /// and `try_wait` observing termination.
    ///
    /// Delegates to [`Self::wait_for_child_exit_inner`] so the test
    /// suite can pin the bounded-poll invariant via a deterministic
    /// iteration counter (see
    /// `pty_session_wait_for_child_exit_bounded_poll_invariant`).
    pub fn wait_for_child_exit(&mut self, timeout_ms: u64) -> ExitStatus {
        self.wait_for_child_exit_inner(timeout_ms, || {})
    }

    /// Inner body of [`Self::wait_for_child_exit`] — see that method
    /// for the bounded-poll contract (this helper implements it
    /// verbatim). The only reason this exists as a separate function
    /// is the `on_iter` callback: the public wrapper passes a no-op
    /// (zero cost), and the test suite passes an iteration counter to
    /// pin the invariant via
    /// `pty_session_wait_for_child_exit_bounded_poll_invariant` in
    /// [`tests`].
    fn wait_for_child_exit_inner<F: FnMut()>(
        &mut self,
        timeout_ms: u64,
        mut on_iter: F,
    ) -> ExitStatus {
        const POLL_SLEEP: Duration = Duration::from_millis(10);
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            on_iter();
            match self.child.try_wait() {
                Ok(Some(status)) => return status,
                Ok(None) => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "child did not exit within {timeout_ms}ms.\nGrid:\n{}",
                        self.grid_text()
                    );
                    // Drain any final output while we wait. If the
                    // reader thread has already closed its channel
                    // (EOF after child exit), `drain_blocking` returns
                    // 0 immediately — bound the poll rate so we don't
                    // burn CPU between reader EOF and try_wait()
                    // observing exit.
                    if self.drain_blocking(50) == 0 {
                        thread::sleep(POLL_SLEEP);
                    }
                }
                Err(e) => panic!("PtySession::wait_for_child_exit: try_wait error: {e}"),
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

    /// Serialize the visible grid to text, preserving full width.
    ///
    /// Each line is terminated with `\n`. Empty cells are spaces. `\0`
    /// cells are normalized to ` ` (matches the historical
    /// `VtTestSession` behavior expected by the existing 198 insta
    /// snapshots).
    #[must_use]
    pub fn grid_text(&self) -> String {
        let grid = self.grid_chars();
        let mut out = String::new();
        for row in &grid {
            let line: String = row.iter().collect();
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// 2D grid of characters at the current viewport.
    ///
    /// Empty cells are spaces; `\0` cells are normalized to ` ` (matches
    /// the historical behavior expected by the existing 198 insta
    /// snapshots). This is the canonical character extraction; both
    /// `grid_text` and consumers that need a 2D Vec call into here.
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

impl Drop for PtySession {
    fn drop(&mut self) {
        // Best-effort kill: vttest/tack children may already have exited
        // (clean quit via `q\n`), in which case `kill` is a no-op. If
        // they're still running, `kill` sends SIGHUP on Unix /
        // TerminateProcess on Windows (see crates/portable-pty/src/lib.rs
        // Child::kill impl on std::process::Child).
        let _ = self.child.kill();
        // Reap the child so the process table entry is cleaned up.
        // Without this, each test run leaves a zombie until the test
        // binary itself exits. wait() consumes the exit status — we
        // discard it (test teardown doesn't inspect it).
        let _ = self.child.wait();
    }
}

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

/// Check if `tic` (terminfo compiler) is installed.
///
/// Probe is `tic -V`, which is the version flag every ncurses build
/// (BSD and GNU) supports. A too-old `tic` (ncurses < 6.0) may exist
/// and still fail to compile modern extension caps; in that case
/// `TerminfoEnv::compile()` panics with the tic stderr output, which
/// IS the failure contract — the user sees the message and upgrades
/// their ncurses package.
#[must_use]
pub fn tic_available() -> bool {
    tool_available("tic", "-V")
}

/// Check if `tack` (terminfo action checker, ncurses) is installed.
///
/// Tack ships with ncurses on Linux/macOS, not on native Windows.
/// Use this gate at the top of every test that spawns tack so the
/// suite skips cleanly on platforms missing the tool.
#[must_use]
pub fn tack_available() -> bool {
    tool_available("tack", "-V")
}

/// Check if `infocmp` (terminfo decompiler / inspector) is installed.
///
/// Probe is `infocmp -V`. Used by `terminfo` round-trip tests to
/// gate cleanly when `infocmp` is missing. `TerminfoEnv::compile()`
/// itself never depends on `infocmp` — that gate is enforced by
/// keeping the `compile()` constructor pure-`tic`.
#[must_use]
pub fn infocmp_available() -> bool {
    tool_available("infocmp", "-V")
}

#[cfg(test)]
mod tests;
