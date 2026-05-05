//! Higher-order screen walker for vttest menu tests.
//!
//! Replaces 17 hand-rolled copies of the same control-flow skeleton
//! across `oriterm_core/tests/vttest/` and
//! `oriterm/src/gpu/visual_regression/vttest/` with a single canonical
//! algorithm that takes a per-screen closure for the variation. See
//! `bug-tracker/plans/completed/00-overview.md` for the design consensus and
//! migration matrix.

use crate::PtySession;

/// Walk vttest screens, calling `on_screen` for each non-sentinel
/// screen until `"Enter choice number"` (or any string in
/// `extra_sentinels`) appears in `grid_text()`, or `max_screens`
/// iterations elapse.
///
/// The closure receives:
/// 1. `&mut PtySession` — for interleaved sends or `grid_chars` access.
/// 2. `&str text` — the captured grid text the helper already allocated
///    for the sentinel check, so the closure does not re-pay the
///    `grid_text()` allocation cost.
/// 3. `usize screen` — the 1-based screen index.
///
/// After the closure returns, `\r` is sent to advance to the next
/// screen via [`PtySession::send`] (which already includes the 300ms
/// quiescent wait that every original walker relied on).
///
/// Returns the number of screens for which `on_screen` was called.
pub fn walk_vttest_screens<F>(
    session: &mut PtySession,
    max_screens: usize,
    extra_sentinels: &[&str],
    mut on_screen: F,
) -> usize
where
    F: FnMut(&mut PtySession, &str, usize),
{
    let mut count = 0usize;
    let mut screen = 1usize;
    loop {
        let text = session.grid_text();
        if text.contains("Enter choice number") || extra_sentinels.iter().any(|s| text.contains(s))
        {
            break;
        }
        if count >= max_screens {
            break;
        }
        on_screen(session, &text, screen);
        count += 1;
        session.send(b"\r");
        screen += 1;
    }
    count
}

#[cfg(test)]
mod tests;
