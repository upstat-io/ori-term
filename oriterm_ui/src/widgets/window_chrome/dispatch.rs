//! Controller-based input dispatch for window chrome.
//!
//! Replaces legacy `handle_mouse`/`handle_hover` on `WindowChromeWidget`
//! with controller dispatch via `dispatch_to_controllers`.

use std::time::Instant;

use crate::action::WidgetAction;
use crate::controllers::{ControllerCtxArgs, dispatch_to_controllers};
use crate::input::dispatch::tree::TreeDispatchResult;
use crate::input::{EventPhase, InputEvent};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;

use super::WindowChromeWidget;
use super::layout::ControlKind;

impl WindowChromeWidget {
    /// Dispatches an input event to chrome control buttons via controllers.
    ///
    /// Performs manual hit testing against control button rects, then
    /// dispatches to the matching button's controllers. Tracks pressed
    /// button index for proper mouse-up routing.
    pub fn dispatch_input(&mut self, event: &InputEvent, now: Instant) -> TreeDispatchResult {
        let mut result = TreeDispatchResult::new();

        if !self.chrome_layout.visible {
            return result;
        }

        // Determine which button to target.
        let target = match event {
            InputEvent::MouseDown { pos, .. } => {
                let idx = self.control_at_point(*pos);
                if let Some(i) = idx {
                    self.pressed_control = Some(i);
                }
                idx
            }
            InputEvent::MouseUp { .. } => self.pressed_control.take(),
            InputEvent::MouseMove { pos, .. } => self.control_at_point(*pos),
            _ => None,
        };

        if let Some(idx) = target {
            if let Some(ctrl_layout) = self.chrome_layout.controls.get(idx) {
                let btn = &mut self.controls[idx];
                let interaction = InteractionState::default();
                let args = ControllerCtxArgs {
                    widget_id: btn.id(),
                    bounds: ctrl_layout.rect,
                    interaction: &interaction,
                    now,
                };
                let output = dispatch_to_controllers(
                    btn.controllers_mut(),
                    event,
                    EventPhase::Target,
                    &args,
                );
                result.merge(output, btn.id());
            }
        }

        result
    }

    /// Returns the widget ID at the given point, if any control button is hit.
    ///
    /// Used by the dialog context to build the `InteractionManager` hot path
    /// for hover state tracking.
    pub fn widget_at_point(&self, point: crate::geometry::Point) -> Option<WidgetId> {
        if !self.chrome_layout.visible {
            return None;
        }
        self.control_at_point(point)
            .map(|idx| self.controls[idx].id())
    }

    /// Maps a widget ID to its window action (Minimize/Maximize/Close).
    ///
    /// Returns `None` if the ID doesn't match any control button.
    pub fn action_for_widget(&self, id: WidgetId) -> Option<WidgetAction> {
        self.controls
            .iter()
            .find(|c| c.id() == id)
            .map(|c| match c.kind() {
                ControlKind::Minimize => WidgetAction::WindowMinimize,
                ControlKind::MaximizeRestore => WidgetAction::WindowMaximize,
                ControlKind::Close => WidgetAction::WindowClose,
            })
    }
}
