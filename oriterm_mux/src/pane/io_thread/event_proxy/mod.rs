//! Event proxy for the Terminal IO thread's `Term`.
//!
//! `IoThreadEventProxy` is the `EventListener` for the IO thread's `Term`.
//! On `Wakeup`, it sets `grid_dirty` so the IO thread knows to produce a
//! new snapshot. On metadata events (title, CWD, bell, `PtyWrite`, clipboard),
//! it forwards them to the mux event channel so they reach the main thread.
//!
//! During the dual-Term migration period (sections 02-06), `suppress_metadata`
//! was `true` to prevent duplicate events from both the old and new `Term`.
//! After section 07, the old `Term` is removed and `suppress_metadata` is
//! `false` — the IO thread's proxy is the sole source of metadata events.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use oriterm_core::{Event, EventListener};

use crate::PaneId;
use crate::mux_event::MuxEvent;

/// Event listener for the IO thread's `Term`.
///
/// Sets `grid_dirty` on `Wakeup`. Forwards metadata events to the mux
/// event channel when `suppress_metadata` is false.
pub struct IoThreadEventProxy {
    /// Set when the IO thread's grid has new content.
    grid_dirty: Arc<AtomicBool>,
    /// When `true`, suppress all events except `Wakeup` → `grid_dirty`.
    suppress_metadata: bool,
    /// Identity of the pane (for wrapping events as `MuxEvent`).
    pane_id: PaneId,
    /// Channel sender to the mux event processor.
    mux_tx: mpsc::Sender<MuxEvent>,
    /// Wakes the event loop when metadata events arrive.
    wakeup: Arc<dyn Fn() + Send + Sync>,
}

impl IoThreadEventProxy {
    /// Create a new proxy.
    ///
    /// When `suppress_metadata` is `true`, only `Wakeup` → `grid_dirty` is
    /// active (dual-Term migration mode). When `false`, all metadata events
    /// are forwarded to the mux channel (post section 07 mode).
    pub fn new(
        grid_dirty: Arc<AtomicBool>,
        suppress_metadata: bool,
        pane_id: PaneId,
        mux_tx: mpsc::Sender<MuxEvent>,
        wakeup: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            grid_dirty,
            suppress_metadata,
            pane_id,
            mux_tx,
            wakeup,
        }
    }

    /// Whether metadata events are currently suppressed.
    #[cfg(test)]
    pub fn is_suppressed(&self) -> bool {
        self.suppress_metadata
    }

    /// Send a `MuxEvent` and wake the event loop.
    fn send(&self, event: MuxEvent) {
        let _ = self.mux_tx.send(event);
        (self.wakeup)();
    }
}

impl EventListener for IoThreadEventProxy {
    fn send_event(&self, event: Event) {
        match event {
            Event::Wakeup => {
                self.grid_dirty.store(true, Ordering::Release);
            }
            _ if self.suppress_metadata => {
                // Dual-Term migration: metadata suppressed to prevent duplicates.
            }
            // Post section 07: forward metadata events to the mux channel.
            Event::Bell => {
                self.send(MuxEvent::PaneBell(self.pane_id));
            }
            Event::Title(title) => {
                self.send(MuxEvent::PaneTitleChanged {
                    pane_id: self.pane_id,
                    title,
                });
            }
            Event::ResetTitle => {
                self.send(MuxEvent::PaneTitleChanged {
                    pane_id: self.pane_id,
                    title: String::new(),
                });
            }
            Event::IconName(name) => {
                self.send(MuxEvent::PaneIconChanged {
                    pane_id: self.pane_id,
                    icon_name: name,
                });
            }
            Event::ResetIconName => {
                self.send(MuxEvent::PaneIconChanged {
                    pane_id: self.pane_id,
                    icon_name: String::new(),
                });
            }
            Event::ClipboardStore(clipboard_type, text) => {
                self.send(MuxEvent::ClipboardStore {
                    pane_id: self.pane_id,
                    clipboard_type,
                    text,
                });
            }
            Event::ClipboardLoad(clipboard_type, formatter) => {
                self.send(MuxEvent::ClipboardLoad {
                    pane_id: self.pane_id,
                    clipboard_type,
                    formatter,
                });
            }
            Event::PtyWrite(data) => {
                self.send(MuxEvent::PtyWrite {
                    pane_id: self.pane_id,
                    data,
                });
            }
            Event::Cwd(cwd) => {
                self.send(MuxEvent::PaneCwdChanged {
                    pane_id: self.pane_id,
                    cwd,
                });
            }
            Event::CommandComplete(duration) => {
                self.send(MuxEvent::CommandComplete {
                    pane_id: self.pane_id,
                    duration,
                });
            }
            Event::ChildExit(code) => {
                self.send(MuxEvent::PaneExited {
                    pane_id: self.pane_id,
                    exit_code: code,
                });
            }
            // Events that don't need mux routing — wake the event loop.
            Event::ColorRequest(..) | Event::CursorBlinkingChange | Event::MouseCursorDirty => {
                (self.wakeup)();
            }
        }
    }
}

#[cfg(test)]
mod tests;
