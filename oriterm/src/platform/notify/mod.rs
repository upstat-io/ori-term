//! Cross-platform desktop notification dispatch via native OS APIs.
//!
//! Sends OS-level notifications for long-running command completions
//! and shell-generated alerts (OSC 9 / 99 / 777) using the platform
//! native API directly via the `notify-rust` crate:
//!
//! - **Windows**: `WinRT` `Windows.UI.Notifications.ToastNotificationManager`
//!   via `tauri-winrt-notification`. Same path Windows Terminal uses.
//!   No PowerShell subprocess, no console-window flash.
//! - **Linux**: D-Bus call to the freedesktop notification daemon
//!   (`org.freedesktop.Notifications.Notify`). No `notify-send` fork.
//! - **macOS**: `NSUserNotification` / `UserNotifications.framework`.
//!   No `osascript` subprocess.
//!
//! All dispatch is fire-and-forget on a background thread to avoid
//! blocking the event loop. Failures are logged, never propagated.
//!
//! # Sound semantics
//!
//! Native notifications include the OS default notification sound by
//! default on Windows + macOS (subject to the user's system sound
//! settings). On Linux, the freedesktop spec sound hint
//! (`message-new-instant`) is set when `with_sound` is `true` and
//! omitted otherwise; daemon honoring is best-effort. The
//! `with_sound` parameter is preserved as a forward-compat surface
//! for future per-platform silent-toast support (Windows Toast
//! `<audio silent='true'/>` via direct `tauri-winrt-notification`
//! access, macOS `NSUserNotification.soundName = nil`).
//!
//! # Why notify-rust over the previous shell-out impl
//!
//! BUG-11-016 follow-up. The previous impl shelled out to
//! `powershell.exe -Command 'New-BurntToastNotification ...'` on
//! Windows, `notify-send` on Linux, `osascript` on macOS. That:
//!
//! 1. Spawned a subprocess per notification (slow + a console window
//!    flashed visibly on Windows).
//! 2. Required `BurntToast` PowerShell module on Windows, with a
//!    fragile `WinRT` XML fallback.
//! 3. Did not match how Windows Terminal / iTerm2 / Ghostty handle
//!    notifications — those use native APIs.
//!
//! `notify-rust` calls the native API directly via `tauri-winrt-notification`
//! on Windows + D-Bus on Linux + `Foundation` on macOS, eliminating all
//! three issues.

/// Send a desktop notification with the given title and body.
///
/// Dispatches via `notify-rust` on a background thread. If the
/// platform call fails, the error is logged and silently ignored —
/// notifications are best-effort.
///
/// `with_sound` is honored on Linux (sets the freedesktop
/// `sound-name` hint to `message-new-instant`); on Windows + macOS
/// the OS plays its default notification sound regardless of this
/// flag, since `notify-rust` does not expose Windows toast audio
/// suppression today. Forward-compat note: when this gap is closed,
/// `with_sound = false` will produce a silent toast on Windows /
/// macOS too.
pub fn send(title: &str, body: &str, with_sound: bool) {
    let title = title.to_owned();
    let body = body.to_owned();
    std::thread::spawn(move || {
        if let Err(e) = native_send(&title, &body, with_sound) {
            log::warn!("notification dispatch failed: {e}");
        }
    });
}

/// Native-API notification dispatch via `notify-rust`.
fn native_send(title: &str, body: &str, with_sound: bool) -> Result<(), notify_rust::error::Error> {
    let mut notification = notify_rust::Notification::new();
    notification.summary(title).body(body).appname("ori_term");

    // Linux freedesktop sound hint. Windows + macOS native toast
    // includes the OS default sound by default; suppression is not
    // currently exposed via notify-rust on those targets.
    #[cfg(all(unix, not(target_os = "macos")))]
    if with_sound {
        notification.sound_name("message-new-instant");
    }
    #[cfg(any(windows, target_os = "macos"))]
    let _ = with_sound;

    notification.show()?;
    Ok(())
}

#[cfg(test)]
mod tests;
