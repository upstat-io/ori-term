//! Main-thread handle to a Terminal IO thread.
//!
//! `PaneIoHandle` provides non-blocking command sending and byte
//! forwarding. `IoThreadConfig` bundles the configuration for creating
//! an IO thread. `new_with_handle()` creates both sides of the channel
//! pair. Extracted from `mod.rs` to keep file sizes under 500 lines.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
use crate::pty::reader::BYTE_CHANNEL_CAPACITY;
use crate::pty::spawn::ExitStatus;

/// Per-pane memory budget for the IO thread's command channel heap.
///
/// Sized to absorb realistic command bursts (mouse wheel, keyboard
/// repeat) without blocking the main thread, while keeping per-pane
/// command-queue heap bounded. Largest command is `SetTheme(Theme,
/// Box<Palette>)` carrying a ~1.6 KiB heap allocation; most commands
/// are ≤32 bytes. 512 KiB / ~2 KiB worst-case = 256 commands of
/// headroom.
const CMD_CHANNEL_MEMORY_BUDGET: usize = 512 * 1024;
const MAX_COMMAND_SIZE_HINT: usize = 2 * 1024;
/// Bounded `cmd_tx` capacity. Cross-module visible so saturation tests
/// can reference the saturation point.
pub(crate) const CMD_CHANNEL_CAPACITY: usize = CMD_CHANNEL_MEMORY_BUDGET / MAX_COMMAND_SIZE_HINT;

const _: () = assert!(
    CMD_CHANNEL_CAPACITY >= 1,
    "CMD_CHANNEL_MEMORY_BUDGET too small relative to MAX_COMMAND_SIZE_HINT",
);

/// Tag bit for a valid pending resize. Setting this bit lets the slot
/// encode any `(rows, cols) ∈ [0, u16::MAX]²` unambiguously, with `0`
/// reserved as the sentinel for "no pending resize".
///
/// Bit layout:
/// - bit 48 = valid tag
/// - bits 32..48 = rows
/// - bits 0..16 = cols
pub(super) const PENDING_RESIZE_VALID: u64 = 1 << 48;
/// Sentinel meaning "no pending resize". The valid bit is clear.
pub(super) const PENDING_RESIZE_NONE: u64 = 0;

/// Encode a pending resize for atomic storage.
#[inline]
pub(super) fn pack_pending_resize(rows: u16, cols: u16) -> u64 {
    PENDING_RESIZE_VALID | ((rows as u64) << 32) | (cols as u64)
}

/// Decode a pending resize. Returns `Some((rows, cols))` when the slot
/// carries a valid pending resize, `None` when the slot is the
/// `PENDING_RESIZE_NONE` sentinel.
#[inline]
pub(super) fn unpack_pending_resize(packed: u64) -> Option<(u16, u16)> {
    if packed & PENDING_RESIZE_VALID == 0 {
        return None;
    }
    let rows = ((packed >> 32) & 0xFFFF) as u16;
    let cols = (packed & 0xFFFF) as u16;
    Some((rows, cols))
}

/// Main-thread handle to a Terminal IO thread.
///
/// Provides non-blocking command sending and byte forwarding. The IO thread
/// processes commands in order and produces snapshots. The main thread reads
/// the latest snapshot via the shared [`SnapshotDoubleBuffer`].
/// Created by [`new_with_handle()`].
pub struct PaneIoHandle {
    /// Send commands to the IO thread.
    ///
    /// Bounded at [`CMD_CHANNEL_CAPACITY`]; senders use `try_send` so
    /// the main thread never blocks. Resize is routed through
    /// [`Self::send_resize`] (atomic slot + wake) and never traverses
    /// this channel.
    pub(crate) cmd_tx: Sender<PaneIoCommand>,
    /// Send raw PTY bytes to the IO thread (cloned for the reader thread).
    pub(crate) byte_tx: Sender<Vec<u8>>,
    /// IO thread join handle (taken on shutdown).
    pub(crate) join: Option<JoinHandle<()>>,
    /// Shared double buffer — main thread reads snapshots from here.
    pub(crate) double_buffer: SnapshotDoubleBuffer,
    /// Bounded(1) wake channel for IO thread state changes.
    ///
    /// Used by [`Self::fulfill_clipboard_load`],
    /// [`Self::fulfill_color_query`], [`Self::send_resize`], the
    /// [`Self::send_command`] Shutdown special-case, and [`Self::shutdown`]
    /// to wake the IO thread out of `select!` within one loop iteration.
    /// `try_send` coalesces multiple wakes between iterations into a
    /// single signal — one wake drains all ready state on the IO thread
    /// side.
    pub(crate) io_wake_tx: Sender<()>,
    /// Pending resize slot — last-writer-wins coalescing.
    ///
    /// Encoded via [`pack_pending_resize`]. Stored by
    /// [`Self::send_resize`], swapped out by the IO thread's
    /// `apply_pending_resize` helper. Replaces routing Resize through
    /// `cmd_tx` (which broke under saturation: see ).
    pub(crate) pending_resize: Arc<AtomicU64>,
    /// Durable shutdown flag, shared with the IO thread.
    ///
    /// Cloned from [`IoThreadConfig::shutdown`] in [`new_with_handle`].
    /// [`Self::shutdown`] and the [`Self::send_command`] Shutdown
    /// special-case set this directly so the IO thread observes
    /// shutdown even when `cmd_tx` and `io_wake_tx` are saturated.
    pub(crate) shutdown_flag: Arc<AtomicBool>,
    /// Test-only Drop sentinel — counter incremented after `shutdown()`
    /// returns inside the existing `Drop` impl. Production builds carry
    /// zero overhead — the field is `#[cfg(test)]`. Used by §03 Pin 4
    /// in `bug-tracker/plans//section-03-tdd-matrix.md`.
    #[cfg(test)]
    pub(crate) drop_counter: Option<Arc<std::sync::atomic::AtomicUsize>>,
}

impl PaneIoHandle {
    /// Send a command to the IO thread.
    ///
    /// Non-blocking. On a saturated `cmd_tx` the command is dropped
    /// with a `log::error!`; reply-bearing commands surface the failure
    /// to the caller via `recv_timeout` returning `Disconnected` when
    /// the dropped reply Sender is closed.
    ///
    /// `Shutdown` is special-cased: it sets the durable
    /// [`Self::shutdown_flag`] BEFORE the `try_send` so the IO thread
    /// observes shutdown even under saturation.
    pub fn send_command(&self, cmd: PaneIoCommand) {
        // Shutdown MUST always reach the IO thread (or set the durable
        // atomic flag) — never silently drop under saturation. Belt-
        // and-suspenders: set the flag first, then try_send the
        // command, then wake.
        //
        // `matches!(&cmd,...)` borrows so `cmd` flows into `try_send`
        // unmoved (PaneIoCommand is non-Copy).
        if matches!(&cmd, PaneIoCommand::Shutdown) {
            self.shutdown_flag.store(true, Ordering::Release);
            let _ = self.cmd_tx.try_send(cmd);
            let _ = self.io_wake_tx.try_send(());
            return;
        }
        if let Err(e) = self.cmd_tx.try_send(cmd) {
            log::error!("IO thread cmd_tx full or disconnected; dropped command: {e}");
        }
    }

    /// Request a resize via the atomic coalescing slot.
    ///
    /// Last-writer-wins: if multiple `send_resize` calls land before
    /// the IO thread drains the slot, only the latest dimensions are
    /// applied. Wakes the IO thread out of `select!` within one loop
    /// iteration via `io_wake_tx`. Never blocks; never drops state.
    pub fn send_resize(&self, rows: u16, cols: u16) {
        let packed = pack_pending_resize(rows, cols);
        self.pending_resize.store(packed, Ordering::Release);
        // Wake the IO thread so it observes the store within one loop
        // iteration. Bounded(1) — `try_send` coalesces N stores between
        // wakes into a single signal.
        let _ = self.io_wake_tx.try_send(());
    }

    /// Clone the byte sender for the PTY reader thread.
    pub fn byte_sender(&self) -> Sender<Vec<u8>> {
        self.byte_tx.clone()
    }

    /// Clone the IO-thread wake sender (for the PTY writer thread).
    ///
    /// The writer thread sets [`IoThreadConfig::shutdown`] directly
    /// when it exits; without a wake, an idle IO thread blocked in
    /// `select!` would not observe the shutdown until `byte_rx` EOF
    /// (timing not guaranteed by the writer-thread path) or the 24h
    /// `IDLE_WAKE_CEILING`. The writer thread `try_send`s a wake
    /// after setting the flag so the IO thread exits within one loop
    /// iteration. See `pty::spawn_pty_writer`.
    pub fn io_wake_sender(&self) -> Sender<()> {
        self.io_wake_tx.clone()
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
        let _ = self.io_wake_tx.try_send(());
        Ok(())
    }

    /// Fulfill a color-query `ResponseToken` and wake the IO thread.
    pub fn fulfill_color_query(
        &self,
        token: &ResponseToken<Rgb>,
        color: Rgb,
    ) -> Result<(), AlreadyFulfilled> {
        token.fulfill(color)?;
        let _ = self.io_wake_tx.try_send(());
        Ok(())
    }

    /// Shut down the IO thread and wait for it to exit.
    ///
    /// Sets [`Self::shutdown_flag`] FIRST so the IO thread observes
    /// shutdown even when `cmd_tx` is saturated. The follow-up
    /// `try_send(Shutdown)` is best-effort (faster path on a non-
    /// saturated channel; ignored if full). The wake guarantees the
    /// IO thread leaves `select!` within one iteration.
    pub fn shutdown(&mut self) {
        // Set the atomic flag FIRST — guarantees the IO thread observes
        // shutdown even if cmd_tx is saturated.
        self.shutdown_flag.store(true, Ordering::Release);
        // Try to also send a Shutdown command (faster path on a
        // non-saturated channel; ignored if full — the atomic flag is
        // the durable signal).
        let _ = self.cmd_tx.try_send(PaneIoCommand::Shutdown);
        // Wake the IO thread so it observes either signal within one
        // iteration.
        let _ = self.io_wake_tx.try_send(());
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }

    /// Set the join handle after spawning.
    pub fn set_join(&mut self, handle: JoinHandle<()>) {
        self.join = Some(handle);
    }

    /// Test-only — install a shared Drop counter.
    ///
    /// The counter is incremented inside the existing `Drop` impl after
    /// `shutdown()` returns, so observers can pin the "every Drop runs
    /// to completion" invariant without a flaky wall-clock budget.
    /// Used by §03 Pin 4 in `bug-tracker/plans//`.
    #[cfg(test)]
    pub(crate) fn set_drop_counter(&mut self, counter: Arc<std::sync::atomic::AtomicUsize>) {
        self.drop_counter = Some(counter);
    }
}

impl Drop for PaneIoHandle {
    fn drop(&mut self) {
        self.shutdown();
        #[cfg(test)]
        if let Some(ref counter) = self.drop_counter {
            counter.fetch_add(1, Ordering::Release);
        }
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
    // Bounded cmd channel — caps the per-pane command queue heap at
    // CMD_CHANNEL_MEMORY_BUDGET, propagating saturation back to the
    // sender via `try_send` returning `Err(TrySendError::Full(_))`.
    // Resize commands are routed through the atomic `pending_resize`
    // slot (see PaneIoHandle::send_resize) and never traverse this
    // channel. Regression:.
    let (cmd_tx, cmd_rx) = crossbeam_channel::bounded(CMD_CHANNEL_CAPACITY);
    // Bounded byte channel — caps the per-pane reader→IO queue heap at
    // BYTE_CHANNEL_MEMORY_BUDGET, propagating back-pressure through the
    // kernel PTY buffer to the child process. Regression:.
    let (byte_tx, byte_rx) = crossbeam_channel::bounded(BYTE_CHANNEL_CAPACITY);
    // Bounded(1) wake channel: `try_send` coalesces N wakes between
    // iterations into a single signal. One wake drains ALL ready state
    // on the IO thread side (response fulfillment, pending resize, or
    // any other shared-state signal).
    let (io_wake_tx, io_wake_rx) = crossbeam_channel::bounded::<()>(1);
    let pending_resize = Arc::new(AtomicU64::new(PENDING_RESIZE_NONE));
    let shutdown_flag = Arc::clone(&config.shutdown);
    let double_buffer = SnapshotDoubleBuffer::new();
    let thread = PaneIoThread {
        terminal: config.terminal,
        pane_id: config.pane_id,
        mux_tx: config.mux_tx,
        child_exit_rx: config.child_exit_rx,
        pending_child_exit: None,
        io_wake_rx,
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
        pending_resize: Arc::clone(&pending_resize),
        #[cfg(test)]
        shrink_call_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let handle = PaneIoHandle {
        cmd_tx,
        byte_tx,
        join: None,
        double_buffer,
        io_wake_tx,
        pending_resize,
        shutdown_flag,
        #[cfg(test)]
        drop_counter: None,
    };
    (thread, handle)
}
