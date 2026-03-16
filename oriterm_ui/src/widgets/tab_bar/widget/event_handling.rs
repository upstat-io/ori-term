//! Event handling logic for [`TabBarWidget`].
//!
//! Contains `handle_mouse_impl()`, `handle_hover_impl()`, and
//! `handle_key_impl()` as inherent methods, delegated from the [`Widget`]
//! trait impl in `draw.rs`. These methods are removed entirely during the
//! Widget trait migration (Section 08), at which point this file is deleted.

use crate::input::{HoverEvent, KeyEvent, MouseEvent};
use crate::widgets::{EventCtx, WidgetResponse};

use super::TabBarWidget;

impl TabBarWidget {
    /// Handles mouse events: hit testing and click dispatch.
    ///
    /// Currently a stub returning [`WidgetResponse::ignored()`]. Full
    /// hit-test dispatch is Section 16.3.
    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        reason = "stub — signature matches Widget trait, will mutate self when implemented"
    )]
    pub(super) fn handle_mouse_impl(
        &mut self,
        _event: &MouseEvent,
        _ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    /// Handles hover enter/leave events.
    ///
    /// Currently a stub returning [`WidgetResponse::ignored()`]. Hover
    /// enter/leave routing is Section 16.3.
    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        reason = "stub — signature matches Widget trait, will mutate self when implemented"
    )]
    pub(super) fn handle_hover_impl(
        &mut self,
        _event: HoverEvent,
        _ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    /// Handles keyboard events.
    ///
    /// Currently a stub returning [`WidgetResponse::ignored()`].
    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        reason = "stub — signature matches Widget trait, will mutate self when implemented"
    )]
    pub(super) fn handle_key_impl(
        &mut self,
        _event: KeyEvent,
        _ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        WidgetResponse::ignored()
    }
}
