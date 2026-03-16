//! Focus controller — keyboard focus management with tab navigation.
//!
//! Handles Tab/Shift+Tab focus cycling and click-to-focus. Communicates
//! via `ControllerRequests` flags rather than calling `FocusManager`
//! directly — the framework reads the flags after dispatch and applies
//! them.

use crate::input::{EventPhase, InputEvent, Key};
use crate::interaction::LifecycleEvent;

use super::{ControllerCtx, ControllerRequests, EventController};

/// Keyboard focus management with tab navigation.
///
/// `tab_index` is metadata for the framework: `None` means natural tree
/// order, `Some(n)` sorts by `n` (lower = earlier). The sorting happens
/// in `FocusManager::set_focus_order()`, not inside this controller.
#[derive(Debug, Clone)]
pub struct FocusController {
    /// Tab index for focus ordering (lower = earlier).
    tab_index: Option<i32>,
}

impl FocusController {
    /// Creates a focus controller using natural tree order.
    pub fn new() -> Self {
        Self { tab_index: None }
    }

    /// Sets a specific tab index for focus ordering.
    #[must_use]
    pub fn with_tab_index(mut self, index: i32) -> Self {
        self.tab_index = Some(index);
        self
    }

    /// Returns the tab index, if set.
    pub fn tab_index(&self) -> Option<i32> {
        self.tab_index
    }
}

impl Default for FocusController {
    fn default() -> Self {
        Self::new()
    }
}

impl EventController for FocusController {
    fn phase(&self) -> EventPhase {
        // Focus controller runs in Bubble phase (default), but we override
        // to make it explicit.
        EventPhase::Bubble
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::KeyDown {
                key: Key::Tab,
                modifiers,
            } => {
                if modifiers.shift() {
                    ctx.requests.insert(ControllerRequests::FOCUS_PREV);
                } else {
                    ctx.requests.insert(ControllerRequests::FOCUS_NEXT);
                }
                true
            }
            // Consume the matching KeyUp to prevent it from leaking to
            // parent widgets as an unmatched key-up event.
            InputEvent::KeyUp { key: Key::Tab, .. } => true,
            // Click-to-focus: request focus when the widget is clicked.
            InputEvent::MouseDown { .. } => {
                ctx.requests.insert(ControllerRequests::REQUEST_FOCUS);
                // Don't consume — let ClickController also handle the press.
                false
            }
            _ => false,
        }
    }

    fn handle_lifecycle(&mut self, event: &LifecycleEvent, ctx: &mut ControllerCtx<'_>) {
        if matches!(event, LifecycleEvent::FocusChanged { .. }) {
            // Repaint so the widget shows its focused/unfocused visual state.
            ctx.requests.insert(ControllerRequests::PAINT);
        }
    }
}

#[cfg(test)]
mod tests;
