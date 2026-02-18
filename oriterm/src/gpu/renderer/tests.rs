//! Unit tests for the GPU renderer.
//!
//! These tests verify `SurfaceError` display formatting.
//! Full GPU integration tests (headless render + readback) live in Section 5.13.

use super::*;

// ── SurfaceError display ──

#[test]
fn surface_error_display() {
    assert_eq!(SurfaceError::Lost.to_string(), "surface lost or outdated");
    assert_eq!(SurfaceError::OutOfMemory.to_string(), "GPU out of memory");
    assert_eq!(SurfaceError::Timeout.to_string(), "surface timeout");
    assert_eq!(SurfaceError::Other.to_string(), "surface error");
}
