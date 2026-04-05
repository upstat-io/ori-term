//! Local domain — spawns shells on the local machine via `portable-pty`.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc;

use oriterm_core::{Term, Theme};

use crate::{DomainId, PaneId};

use super::{Domain, DomainState, SpawnConfig};

use crate::mux_event::MuxEvent;
use crate::pane::io_thread;
use crate::pane::io_thread::event_proxy::IoThreadEventProxy;
use crate::pane::{Pane, PaneNotifier, PaneParts};
use crate::pty::{PtyConfig, PtyReader, spawn_pty, spawn_pty_writer};

/// Spawns shells on the local machine.
///
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
    ///
    /// Creates the PTY, a single `Term` owned exclusively by the IO thread,
    /// a PTY byte reader, and a PTY writer thread. The IO thread handles all
    /// VTE parsing, commands, and snapshot production. The main thread
    /// communicates via channels only — no shared terminal lock.
    #[allow(
        clippy::too_many_arguments,
        reason = "all six parameters are required to assemble a Pane"
    )]
    pub fn spawn_pane(
        &self,
        pane_id: PaneId,
        config: &SpawnConfig,
        theme: Theme,
        mux_tx: &mpsc::Sender<MuxEvent>,
        wakeup: &Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<Pane> {
        // 1. Spawn PTY with the configured shell.
        let pty_config = PtyConfig {
            rows: config.rows,
            cols: config.cols,
            shell: config.shell.clone(),
            working_dir: config.cwd.clone(),
            env: config.env.clone(),
            shell_integration: config.shell_integration,
        };
        let mut pty = spawn_pty(&pty_config)?;

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

        // 3. Set up shared atomics.
        let io_grid_dirty = Arc::new(AtomicBool::new(false));
        let mode_cache = Arc::new(AtomicU32::new(oriterm_core::TermMode::default().bits()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let io_selection_dirty = Arc::new(AtomicBool::new(false));

        // 4. Create the single Term with IoThreadEventProxy (unsuppressed —
        //    this is the sole source of metadata events).
        let io_event_proxy = IoThreadEventProxy::new(
            Arc::clone(&io_grid_dirty),
            false, // suppress_metadata = false (post section 07)
            pane_id,
            mux_tx.clone(),
            Arc::clone(wakeup),
        );
        let io_term = Term::new(
            usize::from(config.rows),
            usize::from(config.cols),
            config.scrollback,
            theme,
            io_event_proxy,
        );

        // 5. Wire the message channel for PTY writes.
        let (tx, rx) = mpsc::channel();
        let notifier = PaneNotifier::new(tx);

        // 6. Spawn the writer thread (owns rx + writer, sets shutdown flag).
        //    The write_stalled flag lets the main thread detect when the
        //    writer is blocked on a full kernel PTY buffer and send SIGINT
        //    directly to the child process group.
        let write_stalled = Arc::new(AtomicBool::new(false));
        let writer_thread = spawn_pty_writer(
            writer,
            rx,
            Arc::clone(&shutdown),
            Arc::clone(&write_stalled),
        )?;

        // 7. Spawn the Terminal IO thread (owns Term, VTE processors, PtyControl).
        let (io_thread, mut io_handle) = io_thread::new_with_handle(io_thread::IoThreadConfig {
            terminal: io_term,
            mode_cache: Arc::clone(&mode_cache),
            shutdown: Arc::clone(&shutdown),
            wakeup: Arc::clone(wakeup),
            grid_dirty: io_grid_dirty,
            pty_control: Some(control),
            initial_rows: config.rows,
            initial_cols: config.cols,
            selection_dirty: Arc::clone(&io_selection_dirty),
        });
        let byte_tx = io_handle.byte_sender();
        let io_join = io_thread.spawn()?;
        io_handle.set_join(io_join);

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
            pty,
            mode_cache,
            io_selection_dirty,
            write_stalled,
        }))
    }
}
