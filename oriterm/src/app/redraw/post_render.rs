//! Post-render cleanup: surface error recovery, blink state, IME.

use std::sync::atomic::Ordering;

use super::App;
use crate::event::TermEvent;
use crate::gpu::SurfaceError;
use crate::gpu::recovery::GpuLossReason;

impl App {
    /// Finalize a render pass: recover from surface errors, update blink
    /// state, and reposition the IME candidate window.
    ///
    /// Called by both the single-pane and multi-pane redraw paths after
    /// GPU submission completes. `cursor_pos` is `Some` only on the
    /// single-pane path where cursor-move detection resets the blink timer.
    pub(super) fn finish_render(
        &mut self,
        render_result: Result<(), SurfaceError>,
        blinking_now: bool,
        cursor_pos: Option<(usize, usize)>,
    ) {
        // Post-present device-lost signal check (5.16.1).
        //
        // The wgpu device-lost callback bumps `device_lost_signal` from a
        // wgpu-internal thread the moment the device dies. If the bump
        // landed *between* this frame's `submit()` and `present()` (or just
        // after present, but before `user_event` has had a chance to
        // dispatch the queued `TermEvent::GpuDeviceLost`), comparing the
        // current counter against the last-seen value tells us this frame's
        // pixels are garbage. 5.16.2's render gate uses the same signal to
        // short-circuit before re-entering submit. For 5.16.1 the check
        // only logs.
        let signal = self.device_lost_signal.load(Ordering::Acquire);
        if signal != self.last_seen_device_lost_signal {
            log::warn!(
                "device-lost callback fired since last frame: \
                 signal {} -> {}, current gpu_health.epoch={:?}",
                self.last_seen_device_lost_signal,
                signal,
                self.gpu_health.epoch(),
            );
            self.last_seen_device_lost_signal = signal;
        }

        // Surface error dispatch (5.16.1 detection plumbing).
        // - `Outdated`  → benign reconfigure (existing path).
        // - `Lost`      → escalate to the recovery state machine via the
        //                 `TermEvent::GpuDeviceLost` event.
        // - `Other`     → soft device loss; carries the wgpu detail string.
        // - `OutOfMemory` → escalate; the recovery state machine will
        //                 transition straight to `Unavailable` per 5.16.10.
        // - `Timeout`   → retryable, log and let the next frame retry.
        match render_result {
            Ok(()) => log::trace!("render ok"),
            Err(SurfaceError::Outdated) => {
                log::warn!("surface outdated, reconfiguring");
                let Some(gpu) = self.gpu.as_ref() else { return };
                if let Some(ctx) = self
                    .focused_window_id
                    .and_then(|id| self.windows.get_mut(&id))
                {
                    let (w, h) = ctx.window.size_px();
                    ctx.window.resize_surface(w, h, gpu);
                    ctx.window.apply_pending_surface_resize(gpu);
                }
            }
            Err(SurfaceError::Lost) => {
                log::error!("surface lost, dispatching GPU recovery");
                self.event_proxy.send(TermEvent::GpuDeviceLost {
                    reason: GpuLossReason::Unknown,
                    message: "SurfaceError::Lost from get_current_texture".to_string(),
                });
            }
            Err(SurfaceError::Other(detail)) => {
                log::error!("surface other error (soft device loss): {detail}");
                self.event_proxy.send(TermEvent::GpuDeviceLost {
                    reason: GpuLossReason::SurfaceOther,
                    message: format!("SurfaceError::Other: {detail}"),
                });
            }
            Err(SurfaceError::OutOfMemory) => {
                log::error!("GPU out of memory, dispatching to Unavailable");
                self.event_proxy.send(TermEvent::GpuDeviceLost {
                    reason: GpuLossReason::OutOfMemory,
                    message: "SurfaceError::OutOfMemory from get_current_texture".to_string(),
                });
            }
            Err(SurfaceError::Timeout) => {
                log::warn!("surface acquisition timed out, retrying next frame");
            }
        }

        // Reset blink to visible when the cursor moves (PTY output moved it).
        if let Some(pos) = cursor_pos {
            if pos != self.last_cursor_pos {
                self.last_cursor_pos = pos;
                self.reset_cursor_blink();
            }
        }

        // Blink state transition: reset on off→on edge.
        if blinking_now && !self.blinking_active {
            self.reset_cursor_blink();
        }
        // Formula: cursor_should_blink(). Two sites exist by design:
        // focus handler (event_loop.rs) for immediate state, post_render for frame state.
        self.blinking_active = self.cursor_should_blink(blinking_now);

        // Keep the IME candidate window positioned at the terminal cursor.
        self.update_ime_cursor_area();
    }
}
