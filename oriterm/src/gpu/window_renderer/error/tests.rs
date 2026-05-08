//! Unit tests for [`SurfaceError`].

use super::SurfaceError;

#[test]
fn surface_error_display() {
    assert_eq!(SurfaceError::Outdated.to_string(), "surface outdated");
    assert_eq!(SurfaceError::Lost.to_string(), "surface or device lost");
    assert_eq!(SurfaceError::Timeout.to_string(), "surface timeout");
    assert_eq!(SurfaceError::OutOfMemory.to_string(), "GPU out of memory");
    assert_eq!(
        SurfaceError::Other("driver TDR".to_string()).to_string(),
        "surface error: driver TDR",
    );
}
