//! Platform-native window operations for ownership, shadows, type hints, and chrome.
//!
//! Each platform module implements [`NativeWindowOps`] and [`NativeChromeOps`]
//! using OS-specific APIs. Methods are best-effort — platforms that don't
//! support an operation silently no-op (e.g., Wayland doesn't support
//! explicit shadow configuration or OS-level interactive rect tracking).

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "windows")]
mod windows;

use winit::window::Window;

use oriterm_ui::geometry::Rect;

use super::types::WindowKind;

// Window management

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
    #[allow(
        dead_code,
        reason = "window manager API — wired during main window migration"
    )]
    fn clear_owner(&self, child: &Window);

    /// Apply OS window type hints based on the window's kind.
    ///
    /// Affects taskbar visibility, z-ordering, and decoration style.
    /// Currently only acts on dialogs; accepts full `WindowKind` to allow
    /// future differentiation (e.g. tear-off vs main).
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

// Chrome management

/// How a window's chrome should behave at the OS level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChromeMode {
    /// Main window: Aero Snap, full resize borders, tab bar caption.
    Main,
    /// Dialog window: no Aero Snap, optional resize, close-only caption.
    Dialog {
        /// Whether the dialog has resize borders.
        resizable: bool,
    },
}

/// Platform-native chrome operations for frameless window management.
///
/// Installs OS-level hit test routing (resize borders, caption drag, window
/// controls) and syncs interactive region geometry with the OS. Each platform
/// implements the trait; app code calls [`chrome_ops()`] instead of
/// `#[cfg(target_os)]` blocks.
///
/// Separate from [`NativeWindowOps`] because chrome operations (hit testing,
/// DWM subclass) are conceptually distinct from window management (ownership,
/// modality, type hints).
pub(crate) trait NativeChromeOps {
    /// Install frameless window chrome with platform-specific subclass/hooks.
    ///
    /// On Windows: `WS_THICKFRAME` (Main only), DWM frame, `WndProc` subclass.
    /// On macOS: `NSFullSizeContentViewWindowMask`, titlebar transparency.
    /// On Linux: no-op (compositor-managed decorations or GTK CSD via winit).
    ///
    /// `border_width` and `caption_height` are in physical pixels (scaled by
    /// the display scale factor).
    fn install_chrome(
        &self,
        window: &Window,
        mode: ChromeMode,
        border_width: f32,
        caption_height: f32,
    );

    /// Update interactive rects (buttons, tabs) for OS-level hit testing.
    ///
    /// Accepts logical-pixel rects and a scale factor. The platform
    /// implementation scales them to physical pixels internally.
    /// On macOS/Linux, this is a no-op.
    fn set_interactive_rects(&self, window: &Window, rects: &[Rect], scale: f32);

    /// Update chrome metrics after DPI change or resize.
    ///
    /// `border_width` and `caption_height` are in physical pixels.
    fn set_chrome_metrics(&self, window: &Window, border_width: f32, caption_height: f32);
}

/// Returns the platform-specific [`NativeChromeOps`] implementation.
pub(crate) fn chrome_ops() -> &'static dyn NativeChromeOps {
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
