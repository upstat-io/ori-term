//! Cross-suite PTY+Term+VTE driver. See [`PtySession`].
//!
//! This module is the canonical home for the PTY/Term/VTE plumbing that
//! used to be duplicated between `oriterm_core/tests/vttest/session.rs`
//! and `oriterm/src/gpu/visual_regression/vttest/mod.rs`. Both consumers
//! adapt this same type. The vttest constructor lives below; the tack
//! constructor lands alongside it.
//!
//! `mod.rs` is the dispatch hub holding type definitions, constructors,
//! accessors, the [`PtySession::send`] write primitive, the [`Drop`]
//! impl, and the `tool_available` family of free functions. The polling
//! helpers (`drain`, `drain_blocking`, `wait`, `wait_for`) live in the
//! [`sync`] submodule together with the private `poll_until` SSOT
//! helper. The child-exit and quit helpers (`wait_for_child_exit`) live
//! in the [`teardown`] submodule. Each leaf module owns its own sibling
//! `tests.rs` per `.claude/rules/test-organization.md`.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use oriterm_core::event::{Event, EventListener};
use oriterm_core::{Term, Theme};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::terminfo::TerminfoEnv;

mod sync;
mod teardown;

/// Process-wide mutex used to serialize Windows `ConPTY` sessions
/// for the entire `PtySession` lifetime.
///
/// **Windows-only contention point.** Empirical testing on
/// Windows 11 shows that running more than ~4 simultaneous
/// active `PtySession`s causes per-test wall-clock to balloon
/// by an order of magnitude (a <1 s test takes 50+ s when run
/// alongside 7 other `ConPTY` tests). The contention surfaces
/// across the entire `ConPTY` lifetime — pseudoconsole
/// allocation, child-process console attachment, PTY I/O, and
/// teardown — not just at the spawn step. Serializing only
/// `openpty + spawn_command` does not eliminate the slowdown.
///
/// Holding this mutex from `PtySession::spawn` until
/// `PtySession::drop` serializes the entire `ConPTY` lifetime on
/// Windows. Non-PTY tests (parser, terminfo, parallel-safe
/// helpers) still run in parallel — only `PtySession`-using
/// tests are forced into serial execution. Total Windows test
/// runtime stays bounded by the sum of individual test costs,
/// rather than collapsing into the contention quagmire.
///
/// Linux and macOS PTYs do not exhibit this contention —
/// `openpty` is a thin libc call that does not contend across
/// threads — so the mutex is `cfg(windows)`-only.
///
/// **Poison recovery.** Tests that intentionally `catch_unwind`
/// inside the session body would poison this mutex if the
/// guard's drop ran during unwind. We recover from poisoning
/// via `PoisonError::into_inner` so a panicked test does not
/// permanently break the next test's spawn.
#[cfg(windows)]
static CONPTY_LIFETIME_LOCK: Mutex<()> = Mutex::new(());

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
///
/// **Field declaration order is load-bearing.** Rust drops struct fields
/// in declaration order after [`Drop::drop`] returns. `child` MUST be
/// declared before `_master` so the child handle drops first and the
/// `PTY` master drops last. On Windows ``ConPTY`` this enforces Microsoft's
/// `ClosePseudoConsole` contract: the call must follow child exit, not
/// precede it. See `Self::_master` for the full rationale.
pub struct PtySession {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
    term: Term<PtyResponder>,
    proc: vte::ansi::Processor,
    cols: u16,
    rows: u16,
    child: Box<dyn Child + Send + Sync>,
    /// Held to keep the underlying `PTY` master (Windows ``ConPTY`` `HPCON`
    /// or Unix master fd) alive for the entire child process lifetime.
    ///
    /// **Why this exists (Windows ``ConPTY`` contract).** Microsoft's
    /// [`ClosePseudoConsole` documentation][docs] states *"you should
    /// never call `ClosePseudoConsole` until after the client has exited
    /// or the call may hang."* Dropping `Box<dyn MasterPty>` triggers
    /// `ConPtyMasterPty::drop` → `Inner::drop` → `PsuedoCon::drop` →
    /// `ClosePseudoConsole(self.con)`. If the master is NOT held here,
    /// `pair.master` falls out of scope at the end of [`Self::spawn`]
    /// and `ClosePseudoConsole` runs while the child is still alive.
    /// This leaks console-subsystem DLL state and eventually causes
    /// new `cmd.exe` spawns to fail with `STATUS_DLL_INIT_FAILED`
    /// (`0xC0000142`) or hang inside `WaitForSingleObject`.
    ///
    /// **Field order matters (see struct doc above).** This field is
    /// declared AFTER `child` so Rust's declaration-order field-drop
    /// sequence runs `child` first (the [`Child`] handle drops, the
    /// process slot is reaped) and THEN drops `_master` (which calls
    /// `ClosePseudoConsole` on a child that has already exited — the
    /// Microsoft-sanctioned ordering). The synchronous
    /// `kill()` + `wait()` inside [`Drop::drop`] ensures the child has
    /// already terminated before any field drops run; the field-drop
    /// order then ensures `ClosePseudoConsole` runs after the handle
    /// to the dead process is released.
    ///
    /// **Production parallel.** `oriterm_mux::pty::spawn::spawn_pty`
    /// (production PTY path) holds `pair.master` inside
    /// `PtyControl(pair.master)` for the same reason. This field is
    /// the test-path equivalent.
    ///
    /// **Reference implementations.** wezterm's `mux/src/domain.rs`
    /// stores `Box<dyn MasterPty + Send>` inside `Mutex<...>` for the
    /// pane lifetime; wezterm's `wezterm/src/asciicast.rs` keeps
    /// `pair.master` alive at function scope through the entire
    /// child-output loop; wezterm's `pty/examples/whoami.rs:81`
    /// explicitly drops `pair.master` AFTER `child.wait()`. The
    /// "master must outlive child" pattern is the documented contract,
    /// not coincidence.
    ///
    /// [docs]: https://learn.microsoft.com/en-us/windows/console/closepseudoconsole
    _master: Box<dyn MasterPty + Send>,
    /// Process-wide `ConPTY` serialization guard. See
    /// [`CONPTY_LIFETIME_LOCK`] for the rationale. Held for the
    /// entire `PtySession` lifetime so concurrent
    /// `PtySession`-using tests are forced into serial
    /// execution on Windows.
    ///
    /// Field declared LAST so its drop runs after `_master`'s
    /// drop — the lock is released only after
    /// `ClosePseudoConsole` has fully completed, so the next
    /// test's `spawn` (which acquires the lock) sees a clean
    /// console subsystem state.
    #[cfg(windows)]
    _conpty_guard: std::sync::MutexGuard<'static, ()>,
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
        // Serialize `ConPTY` sessions on Windows for the entire
        // `PtySession` lifetime — see [`CONPTY_LIFETIME_LOCK`] for
        // the rationale. The guard is held in the `_conpty_guard`
        // field below and dropped only when `PtySession` drops, so
        // concurrent test threads block here until the previous
        // session has fully torn down (including `ClosePseudoConsole`
        // running through `_master`'s drop).
        //
        // Poison recovery: a panicked test inside the session body
        // would poison the mutex on guard drop. We recover via
        // `PoisonError::into_inner` so the next test's spawn proceeds
        // normally instead of panicking with `PoisonError`.
        #[cfg(windows)]
        let conpty_guard = CONPTY_LIFETIME_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
            // Hold the master alive for the child's full lifetime.
            // Dropping `pair.master` at function exit would call
            // `ClosePseudoConsole` on a still-running child, violating
            // Microsoft's contract and corrupting the console subsystem
            // state on Windows. See the `_master` field doc for the
            // full rationale.
            _master: pair.master,
            #[cfg(windows)]
            _conpty_guard: conpty_guard,
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

    /// Send bytes to the child via the PTY writer, then wait for the
    /// screen to settle (300ms quiet period).
    ///
    /// This is the default send primitive for interactive navigation
    /// tests where the caller expects the terminal to repaint before
    /// the next assertion. For teardown loops or rapid-fire sends
    /// where `try_wait()` polling replaces the settle check, use
    /// [`Self::send_raw`] instead.
    pub fn send(&mut self, key: &[u8]) {
        self.writer.write_all(key).expect("write key");
        self.writer.flush().expect("flush");
        self.wait(300);
    }

    /// Write bytes to the child's PTY and flush, WITHOUT the 300ms
    /// quiesce that [`Self::send`] does internally.
    ///
    /// Use this when the caller has its own synchronization strategy
    /// that makes the quiesce unnecessary or actively harmful — e.g.
    /// `quit_tack` polls `try_wait()` between sends and wants to
    /// observe child exit as soon as possible, not 300ms later.
    ///
    /// Error swallow policy: writer errors are silently dropped (same
    /// as `quit_tack`'s teardown context, where a `q\n` after tack
    /// has already exited produces EPIPE/`ERROR_BROKEN_PIPE` that we
    /// do NOT want to crash on). Callers that need error propagation
    /// should use the canonical [`Self::send`] and tolerate the
    /// quiesce.
    pub fn send_raw(&mut self, key: &[u8]) {
        let _ = self.writer.write_all(key);
        let _ = self.writer.flush();
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
        //
        // **Ordering contract.** This synchronous reap MUST happen
        // before `_master` drops below (declaration-order field drop
        // runs `child` before `_master`). On Windows `ConPTY` this
        // ordering is load-bearing: `ClosePseudoConsole` is called
        // inside `_master`'s drop chain, and Microsoft's documented
        // contract is that the call must follow child exit, not
        // precede it. Dropping `_master` while a child is still alive
        // leaks console-subsystem DLL state and eventually causes new
        // `cmd.exe` spawns to fail with `STATUS_DLL_INIT_FAILED` or
        // hang inside `WaitForSingleObject`.
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

// ============================================================
// Tack version gate (added in 05.0.b's 05.0.c subsection).
//
// Section 05's tack catalog is pinned against tack v1.08 (the
// version on the dev host). Every menu key, prompt string, and
// screen layout in the catalog is verified against that exact
// build. A future system upgrade to tack v6.x or v2.0 could
// change the menu structure entirely. Without a version gate,
// the discovery test would fail loudly — but the dozens of
// downstream scenarios would also fail in ways that pollute CI
// noise. The gate skips them cleanly with a concrete
// "tack version not supported, skipping" message and a single
// fix path: pin the new version, re-run discovery, update
// inventory.
// ============================================================

/// Lowest tack major version Section 05's catalog has been pinned
/// against. Bump in lockstep with [`TACK_PINNED_MINOR`] when the
/// catalog is re-verified against a newer tack release.
pub const TACK_PINNED_MAJOR: u32 = 1;

/// Lowest tack minor version Section 05's catalog has been pinned
/// against. The minor version is checked EXACTLY — every minor
/// bump requires a re-discovery pass via 05.0.
pub const TACK_PINNED_MINOR: u32 = 8;

/// Pure parser for `tack -V` output. Returns `Some((major, minor))`
/// when the version banner can be located and parsed, `None`
/// otherwise.
///
/// Tack 1.08 prints to stdout. Future versions might split streams
/// (stderr-only), so the parser scans the concatenation. The parse
/// shape is `<...>version <maj>.<min><...>` where `<maj>` and
/// `<min>` are decimal integers (zero-padded `min` is fine —
/// `parse::<u32>` handles `"08"`).
///
/// Pure (no I/O) so unit tests can exercise the version-string
/// matrix without depending on a host-installed tack.
#[must_use]
pub fn parse_tack_version(stdout: &str, stderr: &str) -> Option<(u32, u32)> {
    let combined = format!("{stdout}{stderr}");
    let pos = combined.find("version ")?;
    // `.get(..)` rather than `&combined[..]` so clippy's
    // `string_slice` lint stays clean: `find` always returns a UTF-8
    // boundary so the slice is provably safe, but the lint can't see
    // that across functions, and a `.get()` + `?` is just as concise.
    let after = combined.get(pos + "version ".len()..)?;
    // Split on either `.` (major/minor separator) or whitespace
    // (terminates the minor number before any trailing build date).
    // The first two non-empty fragments are the major and minor.
    let mut parts = after.split(|c: char| c == '.' || c.is_ascii_whitespace());
    let maj_str = parts.next()?;
    let min_str = parts.next()?;
    let maj = maj_str.parse::<u32>().ok()?;
    let min = min_str.parse::<u32>().ok()?;
    Some((maj, min))
}

/// Build the loud-skip diagnostic text for an unsupported tack
/// version. Pure helper so tests can pin the keyword set without
/// observing the side-effecting `eprintln!`.
///
/// Operators see this string when CI hosts upgrade tack out from
/// under the catalog. The text names the observed version, the
/// pinned version, and the four-step upgrade path so the operator
/// can fix the gate without grepping the source.
#[must_use]
pub fn unsupported_tack_diagnostic(observed_maj: u32, observed_min: u32) -> String {
    format!(
        "tack {observed_maj}.{observed_min:02} installed but Section 05's catalog is pinned to \
         tack {pmaj}.{pmin:02}. Tack scenarios will SKIP. To re-pin: \
         (1) update TACK_PINNED_MAJOR/MINOR in session/mod.rs, \
         (2) run `INSTA_UPDATE=1 cargo test -p oriterm_core --test tack -- \
         test_menu::begin_testing_inventory` to capture the new menu, \
         (3) update BEGIN_TESTING_INVENTORY in scenarios/begin_testing_inventory.rs, \
         (4) re-run the full test_menu suite to update affected snapshots.",
        pmaj = TACK_PINNED_MAJOR,
        pmin = TACK_PINNED_MINOR,
    )
}

/// Pure version check with an injected diagnostic emitter.
///
/// Takes pre-captured `tack -V` stdout/stderr and a closure used to
/// emit the loud-skip diagnostic on mismatch. Used by both
/// [`tack_version_supported`] (with `eprintln!` as the emitter) and
/// the unit tests (with a `String` accumulator). Splitting the I/O
/// from the policy means the loud-skip emit + silent-on-match pins
/// can run without spawning a subprocess and without depending on
/// the host-installed tack version.
///
/// Returns `true` iff the parsed version exactly matches
/// ([`TACK_PINNED_MAJOR`], [`TACK_PINNED_MINOR`]). Calls `emit` with
/// the [`unsupported_tack_diagnostic`] text on mismatch — never on
/// match (silent-on-match invariant).
#[must_use]
pub fn check_tack_version_with_emit(
    stdout: &str,
    stderr: &str,
    emit: &mut dyn FnMut(String),
) -> bool {
    let Some((maj, min)) = parse_tack_version(stdout, stderr) else {
        return false;
    };
    let supported = maj == TACK_PINNED_MAJOR && min == TACK_PINNED_MINOR;
    if !supported {
        emit(unsupported_tack_diagnostic(maj, min));
    }
    supported
}

/// Returns `true` iff `tack -V` reports a version compatible with
/// the begin-testing menu inventory pinned by Section 05.
///
/// Probe is `tack -V`. Output looks like `tack version 1.08
/// (20170726)`. Anything that doesn't parse, or any (major, minor)
/// tuple that doesn't match the pinned values, returns false.
/// Section 05 / 06 / 08 scenarios use this to skip cleanly when
/// running on an unpinned tack — the alternative is dozens of
/// cascading scenario failures that obscure the real issue (a tack
/// upgrade requires re-running discovery).
///
/// Returns `false` (not panic) on missing tack so this gate is
/// safe to call from `ScenarioRunner::available()` without an
/// extra existence check.
///
/// **Loud-skip discipline.** When `tack` IS installed but reports
/// a non-pinned version (e.g., a CI host upgraded to tack 2.x),
/// the function calls `eprintln!` with an actionable message
/// naming the observed version, the pinned version, and the
/// upgrade path. The `eprintln!` is the *only* loud signal — the
/// function still returns `false` so dozens of scenarios skip
/// cleanly instead of cascading failures. Without the loud
/// signal, an upgrade goes unnoticed and the test catalog quietly
/// stops covering anything.
///
/// **Upgrade path.** When a CI host upgrades tack:
/// 1. Update [`TACK_PINNED_MAJOR`] / [`TACK_PINNED_MINOR`] in
///    `session/mod.rs`.
/// 2. Run `INSTA_UPDATE=1 cargo test -p oriterm_core --test tack
///    -- test_menu::begin_testing_inventory` to capture the new
///    menu graph.
/// 3. Update `BEGIN_TESTING_INVENTORY` in
///    `scenarios/begin_testing_inventory.rs` to match the new
///    inventory.
/// 4. Re-run the full `test_menu` suite to update any snapshots
///    affected by changed menu wording.
#[must_use]
pub fn tack_version_supported() -> bool {
    let Ok(out) = std::process::Command::new("tack")
        .arg("-V")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    else {
        return false;
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    check_tack_version_with_emit(&stdout, &stderr, &mut |msg| eprintln!("{msg}"))
}

/// Pure AND-combine of the three test-availability gates.
///
/// Used by [`crate::tack_framework::ScenarioRunner::available`] and
/// pinned by a unit test that injects all 8 boolean combinations to
/// catch a regression that flips AND to OR.
#[must_use]
pub fn tack_runner_available_combine(tack: bool, tic: bool, version_supported: bool) -> bool {
    tack && tic && version_supported
}

#[cfg(test)]
mod tests;
