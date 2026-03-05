//! Shared GPU pipelines and bind group layouts.
//!
//! [`GpuPipelines`] holds the five render pipelines and two bind group layouts
//! that are identical across all windows. Created once at startup and shared
//! by all [`WindowRenderer`](super::window_renderer::WindowRenderer) instances.

use wgpu::{BindGroupLayout, RenderPipeline};

use super::pipeline::{
    create_atlas_bind_group_layout, create_bg_pipeline, create_color_fg_pipeline,
    create_fg_pipeline, create_subpixel_fg_pipeline, create_ui_rect_pipeline,
    create_uniform_bind_group_layout,
};
use super::state::GpuState;

/// Stateless shared GPU resources: render pipelines and bind group layouts.
///
/// Pipelines are device-global (identical shader + layout config for every
/// window). Bind group *layouts* are needed by per-window
/// [`WindowRenderer::new`](super::window_renderer::WindowRenderer::new) to
/// create per-window uniform and atlas bind groups.
pub struct GpuPipelines {
    pub(crate) bg_pipeline: RenderPipeline,
    pub(crate) fg_pipeline: RenderPipeline,
    pub(crate) subpixel_fg_pipeline: RenderPipeline,
    pub(crate) color_fg_pipeline: RenderPipeline,
    pub(crate) ui_rect_pipeline: RenderPipeline,
    /// Layout for the per-window uniform bind group (`screen_size`).
    pub(crate) uniform_layout: BindGroupLayout,
    /// Layout for the per-window atlas bind groups (texture + sampler).
    pub(crate) atlas_layout: BindGroupLayout,
}

impl GpuPipelines {
    /// Create all shared pipelines and layouts from a GPU device.
    pub fn new(gpu: &GpuState) -> Self {
        let device = &gpu.device;
        let uniform_layout = create_uniform_bind_group_layout(device);
        let atlas_layout = create_atlas_bind_group_layout(device);

        Self {
            bg_pipeline: create_bg_pipeline(gpu, &uniform_layout),
            fg_pipeline: create_fg_pipeline(gpu, &uniform_layout, &atlas_layout),
            subpixel_fg_pipeline: create_subpixel_fg_pipeline(gpu, &uniform_layout, &atlas_layout),
            color_fg_pipeline: create_color_fg_pipeline(gpu, &uniform_layout, &atlas_layout),
            ui_rect_pipeline: create_ui_rect_pipeline(gpu, &uniform_layout),
            uniform_layout,
            atlas_layout,
        }
    }
}
