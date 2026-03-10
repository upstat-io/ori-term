//! Platform-native window operations for ownership, shadows, and type hints.
//!
//! Each platform module implements [`NativeWindowOps`] using OS-specific APIs.
//! Methods are best-effort — platforms that don't support an operation silently
//! no-op (e.g., Wayland doesn't support explicit shadow configuration).

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use winit::window::Window;

use super::types::WindowKind;

/// Platform-native window operations beyond what winit provides.
///
/// Enables OS-level window ownership (parent-child stacking), frameless window
/// shadows, window type hints (dialog vs normal), and modal behavior.
pub(crate) trait NativeWindowOps {
    /// Set the owner/parent of a window at the OS level.
    ///
    /// The child window will stack above the parent and be hidden when the
    /// parent is minimized. On Windows, sets the owner HWND. On macOS,
    /// calls `addChildWindow:ordered:`. On X11, sets transient-for hint.
    fn set_owner(&self, child: &Window, parent: &Window);

    /// Remove the owner/parent relationship from a window.
    fn clear_owner(&self, child: &Window);

    /// Enable OS-level shadow on a frameless window.
    ///
    /// On Windows, uses DWM frame extension. On macOS, sets `hasShadow`.
    /// On Linux, shadows are typically compositor-managed.
    fn enable_shadow(&self, window: &Window);

    /// Apply OS window type hints based on the window's kind.
    ///
    /// Affects taskbar visibility, z-ordering, and decoration style.
    fn set_window_type(&self, window: &Window, kind: &WindowKind);

    /// Set a dialog window as modal relative to its owner.
    ///
    /// Disables input to the owner window until the modal is dismissed.
    fn set_modal(&self, dialog: &Window, owner: &Window);

    /// Clear modal state and re-enable the owner window.
    fn clear_modal(&self, dialog: &Window, owner: &Window);
}

/// Returns the platform-specific [`NativeWindowOps`] implementation.
pub(crate) fn platform_ops() -> &'static dyn NativeWindowOps {
    #[cfg(target_os = "windows")]
    {
        &windows::WindowsNativeOps
    }
    #[cfg(target_os = "macos")]
    {
        &macos::MacosNativeOps
    }
    #[cfg(target_os = "linux")]
    {
        &linux::LinuxNativeOps
    }
}
