//! Window lifecycle management — creation requests and exit logic.
//!
//! The [`WindowManager`] doesn't create OS windows directly (that requires
//! `&ActiveEventLoop` which it doesn't own). Instead, it provides helpers
//! to build [`WindowAttributes`] and determine lifecycle outcomes like
//! whether closing a window should exit the application.

use winit::window::{WindowAttributes, WindowId};

use super::WindowManager;
use crate::window_manager::types::WindowRequest;

impl WindowManager {
    /// Build `WindowAttributes` from a creation request.
    ///
    /// The caller (`App`) is responsible for actually creating the winit
    /// window via `ActiveEventLoop::create_window()`, then calling
    /// [`register()`](WindowManager::register) with the resulting ID.
    #[allow(
        dead_code,
        reason = "window manager API — wired during main window migration"
    )]
    pub fn prepare_create(request: &WindowRequest) -> WindowAttributes {
        let mut attrs = WindowAttributes::default()
            .with_title(request.title.clone())
            .with_visible(request.visible)
            .with_decorations(request.decorations);

        if let Some((w, h)) = request.size {
            attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(w, h));
        }

        if let Some((x, y)) = request.position {
            attrs = attrs.with_position(winit::dpi::LogicalPosition::new(x, y));
        }

        // Dialogs are not resizable by default and have no maximize button.
        if request.kind.is_dialog() {
            attrs = attrs.with_resizable(false);
        }

        attrs
    }

    /// Returns `true` if closing this window should exit the application.
    ///
    /// The app should exit when the last primary window (main or tear-off)
    /// closes. Dialog windows don't keep the app alive on their own.
    #[allow(
        dead_code,
        reason = "window manager API — wired during main window migration"
    )]
    pub fn should_exit_on_close(&self, id: WindowId) -> bool {
        let remaining = self
            .windows
            .values()
            .filter(|w| w.winit_id != id)
            .filter(|w| w.kind.is_primary())
            .count();
        remaining == 0
    }

    /// Find the appropriate parent for a new dialog.
    ///
    /// Prefers the currently focused main window. Falls back to any main
    /// window if the focused window is not a primary window (or if no
    /// window is focused).
    #[allow(
        dead_code,
        reason = "window manager API — wired during main window migration"
    )]
    pub fn find_dialog_parent(&self) -> Option<WindowId> {
        // Try the focused window first if it's a primary window.
        if let Some(focused) = self.focused_id {
            if let Some(w) = self.windows.get(&focused) {
                if w.kind.is_primary() {
                    return Some(focused);
                }
            }
        }

        // Fall back to any primary window.
        self.windows
            .values()
            .find(|w| w.kind.is_primary())
            .map(|w| w.winit_id)
    }
}
