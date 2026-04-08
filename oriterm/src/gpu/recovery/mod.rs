//! GPU device-loss recovery state machine.
//!
//! This module is the canonical home for the recovery state machine that
//! coordinates GPU device-loss handling across all windows. It owns the
//! [`GpuHealth`] enum that lives on `App` and the [`GpuLossReason`] enum that
//! flows through `TermEvent::GpuDeviceLost`.
//!
//! # Mission
//!
//! When a laptop user closes the lid, when a discrete GPU is power-gated, when
//! a display driver crashes, or when the OS forces a TDR — `oriterm` does not
//! crash, all PTY processes keep running, scrollback is preserved
//! byte-for-byte, selections and search state are preserved, and the user's
//! next keystroke either renders normally (recovery succeeded) or hits a clear
//! "GPU unavailable" indication (recovery exhausted).
//!
//! # Section split
//!
//! - 5.16 (this scaffold + the core engine) — detection, state machine,
//!   teardown, recreation, first frame, backoff, minimal Unavailable UX,
//!   logging, core tests.
//! - 5.17 — recovery-module scaffold expansion, terminal-state preservation
//!   invariants, canonical-snapshot stress test, exhaustive enum coverage.
//! - 5.18 — cross-section integrations, deferred contracts, manual destructive
//!   matrix.
//!
//! 5.16.1 lands the *detection plumbing* only: this module, the [`GpuHealth`]
//! field on `App`, the `TermEvent::GpuDeviceLost` variant, the device-lost
//! callback registration, and the `SurfaceError::{Outdated, Lost, Other}`
//! split. The actual `App::recover_gpu()` state machine lands in 5.16.2.

mod outcome;
mod wakeup;

pub(crate) use outcome::{RenderOutcome, gate_outcome};
pub(crate) use wakeup::{WakeupSource, should_post_wakeup};

use std::time::Instant;

/// Reason a GPU device was lost.
///
/// This is `oriterm`'s local enum (not `wgpu_types::DeviceLostReason`) so the
/// recovery dispatcher can carry richer cause information than wgpu exposes —
/// in particular, the `OutOfMemory` and `SurfaceOther` variants that originate
/// from `SurfaceError` rather than the wgpu device-lost callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GpuLossReason {
    /// Driver-reported device loss with no further detail. Maps from
    /// `wgpu::DeviceLostReason::Unknown`.
    Unknown,
    /// Device was explicitly destroyed (e.g. via `Device::destroy()`). Maps
    /// from `wgpu::DeviceLostReason::Destroyed`. Used by the synthetic
    /// test-loss path so tests can distinguish "real" loss from "we asked
    /// for it".
    Destroyed,
    /// GPU ran out of memory. Originates from `SurfaceError::OutOfMemory` and
    /// is the only reason that transitions straight to `Unavailable` without
    /// retrying — see 5.16.10.
    OutOfMemory,
    /// `SurfaceError::Other(detail)` escalated to soft device loss. Surfaces
    /// from real-world `WSLg` / DX12 driver bugs that report device-lost
    /// details via wgpu error scopes instead of `SurfaceError::Lost`.
    SurfaceOther,
}

impl GpuLossReason {
    /// Map a `wgpu_types::DeviceLostReason` into the local enum.
    ///
    /// Used by the device-lost callback registered on every fresh
    /// `wgpu::Device` to translate the wgpu reason into the richer reason
    /// the recovery dispatcher carries through `TermEvent::GpuDeviceLost`.
    pub(crate) fn from_wgpu(reason: wgpu::DeviceLostReason) -> Self {
        match reason {
            wgpu::DeviceLostReason::Destroyed => Self::Destroyed,
            wgpu::DeviceLostReason::Unknown => Self::Unknown,
        }
    }
}

/// Global GPU device health on `App`.
///
/// Tracks whether the GPU is currently usable. The render gate consults this
/// before submitting any draw work — `Recovering` and `Unavailable` block
/// rendering. The state machine that mutates this enum lives in
/// `App::recover_gpu()` (5.16.2). 5.16.1 only adds the field with the default
/// `Healthy { epoch: 0 }` so the detection plumbing has a target to read.
///
/// The `epoch` counter monotonically increments on every successful recreate
/// so other state can detect stale epochs without pointer comparison.
#[derive(Debug, Clone)]
pub(crate) enum GpuHealth {
    /// Device is healthy. Render path runs normally.
    Healthy {
        /// Monotonic counter incremented on every successful recreate.
        epoch: u64,
    },
    /// Recovery is in progress. Render path is gated.
    Recovering {
        /// Epoch of the device that was lost. The new device gets `epoch + 1`
        /// on successful recovery.
        epoch: u64,
        /// Current attempt counter (0-indexed). Reset to 0 after 30s of clean
        /// operation.
        attempt: u8,
        /// When the current `Recovering` state was entered. Used by the
        /// backoff scheduler.
        since: Instant,
    },
    /// Recovery has failed past the budget. Render path is gated; user must
    /// trigger a manual retry (F5) for another attempt.
    Unavailable {
        /// Last error message recorded by the recovery state machine.
        last_error: String,
        /// When the `Unavailable` state was entered. Used by the title-bar UX
        /// and the user-attention one-shot in 5.16.10.
        since: Instant,
    },
}

impl GpuHealth {
    /// Construct the default healthy state at epoch 0.
    pub(crate) fn new() -> Self {
        Self::Healthy { epoch: 0 }
    }

    /// Returns the current epoch counter.
    ///
    /// Both `Healthy` and `Recovering` carry an epoch; `Unavailable` does not
    /// (the device is gone — there is no current epoch). Returns `None` for
    /// `Unavailable`.
    pub(crate) fn epoch(&self) -> Option<u64> {
        match self {
            Self::Healthy { epoch } | Self::Recovering { epoch, .. } => Some(*epoch),
            Self::Unavailable { .. } => None,
        }
    }

    /// Returns true when the render path is allowed to submit draw work.
    ///
    /// This is the canonical predicate the render gate (5.16.2) consults
    /// before walking `windows` and `dialogs`.
    pub(crate) fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy { .. })
    }

    /// Returns the last-error message recorded when the device transitioned
    /// to `Unavailable`, or `None` for any other state.
    ///
    /// 5.16.10 will use this to populate the window title bar
    /// (`"ori_term — GPU unavailable, press F5 to retry: <message>"`).
    pub(crate) fn unavailable_message(&self) -> Option<&str> {
        match self {
            Self::Unavailable { last_error, .. } => Some(last_error.as_str()),
            Self::Healthy { .. } | Self::Recovering { .. } => None,
        }
    }

    /// Returns the `Instant` at which the current `Unavailable` state was
    /// entered, or `None` for any other state.
    ///
    /// 5.16.10 uses this to gate the one-shot user-attention request and
    /// to compute "time spent unavailable" for diagnostics.
    pub(crate) fn unavailable_since(&self) -> Option<Instant> {
        match self {
            Self::Unavailable { since, .. } => Some(*since),
            Self::Healthy { .. } | Self::Recovering { .. } => None,
        }
    }

    /// Returns the `Instant` at which the current `Recovering` state was
    /// entered, or `None` for any other state.
    ///
    /// 5.16.2's backoff scheduler uses this to compute the next attempt
    /// timestamp via `next_attempt_at(attempt, since)`. 5.16.10 uses it to
    /// enforce the 30-second total budget cap before transitioning to
    /// `Unavailable`.
    pub(crate) fn recovering_since(&self) -> Option<Instant> {
        match self {
            Self::Recovering { since, .. } => Some(*since),
            Self::Healthy { .. } | Self::Unavailable { .. } => None,
        }
    }

    /// Pure state-transition function for the 5.16.1 detection plumbing.
    ///
    /// Computes the next [`GpuHealth`] state from the current state, an
    /// incoming [`GpuLossReason`], and the message string. Used by
    /// `App::handle_gpu_device_lost` to keep the side-effect-free transition
    /// logic separate from the App-level wiring (event proxy, logging,
    /// window walks). This is the canonical home for the transition table —
    /// 5.16.2's full state machine extends it with backoff and the
    /// single-flight gate.
    ///
    /// Transition table for 5.16.1:
    ///
    /// | Current      | Reason         | Next                                |
    /// |--------------|----------------|-------------------------------------|
    /// | `Healthy`    | `OutOfMemory`  | `Unavailable { last_error, since }` |
    /// | `Healthy`    | other          | `Recovering { epoch, attempt: 0 }`  |
    /// | `Recovering` | `OutOfMemory`  | `Unavailable { last_error, since }` |
    /// | `Recovering` | other          | unchanged (coalesce)                |
    /// | `Unavailable`| `OutOfMemory`  | unchanged (already terminal)        |
    /// | `Unavailable`| other          | unchanged (manual retry only)       |
    pub(crate) fn next_after_loss(
        &self,
        reason: GpuLossReason,
        message: &str,
        now: Instant,
    ) -> Option<Self> {
        // OOM is terminal regardless of current state — but if we're already
        // Unavailable, do nothing (avoid clobbering the original error).
        if matches!(reason, GpuLossReason::OutOfMemory) {
            if matches!(self, Self::Unavailable { .. }) {
                return None;
            }
            return Some(Self::Unavailable {
                last_error: format!("GPU out of memory: {message}"),
                since: now,
            });
        }

        // Non-OOM: only Healthy transitions. Recovering and Unavailable
        // coalesce (single-flight; manual retry to leave Unavailable).
        match self {
            Self::Healthy { epoch } => Some(Self::Recovering {
                epoch: *epoch,
                attempt: 0,
                since: now,
            }),
            Self::Recovering { .. } | Self::Unavailable { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
