//! Capture-phase controller for overlay scrollbar drag interaction.
//!
//! Runs in the Capture phase so it intercepts scrollbar clicks before
//! child widgets see them. Returns `handled=false` for clicks outside
//! the scrollbar area, allowing normal child interaction.
//!
//! Shares scrollbar geometry with the owning widget via
//! [`SharedScrollbarHitZones`]. The widget updates the shared zones
//! during paint; the controller reads them during event handling.

use crate::action::WidgetAction;
use crate::controllers::{ControllerCtx, ControllerRequests, EventController};
use crate::geometry::Point;
use crate::input::{EventPhase, InputEvent, MouseButton};
use crate::widgets::scrollbar::SharedScrollbarHitZones;

/// Drag state for the scrollbar capture controller.
#[derive(Debug)]
enum DragState {
    /// No active drag.
    Idle,
    /// Thumb is being dragged.
    ThumbDrag { press_pos: Point, last_pos: Point },
}

/// Capture-phase controller that intercepts scrollbar press/drag/release.
///
/// For overlay scrollbars that exist only in the paint layer (not the
/// layout/hit-test tree). Without Capture-phase interception, clicks on
/// the scrollbar region would reach the child widget instead.
///
/// **Hover tracking** remains in the widget's `on_input()` — only
/// press/drag/release flows through this controller.
pub struct ScrollbarCaptureController {
    hit_zones: SharedScrollbarHitZones,
    state: DragState,
}

impl ScrollbarCaptureController {
    /// Create a new controller with shared hit-zone geometry.
    pub fn new(hit_zones: SharedScrollbarHitZones) -> Self {
        Self {
            hit_zones,
            state: DragState::Idle,
        }
    }
}

impl EventController for ScrollbarCaptureController {
    fn phase(&self) -> EventPhase {
        EventPhase::Capture
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                let zones = self.hit_zones.borrow();

                // Check vertical thumb first, then track.
                if let Some(ref r) = zones.v_thumb_hit {
                    if r.contains(*pos) {
                        self.state = DragState::ThumbDrag {
                            press_pos: *pos,
                            last_pos: *pos,
                        };
                        ctx.emit_action(WidgetAction::DragStart {
                            id: ctx.widget_id,
                            pos: *pos,
                        });
                        ctx.requests.insert(
                            ControllerRequests::SET_ACTIVE.union(ControllerRequests::PAINT),
                        );
                        return true;
                    }
                }
                if let Some(ref r) = zones.v_track_hit {
                    if r.contains(*pos) {
                        // Track click: jump to position, no capture.
                        ctx.emit_action(WidgetAction::DragStart {
                            id: ctx.widget_id,
                            pos: *pos,
                        });
                        ctx.requests.insert(ControllerRequests::PAINT);
                        return true;
                    }
                }

                // Check horizontal thumb, then track.
                if let Some(ref r) = zones.h_thumb_hit {
                    if r.contains(*pos) {
                        self.state = DragState::ThumbDrag {
                            press_pos: *pos,
                            last_pos: *pos,
                        };
                        ctx.emit_action(WidgetAction::DragStart {
                            id: ctx.widget_id,
                            pos: *pos,
                        });
                        ctx.requests.insert(
                            ControllerRequests::SET_ACTIVE.union(ControllerRequests::PAINT),
                        );
                        return true;
                    }
                }
                if let Some(ref r) = zones.h_track_hit {
                    if r.contains(*pos) {
                        ctx.emit_action(WidgetAction::DragStart {
                            id: ctx.widget_id,
                            pos: *pos,
                        });
                        ctx.requests.insert(ControllerRequests::PAINT);
                        return true;
                    }
                }

                false
            }

            InputEvent::MouseMove { pos, .. } => {
                if let DragState::ThumbDrag {
                    last_pos,
                    press_pos,
                    ..
                } = &mut self.state
                {
                    let delta = Point::new(pos.x - last_pos.x, pos.y - last_pos.y);
                    let total_delta = Point::new(pos.x - press_pos.x, pos.y - press_pos.y);
                    *last_pos = *pos;
                    ctx.emit_action(WidgetAction::DragUpdate {
                        id: ctx.widget_id,
                        delta,
                        total_delta,
                    });
                    ctx.requests.insert(ControllerRequests::PAINT);
                    return true;
                }
                false
            }

            InputEvent::MouseUp {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                if matches!(self.state, DragState::ThumbDrag { .. }) {
                    self.state = DragState::Idle;
                    ctx.emit_action(WidgetAction::DragEnd {
                        id: ctx.widget_id,
                        pos: *pos,
                    });
                    ctx.requests
                        .insert(ControllerRequests::CLEAR_ACTIVE.union(ControllerRequests::PAINT));
                    return true;
                }
                false
            }

            _ => false,
        }
    }

    fn reset(&mut self) {
        self.state = DragState::Idle;
    }
}

#[cfg(test)]
mod tests;
