//! Key activation controller — Enter/Space triggers `Clicked`.
//!
//! Converts keyboard activation (Enter or Space key press) into a
//! `Clicked` action, matching the standard platform convention for
//! activating focused interactive controls. Used by buttons, toggles,
//! and checkboxes.

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key};

use super::{ControllerCtx, ControllerRequests, EventController};

/// Keyboard activation controller for focused interactive widgets.
///
/// On `KeyDown(Enter)` or `KeyDown(Space)`, emits `Clicked(widget_id)`
/// and requests a repaint. Consumes the corresponding `KeyUp` to prevent
/// the event from leaking to parent widgets.
#[derive(Debug, Clone, Default)]
pub struct KeyActivationController;

impl KeyActivationController {
    /// Creates a new key activation controller.
    pub fn new() -> Self {
        Self
    }

    /// Returns `true` if this key triggers activation.
    fn is_activation_key(key: Key) -> bool {
        matches!(key, Key::Enter | Key::Space)
    }
}

impl EventController for KeyActivationController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::KeyDown { key, .. } if Self::is_activation_key(*key) => {
                ctx.emit_action(WidgetAction::Clicked(ctx.widget_id));
                ctx.requests.insert(ControllerRequests::PAINT);
                true
            }
            // Consume KeyUp for activation keys to prevent leaking.
            InputEvent::KeyUp { key, .. } if Self::is_activation_key(*key) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests;
