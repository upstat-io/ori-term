//! Terminal IO thread — owns `Term<T>` exclusively and processes VTE bytes.
//!
//! The IO thread receives raw PTY bytes from the reader thread via a channel,
//! parses them through both VTE processors, and maintains terminal state.
//! Commands from the main thread (resize, scroll, theme, etc.) are processed
//! between parse chunks to stay responsive under sustained output.
//!
//! Section 03 adds snapshot production. Section 05 moves PTY resize to the IO
//! thread with command coalescing — the main thread never does grid reflow.

mod commands;
pub(crate) mod event_proxy;
mod handler;
pub(crate) mod snapshot;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::{self, JoinHandle};
use std::{fmt, io};

use crossbeam_channel::{Receiver, Sender};

use oriterm_core::{EventListener, RenderableContent, Term};

pub use commands::PaneIoCommand;
pub(crate) use snapshot::SnapshotDoubleBuffer;

use crate::pty::PtyControl;
use crate::shell_integration::interceptor::RawInterceptor;

/// Maximum bytes parsed before re-checking for commands.
///
/// Matches the old `PtyEventLoop::MAX_LOCKED_PARSE` (64 KB). A single 1 MB forwarded
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
    wakeup: Arc<dyn Fn() + Send + Sync>,
    /// High-level VTE parser (routes to `Handler` trait methods).
    processor: vte::ansi::Processor,
    /// Raw VTE parser for shell integration sequences (OSC 7, 133, etc.).
    raw_parser: vte::Parser,
    /// Lock-free mode cache (updated after parsing, read by main thread).
    mode_cache: Arc<AtomicU32>,
    /// Double buffer for transferring snapshots to the main thread.
    double_buffer: SnapshotDoubleBuffer,
    /// Work buffer for snapshot production — reused across frames.
    snapshot_buf: RenderableContent,
    /// Set by `IoThreadEventProxy` when VTE parsing sets grid dirty.
    /// Checked after snapshot production to decide whether to fire wakeup.
    grid_dirty: Arc<AtomicBool>,
    /// PTY control handle for resize (SIGWINCH). Owned by the IO thread so
    /// reflow and PTY resize happen atomically on the same thread.
    pty_control: Option<PtyControl>,
    /// Last PTY size sent, packed as `(rows << 16) | cols`. Guards against
    /// redundant syscalls (`ConPTY` `WINDOW_BUFFER_SIZE_EVENT` interference).
    last_pty_size: u32,
    /// Search state — owned by the IO thread so `set_query()` can read the
    /// grid directly without cross-thread locking.
    search: Option<oriterm_core::SearchState>,
    /// Shared selection-dirty flag. Set by the IO thread after VTE parsing
    /// when `Term::selection_dirty` becomes true. Read/cleared by the main
    /// thread in `check_selection_invalidation()`.
    selection_dirty: Arc<AtomicBool>,
}

impl<T: EventListener> PaneIoThread<T> {
    /// Run the IO thread message loop.
    ///
    /// Priority: drain commands first, then process pending bytes with
    /// bounded chunking. Blocks via `crossbeam_channel::select!` when both
    /// channels are empty. Exits on `Shutdown` command or channel disconnect.
    pub fn run(mut self) {
        // Produce an initial snapshot so the main thread has valid content
        // immediately — before any PTY output or commands arrive. Without
        // this, freshly spawned panes expose PaneSnapshot::default() until
        // the shell writes its first output.
        self.grid_dirty.store(true, Ordering::Release);
        self.produce_snapshot();

        loop {
            // 1. Drain all pending commands (priority over bytes).
            self.drain_commands();
            if self.shutdown.load(Ordering::Acquire) {
                // Flush any parsed-but-unpublished state before exiting.
                self.maybe_produce_snapshot();
                return;
            }

            // 2. Process available bytes (non-blocking drain with chunking).
            self.process_pending_bytes();

            // 3. Produce snapshot if state changed and sync output allows it.
            self.maybe_produce_snapshot();

            // 4. Block on either channel when idle.
            crossbeam_channel::select! {
                recv(self.cmd_rx) -> msg => match msg {
                    Ok(PaneIoCommand::Shutdown) => {
                        self.shutdown.store(true, Ordering::Release);
                        self.maybe_produce_snapshot();
                        return;
                    }
                    Ok(cmd) => self.handle_command(cmd),
                    Err(_) => return,
                },
                recv(self.byte_rx) -> msg => match msg {
                    Ok(bytes) => self.handle_bytes_chunked(&bytes),
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
    ///
    /// Resize commands are coalesced — only the last one in the batch is
    /// processed. During drag resize, dozens of `Resize` commands queue up;
    /// only the final dimensions matter. The coalesced resize is processed
    /// after all other commands so reflow sees the latest terminal state.
    fn drain_commands(&mut self) {
        let mut last_resize = None;
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                PaneIoCommand::Resize { rows, cols } => {
                    last_resize = Some((rows, cols));
                }
                PaneIoCommand::Shutdown => {
                    self.shutdown.store(true, Ordering::Release);
                    return;
                }
                other => self.handle_command(other),
            }
        }
        if let Some((rows, cols)) = last_resize {
            self.process_resize(rows, cols);
        }
    }

    /// Parse a byte buffer with bounded chunking.
    ///
    /// Slices `bytes` into [`MAX_PARSE_CHUNK`]-sized pieces. Between chunks,
    /// commands are drained and snapshots are published so that resize/scroll
    /// stay responsive and the main thread sees render progress even within a
    /// single large PTY read (up to 1 MB).
    fn handle_bytes_chunked(&mut self, bytes: &[u8]) {
        let mut offset = 0;
        while offset < bytes.len() {
            let end = (offset + MAX_PARSE_CHUNK).min(bytes.len());
            self.handle_bytes(&bytes[offset..end]);
            offset = end;
            self.drain_commands();
            if self.shutdown.load(Ordering::Acquire) {
                return;
            }
            // Publish intermediate snapshots between chunks so the main thread
            // sees progress even within a single large forwarded read.
            self.maybe_produce_snapshot();
        }
    }

    /// Process all pending byte messages with bounded chunking.
    ///
    /// Drains the byte channel and passes each message through
    /// [`handle_bytes_chunked()`](Self::handle_bytes_chunked). Snapshots
    /// are produced between messages so the main thread sees progress
    /// even during sustained flood output.
    fn process_pending_bytes(&mut self) {
        while let Ok(bytes) = self.byte_rx.try_recv() {
            self.handle_bytes_chunked(&bytes);
            if self.shutdown.load(Ordering::Acquire) {
                return;
            }
            // Produce snapshot between messages to keep the main thread fed.
            // Without this, flood output fills the queue faster than parsing
            // drains it, and `maybe_produce_snapshot()` never runs.
            self.maybe_produce_snapshot();
        }
    }

    /// Parse a chunk of PTY output through both VTE parsers.
    ///
    /// Runs the raw interceptor
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

        // 3b. Set grid_dirty after parsing — the VTE handler does not fire
        //     Event::Wakeup itself. The old reader thread did this explicitly
        //     after each parse chunk. Respects Mode 2026 (synchronized output):
        //     when the sync buffer is non-empty, skip the dirty flag so
        //     `maybe_produce_snapshot()` defers snapshot production.
        if self.processor.sync_bytes_count() == 0 {
            self.grid_dirty.store(true, Ordering::Release);
        }

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

        // 6. Propagate selection-dirty flag for lock-free main-thread reads.
        if self.terminal.is_selection_dirty() {
            self.terminal.clear_selection_dirty();
            self.selection_dirty.store(true, Ordering::Release);
        }
    }

    /// Produce a snapshot if state changed and synchronized output allows it.
    ///
    /// Respects Mode 2026 (synchronized output): when the sync buffer is
    /// non-empty, the application is building a frame — skip snapshot
    /// production to avoid exposing intermediate state.
    fn maybe_produce_snapshot(&mut self) {
        if self.processor.sync_bytes_count() > 0 {
            return;
        }
        if !self.grid_dirty.load(Ordering::Acquire) {
            return;
        }
        self.produce_snapshot();
    }

    /// Fill search state into the snapshot buffer from IO thread's `SearchState`.
    fn fill_search_snapshot(&mut self) {
        if let Some(ref search) = self.search {
            self.snapshot_buf.search_active = true;
            self.snapshot_buf.search_query.clear();
            self.snapshot_buf.search_query.push_str(search.query());
            self.snapshot_buf.search_matches.clear();
            self.snapshot_buf
                .search_matches
                .extend_from_slice(search.matches());
            let total = search.matches().len() as u32;
            self.snapshot_buf.search_total_matches = total;
            self.snapshot_buf.search_focused = if search.matches().is_empty() {
                None
            } else {
                Some(search.focused_index() as u32)
            };
        } else {
            self.snapshot_buf.search_active = false;
            self.snapshot_buf.search_query.clear();
            self.snapshot_buf.search_matches.clear();
            self.snapshot_buf.search_focused = None;
            self.snapshot_buf.search_total_matches = 0;
        }
    }

    /// Produce a rendering snapshot and publish it to the double buffer.
    ///
    /// Called after processing bytes or commands that change terminal state.
    /// Reuses buffer allocations via the double-buffer flip — after warmup,
    /// this is zero-allocation.
    fn produce_snapshot(&mut self) {
        self.terminal
            .renderable_content_into(&mut self.snapshot_buf);
        self.fill_search_snapshot();
        self.terminal.reset_damage();
        self.double_buffer.flip_swap(&mut self.snapshot_buf);

        // Clear grid_dirty and fire wakeup so the main thread renders.
        if self.grid_dirty.swap(false, Ordering::AcqRel) {
            (self.wakeup)();
        }
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
/// processes commands in order and produces snapshots. The main thread reads
/// the latest snapshot via the shared [`SnapshotDoubleBuffer`].
/// Created by [`new_with_handle()`].
pub struct PaneIoHandle {
    /// Send commands to the IO thread.
    cmd_tx: Sender<PaneIoCommand>,
    /// Send raw PTY bytes to the IO thread (cloned for the reader thread).
    byte_tx: Sender<Vec<u8>>,
    /// IO thread join handle (taken on shutdown).
    join: Option<JoinHandle<()>>,
    /// Shared double buffer — main thread reads snapshots from here.
    double_buffer: SnapshotDoubleBuffer,
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

    /// Access the shared snapshot double buffer.
    ///
    /// The main thread uses this to swap its old buffer for the latest
    /// snapshot produced by the IO thread.
    pub fn double_buffer(&self) -> &SnapshotDoubleBuffer {
        &self.double_buffer
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

/// Configuration for creating a Terminal IO thread.
pub struct IoThreadConfig<T: EventListener> {
    /// The terminal state machine — transferred to the IO thread.
    pub terminal: Term<T>,
    /// Lock-free mode cache (shared with main thread).
    pub mode_cache: Arc<AtomicU32>,
    /// Shutdown flag (shared with reader/writer threads).
    pub shutdown: Arc<AtomicBool>,
    /// Wakeup callback — signals the main thread on new state.
    pub wakeup: Arc<dyn Fn() + Send + Sync>,
    /// Grid dirty flag (shared with `IoThreadEventProxy`).
    pub grid_dirty: Arc<AtomicBool>,
    /// PTY control handle for resize (SIGWINCH). `None` in tests.
    pub pty_control: Option<PtyControl>,
    /// Initial PTY dimensions (rows, cols) — seeds the dedup guard so the
    /// first resize at spawn size skips the redundant syscall.
    pub initial_rows: u16,
    /// Initial PTY columns from spawn.
    pub initial_cols: u16,
    /// Shared selection-dirty flag (set by IO thread, read/cleared by main thread).
    pub selection_dirty: Arc<AtomicBool>,
}

/// Create the IO thread and its main-thread handle.
///
/// Channels and the shared double buffer are created here and split
/// between the two sides. The `grid_dirty` atomic is shared with
/// the IO thread's `IoThreadEventProxy` — the proxy sets it during
/// VTE parsing, the IO thread reads + clears it after snapshot
/// production.
///
/// The caller spawns the thread via [`PaneIoThread::spawn()`], then
/// sets the join handle on the returned `PaneIoHandle`.
pub fn new_with_handle<T: EventListener>(
    config: IoThreadConfig<T>,
) -> (PaneIoThread<T>, PaneIoHandle) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
    let double_buffer = SnapshotDoubleBuffer::new();
    let thread = PaneIoThread {
        terminal: config.terminal,
        cmd_rx,
        byte_rx,
        shutdown: config.shutdown,
        wakeup: config.wakeup,
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache: config.mode_cache,
        double_buffer: double_buffer.clone(),
        snapshot_buf: RenderableContent::default(),
        grid_dirty: config.grid_dirty,
        pty_control: config.pty_control,
        last_pty_size: (config.initial_rows as u32) << 16 | config.initial_cols as u32,
        search: None,
        selection_dirty: config.selection_dirty,
    };
    let handle = PaneIoHandle {
        cmd_tx,
        byte_tx,
        join: None,
        double_buffer,
    };
    (thread, handle)
}

#[cfg(test)]
mod tests;
