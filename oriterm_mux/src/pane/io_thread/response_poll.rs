//! Reply-return polling for host-request response tokens.
//!
//! When a consumer fulfills a `ResponseToken` (e.g. providing clipboard text),
//! the IO thread polls pending responses and formats the reply as a PTY write.
//!
//! During the legacy phase (`LegacyEventSink`), `pending_responses` is always
//! empty — the legacy adapter handles the round-trip via closures. This module
//! activates when consumers migrate to `QueueingEffectSink` (in
//! `plans/effect-cutover/`).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{Effect, HostRequest, PendingResponse, PtyEffect, PtyWriteKind};

use super::PaneIoThread;

impl<S: EffectSink> PaneIoThread<S> {
    /// Register a host-request's response token for asynchronous polling.
    ///
    /// Creates a `PendingResponse` that polls the request's `ResponseToken`
    /// and, when fulfilled, produces an `Effect::Pty(PtyEffect::Write { .. })`
    /// with the correctly formatted reply.
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
                        let encoded = STANDARD.encode(text.as_bytes());
                        let response = format!(
                            "\x1b]52;{};{}{}",
                            clipboard_char as char, encoded, terminator,
                        );
                        Some(Effect::Pty(PtyEffect::Write {
                            bytes: response.into_bytes(),
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
                        let response = format!(
                            "\x1b]{};rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}{}",
                            prefix,
                            color.r,
                            color.r,
                            color.g,
                            color.g,
                            color.b,
                            color.b,
                            terminator,
                        );
                        Some(Effect::Pty(PtyEffect::Write {
                            bytes: response.into_bytes(),
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
    /// entries are removed.
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
                self.pending_responses.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
}
