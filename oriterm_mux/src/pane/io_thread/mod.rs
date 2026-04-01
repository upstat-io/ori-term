//! Terminal IO thread — owns `Term<T>` and processes commands + PTY bytes.
//!
//! In this initial scaffold, the thread drains channels without processing.
//! Section 02 adds `Term` ownership and VTE parsing.
//! Section 03 adds snapshot production.

mod commands;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::{fmt, io};

use crossbeam_channel::{Receiver, Sender};

pub use commands::PaneIoCommand;

/// Terminal IO thread — processes commands and PTY bytes.
///
/// In this scaffold phase, the thread drains both channels without acting
/// on the data. Section 02 adds `Term` ownership and VTE parsing;
/// section 03 adds snapshot production.
pub struct PaneIoThread {
    /// Receives commands from the main thread.
    cmd_rx: Receiver<PaneIoCommand>,
    /// Receives raw PTY bytes from the reader thread.
    byte_rx: Receiver<Vec<u8>>,
    /// Shutdown flag shared with reader/writer threads.
    shutdown: Arc<AtomicBool>,
    /// Wakeup callback — signals the main thread that new state is available.
    #[allow(dead_code, reason = "called in section 03 after snapshot production")]
    wakeup: Arc<dyn Fn() + Send + Sync>,
}

impl PaneIoThread {
    /// Run the IO thread message loop.
    ///
    /// Drains commands first (priority), then processes one batch of PTY
    /// bytes. Blocks via `crossbeam_channel::select!` when both channels
    /// are empty. Exits on `Shutdown` command or channel disconnect.
    pub fn run(mut self) {
        loop {
            // 1. Drain all pending commands (priority over bytes).
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                if matches!(cmd, PaneIoCommand::Shutdown) {
                    self.shutdown.store(true, Ordering::Release);
                    return;
                }
                self.handle_command(cmd);
            }

            // 2. Block on either channel when idle.
            crossbeam_channel::select! {
                recv(self.cmd_rx) -> msg => match msg {
                    Ok(PaneIoCommand::Shutdown) => {
                        self.shutdown.store(true, Ordering::Release);
                        return;
                    }
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

    /// Spawn the IO thread.
    pub fn spawn(self) -> io::Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("terminal-io".into())
            .spawn(move || self.run())
    }

    #[allow(
        clippy::needless_pass_by_ref_mut,
        clippy::needless_pass_by_value,
        clippy::unused_self,
        reason = "placeholder — &mut self and owned cmd used in sections 02-06"
    )]
    fn handle_command(&mut self, cmd: PaneIoCommand) {
        log::trace!("IO thread: command {cmd:?}");
    }

    #[allow(
        clippy::needless_pass_by_ref_mut,
        clippy::unused_self,
        reason = "placeholder — &mut self used in section 02"
    )]
    fn handle_bytes(&mut self, _bytes: &[u8]) {
        // Filled in section 02.
    }
}

impl fmt::Debug for PaneIoThread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PaneIoThread")
            .field("shutdown", &self.shutdown.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// Main-thread handle to a Terminal IO thread.
///
/// Provides non-blocking command sending and byte forwarding. The IO thread
/// processes commands in order and (in later sections) produces snapshots.
/// Created by [`PaneIoThread::new_with_handle()`].
pub struct PaneIoHandle {
    /// Send commands to the IO thread.
    cmd_tx: Sender<PaneIoCommand>,
    /// Send raw PTY bytes to the IO thread (cloned for the reader thread).
    byte_tx: Sender<Vec<u8>>,
    /// IO thread join handle (taken on shutdown).
    join: Option<JoinHandle<()>>,
}

/// Send a command to the IO thread (non-blocking).
///
/// Logs a warning if the IO thread has already exited.
impl PaneIoHandle {
    /// Send a command to the IO thread.
    pub fn send_command(&self, cmd: PaneIoCommand) {
        if let Err(e) = self.cmd_tx.send(cmd) {
            log::warn!("IO thread command send failed: {e}");
        }
    }

    /// Clone the byte sender for the PTY reader thread.
    pub fn byte_sender(&self) -> Sender<Vec<u8>> {
        self.byte_tx.clone()
    }

    /// Shut down the IO thread and wait for it to exit.
    pub fn shutdown(&mut self) {
        let _ = self.cmd_tx.send(PaneIoCommand::Shutdown);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }

    /// Set the join handle after spawning.
    pub fn set_join(&mut self, handle: JoinHandle<()>) {
        self.join = Some(handle);
    }
}

impl Drop for PaneIoHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl fmt::Debug for PaneIoHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PaneIoHandle")
            .field("alive", &self.join.is_some())
            .finish_non_exhaustive()
    }
}

/// Create the IO thread and its main-thread handle.
///
/// Channels are created here and split between the two sides.
/// The caller spawns the thread via [`PaneIoThread::spawn()`], then
/// sets the join handle on the returned `PaneIoHandle`.
pub fn new_with_handle(
    shutdown: Arc<AtomicBool>,
    wakeup: Arc<dyn Fn() + Send + Sync>,
) -> (PaneIoThread, PaneIoHandle) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
    let thread = PaneIoThread {
        cmd_rx,
        byte_rx,
        shutdown,
        wakeup,
    };
    let handle = PaneIoHandle {
        cmd_tx,
        byte_tx,
        join: None,
    };
    (thread, handle)
}

#[cfg(test)]
mod tests;
