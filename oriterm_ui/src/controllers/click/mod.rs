//! Click controller — single/double/triple click recognition.
//!
//! Handles click detection with configurable distance threshold and
//! multi-click timeout. Works cooperatively with `DragController`:
//! when distance exceeds the click threshold during a press, the click
//! is cancelled so `DragController` can take over.

use std::time::{Duration, Instant};

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, MouseButton};
use crate::interaction::LifecycleEvent;

use super::{ControllerCtx, ControllerRequests, EventController};

/// Click recognition with single/double/triple click detection.
///
/// On `MouseDown`: records position and time, requests mouse capture.
/// On `MouseUp`: if distance from press is within threshold, emits
/// `Clicked` (and `DoubleClicked`/`TripleClicked` for rapid presses).
/// On `MouseMove`: if distance exceeds threshold, cancels the pending
/// click (allows `DragController` to take over).
#[derive(Debug)]
pub struct ClickController {
    /// Accumulated click count (resets after timeout or movement).
    click_count: u32,
    /// Position of the initial mouse-down.
    press_pos: Option<Point>,
    /// Time of last mouse-down (for double-click detection).
    last_press: Option<Instant>,
    /// Max distance from press to release for a valid click (px).
    click_threshold: f32,
    /// Max time between clicks for multi-click (ms).
    multi_click_timeout: Duration,
}

impl ClickController {
    /// Creates a new click controller with sensible defaults.
    ///
    /// Default threshold: 4.0 px. Default multi-click timeout: 500 ms.
    pub fn new() -> Self {
        Self {
            click_count: 0,
            press_pos: None,
            last_press: None,
            click_threshold: 4.0,
            multi_click_timeout: Duration::from_millis(500),
        }
    }

    /// Sets the max distance from press to release for a valid click.
    #[must_use]
    pub fn with_threshold(mut self, px: f32) -> Self {
        self.click_threshold = px;
        self
    }

    /// Sets the max time between clicks for multi-click detection.
    #[must_use]
    pub fn with_multi_click_timeout(mut self, timeout: Duration) -> Self {
        self.multi_click_timeout = timeout;
        self
    }

    /// Distance between two points (Euclidean).
    fn distance(a: Point, b: Point) -> f32 {
        (a.x - b.x).hypot(a.y - b.y)
    }
}

impl Default for ClickController {
    fn default() -> Self {
        Self::new()
    }
}

impl EventController for ClickController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                // Check multi-click timeout BEFORE updating last_press.
                // If too long since the previous press, reset the count.
                if let Some(last) = self.last_press {
                    if ctx.now.duration_since(last) > self.multi_click_timeout {
                        self.click_count = 0;
                    }
                }

                self.press_pos = Some(*pos);
                self.last_press = Some(ctx.now);
                ctx.requests.insert(ControllerRequests::SET_ACTIVE);
                ctx.requests.insert(ControllerRequests::PAINT);
                true
            }

            InputEvent::MouseUp {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                ctx.requests.insert(ControllerRequests::CLEAR_ACTIVE);
                ctx.requests.insert(ControllerRequests::PAINT);

                let Some(press) = self.press_pos.take() else {
                    // Click was cancelled (drag exceeded threshold).
                    return true;
                };

                if Self::distance(press, *pos) > self.click_threshold {
                    // Too far — not a click.
                    self.click_count = 0;
                    return true;
                }

                self.click_count += 1;

                match self.click_count {
                    1 => ctx.emit_action(WidgetAction::Clicked(ctx.widget_id)),
                    2 => ctx.emit_action(WidgetAction::DoubleClicked(ctx.widget_id)),
                    3 => {
                        ctx.emit_action(WidgetAction::TripleClicked(ctx.widget_id));
                        self.click_count = 0; // Reset after triple.
                    }
                    _ => self.click_count = 0,
                }

                true
            }

            InputEvent::MouseMove { pos, .. } => {
                // If we have a pending press and moved too far, cancel the click.
                if let Some(press) = self.press_pos {
                    if Self::distance(press, *pos) > self.click_threshold {
                        // Cancel click. Widget stays active (captured) so a
                        // co-located DragController can take over.
                        self.press_pos = None;
                    }
                }
                false
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
        self.click_count = 0;
        self.press_pos = None;
        self.last_press = None;
    }
}

#[cfg(test)]
mod tests;
