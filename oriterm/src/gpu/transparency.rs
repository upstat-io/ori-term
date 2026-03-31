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
/// When `opacity < 1.0`, `blur` is true, and the GPU surface supports
/// non-opaque alpha compositing, enables frosted glass / vibrancy.
/// Otherwise disables any previously applied effects.
///
/// `surface_supports_alpha` should be `GpuState::supports_transparency()`.
/// Without it, DWM/compositor blur reads an undefined alpha channel from
/// the surface (Vulkan `Opaque` mode zeroes alpha), making content invisible.
pub fn apply_transparency(
    window: &Window,
    opacity: f32,
    blur: bool,
    bg: Rgb,
    surface_supports_alpha: bool,
) {
    let want_blur = blur && opacity < 1.0 && surface_supports_alpha;

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
        Ok(()) => {
            log::info!("transparency: macOS vibrancy applied");
            // The `window_vibrancy` crate adds the NSVisualEffectView as a
            // subview of the content view. On layer-backed views, subviews
            // render ON TOP of the parent's CAMetalLayer, covering our GPU
            // content. Fix: reparent the vibrancy view under the window's
            // themeFrame (behind the content view) so it composites behind
            // the Metal surface instead of in front of it.
            reparent_vibrancy_view(window);
        }
        Err(e) => log::warn!("transparency: macOS vibrancy failed: {e}"),
    }
}

/// Move the vibrancy view from content view child to themeFrame child.
///
/// `window_vibrancy` adds the `NSVisualEffectView` (tagged with
/// `91376254`) as a subview of the content view. Layer-backed subviews
/// render above the parent's layer, so the vibrancy covers the Metal
/// surface. Reparenting under the `contentView.superview` (the window's
/// `NSThemeFrame`) places it behind the content view in the view
/// hierarchy, letting the Metal content composite on top.
#[cfg(target_os = "macos")]
#[allow(unsafe_code, reason = "Objective-C FFI for view reparenting")]
fn reparent_vibrancy_view(window: &Window) {
    use objc2::ffi::NSInteger;
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    const VIBRANCY_TAG: NSInteger = 91376254;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else {
        return;
    };

    unsafe {
        let ns_view = h.ns_view.as_ptr().cast::<AnyObject>();

        // Find the vibrancy view by its tag.
        let blur_view: *mut AnyObject = msg_send![ns_view, viewWithTag: VIBRANCY_TAG];
        if blur_view.is_null() {
            log::debug!("reparent_vibrancy_view: vibrancy view not found");
            return;
        }

        // Get the content view's superview (NSThemeFrame).
        let superview: *mut AnyObject = msg_send![ns_view, superview];
        if superview.is_null() {
            log::debug!("reparent_vibrancy_view: no superview");
            return;
        }

        // Remove from content view and add to superview behind content view.
        // NSWindowBelow = -1 (place below the relativeTo view).
        let _: () = msg_send![blur_view, removeFromSuperview];
        let _: () = msg_send![
            superview,
            addSubview: blur_view,
            positioned: -1i64,
            relativeTo: ns_view
        ];

        log::info!("reparent_vibrancy_view: moved vibrancy view behind content view");
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
