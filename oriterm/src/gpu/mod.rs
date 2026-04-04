//! GPU rendering: wgpu state management, render pipeline types, and platform transparency.

pub(crate) mod atlas;
pub(crate) mod bind_groups;
pub(crate) mod builtin_glyphs;
pub(crate) mod compositor;
pub(crate) mod extract;
pub(crate) mod frame_input;
pub(crate) mod icon_rasterizer;
pub(crate) mod image_render;
pub(crate) mod instance_writer;
pub(crate) mod pane_cache;
pub(crate) mod pipeline;
pub(crate) mod pipelines;
pub(crate) mod prepare;
pub(crate) mod prepared_frame;
pub(crate) mod render_target;
pub(crate) mod scene_convert;
pub(crate) mod state;
pub(crate) mod transparency;
pub(crate) mod ui_rect_writer;
pub(crate) mod window_renderer;

// Re-exports consumed by App and Window.
pub(crate) use extract::{
    extract_frame_from_snapshot, extract_frame_from_snapshot_into, snapshot_palette,
};
pub(crate) use frame_input::{
    FrameInput, FrameSearch, FrameSelection, MarkCursorOverride, ViewportSize,
};
pub(crate) use pane_cache::PaneRenderCache;
pub(crate) use pipelines::GpuPipelines;
pub(crate) use state::GpuState;
pub(crate) use transparency::apply_transparency;
pub(crate) use window_renderer::{SurfaceError, WindowRenderer};

/// Decode a single sRGB byte (0–255) to a linear-light `f32` (0.0–1.0).
///
/// Uses the IEC 61966-2-1 piecewise transfer function. Values at or below
/// the 0.04045 threshold are scaled linearly; above it the standard 2.4
/// power curve is applied.
pub(crate) fn srgb_to_linear(srgb_byte: u8) -> f32 {
    let s = f32::from(srgb_byte) / 255.0;
    srgb_f32_to_linear(s)
}

/// Decode an sRGB `f32` (0.0–1.0) to linear-light `f32` (0.0–1.0).
///
/// Same transfer function as [`srgb_to_linear`] but for float inputs
/// (e.g. UI Color components stored as sRGB f32).
pub(crate) fn srgb_f32_to_linear(s: f32) -> f32 {
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

// Re-export from oriterm_core for crate-internal use.
pub(crate) use oriterm_core::maybe_shrink_vec;

#[cfg(all(test, feature = "gpu-tests"))]
mod pipeline_tests;
#[cfg(test)]
mod subpixel_blend_tests;
#[cfg(test)]
mod tests;
#[cfg(all(test, feature = "gpu-tests"))]
mod visual_regression;
