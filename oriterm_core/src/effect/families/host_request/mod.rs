//! Typed request/response effects — replaces closure-based Event variants.
//!
//! The handler emits a `HostRequest` with a `ResponseToken`; the consumer
//! fulfills the token with the requested data; the terminal formats the
//! reply and emits it as `Effect::Pty(PtyEffect::Write { .. })`.
//!
//! # Single-assignment fulfill
//!
//! `ResponseToken::fulfill` returns `Result<(), AlreadyFulfilled>`. A duplicate
//! fulfillment is a routing bug — it returns `Err(AlreadyFulfilled)` rather
//! than silently overwriting the stored value.
//!
//! # Move-only across staging buffers
//!
//! `ResponseToken` is `Clone` (its inner `Arc<Mutex<Option<T>>>` is
//! shared-ownership), but consumers MUST NOT clone a `ResponseToken` into
//! intermediate staging buffers. Cancellation detection in `PendingResponse`
//! relies on `Arc::strong_count` dropping when the main-thread handle is
//! dropped; a stray clone in a staging buffer keeps the strong count elevated
//! forever and the IO thread's pending entry leaks. The only legitimate clone
//! is the two-copy handoff: one clone into the IO thread's pending-response
//! closure, one clone into the main-thread's `MuxEvent` reply field. All
//! downstream staging buffers (`notifications: Vec<MuxNotification>`,
//! `notification_buf`, daemon server broadcast, daemon client local) MUST move
//! (`Vec::drain`, `mem::replace`, match-move), never `.clone()`.

use std::sync::{Arc, Mutex};

use super::ClipboardSelection;

/// A host request that requires a response from the consumer.
///
/// Carries typed, inspectable data instead of an opaque formatter
/// closure: each variant exposes the OSC parameters needed to
/// reconstruct the canonical reply via `format_clipboard_reply` /
/// `format_color_reply`. The reply is delivered asynchronously via
/// `ResponseToken<T>::fulfill`.
#[derive(Debug, Clone)]
pub enum HostRequest {
    /// OSC 52 clipboard read.
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
    /// OSC 4/10/11/12 color query.
    ///
    /// `prefix` is the OSC prefix string (e.g. `"4"`, `"10"`, `"11"`).
    /// `terminator` is the OSC string terminator (ST or BEL). The reply
    /// formatter reconstructs the escape sequence from these plus the
    /// fulfilled `Rgb` value.
    ColorQuery {
        prefix: String,
        index: usize,
        terminator: String,
        reply: ResponseToken<crate::color::Rgb>,
    },
}

/// Error returned by `ResponseToken::fulfill` when the token has already
/// been fulfilled.
///
/// Duplicate fulfillment is a routing bug: multiple consumers attempted to
/// answer the same host request. The token's stored value is NOT overwritten
/// — the first fulfill wins. Callers should log an error and continue; do not
/// panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlreadyFulfilled;

impl std::fmt::Display for AlreadyFulfilled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResponseToken already fulfilled — duplicate fulfill is a routing bug")
    }
}

impl std::error::Error for AlreadyFulfilled {}

/// Three-state slot backing [`ResponseToken`].
///
/// The state distinguishes "never fulfilled" from "fulfilled but already
/// drained" so [`fulfill`](ResponseToken::fulfill) can reject duplicate
/// fulfillment AFTER the IO-thread side has consumed the value. Without
/// the explicit `Consumed` state the slot would revert to "empty" after
/// `take()` and a second `fulfill` would silently succeed —
/// indistinguishable from a legitimate first fulfill. Surfaced by
/// `[high]`.
#[derive(Debug)]
enum SlotState<T> {
    /// No fulfill has been issued yet.
    Pending,
    /// Fulfilled with a value, awaiting the terminal-side `take()`.
    Fulfilled(T),
    /// The terminal-side `take()` already drained the value. Further
    /// `fulfill()` calls return [`AlreadyFulfilled`].
    Consumed,
}

/// Token the consumer holds to deliver a reply to a `HostRequest`.
///
/// The terminal handler creates a `ResponseToken` when emitting the request,
/// and the consumer fulfills it by calling `token.fulfill(value)`. The
/// terminal then observes the fulfillment via `take()` and formats the
/// reply for PTY emission.
///
/// # Process-local — cannot cross IPC
///
/// `ResponseToken<T>` wraps an `Arc<Mutex<SlotState<T>>>`. The `Arc` is
/// strictly process-local: serializing it across an IPC boundary makes
/// no sense, and the daemon-mode wire protocol (`oriterm_mux::protocol`)
/// deliberately omits any reply token from `NotifyClipboardLoad` /
/// `NotifyHostColorQuery` PDUs. As a result, `HostRequest` round-trips
/// (OSC 52 clipboard load, OSC 4/10/11/12 color queries) work in
/// embedded mode (`oriterm` and `oriterm_mux::backend::embedded`) but
/// are dropped at the daemon-client boundary today. The daemon
/// request-ID + reply-PDU design is tracked in bug-tracker
/// `BUG-11-11`; see `plans/effect-cutover/section-01-migrate-mux-consumer.md`
/// §01.4 for the deferral rationale (Path B — bug filing + cross-link,
/// chosen over an in-scope daemon redesign).
///
/// # Cloning discipline
///
/// The token is `Clone` because the IO thread (polling side) AND the main
/// thread (fulfilling side) both need a handle. Beyond those two legitimate
/// clones, downstream staging buffers MUST move, not clone. Cancellation
/// detection via [`consumer_strong_count`](Self::consumer_strong_count)
/// relies on the strong count dropping to `1` when the main-thread handle
/// is dropped.
///
/// # Single-assignment for the token's lifetime
///
/// Once `fulfill()` has succeeded, the slot transitions through
/// `Pending → Fulfilled(T) → Consumed` (the latter via `take()`). The
/// `Consumed` state is terminal — a second `fulfill()` after `take()`
/// returns [`AlreadyFulfilled`], not silent success. This protects
/// against routing bugs where the main thread's handle leaks into a
/// second fulfill site after the IO thread has already drained the
/// reply.
#[derive(Debug, Clone)]
pub struct ResponseToken<T> {
    slot: Arc<Mutex<SlotState<T>>>,
}

impl<T> ResponseToken<T> {
    /// Create a new unfulfilled token.
    pub fn new() -> Self {
        Self {
            slot: Arc::new(Mutex::new(SlotState::Pending)),
        }
    }

    /// Fulfill the request with the given value.
    ///
    /// Returns `Err(AlreadyFulfilled)` if the slot is `Fulfilled` or
    /// `Consumed`. The first fulfill wins; duplicates AND post-take
    /// fulfills are rejected, so a routing-bug double-fulfill surfaces
    /// loudly at the caller rather than becoming a last-write-wins race
    /// or a silent overwrite of a previously-drained value.
    pub fn fulfill(&self, value: T) -> Result<(), AlreadyFulfilled> {
        let mut slot = self.slot.lock().expect("ResponseToken mutex poisoned");
        match *slot {
            SlotState::Pending => {
                *slot = SlotState::Fulfilled(value);
                Ok(())
            }
            SlotState::Fulfilled(_) | SlotState::Consumed => Err(AlreadyFulfilled),
        }
    }

    /// Take the fulfilled value, if any. Called by the terminal side to
    /// drain the response.
    ///
    /// Transitions `Fulfilled(T) → Consumed`. `Pending` and `Consumed`
    /// states return `None` and leave the slot state unchanged — taking
    /// from `Pending` MUST NOT consume the slot, otherwise a legitimate
    /// future `fulfill()` would be rejected.
    pub fn take(&self) -> Option<T> {
        let mut slot = self.slot.lock().expect("ResponseToken mutex poisoned");
        match &*slot {
            SlotState::Fulfilled(_) => {
                if let SlotState::Fulfilled(value) =
                    std::mem::replace(&mut *slot, SlotState::Consumed)
                {
                    Some(value)
                } else {
                    unreachable!("matched Fulfilled above");
                }
            }
            SlotState::Pending | SlotState::Consumed => None,
        }
    }

    /// Returns `true` if the token has been fulfilled but not yet taken.
    pub fn is_fulfilled(&self) -> bool {
        matches!(
            *self.slot.lock().expect("ResponseToken mutex poisoned"),
            SlotState::Fulfilled(_)
        )
    }

    /// Current strong count of the internal slot `Arc`.
    ///
    /// Used by [`poll`](Self::poll) to detect consumer-dropped cancellation:
    /// when the main-thread handle is dropped, `consumer_strong_count` drops
    /// from `2` (IO thread + main thread) to `1` (IO thread only).
    ///
    /// **Prefer [`poll`](Self::poll) over a direct call** — `poll` performs
    /// the strong-count check while holding the slot lock, eliminating a
    /// race where a fulfill+drop between `take()` and a separate
    /// `consumer_strong_count()` check would mark a fulfilled value as
    /// cancelled. Direct callers MUST acknowledge that the count is a
    /// snapshot — a racing fulfill/drop after the read can change it.
    pub fn consumer_strong_count(&self) -> usize {
        Arc::strong_count(&self.slot)
    }

    /// Atomically poll the token, returning [`TokenPoll::Ready`] /
    /// [`TokenPoll::Pending`] / [`TokenPoll::Cancelled`].
    ///
    /// All three decisions happen under a single slot-lock acquisition,
    /// closing the race that an interleaved fulfill+drop between a
    /// `take()` and a separate `consumer_strong_count()` check would
    /// open: a racing fulfill MUST take the same lock, so the strong
    /// count we read while holding it is decisive — if no consumer
    /// holds the `Arc`, no future fulfill can land on this slot.
    ///
    /// Surfaced by `[high]` (round 2): the previous
    /// split `if let Some(v) = reply.take() { Ready } else if reply.consumer_strong_count() <= 1 { Cancelled } else { Pending }`
    /// pattern in `response_poll/mod.rs` could see `take() == None`
    /// (slot still `Pending`), then race a `fulfill()` + consumer drop,
    /// then read `strong_count == 1` and return `Cancelled` — losing
    /// the fulfilled value the next iteration would otherwise drain.
    pub fn poll(&self) -> TokenPoll<T> {
        let mut slot = self.slot.lock().expect("ResponseToken mutex poisoned");
        match &*slot {
            SlotState::Fulfilled(_) => {
                if let SlotState::Fulfilled(value) =
                    std::mem::replace(&mut *slot, SlotState::Consumed)
                {
                    TokenPoll::Ready(value)
                } else {
                    unreachable!("matched Fulfilled above")
                }
            }
            SlotState::Consumed => TokenPoll::Cancelled,
            SlotState::Pending => {
                // Strong-count check WHILE HOLDING the slot lock — any
                // racing fulfill must wait for us to release the lock,
                // so the count we observe is decisive.
                if Arc::strong_count(&self.slot) <= 1 {
                    TokenPoll::Cancelled
                } else {
                    TokenPoll::Pending
                }
            }
        }
    }
}

/// Result of an atomic [`ResponseToken::poll`] decision.
///
/// Decoupled from the mux-side `PollResult<Effect>` so the token core
/// stays agnostic of the consumer's reply-format step. The consumer
/// matches on this and produces the formatted `Effect` only on
/// [`TokenPoll::Ready`].
#[derive(Debug)]
pub enum TokenPoll<T> {
    /// The token was fulfilled and is now `Consumed` — the value is
    /// returned for downstream formatting.
    Ready(T),
    /// No fulfill yet, but the consumer-side handle is still alive.
    /// Caller should keep the entry and re-poll on the next cycle.
    Pending,
    /// Either the consumer dropped its handle without fulfilling
    /// (`Pending` with `strong_count <= 1`), or the slot was already
    /// drained on a prior poll (`Consumed`). Caller should remove the
    /// entry without emitting a reply.
    Cancelled,
}

impl<T> Default for ResponseToken<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker type for acknowledgement responses where no data is returned.
pub type ResponseFulfilled = ();

// Reply formatting — canonical home for PTY response construction.
//
// The mux `response_poll` module and any test responder consume these
// functions. Centralizing them here prevents SSOT drift between
// reply-formatting sites.

/// Format an OSC 52 clipboard reply as raw PTY bytes.
///
/// The reply echoes back the clipboard character and the base64-encoded text,
/// terminated by the same string terminator the original query used.
pub fn format_clipboard_reply(text: &str, clipboard_char: u8, terminator: &str) -> Vec<u8> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    format!(
        "\x1b]52;{};{}{}",
        clipboard_char as char,
        STANDARD.encode(text.as_bytes()),
        terminator,
    )
    .into_bytes()
}

/// Format an OSC color query reply as raw PTY bytes.
///
/// The reply uses the `XParseColor` `rgb:RR/GG/BB` doubled-nibble format
/// matching `XTerm`'s behavior, terminated by the query's string terminator.
pub fn format_color_reply(color: crate::color::Rgb, prefix: &str, terminator: &str) -> Vec<u8> {
    format!(
        "\x1b]{};rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}{}",
        prefix, color.r, color.r, color.g, color.g, color.b, color.b, terminator,
    )
    .into_bytes()
}

#[cfg(test)]
mod tests;
