//! Reply-return infrastructure for `HostRequest` response tokens.
//!
//! When a VTE handler emits a `HostRequest` (e.g. `ClipboardLoad`, `ColorQuery`),
//! the consumer fulfills the `ResponseToken`. A `PendingResponse` wraps the token
//! polling + reply formatting into a type-erased callback that the IO thread polls.
//!
//! # Cancellation detection
//!
//! When the main-thread consumer drops its `ResponseToken` handle without
//! fulfilling (user dismissed the permission prompt, overlay closed, pane
//! closed mid-request), the pending entry would leak forever if polled purely
//! by `reply.take()`. To detect cancellation, the registration closure captures
//! a clone of the `ResponseToken` and probes
//! [`ResponseToken::consumer_strong_count`] after every `take()` that returns
//! `None`: if the strong count has dropped to `1` (only the IO thread's clone
//! remains), the consumer is gone and the closure returns
//! [`PollResult::Cancelled`]. The IO thread then removes the entry without
//! emitting a reply.

use super::Effect;

/// Outcome of polling a [`PendingResponse`].
///
/// Replaces the prior `Option<Effect>` return shape so the IO thread can
/// distinguish three states:
///
/// - [`PollResult::Pending`]: consumer is alive but has not yet fulfilled.
///   Keep the entry; poll again on the next tick.
/// - [`PollResult::Ready`]: consumer fulfilled; emit the effect and remove
///   the entry.
/// - [`PollResult::Cancelled`]: consumer dropped without fulfilling. Remove
///   the entry WITHOUT emitting a reply.
#[derive(Debug)]
pub enum PollResult {
    /// Consumer has not fulfilled yet. Keep polling.
    Pending,
    /// Consumer fulfilled; push this effect and remove the entry.
    Ready(Effect),
    /// Consumer dropped its handle unfulfilled; remove the entry without
    /// emitting a reply.
    Cancelled,
}

/// A response token awaiting fulfillment + a formatter that turns the
/// response value into `Effect::Pty` bytes.
///
/// Type-erased so the IO thread can poll heterogeneous response types
/// (clipboard load returns `String`, color query returns `Rgb`) in a
/// single `Vec<PendingResponse>`.
pub struct PendingResponse {
    poll: Box<dyn FnMut() -> PollResult + Send>,
}

impl PendingResponse {
    /// Create a new pending response with the given poll closure.
    ///
    /// The closure should call `token.take()` and, if fulfilled, return
    /// [`PollResult::Ready`] with the formatted reply effect. If not
    /// fulfilled, it should probe the token's `consumer_strong_count()` and
    /// return [`PollResult::Cancelled`] when the consumer handle is gone, or
    /// [`PollResult::Pending`] when the consumer is still alive.
    pub fn new(poll: Box<dyn FnMut() -> PollResult + Send>) -> Self {
        Self { poll }
    }

    /// Poll the token.
    ///
    /// See [`PollResult`] for the return-value contract.
    pub fn poll(&mut self) -> PollResult {
        (self.poll)()
    }
}

impl std::fmt::Debug for PendingResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingResponse").finish_non_exhaustive()
    }
}
