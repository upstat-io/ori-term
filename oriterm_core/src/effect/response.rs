//! Reply-return infrastructure for `HostRequest` response tokens.
//!
//! When a VTE handler emits a `HostRequest` (e.g. `ClipboardLoad`, `ColorQuery`),
//! the consumer fulfills the `ResponseToken`. A `PendingResponse` wraps the token
//! polling + reply formatting into a type-erased callback that the IO thread polls.
//!
//! During the legacy phase (`LegacyEventSink`), the reply-return path is handled
//! entirely by the legacy closure — `PendingResponse` is dormant. It activates
//! when consumers migrate to `QueueingEffectSink` (in `plans/effect-cutover/`).

use super::Effect;

/// A response token awaiting fulfillment + a formatter that turns the
/// response value into `Effect::Pty` bytes.
///
/// Type-erased so the IO thread can poll heterogeneous response types
/// (clipboard load returns `String`, color query returns `Rgb`) in a
/// single `Vec<PendingResponse>`.
pub struct PendingResponse {
    poll: Box<dyn FnMut() -> Option<Effect> + Send>,
}

impl PendingResponse {
    /// Create a new pending response with the given poll closure.
    ///
    /// The closure should call `token.take()` and, if fulfilled, format
    /// the response into `Some(Effect::Pty(PtyEffect::Write { ... }))`.
    pub fn new(poll: Box<dyn FnMut() -> Option<Effect> + Send>) -> Self {
        Self { poll }
    }

    /// Poll the token. Returns `Some(Effect::Pty(...))` if fulfilled.
    pub fn poll(&mut self) -> Option<Effect> {
        (self.poll)()
    }
}

impl std::fmt::Debug for PendingResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingResponse").finish_non_exhaustive()
    }
}
