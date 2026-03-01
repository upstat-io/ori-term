//! Unit tests for notification dispatch.

// Platform-specific dispatch is difficult to unit test (requires OS
// integration), but we can verify the public API compiles and the
// `send` function doesn't panic.

#[test]
fn send_does_not_panic() {
    // Fire-and-forget on a background thread — should not block or panic.
    super::send("Test Title", "Test body text");
}
