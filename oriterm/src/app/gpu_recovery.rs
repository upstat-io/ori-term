//! GPU device-loss recovery dispatcher (5.16 detection plumbing).
//!
//! This is the canonical home for all `App`-side recovery logic. 5.16.1
//! lands the *detection* plumbing — the event handler reached when the
//! `wgpu::Device::set_device_lost_callback` fires or when the render path
//! observes `SurfaceError::{Lost, Other, OutOfMemory}`. The actual recovery
//! state machine (`App::recover_gpu()`, ordered teardown, multi-window
//! adapter validation, pipeline rebuild, render gate) lands in 5.16.2.
//!
//! For 5.16.1, the handler:
//!
//! 1. Logs the loss with the structured format described in 5.16.12.
//! 2. Records the state transition on `App::gpu_health` so the upcoming
//!    render gate has a target to read.
//! 3. For `OutOfMemory`, transitions straight to `Unavailable` per 5.16.10
//!    (OOM is terminal — never retried).
//! 4. For all other reasons, transitions to `Recovering { attempt: 0, ... }`
//!    awaiting the 5.16.2 state machine.

#[cfg(any(test, feature = "gpu-tests"))]
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::App;
use crate::gpu::recovery::{GpuHealth, GpuLossReason};

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
            GpuHealth::Unavailable { .. } => log::error!(
                "gpu_health::unavailable since={:?} message=\"{}\" total_attempts={attempt_before}",
                self.gpu_health.unavailable_since(),
                self.gpu_health.unavailable_message().unwrap_or(""),
            ),
            GpuHealth::Recovering { epoch, .. } => log::warn!(
                "gpu_health::recovering epoch={epoch} attempt=0 since={:?} \
                 (state-machine implementation lands in 5.16.2)",
                self.gpu_health.recovering_since(),
            ),
            GpuHealth::Healthy { .. } => {
                debug_assert!(false, "next_after_loss returned Healthy from a loss event");
            }
        }

        // Use the canonical predicate at least once on this path so 5.16.2's
        // gate-readiness check stays wired into the dispatcher even before
        // the gate itself lands.
        debug_assert!(
            !self.gpu_health.is_healthy(),
            "post-loss state must not be Healthy"
        );
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
