//! Native OS bell-sound dispatch.
//!
//! Plays the system bell using each platform's native API:
//!
//! - **Windows**: `MessageBeep(MB_OK)` via `windows-sys`. Plays the user's
//!   chosen "Default Beep" sound from Settings → System → Sound → Sounds.
//!   Honors system mute and the `System Sounds = (None)` setting.
//! - **macOS**: `NSBeep()` raw `AppKit` FFI. Honors System Settings → Sound →
//!   Sound Effects → "Play user interface sound effects", the user's selected
//!   alert sound, system mute, and Do-Not-Disturb / Focus modes.
//! - **Linux X11**: `XBell(display, 0)` via `x11-dl`. Honors `xset b`
//!   (volume / pitch / on-off).
//! - **Linux Wayland**: no-op stub. Compositor-aware bell
//!   (`xdg_system_bell_v1`) and `libcanberra` sound-theme-spec compliance
//!   are deferred to a follow-up bug.
//!
//! All paths are **in-process FFI** — no subprocess, no PowerShell, no
//! `notify-send`, no `paplay`, no `osascript`. Reference precedent:
//! wezterm `window/src/os/{windows,macos,x11}/connection.rs`.
//!
//! Fire-and-forget: failures (no audio device, X server unavailable, etc.)
//! are silently swallowed by the platform layer. The bell never blocks or
//! errors; the worst case is a silent invocation. Sound output is
//! unobservable from Rust, so tests assert no-panic + idempotency only;
//! the load-bearing verification is the manual user test on each
//! platform.

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use windows::play_bell;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::play_bell;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::play_bell;

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "linux")))]
pub(crate) fn play_bell() {}

#[cfg(test)]
mod tests;
