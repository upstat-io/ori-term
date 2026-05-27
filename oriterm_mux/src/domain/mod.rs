//! Domain trait, spawn configuration, and concrete domain implementations.
//!
//! A domain represents an environment where shells can be spawned: the local
//! machine, a WSL distro, an SSH host, or a serial port. The [`Domain`] trait
//! provides identity and metadata. Concrete implementations ([`LocalDomain`])
//! handle actual PTY spawning.

pub(crate) mod handoff;
pub(crate) mod local;
pub(crate) mod wsl;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use crate::id::DomainId;
use crate::mux_event::MuxEvent;

#[allow(
    unused_imports,
    reason = "consumed by oriterm/src/platform/default_terminal in Section 03.9 Phase 4"
)]
pub use handoff::{AdoptConfig, adopt_pane};
pub use local::LocalDomain;
#[allow(unused_imports, reason = "used when WSL domain is wired in Section 35")]
pub use wsl::WslDomain;

/// Lifecycle state of a domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    /// Domain is connected and can spawn panes.
    Attached,
    /// Domain is disconnected (e.g., SSH session dropped).
    Detached,
}

/// Configuration for spawning a new pane.
///
/// Passed from the mux layer to a domain's spawn method. All fields are
/// optional except grid dimensions — domains apply sensible defaults for
/// anything unset.
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    /// Initial terminal columns.
    pub cols: u16,
    /// Initial terminal rows.
    pub rows: u16,
    /// Shell program override. `None` uses the domain's default.
    pub shell: Option<String>,
    /// Working directory for the child process.
    pub cwd: Option<PathBuf>,
    /// Additional environment variables.
    pub env: Vec<(String, String)>,
    /// Scrollback buffer size in lines.
    pub scrollback: usize,
    /// Enable shell integration (inject scripts for OSC 133/7 support).
    pub shell_integration: bool,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: None,
            env: Vec::new(),
            scrollback: 10_000,
            shell_integration: true,
        }
    }
}

/// Event-loop wiring threaded into a pane at spawn time.
///
/// Groups the borrowed channel + wakeup callback the IO thread needs so
/// [`LocalDomain::spawn_pane`](local::LocalDomain::spawn_pane) takes a single
/// wiring argument rather than separate positional parameters. All fields are
/// shared references, so the type is `Copy`.
#[derive(Clone, Copy)]
pub struct SpawnWiring<'a> {
    /// Channel the IO thread emits `MuxEvent`s on.
    pub mux_tx: &'a mpsc::Sender<MuxEvent>,
    /// Callback fired to wake the main event loop when the pane has new state.
    pub wakeup: &'a Arc<dyn Fn() + Send + Sync>,
}

/// A shell-spawning backend.
///
/// The trait is intentionally minimal: identity, metadata, and capability
/// queries only. Actual spawning requires I/O types (`mpsc::Sender`,
/// `EventLoopProxy`, PTY handles) that live in the binary crate, so
/// `spawn_pane` is a concrete method on each domain implementation rather
/// than a trait method here.
pub trait Domain: Send + Sync {
    /// Unique domain identifier.
    fn id(&self) -> DomainId;

    /// Human-readable domain name (e.g., `"local"`, `"WSL:Ubuntu"`).
    fn name(&self) -> &str;

    /// Current lifecycle state.
    fn state(&self) -> DomainState;

    /// Whether this domain can currently spawn new panes.
    fn can_spawn(&self) -> bool;
}

#[cfg(test)]
mod tests;
