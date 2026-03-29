//! Drag controller — drag recognition with threshold.
//!
//! Tracks a three-state machine: `Idle` -> `Pending` -> `Dragging`.
//! Emits `DragStart`, `DragUpdate`, and `DragEnd` actions. Works
//! cooperatively with `ClickController`: if the drag threshold is not
//! exceeded, the mouse-up produces a click (handled by `ClickController`),
//! not a drag.

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, MouseButton};
use crate::interaction::LifecycleEvent;

use super::{ControllerCtx, ControllerRequests, EventController};

/// Drag recognition with configurable threshold.
///
/// State machine: `Idle` -> (`MouseDown`) -> `Pending` -> (move > threshold)
/// -> `Dragging` -> (`MouseUp`) -> `Idle`. If the threshold is never exceeded,
/// `Pending` -> (`MouseUp`) -> `Idle` with no drag events emitted.
#[derive(Debug)]
pub struct DragController {
    state: DragState,
    /// Minimum distance before drag begins (prevents accidental drags).
    threshold: f32,
}

/// Internal state machine for drag tracking.
#[derive(Debug)]
enum DragState {
    /// No drag in progress.
    Idle,
    /// Mouse down, waiting for threshold to be exceeded.
    Pending { press_pos: Point },
    /// Drag in progress.
    Dragging {
        /// Position where the threshold was exceeded.
        start_pos: Point,
        /// Most recent `MouseMove` position.
        last_pos: Point,
    },
}

impl DragController {
    /// Creates a new drag controller with the default threshold (4.0 px).
    pub fn new() -> Self {
        Self {
            state: DragState::Idle,
            threshold: 4.0,
        }
    }

    /// Sets the minimum distance before drag begins.
    #[must_use]
    pub fn with_threshold(mut self, px: f32) -> Self {
        self.threshold = px;
        self
    }

    /// Distance between two points (Euclidean).
    fn distance(a: Point, b: Point) -> f32 {
        (a.x - b.x).hypot(a.y - b.y)
    }
}

impl Default for DragController {
    fn default() -> Self {
        Self::new()
    }
}

impl EventController for DragController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                self.state = DragState::Pending { press_pos: *pos };
                ctx.requests.insert(ControllerRequests::SET_ACTIVE);
                true
            }

            InputEvent::MouseMove { pos, .. } => match self.state {
                DragState::Pending { press_pos } => {
                    if Self::distance(press_pos, *pos) > self.threshold {
                        self.state = DragState::Dragging {
                            start_pos: *pos,
                            last_pos: *pos,
                        };
                        ctx.emit_action(WidgetAction::DragStart {
                            id: ctx.widget_id,
                            pos: *pos,
                        });
                    }
                    true
                }
                DragState::Dragging {
                    start_pos,
                    ref mut last_pos,
                } => {
                    let delta = Point::new(pos.x - last_pos.x, pos.y - last_pos.y);
                    let total_delta = Point::new(pos.x - start_pos.x, pos.y - start_pos.y);
                    *last_pos = *pos;
                    ctx.emit_action(WidgetAction::DragUpdate {
                        id: ctx.widget_id,
                        delta,
                        total_delta,
                    });
                    true
                }
                DragState::Idle => false,
            },

            InputEvent::MouseUp {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                let was_dragging = matches!(self.state, DragState::Dragging { .. });
                self.state = DragState::Idle;
                ctx.requests.insert(ControllerRequests::CLEAR_ACTIVE);

                if was_dragging {
                    ctx.emit_action(WidgetAction::DragEnd {
                        id: ctx.widget_id,
                        pos: *pos,
                    });
                }
                // If Pending, emit nothing — ClickController handles it.
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
        self.state = DragState::Idle;
    }
}

#[cfg(test)]
mod tests;
