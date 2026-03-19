//! Composable event controllers for widget interaction.
//!
//! Widgets compose behavior by attaching controller objects instead of
//! implementing monolithic `handle_mouse()` / `handle_hover()` / `handle_key()`
//! methods. Each controller is independently testable and reusable across
//! widget types. Inspired by GTK4's `EventController` architecture.

mod click;
mod drag;
mod focus;
mod hover;
mod scroll;
mod scrub;
mod text_edit;

pub use click::ClickController;
pub use drag::DragController;
pub use focus::FocusController;
pub use hover::HoverController;
pub use scroll::ScrollController;
pub use scrub::ScrubController;
pub use text_edit::TextEditController;

use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::input::{EventPhase, InputEvent};
use crate::interaction::{InteractionState, LifecycleEvent};
use crate::widget_id::WidgetId;

/// A composable event handler attached to a widget.
///
/// Controllers declare which propagation phase they handle and receive
/// input events during that phase. Multiple controllers on the same widget
/// are dispatched in declaration order; all controllers see the event even
/// if an earlier one marks it handled (GTK4 semantics). The handled flag
/// only stops propagation to the next widget in the capture/bubble chain.
pub trait EventController {
    /// Which propagation phase this controller handles.
    fn phase(&self) -> EventPhase {
        EventPhase::Bubble
    }

    /// Handle an input event.
    ///
    /// Two ways to signal "handled" (either is sufficient):
    /// 1. Return `true` (convenience shorthand).
    /// 2. Call `ctx.propagation.set_handled()` (explicit API).
    ///
    /// The dispatch function treats both identically. Remaining controllers
    /// on the SAME widget still run; the handled flag stops propagation to
    /// the NEXT widget in the capture/bubble chain.
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool;

    /// Handle a lifecycle event (hot/active/focus changes).
    fn handle_lifecycle(&mut self, event: &LifecycleEvent, ctx: &mut ControllerCtx<'_>) {
        let _ = (event, ctx);
    }

    /// Reset controller state (e.g., on widget removal or disable).
    ///
    /// Called by the framework on `WidgetRemoved` and
    /// `WidgetDisabled(true)`. Clears internal state only — the framework
    /// handles clearing active capture separately.
    fn reset(&mut self) {}
}

/// Tracks whether the current event has been handled during dispatch.
///
/// Controllers signal handling via `ctx.propagation.set_handled()` or by
/// returning `true` from `handle_event()`. The dispatch function merges
/// both signals.
#[derive(Debug, Default)]
pub struct PropagationState {
    handled: bool,
}

impl PropagationState {
    /// Mark the event as handled. Stops propagation to the next widget.
    pub fn set_handled(&mut self) {
        self.handled = true;
    }

    /// Whether the event has been handled.
    pub fn is_handled(&self) -> bool {
        self.handled
    }
}

/// Context passed to controllers during event dispatch.
///
/// **Bounds during active capture:** When the active widget receives events
/// via capture bypass (Section 03.3), `bounds` may be `Rect::default()`
/// because the widget is outside the hit path. Controllers that compare
/// positions against bounds must use recorded positions (e.g., `press_pos`),
/// NOT bounds containment.
pub struct ControllerCtx<'a> {
    /// The widget this controller is attached to.
    pub widget_id: WidgetId,
    /// The widget's computed bounds from layout.
    pub bounds: Rect,
    /// Interaction state for this widget (hot, active, focused, etc.).
    pub interaction: &'a InteractionState,
    /// Collected actions emitted by controllers. Use `emit_action()`.
    pub actions: &'a mut Vec<WidgetAction>,
    /// Accumulated side-effect requests (set by controller, read by framework).
    pub requests: ControllerRequests,
    /// Current frame timestamp for multi-click timeout checks.
    pub now: Instant,
    /// Propagation control — call `set_handled()` to stop propagation.
    pub propagation: &'a mut PropagationState,
}

impl ControllerCtx<'_> {
    /// Emit a semantic action for the application layer.
    pub fn emit_action(&mut self, action: WidgetAction) {
        self.actions.push(action);
    }
}

/// Side-effect requests accumulated by controllers during dispatch.
///
/// Manual bitmask (same pattern as `Sense` in `sense/mod.rs`). The
/// framework reads these flags after controller dispatch and applies
/// side effects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ControllerRequests(u8);

impl ControllerRequests {
    /// No requests.
    pub const NONE: Self = Self(0);
    /// Request a repaint of this widget.
    pub const PAINT: Self = Self(0b0000_0001);
    /// Request an animation frame callback.
    pub const ANIM_FRAME: Self = Self(0b0000_0010);
    /// Request mouse capture for this widget.
    pub const SET_ACTIVE: Self = Self(0b0000_0100);
    /// Release mouse capture.
    pub const CLEAR_ACTIVE: Self = Self(0b0000_1000);
    /// Request keyboard focus for this widget.
    pub const REQUEST_FOCUS: Self = Self(0b0001_0000);
    /// Advance focus to the next widget in tab order.
    pub const FOCUS_NEXT: Self = Self(0b0010_0000);
    /// Move focus to the previous widget in tab order.
    pub const FOCUS_PREV: Self = Self(0b0100_0000);

    /// Whether this set contains all bits of `other`.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Inserts all bits of `other` into this set.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Combines two request sets (bitwise OR).
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether no requests are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Arguments for constructing a `ControllerCtx`.
///
/// Avoids passing 7+ parameters to `dispatch_to_controllers`.
pub struct ControllerCtxArgs<'a> {
    /// Target widget ID.
    pub widget_id: WidgetId,
    /// Widget bounds from layout.
    pub bounds: Rect,
    /// Interaction state for this widget.
    pub interaction: &'a InteractionState,
    /// Current frame timestamp.
    pub now: Instant,
}

/// Output from `dispatch_to_controllers`.
#[derive(Debug)]
pub struct DispatchOutput {
    /// Accumulated side-effect requests from all controllers.
    pub requests: ControllerRequests,
    /// Semantic actions emitted by controllers.
    pub actions: Vec<WidgetAction>,
    /// Whether any controller marked the event as handled.
    pub handled: bool,
}

/// Dispatches an input event to all controllers on a widget.
///
/// Iterates controllers in declaration order. Only controllers whose
/// `phase()` matches the given `phase` are invoked. If any controller
/// returns `true` or calls `set_handled()`, the event is marked handled
/// for propagation purposes — but remaining controllers on the SAME
/// widget still run (GTK4 semantics).
pub fn dispatch_to_controllers(
    controllers: &mut [Box<dyn EventController>],
    event: &InputEvent,
    phase: EventPhase,
    args: &ControllerCtxArgs<'_>,
) -> DispatchOutput {
    let mut actions = Vec::new();
    let mut requests = ControllerRequests::NONE;
    let mut propagation = PropagationState::default();

    for controller in controllers.iter_mut() {
        // Phase gate: skip controllers that don't match the current phase.
        // Target phase invokes all controllers regardless of declared phase.
        if phase != EventPhase::Target && controller.phase() != phase {
            continue;
        }

        let mut ctx = ControllerCtx {
            widget_id: args.widget_id,
            bounds: args.bounds,
            interaction: args.interaction,
            actions: &mut actions,
            requests: ControllerRequests::NONE,
            now: args.now,
            propagation: &mut propagation,
        };

        let consumed = controller.handle_event(event, &mut ctx);
        requests = requests.union(ctx.requests);

        if consumed {
            propagation.set_handled();
        }
    }

    DispatchOutput {
        requests,
        actions,
        handled: propagation.is_handled(),
    }
}

/// Dispatches a lifecycle event to all controllers on a widget.
///
/// No phase filtering — lifecycle events are not part of the
/// capture/bubble pipeline.
pub fn dispatch_lifecycle_to_controllers(
    controllers: &mut [Box<dyn EventController>],
    event: &LifecycleEvent,
    args: &ControllerCtxArgs<'_>,
) -> ControllerRequests {
    let mut actions = Vec::new();
    let mut requests = ControllerRequests::NONE;
    let mut propagation = PropagationState::default();

    // On WidgetDisabled(true) or WidgetRemoved, call reset() on each controller.
    let should_reset = matches!(
        event,
        LifecycleEvent::WidgetDisabled { disabled: true, .. }
            | LifecycleEvent::WidgetRemoved { .. }
    );

    for controller in controllers.iter_mut() {
        if should_reset {
            controller.reset();
        }

        let mut ctx = ControllerCtx {
            widget_id: args.widget_id,
            bounds: args.bounds,
            interaction: args.interaction,
            actions: &mut actions,
            requests: ControllerRequests::NONE,
            now: args.now,
            propagation: &mut propagation,
        };

        controller.handle_lifecycle(event, &mut ctx);
        requests = requests.union(ctx.requests);
    }

    requests
}

#[cfg(test)]
mod tests;
