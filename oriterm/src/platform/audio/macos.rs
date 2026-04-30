//! macOS bell-sound dispatch via AppKit `NSBeep`.

#![allow(
    unsafe_code,
    reason = "AppKit native bell FFI: NSBeep — thread-safe, no caller state"
)]

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    fn NSBeep();
}

/// Play the system alert sound via AppKit `NSBeep`.
///
/// Honors System Settings → Sound → Sound Effects → "Play user interface
/// sound effects", the user's selected alert sound, system mute, and
/// Do-Not-Disturb / Focus modes. Reference precedent: wezterm
/// `window/src/os/macos/connection.rs:169-173`.
///
/// Fire-and-forget. Failures (no audio output, sound subsystem
/// unavailable) are silently swallowed by AppKit per the bell contract.
pub(crate) fn play_bell() {
    // SAFETY: `NSBeep` is a thread-safe AppKit C function with no
    // parameters and no return value. It plays the system alert sound
    // and never modifies caller state. Apple documents it as safe to
    // call from any thread.
    unsafe {
        NSBeep();
    }
}
