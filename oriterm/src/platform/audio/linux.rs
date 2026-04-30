//! Linux bell-sound dispatch via X11 `XBell`; no-op on Wayland.

#![allow(
    unsafe_code,
    reason = "X11 native bell FFI: XOpenDisplay/XBell/XCloseDisplay via x11-dl"
)]

use std::ptr;

use x11_dl::xlib::Xlib;

/// Play the X11 bell via `XBell(display, 0)` if running under X11; no-op
/// on Wayland.
///
/// On X11, honors `xset b` (volume / pitch / on-off). Reference
/// precedent: wezterm `window/src/os/x11/connection.rs:435-437`.
///
/// Wayland users get no audible bell from the terminal itself; the
/// upstream Wayland protocol has no stable system-bell hook in current
/// reference terminals (wezterm Wayland connection has no `beep()`
/// either). Compositor-aware bell via `xdg_system_bell_v1` and sound-theme
/// fallback via `libcanberra` are deferred to a follow-up bug if/when
/// Linux Wayland users report no-bell complaints.
///
/// Fire-and-forget. Failures (no `DISPLAY`, X server unreachable, libX11
/// missing) are silently swallowed per the bell contract.
pub(crate) fn play_bell() {
    // Skip the X11 path entirely on pure-Wayland sessions to avoid an
    // unnecessary XOpenDisplay round-trip + connection-refused
    // round-trip. `DISPLAY` unset is the canonical signal for "no X
    // server reachable" on a Wayland-only session.
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }

    let Ok(xlib) = Xlib::open() else {
        return;
    };

    // SAFETY: `XOpenDisplay(NULL)` opens the default display per the
    // user's `DISPLAY` env var. Returns NULL on failure (no X server,
    // connection refused, auth denied), which we check before the bell
    // call. The pointer is owned by Xlib and freed via `XCloseDisplay`.
    let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
    if display.is_null() {
        return;
    }

    // SAFETY: `XBell(display, 0)` plays the bell at the user's
    // configured base volume (per `xset b`). `display` is non-null per
    // the check above. `XCloseDisplay` releases the connection.
    unsafe {
        (xlib.XBell)(display, 0);
        (xlib.XCloseDisplay)(display);
    }
}
