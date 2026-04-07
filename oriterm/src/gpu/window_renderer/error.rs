//! Surface presentation errors.

use std::fmt;

/// Error returned by [`WindowRenderer::render_to_surface`](super::WindowRenderer).
///
/// Mirrors the wgpu `SurfaceError` variants the recovery state machine needs to
/// distinguish: a benign reconfigure (`Outdated`) versus a full epoch reset
/// (`Lost`), with terminal failure modes called out separately. The
/// `Other(String)` variant carries the wgpu detail string for logging and is
/// treated as a soft device loss by the recovery dispatcher.
#[derive(Debug)]
pub enum SurfaceError {
    /// Surface configuration is stale (e.g. the window was resized between
    /// `prepare()` and `render_to_surface()`).
    ///
    /// The device is healthy — the caller reconfigures the surface and retries
    /// on the next frame. This is the existing benign reconfigure path.
    Outdated,
    /// Surface (or device) is lost.
    ///
    /// The caller must enter the GPU recovery state machine: drop the surface,
    /// recreate the device, rebuild pipelines and per-window renderers.
    Lost,
    /// Surface acquisition timed out. Pass through and retry on the next frame.
    Timeout,
    /// GPU is out of memory.
    ///
    /// Recovery cannot resolve this — the caller transitions straight to the
    /// `Unavailable` state per the 5.16 contract.
    OutOfMemory,
    /// Unspecified surface error. Carries the wgpu detail string for logging.
    ///
    /// Treated as a soft device loss — escalates to the recovery state machine.
    /// Real-world `WSLg` / DX12 driver bugs surface here when a device-lost
    /// detail is reported via error scopes instead of `SurfaceError::Lost`.
    Other(String),
}

impl fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Outdated => f.write_str("surface outdated"),
            Self::Lost => f.write_str("surface or device lost"),
            Self::Timeout => f.write_str("surface timeout"),
            Self::OutOfMemory => f.write_str("GPU out of memory"),
            Self::Other(detail) => write!(f, "surface error: {detail}"),
        }
    }
}

impl std::error::Error for SurfaceError {}
