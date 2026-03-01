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
pub fn send(title: &str, body: &str) {
    let title = title.to_owned();
    let body = body.to_owned();
    std::thread::spawn(move || {
        if let Err(e) = platform_send(&title, &body) {
            log::warn!("notification dispatch failed: {e}");
        }
    });
}

/// Platform-specific notification dispatch (Windows).
#[cfg(windows)]
fn platform_send(title: &str, body: &str) -> std::io::Result<()> {
    use std::process::Command;

    // Use PowerShell to show a Windows toast notification via the
    // BurntToast module (widely available), falling back to the raw
    // Windows.UI.Notifications API.
    let script = format!(
        r#"
        if (Get-Module -ListAvailable -Name BurntToast) {{
            New-BurntToastNotification -Text '{title}', '{body}'
        }} else {{
            [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
            [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
            $xml = [Windows.Data.Xml.Dom.XmlDocument]::new()
            $xml.LoadXml("<toast><visual><binding template='ToastGeneric'><text>{title}</text><text>{body}</text></binding></visual></toast>")
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
fn platform_send(title: &str, body: &str) -> std::io::Result<()> {
    use std::process::Command;

    Command::new("notify-send")
        .args(["--app-name=ori_term", title, body])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

/// Platform-specific notification dispatch (macOS).
#[cfg(target_os = "macos")]
fn platform_send(title: &str, body: &str) -> std::io::Result<()> {
    use std::process::Command;

    let script = format!(
        r#"display notification "{body}" with title "{title}""#,
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
fn platform_send(title: &str, body: &str) -> std::io::Result<()> {
    log::debug!("notification (no platform handler): {title}: {body}");
    Ok(())
}

#[cfg(test)]
mod tests;
