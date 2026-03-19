//! Input simulation methods for the test harness.
//!
//! High-level methods that mirror real user interactions: mouse movement,
//! clicks, keyboard input, scrolling, and drag. Each method constructs the
//! appropriate [`InputEvent`], dispatches through the full pipeline, and
//! returns results.

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, Key, Modifiers, MouseButton, ScrollDelta};
use crate::widget_id::WidgetId;

use super::harness::WidgetTestHarness;

impl WidgetTestHarness {
    // -- Mouse movement --

    /// Moves the mouse to a screen-space position.
    ///
    /// Updates the hot path (which widgets are hovered), delivers
    /// `HotChanged` lifecycle events, and dispatches a `MouseMove` event.
    pub fn mouse_move(&mut self, pos: Point) {
        let event = InputEvent::MouseMove {
            pos,
            modifiers: Modifiers::NONE,
        };
        self.process_event(event);
    }

    /// Moves the mouse to the center of a widget by ID.
    ///
    /// Finds the widget's layout bounds, computes center point,
    /// and calls `mouse_move()`. Panics if the widget ID is not found.
    pub fn mouse_move_to(&mut self, widget_id: WidgetId) {
        let bounds = self
            .find_widget_bounds(widget_id)
            .unwrap_or_else(|| panic!("widget {widget_id:?} not found in layout"));
        let center = Point::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        );
        self.mouse_move(center);
    }

    // -- Mouse buttons --

    /// Simulates a mouse button press at the current position.
    pub fn mouse_down(&mut self, button: MouseButton) {
        let event = InputEvent::MouseDown {
            pos: self.mouse_pos,
            button,
            modifiers: Modifiers::NONE,
        };
        self.process_event(event);
    }

    /// Simulates a mouse button release at the current position.
    pub fn mouse_up(&mut self, button: MouseButton) {
        let event = InputEvent::MouseUp {
            pos: self.mouse_pos,
            button,
            modifiers: Modifiers::NONE,
        };
        self.process_event(event);
    }

    /// Convenience: `mouse_move_to(id)` + `mouse_down` + `mouse_up`.
    ///
    /// Returns the actions emitted during the click.
    pub fn click(&mut self, widget_id: WidgetId) -> Vec<WidgetAction> {
        self.pending_actions.clear();
        self.mouse_move_to(widget_id);
        self.mouse_down(MouseButton::Left);
        self.mouse_up(MouseButton::Left);
        self.take_actions()
    }

    /// Double-click: two clicks within the multi-click timeout.
    pub fn double_click(&mut self, widget_id: WidgetId) -> Vec<WidgetAction> {
        self.pending_actions.clear();
        self.mouse_move_to(widget_id);
        self.mouse_down(MouseButton::Left);
        self.mouse_up(MouseButton::Left);
        self.mouse_down(MouseButton::Left);
        self.mouse_up(MouseButton::Left);
        self.take_actions()
    }

    // -- Drag simulation --

    /// Simulates a drag from `start` to `end` with `steps` intermediate moves.
    pub fn drag(&mut self, start: Point, end: Point, steps: usize) -> Vec<WidgetAction> {
        self.pending_actions.clear();
        self.mouse_move(start);
        self.mouse_down(MouseButton::Left);

        let steps = steps.max(1);
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let pos = Point::new(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            );
            self.mouse_move(pos);
        }

        self.mouse_up(MouseButton::Left);
        self.take_actions()
    }

    // -- Keyboard --

    /// Simulates a key press + release.
    pub fn key_press(&mut self, key: Key, modifiers: Modifiers) -> Vec<WidgetAction> {
        self.pending_actions.clear();
        self.process_event(InputEvent::KeyDown { key, modifiers });
        self.process_event(InputEvent::KeyUp { key, modifiers });
        self.take_actions()
    }

    /// Simulates typing a string (one key event per character).
    pub fn type_text(&mut self, text: &str) -> Vec<WidgetAction> {
        self.pending_actions.clear();
        for ch in text.chars() {
            let key = Key::Character(ch);
            self.process_event(InputEvent::KeyDown {
                key,
                modifiers: Modifiers::NONE,
            });
            self.process_event(InputEvent::KeyUp {
                key,
                modifiers: Modifiers::NONE,
            });
        }
        self.take_actions()
    }

    /// Tab to the next focusable widget.
    ///
    /// Sends a Tab key event through the pipeline. If no widget with a
    /// `FocusController` handles the Tab key (e.g., no focused widget),
    /// the harness cycles focus forward directly via the `FocusManager`.
    pub fn tab(&mut self) -> Vec<WidgetAction> {
        let focus_before = self.focus.focused();
        self.pending_actions.clear();
        self.process_event(InputEvent::KeyDown {
            key: Key::Tab,
            modifiers: Modifiers::NONE,
        });
        self.process_event(InputEvent::KeyUp {
            key: Key::Tab,
            modifiers: Modifiers::NONE,
        });

        // If the pipeline didn't change focus (no FocusController handled it),
        // cycle focus manually.
        if self.focus.focused() == focus_before {
            self.focus.focus_next();
            if let Some(new_id) = self.focus.focused() {
                self.interaction.request_focus(new_id, &mut self.focus);
            }
            self.deliver_lifecycle_events();
        }
        self.take_actions()
    }

    /// Shift+Tab to the previous focusable widget.
    ///
    /// Sends a Shift+Tab key event through the pipeline. If no widget
    /// handles it, the harness cycles focus backward via the `FocusManager`.
    pub fn shift_tab(&mut self) -> Vec<WidgetAction> {
        let focus_before = self.focus.focused();
        self.pending_actions.clear();
        self.process_event(InputEvent::KeyDown {
            key: Key::Tab,
            modifiers: Modifiers::SHIFT_ONLY,
        });
        self.process_event(InputEvent::KeyUp {
            key: Key::Tab,
            modifiers: Modifiers::SHIFT_ONLY,
        });

        if self.focus.focused() == focus_before {
            self.focus.focus_prev();
            if let Some(new_id) = self.focus.focused() {
                self.interaction.request_focus(new_id, &mut self.focus);
            }
            self.deliver_lifecycle_events();
        }
        self.take_actions()
    }

    // -- Scroll --

    /// Simulates a scroll wheel event at the current mouse position.
    pub fn scroll(&mut self, delta: ScrollDelta) -> Vec<WidgetAction> {
        self.pending_actions.clear();
        let event = InputEvent::Scroll {
            pos: self.mouse_pos,
            delta,
            modifiers: Modifiers::NONE,
        };
        self.process_event(event);
        self.take_actions()
    }
}
