//! Fullscreen transition notification observers.
//!
//! macOS provides `NSWindowDelegate` methods for fullscreen transitions, but
//! winit owns the delegate and doesn't expose `windowWillExitFullScreen:`.
//! We observe the corresponding `NSNotification` events instead, which fire
//! on the main thread at the same time as the delegate methods.
//!
//! The handlers center traffic lights synchronously (before macOS captures
//! animation snapshots) and set atomic flags consumed by
//! `process_fullscreen_events()` for tab bar inset adjustments.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use objc2::declare::ClassBuilder;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{msg_send, sel};

use winit::window::Window;

// Bitfield flags for pending fullscreen transition events.
const FS_WILL_EXIT: u8 = 1;
const FS_DID_EXIT: u8 = 2;
const FS_WILL_ENTER: u8 = 4;

/// Pending fullscreen events, set by notification observers on the main thread.
static FULLSCREEN_EVENTS: AtomicU8 = AtomicU8::new(0);

/// Reentrancy guard for the titlebar frame-change observer.
///
/// `center_and_disable_drag_raw` changes the container frame, which
/// re-posts `NSViewFrameDidChangeNotification`. This flag breaks the cycle.
static CENTERING_GUARD: AtomicBool = AtomicBool::new(false);

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
/// Observes `willExit`, `didExit`, and `willEnter` notifications on the
/// given window. Handlers set atomic flags consumed by the event loop.
/// No traffic light visibility toggling — macOS handles it natively.
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
        // Observe the titlebar container's frame changes. During fullscreen
        // exit, macOS rebuilds the container at default height. This observer
        // fires synchronously, re-centering buttons before macOS captures the
        // animation snapshot — eliminating both the "bump" and "pop" artifacts.
        //
        // Set guard BEFORE enabling notifications to prevent immediate firing
        // during the initial layout pass.
        CENTERING_GUARD.store(true, Ordering::Release);
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
        CENTERING_GUARD.store(false, Ordering::Release);

        // The observer is intentionally leaked — it lives for the window's
        // lifetime. NSNotificationCenter filters by `object:`, so no
        // notifications fire after the window is deallocated.
    }
}

/// Returns (or lazily creates) the `ObjC` class for fullscreen notification
/// observers. Methods: `willExit`, `didExit`, `willEnter` set atomic flags
/// for the event loop; `handleFrameChange` re-centers traffic lights when
/// macOS rebuilds the titlebar container.
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
    // Pre-center buttons before macOS begins the exit animation. macOS may
    // rebuild the titlebar during the transition, but the frame-change
    // observer on NSTitlebarContainerView will catch that and re-center.
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
    // Safety-net centering after the animation completes. macOS may have
    // rebuilt the titlebar during the transition, resetting positions.
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

/// Re-center traffic lights when macOS changes the titlebar container frame.
///
/// Fires synchronously during fullscreen exit when macOS rebuilds the
/// `NSTitlebarContainerView` at its default height. By re-centering here,
/// buttons are positioned correctly before macOS captures the animation
/// snapshot — no visible bump or pop.
unsafe extern "C" fn handle_frame_change(
    _this: &AnyObject,
    _cmd: Sel,
    notif: *mut AnyObject,
) {
    // Reentrancy guard: `center_and_disable_drag_raw` calls `setFrame:` on
    // the container, which re-posts this notification.
    if CENTERING_GUARD.swap(true, Ordering::Acquire) {
        return;
    }

    if !notif.is_null() {
        // The notification's object is the NSTitlebarContainerView.
        let view: *mut AnyObject = msg_send![notif, object];
        if !view.is_null() {
            let nswindow: *mut AnyObject = msg_send![view, window];
            if !nswindow.is_null() {
                unsafe { super::center_and_disable_drag_raw(nswindow) };
            }
        }
    }

    CENTERING_GUARD.store(false, Ordering::Release);
}
