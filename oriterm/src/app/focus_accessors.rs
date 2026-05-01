//! Focused-window accessors: `focused_ctx`, `focused_ctx_mut`, `focused_renderer`.
//!
//! Extracted from `mod.rs` to keep the parent under the 500-line limit.

use crate::app::App;
use crate::gpu::WindowRenderer;

use super::window_context::WindowContext;

impl App {
    /// The focused window's context, if any.
    pub(super) fn focused_ctx(&self) -> Option<&WindowContext> {
        self.focused_window_id.and_then(|id| self.windows.get(&id))
    }

    /// The focused window's context (mutable), if any.
    pub(super) fn focused_ctx_mut(&mut self) -> Option<&mut WindowContext> {
        self.focused_window_id
            .and_then(|id| self.windows.get_mut(&id))
    }

    /// The focused window's renderer, if any.
    pub(super) fn focused_renderer(&self) -> Option<&WindowRenderer> {
        self.focused_window_id
            .and_then(|id| self.windows.get(&id))
            .and_then(|ctx| ctx.renderer.as_ref())
    }
}
