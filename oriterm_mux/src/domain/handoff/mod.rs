//! Adopt path — constructs a `Pane` from a Windows console handoff.
//!
//! When the Windows console host hands a session off to `ori_term` via
//! the `ITerminalHandoff3` COM interface (Section 03.9 Phase 3), the
//! COM implementation builds an [`AdoptedPtyHandle`] from the
//! duplicated handles and calls [`adopt_pane`] to wire it into the
//! standard mux pane pipeline. The result is a [`Pane`] that behaves
//! identically to a `LocalDomain::spawn_pane` result, except no child
//! process was spawned by `ori_term` — the console host owns the child
//! lifecycle.
//!
//! This module is cross-platform: [`adopt_pane`] accepts trait objects
//! and an [`AdoptedSignal`] that compiles to a stub on non-Windows. The
//! Windows-specific COM and signal-pipe wiring lives in
//! `oriterm/src/platform/default_terminal/`.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc;

use oriterm_core::{Term, Theme};

use crate::id::{DomainId, PaneId};
use crate::mux_event::MuxEvent;
use crate::pane::io_thread::event_proxy::IoThreadEventProxy;
use crate::pane::{self, Pane, PaneNotifier, PaneParts};
use crate::pty::{AdoptedPtyHandle, PtyReader, spawn_pty_writer};

/// Configuration for [`adopt_pane`].
///
/// Bundles the per-pane parameters so the function signature stays under
/// the hygiene rule's parameter limit.
pub struct AdoptConfig {
    /// New pane identifier (allocated by the mux).
    pub pane_id: PaneId,
    /// Domain that originated the handoff (typically a future
    /// `WindowsHandoffDomain` once Phase 3 wires it; tests pass a
    /// synthetic id).
    pub domain_id: DomainId,
    /// Adopted PTY handle — `adopt_pane` takes the reader/writer out of
    /// it before boxing the remainder as `Box<dyn PtyLifecycle + Send>`.
    pub adopted: AdoptedPtyHandle,
    /// Initial terminal rows.
    pub rows: u16,
    /// Initial terminal columns.
    pub cols: u16,
    /// Scrollback buffer size.
    pub scrollback: usize,
    /// Color theme.
    pub theme: Theme,
}

/// Construct a [`Pane`] from a Windows console handoff.
///
/// Mirrors `LocalDomain::spawn_pane` steps 3-8 (no `spawn_pty` step):
///
/// 1. Take reader and writer from the [`AdoptedPtyHandle`] (signal
///    stays with the handle and is dropped when the pane drops).
/// 2. Set up shared atomics (`mode_cache`, `shutdown`, dirty flags,
///    write-stall flag).
/// 3. Build the IO thread's `Term` with an [`IoThreadEventProxy`] that
///    routes metadata events to the mux channel.
/// 4. Spawn the writer thread.
/// 5. Spawn the Terminal IO thread (`pty_control = None` because adopted
///    PTYs have no `MasterPty`).
/// 6. Spawn the PTY reader thread with an exit signal that wakes
///    [`AdoptedPtyHandle::wait`] on EOF.
/// 7. Box the now-emptied [`AdoptedPtyHandle`] as `Box<dyn PtyLifecycle>`
///    and assemble the `Pane` via `Pane::from_parts`.
pub fn adopt_pane(
    mut config: AdoptConfig,
    mux_tx: &mpsc::Sender<MuxEvent>,
    wakeup: &Arc<dyn Fn() + Send + Sync>,
) -> io::Result<Pane> {
    // 1. Take reader and writer out of the adopted handle. The signal
    //    stays inside the handle so its `Drop` closes the conhost
    //    handles when the pane is dropped.
    let reader = config
        .adopted
        .take_reader()
        .ok_or_else(|| io::Error::other("AdoptedPtyHandle reader already taken"))?;
    let writer = config
        .adopted
        .take_writer()
        .ok_or_else(|| io::Error::other("AdoptedPtyHandle writer already taken"))?;
    let exit_signal = config.adopted.clone_exit_signal();

    // 2. Shared atomics — same shape as LocalDomain::spawn_pane.
    let io_grid_dirty = Arc::new(AtomicBool::new(false));
    let mode_cache = Arc::new(AtomicU32::new(oriterm_core::TermMode::default().bits()));
    let shutdown = Arc::new(AtomicBool::new(false));
    let io_selection_dirty = Arc::new(AtomicBool::new(false));
    let write_stalled = Arc::new(AtomicBool::new(false));

    // 3. Build the IO thread's Term with an unsuppressed event proxy
    //    (post Section 07 — adopted panes participate in the same mux
    //    event flow as spawned panes).
    let io_event_proxy = IoThreadEventProxy::new(
        Arc::clone(&io_grid_dirty),
        false,
        config.pane_id,
        mux_tx.clone(),
        Arc::clone(wakeup),
    );
    let io_term = Term::new(
        usize::from(config.rows),
        usize::from(config.cols),
        config.scrollback,
        config.theme,
        io_event_proxy,
    );

    // 4. Writer channel + writer thread.
    let (tx, rx) = mpsc::channel();
    let notifier = PaneNotifier::new(tx);
    let writer_thread = spawn_pty_writer(
        writer,
        rx,
        Arc::clone(&shutdown),
        Arc::clone(&write_stalled),
    )?;

    // 5. Terminal IO thread. `pty_control = None` because adopted PTYs
    //    have no portable_pty::MasterPty — resize for adopted panes will
    //    travel through the conhost signal pipe in a future phase
    //    (Phase 3 wires AdoptedSignal::resize through a new IO thread
    //    command).
    let (io_thread, mut io_handle) =
        pane::io_thread::new_with_handle(pane::io_thread::IoThreadConfig {
            terminal: io_term,
            mode_cache: Arc::clone(&mode_cache),
            shutdown: Arc::clone(&shutdown),
            wakeup: Arc::clone(wakeup),
            grid_dirty: io_grid_dirty,
            pty_control: None,
            initial_rows: config.rows,
            initial_cols: config.cols,
            selection_dirty: Arc::clone(&io_selection_dirty),
        });
    let byte_tx = io_handle.byte_sender();
    let io_join = io_thread.spawn()?;
    io_handle.set_join(io_join);

    // 6. PTY reader thread with exit signal — fires
    //    AdoptedPtyHandle::deliver_exit when the reader exits, waking
    //    Pane::Drop's pty.wait() call so adopted-pane shutdown does not
    //    deadlock.
    let pty_reader =
        PtyReader::new(reader, byte_tx, Arc::clone(&shutdown)).with_exit_signal(exit_signal);
    let reader_thread = pty_reader.spawn()?;

    // 7. Assemble the Pane. The adopted handle now owns only the signal
    //    handles, exit_signal, and client_pid — boxing it as
    //    `Box<dyn PtyLifecycle + Send>` lets Pane::from_parts read the
    //    client PID via trait dispatch and lets Pane::Drop call kill +
    //    wait through the trait.
    Ok(Pane::from_parts(PaneParts {
        id: config.pane_id,
        domain_id: config.domain_id,
        notifier,
        reader_thread,
        writer_thread,
        io_handle,
        pty: Box::new(config.adopted),
        mode_cache,
        io_selection_dirty,
        write_stalled,
        #[cfg(unix)]
        master_fd: None,
    }))
}

#[cfg(test)]
mod tests;
