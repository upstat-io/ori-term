//! Tab bar mouse click dispatch.
//!
//! Routes left-click presses in the tab bar zone to the appropriate action
//! based on the [`TabBarHit`](oriterm_ui::widgets::tab_bar::TabBarHit) at
//! the cursor position.

use std::time::{Duration, Instant};

use winit::event::ElementState;
use winit::event_loop::ActiveEventLoop;

use oriterm_ui::widgets::tab_bar::TabBarHit;

use super::App;

/// Time window for two clicks to count as a double-click.
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(500);

impl App {
    /// Dispatch a mouse click in the tab bar zone.
    ///
    /// Returns `true` if the event was consumed (click landed on a tab bar
    /// element). Returns `false` if the click is outside the tab bar.
    pub(super) fn try_tab_bar_mouse(
        &mut self,
        button: winit::event::MouseButton,
        state: ElementState,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        // Only handle left-button events.
        if button != winit::event::MouseButton::Left {
            return false;
        }

        let pos = self.mouse.cursor_pos();
        if !self.cursor_in_tab_bar(pos) {
            return false;
        }

        // Consume release events without action — prevents them from
        // falling through to the terminal selection handler.
        if state != ElementState::Pressed {
            return true;
        }

        // Use the hover hit already computed by update_tab_bar_hover.
        let hit = self
            .focused_ctx()
            .map_or(TabBarHit::None, |ctx| ctx.tab_bar.hover_hit());

        match hit {
            TabBarHit::None => false,

            TabBarHit::Tab(idx) => {
                self.switch_to_tab_index(idx);
                self.try_start_tab_drag(idx);
                true
            }

            TabBarHit::CloseTab(idx) => {
                // Acquire width lock for stable close-button targeting
                // during rapid close clicks.
                if let Some(ctx) = self.focused_ctx() {
                    let w = ctx.tab_bar.layout().tab_width;
                    self.acquire_tab_width_lock(w);
                }
                self.close_tab_at_index(idx);
                true
            }

            TabBarHit::NewTab => {
                if let Some(win_id) = self.active_window {
                    self.new_tab_in_window(win_id);
                }
                true
            }

            // Dropdown menu (Section 21): no-op for now.
            TabBarHit::Dropdown => true,

            TabBarHit::Minimize => {
                let action = oriterm_ui::widgets::WidgetAction::WindowMinimize;
                self.handle_chrome_action(&action, event_loop);
                true
            }

            TabBarHit::Maximize => {
                let action = oriterm_ui::widgets::WidgetAction::WindowMaximize;
                self.handle_chrome_action(&action, event_loop);
                true
            }

            TabBarHit::CloseWindow => {
                let action = oriterm_ui::widgets::WidgetAction::WindowClose;
                self.handle_chrome_action(&action, event_loop);
                true
            }

            TabBarHit::DragArea => {
                self.handle_tab_bar_drag_area();
                true
            }
        }
    }

    /// Handle a click in the tab bar drag area.
    ///
    /// Double-click toggles maximize; single click initiates window drag.
    fn handle_tab_bar_drag_area(&mut self) {
        let now = Instant::now();
        let is_double = self
            .focused_ctx()
            .and_then(|ctx| ctx.last_drag_area_press)
            .is_some_and(|t| now.duration_since(t) < DOUBLE_CLICK_THRESHOLD);
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.last_drag_area_press = Some(now);
        }

        if is_double {
            // Double-click: toggle maximize. Reset timestamp to prevent
            // a third click from triggering another toggle.
            if let Some(ctx) = self.focused_ctx_mut() {
                ctx.last_drag_area_press = None;
            }
            self.toggle_maximize();
        } else {
            // Single click: initiate native window drag.
            if let Some(ctx) = self.focused_ctx() {
                let _ = ctx.window.window().drag_window();
            }
        }
    }
}
