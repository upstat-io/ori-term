//! PTY spawning, shell detection, and environment setup.

use std::io;
use std::path::PathBuf;

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

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
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            shell: None,
            working_dir: None,
            env: Vec::new(),
        }
    }
}

/// Handles to a spawned PTY and its child process.
///
/// The reader and writer are taken separately via [`take_reader`] and
/// [`take_writer`] for use by the reader thread and input handler.
/// Resize, kill, and wait operations remain available on the handle.
///
/// [`take_reader`]: PtyHandle::take_reader
/// [`take_writer`]: PtyHandle::take_writer
pub struct PtyHandle {
    reader: Option<Box<dyn io::Read + Send>>,
    writer: Option<Box<dyn io::Write + Send>>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyHandle {
    /// Take the PTY output reader (child to parent).
    ///
    /// Returns `None` if already taken. The reader is typically handed to a
    /// [`PtyReader`](super::PtyReader) background thread.
    pub fn take_reader(&mut self) -> Option<Box<dyn io::Read + Send>> {
        self.reader.take()
    }

    /// Take the PTY input writer (parent to child).
    ///
    /// Returns `None` if already taken. The writer is typically owned by the
    /// input handler or notifier that forwards keyboard input.
    pub fn take_writer(&mut self) -> Option<Box<dyn io::Write + Send>> {
        self.writer.take()
    }

    /// Resize the PTY to new dimensions.
    pub fn resize(&self, rows: u16, cols: u16) -> io::Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| io::Error::other(e.to_string()))
    }

    /// Get the child process ID, if available.
    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Kill the child process.
    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    /// Wait for the child process to exit (blocking).
    pub fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        self.child.wait()
    }

}

/// Spawn a PTY with the configured shell and environment.
///
/// Creates a platform-native PTY pair, spawns the shell as a child process,
/// and returns a handle with reader, writer, and child management methods.
pub fn spawn_pty(config: &PtyConfig) -> io::Result<PtyHandle> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| io::Error::other(e.to_string()))?;

    let cmd = build_command(config);

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| io::Error::other(e.to_string()))?;

    // Drop the slave side so the reader detects EOF when child exits.
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| io::Error::other(e.to_string()))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| io::Error::other(e.to_string()))?;

    Ok(PtyHandle {
        reader: Some(reader),
        writer: Some(writer),
        master: pair.master,
        child,
    })
}

/// Build a `CommandBuilder` with shell detection and environment variables.
pub(crate) fn build_command(config: &PtyConfig) -> CommandBuilder {
    let shell = config
        .shell
        .as_deref()
        .unwrap_or_else(|| default_shell());

    let mut cmd = CommandBuilder::new(shell);

    if let Some(ref dir) = config.working_dir {
        cmd.cwd(dir);
    }

    // Terminal identification variables.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "oriterm");

    // User-provided overrides.
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    cmd
}

/// Returns the default shell for the current platform.
///
/// On Windows, returns `cmd.exe`. On Unix, reads the `SHELL` environment
/// variable and falls back to `/bin/sh`.
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
