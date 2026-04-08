//! Test-side [`EventListener`] that captures round-trip responses.
//!
//! `PtyResponder` buffers `Event::PtyWrite` payloads (DA/DSR replies,
//! bracketed paste) so [`PtySession`](super::PtySession) can write them
//! back through the PTY master. Lives in its own sibling module so the
//! 06.0.c OSC-event extension can grow without pushing `session/mod.rs`
//! over the 500-line limit.

use std::sync::{Arc, Mutex};

use oriterm_core::event::{Event, EventListener};

/// Captures `PtyWrite` responses so the test driver can write them back.
///
/// Completes DA/DSR query/response handshakes inside `vttest`/`tack`.
/// `pub` only because it surfaces in `Term<PtyResponder>`; constructor
/// and drain are `pub(crate)` so external callers cannot steal the
/// reply queue owned by `PtySession::drain`/`drain_blocking`.
pub struct PtyResponder {
    responses: Arc<Mutex<Vec<String>>>,
}

impl PtyResponder {
    /// Construct an empty responder with no buffered responses.
    pub(crate) fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Drain all buffered `PtyWrite` payloads, returning them in arrival
    /// order. The internal buffer is reset to empty.
    pub(crate) fn take_responses(&self) -> Vec<String> {
        std::mem::take(&mut *self.responses.lock().expect("PtyResponder mutex poisoned"))
    }
}

impl EventListener for PtyResponder {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(data) = event {
            self.responses
                .lock()
                .expect("PtyResponder mutex poisoned")
                .push(data);
        }
    }
}

#[cfg(test)]
mod tests;
