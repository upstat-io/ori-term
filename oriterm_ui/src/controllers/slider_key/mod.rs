//! Slider keyboard controller — arrow/Home/End value adjustment.
//!
//! Handles arrow key stepping and Home/End jumps for slider widgets.
//! Owns a copy of the slider parameters (value, min, max, step) and
//! emits `ValueChanged` when the value is adjusted. The widget must
//! sync controller state when parameters change externally.

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key};

use super::{ControllerCtx, ControllerRequests, EventController};

/// Keyboard controller for slider value adjustment.
///
/// Arrow Up/Right increments by `step`. Arrow Down/Left decrements.
/// Home jumps to `min`, End jumps to `max`. Emits `ValueChanged`
/// when the value changes.
#[derive(Debug, Clone)]
pub struct SliderKeyController {
    /// Current value (synced from widget).
    value: f32,
    /// Minimum value.
    min: f32,
    /// Maximum value.
    max: f32,
    /// Step increment.
    step: f32,
}

impl SliderKeyController {
    /// Creates a slider key controller with the given parameters.
    pub fn new(min: f32, max: f32, step: f32) -> Self {
        Self {
            value: min,
            min,
            max,
            step,
        }
    }

    /// Returns the current value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Syncs the value from the widget.
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }

    /// Updates the range and step, clamping the value.
    pub fn set_range(&mut self, min: f32, max: f32, step: f32) {
        self.min = min;
        self.max = max;
        self.step = step;
        self.value = self.value.clamp(min, max);
    }

    /// Snaps a raw value to the nearest step.
    fn snap_to_step(&self, raw: f32) -> f32 {
        if self.step <= 0.0 {
            return raw.clamp(self.min, self.max);
        }
        let steps = ((raw - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    /// Tries to set a new value, emitting `ValueChanged` if it changed.
    fn try_set(&mut self, new_value: f32, ctx: &mut ControllerCtx<'_>) -> bool {
        let clamped = new_value.clamp(self.min, self.max);
        if (clamped - self.value).abs() < f32::EPSILON {
            return true; // Key handled, value unchanged.
        }
        self.value = clamped;
        ctx.emit_action(WidgetAction::ValueChanged {
            id: ctx.widget_id,
            value: self.value,
        });
        ctx.requests.insert(ControllerRequests::PAINT);
        true
    }
}

impl EventController for SliderKeyController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::KeyDown { key, .. } => match key {
                Key::ArrowRight | Key::ArrowUp => {
                    let v = self.snap_to_step(self.value + self.step);
                    self.try_set(v, ctx)
                }
                Key::ArrowLeft | Key::ArrowDown => {
                    let v = self.snap_to_step(self.value - self.step);
                    self.try_set(v, ctx)
                }
                Key::Home => self.try_set(self.min, ctx),
                Key::End => self.try_set(self.max, ctx),
                _ => false,
            },
            // Consume KeyUp for handled keys.
            InputEvent::KeyUp { key, .. } => matches!(
                key,
                Key::ArrowRight
                    | Key::ArrowLeft
                    | Key::ArrowUp
                    | Key::ArrowDown
                    | Key::Home
                    | Key::End
            ),
            _ => false,
        }
    }

    fn reset(&mut self) {
        // Value is synced from widget; nothing to reset.
    }
}

#[cfg(test)]
mod tests;
