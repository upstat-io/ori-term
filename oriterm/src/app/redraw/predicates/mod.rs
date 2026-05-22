//! Split-predicate gating for the single-pane redraw closure.
//!
//! Regression: — `content_changed` previously omitted
//! `preedit_revision` input; IME preedit-shrink frames reused stale
//! `frame.content` carrying the prior frame's destructive overlay.

/// Inputs to [`compute_redraw_predicates`]. Packs the 5 source signals
/// the single-pane redraw closure observes into one struct per the
/// project's parameter-hygiene rule (>4 params → struct).
#[derive(Clone, Copy, Debug)]
pub(in crate::app) struct RedrawPredicateInputs {
    pub snap_is_none: bool,
    pub snap_dirty: bool,
    pub pane_changed: bool,
    pub preedit_revision: u64,
    pub prev_preedit_revision: u64,
}

/// Output of [`compute_redraw_predicates`] — split predicate gating two
/// distinct redraw decisions.
/// `snapshot_changed` gates the snapshot-refresh + embedded
/// `swap_renderable_content` fast-path. `swap_renderable_content` does a
/// blind `std::mem::swap` on its renderable cache (per
/// `oriterm_mux/src/backend/embedded/mod.rs:440-442`), so if the broader
/// `content_changed` predicate fired the swap on a preedit-only frame,
/// the swap would surface the prior frame's destructive preedit overlay
/// rather than a fresh re-extracted buffer.
/// `content_changed` is the broader predicate that gates re-extract from
/// `PaneSnapshot` and the subsequent `WindowRenderer::prepare` shape
/// pass. Re-extracting from the canonical snapshot restores clean cells
/// before the new (smaller) preedit overlay is applied — the regression
/// anchor surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) struct RedrawPredicates {
    pub snapshot_changed: bool,
    pub content_changed: bool,
}

/// Split predicate gating the snapshot-refresh / swap fast-path vs the
/// content-re-extract / GPU-prepare path. See [`RedrawPredicates`] for
/// the load-bearing rationale on why these two predicates must remain
/// separate — see the regression anchor at the type-level doc.
pub(in crate::app) fn compute_redraw_predicates(inputs: RedrawPredicateInputs) -> RedrawPredicates {
    let snapshot_changed = inputs.snap_is_none || inputs.snap_dirty || inputs.pane_changed;
    let preedit_revision_changed = inputs.preedit_revision != inputs.prev_preedit_revision;
    RedrawPredicates {
        snapshot_changed,
        content_changed: snapshot_changed || preedit_revision_changed,
    }
}

#[cfg(test)]
mod tests;
