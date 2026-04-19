//! Test-side [`EffectSink`] that captures round-trip responses.
//!
//! `PtyResponder` collects four effect families that round-trip through the
//! PTY: [`PtyEffect::Write`] (DA/DSR replies), [`HostRequest::ColorQuery`]
//! (OSC 4/10/11/12 color queries), [`HostRequest::ClipboardLoad`] (OSC 52
//! read), and [`HostEffect::ClipboardStore`] (OSC 52 write). All other
//! effect variants are intentionally ignored — the responder is a
//! round-trip collector, not a general effect sink.
//!
//! Lives in its own sibling module so the OSC dispatch can grow without
//! pushing `session/mod.rs` over the 500-line hygiene limit.
//!
//! Implements [`EffectSink`] directly so `Term<PtyResponder>` plugs in
//! without a wrapper. Replies for `HostRequest` variants are formatted
//! inline via the canonical helpers in `oriterm_core::effect`
//! ([`format_clipboard_reply`], [`format_color_reply`]) — no separate IO
//! thread is needed in tests, so the response is computed at push-time
//! rather than via the production `register_host_request_response` poll
//! cycle. The stored reply is byte-identical to what the production
//! `effect_router` + `pending_responses` path produces.

use std::sync::{Arc, Mutex};

use oriterm_core::Rgb;
use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{
    ClipboardSelection, Effect, HostEffect, HostRequest, PtyEffect, format_clipboard_reply,
    format_color_reply,
};
use oriterm_core::event::ClipboardType;

/// Deterministic RGB value injected into every `ColorQuery` reply.
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

/// Deterministic clipboard text injected into every `ClipboardLoad` reply.
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
/// one clone to `Term::new` (which moves the sink) and keep another
/// clone for inspection.
#[derive(Clone, Default)]
pub struct PtyResponder {
    responses: Arc<Mutex<Vec<String>>>,
    osc_responses: Arc<Mutex<Vec<String>>>,
    clipboard_stores: Arc<Mutex<Vec<(ClipboardType, String)>>>,
}

impl PtyResponder {
    /// Construct an empty responder with no buffered responses.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Drain all buffered `PtyWrite` payloads, returning them in arrival
    /// order. The internal buffer is reset to empty.
    pub(crate) fn take_responses(&self) -> Vec<String> {
        std::mem::take(&mut *self.responses.lock().expect("PtyResponder mutex poisoned"))
    }

    /// Drain all buffered OSC reply strings (from `ColorQuery` and
    /// `ClipboardLoad` replies). `PtySession::drain` writes these back
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

    fn push_pty_write(&self, bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes).into_owned();
        self.responses
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push(s);
    }

    fn handle_color_query(&self, prefix: &str, terminator: &str) {
        let bytes = format_color_reply(TEST_COLOR, prefix, terminator);
        let response = String::from_utf8_lossy(&bytes).into_owned();
        self.osc_responses
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push(response);
    }

    fn handle_clipboard_load(&self, clipboard_char: u8, terminator: &str) {
        let bytes = format_clipboard_reply(TEST_CLIPBOARD_TEXT, clipboard_char, terminator);
        let response = String::from_utf8_lossy(&bytes).into_owned();
        self.osc_responses
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push(response);
    }

    fn handle_clipboard_store(&self, selection: ClipboardSelection, data: String) {
        let clipboard = match selection {
            ClipboardSelection::Clipboard => ClipboardType::Clipboard,
            ClipboardSelection::Primary | ClipboardSelection::Select => ClipboardType::Selection,
        };
        self.clipboard_stores
            .lock()
            .expect("PtyResponder mutex poisoned")
            .push((clipboard, data));
    }
}

impl EffectSink for PtyResponder {
    fn push(&self, effect: Effect) {
        match effect {
            Effect::Pty(PtyEffect::Write { bytes, .. }) => self.push_pty_write(&bytes),
            Effect::HostRequest(HostRequest::ColorQuery {
                prefix, terminator, ..
            }) => self.handle_color_query(&prefix, &terminator),
            Effect::HostRequest(HostRequest::ClipboardLoad {
                clipboard_char,
                terminator,
                ..
            }) => self.handle_clipboard_load(clipboard_char, &terminator),
            Effect::Host(HostEffect::ClipboardStore { selection, data }) => {
                self.handle_clipboard_store(selection, data);
            }
            _ => {}
        }
    }

    fn drain_into(&self, _out: &mut Vec<Effect>) {
        // Tests read responses via the side-channel `take_*()` helpers;
        // `drain_into` is required by the trait but unused for this sink.
    }
}

#[cfg(test)]
mod tests;
