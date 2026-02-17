//! GPU rendering: wgpu state management, render pipeline types, and platform transparency.

pub(crate) mod atlas;
pub(crate) mod bind_groups;
pub(crate) mod extract;
pub(crate) mod frame_input;
pub(crate) mod instance_writer;
pub(crate) mod pipeline;
pub(crate) mod prepare;
pub(crate) mod prepared_frame;
pub(crate) mod render_target;
pub(crate) mod renderer;
pub(crate) mod state;
pub(crate) mod transparency;

// Re-exports consumed starting in Section 5.10.
#[expect(
    unused_imports,
    reason = "atlas types used starting in Section 5.10"
)]
pub(crate) use atlas::{AtlasEntry, GlyphAtlas};
#[expect(
    unused_imports,
    reason = "bind group types used starting in Section 5.10"
)]
pub(crate) use bind_groups::{AtlasBindGroup, UniformBuffer, create_placeholder_atlas_texture};

// Extract phase re-exports consumed starting in Section 5.11 (App struct).
#[expect(
    unused_imports,
    reason = "extract functions used starting in Section 5.11"
)]
pub(crate) use extract::{extract_frame, extract_frame_into};

// Prepare phase re-exports consumed starting in Section 5.11 (App struct).
#[expect(
    unused_imports,
    reason = "prepare phase used starting in Section 5.11"
)]
pub(crate) use prepare::{AtlasLookup, prepare_frame, prepare_frame_into};

// Re-exports consumed starting in Section 5.9/5.10.
#[expect(
    unused_imports,
    reason = "render pipeline types used starting in Section 5.11"
)]
pub(crate) use frame_input::{FrameInput, FramePalette, ViewportSize};
#[expect(
    unused_imports,
    reason = "render pipeline types used starting in Section 5.9"
)]
pub(crate) use instance_writer::{InstanceKind, InstanceWriter};
#[expect(
    unused_imports,
    reason = "render pipeline types used starting in Section 5.9"
)]
pub(crate) use prepared_frame::PreparedFrame;
#[expect(
    unused_imports,
    reason = "render targets used starting in Section 5.13"
)]
pub(crate) use render_target::{ReadbackError, RenderTarget};
// Renderer re-exports consumed starting in Section 5.11 (App struct).
#[expect(
    unused_imports,
    reason = "renderer types used starting in Section 5.11"
)]
pub(crate) use renderer::{GpuRenderer, SurfaceError};
pub(crate) use state::validate_gpu;
#[expect(
    unused_imports,
    reason = "GpuState used once event loop is wired in Section 05"
)]
pub(crate) use state::{GpuInitError, GpuState};
