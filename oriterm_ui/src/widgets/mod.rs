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
use crate::input::{EventResponse, HoverEvent, KeyEvent, MouseEvent, MouseEventKind};
use crate::interaction::LifecycleEvent;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

pub use contexts::{AnimCtx, DrawCtx, EventCtx, LayoutCtx, LifecycleCtx};
pub use text_measurer::TextMeasurer;

/// Whether a widget wants to acquire or release mouse capture.
///
/// Capture is a routing directive: when a widget acquires capture, all
/// subsequent mouse events (Move, Up) are routed to that widget regardless
/// of cursor position, until capture is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureRequest {
    /// No capture change requested.
    #[default]
    None,
    /// Request mouse capture for the responding widget.
    Acquire,
    /// Release any active mouse capture.
    Release,
}

impl CaptureRequest {
    /// Whether the capture should be released based on the response and event.
    ///
    /// Returns `true` on explicit `Release`, or on `None` with a mouse-up
    /// event (implicit release when the child didn't request anything).
    pub fn should_release(self, event_kind: &MouseEventKind) -> bool {
        matches!(self, Self::Release)
            || (matches!(self, Self::None) && matches!(event_kind, MouseEventKind::Up(_)))
    }
}

/// How a widget responded to an event, including an optional semantic action.
///
/// Widgets return this from event handlers. The `response` field tells the
/// framework how to handle propagation; the `action` field carries semantic
/// meaning for the application layer.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetResponse {
    /// How the framework should handle this event.
    pub response: EventResponse,
    /// Optional semantic action for the application layer to interpret.
    pub action: Option<WidgetAction>,
    /// Whether the widget wants to acquire or release mouse capture.
    pub capture: CaptureRequest,
    /// The widget that produced this response (for invalidation tracking).
    ///
    /// Set by container-side injection: containers fill this with the child's
    /// `WidgetId` after receiving a response. Widgets themselves leave it `None`.
    pub source: Option<WidgetId>,
}

impl WidgetResponse {
    /// Event handled, no action emitted.
    pub fn handled() -> Self {
        Self {
            response: EventResponse::Handled,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Event ignored — propagate to parent.
    pub fn ignored() -> Self {
        Self {
            response: EventResponse::Ignored,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Visual-only change (hover color, focus ring). Repaint needed, no relayout.
    pub fn paint() -> Self {
        Self {
            response: EventResponse::RequestPaint,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Structural change (text content, visibility). Relayout + repaint needed.
    pub fn layout() -> Self {
        Self {
            response: EventResponse::RequestLayout,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Event handled, focus requested, no action.
    pub fn focus() -> Self {
        Self {
            response: EventResponse::RequestFocus,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Attaches an action to this response.
    #[must_use]
    pub fn with_action(mut self, action: WidgetAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Requests mouse capture for the responding widget.
    #[must_use]
    pub fn with_capture(mut self) -> Self {
        self.capture = CaptureRequest::Acquire;
        self
    }

    /// Requests release of any active mouse capture.
    #[must_use]
    pub fn with_release_capture(mut self) -> Self {
        self.capture = CaptureRequest::Release;
        self
    }

    /// Sets the source widget ID (for invalidation tracking).
    ///
    /// Normally set by container-side injection rather than individual widgets.
    #[must_use]
    pub fn with_source(mut self, id: WidgetId) -> Self {
        self.source = Some(id);
        self
    }

    /// Sets the source widget ID if not already set.
    ///
    /// Used by containers to inject the child's ID without overwriting
    /// a source that a nested container already set.
    pub fn inject_source(&mut self, id: WidgetId) {
        if self.source.is_none() {
            self.source = Some(id);
        }
    }

    /// Marks the tracker if this response carries a dirty source.
    ///
    /// Convenience for the application layer: extracts `source` and
    /// `DirtyKind` from the response and calls `tracker.mark()`.
    pub fn mark_tracker(&self, tracker: &mut crate::invalidation::InvalidationTracker) {
        if let Some(source) = self.source {
            let kind = crate::invalidation::DirtyKind::from(self.response);
            tracker.mark(source, kind);
        }
    }
}

// `WidgetAction` lives in `crate::action` to avoid a circular dependency
// (`controllers -> widgets`). Re-exported here for backward compatibility.
pub use crate::action::WidgetAction;

/// The core widget trait.
///
/// Each widget is a concrete struct that implements this trait. Widgets
/// own their visual state (hovered, pressed) and app state (checked, value),
/// plus a style struct with `Default` dark-theme defaults.
///
/// The trait is in transition: new methods (`paint`, `lifecycle`, `anim_frame`,
/// `controllers`, `visual_states`) coexist with legacy methods (`draw`,
/// `handle_mouse`, `handle_hover`, `handle_key`) during migration. Legacy
/// methods are removed after all widgets are migrated (Section 08.6).
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
    ///
    /// Default forwards to `draw()` for backward compatibility during
    /// migration. Override this and stop implementing `draw()`.
    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        #[allow(deprecated)]
        self.draw(ctx);
    }

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
    ) -> bool {
        false
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

    // --- Legacy methods (deprecated, removed in Section 08.6) ---

    /// Draws the widget into the draw list.
    ///
    /// Deprecated: implement `paint()` instead. This method exists only for
    /// backward compatibility during migration.
    #[deprecated(note = "implement paint() instead — removed in Section 08.6")]
    fn draw(&self, _ctx: &mut DrawCtx<'_>) {}

    /// Handles a mouse event. Returns a response with optional action.
    ///
    /// Deprecated: use event controllers instead.
    fn handle_mouse(&mut self, _event: &MouseEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    /// Handles a synthetic hover event (enter/leave).
    ///
    /// Deprecated: use `HoverController` and `LifecycleEvent::HotChanged`.
    fn handle_hover(&mut self, _event: HoverEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
    }

    /// Handles a keyboard event. Returns a response with optional action.
    ///
    /// Deprecated: use `FocusController` or custom controllers.
    fn handle_key(&mut self, _event: KeyEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        WidgetResponse::ignored()
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
    /// Hit testing skips widgets with `Sense::none()`. The default returns
    /// `Sense::all()` for backward compatibility — changed to `Sense::none()`
    /// after all widgets provide explicit overrides.
    fn sense(&self) -> Sense {
        Sense::all()
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
