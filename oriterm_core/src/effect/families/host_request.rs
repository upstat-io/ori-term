//! Typed request/response effects — replaces closure-based Event variants.
//!
//! The handler emits a `HostRequest` with a `ResponseToken`; the consumer
//! fulfills the token with the requested data; the terminal formats the
//! reply and emits it as `Effect::Pty(PtyEffect::Write { .. })`.

use std::sync::{Arc, Mutex};

use super::ClipboardSelection;

/// A host request that requires a response from the consumer.
///
/// Replaces the closure-carrying `Event::ClipboardLoad` and
/// `Event::ColorRequest` variants with typed, inspectable data.
#[derive(Debug, Clone)]
pub enum HostRequest {
    /// Replaces `Event::ClipboardLoad` — was `Arc<dyn Fn(&str) -> String>`.
    ///
    /// `clipboard_char` is the raw OSC 52 clipboard character (e.g. `b'c'`,
    /// `b'p'`, `b's'`). `terminator` is the OSC string terminator (ST or BEL)
    /// needed to format the reply with the matching terminator.
    ClipboardLoad {
        selection: ClipboardSelection,
        clipboard_char: u8,
        terminator: String,
        reply: ResponseToken<String>,
    },
    /// Replaces `Event::ColorRequest` — was `Arc<dyn Fn(Rgb) -> String>`.
    ///
    /// `prefix` is the OSC prefix string (e.g. `"4"`, `"10"`, `"11"`).
    /// `terminator` is the OSC string terminator (ST or BEL). Both were
    /// captured in the legacy closure; the typed request preserves them
    /// as plain data so the reply formatter can reconstruct the correct
    /// escape sequence.
    ColorQuery {
        prefix: String,
        index: usize,
        terminator: String,
        reply: ResponseToken<crate::color::Rgb>,
    },
}

/// Token the consumer holds to deliver a reply to a `HostRequest`.
///
/// The terminal handler creates a `ResponseToken` when emitting the request,
/// and the consumer fulfills it by calling `token.fulfill(value)`. The
/// terminal then observes the fulfillment via `take()` and formats the
/// reply for PTY emission.
///
/// Implementation: wraps an `Arc<Mutex<Option<T>>>` — the consumer puts
/// the response into the slot; the terminal drains the slot on the next
/// event loop tick. NOT a closure — the value is plain data.
#[derive(Debug, Clone)]
pub struct ResponseToken<T> {
    slot: Arc<Mutex<Option<T>>>,
}

impl<T> ResponseToken<T> {
    /// Create a new unfulfilled token.
    pub fn new() -> Self {
        Self {
            slot: Arc::new(Mutex::new(None)),
        }
    }

    /// Fulfill the request with the given value.
    ///
    /// Called by the consumer (host side) to deliver the response.
    pub fn fulfill(&self, value: T) {
        *self.slot.lock().expect("ResponseToken mutex poisoned") = Some(value);
    }

    /// Take the fulfilled value, if any.
    ///
    /// Called by the terminal side to drain the response.
    pub fn take(&self) -> Option<T> {
        self.slot
            .lock()
            .expect("ResponseToken mutex poisoned")
            .take()
    }

    /// Returns `true` if the token has been fulfilled but not yet taken.
    pub fn is_fulfilled(&self) -> bool {
        self.slot
            .lock()
            .expect("ResponseToken mutex poisoned")
            .is_some()
    }
}

impl<T> Default for ResponseToken<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker type for acknowledgement responses where no data is returned.
pub type ResponseFulfilled = ();
