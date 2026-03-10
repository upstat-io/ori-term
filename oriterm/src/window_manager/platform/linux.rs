//! Linux implementation of [`NativeWindowOps`].
//!
//! Handles both X11 and Wayland at runtime via `RawWindowHandle` discrimination.
//! X11 uses `x11-dl` for transient-for hints, window type atoms, and modal
//! state. Wayland operations are best-effort no-ops where winit doesn't expose
//! the underlying `xdg_toplevel`.

#![allow(unsafe_code)]

use std::ffi::CString;
use std::sync::OnceLock;

use winit::raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::window::Window;

use super::NativeWindowOps;
use crate::window_manager::types::WindowKind;

/// Lazily loaded X11 library handle.
static XLIB: OnceLock<Option<x11_dl::xlib::Xlib>> = OnceLock::new();

/// Returns the lazily loaded Xlib handle, or `None` on pure Wayland systems.
fn xlib() -> Option<&'static x11_dl::xlib::Xlib> {
    XLIB.get_or_init(|| x11_dl::xlib::Xlib::open().ok())
        .as_ref()
}

pub(super) struct LinuxNativeOps;

impl NativeWindowOps for LinuxNativeOps {
    fn set_owner(&self, child: &Window, parent: &Window) {
        if let Some(RawWindowHandle::Xlib(_)) = child.window_handle().ok().map(|h| h.as_raw()) {
            set_owner_x11(child, parent);
        }
        // Wayland: winit 0.30 doesn't expose xdg_toplevel for set_parent().
        // The window works without stacking hints.
    }

    fn clear_owner(&self, child: &Window) {
        if let Some(RawWindowHandle::Xlib(_)) = child.window_handle().ok().map(|h| h.as_raw()) {
            clear_owner_x11(child);
        }
    }

    fn enable_shadow(&self, _window: &Window) {
        // On X11 with a compositor (picom, mutter, kwin), frameless windows
        // receive shadows automatically based on _NET_WM_WINDOW_TYPE.
        // On Wayland, shadows are compositor-managed for xdg_toplevel.
    }

    fn set_window_type(&self, window: &Window, kind: &WindowKind) {
        if !kind.is_dialog() {
            return;
        }
        if let Some(RawWindowHandle::Xlib(_)) = window.window_handle().ok().map(|h| h.as_raw()) {
            set_window_type_x11(window);
        }
    }

    fn set_modal(&self, dialog: &Window, _owner: &Window) {
        if let Some(RawWindowHandle::Xlib(_)) = dialog.window_handle().ok().map(|h| h.as_raw()) {
            set_modal_x11(dialog, true);
        }
    }

    fn clear_modal(&self, dialog: &Window, _owner: &Window) {
        if let Some(RawWindowHandle::Xlib(_)) = dialog.window_handle().ok().map(|h| h.as_raw()) {
            set_modal_x11(dialog, false);
        }
    }
}

// X11 implementations

/// Sets `WM_TRANSIENT_FOR` on the child window, hinting to the window manager
/// that it should stack above the parent.
fn set_owner_x11(child: &Window, parent: &Window) {
    let Some(xlib) = xlib() else { return };
    let Some((display, child_xid)) = extract_x11(child) else {
        return;
    };
    let Some((_, parent_xid)) = extract_x11(parent) else {
        return;
    };
    unsafe {
        (xlib.XSetTransientForHint)(display, child_xid, parent_xid);
    }
}

/// Clears `WM_TRANSIENT_FOR` by setting it to the root window (None).
fn clear_owner_x11(child: &Window) {
    let Some(xlib) = xlib() else { return };
    let Some((display, child_xid)) = extract_x11(child) else {
        return;
    };
    // Setting transient-for to None (0) removes the hint.
    unsafe {
        (xlib.XSetTransientForHint)(display, child_xid, 0);
    }
}

/// Sets `_NET_WM_WINDOW_TYPE` to `_NET_WM_WINDOW_TYPE_DIALOG`.
///
/// Tells the window manager to skip taskbar, stack above parent, and use
/// dialog-style decoration.
fn set_window_type_x11(window: &Window) {
    let Some(xlib) = xlib() else { return };
    let Some((display, xid)) = extract_x11(window) else {
        return;
    };
    unsafe {
        let wm_type = intern_atom(xlib, display, "_NET_WM_WINDOW_TYPE");
        let dialog = intern_atom(xlib, display, "_NET_WM_WINDOW_TYPE_DIALOG");
        let xa_atom = 4u64; // XA_ATOM
        (xlib.XChangeProperty)(
            display,
            xid,
            wm_type,
            xa_atom,
            32,                            // format: 32-bit values
            x11_dl::xlib::PropModeReplace, // mode
            (&raw const dialog).cast(),    // data
            1,                             // nelements
        );
    }
}

/// Adds or removes `_NET_WM_STATE_MODAL` via a client message to the root
/// window (EWMH spec requires this for mapped windows).
fn set_modal_x11(dialog: &Window, modal: bool) {
    let Some(xlib) = xlib() else { return };
    let Some((display, xid)) = extract_x11(dialog) else {
        return;
    };
    unsafe {
        let wm_state = intern_atom(xlib, display, "_NET_WM_STATE");
        let modal_atom = intern_atom(xlib, display, "_NET_WM_STATE_MODAL");
        // _NET_WM_STATE_ADD = 1, _NET_WM_STATE_REMOVE = 0
        let action = i64::from(modal);

        let root = (xlib.XDefaultRootWindow)(display);

        let mut event: x11_dl::xlib::XClientMessageEvent = std::mem::zeroed();
        event.type_ = x11_dl::xlib::ClientMessage;
        event.window = xid;
        event.message_type = wm_state;
        event.format = 32;
        event.data.set_long(0, action);
        event.data.set_long(1, modal_atom as i64);
        event.data.set_long(2, 0);
        event.data.set_long(3, 1); // source: normal application

        let mask = x11_dl::xlib::SubstructureNotifyMask | x11_dl::xlib::SubstructureRedirectMask;
        (xlib.XSendEvent)(
            display,
            root,
            x11_dl::xlib::False,
            mask,
            (&raw mut event).cast(),
        );
        (xlib.XFlush)(display);
    }
}

// Helpers

/// Extracts the X11 display and window ID from a winit `Window`.
fn extract_x11(window: &Window) -> Option<(*mut x11_dl::xlib::Display, u64)> {
    let wh = window.window_handle().ok()?;
    let dh = window.display_handle().ok()?;

    let xid = match wh.as_raw() {
        RawWindowHandle::Xlib(h) => h.window,
        _ => return None,
    };
    let display = match dh.as_raw() {
        RawDisplayHandle::Xlib(h) => h.display?.as_ptr().cast(),
        _ => return None,
    };

    Some((display, xid))
}

/// Interns an X11 atom name, returning the `Atom` value.
unsafe fn intern_atom(
    xlib: &x11_dl::xlib::Xlib,
    display: *mut x11_dl::xlib::Display,
    name: &str,
) -> u64 {
    let cname = CString::new(name).unwrap_or_else(|_| CString::new("_NET_WM_WINDOW_TYPE").unwrap());
    unsafe { (xlib.XInternAtom)(display, cname.as_ptr(), x11_dl::xlib::False) }
}
