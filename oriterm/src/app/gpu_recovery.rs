//! GPU device-loss recovery dispatcher (5.16 — detection + state machine).
//!
//! This is the canonical home for all `App`-side recovery logic.
//!
//! - 5.16.1 lands the *detection* plumbing — the event handler reached when
//!   `wgpu::Device::set_device_lost_callback` fires or the render path
//!   observes `SurfaceError::{Lost, Other, OutOfMemory}`.
//! - 5.16.2 lands the *state-machine scaffold* — the render gate (in
//!   `render_dispatch.rs`), wakeup-source gates (in `event_loop.rs` /
//!   `event_loop_helpers/mod.rs`), single-flight enforcement, and the
//!   ordered teardown invariant scaffold of [`App::recover_gpu`]. The actual
//!   teardown bodies (drop helpers, `GpuState::recreate`, surface rebuild,
//!   pipeline rebuild, renderer rebuild) live in 5.16.3–5.16.8.
//!
//! 5.16.2 makes [`App::recover_gpu`] the canonical recovery entry point.
//! [`App::handle_gpu_device_lost`] coalesces concurrent loss events through
//! [`GpuHealth::next_after_loss`] (single-flight at the state-machine level)
//! and then calls [`App::recover_gpu`] when a transition to `Recovering`
//! actually occurred. Subsequent losses while already `Recovering` skip both
//! the state mutation and the `recover_gpu` call.

#[cfg(any(test, feature = "gpu-tests"))]
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::App;
use crate::gpu::recovery::{GpuHealth, GpuLossReason, WakeupSource, should_post_wakeup};

impl App {
    /// Handle a `TermEvent::GpuDeviceLost` event.
    ///
    /// Called from `user_event` for both the wgpu device-lost callback path
    /// and the render-path escalation paths (`SurfaceError::Lost`,
    /// `SurfaceError::Other`, `SurfaceError::OutOfMemory`). Delegates the
    /// state transition to the pure
    /// [`GpuHealth::next_after_loss`](crate::gpu::recovery::GpuHealth::next_after_loss)
    /// helper so the transition table is unit-testable without an `App`.
    pub(super) fn handle_gpu_device_lost(&mut self, reason: GpuLossReason, message: &str) {
        let attempt_before = match self.gpu_health {
            GpuHealth::Recovering { attempt, .. } => attempt,
            GpuHealth::Healthy { .. } | GpuHealth::Unavailable { .. } => 0,
        };
        log::error!(
            "gpu_health::recover trigger=event reason={reason:?} attempt={attempt_before} \
             windows={} message=\"{message}\"",
            self.windows.len(),
        );

        let now = Instant::now();
        let Some(next) = self.gpu_health.next_after_loss(reason, message, now) else {
            log::warn!(
                "gpu_health::coalesce no transition: current_epoch={:?} reason={reason:?}",
                self.gpu_health.epoch(),
            );
            return;
        };
        self.gpu_health = next;

        match &self.gpu_health {
            GpuHealth::Unavailable { .. } => {
                log::error!(
                    "gpu_health::unavailable since={:?} message=\"{}\" total_attempts={attempt_before}",
                    self.gpu_health.unavailable_since(),
                    self.gpu_health.unavailable_message().unwrap_or(""),
                );
                // Unavailable is terminal until manual retry (5.16.10) — do
                // NOT call recover_gpu(). Wakeups are gated; the event loop
                // sleeps until the user triggers F5.
            }
            GpuHealth::Recovering { epoch, .. } => {
                log::warn!(
                    "gpu_health::recovering epoch={epoch} attempt=0 since={:?}",
                    self.gpu_health.recovering_since(),
                );
                // Single-flight: only the first transition Healthy →
                // Recovering reaches this branch (next_after_loss coalesces
                // subsequent losses to None). Run the recovery state machine.
                self.recover_gpu();
            }
            GpuHealth::Healthy { .. } => {
                debug_assert!(false, "next_after_loss returned Healthy from a loss event");
            }
        }
    }

    /// Run the recovery state machine.
    ///
    /// 5.16.2 scaffold: implements steps 1 and 7 of the ordered teardown
    /// invariant from the plan, with steps 2–6 stubbed pending 5.16.3–5.16.8.
    /// The function is invoked exactly once per `Healthy → Recovering`
    /// transition (single-flight is enforced by `handle_gpu_device_lost`'s
    /// delegation to `next_after_loss`, which coalesces a second loss into
    /// `None`).
    ///
    /// Ordering invariant (each step asserted with `debug_assert!` at the
    /// call site, mirroring the plan's I1 invariant):
    ///
    /// 1. ✅ State already set to `Recovering` (caller did this via
    ///    `next_after_loss`); render gate is now closed.
    /// 2. ⏳ Drop in-flight `SurfaceTexture`s and `wgpu::Surface<'static>`
    ///    fields per window — **5.16.3**.
    /// 3. ⏳ Drop per-window `WindowRenderer`s, preserving `FontCollection`
    ///    + `UiFontSizes` on the stack — **5.16.3**.
    /// 4. ⏳ Drop `self.pipelines` — **5.16.3**.
    /// 5. ⏳ Drop `self.gpu` (`GpuState`) — **5.16.3**.
    /// 6. ⏳ `GpuState::recreate(...)` → recreate per-window surfaces →
    ///    rebuild `GpuPipelines` → rebuild every `WindowRenderer` —
    ///    **5.16.4–5.16.8**.
    /// 7. ✅ Bump epoch, set `gpu_health = Healthy { epoch }`, queue redraw
    ///    on every window. The 5.16.2 stub does this immediately — until
    ///    5.16.3–5.16.8 land the recreate path, the stub immediately marks
    ///    recovery "complete" so the rest of the system can be exercised.
    ///    Real recovery semantics land with 5.16.4.
    pub(super) fn recover_gpu(&mut self) {
        // Step 1: precondition — state must be Recovering, set by the caller.
        debug_assert!(
            !self.gpu_health.is_healthy(),
            "recover_gpu called from Healthy state — single-flight violation"
        );

        let pre_epoch = self.gpu_health.epoch().unwrap_or(0);
        log::warn!(
            "recover_gpu (5.16.2 stub): pre_epoch={pre_epoch} \
             windows={} dialogs={} \
             — full teardown + recreate land in 5.16.3–5.16.8",
            self.windows.len(),
            self.dialogs.len(),
        );

        // Steps 2–6 are stubbed for 5.16.2. The teardown helpers
        // (drop_per_frame_state, drop_per_window_gpu_state,
        // drop_per_dialog_gpu_state, drop_window_surfaces, drop_pipelines,
        // drop_gpu_state) and the rebuild (GpuState::recreate, surface
        // rebuild, GpuPipelines::new, WindowRenderer::new × N) land in
        // 5.16.3–5.16.8. Until they exist, the stub bumps the epoch and
        // returns to Healthy so the state-machine plumbing is testable.

        // Step 7: bump epoch and return to Healthy. Queue a redraw on every
        // window so the gate-released first frame is a full repaint.
        let new_epoch = pre_epoch.wrapping_add(1);
        self.gpu_health = GpuHealth::Healthy { epoch: new_epoch };
        for ctx in self.windows.values_mut() {
            ctx.root.invalidation_mut().invalidate_all();
            ctx.root.mark_dirty();
            ctx.window.window().request_redraw();
        }
        for ctx in self.dialogs.values_mut() {
            ctx.root.invalidation_mut().invalidate_all();
            ctx.root.mark_dirty();
            ctx.window.request_redraw();
        }
        log::info!(
            "gpu_health::recover stub-complete epoch={new_epoch} \
             windows_marked_dirty={} dialogs_marked_dirty={}",
            self.windows.len(),
            self.dialogs.len(),
        );
    }

    /// Returns true when the animation-tier wakeup sources are allowed to
    /// post redraws this frame.
    ///
    /// Centralizes the exhaustive per-source check for the 5.16.2 wakeup
    /// gate. Every animation-tier `WakeupSource` variant must appear in
    /// the match below — adding a new variant forces an explicit policy
    /// decision (gated vs. always-on) and avoids silent drift between the
    /// enum and the call sites. Currently every variant shares the
    /// uniform "gated when not Healthy" policy, but the per-source reads
    /// preserve the SSOT contract that 5.16.13's exhaustive
    /// (state, source) test matrix asserts.
    pub(super) fn allow_animation_wakeups(&self) -> bool {
        let sources = [
            WakeupSource::TabSlide,
            WakeupSource::LayerAnimator,
            WakeupSource::RenderScheduler,
            WakeupSource::VisualStateAnimator,
            WakeupSource::AutoScroll,
            WakeupSource::CursorHoverHold,
        ];
        sources
            .iter()
            .all(|&src| should_post_wakeup(&self.gpu_health, src))
    }

    /// Synthetic test-only device-loss trigger.
    ///
    /// Posts the same effect the wgpu device-lost callback would: bumps
    /// the cross-thread `device_lost_signal` counter and routes a
    /// `TermEvent::GpuDeviceLost` through the event proxy. Required by the
    /// 5.16.13 state-machine tests (which land alongside the 5.16.2 render
    /// gate) so they can exercise the full recovery path without a real
    /// GPU crash.
    ///
    /// Gated behind `cfg(any(test, feature = "gpu-tests"))` so the helper
    /// is unreachable from release builds. The
    /// `#[expect(dead_code, ...)]` attribute reflects that 5.16.13's
    /// integration test is the one true caller — once that test lands the
    /// expect will become unfulfilled and force its own removal.
    #[cfg(any(test, feature = "gpu-tests"))]
    #[expect(
        dead_code,
        reason = "5.16.13 state-machine integration test (gpu-tests gated, lands with 5.16.2's render gate) is the one true caller; helper exists in 5.16.1 so the API surface is reviewable up front"
    )]
    pub(crate) fn trigger_test_device_loss(&mut self, reason: GpuLossReason, message: String) {
        self.device_lost_signal.fetch_add(1, Ordering::Release);
        self.event_proxy
            .send(crate::event::TermEvent::GpuDeviceLost { reason, message });
    }
}
