//! macOS implementation of [`NativeWindowOps`].
//!
//! Uses the Objective-C runtime via `objc2` for `NSWindow` child window
//! management, shadow configuration, and window level control.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::ffi::NSInteger;
use objc2::runtime::{AnyObject, Bool};
use objc2::{msg_send, sel};

use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::NativeWindowOps;
use crate::window_manager::types::WindowKind;

/// `NSWindowAbove` — child window appears above parent.
const NS_WINDOW_ABOVE: NSInteger = 1;

/// `NSFloatingWindowLevel` — above normal windows.
const NS_FLOATING_WINDOW_LEVEL: NSInteger = 3;

/// `NSModalPanelWindowLevel` — above floating windows, for modals.
const NS_MODAL_PANEL_WINDOW_LEVEL: NSInteger = 8;

/// `NSNormalWindowLevel` — standard window level.
const NS_NORMAL_WINDOW_LEVEL: NSInteger = 0;

pub(super) struct MacosNativeOps;

impl NativeWindowOps for MacosNativeOps {
    fn set_owner(&self, child: &Window, parent: &Window) {
        let Some(parent_nswindow) = get_nswindow(parent) else {
            return;
        };
        let Some(child_nswindow) = get_nswindow(child) else {
            return;
        };
        // `addChildWindow:ordered:` makes the child float above the parent
        // and move with it. Only used for dialogs — tear-off windows must
        // remain independent.
        unsafe {
            let _: () = msg_send![
                parent_nswindow,
                addChildWindow: child_nswindow,
                ordered: NS_WINDOW_ABOVE,
            ];
        }
    }

    fn clear_owner(&self, child: &Window) {
        let Some(child_nswindow) = get_nswindow(child) else {
            return;
        };
        let parent: *mut AnyObject = unsafe { msg_send![child_nswindow, parentWindow] };
        if !parent.is_null() {
            unsafe {
                let _: () = msg_send![parent, removeChildWindow: child_nswindow];
            }
        }
    }

    fn enable_shadow(&self, window: &Window) {
        let Some(nswindow) = get_nswindow(window) else {
            return;
        };
        // macOS frameless windows (borderless styleMask) don't get shadows
        // by default. Setting hasShadow explicitly enables them.
        unsafe {
            let _: () = msg_send![nswindow, setHasShadow: Bool::YES];
        }
    }

    fn set_window_type(&self, window: &Window, kind: &WindowKind) {
        if !kind.is_dialog() {
            return;
        }
        let Some(nswindow) = get_nswindow(window) else {
            return;
        };
        // NSFloatingWindowLevel keeps the dialog above normal windows.
        unsafe {
            let _: () = msg_send![nswindow, setLevel: NS_FLOATING_WINDOW_LEVEL];
        }
    }

    fn set_modal(&self, dialog: &Window, _owner: &Window) {
        let Some(nswindow) = get_nswindow(dialog) else {
            return;
        };
        // Use NSModalPanelWindowLevel instead of NSApp.runModalForWindow:
        // since we control the event loop ourselves.
        unsafe {
            let _: () = msg_send![nswindow, setLevel: NS_MODAL_PANEL_WINDOW_LEVEL];
        }
    }

    fn clear_modal(&self, dialog: &Window, _owner: &Window) {
        let Some(nswindow) = get_nswindow(dialog) else {
            return;
        };
        unsafe {
            let _: () = msg_send![nswindow, setLevel: NS_NORMAL_WINDOW_LEVEL];
        }
    }
}

/// Gets the `NSWindow` from a winit `Window` via the `AppKit` raw handle.
///
/// winit provides an `NSView` handle; the `NSWindow` is its `window` property.
fn get_nswindow(window: &Window) -> Option<NonNull<AnyObject>> {
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => {
            let ns_view = h.ns_view.as_ptr() as *mut AnyObject;
            let ns_window: *mut AnyObject = unsafe { msg_send![ns_view, window] };
            NonNull::new(ns_window)
        }
        _ => None,
    }
}
