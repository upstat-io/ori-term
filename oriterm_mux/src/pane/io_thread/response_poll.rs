//! Reply-return polling for host-request response tokens.
//!
//! When a consumer fulfills a `ResponseToken` (e.g. providing clipboard text),
//! the IO thread polls pending responses and formats the reply as a PTY write.
//!
//! During the legacy phase (`LegacyEventSink`), `pending_responses` is always
//! empty — the legacy adapter handles the round-trip via closures. This module
//! activates when consumers migrate to `QueueingEffectSink` (in
//! `plans/effect-cutover/`).

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{
    Effect, HostRequest, PendingResponse, PtyEffect, PtyWriteKind, format_clipboard_reply,
    format_color_reply,
};

use super::PaneIoThread;

impl<S: EffectSink> PaneIoThread<S> {
    /// Register a host-request's response token for asynchronous polling.
    ///
    /// Creates a `PendingResponse` that polls the request's `ResponseToken`
    /// and, when fulfilled, produces an `Effect::Pty(PtyEffect::Write { .. })`
    /// with the correctly formatted reply. Formatting is delegated to the
    /// canonical functions in `oriterm_core::effect` (shared with the legacy
    /// adapter to prevent SSOT drift).
    ///
    /// # Dormant during legacy phase
    ///
    /// During the legacy phase, this method is never called — the legacy
    /// adapter handles the round-trip internally. It activates when consumers
    /// migrate to subscribe to `Effect::HostRequest` directly.
    #[allow(
        dead_code,
        reason = "dormant during legacy phase; activates at effect-cutover"
    )]
    pub(super) fn register_host_request_response(&mut self, request: HostRequest) {
        match request {
            HostRequest::ClipboardLoad {
                clipboard_char,
                terminator,
                reply,
                ..
            } => {
                self.pending_responses
                    .push(PendingResponse::new(Box::new(move || {
                        let text = reply.take()?;
                        let bytes = format_clipboard_reply(&text, clipboard_char, &terminator);
                        Some(Effect::Pty(PtyEffect::Write {
                            bytes,
                            kind: PtyWriteKind::Other,
                        }))
                    })));
            }
            HostRequest::ColorQuery {
                prefix,
                terminator,
                reply,
                ..
            } => {
                self.pending_responses
                    .push(PendingResponse::new(Box::new(move || {
                        let color = reply.take()?;
                        let bytes = format_color_reply(color, &prefix, &terminator);
                        Some(Effect::Pty(PtyEffect::Write {
                            bytes,
                            kind: PtyWriteKind::Other,
                        }))
                    })));
            }
        }
    }

    /// Poll pending host-request responses and push fulfilled replies.
    ///
    /// Iterates `pending_responses`, polls each token. Fulfilled responses
    /// produce `Effect::Pty(PtyEffect::Write { .. })` which is pushed through
    /// the terminal's effect sink for the consumer to process. Fulfilled
    /// entries are removed in FIFO order (preserving insertion ordering per
    /// the `EffectSink` ordering contract).
    ///
    /// During the legacy phase, `pending_responses` is always empty — this
    /// is effectively a no-op.
    pub(super) fn poll_pending_responses(&mut self) {
        if self.pending_responses.is_empty() {
            return;
        }
        let mut i = 0;
        while i < self.pending_responses.len() {
            if let Some(effect) = self.pending_responses[i].poll() {
                self.terminal.effect_sink().push(effect);
                // Use `remove` (not `swap_remove`) to preserve FIFO order.
                self.pending_responses.remove(i);
            } else {
                i += 1;
            }
        }
    }
}
