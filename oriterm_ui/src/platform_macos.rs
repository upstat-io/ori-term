//! macOS platform glue for frameless window management.
//!
//! Thin layer — winit handles most macOS windowing. Traffic light buttons
//! are positioned automatically by `fullsize_content_view(true)` (set in
//! [`crate::window`]). Retina (`HiDPI`) is handled by winit's
//! `ScaleFactorChanged` event. Full screen is achieved via
//! `window.set_fullscreen()`.

use winit::window::Window;

use crate::hit_test::ResizeDirection;

/// Configures macOS-specific window properties.
///
/// Currently a no-op — all attributes are set during window creation in
/// [`crate::window`] via `with_titlebar_transparent(true)` and
/// `with_fullsize_content_view(true)`.
pub fn configure_window(_window: &Window) {
    // Traffic lights positioned automatically by winit.
}

/// Initiates a window drag (title bar drag).
///
/// Called when `hit_test()` returns `Caption`. winit translates this to
/// the appropriate `performWindowDrag:` Cocoa call.
pub fn start_drag(window: &Window) {
    if let Err(e) = window.drag_window() {
        log::warn!("drag_window failed: {e}");
    }
}

/// Initiates a window resize from the given edge or corner.
///
/// Called when `hit_test()` returns `ResizeBorder(direction)`. winit
/// maps the direction to the appropriate Cocoa resize behavior.
pub fn start_resize(window: &Window, direction: ResizeDirection) {
    if let Err(e) = window.drag_resize_window(direction.to_winit()) {
        log::warn!("drag_resize_window failed: {e}");
    }
}
