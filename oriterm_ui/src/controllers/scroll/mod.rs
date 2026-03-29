//! Scroll controller — handles wheel and trackpad scroll events.
//!
//! Converts `ScrollDelta::Lines` to pixels using a configurable line
//! height, passes `ScrollDelta::Pixels` through as-is. Emits a
//! `ScrollBy` action. Does not set active (scroll is instantaneous).

use crate::action::WidgetAction;
use crate::input::{InputEvent, ScrollDelta};

use super::{ControllerCtx, EventController};

/// Scroll event handling for wheel and trackpad.
///
/// Phase: `Bubble` (default). Children get scroll first; if unhandled,
/// parent scrolls.
#[derive(Debug, Clone)]
pub struct ScrollController {
    /// Pixels per line for `ScrollDelta::Lines` conversion.
    line_height: f32,
}

impl ScrollController {
    /// Creates a new scroll controller with the given line height.
    pub fn new(line_height: f32) -> Self {
        Self { line_height }
    }
}

impl EventController for ScrollController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        if let InputEvent::Scroll { delta, .. } = event {
            let (dx, dy) = match *delta {
                ScrollDelta::Lines { x, y } => (x * self.line_height, y * self.line_height),
                ScrollDelta::Pixels { x, y } => (x, y),
            };
            ctx.emit_action(WidgetAction::ScrollBy {
                id: ctx.widget_id,
                delta_x: dx,
                delta_y: dy,
            });
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests;
