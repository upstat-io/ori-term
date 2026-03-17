//! Core widget types and traits for the UI framework.
//!
//! Provides the [`Widget`] trait, action/response types, and context structs
//! that widgets use during layout, drawing, and event handling. Each widget
//! is a concrete struct implementing `Widget`. Trait objects (`Box<dyn Widget>`)
//! are used for dynamic dispatch in overlay and container contexts.

pub mod contexts;
pub mod text_measurer;

pub mod button;
pub mod checkbox;
pub mod container;
pub mod dialog;
pub mod dropdown;
pub mod form_layout;
pub mod form_row;
pub mod form_section;
pub mod label;
pub mod menu;
pub mod panel;
pub mod rich_label;
pub mod scroll;
pub mod separator;
pub mod settings_panel;
pub mod slider;
pub mod spacer;
pub mod stack;
pub mod status_badge;
pub mod tab_bar;
pub mod text_input;
pub mod toggle;
pub mod window_chrome;

use crate::animation::anim_frame::AnimFrameEvent;
use crate::controllers::EventController;
use crate::hit_test_behavior::HitTestBehavior;
use crate::interaction::LifecycleEvent;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

pub use contexts::{AnimCtx, DrawCtx, EventCtx, LayoutCtx, LifecycleCtx};
pub use text_measurer::TextMeasurer;

// `WidgetAction` lives in `crate::action` to avoid a circular dependency
// (`controllers -> widgets`). Re-exported here for backward compatibility.
pub use crate::action::WidgetAction;

/// Result of `Widget::on_input()` fallback handling.
///
/// Returned by widgets that handle input events directly (not via controllers).
/// Carries an optional semantic action for the application layer.
#[derive(Debug, Default)]
pub struct OnInputResult {
    /// Whether the event was handled.
    pub handled: bool,
    /// Semantic action emitted, if any.
    pub action: Option<WidgetAction>,
}

impl OnInputResult {
    /// Event was handled, no action emitted.
    pub fn handled() -> Self {
        Self {
            handled: true,
            action: None,
        }
    }

    /// Event was not handled.
    pub fn ignored() -> Self {
        Self {
            handled: false,
            action: None,
        }
    }

    /// Attaches a semantic action to this result.
    #[must_use]
    pub fn with_action(mut self, action: WidgetAction) -> Self {
        self.action = Some(action);
        self
    }
}

/// The core widget trait.
///
/// Each widget is a concrete struct that implements this trait. Widgets
/// own their visual state (hovered, pressed) and app state (checked, value),
/// plus a style struct with `Default` dark-theme defaults.
///
/// Input is handled by event controllers (`controllers()`), with `on_input()`
/// as a fallback for widget-internal logic. Visual state transitions are
/// driven by `VisualStateAnimator` (`visual_states()`). Lifecycle events
/// (`lifecycle()`) notify widgets of hot/active/focus changes.
pub trait Widget {
    /// Returns this widget's unique identifier.
    fn id(&self) -> WidgetId;

    /// Whether this widget can receive keyboard focus.
    ///
    /// Default derives from `sense().has_focus()`. Override only if focusability
    /// depends on runtime state (e.g., disabled widgets).
    fn is_focusable(&self) -> bool {
        self.sense().has_focus()
    }

    /// Builds a layout descriptor for the layout solver.
    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox;

    // --- New methods (Section 08.1) ---

    /// Paints the widget into the draw list.
    ///
    /// Use `ctx.is_hot()`, `ctx.is_active()`, `ctx.is_focused()` for
    /// interaction-dependent rendering. Use `VisualStateAnimator` for
    /// animated property interpolation.
    fn paint(&self, _ctx: &mut DrawCtx<'_>) {}

    /// Handles lifecycle events (hot/active/focus changes, widget add/remove).
    ///
    /// Called by the framework when interaction state changes. Default is a no-op.
    fn lifecycle(&mut self, _event: &LifecycleEvent, _ctx: &mut LifecycleCtx<'_>) {}

    /// Handles animation frame ticks.
    ///
    /// Called only when the widget previously requested an animation frame
    /// via `ctx.request_anim_frame()`. Default is a no-op.
    fn anim_frame(&mut self, _event: &AnimFrameEvent, _ctx: &mut AnimCtx<'_>) {}

    /// Event controllers attached to this widget.
    ///
    /// Controllers handle input events (hover, click, drag, scroll, focus)
    /// via the event propagation pipeline. Default returns an empty slice.
    fn controllers(&self) -> &[Box<dyn EventController>] {
        &[]
    }

    /// Mutable access to controllers (for event dispatch).
    ///
    /// The framework calls this during event propagation to deliver events
    /// to each controller. Default returns an empty slice.
    fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>] {
        &mut []
    }

    /// Visual state groups for automatic state resolution and animation.
    ///
    /// Returns `None` if this widget doesn't use visual state management.
    fn visual_states(&self) -> Option<&VisualStateAnimator> {
        None
    }

    /// Mutable access to the visual state animator.
    fn visual_states_mut(&mut self) -> Option<&mut VisualStateAnimator> {
        None
    }

    /// Visits each mutable child widget for tree traversal.
    ///
    /// The framework calls this to walk the widget tree during the pre-paint
    /// pipeline (lifecycle delivery, animation ticks, visual state updates).
    /// Containers override to yield their children; leaf widgets use the
    /// default (no children).
    fn for_each_child_mut(&mut self, _visitor: &mut dyn FnMut(&mut dyn Widget)) {}

    /// Handles input events not consumed by controllers.
    ///
    /// Called by the dispatch pipeline after controller dispatch when no
    /// controller marked the event as handled. Used for widget-internal
    /// interaction logic (e.g., menu item hover tracking) that doesn't fit
    /// the generic controller model. Return `true` if the widget handled the
    /// event.
    fn on_input(
        &mut self,
        _event: &crate::input::InputEvent,
        _bounds: crate::geometry::Rect,
    ) -> OnInputResult {
        OnInputResult::ignored()
    }

    /// Transforms a controller-emitted action into a widget-specific action.
    ///
    /// Called by the dispatch pipeline after a controller on this widget emits
    /// an action. The widget can replace generic actions (e.g., `Clicked`) with
    /// semantic actions (e.g., `OpenDropdown`, `Toggled`) using its own state,
    /// and perform side effects (e.g., toggling internal state, starting
    /// animations). The `bounds` parameter is the widget's layout bounds from
    /// hit testing (used by dropdowns for popup anchor positioning).
    ///
    /// Return `Some(action)` to propagate (original or transformed), or `None`
    /// to suppress the action.
    fn on_action(
        &mut self,
        action: WidgetAction,
        _bounds: crate::geometry::Rect,
    ) -> Option<WidgetAction> {
        Some(action)
    }

    /// Propagates an externally-originated action to a descendant widget.
    ///
    /// Used when an action from a popup overlay (e.g. dropdown menu selection)
    /// needs to update a widget buried in the tree. Containers propagate to
    /// children; leaf widgets check if the action targets them.
    /// Returns `true` if a descendant handled the action.
    fn accept_action(&mut self, _action: &WidgetAction) -> bool {
        false
    }

    /// Collects focusable widget IDs reachable from this widget.
    ///
    /// Leaf widgets return their own ID if focusable; containers override
    /// to recurse into children. Used by the overlay manager to build
    /// modal focus order.
    fn focusable_children(&self) -> Vec<WidgetId> {
        if self.is_focusable() {
            vec![self.id()]
        } else {
            Vec::new()
        }
    }

    /// Declares what interactions this widget cares about.
    ///
    /// Hit testing skips widgets with `Sense::none()`. All production widgets
    /// provide explicit overrides. The default returns `Sense::none()`.
    fn sense(&self) -> Sense {
        Sense::none()
    }

    /// Controls how this widget participates in hit testing relative to
    /// its children.
    ///
    /// The default is `DeferToChild`: children are tested first, and the
    /// widget itself is only hit if no child handles the point.
    fn hit_test_behavior(&self) -> HitTestBehavior {
        HitTestBehavior::DeferToChild
    }
}

#[cfg(test)]
pub(crate) mod tests;
