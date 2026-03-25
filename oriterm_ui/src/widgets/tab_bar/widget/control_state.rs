//! Window control button state management.
//!
//! Handles hover routing, click dispatch, maximized/active state, and
//! interactive rect reporting for the tab bar's window control buttons.
//! Extracted from `mod.rs` to keep that file under the 500-line limit.
//!
//! Control button methods are gated to non-macOS platforms: macOS uses
//! native traffic light buttons and does not need custom control widgets.

use crate::geometry::Rect;

#[cfg(not(target_os = "macos"))]
use std::time::Instant;

#[cfg(not(target_os = "macos"))]
use crate::action::WidgetAction;
#[cfg(not(target_os = "macos"))]
use crate::controllers::{ControllerCtxArgs, dispatch_to_controllers};
#[cfg(not(target_os = "macos"))]
use crate::geometry::Point;
#[cfg(not(target_os = "macos"))]
use crate::input::dispatch::tree::TreeDispatchResult;
#[cfg(not(target_os = "macos"))]
use crate::input::{EventPhase, InputEvent};
#[cfg(not(target_os = "macos"))]
use crate::interaction::InteractionState;
#[cfg(not(target_os = "macos"))]
use crate::widget_id::WidgetId;
#[cfg(not(target_os = "macos"))]
use crate::widgets::Widget;

use super::super::constants::{DROPDOWN_BUTTON_WIDTH, NEW_TAB_BUTTON_WIDTH};
use super::TabBarWidget;

#[cfg(not(target_os = "macos"))]
use crate::widgets::window_chrome::layout::ControlKind;

#[cfg(not(target_os = "macos"))]
impl TabBarWidget {
    /// Sets the maximized state on all control buttons.
    ///
    /// The maximize/restore button changes symbol (□ vs ⧉).
    pub fn set_maximized(&mut self, maximized: bool) {
        for ctrl in &mut self.controls {
            ctrl.set_maximized(maximized);
        }
    }

    /// Dispatches an input event to control buttons via their controllers.
    ///
    /// Performs hit testing against control button rects, then dispatches
    /// to the matching button's controllers. Tracks pressed button index
    /// for proper mouse-up routing.
    pub fn dispatch_control_input(
        &mut self,
        event: &InputEvent,
        now: Instant,
    ) -> TreeDispatchResult {
        let mut result = TreeDispatchResult::new();

        let target = match event {
            InputEvent::MouseDown { pos, .. } => {
                let idx = (0..3).find(|&i| self.control_rect(i).contains(*pos));
                if let Some(i) = idx {
                    self.pressed_control = Some(i);
                }
                idx
            }
            InputEvent::MouseUp { .. } => self.pressed_control.take(),
            InputEvent::MouseMove { pos, .. } => {
                (0..3).find(|&i| self.control_rect(i).contains(*pos))
            }
            _ => None,
        };

        if let Some(idx) = target {
            let bounds = self.control_rect(idx);
            let btn = &mut self.controls[idx];
            let interaction = InteractionState::default();
            let args = ControllerCtxArgs {
                widget_id: btn.id(),
                bounds,
                interaction: &interaction,
                now,
            };
            let mut output =
                dispatch_to_controllers(btn.controllers_mut(), event, EventPhase::Target, &args);
            // Let the button transform controller actions via on_action.
            output.actions = output
                .actions
                .into_iter()
                .filter_map(|a| btn.on_action(a, bounds))
                .collect();
            result.merge(output, btn.id());
        }

        result
    }

    /// Returns the control button's widget ID at the given point, if any.
    ///
    /// Used by the app layer to build `InteractionManager` hot paths for
    /// hover state tracking on control buttons.
    pub fn control_widget_at_point(&self, pos: Point) -> Option<WidgetId> {
        (0..3)
            .find(|&i| self.control_rect(i).contains(pos))
            .map(|i| self.controls[i].id())
    }

    /// Updates control button visual states based on cursor position.
    ///
    /// Directly drives `VisualStateAnimator` on each control button: the
    /// hovered button gets `InteractionState::with_hot()`, others get default.
    /// Returns `true` if any animator is mid-transition (caller should redraw).
    pub fn update_control_hover_state(&mut self, pos: Point, now: Instant) -> bool {
        let hovered_idx = (0..3).find(|&i| self.control_rect(i).contains(pos));
        let mut animating = false;
        for (i, ctrl) in self.controls.iter_mut().enumerate() {
            let state = if hovered_idx == Some(i) {
                InteractionState::new().with_hot()
            } else {
                InteractionState::new()
            };
            if let Some(animator) = ctrl.visual_states_mut() {
                animator.update(&state, now);
                animator.tick(now);
                if animator.is_animating(now) {
                    animating = true;
                }
            }
        }
        animating
    }

    /// Clears hover state on all control buttons.
    ///
    /// Called when the cursor leaves the window or moves out of the control area.
    /// Returns `true` if any animator is mid-transition.
    pub fn clear_control_hover_state(&mut self, now: Instant) -> bool {
        let normal = InteractionState::new();
        let mut animating = false;
        for ctrl in &mut self.controls {
            if let Some(animator) = ctrl.visual_states_mut() {
                animator.update(&normal, now);
                animator.tick(now);
                if animator.is_animating(now) {
                    animating = true;
                }
            }
        }
        animating
    }

    /// Maps a widget ID to its window action (Minimize/Maximize/Close).
    ///
    /// Returns `None` if the ID doesn't match any control button.
    pub fn action_for_control(&self, id: WidgetId) -> Option<WidgetAction> {
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

impl TabBarWidget {
    /// Sets the active/focused state (affects caption background).
    #[allow(
        clippy::unused_self,
        reason = "preserved for API compatibility — state not yet used"
    )]
    pub fn set_active(&self, _active: bool) {
        // No-op: caption_bg was previously forwarded to control buttons
        // for the restore symbol's background occlusion trick. With vector
        // icons, the restore symbol no longer needs this.
    }

    /// Returns all interactive rects in logical pixels.
    ///
    /// Includes tab rects, new-tab button, dropdown button, and (on
    /// non-macOS) the three control buttons. The platform layer scales
    /// these to physical pixels and uses them for `WM_NCHITTEST` — points
    /// inside are `HTCLIENT` (clickable), everything else is `HTCAPTION`
    /// (draggable).
    pub fn interactive_rects(&self) -> Vec<Rect> {
        #[cfg(target_os = "macos")]
        let extra = 2;
        #[cfg(not(target_os = "macos"))]
        let extra = 5;
        let mut rects = Vec::with_capacity(self.tabs.len() + extra);
        // Tab rects.
        for i in 0..self.tabs.len() {
            let x = self.layout.tab_x(i);
            rects.push(Rect::new(
                x,
                0.0,
                self.layout.tab_width_at(i),
                self.metrics.height,
            ));
        }
        // New-tab button.
        let ntx = self.layout.new_tab_x();
        rects.push(Rect::new(
            ntx,
            0.0,
            NEW_TAB_BUTTON_WIDTH,
            self.metrics.height,
        ));
        // Dropdown button.
        let ddx = self.layout.dropdown_x();
        rects.push(Rect::new(
            ddx,
            0.0,
            DROPDOWN_BUTTON_WIDTH,
            self.metrics.height,
        ));
        // Control buttons (not on macOS — OS provides native traffic lights).
        #[cfg(not(target_os = "macos"))]
        for i in 0..3 {
            rects.push(self.control_rect(i));
        }
        rects
    }
}
