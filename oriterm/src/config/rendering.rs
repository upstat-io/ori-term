//! GPU rendering configuration.

use serde::{Deserialize, Serialize};

/// GPU backend selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GpuBackend {
    /// Automatic backend selection (default).
    #[default]
    Auto,
    /// Vulkan backend.
    Vulkan,
    /// DirectX 12 backend.
    #[serde(alias = "dx12")]
    DirectX12,
    /// Metal backend (macOS only).
    Metal,
}

impl GpuBackend {
    /// Returns the backends available on the current platform.
    pub(crate) fn available() -> &'static [(Self, &'static str)] {
        #[cfg(target_os = "windows")]
        {
            &[
                (Self::Auto, "Auto"),
                (Self::Vulkan, "Vulkan"),
                (Self::DirectX12, "DirectX 12"),
            ]
        }
        #[cfg(target_os = "macos")]
        {
            &[(Self::Auto, "Auto"), (Self::Metal, "Metal")]
        }
        #[cfg(target_os = "linux")]
        {
            &[(Self::Auto, "Auto"), (Self::Vulkan, "Vulkan")]
        }
    }
}

/// GPU rendering configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct RenderingConfig {
    /// GPU backend to use (default: Auto).
    pub gpu_backend: GpuBackend,
}
