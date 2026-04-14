//! Main-thread handle to a Terminal IO thread.
//!
//! `PaneIoHandle` provides non-blocking command sending and byte
//! forwarding. `IoThreadConfig` bundles the configuration for creating
//! an IO thread. `new_with_handle()` creates both sides of the channel
//! pair. Extracted from `mod.rs` to keep file sizes under 500 lines.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::thread::JoinHandle;

use crossbeam_channel::Sender;

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::{RenderableContent, Term};

use super::snapshot::SnapshotDoubleBuffer;
use super::{PaneIoCommand, PaneIoThread};
use crate::pty::PtyControl;
use crate::pty::adopt::AdoptedSignal;

/// Main-thread handle to a Terminal IO thread.
///
/// Provides non-blocking command sending and byte forwarding. The IO thread
/// processes commands in order and produces snapshots. The main thread reads
/// the latest snapshot via the shared [`SnapshotDoubleBuffer`].
/// Created by [`new_with_handle()`].
pub struct PaneIoHandle {
    /// Send commands to the IO thread.
    pub(crate) cmd_tx: Sender<PaneIoCommand>,
    /// Send raw PTY bytes to the IO thread (cloned for the reader thread).
    pub(crate) byte_tx: Sender<Vec<u8>>,
    /// IO thread join handle (taken on shutdown).
    pub(crate) join: Option<JoinHandle<()>>,
    /// Shared double buffer — main thread reads snapshots from here.
    pub(crate) double_buffer: SnapshotDoubleBuffer,
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
pub struct IoThreadConfig<S: EffectSink + 'static> {
    /// The terminal state machine — transferred to the IO thread.
    pub terminal: Term<S>,
    /// Lock-free mode cache (shared with main thread).
    pub mode_cache: Arc<AtomicU32>,
    /// Shutdown flag (shared with reader/writer threads).
    pub shutdown: Arc<AtomicBool>,
    /// Wakeup callback — signals the main thread on new state.
    pub wakeup: Arc<dyn Fn() + Send + Sync>,
    /// Grid dirty flag (shared with `IoThreadEventProxy`).
    pub grid_dirty: Arc<AtomicBool>,
    /// PTY control handle for resize (SIGWINCH). `None` in tests and
    /// for adopted (default-terminal handoff) panes.
    pub pty_control: Option<PtyControl>,
    /// Adopted conhost signal handle for resize on Windows Default
    /// Terminal handoff panes. `None` for spawned panes (which use
    /// `pty_control`) and tests. The IO thread's `process_resize`
    /// falls back to this when `pty_control` is `None`.
    pub adopted_signal: Option<AdoptedSignal>,
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
pub fn new_with_handle<S: EffectSink + 'static>(
    config: IoThreadConfig<S>,
) -> (PaneIoThread<S>, PaneIoHandle) {
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
        adopted_signal: config.adopted_signal,
        last_pty_size: (config.initial_rows as u32) << 16 | config.initial_cols as u32,
        search: None,
        selection_dirty: config.selection_dirty,
        pending_responses: Vec::new(),
    };
    let handle = PaneIoHandle {
        cmd_tx,
        byte_tx,
        join: None,
        double_buffer,
    };
    (thread, handle)
}
