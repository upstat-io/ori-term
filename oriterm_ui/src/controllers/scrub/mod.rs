//! Scrub controller — immediate drag without threshold.
//!
//! Like [`DragController`] but fires `DragStart` on `MouseDown` (not on the
//! first move past a threshold). Designed for sliders and scrollbar thumbs
//! where the interaction begins immediately at the press position.

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, MouseButton};
use crate::interaction::LifecycleEvent;

use super::{ControllerCtx, ControllerRequests, EventController};

/// Immediate-start drag controller for scrub interactions.
///
/// On `MouseDown`: emits `DragStart` and requests mouse capture.
/// On `MouseMove` (while pressed): emits `DragUpdate` with delta and
/// total delta from the press position.
/// On `MouseUp`: emits `DragEnd` and releases capture.
///
/// Unlike [`DragController`], there is no distance threshold — the drag
/// begins at the press position. This makes it suitable for sliders where
/// the value should update immediately on click.
#[derive(Debug)]
pub struct ScrubController {
    /// Position at press, `None` when idle.
    press_pos: Option<Point>,
    /// Most recent position during drag.
    last_pos: Point,
}

impl ScrubController {
    /// Creates a new scrub controller.
    pub fn new() -> Self {
        Self {
            press_pos: None,
            last_pos: Point::new(0.0, 0.0),
        }
    }
}

impl Default for ScrubController {
    fn default() -> Self {
        Self::new()
    }
}

impl EventController for ScrubController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                self.press_pos = Some(*pos);
                self.last_pos = *pos;
                ctx.emit_action(WidgetAction::DragStart {
                    id: ctx.widget_id,
                    pos: *pos,
                });
                ctx.requests.insert(ControllerRequests::SET_ACTIVE);
                ctx.requests.insert(ControllerRequests::PAINT);
                true
            }

            InputEvent::MouseMove { pos, .. } => {
                if let Some(start) = self.press_pos {
                    let delta = Point::new(pos.x - self.last_pos.x, pos.y - self.last_pos.y);
                    let total_delta = Point::new(pos.x - start.x, pos.y - start.y);
                    self.last_pos = *pos;
                    ctx.emit_action(WidgetAction::DragUpdate {
                        id: ctx.widget_id,
                        delta,
                        total_delta,
                    });
                    ctx.requests.insert(ControllerRequests::PAINT);
                    true
                } else {
                    false
                }
            }

            InputEvent::MouseUp {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                if self.press_pos.take().is_some() {
                    ctx.emit_action(WidgetAction::DragEnd {
                        id: ctx.widget_id,
                        pos: *pos,
                    });
                    ctx.requests.insert(ControllerRequests::CLEAR_ACTIVE);
                    ctx.requests.insert(ControllerRequests::PAINT);
                }
                true
            }

            _ => false,
        }
    }

    fn handle_lifecycle(&mut self, event: &LifecycleEvent, _ctx: &mut ControllerCtx<'_>) {
        if matches!(event, LifecycleEvent::WidgetDisabled { disabled: true, .. }) {
            self.reset();
        }
    }

    fn reset(&mut self) {
        self.press_pos = None;
    }
}

#[cfg(test)]
mod tests;
