//! Unit tests for notification dispatch.

// Platform-specific dispatch is difficult to unit test (requires OS
// integration), but we can verify the public API compiles and the
// `send` function doesn't panic.

#[test]
fn send_does_not_panic_silent() {
    // Fire-and-forget on a background thread — should not block or panic.
    super::send("Test Title", "Test body text", false);
}

/// Regression: BUG-11-016 — `with_sound: true` requests platform default
/// notification sound (libnotify hint on Linux, BurntToast `-Sound 'Default'`
/// on Windows, `with sound name "default"` on macOS). Verifies the sound
/// path doesn't panic and the spawn succeeds.
#[test]
fn send_does_not_panic_with_sound() {
    super::send("Test Title", "Test body text", true);
}
