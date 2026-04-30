//! Windows bell-sound dispatch via `MessageBeep`.

#![allow(
    unsafe_code,
    reason = "Win32 native bell FFI: MessageBeep — thread-safe, no caller state"
)]

use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
use windows_sys::Win32::UI::WindowsAndMessaging::MB_OK;

/// Play the system Default Beep via Win32 `MessageBeep`.
///
/// Honors the user's chosen sound from Settings → System → Sound → Sounds
/// (Default Beep entry); honors system mute and the `System Sounds = (None)`
/// setting. Reference precedent: wezterm
/// `window/src/os/windows/connection.rs:97-101`.
///
/// Fire-and-forget. Returns `BOOL` from the underlying API; failures
/// (e.g. no audio device, sound subsystem unavailable) are silently
/// swallowed per the bell contract.
pub(crate) fn play_bell() {
    // SAFETY: `MessageBeep` is a thread-safe Win32 API call with one
    // parameter (a sound-type constant). It returns BOOL; non-zero on
    // success. The function is documented as safe to call from any thread
    // and never modifies caller state.
    unsafe {
        MessageBeep(MB_OK);
    }
}
