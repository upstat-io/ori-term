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

/// GPU rendering configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct RenderingConfig {
    /// GPU backend to use (default: Auto).
    pub gpu_backend: GpuBackend,
}
