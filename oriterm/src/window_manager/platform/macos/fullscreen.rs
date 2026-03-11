//! Fullscreen transition notification observers.
//!
//! macOS provides `NSWindowDelegate` methods for fullscreen transitions, but
//! winit owns the delegate and doesn't expose `windowWillExitFullScreen:`.
//! We observe the corresponding `NSNotification` events instead, which fire
//! on the main thread at the same time as the delegate methods.
//!
//! The handlers hide/show the `NSTitlebarContainerView` during fullscreen
//! exit (Electron pattern) to prevent traffic light jump artifacts, center
//! buttons, and set atomic flags consumed by `process_fullscreen_events()`
//! for tab bar inset adjustments.
//!
//! Note: earlier versions used an `NSViewFrameDidChangeNotification` observer
//! on the titlebar container as a supplementary centering mechanism. This was
//! removed because on macOS 26 (Tahoe), resizing the container inside a
//! frame-change handler triggers an infinite `_syncToolbarPosition` ↔
//! `_updateTitlebarContainerViewFrameIfNecessary` recursion in `AppKit`. The
//! hide/show pattern makes it unnecessary — buttons are hidden during the
//! exit animation and explicitly centered in `handle_did_exit_fs`.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use objc2::declare::ClassBuilder;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{msg_send, sel};

use winit::window::Window;

// Bitfield flags for pending fullscreen transition events.
const FS_WILL_EXIT: u8 = 1;
const FS_DID_EXIT: u8 = 2;
const FS_WILL_ENTER: u8 = 4;

/// Pending fullscreen events, set by notification observers on the main thread.
///
/// The hide/show calls in notification handlers operate per-window (observers
/// are registered with `object: nswindow`). This global bitfield is a
/// simplification for the event-loop flags — it applies to the focused window
/// only. Acceptable because only one window can be focused during a fullscreen
/// transition.
static FULLSCREEN_EVENTS: AtomicU8 = AtomicU8::new(0);

/// Atomically consume all pending fullscreen transition events.
///
/// Returns `None` if no events are pending. The event loop calls this
/// in `about_to_wait` before rendering to apply tab bar inset changes.
pub fn take_fullscreen_events() -> Option<FullscreenEvents> {
    let bits = FULLSCREEN_EVENTS.swap(0, Ordering::AcqRel);
    if bits == 0 {
        None
    } else {
        Some(FullscreenEvents(bits))
    }
}

/// Pending fullscreen transition events consumed by the event loop.
#[derive(Debug, Clone, Copy)]
pub struct FullscreenEvents(u8);

impl FullscreenEvents {
    /// Window is about to exit fullscreen — restore tab bar inset.
    pub fn will_exit(self) -> bool {
        self.0 & FS_WILL_EXIT != 0
    }

    /// Window finished exiting fullscreen — re-apply traffic light centering.
    pub fn did_exit(self) -> bool {
        self.0 & FS_DID_EXIT != 0
    }

    /// Window is about to enter fullscreen — remove tab bar inset.
    pub fn will_enter(self) -> bool {
        self.0 & FS_WILL_ENTER != 0
    }
}

/// Register `NSNotificationCenter` observers for fullscreen transitions.
///
/// Observes `willExit`, `didExit`, `willEnter`, and `didEnter` notifications
/// on the given window. The `willExit`/`didExit` handlers toggle
/// `NSTitlebarContainerView` visibility (Electron pattern) to prevent
/// traffic light jump artifacts. Handlers also set atomic flags consumed
/// by the event loop.
///
/// Called once per window from `install_chrome`.
pub(super) fn install_fullscreen_observers(window: &Window) {
    let Some(nswindow) = super::get_nswindow(window) else {
        return;
    };
    let cls = fullscreen_observer_class();

    unsafe {
        // Allocate an observer instance (retained by NSNotificationCenter).
        let observer: *mut AnyObject = msg_send![cls, alloc];
        let observer: *mut AnyObject = msg_send![observer, init];
        if observer.is_null() {
            return;
        }

        let nc_cls =
            AnyClass::get("NSNotificationCenter").expect("NSNotificationCenter not found");
        let center: *mut AnyObject = msg_send![nc_cls, defaultCenter];
        let str_cls = AnyClass::get("NSString").expect("NSString not found");

        // Register for each fullscreen notification.
        let registrations: &[(*const i8, Sel)] = &[
            (
                c"NSWindowWillExitFullScreenNotification".as_ptr(),
                sel!(handleWillExit:),
            ),
            (
                c"NSWindowDidExitFullScreenNotification".as_ptr(),
                sel!(handleDidExit:),
            ),
            (
                c"NSWindowWillEnterFullScreenNotification".as_ptr(),
                sel!(handleWillEnter:),
            ),
            (
                c"NSWindowDidEnterFullScreenNotification".as_ptr(),
                sel!(handleDidEnter:),
            ),
        ];

        for &(name_ptr, handler_sel) in registrations {
            let ns_name: *mut AnyObject =
                msg_send![str_cls, stringWithUTF8String: name_ptr];
            let _: () = msg_send![
                center,
                addObserver: observer,
                selector: handler_sel,
                name: ns_name,
                object: nswindow.as_ptr(),
            ];
        }

        // The observer is intentionally leaked — it lives for the window's
        // lifetime. NSNotificationCenter filters by `object:`, so no
        // notifications fire after the window is deallocated.
    }
}

/// Returns (or lazily creates) the `ObjC` class for fullscreen notification
/// observers. Methods: `willExit`, `didExit`, `willEnter`, `didEnter` set
/// atomic flags and manage titlebar container visibility.
fn fullscreen_observer_class() -> &'static AnyClass {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    CLASS.get_or_init(|| {
        let superclass = AnyClass::get("NSObject").expect("NSObject not found");
        let mut builder = ClassBuilder::new("OriFullscreenObserver", superclass)
            .expect("OriFullscreenObserver already registered");

        // SAFETY: Handler signatures match the ObjC notification callback
        // convention: (self, _cmd, NSNotification*). The `as fn(_, _, _)`
        // cast uses wildcards so Rust preserves the HRTB on `&AnyObject`.
        unsafe {
            builder.add_method(
                sel!(handleWillExit:),
                handle_will_exit_fs as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(handleDidExit:),
                handle_did_exit_fs as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(handleWillEnter:),
                handle_will_enter_fs as unsafe extern "C" fn(_, _, _),
            );
            builder.add_method(
                sel!(handleDidEnter:),
                handle_did_enter_fs as unsafe extern "C" fn(_, _, _),
            );
        }

        builder.register()
    })
}

unsafe extern "C" fn handle_will_exit_fs(
    _this: &AnyObject,
    _cmd: Sel,
    notif: *mut AnyObject,
) {
    if !notif.is_null() {
        let nswindow: *mut AnyObject = msg_send![notif, object];
        if !nswindow.is_null() {
            // Center buttons, then hide the container so the macOS animation
            // snapshot does not show buttons at default positions. Electron
            // uses this same pattern to prevent the traffic light jump.
            unsafe { super::center_and_disable_drag_raw(nswindow) };
            unsafe { super::set_titlebar_container_hidden(nswindow, true) };
        }
    }
    FULLSCREEN_EVENTS.fetch_or(FS_WILL_EXIT, Ordering::Release);
}

unsafe extern "C" fn handle_did_exit_fs(
    _this: &AnyObject,
    _cmd: Sel,
    notif: *mut AnyObject,
) {
    if !notif.is_null() {
        let nswindow: *mut AnyObject = msg_send![notif, object];
        if !nswindow.is_null() {
            // Center buttons while still hidden, then show the container so
            // traffic lights appear at the correct position with no jump.
            unsafe { super::center_and_disable_drag_raw(nswindow) };
            unsafe { super::set_titlebar_container_hidden(nswindow, false) };
            // Re-center after show — macOS 26 (Tahoe) re-layouts the
            // container when setHidden: is toggled, resetting positions.
            unsafe { super::center_and_disable_drag_raw(nswindow) };
        }
    }
    FULLSCREEN_EVENTS.fetch_or(FS_DID_EXIT, Ordering::Release);
}

unsafe extern "C" fn handle_will_enter_fs(
    _this: &AnyObject,
    _cmd: Sel,
    notif: *mut AnyObject,
) {
    // Ensure container is visible when entering fullscreen (safety net for
    // interrupted exit transitions that may have left it hidden).
    if !notif.is_null() {
        let nswindow: *mut AnyObject = msg_send![notif, object];
        if !nswindow.is_null() {
            unsafe { super::set_titlebar_container_hidden(nswindow, false) };
        }
    }
    FULLSCREEN_EVENTS.fetch_or(FS_WILL_ENTER, Ordering::Release);
}

unsafe extern "C" fn handle_did_enter_fs(
    _this: &AnyObject,
    _cmd: Sel,
    _notif: *mut AnyObject,
) {
    // No-op — exists so `didEnter` is observed. Future use: clear transition
    // state or re-apply settings after entering fullscreen.
}
