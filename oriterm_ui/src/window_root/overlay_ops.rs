//! Overlay convenience methods for `WindowRoot`.
//!
//! These methods handle the borrow splitting needed when overlay operations
//! require simultaneous mutable access to `OverlayManager`, `LayerTree`,
//! and `LayerAnimator` — all of which live inside `WindowRoot`.

use std::time::{Duration, Instant};

use crate::animation::Easing;
use crate::color::Color;
use crate::geometry::Rect;
use crate::overlay::{CompositorHandles, FlashSpec, OverlayId, Placement};
use crate::widget_id::WidgetId;
use crate::widgets::{LayoutCtx, Widget};

use super::WindowRoot;

impl WindowRoot {
    /// Pushes a popup overlay at the given anchor with the specified placement.
    pub fn push_overlay(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        _now: Instant,
    ) -> OverlayId {
        self.overlays.push_overlay(
            widget,
            anchor,
            placement,
            &mut CompositorHandles {
                tree: &mut self.layer_tree,
                animator: &mut self.layer_animator,
            },
        )
    }

    /// Pushes a modal overlay with dim background.
    pub fn push_modal(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        _now: Instant,
    ) -> OverlayId {
        self.overlays.push_modal(
            widget,
            anchor,
            placement,
            &mut CompositorHandles {
                tree: &mut self.layer_tree,
                animator: &mut self.layer_animator,
            },
        )
    }

    /// Replaces the topmost popup overlay with a new widget.
    pub fn replace_popup(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        _now: Instant,
    ) -> OverlayId {
        self.overlays.replace_popup(
            widget,
            anchor,
            placement,
            &mut CompositorHandles {
                tree: &mut self.layer_tree,
                animator: &mut self.layer_animator,
            },
        )
    }

    /// Pushes a transient full-viewport color flash that fades to transparent.
    ///
    /// Used by the `MuxNotification::PaneBell` consumer in `oriterm`'s
    /// `mux_pump` to render the visual bell when `BellConfig::is_enabled()`.
    /// A `duration_ms` of `0` is a no-op (caller is expected to gate on
    /// `BellConfig::is_enabled()`, but defending in depth here makes the
    /// API safe to call unconditionally).
    ///
    /// The flash overlay never captures input — clicks pass through to the
    /// underlying widget tree. A subsequent call replaces any in-flight
    /// flash so bell-storm scenarios cannot accumulate overlays.
    pub fn ring_visual_bell(
        &mut self,
        now: Instant,
        duration_ms: u16,
        color: Color,
        easing: Easing,
    ) {
        if duration_ms == 0 {
            return;
        }
        let duration = Duration::from_millis(u64::from(duration_ms));
        self.overlays.push_flash(
            FlashSpec {
                color,
                duration,
                easing,
            },
            &mut CompositorHandles {
                tree: &mut self.layer_tree,
                animator: &mut self.layer_animator,
            },
            now,
        );
        self.mark_dirty();
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
        reason = "app-facing overlay entry point — signature fixed by oriterm callers (mouse_input, dialog_context)"
    )]
    pub fn process_overlay_mouse_event(
        &mut self,
        event: &crate::input::MouseEvent,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &crate::theme::UiTheme,
        _focused_widget: Option<WidgetId>,
        now: Instant,
    ) -> crate::overlay::OverlayEventResult {
        self.overlays.process_mouse_event(
            event,
            &LayoutCtx { measurer, theme },
            &mut CompositorHandles {
                tree: &mut self.layer_tree,
                animator: &mut self.layer_animator,
            },
            now,
        )
    }

    /// Routes a key event through the overlay manager with keymap-first
    /// dispatch ().
    ///
    /// Resolves the topmost overlay's `key_context()` against
    /// `self.keymap` and dispatches via `dispatch_keymap_action` when a
    /// binding matches. Falls through to the legacy `on_input` pipeline +
    /// inline-Escape backstop on a keymap miss. Returns `PassThrough` if
    /// no overlay consumed the event.
    #[expect(
        clippy::too_many_arguments,
        reason = "app-facing overlay entry point — signature fixed by oriterm callers (keyboard_input, dialog_context)"
    )]
    pub fn process_overlay_key_event(
        &mut self,
        event: crate::input::KeyEvent,
        _measurer: &dyn crate::widgets::TextMeasurer,
        _theme: &crate::theme::UiTheme,
        _focused_widget: Option<WidgetId>,
        now: Instant,
    ) -> crate::overlay::OverlayEventResult {
        self.overlays.process_key_event_with_keymap(
            event,
            &self.keymap,
            &mut CompositorHandles {
                tree: &mut self.layer_tree,
                animator: &mut self.layer_animator,
            },
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
