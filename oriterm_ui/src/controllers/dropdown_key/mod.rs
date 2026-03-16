//! Dropdown keyboard controller — arrow navigation and confirm/dismiss.
//!
//! Handles Up/Down arrow cycling, Enter confirmation, and Escape dismissal
//! for the dropdown trigger widget. Owns the selected index and items count
//! to compute wrap-around navigation independently.

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key};

use super::{ControllerCtx, ControllerRequests, EventController};

/// Keyboard controller for dropdown item cycling.
///
/// Arrow Up/Down cycle through items with wrap-around. Enter confirms
/// the current selection. Escape dismisses the overlay.
#[derive(Debug, Clone)]
pub struct DropdownKeyController {
    /// Total number of items in the dropdown.
    items_count: usize,
    /// Currently selected item index.
    selected: usize,
}

impl DropdownKeyController {
    /// Creates a dropdown key controller with the given item count.
    ///
    /// Panics if `items_count` is zero.
    pub fn new(items_count: usize) -> Self {
        assert!(items_count > 0, "dropdown requires at least one item");
        Self {
            items_count,
            selected: 0,
        }
    }

    /// Returns the currently selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Sets the selected index, clamping to valid range.
    pub fn set_selected(&mut self, index: usize) {
        self.selected = index.min(self.items_count - 1);
    }

    /// Updates the total item count and clamps the selection.
    pub fn set_items_count(&mut self, count: usize) {
        assert!(count > 0, "dropdown requires at least one item");
        self.items_count = count;
        self.selected = self.selected.min(count - 1);
    }

    /// Selects the next item, wrapping at the end.
    fn select_next(&mut self) {
        self.selected = (self.selected + 1) % self.items_count;
    }

    /// Selects the previous item, wrapping at the start.
    fn select_prev(&mut self) {
        self.selected = if self.selected == 0 {
            self.items_count - 1
        } else {
            self.selected - 1
        };
    }
}

impl EventController for DropdownKeyController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::KeyDown { key, .. } => match key {
                Key::ArrowDown => {
                    self.select_next();
                    ctx.emit_action(WidgetAction::Selected {
                        id: ctx.widget_id,
                        index: self.selected,
                    });
                    ctx.requests.insert(ControllerRequests::PAINT);
                    true
                }
                Key::ArrowUp => {
                    self.select_prev();
                    ctx.emit_action(WidgetAction::Selected {
                        id: ctx.widget_id,
                        index: self.selected,
                    });
                    ctx.requests.insert(ControllerRequests::PAINT);
                    true
                }
                Key::Enter => {
                    ctx.emit_action(WidgetAction::Selected {
                        id: ctx.widget_id,
                        index: self.selected,
                    });
                    true
                }
                Key::Escape => {
                    ctx.emit_action(WidgetAction::DismissOverlay(ctx.widget_id));
                    true
                }
                _ => false,
            },
            // Consume KeyUp for keys we handle.
            InputEvent::KeyUp { key, .. } => matches!(
                key,
                Key::ArrowDown | Key::ArrowUp | Key::Enter | Key::Escape
            ),
            _ => false,
        }
    }

    fn reset(&mut self) {
        self.selected = 0;
    }
}

#[cfg(test)]
mod tests;
