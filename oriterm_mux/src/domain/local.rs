//! Local domain — spawns shells on the local machine via `portable-pty`.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc;

use oriterm_core::effect::QueueingEffectSink;
use oriterm_core::{Term, Theme};

use crate::{DomainId, PaneId};

use super::{Domain, DomainState, SpawnConfig, SpawnWiring};

use crate::pane::io_thread;
use crate::pane::{Pane, PaneNotifier, PaneParts};
use crate::pty::{PtyConfig, PtyReader, spawn_pty, spawn_pty_writer};

/// Spawns shells on the local machine.
/// The simplest domain — creates a PTY via `portable-pty`, wires up
/// the Terminal IO thread as the sole terminal owner, and returns a
/// fully assembled `Pane`.
pub struct LocalDomain {
    /// Domain identity.
    id: DomainId,
    /// Lifecycle state.
    state: DomainState,
}

impl Domain for LocalDomain {
    fn id(&self) -> DomainId {
        self.id
    }

    #[allow(
        clippy::unnecessary_literal_bound,
        reason = "trait signature requires &str, literal 'local' is always valid"
    )]
    fn name(&self) -> &str {
        "local"
    }

    fn state(&self) -> DomainState {
        self.state
    }

    fn can_spawn(&self) -> bool {
        self.state == DomainState::Attached
    }
}

impl LocalDomain {
    /// Create a new local domain.
    pub fn new(id: DomainId) -> Self {
        Self {
            id,
            state: DomainState::Attached,
        }
    }

    /// Spawn a new pane with a live shell process.
    /// Creates the PTY, a single `Term` owned exclusively by the IO thread,
    /// a PTY byte reader, and a PTY writer thread. The IO thread handles all
    /// VTE parsing, commands, and snapshot production. The main thread
    /// communicates via channels only — no shared terminal lock.
    pub fn spawn_pane(
        &self,
        pane_id: PaneId,
        config: &SpawnConfig,
        theme: Theme,
        wiring: SpawnWiring<'_>,
    ) -> io::Result<Pane> {
        let SpawnWiring { mux_tx, wakeup } = wiring;
        // 1. Spawn PTY with the configured shell.
        let pty_config = PtyConfig {
            rows: config.rows,
            cols: config.cols,
            shell: config.shell.clone(),
            working_dir: config.cwd.clone(),
            env: config.env.clone(),
            shell_integration: config.shell_integration,
        };
        let (mut pty, child_exit_rx) = spawn_pty(&pty_config)?;

        // 2. Take handles before they're moved into threads.
        let reader = pty
            .take_reader()
            .ok_or_else(|| io::Error::other("PTY reader unavailable"))?;
        let writer = pty
            .take_writer()
            .ok_or_else(|| io::Error::other("PTY writer unavailable"))?;
        let control = pty
            .take_control()
            .ok_or_else(|| io::Error::other("PTY control unavailable"))?;

        // 2b. Dup the master fd for signal delivery (Unix).
        // `tcgetpgrp(master_fd)` needs a valid fd to the PTY master.
        // The control handle moves to the IO thread, so we dup it now.
        #[cfg(unix)]
        #[allow(
            unsafe_code,
            reason = "libc::dup + OwnedFd::from_raw_fd require unsafe"
        )]
        let master_fd = {
            use std::os::unix::io::FromRawFd;
            control.master_fd().map(|fd| {
                // SAFETY: dup() is a standard POSIX syscall. The source fd
                // is valid (obtained from MasterPty::as_raw_fd()). OwnedFd
                // takes ownership and closes on drop.
                let duped = unsafe { libc::dup(fd) };
                assert!(duped >= 0, "dup(master_fd) failed");
                unsafe { std::os::unix::io::OwnedFd::from_raw_fd(duped) }
            })
        };

        // 3. Set up shared atomics.
        let io_grid_dirty = Arc::new(AtomicBool::new(false));
        let mode_cache = Arc::new(AtomicU64::new(oriterm_core::TermMode::default().bits()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let io_selection_dirty = Arc::new(AtomicBool::new(false));

        // 4. Create the single Term with a QueueingEffectSink. The IO
        // thread drains the sink after every VTE parse chunk and
        // routes effects to MuxEvents via the effect router
        // (effect-cutover 01.1). This is the only event path —
        // legacy adapters were removed in §01.3.
        let io_term = Term::new(
            usize::from(config.rows),
            usize::from(config.cols),
            config.scrollback,
            theme,
            QueueingEffectSink::new(),
        );

        // 5. Wire the message channel for PTY writes.
        let (tx, rx) = mpsc::channel();
        let notifier = PaneNotifier::new(tx);
        let write_stalled = Arc::new(AtomicBool::new(false));

        // 6. Create the Terminal IO thread + handle FIRST so the
        // writer thread receives a clone of the IO-thread wake
        // sender. The writer thread `try_send`s on this channel
        // when it sets `shutdown` so the IO thread observes
        // shutdown out of `select!` within one iteration —
        // closes the §04 review round 5 in-`select!`
        // window for the writer-thread setter.
        let (io_thread, mut io_handle) = io_thread::new_with_handle(io_thread::IoThreadConfig {
            terminal: io_term,
            pane_id,
            mux_tx: mux_tx.clone(),
            child_exit_rx,
            mode_cache: Arc::clone(&mode_cache),
            shutdown: Arc::clone(&shutdown),
            wakeup: Arc::clone(wakeup),
            grid_dirty: io_grid_dirty,
            pty_control: Some(control),
            // Spawned panes use pty_control above; the adopted_signal
            // path is for Section 03.9 Windows Default Terminal handoff.
            adopted_signal: None,
            initial_rows: config.rows,
            initial_cols: config.cols,
            selection_dirty: Arc::clone(&io_selection_dirty),
        });
        let byte_tx = io_handle.byte_sender();
        let io_wake_tx = io_handle.io_wake_sender();
        let io_join = io_thread.spawn()?;
        io_handle.set_join(io_join);

        // 7. Spawn the writer thread (owns rx + writer, sets shutdown
        // flag, wakes the IO thread on exit). The write_stalled
        // flag lets the main thread detect when the writer is
        // blocked on a full kernel PTY buffer and send SIGINT
        // directly to the child process group.
        let writer_thread = spawn_pty_writer(
            writer,
            rx,
            Arc::clone(&shutdown),
            Arc::clone(&write_stalled),
            io_wake_tx,
        )?;

        // 8. Spawn the PTY reader thread (byte forwarder only — no VTE parsing).
        let pty_reader = PtyReader::new(reader, byte_tx, Arc::clone(&shutdown));
        let reader_thread = pty_reader.spawn()?;

        Ok(Pane::from_parts(PaneParts {
            id: pane_id,
            domain_id: self.id,
            notifier,
            reader_thread,
            writer_thread,
            io_handle,
            pty: Box::new(pty),
            mode_cache,
            io_selection_dirty,
            write_stalled,
            #[cfg(unix)]
            master_fd,
        }))
    }
}
