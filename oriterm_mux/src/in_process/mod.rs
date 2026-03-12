//! In-process multiplexer orchestrating pane lifecycle.
//!
//! [`InProcessMux`] is the synchronous, single-thread multiplexer that owns
//! the pane registry, ID allocators, and domain list. It does not own `Pane`
//! structs — those live in the caller's pane map to avoid borrow conflicts.
//!
//! Event flow: PTY reader threads → `mpsc` → [`InProcessMux::poll_events`] →
//! [`MuxNotification`] queue → caller drains notifications.

mod event_pump;

use std::io;
use std::sync::Arc;
use std::sync::mpsc;

use oriterm_core::Theme;

use crate::DomainId;
use crate::domain::{Domain, LocalDomain, SpawnConfig};
use crate::id::{IdAllocator, PaneId};
use crate::mux_event::{MuxEvent, MuxNotification};
use crate::pane::Pane;
use crate::registry::{PaneEntry, PaneRegistry};

/// Result of closing a single pane.
#[derive(Debug, PartialEq, Eq)]
pub enum ClosePaneResult {
    /// Pane removed from the registry.
    PaneRemoved,
    /// Pane ID was not found in the registry.
    NotFound,
}

/// Synchronous in-process multiplexer.
///
/// Orchestrates pane lifecycle, owns the pane registry and ID allocators,
/// and bridges PTY events to notifications. All operations run on the
/// main thread — no daemon, no IPC.
pub struct InProcessMux {
    pane_registry: PaneRegistry,

    // Domain — stored concretely to call `spawn_pane` without downcasting.
    // Extended to a domain registry when WSL/SSH domains are added (Section 35).
    local_domain: LocalDomain,

    // ID allocators.
    #[allow(
        dead_code,
        reason = "used when WSL/SSH domains are added in Section 35"
    )]
    domain_alloc: IdAllocator<DomainId>,
    pane_alloc: IdAllocator<PaneId>,

    // Event channels.
    event_tx: mpsc::Sender<MuxEvent>,
    event_rx: mpsc::Receiver<MuxEvent>,
    notifications: Vec<MuxNotification>,
}

impl Default for InProcessMux {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessMux {
    /// Create a new in-process mux with a local domain.
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        let domain_alloc: IdAllocator<DomainId> = IdAllocator::new();
        // Local domain uses the well-known ID 0. The allocator starts at 1,
        // so dynamically added domains (WSL, SSH) never collide.
        let local = LocalDomain::new(DomainId::LOCAL);

        Self {
            pane_registry: PaneRegistry::new(),
            local_domain: local,
            domain_alloc,
            pane_alloc: IdAllocator::new(),
            event_tx,
            event_rx,
            notifications: Vec::new(),
        }
    }

    // Pane operations

    /// Spawn a pane with a new PTY process.
    ///
    /// The pane is registered in the pane registry. The caller receives
    /// the `Pane` struct to store in its own map.
    pub fn spawn_standalone_pane(
        &mut self,
        config: &SpawnConfig,
        theme: Theme,
        wakeup: &Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<(PaneId, Pane)> {
        let pane_id = self.pane_alloc.alloc();
        let domain_id = self.local_domain.id();
        let pane = self.local_domain.spawn_pane(
            pane_id,
            config,
            theme,
            &self.event_tx,
            Arc::clone(wakeup),
        )?;

        self.pane_registry.register(PaneEntry {
            pane: pane_id,
            domain: domain_id,
        });

        Ok((pane_id, pane))
    }

    /// Close a pane, removing it from the registry.
    ///
    /// Uses exit code 0 (clean exit). For process-initiated closes with a
    /// real exit code, use [`close_pane_with_exit_code`](Self::close_pane_with_exit_code).
    pub fn close_pane(&mut self, pane_id: PaneId) -> ClosePaneResult {
        self.close_pane_with_exit_code(pane_id, 0)
    }

    /// Close a pane with a specific exit code.
    ///
    /// The caller is responsible for dropping the `Pane` struct from its map.
    pub fn close_pane_with_exit_code(
        &mut self,
        pane_id: PaneId,
        exit_code: i32,
    ) -> ClosePaneResult {
        if self.pane_registry.unregister(pane_id).is_none() {
            return ClosePaneResult::NotFound;
        }

        self.notifications
            .push(MuxNotification::PaneClosed { pane_id, exit_code });
        ClosePaneResult::PaneRemoved
    }

    /// Look up a pane's metadata entry.
    pub fn get_pane_entry(&self, pane_id: PaneId) -> Option<&PaneEntry> {
        self.pane_registry.get(pane_id)
    }
}

// Test helpers

#[cfg(test)]
impl InProcessMux {
    /// Register a test pane in the registry without spawning a PTY.
    ///
    /// Use raw IDs starting at 100+ to avoid collision with the allocator.
    pub(crate) fn inject_test_pane(&mut self, pid: PaneId) {
        self.pane_registry.register(PaneEntry {
            pane: pid,
            domain: self.local_domain.id(),
        });
        self.notifications.clear();
    }
}

#[cfg(test)]
mod tests;
