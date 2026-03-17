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
use crate::geometry::Point;
#[cfg(not(target_os = "macos"))]
use crate::input::{HoverEvent, MouseButton, MouseEvent, MouseEventKind};

use super::super::constants::{DROPDOWN_BUTTON_WIDTH, NEW_TAB_BUTTON_WIDTH, TAB_BAR_HEIGHT};
use super::TabBarWidget;

#[cfg(not(target_os = "macos"))]
use crate::widgets::{EventCtx, Widget, WidgetResponse};

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

    /// Updates hover state for control buttons based on cursor position.
    ///
    /// Routes `HoverEvent::Enter`/`Leave` to the appropriate
    /// [`WindowControlButton`] so animation transitions play correctly.
    pub fn update_control_hover(&mut self, pos: Point, ctx: &EventCtx<'_>) -> WidgetResponse {
        let new_idx = (0..3).find(|&i| self.control_rect(i).contains(pos));

        if new_idx == self.hovered_control {
            return WidgetResponse::ignored();
        }

        // Leave old control.
        let left = if let Some(old) = self.hovered_control {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: self.control_rect(old),
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            self.controls[old].handle_hover(HoverEvent::Leave, &child_ctx);
            true
        } else {
            false
        };

        // Enter new control.
        let entered = if let Some(new) = new_idx {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: self.control_rect(new),
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            self.controls[new].handle_hover(HoverEvent::Enter, &child_ctx);
            true
        } else {
            false
        };

        self.hovered_control = new_idx;

        if left || entered {
            WidgetResponse::paint()
        } else {
            WidgetResponse::ignored()
        }
    }

    /// Clears control button hover state (e.g. when cursor leaves the tab bar).
    pub fn clear_control_hover(&mut self, ctx: &EventCtx<'_>) {
        if let Some(old) = self.hovered_control.take() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: self.control_rect(old),
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            self.controls[old].handle_hover(HoverEvent::Leave, &child_ctx);
        }
    }

    /// Routes a mouse event to the appropriate control button.
    ///
    /// On button down: sets pressed state on the hovered control.
    /// On button up: releases the pressed control and emits the action.
    pub fn handle_control_mouse(
        &mut self,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let idx = (0..3).find(|&i| self.control_rect(i).contains(event.pos));
                if let Some(i) = idx {
                    self.pressed_control = Some(i);
                    let child_ctx = EventCtx {
                        measurer: ctx.measurer,
                        bounds: self.control_rect(i),
                        is_focused: false,
                        focused_widget: ctx.focused_widget,
                        theme: ctx.theme,
                        interaction: None,
                        widget_id: None,
                        frame_requests: None,
                    };
                    return self.controls[i].handle_mouse(event, &child_ctx);
                }
                WidgetResponse::ignored()
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(i) = self.pressed_control.take() {
                    let child_ctx = EventCtx {
                        measurer: ctx.measurer,
                        bounds: self.control_rect(i),
                        is_focused: false,
                        focused_widget: ctx.focused_widget,
                        theme: ctx.theme,
                        interaction: None,
                        widget_id: None,
                        frame_requests: None,
                    };
                    return self.controls[i].handle_mouse(event, &child_ctx);
                }
                WidgetResponse::ignored()
            }
            _ => WidgetResponse::ignored(),
        }
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
                TAB_BAR_HEIGHT,
            ));
        }
        // New-tab button.
        let ntx = self.layout.new_tab_x();
        rects.push(Rect::new(ntx, 0.0, NEW_TAB_BUTTON_WIDTH, TAB_BAR_HEIGHT));
        // Dropdown button.
        let ddx = self.layout.dropdown_x();
        rects.push(Rect::new(ddx, 0.0, DROPDOWN_BUTTON_WIDTH, TAB_BAR_HEIGHT));
        // Control buttons (not on macOS — OS provides native traffic lights).
        #[cfg(not(target_os = "macos"))]
        for i in 0..3 {
            rects.push(self.control_rect(i));
        }
        rects
    }
}
