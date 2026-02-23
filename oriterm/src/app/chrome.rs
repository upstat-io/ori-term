//! Window chrome action dispatch.
//!
//! Handles `WidgetAction::WindowMinimize`, `WindowMaximize`, and
//! `WindowClose` by forwarding to the appropriate winit window operations.

use winit::event_loop::ActiveEventLoop;

use oriterm_ui::widgets::WidgetAction;

use super::App;

impl App {
    /// Dispatch a window chrome action to the corresponding window operation.
    ///
    /// Returns `true` if the action was handled (recognized as a chrome action).
    pub(super) fn handle_chrome_action(
        &mut self,
        action: &WidgetAction,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        match action {
            WidgetAction::WindowMinimize => {
                if let Some(window) = &self.window {
                    window.window().set_minimized(true);
                }
                true
            }
            WidgetAction::WindowMaximize => {
                if let Some(window) = &mut self.window {
                    let maximized = !window.is_maximized();
                    window.window().set_maximized(maximized);
                    window.set_maximized(maximized);
                    if let Some(chrome) = &mut self.chrome {
                        chrome.set_maximized(maximized);
                    }
                    self.dirty = true;
                }
                true
            }
            WidgetAction::WindowClose => {
                if let Some(gpu) = &self.gpu {
                    gpu.save_pipeline_cache();
                }
                event_loop.exit();
                true
            }
            _ => false,
        }
    }
}
