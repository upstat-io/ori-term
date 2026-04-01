//! Event proxy for the Terminal IO thread's `Term`.
//!
//! During the dual-Term migration period (sections 02-06), both the old
//! `PtyEventLoop` and the new `PaneIoThread` each own a `Term`. The old
//! `Term` uses `MuxEventProxy` which fires title, CWD, bell, and `PtyWrite`
//! events. If the IO thread's `Term` also used `MuxEventProxy`, those
//! events would fire twice (duplicate DA responses = protocol violation).
//!
//! `IoThreadEventProxy` suppresses all metadata events. On `Wakeup`, it
//! only sets a `grid_dirty` flag. Section 07 flips `suppress_metadata` to
//! `false` when the old path is removed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use oriterm_core::{Event, EventListener};

/// Event listener for the IO thread's `Term`.
///
/// Suppresses metadata events (title, CWD, bell, `PtyWrite`) to prevent
/// duplicates during the dual-Term period. Sets `grid_dirty` on `Wakeup`.
pub struct IoThreadEventProxy {
    /// Set when the IO thread's grid has new content.
    grid_dirty: Arc<AtomicBool>,
    /// When `true`, suppress all events except `Wakeup` → `grid_dirty`.
    suppress_metadata: AtomicBool,
}

impl IoThreadEventProxy {
    /// Create a new proxy with metadata suppression enabled.
    pub fn new(grid_dirty: Arc<AtomicBool>, suppress_metadata: bool) -> Self {
        Self {
            grid_dirty,
            suppress_metadata: AtomicBool::new(suppress_metadata),
        }
    }

    /// Whether metadata events are currently suppressed.
    #[cfg(test)]
    pub fn is_suppressed(&self) -> bool {
        self.suppress_metadata.load(Ordering::Acquire)
    }
}

impl EventListener for IoThreadEventProxy {
    fn send_event(&self, event: Event) {
        match event {
            Event::Wakeup => {
                self.grid_dirty.store(true, Ordering::Release);
            }
            // All other events are suppressed while `suppress_metadata` is true.
            // This prevents duplicate title/CWD/bell/PtyWrite events during
            // the dual-Term period (old PtyEventLoop's Term handles those).
            _ => {
                if !self.suppress_metadata.load(Ordering::Acquire) {
                    // Section 07: when the old path is removed, metadata
                    // events will be forwarded here. Not yet implemented.
                    log::trace!("IoThreadEventProxy: unsuppressed event {event:?}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
