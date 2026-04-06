//! Cross-platform PTY abstraction.
//!
//! Provides PTY creation, shell spawning, a background reader thread, a
//! dedicated writer thread, and the message channel for main-thread → PTY
//! communication. Uses `portable-pty` for platform abstraction: `ConPTY`
//! on Windows, `openpty`/`forkpty` on Linux, POSIX PTY on macOS.

pub(crate) mod reader;
mod spawn;

use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(crate) use reader::PtyReader;
#[allow(
    unused_imports,
    reason = "returned by PtyHandle::wait/try_wait; callers need access"
)]
pub use spawn::ExitStatus;
pub(crate) use spawn::compute_wslenv;
pub use spawn::{PtyConfig, PtyControl, PtyHandle, spawn_pty};

/// Commands sent from the main thread to the PTY writer thread.
///
/// Delivered via `std::sync::mpsc::channel`. The sender is held by
/// [`PaneNotifier`](crate::pane::PaneNotifier), the receiver by the
/// writer thread spawned via [`spawn_pty_writer`].
#[derive(Debug)]
pub enum Msg {
    /// Raw bytes to write to the PTY (keyboard input, escape responses).
    ///
    /// Sent by `PaneNotifier::notify()` on the main thread, written
    /// immediately by the dedicated writer thread.
    Input(Vec<u8>),
    /// Gracefully stop both the writer and reader threads.
    Shutdown,
}

/// How long to wait for channel messages when the writer has no pending data.
const WRITER_RECV_TIMEOUT: Duration = Duration::from_millis(100);

/// Spawn a dedicated PTY writer thread.
///
/// Uses a write-stall detection flag (`write_stalled`) to signal the main
/// thread when a blocking `write()` is stuck (e.g., child doesn't read
/// stdin during output flooding). The main thread checks this flag when
/// the user presses Ctrl+C and sends SIGINT directly to the child process
/// group, bypassing the blocked writer.
///
/// Separating reads and writes onto different threads prevents a deadlock
/// during shell startup: the shell sends DA1 (device attributes query),
/// the VTE parser generates the response via `Event::PtyWrite`, and the
/// main thread enqueues it as `Msg::Input`. If the writer lived on the
/// reader thread, the response would be stuck behind a blocking `read()`
/// that never returns because the shell is waiting for the DA response.
pub fn spawn_pty_writer(
    mut writer: Box<dyn Write + Send>,
    rx: mpsc::Receiver<Msg>,
    shutdown: Arc<AtomicBool>,
    write_stalled: Arc<AtomicBool>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("pty-writer".into())
        .spawn(move || {
            pty_writer_loop(&mut *writer, &rx, &shutdown, &write_stalled);
            shutdown.store(true, Ordering::Release);
        })
}

/// Writer loop — coalesces queued input, detects write stalls.
///
/// Sets `write_stalled` before each potentially-blocking `write()` call
/// and clears it after. The main thread reads this flag to decide whether
/// to deliver Ctrl+C via direct signal rather than through the PTY pipe.
fn pty_writer_loop(
    writer: &mut dyn Write,
    rx: &mpsc::Receiver<Msg>,
    shutdown: &AtomicBool,
    write_stalled: &AtomicBool,
) {
    let mut pending = Vec::<u8>::new();
    let mut shutting_down = false;
    loop {
        // Drain all queued messages into the pending buffer.
        if drain_channel(rx, &mut pending) {
            shutting_down = true;
        }

        if pending.is_empty() {
            if shutting_down {
                return;
            }
            // Nothing to write — block on the channel with a timeout so we
            // can check for shutdown periodically.
            match rx.recv_timeout(WRITER_RECV_TIMEOUT) {
                Ok(Msg::Input(data)) => pending.extend_from_slice(&data),
                Ok(Msg::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if shutdown.load(Ordering::Acquire) {
                        return;
                    }
                    continue;
                }
            }
            // Drain any messages that arrived while we waited.
            if drain_channel(rx, &mut pending) {
                shutting_down = true;
            }
        }

        // Write all pending data. This may block if the kernel PTY buffer
        // is full (child not reading stdin). The stall flag lets the main
        // thread detect this and send SIGINT directly.
        write_stalled.store(true, Ordering::Release);
        let result = writer.write_all(&pending);
        write_stalled.store(false, Ordering::Release);

        match result {
            Ok(()) => {
                pending.clear();
                let _ = writer.flush();
            }
            Err(e) => {
                log::warn!("PTY write failed: {e}");
                return;
            }
        }

        if shutting_down {
            return;
        }
    }
}

/// Drain all immediately-available messages into `buf`.
///
/// Returns `true` if a `Shutdown` message was received.
fn drain_channel(rx: &mpsc::Receiver<Msg>, buf: &mut Vec<u8>) -> bool {
    while let Ok(msg) = rx.try_recv() {
        match msg {
            Msg::Input(data) => buf.extend_from_slice(&data),
            Msg::Shutdown => return true,
        }
    }
    false
}

#[cfg(test)]
mod tests;
