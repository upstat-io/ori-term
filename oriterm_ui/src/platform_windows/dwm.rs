//! DWM (Desktop Window Manager) helpers for visibility and animation control.
//!
//! Wraps `DwmSetWindowAttribute` and `DwmGetWindowAttribute` for:
//! - Transition animation control (fade-in suppression for tear-off)
//! - Window cloaking (instant-invisible for close animation suppression)
//! - Extended frame bounds (visible area excluding invisible DWM border)

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_CLOAK, DWMWA_EXTENDED_FRAME_BOUNDS, DWMWA_TRANSITIONS_FORCEDISABLED,
    DwmGetWindowAttribute, DwmSetWindowAttribute,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
use winit::window::Window;

use super::hwnd_from_window;

/// Disable or enable DWM window transition animations.
///
/// Chrome pattern: wrap `set_visible(true)` with `set_transitions_enabled(false/true)`
/// to prevent the OS fade-in animation during tab tear-off. This gives an
/// instantaneous window appearance instead of a distracting transition.
pub fn set_transitions_enabled(window: &Window, enabled: bool) {
    let Some(hwnd) = hwnd_from_window(window) else {
        return;
    };
    let value: i32 = i32::from(!enabled);
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED as u32,
            (&raw const value).cast(),
            size_of::<i32>() as u32,
        );
    }
}

/// Cloak a window so DWM considers it invisible without any animation.
///
/// Unlike `ShowWindow(SW_HIDE)`, cloaking operates at the DWM compositor
/// level — the window becomes instantly invisible with no transition.
/// A cloaked window produces no visible animation when subsequently
/// destroyed, because DWM has nothing to animate.
pub fn cloak_window(window: &Window) {
    let Some(hwnd) = hwnd_from_window(window) else {
        return;
    };
    let cloak: i32 = 1;
    // SAFETY: `DwmSetWindowAttribute` with `DWMWA_CLOAK` is a standard
    // DWM API (Windows 8+). Writing a BOOL-sized value is the documented
    // calling convention.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK as u32,
            (&raw const cloak).cast(),
            size_of::<i32>() as u32,
        );
    }
}

/// Queries DWM for the visible frame bounds of an HWND.
///
/// Returns `None` if DWM composition is unavailable (e.g. disabled,
/// or running on an older Windows version without DWM).
pub(in crate::platform_windows) fn try_dwm_frame_bounds(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&raw mut rect).cast(),
            size_of::<RECT>() as u32,
        )
    };
    if hr == 0 { Some(rect) } else { None }
}

/// Returns the visible frame bounds for an HWND via DWM.
///
/// Uses `DWMWA_EXTENDED_FRAME_BOUNDS` which excludes the invisible DWM
/// border that `GetWindowRect` includes on windows with `WS_THICKFRAME`.
/// Falls back to `GetWindowRect` if the DWM query fails.
pub(in crate::platform_windows) fn visible_frame_bounds_hwnd(hwnd: HWND) -> RECT {
    try_dwm_frame_bounds(hwnd).unwrap_or_else(|| {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        unsafe { GetWindowRect(hwnd, &raw mut rect) };
        rect
    })
}
