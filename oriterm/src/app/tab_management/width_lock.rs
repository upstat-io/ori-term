//! Tab-bar width-lock accessors.
//!
//! Extracted from `mod.rs` to keep the parent under the 500-line limit.
//!
//! Visibility is `pub(in crate::app)` (not `pub(super)`) because callers live
//! across many sibling modules of `tab_management/` (`chrome`, `tab_drag`,
//! `tab_bar_input`, `event_loop`) — `pub(super)` from this depth would only
//! expose to `tab_management/` itself.

use crate::app::App;

impl App {
    /// Current tab width lock value, if active.
    ///
    /// Delegates to the tab bar widget — the widget is the single source
    /// of truth for this value.
    pub(in crate::app) fn tab_width_lock(&self) -> Option<f32> {
        self.focused_ctx()
            .and_then(|ctx| ctx.tab_bar.tab_width_lock())
    }

    /// Freeze tab widths at `width` to prevent layout jitter.
    pub(in crate::app) fn acquire_tab_width_lock(&mut self, width: f32) {
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.tab_bar.set_tab_width_lock(Some(width));
        }
    }

    /// Release the tab width lock, allowing tabs to recompute widths.
    pub(in crate::app) fn release_tab_width_lock(&mut self) {
        if self.tab_width_lock().is_some() {
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.tab_bar.set_tab_width_lock(None);
                ctx.root.mark_dirty();
            }
        }
    }
}
