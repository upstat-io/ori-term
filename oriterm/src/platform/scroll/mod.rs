//! Platform-specific mouse wheel scroll lines multiplier.
//!
//! Winit's `LineDelta` reports raw wheel notches without applying the OS
//! scroll lines preference. The multiplier returned here converts raw
//! notches to the number of lines the user expects per scroll.
//!
//! - **Windows**: Queries `SystemParametersInfoW(SPI_GETWHEELSCROLLLINES)`.
//!   Default: 3. The user can change this in Settings → Mouse → Scroll.
//! - **macOS**: Returns 1. `NSEvent`'s `scrollingDeltaY()` already applies
//!   the system scroll acceleration before winit sees it — multiplying
//!   again would double-count.
//! - **Linux**: Returns 3 (de facto standard). X11 button events and
//!   Wayland `axis_discrete` events are raw notches with no OS-level
//!   lines-per-notch setting. GNOME/KDE configure scroll *acceleration*
//!   (speed curve), not a discrete line count — there is no cross-DE API
//!   equivalent to Windows' `SPI_GETWHEELSCROLLLINES`.

/// Number of lines to scroll per mouse wheel notch.
///
/// On Windows this queries the system parameter. On macOS it returns 1
/// (OS pre-applies acceleration). On Linux it returns 3 (standard default
/// matching Alacritty and other terminal emulators).
pub fn wheel_scroll_lines() -> u32 {
    platform_wheel_scroll_lines()
}

/// Windows: query `SPI_GETWHEELSCROLLLINES` via `SystemParametersInfoW`.
#[cfg(windows)]
fn platform_wheel_scroll_lines() -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETWHEELSCROLLLINES, SystemParametersInfoW,
    };

    let mut lines: u32 = 0;

    // SAFETY: `SystemParametersInfoW` is a standard Win32 API. We pass a
    // valid output pointer sized to `u32` and request `SPI_GETWHEELSCROLLLINES`.
    #[allow(unsafe_code, reason = "SystemParametersInfoW is a standard Win32 API")]
    let ok =
        unsafe { SystemParametersInfoW(SPI_GETWHEELSCROLLLINES, 0, (&raw mut lines).cast(), 0) };

    if ok != 0 && lines > 0 {
        lines
    } else {
        3 // Windows default.
    }
}

/// macOS: return 1 (OS pre-applies scroll acceleration to `NSEvent` deltas).
///
/// `NSEvent`'s `scrollingDeltaY()` already reflects the user's "Scroll speed"
/// preference from System Settings → Mouse. For discrete mice (non-trackpad),
/// the delta value is pre-multiplied by the system's scroll speed factor.
/// Applying our own multiplier would double-count.
#[cfg(target_os = "macos")]
fn platform_wheel_scroll_lines() -> u32 {
    1
}

/// Linux: return 3 (standard default, no universal lines-per-notch API).
///
/// Neither X11 nor Wayland expose a "lines per scroll notch" setting.
/// GNOME (`org.gnome.desktop.peripherals.mouse speed`) and KDE configure
/// scroll *acceleration curves*, not discrete line counts. Every major
/// terminal emulator (Alacritty, Ghostty, `WezTerm`) uses a hardcoded
/// default of 3–5 lines per notch on Linux.
#[cfg(target_os = "linux")]
fn platform_wheel_scroll_lines() -> u32 {
    3
}

/// Fallback for other platforms.
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn platform_wheel_scroll_lines() -> u32 {
    3
}

#[cfg(test)]
mod tests;
