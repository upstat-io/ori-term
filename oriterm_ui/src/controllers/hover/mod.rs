//! Hover controller — tracks enter/leave and emits actions.
//!
//! Replaces all manual `hovered: bool` + `HoverEvent::Enter/Leave` tracking
//! in individual widgets. Purely observational: does not set active or
//! capture events.

use crate::action::WidgetAction;
use crate::input::InputEvent;
use crate::interaction::LifecycleEvent;

use super::{ControllerCtx, ControllerRequests, EventController};

/// Tracks hover enter/leave and optionally emits actions.
///
/// Responds to `LifecycleEvent::HotChanged` (not raw mouse events).
/// When `on_move` is `true`, also requests repaint on `MouseMove`.
#[derive(Debug, Clone)]
pub struct HoverController {
    /// Action emitted when the pointer enters.
    enter_action: Option<WidgetAction>,
    /// Action emitted when the pointer leaves.
    leave_action: Option<WidgetAction>,
    /// If `true`, request repaint on continuous pointer movement.
    track_move: bool,
}

impl HoverController {
    /// Creates a new hover controller with no actions and no move tracking.
    pub fn new() -> Self {
        Self {
            enter_action: None,
            leave_action: None,
            track_move: false,
        }
    }

    /// Sets the action emitted on pointer enter.
    #[must_use]
    pub fn with_on_enter(mut self, action: WidgetAction) -> Self {
        self.enter_action = Some(action);
        self
    }

    /// Sets the action emitted on pointer leave.
    #[must_use]
    pub fn with_on_leave(mut self, action: WidgetAction) -> Self {
        self.leave_action = Some(action);
        self
    }

    /// Enables repaint on continuous pointer movement while hovered.
    #[must_use]
    pub fn with_on_move(mut self) -> Self {
        self.track_move = true;
        self
    }
}

impl Default for HoverController {
    fn default() -> Self {
        Self::new()
    }
}

impl EventController for HoverController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        if self.track_move {
            // Only fire on bare moves (not during drag — MouseDown events aren't ours).
            if matches!(event, InputEvent::MouseMove { .. }) {
                ctx.requests.insert(ControllerRequests::PAINT);
            }
        }
        // Hover controller never consumes events — purely observational.
        false
    }

    fn handle_lifecycle(&mut self, event: &LifecycleEvent, ctx: &mut ControllerCtx<'_>) {
        match event {
            LifecycleEvent::HotChanged { is_hot: true, .. } => {
                if let Some(action) = &self.enter_action {
                    ctx.emit_action(action.clone());
                }
                ctx.requests.insert(ControllerRequests::PAINT);
            }
            LifecycleEvent::HotChanged { is_hot: false, .. } => {
                if let Some(action) = &self.leave_action {
                    ctx.emit_action(action.clone());
                }
                ctx.requests.insert(ControllerRequests::PAINT);
            }
            LifecycleEvent::WidgetDisabled { disabled: true, .. } => {
                // Framework calls reset() separately; we just request repaint
                // so the widget redraws without hover state.
                ctx.requests.insert(ControllerRequests::PAINT);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
