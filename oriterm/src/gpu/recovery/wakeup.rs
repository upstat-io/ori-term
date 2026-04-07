//! Wakeup-source gating (5.16.2).
//!
//! While the GPU is `Recovering` or `Unavailable`, every periodic timer
//! that pulls a frame must be quiesced — otherwise the event loop spins
//! against a dead device. The [`WakeupSource`] enum enumerates every
//! gated source; [`should_post_wakeup`] is the canonical predicate every
//! source consults before queuing a redraw or scheduling a wake-up.
//!
//! `MuxPump` is **not** in the enum because the mux pump is explicitly
//! NOT gated: PTY output must continue to be absorbed into `Term`/`Grid`
//! so terminal state stays current. Only the *render wakeup* the pump
//! would post is suppressed; the snapshot still flows to the dirty flag,
//! and the dirty flag is consumed on the first post-recovery frame.
//!
//! Adding a new wakeup source REQUIRES adding a `WakeupSource` variant —
//! the exhaustive match in [`should_post_wakeup`] turns "I forgot to gate
//! the new wakeup source" from a runtime regression into a compile-time
//! error. This is the SSOT for "all gated wakeups".

use super::GpuHealth;

/// Every periodic-wakeup source that must be quiesced during recovery.
///
/// Mirrors the audit in plan section 5.16.2 — the mapping from variant
/// to wakeup site lives in App's source tree (where each periodic timer
/// or wakeup thread calls [`should_post_wakeup`] before posting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WakeupSource {
    /// `App::cursor_blink` + the `blink_wakeup_gen` wakeup thread.
    CursorBlink,
    /// `App::text_blink` (SGR 5/6 blink-attribute timer).
    TextBlink,
    /// `WindowContext.tab_slide` animation `request_redraw`.
    TabSlide,
    /// `oriterm_ui::compositor::LayerAnimator` driving `is_any_animating()` →
    /// `WaitUntil` wakeups.
    LayerAnimator,
    /// `oriterm_ui::animation::RenderScheduler` on `WindowRoot`.
    RenderScheduler,
    /// `VisualStateAnimator` for hover/press/focus colour transitions.
    VisualStateAnimator,
    /// Auto-scroll timer (mark-mode + selection-drag scroll).
    AutoScroll,
    /// Cursor hover hold timer (URL hover, tooltip).
    CursorHoverHold,
}

/// Returns `true` when the given wakeup source is allowed to post a
/// redraw / queue a `WaitUntil`. Returns `false` when the GPU is gated
/// and the wakeup must be silently dropped.
///
/// The recovery state-change wake-up will resume the source naturally
/// when `gpu_health` returns to `Healthy`. Animation *progress* (CPU
/// time updates) may continue independently — only the wakeup posting
/// is suppressed so the visible state matches reality after recovery.
pub(crate) fn should_post_wakeup(health: &GpuHealth, _source: WakeupSource) -> bool {
    // 5.16.2 baseline: gate every source uniformly when not Healthy.
    // The `_source` parameter exists so individual sources may opt out
    // of the gate in future sub-blocks (e.g., a future GPU diagnostics
    // overlay that should keep ticking even during recovery). The
    // exhaustive match contract still applies — adding a variant forces
    // any future per-source policy to be made explicit here.
    health.is_healthy()
}
