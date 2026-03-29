//! Animation frame types — timing pulses and frame request flags.
//!
//! [`AnimFrameEvent`] delivers timing information to widgets that requested
//! an animation frame. [`FrameRequestFlags`] collects frame/paint requests
//! from widgets during a dispatch pass.

use std::cell::Cell;
use std::time::Instant;

/// Timing pulse delivered to widgets that requested an animation frame.
///
/// Widgets receive this event after calling `ctx.request_anim_frame()`.
/// The event loop delivers one `AnimFrameEvent` per frame to each
/// requesting widget until the widget stops requesting frames.
#[derive(Debug, Clone, Copy)]
pub struct AnimFrameEvent {
    /// Nanoseconds since the last `AnimFrameEvent` delivered to this widget.
    /// `0` on the first frame after transitioning from idle to animating.
    pub delta_nanos: u64,
    /// Absolute timestamp for this frame.
    pub now: Instant,
}

/// Output flags for widget animation frame and repaint requests.
///
/// Shared between parent and child contexts via `Option<&FrameRequestFlags>`.
/// When any widget in the dispatch chain requests an animation frame or
/// repaint, the flag is visible to the top-level caller without explicit
/// merge logic in container widgets.
///
/// The framework reads these flags after each widget method call and
/// forwards them to the [`RenderScheduler`](super::scheduler) (Section 05.5).
#[derive(Debug)]
pub struct FrameRequestFlags {
    anim_frame: Cell<bool>,
    paint: Cell<bool>,
}

impl FrameRequestFlags {
    /// Creates a new set of flags with no requests.
    pub fn new() -> Self {
        Self {
            anim_frame: Cell::new(false),
            paint: Cell::new(false),
        }
    }

    /// Request an animation frame on the next vsync.
    ///
    /// The widget will receive an [`AnimFrameEvent`] with the time delta
    /// since the last frame.
    pub fn request_anim_frame(&self) {
        self.anim_frame.set(true);
    }

    /// Request a repaint without an animation frame.
    pub fn request_paint(&self) {
        self.paint.set(true);
    }

    /// Whether an animation frame was requested.
    pub fn anim_frame_requested(&self) -> bool {
        self.anim_frame.get()
    }

    /// Whether a repaint was requested.
    pub fn paint_requested(&self) -> bool {
        self.paint.get()
    }

    /// Resets all flags to `false`.
    pub fn reset(&self) {
        self.anim_frame.set(false);
        self.paint.set(false);
    }
}

impl Default for FrameRequestFlags {
    fn default() -> Self {
        Self::new()
    }
}
