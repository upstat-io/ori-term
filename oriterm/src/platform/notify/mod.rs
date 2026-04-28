//! Cross-platform desktop notification dispatch.
//!
//! Sends OS-level notifications for long-running command completions
//! and shell-generated alerts (OSC 9/99/777):
//! - **Windows**: `PowerShell` toast via `New-BurntToastNotification` or
//!   `[Windows.UI.Notifications]` fallback.
//! - **Linux**: `notify-send` subprocess (libnotify/D-Bus).
//! - **macOS**: `osascript` display notification.
//!
//! All dispatch is fire-and-forget on a background thread to avoid
//! blocking the event loop. Failures are logged, never propagated.

/// Send a desktop notification with the given title and body.
///
/// Dispatches to the platform-specific notification mechanism on a
/// background thread. If the platform call fails, the error is logged
/// and silently ignored — notifications are best-effort.
///
/// `with_sound` requests the platform's default notification sound
/// alongside the visual toast. On Linux, this passes
/// `--hint=string:sound-name:message-new-instant` to `libnotify` (the
/// `freedesktop` spec sound hint). On Windows, `BurntToast` invocations
/// add `-Sound 'Default'`; the `WinRT` XML fallback adds an `<audio>`
/// element. On macOS, the `osascript` script appends
/// `with sound name "default"`. Sounds are best-effort — daemons /
/// theme settings ultimately decide whether audio plays.
pub fn send(title: &str, body: &str, with_sound: bool) {
    let title = title.to_owned();
    let body = body.to_owned();
    std::thread::spawn(move || {
        if let Err(e) = platform_send(&title, &body, with_sound) {
            log::warn!("notification dispatch failed: {e}");
        }
    });
}

/// Platform-specific notification dispatch (Windows).
#[cfg(windows)]
fn platform_send(title: &str, body: &str, with_sound: bool) -> std::io::Result<()> {
    use std::process::Command;

    // BurntToast `-Sound 'Default'` selects the system default toast
    // sound. The WinRT XML fallback adds an `<audio>` element pointing
    // at the default sound. Omitting both produces a silent toast.
    let burnt_sound_arg = if with_sound { " -Sound 'Default'" } else { "" };
    let xml_audio_element = if with_sound {
        "<audio src='ms-winsoundevent:Notification.Default' />"
    } else {
        ""
    };

    let script = format!(
        r#"
        if (Get-Module -ListAvailable -Name BurntToast) {{
            New-BurntToastNotification -Text '{title}', '{body}'{burnt_sound_arg}
        }} else {{
            [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
            [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
            $xml = [Windows.Data.Xml.Dom.XmlDocument]::new()
            $xml.LoadXml("<toast><visual><binding template='ToastGeneric'><text>{title}</text><text>{body}</text></binding></visual>{xml_audio_element}</toast>")
            $toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('ori_term').Show($toast)
        }}
        "#,
        title = title.replace('\'', "''"),
        body = body.replace('\'', "''"),
    );

    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

/// Platform-specific notification dispatch (Linux).
#[cfg(target_os = "linux")]
fn platform_send(title: &str, body: &str, with_sound: bool) -> std::io::Result<()> {
    use std::process::Command;

    let mut cmd = Command::new("notify-send");
    cmd.args(["--app-name=ori_term"]);
    if with_sound {
        // Freedesktop sound naming spec: `message-new-instant` is the
        // canonical "new message" notification sound. notify-send +
        // libnotify pass the hint to the notification daemon, which
        // plays the sound from the active sound theme. Honoring the
        // hint is daemon-dependent — best-effort.
        cmd.args(["--hint=string:sound-name:message-new-instant"]);
    }
    cmd.args([title, body])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

/// Platform-specific notification dispatch (macOS).
#[cfg(target_os = "macos")]
fn platform_send(title: &str, body: &str, with_sound: bool) -> std::io::Result<()> {
    use std::process::Command;

    let sound_clause = if with_sound {
        r#" sound name "default""#
    } else {
        ""
    };

    let script = format!(
        r#"display notification "{body}" with title "{title}"{sound_clause}"#,
        title = title.replace('"', r#"\""#),
        body = body.replace('"', r#"\""#),
    );

    Command::new("osascript")
        .args(["-e", &script])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

/// Fallback for unsupported platforms.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn platform_send(title: &str, body: &str, with_sound: bool) -> std::io::Result<()> {
    log::debug!("notification (no platform handler): {title}: {body} (with_sound={with_sound})");
    Ok(())
}

#[cfg(test)]
mod tests;
