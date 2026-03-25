//! Platform-specific window transparency and compositor effects.
//!
//! Manages blur/vibrancy effects symmetrically: enables them when the window
//! has sub-1.0 opacity and `blur` is true, disables them otherwise.
//!
//! - **Windows**: Acrylic blur via `DwmSetWindowAttribute` (Windows 11),
//!   using the `window-vibrancy` crate. Falls back to opaque on Win10
//!   without DWM composition.
//! - **macOS**: `NSVisualEffectView` vibrancy via `window-vibrancy`.
//! - **Linux**: Compositor-driven blur via `winit::Window::set_blur()`.
//!   Requires a compositor (Picom, `KWin`, Mutter, Sway). Falls back to
//!   opaque when no compositor is running.

use oriterm_core::Rgb;
use winit::window::Window;

/// Apply or remove platform-specific transparency effects on a window.
///
/// When `opacity < 1.0` and `blur` is true, enables frosted glass / vibrancy.
/// When `opacity >= 1.0` or `blur` is false, disables any previously applied
/// effects. The `bg` color tints the acrylic/blur layer on Windows (ignored
/// on other platforms).
pub fn apply_transparency(window: &Window, opacity: f32, blur: bool, bg: Rgb) {
    let want_blur = blur && opacity < 1.0;

    if want_blur {
        apply_blur(window, opacity, bg);
    } else {
        clear_blur(window);
    }
}

/// Apply platform-specific blur effects.
#[cfg(target_os = "windows")]
fn apply_blur(window: &Window, opacity: f32, bg: Rgb) {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
    let color = Some((bg.r, bg.g, bg.b, alpha));
    match window_vibrancy::apply_acrylic(window, color) {
        Ok(()) => log::info!("transparency: acrylic applied (alpha={alpha})"),
        Err(e) => log::warn!("transparency: acrylic failed: {e}"),
    }
}

#[cfg(target_os = "macos")]
fn apply_blur(window: &Window, _opacity: f32, _bg: Rgb) {
    match window_vibrancy::apply_vibrancy(
        window,
        window_vibrancy::NSVisualEffectMaterial::UnderWindowBackground,
        None,
        None,
    ) {
        Ok(()) => log::info!("transparency: macOS vibrancy applied"),
        Err(e) => log::warn!("transparency: macOS vibrancy failed: {e}"),
    }
}

#[cfg(target_os = "linux")]
fn apply_blur(window: &Window, _opacity: f32, _bg: Rgb) {
    window.set_blur(true);
    log::info!("transparency: compositor blur enabled");
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn apply_blur(_window: &Window, _opacity: f32, _bg: Rgb) {
    log::debug!("transparency: blur not supported on this platform");
}

/// Remove platform-specific blur effects.
///
/// Called when transitioning to opaque or when blur is disabled via config.
/// Idempotent — safe to call even if no blur was applied.
#[cfg(target_os = "windows")]
fn clear_blur(window: &Window) {
    match window_vibrancy::clear_acrylic(window) {
        Ok(()) => log::info!("transparency: acrylic cleared"),
        Err(e) => log::warn!("transparency: acrylic clear failed: {e}"),
    }
}

#[cfg(target_os = "macos")]
fn clear_blur(window: &Window) {
    match window_vibrancy::clear_vibrancy(window) {
        Ok(_) => log::info!("transparency: macOS vibrancy cleared"),
        Err(e) => log::warn!("transparency: macOS vibrancy clear failed: {e}"),
    }
}

#[cfg(target_os = "linux")]
fn clear_blur(window: &Window) {
    window.set_blur(false);
    log::info!("transparency: compositor blur disabled");
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn clear_blur(_window: &Window) {
    log::debug!("transparency: blur clear not supported on this platform");
}
