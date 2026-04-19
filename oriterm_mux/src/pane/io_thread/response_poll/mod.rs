//! Reply-return polling for host-request response tokens.
//!
//! When a consumer fulfills a `ResponseToken` (e.g. providing clipboard text),
//! the IO thread polls pending responses and formats the reply as a PTY write.
//!
//! When the consumer DROPS the `ResponseToken` without fulfilling (user
//! dismisses the overlay, pane closes mid-request), the IO thread detects the
//! cancelled state via `Arc::strong_count` on the token's inner slot and
//! removes the entry without emitting a reply.

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{
    Effect, HostRequest, PendingResponse, PollResult, PtyEffect, PtyWriteKind, TokenPoll,
    format_clipboard_reply, format_color_reply,
};

use super::PaneIoThread;

impl<S: EffectSink> PaneIoThread<S> {
    /// Register a host-request's response token for asynchronous polling.
    ///
    /// Creates a `PendingResponse` whose poll closure:
    /// 1. Attempts to `take()` a fulfilled value from the token; on `Some`,
    ///    returns [`PollResult::Ready`] with the formatted PTY reply.
    /// 2. On `None`, checks `reply.consumer_strong_count() <= 1` — when the
    ///    main-thread consumer has dropped its handle, returns
    ///    [`PollResult::Cancelled`] so the IO thread can remove the entry.
    /// 3. Otherwise returns [`PollResult::Pending`].
    ///
    /// Reply formatting is delegated to the canonical functions in
    /// `oriterm_core::effect` (shared with the legacy adapter to prevent
    /// SSOT drift).
    pub(super) fn register_host_request_response(&mut self, request: HostRequest) {
        match request {
            HostRequest::ClipboardLoad {
                clipboard_char,
                terminator,
                reply,
                ..
            } => {
                self.pending_responses
                    .push(PendingResponse::new(Box::new(move || match reply.poll() {
                        TokenPoll::Ready(text) => {
                            let bytes = format_clipboard_reply(&text, clipboard_char, &terminator);
                            PollResult::Ready(Effect::Pty(PtyEffect::Write {
                                bytes,
                                kind: PtyWriteKind::Other,
                            }))
                        }
                        TokenPoll::Cancelled => PollResult::Cancelled,
                        TokenPoll::Pending => PollResult::Pending,
                    })));
            }
            HostRequest::ColorQuery {
                prefix,
                terminator,
                reply,
                ..
            } => {
                self.pending_responses
                    .push(PendingResponse::new(Box::new(move || match reply.poll() {
                        TokenPoll::Ready(color) => {
                            let bytes = format_color_reply(color, &prefix, &terminator);
                            PollResult::Ready(Effect::Pty(PtyEffect::Write {
                                bytes,
                                kind: PtyWriteKind::Other,
                            }))
                        }
                        TokenPoll::Cancelled => PollResult::Cancelled,
                        TokenPoll::Pending => PollResult::Pending,
                    })));
            }
        }
    }

    /// Poll pending host-request responses and push fulfilled replies.
    ///
    /// Iterates `pending_responses`, polls each entry.
    /// - `PollResult::Ready(effect)` → push through the terminal's effect
    ///   sink and remove the entry (FIFO order via `Vec::remove`).
    /// - `PollResult::Cancelled` → remove the entry WITHOUT pushing a reply
    ///   (consumer dropped its handle).
    /// - `PollResult::Pending` → retain the entry for the next poll.
    pub(super) fn poll_pending_responses(&mut self) {
        if self.pending_responses.is_empty() {
            return;
        }
        let mut i = 0;
        while i < self.pending_responses.len() {
            match self.pending_responses[i].poll() {
                PollResult::Ready(effect) => {
                    self.terminal.effect_sink().push(effect);
                    // Use `remove` (not `swap_remove`) to preserve FIFO order.
                    self.pending_responses.remove(i);
                }
                PollResult::Cancelled => {
                    self.pending_responses.remove(i);
                }
                PollResult::Pending => {
                    i += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
