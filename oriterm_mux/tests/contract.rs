//! Shared contract tests for the [`MuxBackend`] trait.
//!
//! A macro generates the same test suite for both [`EmbeddedMux`] (in-process)
//! and [`MuxClient`] (daemon IPC), verifying both backends produce identical
//! observable behavior for every `MuxBackend` method.

#![cfg(unix)]

use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use oriterm_core::grid::StableRowIndex;
use oriterm_core::selection::{Selection, SelectionMode, SelectionPoint};
use oriterm_core::{Side, Theme};
#[cfg(target_os = "linux")]
use oriterm_mux::MuxClient;
use oriterm_mux::backend::MuxBackend;
use oriterm_mux::domain::SpawnConfig;
#[cfg(target_os = "linux")]
use oriterm_mux::server::MuxServer;
use oriterm_mux::{EmbeddedMux, PaneId, PaneSnapshot, WireCursorShape};

// ---------------------------------------------------------------------------
// Test context: holds the backend + IDs + optional daemon handle
// ---------------------------------------------------------------------------

/// Wrapper providing a `MuxBackend` and the pane ID needed for testing.
///
/// Owns either an `EmbeddedMux` directly or a `MuxClient` + `TestDaemon`.
/// The daemon (if any) is kept alive by the `_daemon` field.
struct TestContext {
 backend: Box<dyn MuxBackend>,
 pane_id: PaneId,
 #[cfg(target_os = "linux")]
 _daemon: Option<TestDaemon>,
}

impl TestContext {
 /// Borrow the backend mutably.
 fn b(&mut self) -> &mut dyn MuxBackend {
 &mut *self.backend
 }

 /// Wait until the snapshot contains `text`, returning an owned copy.
 fn wait_for_text(&mut self, text: &str, timeout: Duration) -> PaneSnapshot {
 let deadline = Instant::now() + timeout;
 let pid = self.pane_id;
 loop {
 self.b().poll_events();
 let mut notifs = Vec::new();
 self.b().drain_notifications(&mut notifs);

 if let Some(snap) = self.b().refresh_pane_snapshot(pid) {
 if snapshot_contains(snap, text) {
 return snap.clone();
 }
 }

 assert!(
 Instant::now() < deadline,
 "timed out waiting for text {text:?} in pane {pid}"
);
 thread::sleep(Duration::from_millis(50));
 }
 }

 /// Refresh and return an owned snapshot.
 fn snapshot(&mut self) -> PaneSnapshot {
 let pid = self.pane_id;
 self.b()
 .refresh_pane_snapshot(pid)
 .expect("snapshot should be available")
 .clone()
 }

 /// Poll until a snapshot predicate is satisfied, returning an owned copy.
 fn wait_for(&mut self, what: &str, predicate: impl Fn(&PaneSnapshot) -> bool) -> PaneSnapshot {
 let deadline = Instant::now() + Duration::from_secs(30);
 let pid = self.pane_id;
 loop {
 self.b().poll_events();
 let mut notifs = Vec::new();
 self.b().drain_notifications(&mut notifs);

 if let Some(snap) = self.b().refresh_pane_snapshot(pid) {
 if predicate(snap) {
 return snap.clone();
 }
 }

 assert!(
 Instant::now() < deadline,
 "timed out waiting for condition: {what}"
);
 thread::sleep(Duration::from_millis(50));
 }
 }

 /// Wait until the snapshot contains `text` — but only call `poll_events`
 /// when `mux.has_pending_wakeup()` returns true, mirroring the gate logic
 /// in `oriterm/src/app/mux_pump/mod.rs:35-43`.
 ///
 /// This is the §05 Step 3 helper: the gated drain pins the
 /// MUX-LAYER CONTRACT that `App::pump_mux_events` relies on (early-exit
 /// when no wakeup pending, drain when set, flag-clear after poll). The
 /// existing [`wait_for_text`] polls unconditionally and is the right
 /// helper for general backend-progress contract tests where the gate's
 /// performance-optimization role doesn't apply.
 fn wait_for_text_via_gated_drain(&mut self, text: &str, timeout: Duration) -> PaneSnapshot {
 let deadline = Instant::now() + timeout;
 let pid = self.pane_id;
 loop {
 // Mirror App::pump_mux_events's exact gate logic:
 // if mux.has_pending_wakeup() { mux.poll_events(); drain_notifications(); }
 if self.b().has_pending_wakeup() {
 self.b().poll_events();
 let mut notifs = Vec::new();
 self.b().drain_notifications(&mut notifs);
 }

 if let Some(snap) = self.b().refresh_pane_snapshot(pid) {
 if snapshot_contains(snap, text) {
 return snap.clone();
 }
 }

 assert!(
 Instant::now() < deadline,
 "timed out waiting for text {text:?} via gated drain in pane {pid}"
);
 thread::sleep(Duration::from_millis(20));
 }
 }
}

/// Compose the shell command for a gated round-trip test.
///
/// Disables prompt-framework hooks BEFORE sending the query, then unsets the
/// shell prompt to a bare `$ ` and runs the query/capture/print sequence.
/// User shells (zsh + starship/p10k) install async stdin readers in their
/// `precmd` hooks that consume DA2/CSI 18t/DECRQM responses asynchronously
/// — the disable+unset step neutralizes those hooks so `head -c` is the
/// only reader competing for the PTY response bytes.
///
/// Per §02 consensus pattern: `stty raw -echo` disables ICANON +
/// ECHOCTL so response bytes pass byte-exact AND command echo doesn't
/// pre-trigger the parser; an inline `perl` reader reads up to N response
/// bytes within a 1-second SIGALRM budget (POSIX-portable, macOS / Linux /
/// BSD all ship `/usr/bin/perl` by default — unlike GNU coreutils `timeout`
/// which is absent from default macOS PATH on GitHub runners, and unlike
/// backgrounded `head -c N` which loses controlling-terminal access via
/// SIGTTIN); `od -An -tx1` emits the captured bytes in lowercase hex with
/// no spaces or newlines. Filename uses `$$` (shell PID) for uniqueness so
/// parallel tests in `cargo test` do not collide on /tmp.
///
/// `cup_to_origin: true` prepends `\x1b[1;1H` (CUP to row 1 col 1) so
/// cursor-position-dependent responses (DSR 6) report a deterministic
/// `\x1b[1;1R` regardless of where the shell prompt left the cursor.
///
/// **Sentinel discipline**: the `TPR_RESP=` marker on the printed line is
/// constructed via shell concatenation (`"TPR""_RESP="`) so the literal
/// substring `TPR_RESP=` never appears in the SHELL ECHO of this command
/// (which renders BEFORE `stty raw -echo` takes effect). The `TPR_RESP=`
/// substring appears in the snapshot ONLY after the printf runs at the end
/// of the pipeline, guaranteeing the wait-loop synchronizes on the actual
/// response, not on the command echo.
///
/// Uses octal escapes (`\033`) instead of hex (`\x1b`) — POSIX-portable
/// across `/bin/sh`, `bash`, and `zsh` (POSIX `printf` is not required to
/// support `\xNN`, but `\NNN` is mandated).
fn gated_round_trip_cmd(query: &str, n: usize, label: &str, cup_to_origin: bool) -> String {
 let cup = if cup_to_origin {
 "printf '\\033[1;1H'; "
 } else {
 ""
 };
 // Portable 1-second-bounded reader: perl with SIGALRM. `/usr/bin/perl`
 // ships on macOS, Linux, and BSD by default. Avoids GNU coreutils
 // `timeout` (missing on macOS GitHub runners) and the SIGTTIN trap that
 // breaks `( head -c N) &` in interactive shells. Reads one byte at a
 // time and writes immediately so any bytes that arrived before the
 // alarm fired are still captured.
 let reader = format!(
 "perl -e '$| = 1; eval {{ local $SIG{{ALRM}} = sub {{ exit 0 }}; alarm 1; \
 my $g = 0; while ($g < {n}) {{ my $r = sysread(STDIN, my $c, 1); last unless $r; \
 syswrite(STDOUT, $c); $g++; }} }}' > /tmp/tpr-{label}-$$"
);
 format!(
 "stty raw -echo; {cup}printf '{query}'; \
 {reader}; \
 printf '%s_RESP=BYTES=%s|HEX=%s\\n' 'TPR' \"$(wc -c < /tmp/tpr-{label}-$$)\" \
 \"$(od -An -tx1 -v /tmp/tpr-{label}-$$ | tr -d ' \\n')\"; \
 rm -f /tmp/tpr-{label}-$$; stty -raw echo\n"
)
}

/// Drive a gated round-trip for one device-query response kind.
///
/// Sends the composed shell command via `send_input`, then waits via
/// [`TestContext::wait_for_text_via_gated_drain`] for any `BYTES=N|HEX=...`
/// summary to appear in the snapshot. Returns the captured snapshot for
/// per-test assertions on the hex bytes. The caller asserts the exact
/// byte count via [`extract_hex_response`] which also serves as the
/// regression guard (an empty / 0-byte response renders as `BYTES=0|HEX=`).
///
/// `cup_to_origin: true` for cursor-position-dependent responses (DSR 6)
/// so the shell-prompt cursor placement does not pollute the response.
fn assert_gated_round_trip(
 ctx: &mut TestContext,
 query: &str,
 n: usize,
 label: &str,
 cup_to_origin: bool,
) -> PaneSnapshot {
 let pid = ctx.pane_id;
 let cmd = gated_round_trip_cmd(query, n, label, cup_to_origin);
 ctx.b().send_input(pid, cmd.as_bytes());
 // Wait for the runtime-constructed `TPR_RESP=` sentinel — see
 // `gated_round_trip_cmd` doc for why this never appears in the
 // command echo. Synchronizes on the actual printf output, not on
 // the shell's pre-`stty -echo` line echo.
 let snap = ctx.wait_for_text_via_gated_drain("TPR_RESP=BYTES=", Duration::from_secs(30));
 // Regression guard per §03: response bytes must NOT be empty.
 let line = snapshot_find_line_with(&snap, "TPR_RESP=BYTES=").unwrap_or_default();
 assert!(
 !line.contains("BYTES=0|"),
 "regression guard: response did not arrive (label={label}, line={line:?})"
);
 snap
}

/// Find the first row in the snapshot that contains `needle`; return the
/// row text trimmed of trailing spaces.
fn snapshot_find_line_with(snapshot: &PaneSnapshot, needle: &str) -> Option<String> {
 for row in &snapshot.cells {
 let line: String = row.iter().map(|c| c.ch).collect();
 if line.contains(needle) {
 return Some(line.trim_end().to_string());
 }
 }
 None
}

/// Extract the hex blob from the first `TPR_RESP=BYTES=N|HEX=<hex>` line in
/// the snapshot. Returns the hex blob with no leading/trailing whitespace.
fn extract_hex_response(snapshot: &PaneSnapshot) -> String {
 let line = snapshot_find_line_with(snapshot, "TPR_RESP=BYTES=").unwrap_or_default();
 line.split("HEX=").nth(1).unwrap_or("").trim().to_string()
}

/// Extract the byte-count from the `TPR_RESP=BYTES=N|HEX=...` line.
fn extract_byte_count(snapshot: &PaneSnapshot) -> usize {
 let line = snapshot_find_line_with(snapshot, "TPR_RESP=BYTES=").unwrap_or_default();
 line.split("BYTES=")
 .nth(1)
 .and_then(|s| s.split('|').next())
 .and_then(|s| s.trim().parse::<usize>().ok())
 .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// TestDaemon (duplicated from e2e.rs — integration tests can't share code)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
struct TestDaemon {
 socket_path: std::path::PathBuf,
 shutdown: Arc<std::sync::atomic::AtomicBool>,
 thread: Option<thread::JoinHandle<()>>,
 _tmpdir: tempfile::TempDir,
}

#[cfg(target_os = "linux")]
impl TestDaemon {
 fn start() -> Self {
 let tmpdir = tempfile::tempdir().expect("failed to create temp dir");
 let socket_path = tmpdir.path().join("mux.sock");
 let pid_path = tmpdir.path().join("mux.pid");

 let mut server =
 MuxServer::with_paths(&socket_path, &pid_path).expect("failed to create MuxServer");
 let shutdown = server.shutdown_flag();

 let thread = thread::spawn(move || {
 if let Err(e) = server.run() {
 eprintln!("MuxServer error: {e}");
 }
 });

 let deadline = Instant::now() + Duration::from_secs(30);
 while !socket_path.exists() {
 if Instant::now() > deadline {
 panic!("daemon socket did not appear within 5 seconds");
 }
 thread::sleep(Duration::from_millis(10));
 }

 Self {
 socket_path,
 shutdown,
 thread: Some(thread),
 _tmpdir: tmpdir,
 }
 }

 fn connect_client(&self) -> MuxClient {
 let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
 MuxClient::connect(&self.socket_path, wakeup).expect("failed to connect MuxClient")
 }
}

#[cfg(target_os = "linux")]
impl Drop for TestDaemon {
 fn drop(&mut self) {
 self.shutdown.store(true, Ordering::Release);
 if let Some(handle) = self.thread.take() {
 let _ = handle.join();
 }
 }
}

// ---------------------------------------------------------------------------
// Factory functions
// ---------------------------------------------------------------------------

/// Build a `SpawnConfig` for tests with history suppressed so fence commands
/// don't pollute the user's `~/.zsh_history`.
fn test_spawn_config() -> SpawnConfig {
 SpawnConfig {
 env: vec![("HISTFILE".into(), "/dev/null".into())],
 ..SpawnConfig::default()
 }
}

/// Create a `TestContext` backed by `EmbeddedMux`.
fn embedded_context() -> TestContext {
 let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
 let mut mux = EmbeddedMux::new(wakeup);

 let config = test_spawn_config();
 let pane_id = mux.spawn_pane(&config, Theme::Dark).expect("spawn_pane");

 // Wait for the shell to be ready by sending a fence command and
 // polling until its output appears — no fixed sleep.
 wait_for_shell_ready(&mut mux, pane_id);

 TestContext {
 backend: Box::new(mux),
 pane_id,
 #[cfg(target_os = "linux")]
 _daemon: None,
 }
}

/// Create a `TestContext` backed by `MuxClient` connected to a `TestDaemon`.
#[cfg(target_os = "linux")]
fn daemon_context() -> TestContext {
 let daemon = TestDaemon::start();
 let mut client = daemon.connect_client();

 let config = test_spawn_config();
 let pane_id = client.spawn_pane(&config, Theme::Dark).expect("spawn_pane");

 // Wait for the shell to be ready by sending a fence command and
 // polling until its output appears — no fixed sleep.
 wait_for_shell_ready(&mut client, pane_id);

 TestContext {
 backend: Box::new(client),
 pane_id,
 _daemon: Some(daemon),
 }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn snapshot_contains(snapshot: &PaneSnapshot, text: &str) -> bool {
 snapshot.cells.iter().any(|row| {
 let line: String = row.iter().map(|c| c.ch).collect();
 line.contains(text)
 })
}

/// Wait for the shell to be ready by sending a fence and polling until
/// its output appears. This replaces fixed `thread::sleep` calls.
fn wait_for_shell_ready(backend: &mut dyn MuxBackend, pane_id: PaneId) {
 backend.send_input(pane_id, b"echo SHELL_READY_FENCE\n");
 let deadline = Instant::now() + Duration::from_secs(30);
 loop {
 backend.poll_events();
 let mut n = Vec::new();
 backend.drain_notifications(&mut n);
 if let Some(snap) = backend.refresh_pane_snapshot(pane_id) {
 // Wait for the output line (not just the command echo).
 let count = snap
 .cells
 .iter()
 .filter(|row| {
 let line: String = row.iter().map(|c| c.ch).collect();
 line.contains("SHELL_READY_FENCE")
 })
 .count();
 if count >= 2 {
 return;
 }
 }
 assert!(
 Instant::now() < deadline,
 "shell did not start within 30 seconds"
);
 thread::sleep(Duration::from_millis(50));
 }
}

// ---------------------------------------------------------------------------
// Contract test macro
// ---------------------------------------------------------------------------

/// Generate identical test functions for both backends.
///
/// Each test receives a `TestContext` with a window, tab, and pane already
/// created and the shell initialized. Wrapped in a `mod` for namespacing.
macro_rules! muxbackend_contract_tests {
 ($factory:path) => {
 use super::*;

 #[test]
 fn contract_spawn_pane_and_see_output() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;
 ctx.b().send_input(pid, b"echo CONTRACT_OUTPUT\n");
 let snap = ctx.wait_for_text("CONTRACT_OUTPUT", Duration::from_secs(30));
 assert!(snapshot_contains(&snap, "CONTRACT_OUTPUT"));
 }

 #[test]
 fn contract_resize() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;
 ctx.b().resize_pane_grid(pid, 30, 90);

 // Poll until the resize is reflected in the snapshot.
 // CI runners can be slow so a fixed sleep is unreliable.
 let deadline = Instant::now() + Duration::from_secs(30);
 loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 if let Some(snap) = ctx.b().refresh_pane_snapshot(pid) {
 if snap.cols == 90 && snap.cells.len() == 30 {
 return;
 }
 }
 assert!(
 Instant::now() < deadline,
 "timed out waiting for resize to 30x90"
);
 thread::sleep(Duration::from_millis(50));
 }
 }

 #[test]
 fn contract_scroll() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;

 // Generate scrollback and wait for completion. Send a fence
 // command after the loop so we know all output has landed.
 ctx.b()
 .send_input(pid, b"for i in $(seq 1 200); do echo L$i; done\n");
 ctx.b().send_input(pid, b"echo SCROLL_FENCE\n");
 // Wait for the fence output (not the command echo). When the
 // fence appears on 2 rows, the loop output is fully rendered.
 let deadline = Instant::now() + Duration::from_secs(30);
 loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 if let Some(snap) = ctx.b().refresh_pane_snapshot(pid) {
 let count = snap
 .cells
 .iter()
 .filter(|row| {
 let line: String = row.iter().map(|c| c.ch).collect();
 line.contains("SCROLL_FENCE")
 })
 .count();
 if count >= 2 {
 break;
 }
 }
 assert!(
 Instant::now() < deadline,
 "timed out waiting for scroll fence"
);
 thread::sleep(Duration::from_millis(50));
 }
 // Send a second fence and wait for it — this ensures the shell
 // prompt after the loop has fully rendered, preventing a race
 // where late-arriving prompt output resets display_offset.
 ctx.b().send_input(pid, b"echo QUIESCE_FENCE\n");
 let deadline2 = Instant::now() + Duration::from_secs(30);
 loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 if let Some(snap) = ctx.b().refresh_pane_snapshot(pid) {
 let count = snap
 .cells
 .iter()
 .filter(|row| {
 let line: String = row.iter().map(|c| c.ch).collect();
 line.contains("QUIESCE_FENCE")
 })
 .count();
 if count >= 2 {
 break;
 }
 }
 assert!(
 Instant::now() < deadline2,
 "timed out waiting for quiesce fence"
);
 thread::sleep(Duration::from_millis(50));
 }

 // Wait for the shell to fully settle — no new dirty notifications
 // for 300ms. Late-arriving prompt redraws can reset display_offset
 // after scroll_display, causing flaky failures in the daemon path.
 ctx.b().refresh_pane_snapshot(pid);
 ctx.b().clear_pane_snapshot_dirty(pid);
 let mut quiet_since = Instant::now();
 let quiesce_deadline = Instant::now() + Duration::from_secs(10);
 while quiet_since.elapsed() < Duration::from_millis(300) {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 if ctx.b().is_pane_snapshot_dirty(pid) {
 ctx.b().refresh_pane_snapshot(pid);
 ctx.b().clear_pane_snapshot_dirty(pid);
 quiet_since = Instant::now();
 }
 assert!(
 Instant::now() < quiesce_deadline,
 "shell never quiesced after QUIESCE_FENCE"
);
 thread::sleep(Duration::from_millis(20));
 }

 // Scroll up.
 ctx.b().scroll_display(pid, 10);
 let snap = ctx.wait_for("display_offset == 10", |s| s.display_offset == 10);
 assert_eq!(
 snap.display_offset, 10,
 "display_offset after scroll_display(10)"
);

 // Scroll to bottom.
 ctx.b().scroll_to_bottom(pid);
 let snap = ctx.wait_for("display_offset == 0", |s| s.display_offset == 0);
 assert_eq!(
 snap.display_offset, 0,
 "display_offset after scroll_to_bottom"
);
 }

 #[test]
 fn contract_mode_query() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;
 let bracketed_paste_bit = 1u64 << 13;

 // Use printf to emit the DECSET sequence through the shell's
 // stdout, ensuring the terminal emulator processes it.
 ctx.b().send_input(pid, b"printf '\\033[?2004h'\n");

 // Poll until the mode bit is set (avoids flaky fixed timeouts).
 let deadline = Instant::now() + Duration::from_secs(30);
 loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 let snap = ctx.snapshot();
 if snap.modes & bracketed_paste_bit != 0 {
 break;
 }
 assert!(
 Instant::now() < deadline,
 "timed out waiting for bracketed paste mode bit"
);
 thread::sleep(Duration::from_millis(50));
 }
 }

 #[test]
 fn contract_cursor_shape() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;
 ctx.b()
 .set_cursor_shape(pid, oriterm_core::CursorShape::Bar);
 let snap = ctx.wait_for("cursor shape Bar", |s| {
 s.cursor.shape == WireCursorShape::Bar
 });
 assert_eq!(
 snap.cursor.shape,
 WireCursorShape::Bar,
 "cursor shape should be Bar"
);
 }

 #[test]
 fn contract_snapshot_lifecycle() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;

 // Refresh + clear dirty. In daemon mode the refresh is async
 // (triggers MarkAllDirty → server pushes snapshot), so we must
 // poll until the pushed snapshot arrives and the clear succeeds.
 let deadline = Instant::now() + Duration::from_secs(10);
 loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 ctx.b().refresh_pane_snapshot(pid);
 ctx.b().clear_pane_snapshot_dirty(pid);
 if !ctx.b().is_pane_snapshot_dirty(pid) {
 break;
 }
 assert!(
 Instant::now() < deadline,
 "timed out waiting for clean snapshot state"
);
 thread::sleep(Duration::from_millis(20));
 }

 // Generate output → dirty.
 ctx.b().send_input(pid, b"echo DIRTY\n");
 let deadline = Instant::now() + Duration::from_secs(30);
 loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 if ctx.b().is_pane_snapshot_dirty(pid) {
 break;
 }
 assert!(Instant::now() < deadline, "timed out waiting for dirty");
 thread::sleep(Duration::from_millis(20));
 }
 assert!(ctx.b().is_pane_snapshot_dirty(pid), "should be dirty");

 ctx.b().clear_pane_snapshot_dirty(pid);
 assert!(
 !ctx.b().is_pane_snapshot_dirty(pid),
 "should be clean again"
);
 }

 #[test]
 fn contract_search() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;
 ctx.b().send_input(pid, b"echo NEEDLE\n");
 ctx.wait_for_text("NEEDLE", Duration::from_secs(30));

 // Open search.
 ctx.b().open_search(pid);
 let snap = ctx.wait_for("search_active", |s| s.search_active);
 assert!(snap.search_active, "search should be active");

 // Set query.
 ctx.b().search_set_query(pid, "NEEDLE".to_string());
 let snap = ctx.wait_for("search matches", |s| !s.search_matches.is_empty());
 assert_eq!(snap.search_query, "NEEDLE");
 assert!(!snap.search_matches.is_empty(), "should find NEEDLE");

 // Close search.
 ctx.b().close_search(pid);
 let snap = ctx.wait_for("search inactive", |s| !s.search_active);
 assert!(!snap.search_active, "search should be inactive");
 assert!(snap.search_matches.is_empty(), "matches should be cleared");
 }

 #[test]
 fn contract_flood_output() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;

 // Generate large output: 2000 lines of 120-char padded numbers.
 // Kept smaller than a stress test so daemon IPC finishes within CI
 // time limits even at --test-threads=2.
 ctx.b().send_input(
 pid,
 b"for i in $(seq 1 2000); do printf '%0120d\\n' $i; done\n",
);

 // Poll until the flood finishes (last line appears).
 // CI runners (especially macOS) are slow — use a generous deadline.
 let deadline = Instant::now() + Duration::from_secs(60);
 loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);
 if let Some(snap) = ctx.b().refresh_pane_snapshot(pid) {
 if snapshot_contains(snap, "2000") {
 break;
 }
 }
 assert!(
 Instant::now() < deadline,
 "timed out during flood output — main thread likely blocked"
);
 thread::sleep(Duration::from_millis(100));
 }

 // Verify responsiveness after the flood.
 ctx.b().send_input(pid, b"echo FLOOD_ALIVE\n");
 let snap = ctx.wait_for_text("FLOOD_ALIVE", Duration::from_secs(30));
 assert!(snapshot_contains(&snap, "FLOOD_ALIVE"));
 }

 /// Simulates the real UI rendering loop during flood output.
 ///
 /// Unlike `contract_flood_output` (which sleeps 100ms between polls),
 /// this test calls `refresh_pane_snapshot` in a tight loop at ~60fps,
 /// matching the actual rendering cadence. The test fails if the main
 /// thread blocks for more than 500ms on any single snapshot refresh,
 /// which would manifest as a UI hang/freeze in the real application.
 #[test]
 fn contract_flood_render_loop() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;

 // Start infinite flood output.
 ctx.b()
 .send_input(pid, b"while true; do printf '%0200d\\n' 1; done\n");

 // Simulate the UI rendering loop for 3 seconds.
 // The real App does: poll_events → refresh_snapshot → GPU render.
 // We skip GPU but measure how long each snapshot takes.
 let test_duration = Duration::from_secs(3);
 // CI runners can have extreme scheduling latency — only fail
 // on multi-second hangs, not scheduling jitter.
 let max_frame_time = Duration::from_secs(2);
 let start = Instant::now();
 let mut frame_count = 0u32;
 let mut max_snapshot_time = Duration::ZERO;
 let mut saw_output = false;

 while start.elapsed() < test_duration {
 let frame_start = Instant::now();

 // Phase 1: poll events (what about_to_wait does).
 ctx.b().poll_events();
 let mut notifs = Vec::new();
 ctx.b().drain_notifications(&mut notifs);

 // Phase 2: refresh snapshot (what handle_redraw does).
 // This is where the hang occurs — build_snapshot blocks on
 // pane.terminal().lock(), a fair lock that waits for the
 // PTY reader's lease to release.
 if ctx.b().is_pane_snapshot_dirty(pid) || ctx.b().pane_snapshot(pid).is_none() {
 ctx.b().refresh_pane_snapshot(pid);
 }
 ctx.b().clear_pane_snapshot_dirty(pid);

 // Check if we got any output (sanity check).
 if let Some(snap) = ctx.b().pane_snapshot(pid) {
 if snapshot_contains(snap, "0000000") {
 saw_output = true;
 }
 }

 let frame_time = frame_start.elapsed();
 if frame_time > max_snapshot_time {
 max_snapshot_time = frame_time;
 }

 // This is the critical assertion: no single frame should
 // block for more than 500ms. A hang would block indefinitely.
 assert!(
 frame_time < max_frame_time,
 "frame {frame_count} took {frame_time:?} (max {max_frame_time:?}) — \
 main thread blocked on terminal lock during flood output"
);

 frame_count += 1;

 // Simulate GPU render time (~16ms for 60fps VSync).
 thread::sleep(Duration::from_millis(16));
 }

 // Stop the flood.
 ctx.b().send_input(pid, b"\x03");
 thread::sleep(Duration::from_millis(200));

 let elapsed = start.elapsed();
 let fps = frame_count as f64 / elapsed.as_secs_f64();

 eprintln!("--- flood render loop ---");
 eprintln!(" frames: {frame_count}");
 eprintln!(" fps: {fps:.1}");
 eprintln!(" max frame time: {max_snapshot_time:?}");
 eprintln!(" saw output: {saw_output}");

 // Must achieve at least 5 fps — CI runners (especially macOS)
 // run significantly slower than local machines. Real target is
 // 60; this threshold only catches true hangs.
 assert!(
 fps >= 5.0,
 "rendering too slow during flood: {fps:.1} fps (need >= 5)"
);
 assert!(saw_output, "flood output never appeared in snapshots");
 }

 #[test]
 fn contract_extract_text() {
 let mut ctx = $factory();
 let pid = ctx.pane_id;

 ctx.b().send_input(pid, b"echo CXTR_MARKER\n");

 // Wait until the output row (containing CXTR_MARKER but not
 // "echo") appears. We need both the command echo and the
 // output line to be present.
 let deadline = Instant::now() + Duration::from_secs(30);
 let (snap, target_row, row_text) = loop {
 ctx.b().poll_events();
 let mut n = Vec::new();
 ctx.b().drain_notifications(&mut n);

 if let Some(snap) = ctx.b().refresh_pane_snapshot(pid) {
 let found = snap
 .cells
 .iter()
 .enumerate()
 .filter_map(|(i, row)| {
 let line: String = row.iter().map(|c| c.ch).collect();
 if line.contains("CXTR_MARKER") && !line.contains("echo") {
 Some((i, line))
 } else {
 None
 }
 })
 .next();
 if let Some((row, text)) = found {
 break (snap.clone(), row, text);
 }
 }
 assert!(
 Instant::now() < deadline,
 "timed out waiting for CXTR_MARKER output row"
);
 thread::sleep(Duration::from_millis(50));
 };

 let col_start = row_text
 .find("CXTR_MARKER")
 .expect("should find text in row");
 let col_end = col_start + "CXTR_MARKER".len() - 1;
 let abs_row = snap.stable_row_base + target_row as u64;

 let selection = Selection {
 mode: SelectionMode::Char,
 anchor: SelectionPoint {
 row: StableRowIndex(abs_row),
 col: col_start,
 side: Side::Left,
 },
 pivot: SelectionPoint {
 row: StableRowIndex(abs_row),
 col: col_start,
 side: Side::Left,
 },
 end: SelectionPoint {
 row: StableRowIndex(abs_row),
 col: col_end,
 side: Side::Right,
 },
 };

 let text = ctx
 .b()
 .extract_text(pid, &selection)
 .expect("extract_text should return text");
 assert_eq!(text.trim(), "CXTR_MARKER");
 }

 // ───────────────────────────────────────────────────────────────
 // §03 Cross-kind matrix: gated round-trip per kind
 //
 // Each test sends a device-query (DA/DSR/CSI 18t/DECRQM/DECRQSS)
 // through a real PTY via `stty raw -echo + printf + head -c`,
 // then drives the gated drain (`if mux.has_pending_wakeup()
 // { mux.poll_events(); }`) until the BYTES=N|HEX=... summary
 // appears in the snapshot. Pins the FULL CONTRACT
 // App::pump_mux_events relies on at oriterm/src/app/mux_pump/mod.rs:35-43:
 // gate-flag transitions, response-bytes path through poll_events,
 // wakeup-callback shape, coalescing guard.
 // ───────────────────────────────────────────────────────────────

 #[test]
 fn da1_gated_round_trip() {
 let mut ctx = $factory();
 // DA1: \x1b[c → \x1b[?64;6;4c (10 bytes)
 let snap = assert_gated_round_trip(&mut ctx, "\\033[c", 10, "da1", false);
 assert_eq!(extract_byte_count(&snap), 10, "DA1 byte count mismatch");
 let hex = extract_hex_response(&snap);
 // ESC [ ? 6 4; 6; 4 c → 1b 5b 3f 36 34 3b 36 3b 34 63
 assert_eq!(hex, "1b5b3f36343b363b3463", "DA1 response bytes mismatch");
 }

 #[test]
 #[ignore = "DA2 response consumed by interactive shell prompt-framework (zsh + starship/p10k); Layer-1 test in oriterm_mux/src/pane/io_thread/effect_router/tests.rs::da2_byte_parse_emits_pty_write_response_with_version pins the byte-parse → MuxEvent emission contract; un-ignore once a portable test seam (Rust helper binary or non-shell PTY harness) lands per the §07 follow-up bug filing."]
 fn da2_gated_round_trip() {
 let mut ctx = $factory();
 // DA2: \x1b[>c → \x1b[>0;<version>;1c — variable version length.
 // version_number for "0.2.0" → 200, so realistic response is
 // \x1b[>0;200;1c = 11 bytes. Use n=14 max with timeout-bounded
 // read; pattern match prefix/suffix on captured bytes.
 let snap = assert_gated_round_trip(&mut ctx, "\\033[>c", 14, "da2", false);
 let bytes = extract_byte_count(&snap);
 assert!(bytes >= 9 && bytes <= 14, "DA2 byte count out of range: {bytes}");
 let hex = extract_hex_response(&snap);
 // Prefix: ESC [ > 0; → 1b5b3e303b
 // Suffix:; 1 c → 3b3163 (last 6 hex chars)
 assert!(hex.starts_with("1b5b3e303b"), "DA2 prefix mismatch: {hex}");
 assert!(hex.ends_with("3b3163"), "DA2 suffix mismatch: {hex}");
 }

 #[test]
 fn da3_gated_round_trip() {
 let mut ctx = $factory();
 // DA3: \x1b[=c → \x1bP!|00000000\x1b\\ (14 bytes)
 let snap = assert_gated_round_trip(&mut ctx, "\\033[=c", 14, "da3", false);
 assert_eq!(extract_byte_count(&snap), 14, "DA3 byte count mismatch");
 let hex = extract_hex_response(&snap);
 // ESC P ! | 0 0 0 0 0 0 0 0 ESC \ → 1b 50 21 7c 30303030 30303030 1b 5c
 assert_eq!(hex, "1b50217c30303030303030301b5c", "DA3 response bytes mismatch");
 }

 #[test]
 fn dsr5_gated_round_trip() {
 let mut ctx = $factory();
 // DSR 5: \x1b[5n → \x1b[0n (4 bytes)
 let snap = assert_gated_round_trip(&mut ctx, "\\033[5n", 4, "dsr5", false);
 assert_eq!(extract_byte_count(&snap), 4, "DSR 5 byte count mismatch");
 let hex = extract_hex_response(&snap);
 // ESC [ 0 n → 1b5b306e
 assert_eq!(hex, "1b5b306e", "DSR 5 response bytes mismatch");
 }

 #[test]
 fn dsr6_gated_round_trip() {
 let mut ctx = $factory();
 // DSR 6: \x1b[6n → \x1b[<line>;<col>R. cup_to_origin=true forces
 // cursor to row 1 col 1 BEFORE the query so the response is
 // deterministically \x1b[1;1R = 6 bytes regardless of where the
 // shell prompt left the cursor.
 let snap = assert_gated_round_trip(&mut ctx, "\\033[6n", 6, "dsr6", true);
 assert_eq!(extract_byte_count(&snap), 6, "DSR 6 byte count mismatch");
 let hex = extract_hex_response(&snap);
 // ESC [ 1; 1 R → 1b 5b 31 3b 31 52
 assert_eq!(hex, "1b5b313b3152", "DSR 6 response bytes mismatch");
 }

 #[test]
 #[ignore = "CSI 18t response consumed by interactive shell prompt-framework (zsh + starship/p10k); Layer-1 test in oriterm_mux/src/pane/io_thread/effect_router/tests.rs::csi_18t_byte_parse_at_default_grid_emits_size_24_80 pins the byte-parse → MuxEvent emission contract; un-ignore once a portable test seam (Rust helper binary or non-shell PTY harness) lands per the §07 follow-up bug filing."]
 fn csi_18t_gated_round_trip() {
 let mut ctx = $factory();
 // CSI 18t: \x1b[18t → \x1b[8;<lines>;<cols>t. Default test grid
 // dimensions vary by PTY environment; use timeout-bounded read
 // and pattern match the prefix/suffix on captured bytes.
 let snap = assert_gated_round_trip(&mut ctx, "\\033[18t", 14, "csi18t", false);
 let bytes = extract_byte_count(&snap);
 assert!(bytes >= 8 && bytes <= 14, "CSI 18t byte count out of range: {bytes}");
 let hex = extract_hex_response(&snap);
 // Prefix: ESC [ 8; → 1b5b383b
 // Suffix: t → 74 (last 2 hex chars)
 assert!(hex.starts_with("1b5b383b"), "CSI 18t prefix mismatch: {hex}");
 assert!(hex.ends_with("74"), "CSI 18t suffix mismatch: {hex}");
 }

 #[test]
 fn decrqm_set_gated_round_trip() {
 let mut ctx = $factory();
 // DECRQM ?25$p → \x1b[?25;1$y — cursor visible by default = set.
 // Response: ESC [ ? 2 5; 1 $ y = 9 bytes.
 let snap = assert_gated_round_trip(&mut ctx, "\\033[?25\\044p", 9, "decrqm-set", false);
 assert_eq!(extract_byte_count(&snap), 9, "DECRQM ?25 byte count mismatch");
 let hex = extract_hex_response(&snap);
 assert_eq!(hex, "1b5b3f32353b312479", "DECRQM ?25 response bytes mismatch");
 }

 #[test]
 fn decrqm_reset_gated_round_trip() {
 let mut ctx = $factory();
 // DECRQM ?1049$p → \x1b[?1049;2$y — alt screen off by default = reset.
 // Response: ESC [ ? 1 0 4 9; 2 $ y = 11 bytes.
 let snap = assert_gated_round_trip(&mut ctx, "\\033[?1049\\044p", 11, "decrqm-reset", false);
 assert_eq!(extract_byte_count(&snap), 11, "DECRQM ?1049 byte count mismatch");
 let hex = extract_hex_response(&snap);
 assert_eq!(hex, "1b5b3f313034393b322479", "DECRQM ?1049 response bytes mismatch");
 }

 #[test]
 fn decrqm_unknown_gated_round_trip() {
 let mut ctx = $factory();
 // DECRQM ?9999$p → \x1b[?9999;0$y → unknown mode = 0.
 // Response: ESC [ ? 9 9 9 9; 0 $ y = 11 bytes.
 let snap = assert_gated_round_trip(&mut ctx, "\\033[?9999\\044p", 11, "decrqm-unknown", false);
 assert_eq!(extract_byte_count(&snap), 11, "DECRQM ?9999 byte count mismatch");
 let hex = extract_hex_response(&snap);
 assert_eq!(hex, "1b5b3f393939393b302479", "DECRQM ?9999 response bytes mismatch");
 }

 #[test]
 fn decrqss_decscl_gated_round_trip() {
 let mut ctx = $factory();
 // DECRQSS DECSCL: \x1bP$q"p\x1b\\ → \x1bP1$r64;1"p\x1b\\
 // Response: ESC P 1 $ r 6 4; 1 " p ESC \ = 13 bytes.
 let snap = assert_gated_round_trip(
 &mut ctx,
 "\\033P\\044q\\042p\\033\\\\",
 13,
 "decrqss-decscl",
 false,
);
 assert_eq!(extract_byte_count(&snap), 13, "DECRQSS DECSCL byte count mismatch");
 let hex = extract_hex_response(&snap);
 // ESC P 1 $ r 6 4; 1 " p ESC \ → 1b 50 31 24 72 36 34 3b 31 22 70 1b 5c
 assert_eq!(
 hex, "1b5031247236343b3122701b5c",
 "DECRQSS DECSCL response bytes mismatch"
);
 }

 #[test]
 fn decrqss_sgr_gated_round_trip() {
 let mut ctx = $factory();
 // DECRQSS SGR: \x1bP$qm\x1b\\ → \x1bP1$r0m\x1b\\ at default cursor template.
 // Response: ESC P 1 $ r 0 m ESC \ = 9 bytes.
 let snap = assert_gated_round_trip(
 &mut ctx,
 "\\033P\\044qm\\033\\\\",
 9,
 "decrqss-sgr",
 false,
);
 assert_eq!(extract_byte_count(&snap), 9, "DECRQSS SGR byte count mismatch");
 let hex = extract_hex_response(&snap);
 // ESC P 1 $ r 0 m ESC \ → 1b 50 31 24 72 30 6d 1b 5c
 assert_eq!(hex, "1b50312472306d1b5c", "DECRQSS SGR response bytes mismatch");
 }

 #[test]
 fn decrqss_unknown_gated_round_trip() {
 let mut ctx = $factory();
 // DECRQSS unknown: \x1bP$qXX\x1b\\ → \x1bP0$r\x1b\\
 // Response: ESC P 0 $ r ESC \ = 7 bytes.
 let snap = assert_gated_round_trip(
 &mut ctx,
 "\\033P\\044qXX\\033\\\\",
 7,
 "decrqss-unknown",
 false,
);
 assert_eq!(extract_byte_count(&snap), 7, "DECRQSS unknown byte count mismatch");
 let hex = extract_hex_response(&snap);
 // ESC P 0 $ r ESC \ → 1b 50 30 24 72 1b 5c
 assert_eq!(hex, "1b503024721b5c", "DECRQSS unknown response bytes mismatch");
 }
 };
}

// ---------------------------------------------------------------------------
// Instantiate for both backends
// ---------------------------------------------------------------------------

mod embedded {
 muxbackend_contract_tests!(embedded_context);
}

// Daemon IPC does not work reliably on macOS CI runners — shells spawned
// through the daemon never produce output. Embedded tests verify the same
// contract, so skipping daemon tests on macOS is safe.
#[cfg(target_os = "linux")]
mod daemon {
 muxbackend_contract_tests!(daemon_context);
}
