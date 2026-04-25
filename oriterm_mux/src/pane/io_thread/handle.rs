//! Main-thread handle to a Terminal IO thread.
//!
//! `PaneIoHandle` provides non-blocking command sending and byte
//! forwarding. `IoThreadConfig` bundles the configuration for creating
//! an IO thread. `new_with_handle()` creates both sides of the channel
//! pair. Extracted from `mod.rs` to keep file sizes under 500 lines.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};

use oriterm_core::color::Rgb;
use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{AlreadyFulfilled, ResponseToken};
use oriterm_core::{RenderableContent, Term};

use super::snapshot::SnapshotDoubleBuffer;
use super::{PaneIoCommand, PaneIoThread};
use crate::PaneId;
use crate::mux_event::MuxEvent;
use crate::pty::PtyControl;
use crate::pty::adopt::AdoptedSignal;
use crate::pty::spawn::ExitStatus;

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
    /// Bounded-size-1 wake channel for response-token fulfillment.
    ///
    /// When the main thread calls
    /// [`fulfill_clipboard_load`](Self::fulfill_clipboard_load) or
    /// [`fulfill_color_query`](Self::fulfill_color_query), the helper
    /// signals this channel after writing the value so the IO thread's
    /// `select!` wakes within one iteration without requiring unrelated
    /// PTY or command activity.
    pub(crate) response_wake_tx: Sender<()>,
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
    pub fn double_buffer(&self) -> &SnapshotDoubleBuffer {
        &self.double_buffer
    }

    /// Fulfill a clipboard-load `ResponseToken` and wake the IO thread.
    ///
    /// Returns `Err(AlreadyFulfilled)` if a previous fulfill already
    /// succeeded (routing-bug detection per the single-assignment
    /// contract on [`ResponseToken::fulfill`]).
    pub fn fulfill_clipboard_load(
        &self,
        token: &ResponseToken<String>,
        text: String,
    ) -> Result<(), AlreadyFulfilled> {
        token.fulfill(text)?;
        // Bounded(1) channel: `try_send` coalesces multiple fulfills
        // between wakes into one signal. If the channel is already
        // full, the prior pending wake is enough.
        let _ = self.response_wake_tx.try_send(());
        Ok(())
    }

    /// Fulfill a color-query `ResponseToken` and wake the IO thread.
    pub fn fulfill_color_query(
        &self,
        token: &ResponseToken<Rgb>,
        color: Rgb,
    ) -> Result<(), AlreadyFulfilled> {
        token.fulfill(color)?;
        let _ = self.response_wake_tx.try_send(());
        Ok(())
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
    /// Pane identity — used by the effect router to tag outbound
    /// `MuxEvent`s with `pane_id`.
    pub pane_id: PaneId,
    /// Output channel for `MuxEvent`s — the effect router sends through
    /// this to reach the main thread's mux event pump.
    pub mux_tx: mpsc::Sender<MuxEvent>,
    /// Child-process exit channel.
    ///
    /// Delivered by the watcher thread spawned in [`crate::pty::spawn_pty`]
    /// (or the adopted-pane equivalent). When the PTY reader observes
    /// EOF, the IO thread waits briefly on this channel for the real
    /// exit code before emitting `HostEffect::ChildExit`.
    pub child_exit_rx: Receiver<ExitStatus>,
    /// Lock-free mode cache (shared with main thread).
    pub mode_cache: Arc<AtomicU64>,
    /// Shutdown flag (shared with reader/writer threads).
    pub shutdown: Arc<AtomicBool>,
    /// Wakeup callback — signals the main thread on new state.
    pub wakeup: Arc<dyn Fn() + Send + Sync>,
    /// Grid dirty flag — set by the IO thread when VTE parsing produces
    /// new state, cleared after snapshot publication.
    pub grid_dirty: Arc<AtomicBool>,
    /// PTY control handle for resize (SIGWINCH). `None` in tests and
    /// for adopted (default-terminal handoff) panes.
    pub pty_control: Option<PtyControl>,
    /// Adopted conhost signal handle for resize on Windows Default
    /// Terminal handoff panes. `None` for spawned panes (which use
    /// `pty_control`) and tests.
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
pub fn new_with_handle<S: EffectSink + 'static>(
    config: IoThreadConfig<S>,
) -> (PaneIoThread<S>, PaneIoHandle) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (byte_tx, byte_rx) = crossbeam_channel::unbounded();
    // Bounded(1) wake channel: `try_send` coalesces N fulfills between
    // wakes into a single signal. One wake drains ALL ready tokens on
    // the IO thread side (see `response_poll::poll_pending_responses`).
    let (response_wake_tx, response_wake_rx) = crossbeam_channel::bounded::<()>(1);
    let double_buffer = SnapshotDoubleBuffer::new();
    let thread = PaneIoThread {
        terminal: config.terminal,
        pane_id: config.pane_id,
        mux_tx: config.mux_tx,
        child_exit_rx: config.child_exit_rx,
        pending_child_exit: None,
        response_wake_rx,
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
        effects_buf: Vec::new(),
        last_animation_deadline: None,
    };
    let handle = PaneIoHandle {
        cmd_tx,
        byte_tx,
        join: None,
        double_buffer,
        response_wake_tx,
    };
    (thread, handle)
}
