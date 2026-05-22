//! Top-level effect family enum.

use super::families::{HostEffect, HostRequest, PresentationEffect, PtyEffect, UiEffect};
use crate::image::worker_pipeline::ImageDecodeRequest;

/// Top-level effect family routed via the [`EffectSink`](super::EffectSink).
///
/// The five families partition by purpose: `Pty` for PTY writes, `Host` for
/// fire-and-forget platform calls, `HostRequest` for typed request/response
/// (NOT closures), `Ui` for UI hints, `Presentation` for sync gates.
///
/// **Design decision — no closures**: Alacritty's `Event` enum uses
/// `Arc<dyn Fn>` closures for `ClipboardLoad`, `ColorRequest`, and
/// `TextAreaSizeRequest`. We deliberately diverge: closures capture
/// formatter state from the OSC handler and leak formatting logic out of
/// `oriterm_core`, preventing tests from cleanly observing the request
/// parameters. `ResponseToken` replaces closures with plain data.
#[derive(Debug, Clone)]
pub enum Effect {
    /// Write bytes back to the PTY (device replies, mouse/keyboard events).
    Pty(PtyEffect),
    /// Fire-and-forget host platform calls (bell, title, clipboard store, etc.).
    Host(HostEffect),
    /// Typed request/response replacing closure-based Event variants.
    HostRequest(HostRequest),
    /// UI hints (cursor blink state, mouse cursor dirty).
    Ui(UiEffect),
    /// Sync output gates (Mode 2026 begin/commit/abort).
    Presentation(PresentationEffect),
    /// Off-IO-thread image decode request. Consumed by
    /// `oriterm_mux::pane::io_thread::image_worker::ImageWorker::enqueue`;
    /// worker thread calls `oriterm_core::image::worker_pipeline::run_image_decode`,
    /// pushes the result to the result channel; IO thread drains and calls
    /// `Term::apply_decoded_image` to land the decoded RGBA + emit the kitty
    /// reply in sequencer order.
    /// See: bug-tracker/plans/BUG-06-088/
    ImageDecode(ImageDecodeRequest),
}
