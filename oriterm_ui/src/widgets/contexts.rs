//! Widget context types for layout, drawing, and event handling.
//!
//! These types are passed to [`Widget`] trait methods to provide access to
//! shared framework state during layout, rendering, and event dispatch.

use std::time::Instant;

use crate::animation::FrameRequestFlags;
use crate::controllers::ControllerRequests;
use crate::draw::Scene;
use crate::geometry::Rect;
use crate::icons::ResolvedIcons;
use crate::interaction::{InteractionManager, InteractionState};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::TextMeasurer;

/// Context passed to [`Widget::layout`].
pub struct LayoutCtx<'a> {
    /// Text measurement provider.
    pub measurer: &'a dyn TextMeasurer,
    /// Active UI theme.
    pub theme: &'a UiTheme,
}

/// Context passed to [`Widget::draw`].
pub struct DrawCtx<'a> {
    /// Text shaping provider.
    pub measurer: &'a dyn TextMeasurer,
    /// The scene to paint into.
    pub scene: &'a mut Scene,
    /// The widget's computed bounds (from layout).
    pub bounds: Rect,
    /// Current frame timestamp for animation interpolation.
    pub now: Instant,
    /// Active UI theme.
    pub theme: &'a UiTheme,
    /// Pre-resolved icon atlas entries for this frame.
    ///
    /// `None` in tests or when the GPU renderer is not available.
    /// Widgets fall back to `push_line()` when this is `None`.
    pub icons: Option<&'a ResolvedIcons>,
    /// Framework interaction state manager.
    ///
    /// `None` until the interaction system is fully wired (Section 03).
    /// Widgets use `is_hot()`, `is_active()`, `is_focused()` convenience
    /// methods which return `false` when this is `None`.
    pub interaction: Option<&'a InteractionManager>,
    /// The widget being drawn. `None` at the root level (app frame).
    pub widget_id: Option<WidgetId>,
    /// Shared animation frame and repaint request flags.
    ///
    /// `None` until the animation scheduling system is wired (Section 05.5).
    /// Widgets use `request_anim_frame()` and `request_paint()` convenience
    /// methods which are no-ops when this is `None`.
    pub frame_requests: Option<&'a FrameRequestFlags>,
}

impl DrawCtx<'_> {
    /// Whether the pointer is over this widget or any descendant.
    pub fn is_hot(&self) -> bool {
        match (self.interaction, self.widget_id) {
            (Some(mgr), Some(id)) => mgr.get_state(id).is_hot(),
            _ => false,
        }
    }

    /// Whether the pointer is directly over this widget (not a descendant).
    pub fn is_hot_direct(&self) -> bool {
        match (self.interaction, self.widget_id) {
            (Some(mgr), Some(id)) => mgr.get_state(id).is_hot_direct(),
            _ => false,
        }
    }

    /// Whether this widget has captured mouse events.
    pub fn is_active(&self) -> bool {
        match (self.interaction, self.widget_id) {
            (Some(mgr), Some(id)) => mgr.get_state(id).is_active(),
            _ => false,
        }
    }

    /// Whether this widget has keyboard focus (via `InteractionManager`).
    pub fn is_interaction_focused(&self) -> bool {
        match (self.interaction, self.widget_id) {
            (Some(mgr), Some(id)) => mgr.get_state(id).is_focused(),
            _ => false,
        }
    }

    /// Build a child draw context with child-specific bounds and widget ID.
    ///
    /// Reborrows `scene` and `interaction` from `self`.
    /// Copies all other fields. Containers should use this instead of
    /// constructing `DrawCtx` struct literals directly.
    pub fn for_child(&mut self, child_id: WidgetId, child_bounds: Rect) -> DrawCtx<'_> {
        self.scene.set_widget_id(Some(child_id));
        DrawCtx {
            measurer: self.measurer,
            scene: self.scene,
            bounds: child_bounds,
            now: self.now,
            theme: self.theme,
            icons: self.icons,
            interaction: self.interaction,
            widget_id: Some(child_id),
            frame_requests: self.frame_requests,
        }
    }

    /// Request an animation frame on the next vsync.
    ///
    /// The widget will receive an `AnimFrameEvent` with the time delta
    /// since the last frame. No-op until the scheduling system is wired.
    pub fn request_anim_frame(&self) {
        if let Some(flags) = self.frame_requests {
            flags.request_anim_frame();
        }
    }

    /// Request a repaint without an animation frame.
    ///
    /// No-op until the scheduling system is wired.
    pub fn request_paint(&self) {
        if let Some(flags) = self.frame_requests {
            flags.request_paint();
        }
    }
}

/// Context passed to mouse and keyboard event handlers.
pub struct EventCtx<'a> {
    /// Text measurement provider.
    pub measurer: &'a dyn TextMeasurer,
    /// The widget's computed bounds (from layout).
    pub bounds: Rect,
    /// Active UI theme.
    pub theme: &'a UiTheme,
    /// Framework interaction state manager.
    ///
    /// `None` until the interaction system is fully wired (Section 03).
    pub interaction: Option<&'a InteractionManager>,
    /// The widget receiving the event. `None` at the root level.
    pub widget_id: Option<WidgetId>,
    /// Shared animation frame and repaint request flags.
    ///
    /// `None` until the animation scheduling system is wired (Section 05.5).
    /// Widgets use `request_anim_frame()` and `request_paint()` convenience
    /// methods which are no-ops when this is `None`.
    pub frame_requests: Option<&'a FrameRequestFlags>,
}

impl EventCtx<'_> {
    /// Build a child context with child-specific bounds and widget ID.
    ///
    /// `child_id` identifies the child widget for `InteractionManager` lookups.
    /// Pass `None` for non-focusable children.
    #[must_use]
    pub fn for_child(&self, child_bounds: Rect, child_id: Option<WidgetId>) -> Self {
        Self {
            measurer: self.measurer,
            bounds: child_bounds,
            theme: self.theme,
            interaction: self.interaction,
            widget_id: child_id,
            frame_requests: self.frame_requests,
        }
    }

    /// Request an animation frame on the next vsync.
    ///
    /// The widget will receive an `AnimFrameEvent` with the time delta
    /// since the last frame. No-op until the scheduling system is wired.
    pub fn request_anim_frame(&self) {
        if let Some(flags) = self.frame_requests {
            flags.request_anim_frame();
        }
    }

    /// Request a repaint without an animation frame.
    ///
    /// No-op until the scheduling system is wired.
    pub fn request_paint(&self) {
        if let Some(flags) = self.frame_requests {
            flags.request_paint();
        }
    }

    /// Whether the pointer is over this widget or any descendant.
    pub fn is_hot(&self) -> bool {
        match (self.interaction, self.widget_id) {
            (Some(mgr), Some(id)) => mgr.get_state(id).is_hot(),
            _ => false,
        }
    }

    /// Whether this widget has captured mouse events.
    pub fn is_active(&self) -> bool {
        match (self.interaction, self.widget_id) {
            (Some(mgr), Some(id)) => mgr.get_state(id).is_active(),
            _ => false,
        }
    }

    /// Whether this widget has keyboard focus (via `InteractionManager`).
    pub fn is_interaction_focused(&self) -> bool {
        match (self.interaction, self.widget_id) {
            (Some(mgr), Some(id)) => mgr.get_state(id).is_focused(),
            _ => false,
        }
    }
}

/// Context passed to [`Widget::lifecycle`].
///
/// Provides per-widget interaction state and a request mechanism for
/// side effects (repaint, relayout). Matches the `ControllerCtx` pattern
/// from Section 04 — widgets see only their own state, not the full manager.
pub struct LifecycleCtx<'a> {
    /// The widget receiving the lifecycle event.
    pub widget_id: WidgetId,
    /// Per-widget interaction state (hot, active, focused, disabled).
    pub interaction: &'a InteractionState,
    /// Side-effect requests accumulated during the lifecycle handler.
    pub requests: ControllerRequests,
}

/// Context passed to [`Widget::anim_frame`].
///
/// Provides timing information and request flags so widgets can continue
/// animation or request repaints. The framework calls `anim_frame()` only
/// when the widget previously requested an animation frame.
pub struct AnimCtx<'a> {
    /// The widget receiving the animation frame.
    pub widget_id: WidgetId,
    /// Current frame timestamp.
    pub now: Instant,
    /// Side-effect requests accumulated during the animation handler.
    pub requests: ControllerRequests,
    /// Shared frame request flags for scheduling follow-up frames.
    ///
    /// `None` until the scheduling system is wired. Widgets call
    /// `request_anim_frame()` to continue animation, or `request_paint()`
    /// for a one-shot repaint.
    pub frame_requests: Option<&'a FrameRequestFlags>,
}

impl AnimCtx<'_> {
    /// Request another animation frame on the next vsync.
    ///
    /// Call this when `animator.is_animating(now)` is true.
    pub fn request_anim_frame(&self) {
        if let Some(flags) = self.frame_requests {
            flags.request_anim_frame();
        }
    }

    /// Request a repaint without an animation frame.
    pub fn request_paint(&self) {
        if let Some(flags) = self.frame_requests {
            flags.request_paint();
        }
    }
}
