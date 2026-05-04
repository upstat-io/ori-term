//! Recorded-callback fixture mirroring the wakeup-callback shape from
//! `oriterm/src/app/constructors.rs:make_mux_wakeup`.
//!
//! Tests can construct a [`RecordedProxy`], pass its wakeup closure into
//! `EmbeddedMux::new` (or any `Arc<dyn Fn() + Send + Sync>` consumer), and
//! observe every fire as a [`RecordedTermEvent`] entry — without standing
//! up a real winit `EventLoopProxy`.
//!
//! The local [`RecordedTermEvent`] enum keeps `oriterm_test_support`
//! dependency-free of the `oriterm` crate (per the §02 fix
//! consensus, revision: avoid dev-dep direction inversion).

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// Local enum mirroring `oriterm::TermEvent` at the test boundary.
///
/// Tests that exercise wakeup-driven event-loop semantics can observe
/// every `RecordedTermEvent::MuxWakeup` instance without taking a
/// dependency on the `oriterm` crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordedTermEvent {
    MuxWakeup,
}

/// In-memory replacement for `winit::event_loop::EventLoopProxy<TermEvent>`
/// that records every fire.
///
/// Construct via [`RecordedProxy::new`], extract a wakeup closure with
/// [`make_mux_wakeup`](Self::make_mux_wakeup) and pass it into the
/// production callback consumer (e.g. `EmbeddedMux::new`). After the
/// test action runs, inspect the observed event sequence with
/// [`observed`](Self::observed) or block on the next event with
/// [`wait_for_wakeup`](Self::wait_for_wakeup).
pub struct RecordedProxy {
    events: Arc<Mutex<Vec<RecordedTermEvent>>>,
    tx: Sender<RecordedTermEvent>,
    rx: Mutex<Receiver<RecordedTermEvent>>,
}

impl RecordedProxy {
    /// Construct a fresh [`RecordedProxy`] with an empty event log.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            tx,
            rx: Mutex::new(rx),
        }
    }

    /// Build a wakeup closure mirroring the production shape at
    /// `oriterm/src/app/constructors.rs:make_mux_wakeup`.
    ///
    /// The closure pushes `RecordedTermEvent::MuxWakeup` into the
    /// observation log AND sends it through the mpsc channel for
    /// poll-the-condition tests via
    /// [`wait_for_wakeup`](Self::wait_for_wakeup). Both writes use
    /// `let _ = ...` semantics so the closure is byte-loss-safe even
    /// when the receiver has been dropped — matches the production
    /// `let _ = proxy.send_event(TermEvent::MuxWakeup)` semantics.
    pub fn make_mux_wakeup(&self) -> Arc<dyn Fn() + Send + Sync> {
        let tx = self.tx.clone();
        let events = Arc::clone(&self.events);
        Arc::new(move || {
            if let Ok(mut log) = events.lock() {
                log.push(RecordedTermEvent::MuxWakeup);
            }
            let _ = tx.send(RecordedTermEvent::MuxWakeup);
        })
    }

    /// Block until the next wakeup arrives or `deadline` elapses.
    ///
    /// The deadline is the safety valve to surface a hang — the awaited
    /// condition is the channel receive, not the elapsed time.
    ///
    /// # Errors
    ///
    /// Returns [`RecvTimeoutError::Timeout`] when no wakeup arrives
    /// within `deadline`; returns [`RecvTimeoutError::Disconnected`]
    /// when every sender has been dropped (only possible if the
    /// `RecordedProxy` itself was dropped).
    pub fn wait_for_wakeup(
        &self,
        deadline: Duration,
    ) -> Result<RecordedTermEvent, RecvTimeoutError> {
        let rx = self.rx.lock().expect("RecordedProxy receiver poisoned");
        rx.recv_timeout(deadline)
    }

    /// Snapshot the observed event sequence in order.
    ///
    /// The returned `Vec` is a clone so the underlying log can continue
    /// to grow during subsequent test actions.
    #[must_use]
    pub fn observed(&self) -> Vec<RecordedTermEvent> {
        self.events
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }
}

impl Default for RecordedProxy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
