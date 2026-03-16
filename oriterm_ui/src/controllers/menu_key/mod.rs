//! Menu keyboard controller — arrow navigation, selection, and dismiss.
//!
//! Handles ArrowUp/Down navigation through clickable items (skipping
//! separators), Enter/Space item activation, and Escape overlay dismissal.
//! Designed for `MenuWidget` (Section 08.4 migration).

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key};

use super::{ControllerCtx, ControllerRequests, EventController};

/// Keyboard controller for menu item navigation.
///
/// Tracks the currently highlighted item and navigates only to clickable
/// entries (skipping separators). Enter/Space activates the highlighted
/// item. Escape dismisses the overlay.
#[derive(Debug, Clone)]
pub struct MenuKeyController {
    /// Currently highlighted entry index.
    hovered: Option<usize>,
    /// Indices of clickable entries (excluding separators).
    clickable: Vec<usize>,
}

impl MenuKeyController {
    /// Creates a menu key controller for entries with the given clickable indices.
    ///
    /// `clickable_indices` should contain the indices of all non-separator
    /// entries, in order.
    pub fn new(clickable_indices: Vec<usize>) -> Self {
        Self {
            hovered: None,
            clickable: clickable_indices,
        }
    }

    /// Returns the currently highlighted entry index.
    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    /// Sets the highlighted entry index.
    pub fn set_hovered(&mut self, index: Option<usize>) {
        self.hovered = index;
    }

    /// Navigates to the next clickable entry in the given direction.
    /// Returns `true` if the hover changed.
    fn navigate(&mut self, forward: bool) -> bool {
        if self.clickable.is_empty() {
            return false;
        }

        let next = match self.hovered {
            Some(current) => {
                let pos = self.clickable.iter().position(|&i| i == current);
                match pos {
                    Some(p) => {
                        if forward {
                            self.clickable[(p + 1) % self.clickable.len()]
                        } else {
                            self.clickable[(p + self.clickable.len() - 1) % self.clickable.len()]
                        }
                    }
                    // Current hover isn't clickable; start from edge.
                    None => {
                        if forward {
                            self.clickable[0]
                        } else {
                            self.clickable[self.clickable.len() - 1]
                        }
                    }
                }
            }
            None => {
                if forward {
                    self.clickable[0]
                } else {
                    self.clickable[self.clickable.len() - 1]
                }
            }
        };

        self.hovered = Some(next);
        true
    }
}

impl EventController for MenuKeyController {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        match event {
            InputEvent::KeyDown { key, .. } => match key {
                Key::ArrowDown => {
                    if self.navigate(true) {
                        ctx.requests.insert(ControllerRequests::PAINT);
                    }
                    true
                }
                Key::ArrowUp => {
                    if self.navigate(false) {
                        ctx.requests.insert(ControllerRequests::PAINT);
                    }
                    true
                }
                Key::Enter | Key::Space => {
                    if let Some(idx) = self.hovered {
                        ctx.emit_action(WidgetAction::Selected {
                            id: ctx.widget_id,
                            index: idx,
                        });
                        ctx.requests.insert(ControllerRequests::PAINT);
                    }
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
                Key::ArrowDown | Key::ArrowUp | Key::Enter | Key::Space | Key::Escape
            ),
            _ => false,
        }
    }

    fn reset(&mut self) {
        self.hovered = None;
    }
}

#[cfg(test)]
mod tests;
