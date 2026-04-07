//! Render dispatch outcome — the canonical return type for the GPU render
//! gate (5.16.2). Lives next to [`GpuHealth`](super::GpuHealth) so the gate
//! decision and the state it consults share a single module home.
//!
//! The render gate is a pure function over [`GpuHealth`]: it returns
//! `Some(GatedRecovering)` or `Some(GatedUnavailable)` when the dispatcher
//! must skip GPU work, and `None` when the dispatcher is free to render.
//! The full dispatcher converts the `None` → `RenderOutcome::Submitted`
//! after a successful frame, or `RenderOutcome::Skipped` when there is
//! nothing to render (no dirty windows).
//!
//! Per `.claude/rules/impl-hygiene.md` SSOT: only [`gate_outcome`] reads
//! `GpuHealth` to make the gate decision. The dispatcher and any caller
//! that needs to know "is render allowed?" routes through this function;
//! no consumer pattern-matches `GpuHealth` directly outside the recovery
//! module.

use super::GpuHealth;

/// The canonical outcome of one render-dispatch call.
///
/// Used by `App::render_dirty_windows` (5.16.2 render gate) so the gate is
/// testable without a GPU. Each variant is a single observable behaviour
/// the dispatcher chose for the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenderOutcome {
    /// At least one window was rendered and presented this frame.
    Submitted,
    /// The dispatcher returned early because the GPU is `Recovering`. No
    /// dirty flags were cleared; the next post-recovery frame will be a
    /// full repaint.
    GatedRecovering,
    /// The dispatcher returned early because the GPU is `Unavailable`. No
    /// dirty flags were cleared; the next render must wait for a manual
    /// retry (5.16.10).
    GatedUnavailable,
    /// The dispatcher ran but had nothing to render (no dirty windows).
    Skipped,
}

/// Pure render-gate decision.
///
/// Returns `Some(GatedRecovering)` or `Some(GatedUnavailable)` when the
/// dispatcher must skip GPU work, and `None` when the dispatcher is free
/// to walk the dirty list. This is the canonical I1 invariant check —
/// "no submit against a stale device" — encoded as a function the
/// dispatcher consults before any `WindowRenderer` method runs.
///
/// Pure tests in `recovery::tests` exercise the full transition table.
pub(crate) fn gate_outcome(health: &GpuHealth) -> Option<RenderOutcome> {
    match health {
        GpuHealth::Healthy { .. } => None,
        GpuHealth::Recovering { .. } => Some(RenderOutcome::GatedRecovering),
        GpuHealth::Unavailable { .. } => Some(RenderOutcome::GatedUnavailable),
    }
}
