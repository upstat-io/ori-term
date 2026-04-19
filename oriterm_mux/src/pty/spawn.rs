//! PTY spawning, shell detection, and environment setup.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};

use super::PtyLifecycle;

/// Convert a `portable_pty` error into `io::Error`.
fn pty_err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

/// Exit status from a child process.
///
/// Wraps the underlying PTY library's exit status so callers don't depend
/// on `portable_pty` types directly.
#[derive(Debug, Clone)]
#[allow(
    dead_code,
    reason = "fields read by methods; methods used when UI reports exit status"
)]
pub struct ExitStatus {
    /// Process exit code.
    code: u32,
    /// Signal name if the process was terminated by a signal.
    signal: Option<String>,
}

#[allow(dead_code, reason = "used when UI reports exit status")]
impl ExitStatus {
    /// Returns `true` if the process exited successfully (code 0, no signal).
    pub fn success(&self) -> bool {
        self.signal.is_none() && self.code == 0
    }

    /// Returns the process exit code.
    pub fn exit_code(&self) -> u32 {
        self.code
    }

    /// Returns the signal name if the process was killed by a signal.
    pub fn signal(&self) -> Option<&str> {
        self.signal.as_deref()
    }

    /// Synthesize a "successful EOF" status for adopted PTYs.
    ///
    /// `ori_term` does not own the child process behind an adopted PTY —
    /// the console host that handed the session off owns it. When the
    /// reader thread observes EOF on the adopted reader, it signals
    /// `AdoptedPtyHandle::wait` to wake using this synthesized status.
    /// The actual exit code is unknown to `ori_term`; reporting `0`/no
    /// signal is the closest fit because the I/O stream closed cleanly.
    pub(crate) fn synthesized_eof() -> Self {
        Self {
            code: 0,
            signal: None,
        }
    }
}

impl From<portable_pty::ExitStatus> for ExitStatus {
    fn from(status: portable_pty::ExitStatus) -> Self {
        Self {
            code: status.exit_code(),
            signal: status.signal().map(String::from),
        }
    }
}

/// Owned PTY control handle for resize operations.
pub struct PtyControl(Box<dyn MasterPty + Send>);

impl PtyControl {
    /// Resize the PTY to the given dimensions.
    pub fn resize(&self, rows: u16, cols: u16) -> io::Result<()> {
        self.0
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(pty_err)
    }

    /// Get the PTY master file descriptor (Unix only).
    #[cfg(unix)]
    pub fn master_fd(&self) -> Option<std::os::unix::io::RawFd> {
        self.0.as_raw_fd()
    }
}

/// Configuration for spawning a PTY.
pub struct PtyConfig {
    /// Terminal dimensions in rows.
    pub rows: u16,
    /// Terminal dimensions in columns.
    pub cols: u16,
    /// Shell program override. If `None`, uses the platform default.
    pub shell: Option<String>,
    /// Working directory for the child process.
    pub working_dir: Option<PathBuf>,
    /// Additional environment variables to set in the child.
    pub env: Vec<(String, String)>,
    /// Enable shell integration (inject scripts for OSC 133/7 support).
    pub shell_integration: bool,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            shell: None,
            working_dir: None,
            env: Vec::new(),
            shell_integration: true,
        }
    }
}

/// Shared cell holding a child's exit status once the watcher thread
/// observes it via `Child::wait()`.
///
/// Uses `Result<ExitStatus, String>` (not `io::Error`) because `io::Error`
/// is not `Clone`. The watcher maps the underlying error to a string so
/// `PtyHandle::wait`/`try_wait` can clone the stored value and rehydrate
/// via `io::Error::other(s)`.
type ExitResultCell = Arc<Mutex<Option<Result<ExitStatus, String>>>>;

/// Handles to a spawned PTY.
///
/// # Ownership model (effect-cutover 01.1)
///
/// The child process (`Box<dyn portable_pty::Child>`) is owned by a
/// dedicated watcher thread spawned in [`spawn_pty`]. `PtyHandle` keeps
/// only:
/// - `killer: Box<dyn ChildKiller>` — cloned from `child.clone_killer()`
///   *before* the child is moved to the watcher, so `PtyLifecycle::kill`
///   still works.
/// - `process_id: Option<u32>` — captured from `child.process_id()` at
///   spawn time; never re-queried.
/// - `exit_result: Arc<Mutex<Option<Result<ExitStatus, String>>>>` —
///   shared cell populated by the watcher. `PtyLifecycle::wait` blocks
///   on `exit_notifier.wait(...)` until `exit_result` is `Some`;
///   `try_wait` reads it non-blocking.
/// - `exit_notifier: Arc<Condvar>` — signalled by the watcher after
///   writing `exit_result`.
///
/// [`spawn_pty`] now returns `(PtyHandle, Receiver<ExitStatus>)`. The
/// receiver side is threaded into `PaneIoThread` so the effect router
/// can emit `HostEffect::ChildExit { code }` on EOF + exit.
pub struct PtyHandle {
    reader: Option<Box<dyn io::Read + Send>>,
    writer: Option<Box<dyn io::Write + Send>>,
    control: Option<PtyControl>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    child_process_id: Option<u32>,
    exit_result: ExitResultCell,
    exit_notifier: Arc<Condvar>,
}

impl PtyHandle {
    /// Take the PTY output reader (child to parent).
    pub fn take_reader(&mut self) -> Option<Box<dyn io::Read + Send>> {
        self.reader.take()
    }

    /// Take the PTY input writer (parent to child).
    pub fn take_writer(&mut self) -> Option<Box<dyn io::Write + Send>> {
        self.writer.take()
    }

    /// Take the PTY control handle (for resize operations).
    pub fn take_control(&mut self) -> Option<PtyControl> {
        self.control.take()
    }

    /// Resize the PTY to new dimensions.
    #[allow(
        dead_code,
        reason = "used for direct resize before control handle is taken"
    )]
    pub fn resize(&self, rows: u16, cols: u16) -> io::Result<()> {
        let ctl = self
            .control
            .as_ref()
            .ok_or_else(|| io::Error::other("PTY control handle already taken"))?;
        ctl.resize(rows, cols)
    }

    /// Get the child process ID, if available.
    pub fn process_id(&self) -> Option<u32> {
        self.child_process_id
    }

    /// Kill the child process via the cloned `ChildKiller`.
    pub fn kill(&mut self) -> io::Result<()> {
        self.killer.kill()
    }

    /// Block until the child process has exited.
    ///
    /// Waits on `exit_notifier` until the watcher populates
    /// `exit_result`. Re-hydrates the stored `Result<_, String>` back
    /// into `io::Result<ExitStatus>`.
    #[allow(
        clippy::needless_pass_by_ref_mut,
        reason = "signature matches PtyLifecycle trait method which takes &mut self"
    )]
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        let mut guard = self.exit_result.lock().map_err(poisoned_mutex_err)?;
        while guard.is_none() {
            guard = self
                .exit_notifier
                .wait(guard)
                .map_err(poisoned_notifier_err)?;
        }
        clone_exit_result(guard.as_ref().expect("guard is Some after wait loop"))
    }

    /// Non-blocking check for child exit.
    #[allow(dead_code, reason = "used when pane reports child exit to UI")]
    #[allow(
        clippy::needless_pass_by_ref_mut,
        reason = "signature matches PtyLifecycle trait method which takes &mut self"
    )]
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let guard = self.exit_result.lock().map_err(poisoned_mutex_err)?;
        if let Some(r) = guard.as_ref() {
            Ok(Some(clone_exit_result(r)?))
        } else {
            Ok(None)
        }
    }
}

impl PtyLifecycle for PtyHandle {
    fn kill(&mut self) -> io::Result<()> {
        Self::kill(self)
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        Self::wait(self)
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        Self::try_wait(self)
    }

    fn process_id(&self) -> Option<u32> {
        Self::process_id(self)
    }
}

/// Clone a stored `Result<ExitStatus, String>` back to `io::Result<ExitStatus>`.
fn clone_exit_result(stored: &Result<ExitStatus, String>) -> io::Result<ExitStatus> {
    match stored {
        Ok(status) => Ok(status.clone()),
        Err(msg) => Err(io::Error::other(msg.clone())),
    }
}

#[cold]
fn poisoned_mutex_err<T>(_err: std::sync::PoisonError<T>) -> io::Error {
    io::Error::other("exit_result mutex poisoned")
}

#[cold]
fn poisoned_notifier_err<T>(_err: std::sync::PoisonError<T>) -> io::Error {
    io::Error::other("exit_notifier wait poisoned")
}

/// Spawn a PTY with the configured shell and environment.
///
/// Returns a `(PtyHandle, child_exit_rx)` pair. The receiver delivers
/// `ExitStatus` when the child exits; the sender lives on a dedicated
/// watcher thread that owns the `Box<dyn Child>` for the duration of
/// the process's lifetime.
pub fn spawn_pty(config: &PtyConfig) -> io::Result<(PtyHandle, Receiver<ExitStatus>)> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(pty_err)?;

    let cmd = build_command(config);

    let mut child = pair.slave.spawn_command(cmd).map_err(pty_err)?;

    // Capture kill handle and PID BEFORE moving child into the watcher.
    let killer = child.clone_killer();
    let child_process_id = child.process_id();

    // Drop the slave side so the reader detects EOF when child exits.
    drop(pair.slave);

    let reader = pair.master.try_clone_reader().map_err(pty_err)?;

    let writer = pair.master.take_writer().map_err(pty_err)?;

    // Shared cell + condvar — populated by the watcher thread on exit.
    let exit_result: ExitResultCell = Arc::new(Mutex::new(None));
    let exit_notifier = Arc::new(Condvar::new());
    let (child_exit_tx, child_exit_rx): (Sender<ExitStatus>, Receiver<ExitStatus>) = bounded(1);

    // Spawn the watcher thread.
    let watcher_exit_result = Arc::clone(&exit_result);
    let watcher_exit_notifier = Arc::clone(&exit_notifier);
    thread::Builder::new()
        .name("pty-child-watcher".into())
        .spawn(move || {
            let wait_result = child
                .wait()
                .map(ExitStatus::from)
                .map_err(|e| e.to_string());

            // Store the result and wake any waiters.
            if let Ok(mut guard) = watcher_exit_result.lock() {
                *guard = Some(wait_result.clone());
            }
            watcher_exit_notifier.notify_all();

            // Forward success onto the bounded channel. On error, drop
            // the sender — downstream `recv_timeout` will observe
            // `Disconnected` and fall back to a `code: 0` emission.
            if let Ok(status) = wait_result {
                let _ = child_exit_tx.send(status);
            }
            // Child dropped here — slave process is fully reaped.
        })?;

    let handle = PtyHandle {
        reader: Some(reader),
        writer: Some(writer),
        control: Some(PtyControl(pair.master)),
        killer,
        child_process_id,
        exit_result,
        exit_notifier,
    };
    Ok((handle, child_exit_rx))
}

/// Build a `CommandBuilder` with shell detection and environment variables.
pub(crate) fn build_command(config: &PtyConfig) -> CommandBuilder {
    use crate::shell_integration::set_common_env;

    let shell = config.shell.as_deref().unwrap_or_else(|| default_shell());

    let mut cmd = CommandBuilder::new(shell);

    if let Some(ref dir) = config.working_dir {
        cmd.cwd(dir);
    }

    // Terminal identification variables.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    // Oriterm identification env vars (ORITERM, TERM_PROGRAM, TERM_PROGRAM_VERSION).
    // Always set — not gated by shell_integration.
    set_common_env(&mut cmd);

    // User-provided overrides.
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Shell integration: detect shell, write scripts, configure injection.
    if config.shell_integration {
        inject_shell_integration(&mut cmd, shell, config.working_dir.as_deref());
    }

    // Propagate terminal + user variables across the Win32/WSL boundary.
    // Without WSLENV, tools running inside WSL won't see these env vars.
    #[cfg(windows)]
    build_wslenv(&mut cmd, config);

    cmd
}

/// Detect the shell and inject integration scripts if supported.
fn inject_shell_integration(
    cmd: &mut CommandBuilder,
    shell_program: &str,
    working_dir: Option<&Path>,
) {
    use crate::shell_integration::{detect_shell, ensure_scripts_on_disk, setup_injection};

    let Some(shell) = detect_shell(shell_program) else {
        log::info!("shell_integration: unknown shell '{shell_program}', skipping injection");
        return;
    };

    // Write scripts next to the executable.
    let base = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let integration_dir = match ensure_scripts_on_disk(&base) {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!("shell_integration: failed to write scripts: {e}");
            return;
        }
    };

    let cwd = working_dir.and_then(|p| p.to_str());
    if let Some(extra_arg) = setup_injection(cmd, shell, &integration_dir, cwd) {
        cmd.arg(extra_arg);
    }
}

/// Build the `WSLENV` value that propagates env vars across the Win32/WSL boundary.
#[cfg(windows)]
fn build_wslenv(cmd: &mut CommandBuilder, config: &PtyConfig) {
    let existing = std::env::var("WSLENV").unwrap_or_default();
    let user_keys: Vec<&str> = config.env.iter().map(|(k, _)| k.as_str()).collect();

    if let Some(wslenv) = compute_wslenv(&existing, &user_keys) {
        cmd.env("WSLENV", wslenv);
    }
}

/// Compute the new `WSLENV` value by merging builtin terminal variables and
/// user-provided keys into the existing value.
pub(crate) fn compute_wslenv(existing: &str, user_keys: &[&str]) -> Option<String> {
    use std::collections::HashSet;

    // Collect keys already present in WSLENV. Each entry is `KEY` or `KEY/flags`.
    let mut seen: HashSet<String> = existing
        .split(':')
        .filter(|s| !s.is_empty())
        .map(|entry| {
            // Each WSLENV entry is `KEY` or `KEY/flags` — extract just the key.
            entry
                .rsplit_once('/')
                .map_or(entry, |(key, _)| key)
                .to_uppercase()
        })
        .collect();

    // PATH must never be added — Windows PATH breaks WSL's computed PATH.
    seen.insert("PATH".into());

    // Variables we want to propagate across the WSL boundary.
    let builtin = [
        "TERM",
        "COLORTERM",
        "ORITERM",
        "TERM_PROGRAM",
        "TERM_PROGRAM_VERSION",
    ];

    let mut additions = String::new();
    for key in builtin.iter().copied().chain(user_keys.iter().copied()) {
        if seen.insert(key.to_uppercase()) {
            if !additions.is_empty() {
                additions.push(':');
            }
            additions.push_str(key);
        }
    }

    if additions.is_empty() {
        // Everything was already in WSLENV — nothing to add.
        return None;
    }

    if existing.is_empty() {
        Some(additions)
    } else {
        Some(format!("{existing}:{additions}"))
    }
}

/// Returns the default shell for the current platform.
#[cfg(windows)]
pub(crate) fn default_shell() -> &'static str {
    "cmd.exe"
}

/// Returns the default shell for the current platform.
#[cfg(not(windows))]
pub(crate) fn default_shell() -> &'static str {
    // Leak a static reference from the environment variable.
    // Called once at startup, so the small allocation is acceptable.
    static SHELL: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    SHELL.get_or_init(|| match std::env::var("SHELL") {
        Ok(shell) if !shell.is_empty() => Box::leak(shell.into_boxed_str()),
        _ => "/bin/sh",
    })
}
