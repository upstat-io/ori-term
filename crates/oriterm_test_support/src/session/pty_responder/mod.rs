//! Test-side [`EventListener`] that captures round-trip responses.
//!
//! `PtyResponder` buffers four event variants that round-trip through the
//! PTY: [`Event::PtyWrite`] (DA/DSR replies), [`Event::ColorRequest`]
//! (OSC 4/10/11/12 color queries), [`Event::ClipboardLoad`] (OSC 52 read),
//! and [`Event::ClipboardStore`] (OSC 52 write). All other event variants
//! are intentionally ignored — the responder is a round-trip collector,
//! not a general event sink.
//!
//! Lives in its own sibling module so the OSC dispatch can grow without
//! pushing `session/mod.rs` over the 500-line hygiene limit.

use std::sync::{Arc, Mutex};

use oriterm_core::Rgb;
use oriterm_core::event::{ClipboardType, Event, EventListener};

/// Deterministic RGB value injected into every `ColorRequest` callback.
///
/// OSC 4/10/11/12 responses need *some* color — in a headless test we
/// cannot query the real theme, and production values would couple every
/// test to the default palette. A fixed `0xabcdef` is memorable in failure
/// output (rgb:abab/cdcd/efef) and collision-resistant against accidental
/// palette-origin matches.
const TEST_COLOR: Rgb = Rgb {
    r: 0xab,
    g: 0xcd,
    b: 0xef,
};

/// Deterministic clipboard text injected into every `ClipboardLoad` callback.
///
/// OSC 52 load asks us to produce clipboard contents; a pinned string
/// keeps the base64-encoded response stable across runs and makes
/// assertion failures decodable by eye.
const TEST_CLIPBOARD_TEXT: &str = "ori-term-clipboard-stub";

/// Captures round-trip responses so the test driver can write them back.
///
/// Completes DA/DSR query/response handshakes inside `vttest`/`tack` and
/// OSC 4/10/11/12/52 round-trips for Section 06.0.c. `pub` only because
/// it surfaces in `Term<PtyResponder>`; constructor, drain, and handlers
/// are `pub(crate)`/private so external callers cannot steal the reply
/// queues owned by `PtySession::drain`/`drain_blocking`.
///
/// **Clone semantics.** The struct holds only `Arc<Mutex<_>>` handles so
/// all clones share the same underlying queues. Tests use this to hand
/// one clone to `Term::new` (which moves the listener) and keep another
/// clone for inspection.
#[derive(Clone)]
pub struct PtyResponder {
    responses: Arc<Mutex<Vec<String>>>,
    osc_responses: Arc<Mutex<Vec<String>>>,
    clipboard_stores: Arc<Mutex<Vec<(ClipboardType, String)>>>,
}

impl PtyResponder {
    /// Construct an empty responder with no buffered responses.
    pub(crate) fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            osc_responses: Arc::new(Mutex::new(Vec::new())),
            clipboard_stores: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Drain all buffered `PtyWrite` payloads, returning them in arrival
    /// order. The internal buffer is reset to empty.
    pub(crate) fn take_responses(&self) -> Vec<String> {
        std::mem::take(&mut *self.responses.lock().expect("PtyResponder mutex poisoned"))
    }

    /// Drain all buffered OSC response strings (from `ColorRequest` and
    /// `ClipboardLoad` callbacks). `PtySession::drain` writes these back
    /// through the PTY writer automatically.
    pub(crate) fn take_osc_responses(&self) -> Vec<String> {
        std::mem::take(
            &mut *self
                .osc_responses
                .lock()
                .expect("PtyResponder mutex poisoned"),
        )
    }

    /// Drain all buffered clipboard-store events. These are consumed by
    /// callers that need to verify what the client attempted to paste
    /// into the host clipboard — they are NOT written back through the
    /// PTY (OSC 52 store is one-way).
    pub(crate) fn take_clipboard_stores(&self) -> Vec<(ClipboardType, String)> {
        std::mem::take(
            &mut *self
                .clipboard_stores
                .lock()
                .expect("PtyResponder mutex poisoned"),
        )
    }

    fn push_pty_write(&self, data: String) {
        self.responses
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push(data);
    }

    fn handle_color_request(
        &self,
        _index: usize,
        formatter: &Arc<dyn Fn(Rgb) -> String + Send + Sync>,
    ) {
        let response = formatter(TEST_COLOR);
        self.osc_responses
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push(response);
    }

    fn handle_clipboard_load(
        &self,
        _clipboard: ClipboardType,
        formatter: &Arc<dyn Fn(&str) -> String + Send + Sync>,
    ) {
        let response = formatter(TEST_CLIPBOARD_TEXT);
        self.osc_responses
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push(response);
    }

    fn handle_clipboard_store(&self, clipboard: ClipboardType, text: String) {
        self.clipboard_stores
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push((clipboard, text));
    }
}

impl EventListener for PtyResponder {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(data) => self.push_pty_write(data),
            Event::ColorRequest(index, formatter) => self.handle_color_request(index, &formatter),
            Event::ClipboardLoad(clipboard, formatter) => {
                self.handle_clipboard_load(clipboard, &formatter);
            }
            Event::ClipboardStore(clipboard, text) => self.handle_clipboard_store(clipboard, text),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
