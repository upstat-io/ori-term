//! `TackNavigator`: walks `&[MenuStep]` against a live `PtySession`.
//!
//! See `plans/tack-conformance/section-04-scenario-framework.md`
//! § 04.2 for the pre-existing-anchor guard rationale (C1) and the
//! non-panicking `wait_for_any`-based alternate-anchor matching
//! (M4b — replaces an earlier draft's `catch_unwind` antipattern).

use crate::session::PtySession;

use super::spec::MenuStep;

/// Walks a `&[MenuStep]` against a live [`PtySession`] running tack.
///
/// Each step is `pre-existing-guard → send → wait_for_any`, with no
/// fixed sleeps anywhere and no `catch_unwind`. On wait timeout the
/// navigator panics with a message that includes the failing step
/// index, the bytes sent, the primary + alternate anchors, and the
/// current grid contents.
///
/// Calls [`PtySession::wait_for_any`] (introduced in 04.0.b.2) — a
/// non-panicking multi-anchor primitive that returns `Some(idx)` on
/// match or `None` on timeout. There is NO parallel wait-for
/// implementation here, by design: if a future change needs richer
/// step diagnostics, extend `wait_for_any` once and every consumer
/// benefits.
pub struct TackNavigator;

/// Total CI-safe timeout for one navigation step. Bump only on
/// observed flakes.
const STEP_TIMEOUT_MS: u64 = 5_000;

/// Stack-friendly upper bound for a single [`MenuStep`]'s combined
/// `[primary, ...alternates]` anchor slice. Tack menus in practice
/// never produce more than two or three alternates, so 8 is a
/// comfortable cap with no heap allocation in the navigator loop.
const MAX_ANCHORS_PER_STEP: usize = 8;

impl TackNavigator {
    /// Walk `steps` against `session`. Panics on any wait timeout or
    /// pre-existing-anchor violation.
    pub fn navigate(session: &mut PtySession, steps: &[MenuStep]) {
        for (idx, step) in steps.iter().enumerate() {
            Self::guard_pre_existing_anchor(session, step, idx);
            session.send(step.send);
            Self::wait_for_step(session, step, idx);
        }
    }

    /// Pre-send guard: panics if `step.wait_for` (or any
    /// `or_wait_for` alternate) is already present in the grid
    /// BEFORE we send `step.send`. Picking an anchor that's on the
    /// prior screen makes `wait_for` return immediately and the next
    /// keystroke goes to the wrong state.
    fn guard_pre_existing_anchor(session: &mut PtySession, step: &MenuStep, idx: usize) {
        // Drain any pending output so the snapshot is current.
        session.drain();
        let pre_grid = session.grid_text();
        let mut already: Vec<&str> = Vec::new();
        if pre_grid.contains(step.wait_for) {
            already.push(step.wait_for);
        }
        for alt in step.or_wait_for {
            if pre_grid.contains(alt) {
                already.push(alt);
            }
        }
        assert!(
            already.is_empty(),
            "TackNavigator: step {idx} pre-existing-anchor violation: \
             anchor(s) {already:?} already present in grid before send. \
             Pick a SUBMENU-specific anchor (sub-menu prompt or screen-\
             unique heading), not a word that's already on the prior \
             screen.\nSent: {send_repr:?}\nGrid:\n{pre_grid}",
            send_repr = String::from_utf8_lossy(step.send),
        );
    }

    /// Wait for `step.wait_for` OR any `or_wait_for` alternate to
    /// appear in the grid, via a single
    /// [`PtySession::wait_for_any`] call.
    ///
    /// Builds a fixed-size stack array of `[primary, ...alternates]`
    /// (capped at [`MAX_ANCHORS_PER_STEP`]) and passes it to
    /// `wait_for_any`. On `Some(_)` the step succeeds. On `None` the
    /// navigator panics with a full-context message listing every
    /// anchor that was tried.
    fn wait_for_step(session: &mut PtySession, step: &MenuStep, idx: usize) {
        // Overflow guard: MenuStep::or_wait_for is &'static so this
        // is effectively a static assertion — if a scenario ever
        // lists more than 7 alternates it's a design smell and we
        // want a loud failure at navigate-time rather than a silent
        // truncation.
        assert!(
            step.or_wait_for.len() < MAX_ANCHORS_PER_STEP,
            "TackNavigator: step {idx} has {n} anchors (primary + \
             {alt} alternates) — the cap is {MAX_ANCHORS_PER_STEP}. \
             Split the MenuStep or raise MAX_ANCHORS_PER_STEP.",
            n = 1 + step.or_wait_for.len(),
            alt = step.or_wait_for.len(),
        );

        let mut anchors: [&str; MAX_ANCHORS_PER_STEP] = [""; MAX_ANCHORS_PER_STEP];
        anchors[0] = step.wait_for;
        for (i, alt) in step.or_wait_for.iter().enumerate() {
            anchors[i + 1] = alt;
        }
        let active = 1 + step.or_wait_for.len();

        let matched = session.wait_for_any(&anchors[..active], STEP_TIMEOUT_MS);
        if matched.is_some() {
            return;
        }

        // Timeout — build the full-context panic.
        panic!(
            "TackNavigator: step {idx} failed — none of the anchors \
             appeared within {STEP_TIMEOUT_MS}ms total.\n\
             Sent: {send_repr:?}\n\
             Primary anchor: {primary:?}\n\
             Alternate anchors: {alts:?}\n\
             Grid:\n{grid}",
            send_repr = String::from_utf8_lossy(step.send),
            primary = step.wait_for,
            alts = step.or_wait_for,
            grid = session.grid_text(),
        );
    }
}

#[cfg(test)]
mod tests;
