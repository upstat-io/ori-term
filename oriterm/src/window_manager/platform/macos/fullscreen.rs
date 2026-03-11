//! Fullscreen transition notification observers.
//!
//! macOS provides `NSWindowDelegate` methods for fullscreen transitions, but
//! winit owns the delegate and doesn't expose `windowWillExitFullScreen:`.
//! We observe the corresponding `NSNotification` events instead, which fire
//! on the main thread at the same time as the delegate methods.
//!
//! The handlers center traffic lights synchronously and set atomic flags
//! consumed by `process_fullscreen_events()` for tab bar inset adjustments.
//!
//! An `NSViewFrameDidChangeNotification` observer on the titlebar container
//! repositions buttons when macOS rebuilds the container during transitions.
//! This observer calls [`reposition_buttons_raw`](super::reposition_buttons_raw)
//! (button-only, no container resize) to avoid the `_syncToolbarPosition`
//! infinite recursion that macOS 26 (Tahoe) triggers when the container
//! frame is changed inside a frame-change handler.

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
/// The per-window notification filtering (`object: nswindow`) ensures
/// handlers only fire for the transitioning window. This global bitfield
/// applies the event-loop flags to the focused window — acceptable because
/// only one window can be focused during a fullscreen transition.
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
/// on the given window, plus `NSViewFrameDidChangeNotification` on the
/// titlebar container. Handlers center traffic lights and set atomic flags
/// consumed by the event loop.
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

        // Observe the titlebar container's frame changes. When macOS
        // rebuilds the container during transitions, this repositions
        // buttons (without resizing the container) to keep them centered.
        let close_btn: *mut AnyObject =
            msg_send![nswindow.as_ptr(), standardWindowButton: 0i64];
        if !close_btn.is_null() {
            let titlebar_view: *mut AnyObject = msg_send![close_btn, superview];
            if !titlebar_view.is_null() {
                let container: *mut AnyObject = msg_send![titlebar_view, superview];
                if !container.is_null() {
                    let _: () =
                        msg_send![container, setPostsFrameChangedNotifications: true];
                    let name: *mut AnyObject = msg_send![
                        str_cls,
                        stringWithUTF8String:
                            c"NSViewFrameDidChangeNotification".as_ptr()
                    ];
                    let _: () = msg_send![
                        center,
                        addObserver: observer,
                        selector: sel!(handleFrameChange:),
                        name: name,
                        object: container,
                    ];
                }
            }
        }

        // The observer is intentionally leaked — it lives for the window's
        // lifetime. NSNotificationCenter filters by `object:`, so no
        // notifications fire after the window is deallocated.
    }
}

/// Returns (or lazily creates) the `ObjC` class for fullscreen notification
/// observers.
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
            builder.add_method(
                sel!(handleFrameChange:),
                handle_frame_change as unsafe extern "C" fn(_, _, _),
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
    // Full center (container resize + button reposition) before the exit
    // animation. Runs synchronously before macOS captures the snapshot.
    if !notif.is_null() {
        let nswindow: *mut AnyObject = msg_send![notif, object];
        if !nswindow.is_null() {
            unsafe { super::center_and_disable_drag_raw(nswindow) };
        }
    }
    FULLSCREEN_EVENTS.fetch_or(FS_WILL_EXIT, Ordering::Release);
}

unsafe extern "C" fn handle_did_exit_fs(
    _this: &AnyObject,
    _cmd: Sel,
    notif: *mut AnyObject,
) {
    // Full center after the animation completes. macOS may have rebuilt
    // the titlebar at default height during the transition.
    if !notif.is_null() {
        let nswindow: *mut AnyObject = msg_send![notif, object];
        if !nswindow.is_null() {
            unsafe { super::center_and_disable_drag_raw(nswindow) };
        }
    }
    FULLSCREEN_EVENTS.fetch_or(FS_DID_EXIT, Ordering::Release);
}

unsafe extern "C" fn handle_will_enter_fs(
    _this: &AnyObject,
    _cmd: Sel,
    _notif: *mut AnyObject,
) {
    FULLSCREEN_EVENTS.fetch_or(FS_WILL_ENTER, Ordering::Release);
}

unsafe extern "C" fn handle_did_enter_fs(
    _this: &AnyObject,
    _cmd: Sel,
    _notif: *mut AnyObject,
) {
    // No-op — observed for completeness.
}

/// Reposition buttons when macOS changes the titlebar container frame.
///
/// Only repositions buttons (`setFrameOrigin:`) — does NOT resize the
/// container (`setFrame:`). This avoids the `_syncToolbarPosition` infinite
/// recursion on macOS 26 that the full `center_and_disable_drag_raw` would
/// trigger when called from a frame-change handler.
unsafe extern "C" fn handle_frame_change(
    _this: &AnyObject,
    _cmd: Sel,
    notif: *mut AnyObject,
) {
    if !notif.is_null() {
        // The notification's object is the NSTitlebarContainerView.
        // Walk up to the NSWindow to call reposition_buttons_raw.
        let view: *mut AnyObject = msg_send![notif, object];
        if !view.is_null() {
            let nswindow: *mut AnyObject = msg_send![view, window];
            if !nswindow.is_null() {
                unsafe { super::reposition_buttons_raw(nswindow) };
            }
        }
    }
}
