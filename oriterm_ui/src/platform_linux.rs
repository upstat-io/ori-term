//! Linux platform glue for frameless window management.
//!
//! Thin layer — winit handles X11 `_NET_WM_MOVERESIZE` and Wayland
//! `xdg_toplevel.move`/`xdg_toplevel.resize` internally. No additional
//! platform dependencies are needed.

use winit::window::Window;

use crate::hit_test::ResizeDirection;

/// Initiates a window drag (title bar drag).
///
/// Called when `hit_test()` returns `Caption`. winit translates this to
/// `_NET_WM_MOVERESIZE` on X11 or `xdg_toplevel.move` on Wayland.
pub fn start_drag(window: &Window) {
    if let Err(e) = window.drag_window() {
        log::warn!("drag_window failed: {e}");
    }
}

/// Initiates a window resize from the given edge or corner.
///
/// Called when `hit_test()` returns `ResizeBorder(direction)`. winit maps
/// the direction to `_NET_WM_MOVERESIZE` on X11 or `xdg_toplevel.resize`
/// on Wayland.
pub fn start_resize(window: &Window, direction: ResizeDirection) {
    if let Err(e) = window.drag_resize_window(direction.to_winit()) {
        log::warn!("drag_resize_window failed: {e}");
    }
}
