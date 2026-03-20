//! Overlay convenience methods for `WindowRoot`.
//!
//! These methods handle the borrow splitting needed when overlay operations
//! require simultaneous mutable access to `OverlayManager`, `LayerTree`,
//! and `LayerAnimator` — all of which live inside `WindowRoot`.

use std::time::Instant;

use crate::geometry::Rect;
use crate::overlay::{OverlayId, Placement};
use crate::widget_id::WidgetId;
use crate::widgets::Widget;

use super::WindowRoot;

impl WindowRoot {
    /// Pushes a popup overlay at the given anchor with the specified placement.
    pub fn push_overlay(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        now: Instant,
    ) -> OverlayId {
        self.overlays.push_overlay(
            widget,
            anchor,
            placement,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Pushes a modal overlay with dim background.
    pub fn push_modal(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        now: Instant,
    ) -> OverlayId {
        self.overlays.push_modal(
            widget,
            anchor,
            placement,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Replaces the topmost popup overlay with a new widget.
    pub fn replace_popup(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        now: Instant,
    ) -> OverlayId {
        self.overlays.replace_popup(
            widget,
            anchor,
            placement,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Begins dismissing the topmost overlay with a fade-out animation.
    pub fn dismiss_topmost(&mut self, now: Instant) -> Option<OverlayId> {
        self.overlays
            .begin_dismiss_topmost(&mut self.layer_tree, &mut self.layer_animator, now)
    }

    /// Removes all popup overlays immediately.
    pub fn clear_popups(&mut self) -> usize {
        self.overlays
            .clear_popups(&mut self.layer_tree, &mut self.layer_animator)
    }

    /// Returns whether any overlays are active.
    pub fn has_overlays(&self) -> bool {
        !self.overlays.is_empty()
    }

    /// Routes a mouse event through the overlay manager.
    ///
    /// Returns `PassThrough` if no overlay consumed the event.
    #[expect(
        clippy::too_many_arguments,
        reason = "forwarding overlay manager params with borrow splitting"
    )]
    pub fn process_overlay_mouse_event(
        &mut self,
        event: &crate::input::MouseEvent,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &crate::theme::UiTheme,
        focused_widget: Option<WidgetId>,
        now: Instant,
    ) -> crate::overlay::OverlayEventResult {
        self.overlays.process_mouse_event(
            event,
            measurer,
            theme,
            focused_widget,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Routes a key event through the overlay manager.
    ///
    /// Returns `PassThrough` if no overlay consumed the event.
    #[expect(
        clippy::too_many_arguments,
        reason = "forwarding overlay manager params with borrow splitting"
    )]
    pub fn process_overlay_key_event(
        &mut self,
        event: crate::input::KeyEvent,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &crate::theme::UiTheme,
        focused_widget: Option<WidgetId>,
        now: Instant,
    ) -> crate::overlay::OverlayEventResult {
        self.overlays.process_key_event(
            event,
            measurer,
            theme,
            focused_widget,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Returns the number of drawable overlays.
    pub fn overlay_draw_count(&self) -> usize {
        self.overlays.draw_count()
    }

    /// Computes layout for all overlay widgets.
    pub fn layout_overlays(
        &mut self,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &crate::theme::UiTheme,
    ) {
        self.overlays.layout_overlays(measurer, theme);
    }

    /// Draws overlay at the given draw index, returning its opacity.
    pub fn draw_overlay_at(&self, draw_idx: usize, ctx: &mut crate::widgets::DrawCtx<'_>) -> f32 {
        self.overlays
            .draw_overlay_at(draw_idx, ctx, &self.layer_tree)
    }

    /// Ticks layer animations and cleans up dismissed overlays.
    ///
    /// Returns `true` if any animations are still in progress.
    pub fn tick_overlay_animations(&mut self, now: Instant) -> bool {
        if !self.layer_animator.is_any_animating() {
            return false;
        }
        let animating = self.layer_animator.tick(&mut self.layer_tree, now);
        self.overlays
            .cleanup_dismissed(&mut self.layer_tree, &self.layer_animator);
        animating
    }
}
