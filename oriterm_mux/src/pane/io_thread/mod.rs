//! Terminal IO thread — owns `Term<T>` exclusively and processes VTE bytes.
//!
//! The IO thread receives raw PTY bytes from the reader thread via a channel,
//! parses them through both VTE processors, and maintains terminal state.
//! Commands from the main thread (resize, scroll, theme, etc.) are processed
//! between parse chunks to stay responsive under sustained output.
//!
//! Section 03 adds snapshot production; section 05+ adds command handling.

mod commands;
pub(crate) mod event_proxy;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::{self, JoinHandle};
use std::{fmt, io};

use crossbeam_channel::{Receiver, Sender};

use oriterm_core::{EventListener, Term};

pub use commands::PaneIoCommand;

use crate::shell_integration::interceptor::RawInterceptor;

/// Maximum bytes parsed before re-checking for commands.
///
/// Matches `PtyEventLoop::MAX_LOCKED_PARSE` (64 KB). A single 1 MB forwarded
/// read is sliced into chunks at this boundary so resize/copy commands stay
/// responsive under sustained output.
const MAX_PARSE_CHUNK: usize = 0x1_0000; // 64 KB

/// Terminal IO thread — owns `Term<T>` and processes commands + PTY bytes.
///
/// Generic over `T: EventListener` so the IO thread's `Term` can use
/// `IoThreadEventProxy` (suppresses metadata during dual-Term migration)
/// while the old path uses `MuxEventProxy`.
pub struct PaneIoThread<T: EventListener> {
    /// The terminal state machine — exclusively owned by this thread.
    terminal: Term<T>,
    /// Receives commands from the main thread.
    cmd_rx: Receiver<PaneIoCommand>,
    /// Receives raw PTY bytes from the reader thread.
    byte_rx: Receiver<Vec<u8>>,
    /// Shutdown flag shared with reader/writer threads.
    shutdown: Arc<AtomicBool>,
    /// Wakeup callback — signals the main thread that new state is available.
    #[allow(dead_code, reason = "called in section 03 after snapshot production")]
    wakeup: Arc<dyn Fn() + Send + Sync>,
    /// High-level VTE parser (routes to `Handler` trait methods).
    processor: vte::ansi::Processor,
    /// Raw VTE parser for shell integration sequences (OSC 7, 133, etc.).
    raw_parser: vte::Parser,
    /// Lock-free mode cache (updated after parsing, read by main thread).
    mode_cache: Arc<AtomicU32>,
}

impl<T: EventListener> PaneIoThread<T> {
    /// Run the IO thread message loop.
    ///
    /// Priority: drain commands first, then process pending bytes with
    /// bounded chunking. Blocks via `crossbeam_channel::select!` when both
    /// channels are empty. Exits on `Shutdown` command or channel disconnect.
    pub fn run(mut self) {
        loop {
            // 1. Drain all pending commands (priority over bytes).
            self.drain_commands();
            if self.shutdown.load(Ordering::Acquire) {
                return;
            }

            // 2. Process available bytes (non-blocking drain with chunking).
            self.process_pending_bytes();

            // TODO (section 03): produce snapshot + send wakeup here.

            // 3. Block on either channel when idle.
            crossbeam_channel::select! {
                recv(self.cmd_rx) -> msg => match msg {
                    Ok(PaneIoCommand::Shutdown) => {
                        self.shutdown.store(true, Ordering::Release);
                        return;
                    }
                    Ok(cmd) => self.handle_command(cmd),
                    Err(_) => return,
                },
                recv(self.byte_rx) -> msg => match msg {
                    Ok(bytes) => self.handle_bytes(&bytes),
                    Err(_) => return,
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

    /// Drain all pending commands from the command channel.
    fn drain_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            if matches!(cmd, PaneIoCommand::Shutdown) {
                self.shutdown.store(true, Ordering::Release);
                return;
            }
            self.handle_command(cmd);
        }
    }

    /// Process all pending byte messages with bounded chunking.
    ///
    /// Each byte message is sliced into [`MAX_PARSE_CHUNK`]-sized pieces.
    /// Between chunks, commands are drained so resize/scroll stay responsive
    /// during sustained output.
    fn process_pending_bytes(&mut self) {
        while let Ok(bytes) = self.byte_rx.try_recv() {
            let mut offset = 0;
            while offset < bytes.len() {
                let end = (offset + MAX_PARSE_CHUNK).min(bytes.len());
                self.handle_bytes(&bytes[offset..end]);
                offset = end;
                // Re-check for priority commands between chunks.
                self.drain_commands();
                if self.shutdown.load(Ordering::Acquire) {
                    return;
                }
            }
        }
    }

    /// Parse a chunk of PTY output through both VTE parsers.
    ///
    /// Adapted from `PtyEventLoop::parse_chunk()` — runs the raw interceptor
    /// for shell integration, then the high-level processor, then deferred
    /// prompt marking and marker pruning.
    fn handle_bytes(&mut self, bytes: &[u8]) {
        let evicted_before = self.terminal.grid().total_evicted();

        // 1. Raw interceptor for shell integration (OSC 7, 133, etc.).
        {
            let mut interceptor = RawInterceptor::new(&mut self.terminal);
            self.raw_parser.advance(&mut interceptor, bytes);
        }

        // 2. High-level VTE processor.
        self.processor.advance(&mut self.terminal, bytes);

        // 3. Deferred prompt marking.
        if self.terminal.prompt_mark_pending() {
            self.terminal.mark_prompt_row();
        }
        if self.terminal.command_start_mark_pending() {
            self.terminal.mark_command_start_row();
        }
        if self.terminal.output_start_mark_pending() {
            self.terminal.mark_output_start_row();
        }

        // 4. Prune prompt markers invalidated by scrollback eviction.
        let newly_evicted = self.terminal.grid().total_evicted() - evicted_before;
        if newly_evicted > 0 {
            self.terminal.prune_prompt_markers(newly_evicted);
        }

        // 5. Update mode cache for lock-free queries from main thread.
        self.mode_cache
            .store(self.terminal.mode().bits(), Ordering::Release);
    }

    #[allow(
        clippy::needless_pass_by_ref_mut,
        clippy::needless_pass_by_value,
        clippy::unused_self,
        reason = "placeholder — &mut self and owned cmd used in sections 05-06"
    )]
    fn handle_command(&mut self, cmd: PaneIoCommand) {
        log::trace!("IO thread: command {cmd:?}");
    }
}

impl<T: EventListener> fmt::Debug for PaneIoThread<T> {
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
/// Created by [`new_with_handle()`].
pub struct PaneIoHandle {
    /// Send commands to the IO thread.
    cmd_tx: Sender<PaneIoCommand>,
    /// Send raw PTY bytes to the IO thread (cloned for the reader thread).
    byte_tx: Sender<Vec<u8>>,
    /// IO thread join handle (taken on shutdown).
    join: Option<JoinHandle<()>>,
}

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
pub fn new_with_handle<T: EventListener>(
    terminal: Term<T>,
    mode_cache: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    wakeup: Arc<dyn Fn() + Send + Sync>,
) -> (PaneIoThread<T>, PaneIoHandle) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
    let thread = PaneIoThread {
        terminal,
        cmd_rx,
        byte_rx,
        shutdown,
        wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache,
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
