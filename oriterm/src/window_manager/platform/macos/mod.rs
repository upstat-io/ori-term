//! macOS implementation of [`NativeWindowOps`] and [`NativeChromeOps`].
//!
//! Uses the Objective-C runtime via `objc2` for `NSWindow` child window
//! management, shadow configuration, and window level control.
//! Chrome operations are mostly no-ops — macOS handles DPI scaling and
//! title bar hit testing automatically via `NSFullSizeContentViewWindowMask`.
//!
//! Fullscreen transition handling lives in the [`fullscreen`] submodule.

#![allow(unsafe_code, reason = "Objective-C FFI via objc2")]

mod fullscreen;
mod types;

pub use fullscreen::take_fullscreen_events;

use types::{NSPoint, NSRect};

use std::ptr::NonNull;

use objc2::ffi::NSInteger;
use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};

use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use oriterm_ui::geometry::Rect;

use super::{ChromeMode, NativeChromeOps, NativeWindowOps};
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
        // NSModalPanelWindowLevel raises the dialog but doesn't block parent
        // input. True modality requires beginSheet:completionHandler: or
        // NSApp.runModalSession — deferred until we validate the event loop
        // interaction with winit's run_app.
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

impl NativeChromeOps for MacosNativeOps {
    fn install_chrome(
        &self,
        window: &Window,
        _mode: ChromeMode,
        _border_width: f32,
        caption_height: f32,
    ) {
        // macOS uses NSFullSizeContentViewWindowMask + titlebarAppearsTransparent
        // configured at window creation via winit. Position the traffic light
        // buttons vertically centered in our custom tab bar.
        center_traffic_lights(window, caption_height);

        // Disable the default titlebar drag behavior so our tab bar handles
        // drag events (tab reorder, tear-off). Empty areas use `drag_window()`
        // via the `DragArea` hit handler.
        disable_titlebar_drag(window);

        // Register fullscreen transition observers so the event loop can
        // update the tab bar inset before the macOS animation begins.
        fullscreen::install_fullscreen_observers(window);
    }

    fn set_interactive_rects(&self, _window: &Window, _rects: &[Rect], _scale: f32) {
        // macOS does not use OS-level interactive rect tracking.
        // Title bar buttons (traffic lights) are managed by AppKit.
    }

    fn set_chrome_metrics(&self, window: &Window, _border_width: f32, caption_height: f32) {
        // Reposition traffic lights after resize or DPI change.
        center_traffic_lights(window, caption_height);
        disable_titlebar_drag(window);
    }
}

/// Re-apply traffic light centering and titlebar drag disable.
///
/// Called during fullscreen exit transitions (via resize events) and as a
/// safety-net after the animation completes. macOS resets the titlebar
/// container during animations; this re-applies our customizations.
///
/// Traffic light visibility is managed by macOS natively during fullscreen
/// transitions. The [`fullscreen`] notification observers center buttons
/// in `willExit`/`didExit` handlers.
pub fn reapply_traffic_lights(window: &Window, caption_height: f32) {
    center_traffic_lights(window, caption_height);
    disable_titlebar_drag(window);
}

/// Center traffic lights and disable titlebar drag using a raw `NSWindow` pointer.
///
/// Called synchronously from the `didExitFullScreen` notification handler
/// to position buttons correctly before any render frame occurs. Uses
/// `TAB_BAR_HEIGHT` directly (already in logical points).
///
/// # Safety
///
/// `nswindow` must be a valid, non-null `NSWindow` pointer.
unsafe fn center_and_disable_drag_raw(nswindow: *mut AnyObject) {
    let logical_height = oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT as f64;

    // Get the close button to find the titlebar container view hierarchy.
    let close_button: *mut AnyObject = msg_send![nswindow, standardWindowButton: 0i64];
    if close_button.is_null() {
        return;
    }

    // Disable implicit Core Animation to prevent animated position changes.
    let ca = AnyClass::get("CATransaction").expect("CATransaction not found");
    let _: () = msg_send![ca, begin];
    let _: () = msg_send![ca, setDisableActions: true];

    // Resize the titlebar container to match our tab bar height.
    // Skip `setFrame:` when already correct — the call posts
    // `NSViewFrameDidChangeNotification` synchronously, which can
    // trigger resize → centering → notification infinite recursion.
    let titlebar_view: *mut AnyObject = msg_send![close_button, superview];
    if !titlebar_view.is_null() {
        let container: *mut AnyObject = msg_send![titlebar_view, superview];
        if !container.is_null() {
            let c_frame: NSRect = msg_send![container, frame];
            let delta = logical_height - c_frame.h;
            if delta.abs() > 0.5 {
                let new_frame = NSRect {
                    x: c_frame.x,
                    y: c_frame.y - delta,
                    w: c_frame.w,
                    h: logical_height,
                };
                let _: () = msg_send![container, setFrame: new_frame];
            }
        }
    }

    // Center each button vertically within the container.
    let target_y_from_top = (logical_height - TRAFFIC_LIGHT_SIZE) / 2.0;
    for button_type in 0i64..3 {
        let button: *mut AnyObject = msg_send![nswindow, standardWindowButton: button_type];
        if button.is_null() {
            continue;
        }
        let frame: NSRect = msg_send![button, frame];
        let superview: *mut AnyObject = msg_send![button, superview];
        if superview.is_null() {
            continue;
        }
        let sv_frame: NSRect = msg_send![superview, frame];
        let new_y = sv_frame.h - target_y_from_top - frame.h;
        if (new_y - frame.y).abs() < 0.5 {
            continue;
        }
        let origin = NSPoint {
            x: frame.x,
            y: new_y,
        };
        let _: () = msg_send![button, setFrameOrigin: origin];
    }

    let _: () = msg_send![ca, commit];

    // Disable titlebar drag.
    let _: () = msg_send![nswindow, setMovableByWindowBackground: false];
    let _: () = msg_send![nswindow, setMovable: false];
}

/// Reposition traffic light buttons without resizing the container.
///
/// Safe to call from the `NSViewFrameDidChangeNotification` observer during
/// fullscreen transitions. Unlike [`center_and_disable_drag_raw`], this
/// does NOT call `setFrame:` on the `NSTitlebarContainerView`, avoiding
/// the `_syncToolbarPosition` infinite recursion on macOS 26 (Tahoe).
///
/// # Safety
///
/// `nswindow` must be a valid, non-null `NSWindow` pointer.
unsafe fn reposition_buttons_raw(nswindow: *mut AnyObject) {
    let logical_height = oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT as f64;

    let ca = AnyClass::get("CATransaction").expect("CATransaction not found");
    let _: () = msg_send![ca, begin];
    let _: () = msg_send![ca, setDisableActions: true];

    let target_y_from_top = (logical_height - TRAFFIC_LIGHT_SIZE) / 2.0;
    for button_type in 0i64..3 {
        let button: *mut AnyObject = msg_send![nswindow, standardWindowButton: button_type];
        if button.is_null() {
            continue;
        }
        let frame: NSRect = msg_send![button, frame];
        let superview: *mut AnyObject = msg_send![button, superview];
        if superview.is_null() {
            continue;
        }
        let sv_frame: NSRect = msg_send![superview, frame];
        let new_y = sv_frame.h - target_y_from_top - frame.h;
        if (new_y - frame.y).abs() < 0.5 {
            continue;
        }
        let origin = NSPoint {
            x: frame.x,
            y: new_y,
        };
        let _: () = msg_send![button, setFrameOrigin: origin];
    }

    let _: () = msg_send![ca, commit];
}

/// Disable `isMovableByWindowBackground` on the `NSWindow`.
///
/// With `fullsize_content_view(true)`, the transparent titlebar intercepts
/// mouse drags for window movement. This prevents our tab bar from receiving
/// drag events for tab reorder and tear-off. Setting this to `NO` makes the
/// titlebar inert — our `DragArea` hit handler calls `drag_window()` for
/// empty regions, matching Chrome's behavior.
fn disable_titlebar_drag(window: &Window) {
    let Some(nswindow) = get_nswindow(window) else {
        return;
    };
    unsafe {
        let _: () = msg_send![nswindow, setMovableByWindowBackground: false];
        let _: () = msg_send![nswindow, setMovable: false];
    }
}

/// Traffic light button diameter in points.
const TRAFFIC_LIGHT_SIZE: f64 = 12.0;

/// Vertically center the macOS traffic light buttons within the tab bar.
///
/// `caption_height` is in physical pixels (already scaled). We convert back
/// to logical points for `NSView` frame manipulation since `AppKit` works in
/// points.
///
/// Two adjustments are made:
/// 1. Resize the titlebar container view (`NSTitlebarContainerView`) to match
///    our tab bar height so hit-testing covers the full button area.
/// 2. Reposition each button vertically centered within the container.
fn center_traffic_lights(window: &Window, caption_height: f32) {
    let Some(nswindow) = get_nswindow(window) else {
        return;
    };
    let scale = window.scale_factor();
    let logical_height = caption_height as f64 / scale;

    // Get the close button to find the titlebar container view hierarchy.
    // Button → superview (NSTitlebarView) → superview (NSTitlebarContainerView).
    let close_button: *mut AnyObject = unsafe { msg_send![nswindow, standardWindowButton: 0i64] };
    if close_button.is_null() {
        return;
    }

    // Disable implicit Core Animation to prevent animated repositioning.
    let ca = AnyClass::get("CATransaction").expect("CATransaction not found");
    unsafe {
        let _: () = msg_send![ca, begin];
        let _: () = msg_send![ca, setDisableActions: true];
    }

    // Resize the titlebar container to match our tab bar height.
    // Skip when already correct — `setFrame:` posts
    // `NSViewFrameDidChangeNotification` synchronously, risking recursion.
    let titlebar_view: *mut AnyObject = unsafe { msg_send![close_button, superview] };
    if !titlebar_view.is_null() {
        let container: *mut AnyObject = unsafe { msg_send![titlebar_view, superview] };
        if !container.is_null() {
            let c_frame: NSRect = unsafe { msg_send![container, frame] };
            let delta = logical_height - c_frame.h;
            if delta.abs() > 0.5 {
                let new_frame = NSRect {
                    x: c_frame.x,
                    y: c_frame.y - delta,
                    w: c_frame.w,
                    h: logical_height,
                };
                unsafe {
                    let _: () = msg_send![container, setFrame: new_frame];
                }
            }
        }
    }

    // Center each button vertically within the (now resized) container.
    let target_y_from_top = (logical_height - TRAFFIC_LIGHT_SIZE) / 2.0;
    for button_type in 0i64..3 {
        let button: *mut AnyObject =
            unsafe { msg_send![nswindow, standardWindowButton: button_type] };
        if button.is_null() {
            continue;
        }

        let frame: NSRect = unsafe { msg_send![button, frame] };
        let superview: *mut AnyObject = unsafe { msg_send![button, superview] };
        if superview.is_null() {
            continue;
        }
        let sv_frame: NSRect = unsafe { msg_send![superview, frame] };

        let new_y = sv_frame.h - target_y_from_top - frame.h;
        if (new_y - frame.y).abs() < 0.5 {
            continue;
        }
        let origin = NSPoint {
            x: frame.x,
            y: new_y,
        };
        unsafe {
            let _: () = msg_send![button, setFrameOrigin: origin];
        }
    }

    unsafe {
        let _: () = msg_send![ca, commit];
    }
}

/// Gets the `NSWindow` from a winit `Window` via the `AppKit` raw handle.
///
/// winit provides an `NSView` handle; the `NSWindow` is its `window` property.
fn get_nswindow(window: &Window) -> Option<NonNull<AnyObject>> {
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => {
            let ns_view = h.ns_view.as_ptr().cast::<AnyObject>();
            let ns_window: *mut AnyObject = unsafe { msg_send![ns_view, window] };
            NonNull::new(ns_window)
        }
        _ => None,
    }
}

/// Get the current mouse cursor position in screen coordinates.
///
/// Returns `(x, y)` where y increases downward from the top of the
/// primary display. Uses `[NSEvent mouseLocation]` and converts from
/// macOS bottom-up to top-down coordinates.
pub fn cursor_screen_pos() -> (i32, i32) {
    unsafe {
        let cls = AnyClass::get("NSEvent").expect("NSEvent not found");
        let pos: NSPoint = msg_send![cls, mouseLocation];

        // Get the primary screen height for coordinate flipping.
        let screen_cls = AnyClass::get("NSScreen").expect("NSScreen not found");
        let main_screen: *mut AnyObject = msg_send![screen_cls, mainScreen];
        let screen_frame: NSRect = msg_send![main_screen, frame];

        // macOS y is bottom-up. Flip to top-down.
        let y = screen_frame.h - pos.y;
        (pos.x as i32, y as i32)
    }
}

/// Get a window's frame in screen coordinates (top-down y).
///
/// Returns `(left, top, right, bottom)` or `None` if the handle is invalid.
pub fn window_frame_bounds(window: &Window) -> Option<(i32, i32, i32, i32)> {
    let nswindow = get_nswindow(window)?;
    unsafe {
        let frame: NSRect = msg_send![nswindow.as_ptr(), frame];

        // Get primary screen height for coordinate flipping.
        let screen_cls = AnyClass::get("NSScreen").expect("NSScreen not found");
        let main_screen: *mut AnyObject = msg_send![screen_cls, mainScreen];
        let screen_frame: NSRect = msg_send![main_screen, frame];

        let left = frame.x as i32;
        // macOS frame.y is the bottom edge. Top = screen_h - (frame.y + frame.h).
        let top = (screen_frame.h - frame.y - frame.h) as i32;
        let right = (frame.x + frame.w) as i32;
        let bottom = top + frame.h as i32;
        Some((left, top, right, bottom))
    }
}
