//! Multi-pane scratch buffer helpers.

use oriterm_mux::PaneSnapshot;

use crate::gpu::FrameSearch;
use crate::session::PaneLayout;

use super::super::App;

/// Whether the shared scratch frame needs re-extraction for a pane.
///
/// Returns `true` when content changed, no frame exists yet, or the
/// scratch buffer currently holds another pane's data.
pub(super) fn should_reextract_scratch_frame(
    content_refreshed: bool,
    frame_missing: bool,
    scratch_matches_pane: bool,
) -> bool {
    content_refreshed || frame_missing || !scratch_matches_pane
}

/// Compute focused-pane chrome state for `ChromeParams`.
///
/// Returns `(content_cols, content_rows, search)` extracted from the
/// FOCUSED pane's snapshot — never from the per-pane scratch buffer
/// `ctx.frame`, which holds the last-iterated pane's state in multi-pane
/// mode. When the focused pane has no snapshot (mux unavailable, pane
/// closed), returns `(0, 0, None)`.
pub(super) fn focused_pane_chrome_state(
    focused_snapshot: Option<&PaneSnapshot>,
) -> (usize, usize, Option<FrameSearch>) {
    focused_snapshot.map_or((0, 0, None), |snap| {
        (
            snap.cols as usize,
            snap.cells.len(),
            FrameSearch::from_snapshot(snap),
        )
    })
}

impl App {
    /// Copy per-pane selections and mark cursors into scratch buffers.
    ///
    /// This must run before the render block because `ctx.renderer` is
    /// mutably borrowed during render, preventing `&self` access.
    pub(super) fn populate_multi_pane_scratch(&mut self, layouts: &[PaneLayout]) {
        self.scratch_pane_sels.clear();
        for l in layouts {
            if let Some(sel) = self.pane_selection(l.pane_id).copied() {
                self.scratch_pane_sels.insert(l.pane_id, sel);
            }
        }
        self.scratch_pane_mcs.clear();
        for l in layouts {
            if let Some(mc) = self.pane_mark_cursor(l.pane_id) {
                self.scratch_pane_mcs.insert(l.pane_id, mc);
            }
        }
    }
}
